#!/usr/bin/env python3
"""Export Segmind-Vega + T2I-Adapter (depth-midas SDXL) to ONNX for CPU inference.

Outputs go to poc/faces/downloads/onnx/:
  unet/unet.onnx (+ unet.onnx.data)   UNet2DConditionModel with 4 extra adapter-residual inputs
  adapter/adapter.onnx                T2IAdapter (full_adapter_xl): depth RGB [0,1] -> 4 residual maps
  vae_encoder/vae_encoder.onnx        image [-1,1] -> latent moments (mean, logvar)
  vae_decoder/vae_decoder.onnx        latents (already / scaling_factor) -> image [-1,1]
  embeds/*.npy                        prompt + negative-prompt embeddings, precomputed ONCE in torch
                                      (both CLIP text encoders run here, not in ORT: the prompt is fixed)

Every component is exported in its own subprocess so peak RAM stays at one model (UNet ~3 GB fp32,
peak ~8-9 GB while tracing). Weights are the fp16 safetensors upcast to fp32 (CPU runs fp32).

Adapter wiring was read from the installed diffusers source (0.40.0):
  UNet2DConditionModel.forward(..., down_intrablock_additional_residuals=[r0, r1, r2, r3])
  r0: added after down_blocks[0] (DownBlock2D, after its downsampler)     320 ch @ latent/2
  r1: added inside down_blocks[1] before its downsampler                  640 ch @ latent/2
  r2: added inside down_blocks[2] (no downsampler)                        1280 ch @ latent/4
  r3: added after mid_block (only if shapes match)                        1280 ch @ latent/4
  and StableDiffusionXLAdapterPipeline multiplies each adapter output by adapter_conditioning_scale.

Usage:
  python export_onnx.py                # all components
  python export_onnx.py unet adapter   # selected components
  python export_onnx.py --check        # compare each ONNX model vs torch on random inputs
"""
import argparse
import gc
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
DL = HERE.parent / "downloads"
VEGA = DL / "hf" / "Segmind-Vega"
ADAPTER = DL / "hf" / "t2i-adapter-depth-midas-sdxl-1.0"
OUT = DL / "onnx"

PROMPT = (
    "studio headshot photograph of a professional footballer, natural skin texture, "
    "soft studio lighting, neutral grey background, sharp focus"
)
NEGATIVE = (
    "cartoon, 3d render, cgi, painting, illustration, plastic skin, waxy, doll, blurry, "
    "deformed, distorted face, extra eyes, makeup, text, watermark, lowres"
)
OPSET = 17
COMPONENTS = ["embeds", "adapter", "vae_encoder", "vae_decoder", "unet"]


def _torch():
    import torch

    torch.set_grad_enabled(False)
    return torch


def export_embeds(prompt=None, negative=None, name="embeds", pipe=None):
    torch = _torch()
    from diffusers import StableDiffusionXLPipeline

    prompt = prompt or PROMPT
    negative = negative or NEGATIVE
    pipe = pipe or StableDiffusionXLPipeline.from_pretrained(
        VEGA, unet=None, vae=None, variant="fp16", torch_dtype=torch.float32
    )
    pe, npe, pooled, npooled = pipe.encode_prompt(
        prompt=prompt,
        negative_prompt=negative,
        device="cpu",
        num_images_per_prompt=1,
        do_classifier_free_guidance=True,
    )
    d = OUT / name
    d.mkdir(parents=True, exist_ok=True)
    import numpy as np

    np.save(d / "prompt_embeds.npy", pe.numpy().astype(np.float32))
    np.save(d / "negative_prompt_embeds.npy", npe.numpy().astype(np.float32))
    np.save(d / "pooled_prompt_embeds.npy", pooled.numpy().astype(np.float32))
    np.save(d / "negative_pooled_prompt_embeds.npy", npooled.numpy().astype(np.float32))
    (d / "prompt.json").write_text(json.dumps({"prompt": prompt, "negative_prompt": negative}, indent=2))
    print(name, "embeds:", pe.shape, npe.shape, pooled.shape, npooled.shape)
    return pipe


def export_prompt_sets():
    """Embeddings for every set in prompts.py, into downloads/onnx/embeds_<name>/."""
    import sys

    sys.path.insert(0, str(Path(__file__).parent))
    from prompts import prompt_sets

    pipe = None
    for name, p, n in prompt_sets():
        pipe = export_embeds(p, n, "embeds_" + name, pipe)


def _export(model, args, path, input_names, output_names, dynamic_axes):
    torch = _torch()
    path.parent.mkdir(parents=True, exist_ok=True)
    t = time.time()
    torch.onnx.export(
        model,
        args,
        str(path),
        dynamo=False,  # TorchScript exporter: no extra deps; handles >2GB via external data
        opset_version=OPSET,
        input_names=input_names,
        output_names=output_names,
        dynamic_axes=dynamic_axes,
        do_constant_folding=True,
    )
    print(f"exported {path.name} in {time.time() - t:.0f}s")


def _consolidate_external(path: Path):
    """Re-save so all weights sit in one <name>.onnx.data file (TorchScript writes one file per tensor)."""
    import onnx

    m = onnx.load(str(path), load_external_data=True)
    for f in path.parent.iterdir():
        if f != path:
            f.unlink()
    onnx.save_model(
        m,
        str(path),
        save_as_external_data=True,
        all_tensors_to_one_file=True,
        location=path.name + ".data",
        size_threshold=1024,
    )


def export_adapter():
    torch = _torch()
    from diffusers import T2IAdapter

    ad = T2IAdapter.from_pretrained(ADAPTER, variant="fp16", torch_dtype=torch.float32).eval()

    class W(torch.nn.Module):
        def __init__(self, a):
            super().__init__()
            self.a = a

        def forward(self, x):
            return tuple(self.a(x))

    x = torch.rand(1, 3, 512, 512)
    dyn = {"depth": {0: "b", 2: "h", 3: "w"}}
    for i in range(4):
        dyn[f"r{i}"] = {0: "b", 2: f"h{i}", 3: f"w{i}"}
    _export(W(ad), (x,), OUT / "adapter" / "adapter.onnx", ["depth"], [f"r{i}" for i in range(4)], dyn)


def _vae():
    torch = _torch()
    from diffusers import AutoencoderKL

    return AutoencoderKL.from_pretrained(VEGA / "vae", variant="fp16", torch_dtype=torch.float32).eval()


def export_vae_encoder():
    torch = _torch()
    vae = _vae()

    class W(torch.nn.Module):
        def __init__(self, v):
            super().__init__()
            self.v = v

        def forward(self, x):
            h = self.v.quant_conv(self.v.encoder(x))
            mean, logvar = torch.chunk(h, 2, dim=1)
            return mean, logvar

    x = torch.rand(1, 3, 512, 512) * 2 - 1
    _export(W(vae), (x,), OUT / "vae_encoder" / "vae_encoder.onnx", ["image"], ["mean", "logvar"],
            {"image": {0: "b", 2: "h", 3: "w"}, "mean": {0: "b", 2: "lh", 3: "lw"},
             "logvar": {0: "b", 2: "lh", 3: "lw"}})


def export_vae_decoder():
    torch = _torch()
    vae = _vae()

    class W(torch.nn.Module):
        def __init__(self, v):
            super().__init__()
            self.v = v

        def forward(self, z):
            return self.v.decoder(self.v.post_quant_conv(z))

    z = torch.randn(1, 4, 64, 64)
    _export(W(vae), (z,), OUT / "vae_decoder" / "vae_decoder.onnx", ["latent"], ["image"],
            {"latent": {0: "b", 2: "lh", 3: "lw"}, "image": {0: "b", 2: "h", 3: "w"}})


class UNetWithAdapter:
    """Factory for a torch wrapper exposing adapter residuals as plain positional inputs."""

    @staticmethod
    def build(unet):
        torch = _torch()

        class W(torch.nn.Module):
            def __init__(self, u):
                super().__init__()
                self.u = u

            def forward(self, sample, timestep, encoder_hidden_states, text_embeds, time_ids, r0, r1, r2, r3):
                return self.u(
                    sample,
                    timestep,
                    encoder_hidden_states=encoder_hidden_states,
                    added_cond_kwargs={"text_embeds": text_embeds, "time_ids": time_ids},
                    down_intrablock_additional_residuals=[r0, r1, r2, r3],  # list is pop()'d inside
                    return_dict=False,
                )[0]

        return W(unet)


def _unet():
    torch = _torch()
    from diffusers import UNet2DConditionModel

    return UNet2DConditionModel.from_pretrained(VEGA / "unet", variant="fp16", torch_dtype=torch.float32).eval()


def _unet_dummy(b=2, lh=64, lw=64):
    torch = _torch()
    return (
        torch.randn(b, 4, lh, lw),
        torch.tensor([500.0]),
        torch.randn(b, 77, 2048),
        torch.randn(b, 1280),
        torch.tensor([[512.0, 512.0, 0.0, 0.0, 512.0, 512.0]] * b),
        torch.randn(b, 320, lh // 2, lw // 2) * 0.1,
        torch.randn(b, 640, lh // 2, lw // 2) * 0.1,
        torch.randn(b, 1280, lh // 4, lw // 4) * 0.1,
        torch.randn(b, 1280, lh // 4, lw // 4) * 0.1,
    )


UNET_INPUTS = ["sample", "timestep", "encoder_hidden_states", "text_embeds", "time_ids", "r0", "r1", "r2", "r3"]


def export_unet():
    unet = _unet()
    w = UNetWithAdapter.build(unet)
    args = _unet_dummy()
    gc.collect()
    dyn = {
        "sample": {0: "b", 2: "lh", 3: "lw"},
        "encoder_hidden_states": {0: "b"},
        "text_embeds": {0: "b"},
        "time_ids": {0: "b"},
        "r0": {0: "b", 2: "h2", 3: "w2"},
        "r1": {0: "b", 2: "h2", 3: "w2"},
        "r2": {0: "b", 2: "h4", 3: "w4"},
        "r3": {0: "b", 2: "h4", 3: "w4"},
        "noise_pred": {0: "b", 2: "lh", 3: "lw"},
    }
    path = OUT / "unet" / "unet.onnx"
    _export(w, args, path, UNET_INPUTS, ["noise_pred"], dyn)
    del w, unet
    gc.collect()
    _consolidate_external(path)


def check():
    """Numerically compare ORT vs torch for each exported model (fresh random inputs, second shape too)."""
    import numpy as np
    import onnxruntime as ort

    torch = _torch()
    so = ort.SessionOptions()

    def run(p, feeds):
        s = ort.InferenceSession(str(p), so, providers=["CPUExecutionProvider"])
        return s.run(None, feeds)

    def rep(name, a, b):
        a, b = np.asarray(a), np.asarray(b)
        print(f"  {name}: max|diff|={np.abs(a - b).max():.2e}  max|ref|={np.abs(b).max():.2e}")

    from diffusers import T2IAdapter

    ad = T2IAdapter.from_pretrained(ADAPTER, variant="fp16", torch_dtype=torch.float32).eval()
    x = torch.rand(1, 3, 768, 768)
    ref = ad(x)
    got = run(OUT / "adapter" / "adapter.onnx", {"depth": x.numpy()})
    print("adapter @768:")
    for i in range(4):
        rep(f"r{i} {tuple(got[i].shape)}", got[i], ref[i].numpy())
    del ad
    vae = _vae()
    x = torch.rand(1, 3, 256, 256) * 2 - 1
    ref = vae.encode(x).latent_dist
    got = run(OUT / "vae_encoder" / "vae_encoder.onnx", {"image": x.numpy()})
    print("vae_encoder @256:")
    rep("mean", got[0], ref.mean.numpy())
    z = torch.randn(1, 4, 32, 32)
    ref = vae.decode(z).sample
    got = run(OUT / "vae_decoder" / "vae_decoder.onnx", {"latent": z.numpy()})
    print("vae_decoder @256:")
    rep("image", got[0], ref.numpy())
    del vae
    gc.collect()
    unet = _unet()
    w = UNetWithAdapter.build(unet)
    args = _unet_dummy(2, 96, 96)
    ref = w(*args).numpy()
    del w, unet
    gc.collect()
    got = run(OUT / "unet" / "unet.onnx", {n: a.numpy() for n, a in zip(UNET_INPUTS, args)})
    print("unet @768 (latent 96):")
    rep("noise_pred", got[0], ref)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("components", nargs="*", default=COMPONENTS)
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--inproc", action="store_true", help=argparse.SUPPRESS)
    a = ap.parse_args()
    if a.check:
        check()
        return
    if a.inproc:
        for c in a.components:
            globals()[f"export_{c}"]()
        return
    for c in a.components:
        if c not in COMPONENTS + ["prompt_sets"]:
            sys.exit(f"unknown component {c}; choose from {COMPONENTS} or prompt_sets")
        print(f"=== {c}")
        t = time.time()
        subprocess.run([sys.executable, __file__, "--inproc", c], check=True)
        import resource

        peak = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss / 1e6  # KB -> GB (largest child so far)
        print(f"=== {c} done in {time.time() - t:.0f}s (peak child RSS so far {peak:.1f} GB)")


if __name__ == "__main__":
    main()
