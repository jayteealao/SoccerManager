#!/usr/bin/env python3
"""Fetch only the files the photo-finish POC needs, into poc/faces/downloads/hf/.

- segmind/Segmind-Vega             (Apache-2.0, ungated)  -> fp16 safetensors per component
- TencentARC/t2i-adapter-depth-midas-sdxl-1.0 (Apache-2.0, ungated) -> fp16 safetensors

fp16 weights are downloaded to halve disk/bandwidth; export_onnx.py upcasts to fp32 for CPU.
Each model card README.md is saved next to the weights and its `license:` field is printed.
No login, no token, no gated repos.

Usage:  python fetch_models.py
"""
import re
import sys
from pathlib import Path

from huggingface_hub import hf_hub_download

HERE = Path(__file__).resolve().parent
HF_DIR = HERE.parent / "downloads" / "hf"

REPOS = {
    "segmind/Segmind-Vega": [
        "README.md",
        "model_index.json",
        "scheduler/scheduler_config.json",
        "text_encoder/config.json",
        "text_encoder/model.fp16.safetensors",
        "text_encoder_2/config.json",
        "text_encoder_2/model.fp16.safetensors",
        "tokenizer/merges.txt",
        "tokenizer/special_tokens_map.json",
        "tokenizer/tokenizer_config.json",
        "tokenizer/vocab.json",
        "tokenizer_2/merges.txt",
        "tokenizer_2/special_tokens_map.json",
        "tokenizer_2/tokenizer_config.json",
        "tokenizer_2/vocab.json",
        "unet/config.json",
        "unet/diffusion_pytorch_model.fp16.safetensors",
        "vae/config.json",
        "vae/diffusion_pytorch_model.fp16.safetensors",
    ],
    "TencentARC/t2i-adapter-depth-midas-sdxl-1.0": [
        "README.md",
        "config.json",
        "diffusion_pytorch_model.fp16.safetensors",
    ],
}


def card_license(readme: Path) -> str:
    text = readme.read_text(encoding="utf-8", errors="replace")
    m = re.match(r"^---\n(.*?)\n---", text, re.S)
    front = m.group(1) if m else ""
    lic = re.search(r"^license:\s*(.+)$", front, re.M)
    return lic.group(1).strip() if lic else "<no license: field in card front matter>"


def main() -> int:
    HF_DIR.mkdir(parents=True, exist_ok=True)
    for repo, files in REPOS.items():
        local = HF_DIR / repo.split("/")[1]
        print(f"== {repo} -> {local}")
        for f in files:
            p = hf_hub_download(repo_id=repo, filename=f, local_dir=local)
            print(f"   {f}  ({Path(p).stat().st_size / 1e6:.1f} MB)")
        lic = card_license(local / "README.md")
        print(f"   model card license: {lic}   (https://huggingface.co/{repo})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
