#!/usr/bin/env python3
"""Photo finish: low-strength, depth-guided img2img with Segmind-Vega + T2I-Adapter (depth-midas SDXL),
all network inference through ONNX Runtime on CPU. The scheduler loop is plain numpy.

Inputs : --color  512x512 RGB render
         --depth  512x512 grayscale depth, NEAR = WHITE, FAR/background = BLACK (MiDaS convention)
Output : --out    512x512 RGB PNG, plus <out>.timing.json (and one line appended to --log if given)

Example:
  python finish.py --color c.png --depth d.png --out o.png --strength 0.3 --steps 20 --seed 1
Batch (loads models once): --manifest pairs.csv
  lines: color,depth,out[,embeds[,mask[,seed[,target_lab]]]]

PoC 8 additions (roadmap 2 and 4):
  - Guardrails: the noise seed comes from the manifest (derived from the genome
    seed and age), so a regeneration is bit-identical; every output records a
    pipeline key (hash of model files, prompt set, scheduler, parameters and
    FINISH_VERSION) and, with --cache, is stored under cache/<key>/ and reused.
    Model provenance lives in models.lock.json (provenance.py).
  - Constrained finish: with a region mask (R = skin the finish may change;
    eyes, brows, lashes, hair are renderer-owned) the latent is re-noised from
    the render outside the mask at every step (latent inpainting), the result
    is composited over the render with a feathered mask, and skin tone is
    locked to the render in CIELAB (mean L*, a*, b* over the skin mask).

Scheduler: EulerDiscreteScheduler reproduced from Vega's scheduler/scheduler_config.json
(scaled_linear betas 0.00085..0.012, 1000 train steps, epsilon prediction, timestep_spacing=leading,
steps_offset=1, linear sigma interpolation, final sigma 0) with diffusers' SDXL img2img strength logic:
  init_timestep = min(int(steps*strength), steps); start at index steps - init_timestep;
  x = z0 + sigma_start * noise; then Euler steps. Only int(steps*strength) UNet evaluations run.
"""
import argparse
import json
import os
import platform
import resource
import subprocess
import time
from pathlib import Path

import numpy as np
import onnxruntime as ort
from PIL import Image

HERE = Path(__file__).resolve().parent
DL = HERE.parent / "downloads"
ONNX = DL / "onnx"
SCHED_CFG = DL / "hf" / "Segmind-Vega" / "scheduler" / "scheduler_config.json"
VAE_CFG = DL / "hf" / "Segmind-Vega" / "vae" / "config.json"


def cpu_model() -> str:
    try:
        out = subprocess.run(["lscpu"], capture_output=True, text=True).stdout
        for line in out.splitlines():
            if line.startswith("Model name:"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return platform.processor() or "unknown"


class EulerScheduler:
    def __init__(self, cfg: dict):
        assert cfg["_class_name"] == "EulerDiscreteScheduler", cfg["_class_name"]
        assert cfg["beta_schedule"] == "scaled_linear" and cfg["prediction_type"] == "epsilon"
        assert cfg["timestep_spacing"] == "leading" and cfg["interpolation_type"] == "linear"
        assert not cfg.get("use_karras_sigmas")
        self.T = cfg["num_train_timesteps"]
        self.offset = cfg["steps_offset"]
        betas = np.linspace(cfg["beta_start"] ** 0.5, cfg["beta_end"] ** 0.5, self.T, dtype=np.float32) ** 2
        ac = np.cumprod(1.0 - betas.astype(np.float64))
        self.train_sigmas = ((1 - ac) / ac) ** 0.5

    def schedule(self, steps: int, strength: float):
        ratio = self.T // steps
        ts = (np.arange(0, steps) * ratio).round()[::-1].astype(np.float32) + self.offset
        sig = np.interp(ts, np.arange(self.T), self.train_sigmas)
        sig = np.concatenate([sig, [0.0]]).astype(np.float32)
        init = min(int(steps * strength), steps)
        start = max(steps - init, 0)
        return ts[start:], sig[start:]


FINISH_VERSION = "poc8-1"


def srgb_to_lab(rgb):
    """sRGB uint8 (..., 3) -> CIELAB D65."""
    c = rgb.astype(np.float64) / 255.0
    c = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    M = np.array([[0.4124, 0.3576, 0.1805], [0.2126, 0.7152, 0.0722], [0.0193, 0.1192, 0.9505]])
    xyz = c @ M.T / np.array([0.9505, 1.0, 1.089])
    f = np.where(xyz > 0.008856, np.cbrt(xyz), 7.787 * xyz + 16 / 116)
    return np.stack([116 * f[..., 1] - 16, 500 * (f[..., 0] - f[..., 1]), 200 * (f[..., 1] - f[..., 2])], -1)


def lab_to_srgb(lab):
    fy = (lab[..., 0] + 16) / 116
    fx = fy + lab[..., 1] / 500
    fz = fy - lab[..., 2] / 200
    inv = lambda f: np.where(f ** 3 > 0.008856, f ** 3, (f - 16 / 116) / 7.787)
    xyz = np.stack([inv(fx) * 0.9505, inv(fy), inv(fz) * 1.089], -1)
    Mi = np.linalg.inv(np.array([[0.4124, 0.3576, 0.1805], [0.2126, 0.7152, 0.0722], [0.0193, 0.1192, 0.9505]]))
    c = np.clip(xyz @ Mi.T, 0, 1)
    c = np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)
    return (c * 255).round().clip(0, 255).astype(np.uint8)


def feather(mask, px):
    """Box-blur a float mask a few times (cheap Gaussian-ish feather)."""
    m = mask.astype(np.float32)
    k = max(int(px), 1)
    for _ in range(3):
        pad = np.pad(m, k, mode="edge")
        c = np.cumsum(np.cumsum(pad, 0), 1)
        c = np.pad(c, ((1, 0), (1, 0)))
        n = 2 * k + 1
        m = (c[n:, n:] - c[:-n, n:] - c[n:, :-n] + c[:-n, :-n]) / (n * n)
    return m


class Finisher:
    def __init__(self, threads: int, arena: bool = False):
        so = ort.SessionOptions()
        so.intra_op_num_threads = threads
        so.inter_op_num_threads = 1
        so.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
        so.enable_cpu_mem_arena = arena  # arena off lowers peak RAM (activations freed between runs)
        prov = ["CPUExecutionProvider"]
        t = time.time()
        self.adapter = ort.InferenceSession(str(ONNX / "adapter" / "adapter.onnx"), so, providers=prov)
        self.vae_enc = ort.InferenceSession(str(ONNX / "vae_encoder" / "vae_encoder.onnx"), so, providers=prov)
        self.vae_dec = ort.InferenceSession(str(ONNX / "vae_decoder" / "vae_decoder.onnx"), so, providers=prov)
        self.unet = ort.InferenceSession(str(ONNX / "unet" / "unet.onnx"), so, providers=prov)
        self.load_s = time.time() - t
        self._embeds = {}
        self.use_embeds("embeds")
        self.sched = EulerScheduler(json.loads(SCHED_CFG.read_text()))
        self.scaling = json.loads(VAE_CFG.read_text())["scaling_factor"]
        self.threads = threads

    def use_embeds(self, name):
        """Selects a precomputed prompt set (a folder under downloads/onnx/)."""
        if name not in self._embeds:
            e = ONNX / name
            pe = np.concatenate([np.load(e / "negative_prompt_embeds.npy"), np.load(e / "prompt_embeds.npy")])
            pooled = np.concatenate(
                [np.load(e / "negative_pooled_prompt_embeds.npy"), np.load(e / "pooled_prompt_embeds.npy")]
            )
            self._embeds[name] = (pe, pooled, json.loads((e / "prompt.json").read_text()))
        self.pe, self.pooled, self.prompt = self._embeds[name]

    def run(self, color_p, depth_p, out_p, strength, steps, seed, guidance, adapter_scale, res, mask_p=None, tone_lock=True):
        T = {}
        t0 = time.time()
        rng = np.random.default_rng(seed)
        skin = None
        if mask_p:
            mk = Image.open(mask_p).convert("RGB")
            skin = np.asarray(mk.resize((res, res), Image.BILINEAR), np.float32)[..., 0] / 255.0
        col = Image.open(color_p).convert("RGB")
        dep = Image.open(depth_p).convert("L")
        in_size = col.size
        col_r = col.resize((res, res), Image.LANCZOS) if col.size != (res, res) else col
        dep_r = dep.resize((res, res), Image.LANCZOS) if dep.size != (res, res) else dep
        img = np.asarray(col_r, np.float32).transpose(2, 0, 1)[None] / 127.5 - 1.0
        d = np.asarray(dep_r, np.float32)[None, None] / 255.0
        d = np.repeat(d, 3, axis=1)  # adapter expects 3-channel [0,1] (diffusers _preprocess_adapter_image)

        t = time.time()
        mean, logvar = self.vae_enc.run(None, {"image": img})
        logvar = np.clip(logvar, -30.0, 20.0)
        z0 = (mean + np.exp(0.5 * logvar) * rng.standard_normal(mean.shape).astype(np.float32)) * self.scaling
        T["vae_encode_s"] = time.time() - t

        t = time.time()
        res_maps = [r * adapter_scale for r in self.adapter.run(None, {"depth": d})]
        res_maps = [np.concatenate([r, r]).astype(np.float32) for r in res_maps]  # CFG batch [uncond, cond]
        T["adapter_s"] = time.time() - t

        ts, sig = self.sched.schedule(steps, strength)
        noise = rng.standard_normal(z0.shape).astype(np.float32)
        x = (z0 + sig[0] * noise).astype(np.float32)
        if skin is not None:
            # Latent-space mask (64x64 for 512), slightly eroded so the finish
            # never reaches into renderer-owned regions.
            lat = z0.shape[-1]
            m_lat = np.asarray(Image.fromarray((skin * 255).astype(np.uint8)).resize((lat, lat), Image.BILINEAR), np.float32) / 255.0
            m_lat = np.clip((m_lat - 0.3) / 0.6, 0, 1)[None, None]
        time_ids = np.array([[res, res, 0, 0, res, res]] * 2, np.float32)
        step_times = []
        for i, tt in enumerate(ts):
            t = time.time()
            inp = (x / np.sqrt(sig[i] ** 2 + 1)).astype(np.float32)
            (eps,) = self.unet.run(
                None,
                {
                    "sample": np.concatenate([inp, inp]),
                    "timestep": np.array([tt], np.float32),
                    "encoder_hidden_states": self.pe,
                    "text_embeds": self.pooled,
                    "time_ids": time_ids,
                    "r0": res_maps[0],
                    "r1": res_maps[1],
                    "r2": res_maps[2],
                    "r3": res_maps[3],
                },
            )
            eps = eps[0:1] + guidance * (eps[1:2] - eps[0:1])
            x = (x + eps * (sig[i + 1] - sig[i])).astype(np.float32)  # Euler, epsilon pred, s_churn=0
            if skin is not None:
                # Outside the mask, follow the render's own noised latent.
                x = (m_lat * x + (1 - m_lat) * (z0 + sig[i + 1] * noise)).astype(np.float32)
            step_times.append(time.time() - t)
        T["unet_steps"] = len(ts)
        T["unet_total_s"] = float(sum(step_times))
        T["unet_per_step_s"] = float(np.mean(step_times)) if step_times else 0.0

        t = time.time()
        (im,) = self.vae_dec.run(None, {"latent": (x / self.scaling).astype(np.float32)})
        T["vae_decode_s"] = time.time() - t
        im = ((np.clip(im[0], -1, 1) + 1) * 127.5).round().astype(np.uint8).transpose(1, 2, 0)
        lock = None
        if skin is not None:
            src = np.asarray(col_r, np.uint8)
            sel = skin > 0.6
            if tone_lock and sel.sum() > 200:
                lab_f = srgb_to_lab(im)
                lab_r = srgb_to_lab(src)
                shift = lab_r[sel].mean(0) - lab_f[sel].mean(0)
                w = feather(skin, 6)[..., None]
                im = lab_to_srgb(lab_f + shift * w)
                lock = [round(float(v), 2) for v in shift]
            # Renderer-owned regions come back exactly from the render.
            w = np.clip(feather(np.clip((skin - 0.3) / 0.6, 0, 1), 3), 0, 1)[..., None]
            im = (w * im + (1 - w) * src).round().astype(np.uint8)
        out = Image.fromarray(im, "RGB")
        if out.size != in_size:
            out = out.resize(in_size, Image.LANCZOS)
        Path(out_p).parent.mkdir(parents=True, exist_ok=True)
        out.save(out_p)
        T["total_s"] = time.time() - t0
        rec = {
            "color": str(color_p), "depth": str(depth_p), "out": str(out_p),
            "work_res": res, "steps": steps, "strength": strength, "seed": seed,
            "guidance_scale": guidance, "adapter_conditioning_scale": adapter_scale,
            "timesteps": [float(v) for v in ts],
            "timings": {k: round(v, 3) if isinstance(v, float) else v for k, v in T.items()},
            "model_load_s": round(self.load_s, 3), "threads": self.threads,
            "peak_rss_gb": round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1e6, 2),
            "cpu": cpu_model(), "onnxruntime": ort.__version__,
            "prompt": self.prompt["prompt"], "negative_prompt": self.prompt["negative_prompt"],
            "mask": str(mask_p) if mask_p else None, "tone_lock_lab_shift": lock,
            "finish_version": FINISH_VERSION,
        }
        return rec


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--color")
    ap.add_argument("--depth")
    ap.add_argument("--out")
    ap.add_argument("--manifest", help="CSV lines color,depth,out[,embeds] (models are loaded once)")
    ap.add_argument("--strength", type=float, default=0.3)
    ap.add_argument("--steps", type=int, default=20, help="scheduler steps; UNet runs int(steps*strength)")
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--guidance", type=float, default=5.0)
    ap.add_argument("--adapter-scale", type=float, default=0.8)
    ap.add_argument("--res", type=int, default=512, help="internal working resolution (multiple of 64)")
    ap.add_argument("--threads", type=int, default=os.cpu_count())
    ap.add_argument("--arena", action="store_true", help="enable ORT CPU memory arena (more RAM)")
    ap.add_argument("--embeds", default="embeds", help="prompt set folder under downloads/onnx (see prompts.py)")
    ap.add_argument("--log", help="append one JSON line per image to this file")
    ap.add_argument("--no-tone-lock", action="store_true")
    ap.add_argument("--cache", help="cache root: outputs stored under <cache>/<pipeline key>/ and reused")
    a = ap.parse_args()
    assert a.res % 64 == 0, "--res must be a multiple of 64"
    if a.manifest:
        jobs = [l.strip().split(",") for l in Path(a.manifest).read_text().splitlines() if l.strip()]
    else:
        if not (a.color and a.depth and a.out):
            ap.error("need --color --depth --out (or --manifest)")
        jobs = [(a.color, a.depth, a.out, a.embeds)]

    from provenance import pipeline_key
    f = Finisher(a.threads, a.arena)
    print(f"models loaded in {f.load_s:.1f}s")
    for job in jobs:
        c, d, o = job[:3]
        emb = job[3] if len(job) > 3 and job[3] else a.embeds
        mask = job[4] if len(job) > 4 and job[4] else None
        seed = int(job[5]) if len(job) > 5 and job[5] else a.seed
        f.use_embeds(emb)
        params = dict(strength=a.strength, steps=a.steps, guidance=a.guidance, adapter_scale=a.adapter_scale,
                      res=a.res, masked=bool(mask), tone_lock=not a.no_tone_lock)
        key = pipeline_key(emb, params, FINISH_VERSION)
        cached = Path(a.cache) / key / (Path(o).stem + f"_s{seed}.png") if a.cache else None
        if cached and cached.exists():
            Path(o).parent.mkdir(parents=True, exist_ok=True)
            Path(o).write_bytes(cached.read_bytes())
            print(f"{o}: cache hit {cached}")
            continue
        rec = f.run(c, d, o, a.strength, a.steps, seed, a.guidance, a.adapter_scale, a.res, mask, not a.no_tone_lock)
        rec["pipeline_key"] = key
        if cached:
            cached.parent.mkdir(parents=True, exist_ok=True)
            cached.write_bytes(Path(o).read_bytes())
        tm = rec["timings"]
        print(
            f"{o}: {tm['total_s']:.1f}s/image  (res {a.res}, {tm['unet_steps']} UNet steps x "
            f"{tm['unet_per_step_s']:.1f}s, vae enc {tm['vae_encode_s']:.1f}s, dec {tm['vae_decode_s']:.1f}s, "
            f"adapter {tm['adapter_s']:.1f}s, peak RSS {rec['peak_rss_gb']:.1f} GB)"
        )
        Path(str(o) + ".timing.json").write_text(json.dumps(rec, indent=2))
        if a.log:
            with open(a.log, "a") as fh:
                fh.write(json.dumps(rec) + "\n")


if __name__ == "__main__":
    main()
