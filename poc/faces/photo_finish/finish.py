#!/usr/bin/env python3
"""Photo finish: low-strength, depth-guided img2img with Segmind-Vega + T2I-Adapter (depth-midas SDXL),
all network inference through ONNX Runtime on CPU. The scheduler loop is plain numpy.

Inputs : --color  512x512 RGB render
         --depth  512x512 grayscale depth, NEAR = WHITE, FAR/background = BLACK (MiDaS convention)
Output : --out    512x512 RGB PNG, plus <out>.timing.json (and one line appended to --log if given)

Example:
  python finish.py --color c.png --depth d.png --out o.png --strength 0.3 --steps 20 --seed 1
Batch (loads models once): --manifest pairs.csv   # lines: color_path,depth_path,out_path

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
        e = ONNX / "embeds"
        self.pe = np.concatenate([np.load(e / "negative_prompt_embeds.npy"), np.load(e / "prompt_embeds.npy")])
        self.pooled = np.concatenate(
            [np.load(e / "negative_pooled_prompt_embeds.npy"), np.load(e / "pooled_prompt_embeds.npy")]
        )
        self.prompt = json.loads((e / "prompt.json").read_text())
        self.sched = EulerScheduler(json.loads(SCHED_CFG.read_text()))
        self.scaling = json.loads(VAE_CFG.read_text())["scaling_factor"]
        self.threads = threads

    def run(self, color_p, depth_p, out_p, strength, steps, seed, guidance, adapter_scale, res):
        T = {}
        t0 = time.time()
        rng = np.random.default_rng(seed)
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
        x = (z0 + sig[0] * rng.standard_normal(z0.shape).astype(np.float32)).astype(np.float32)
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
            step_times.append(time.time() - t)
        T["unet_steps"] = len(ts)
        T["unet_total_s"] = float(sum(step_times))
        T["unet_per_step_s"] = float(np.mean(step_times)) if step_times else 0.0

        t = time.time()
        (im,) = self.vae_dec.run(None, {"latent": (x / self.scaling).astype(np.float32)})
        T["vae_decode_s"] = time.time() - t
        im = ((np.clip(im[0], -1, 1) + 1) * 127.5).round().astype(np.uint8).transpose(1, 2, 0)
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
        }
        return rec


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--color")
    ap.add_argument("--depth")
    ap.add_argument("--out")
    ap.add_argument("--manifest", help="CSV lines color,depth,out (models are loaded once)")
    ap.add_argument("--strength", type=float, default=0.3)
    ap.add_argument("--steps", type=int, default=20, help="scheduler steps; UNet runs int(steps*strength)")
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--guidance", type=float, default=5.0)
    ap.add_argument("--adapter-scale", type=float, default=0.8)
    ap.add_argument("--res", type=int, default=768, help="internal working resolution (multiple of 64)")
    ap.add_argument("--threads", type=int, default=os.cpu_count())
    ap.add_argument("--arena", action="store_true", help="enable ORT CPU memory arena (more RAM)")
    ap.add_argument("--log", help="append one JSON line per image to this file")
    a = ap.parse_args()
    assert a.res % 64 == 0, "--res must be a multiple of 64"
    if a.manifest:
        jobs = [l.strip().split(",") for l in Path(a.manifest).read_text().splitlines() if l.strip()]
    else:
        if not (a.color and a.depth and a.out):
            ap.error("need --color --depth --out (or --manifest)")
        jobs = [(a.color, a.depth, a.out)]

    f = Finisher(a.threads, a.arena)
    print(f"models loaded in {f.load_s:.1f}s")
    for c, d, o in jobs:
        rec = f.run(c, d, o, a.strength, a.steps, a.seed, a.guidance, a.adapter_scale, a.res)
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
