"""Model provenance register and pipeline cache key (PoC 8, roadmap 2).

  python provenance.py          hash the model files, write models.lock.json

models.lock.json lists every model file the finish loads, with its source,
revision, licence, obligations and SHA-256. `pipeline_key()` hashes that
register together with the prompt set, the finish parameters and the finish
code version, so a cached portrait is reused only when nothing that could
change its pixels has changed. File hashes are cached by (size, mtime) in
downloads/onnx/.hashes.json so the multi-GB UNet is hashed once.
"""
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ONNX = HERE.parent / "downloads" / "onnx"
LOCK = HERE / "models.lock.json"
HCACHE = ONNX / ".hashes.json"

OPENRAIL = ("CreativeML Open RAIL++-M (SDXL lineage): pass the Attachment A use restrictions on as an "
            "enforceable EULA clause and ship a copy of the licence; see OPENRAIL_NOTICE.md")

MODELS = [
    {"name": "Segmind-Vega UNet (ONNX export)", "files": ["unet/unet.onnx", "unet/unet.onnx.data"],
     "source": "https://huggingface.co/segmind/Segmind-Vega", "licence": "Apache-2.0 (model card)",
     "obligations": OPENRAIL, "export": "photo_finish/export_onnx.py"},
    {"name": "Segmind-Vega VAE encoder / decoder", "files": ["vae_encoder/vae_encoder.onnx", "vae_decoder/vae_decoder.onnx"],
     "source": "https://huggingface.co/segmind/Segmind-Vega", "licence": "Apache-2.0 (model card)",
     "obligations": OPENRAIL, "export": "photo_finish/export_onnx.py"},
    {"name": "T2I-Adapter depth-midas SDXL", "files": ["adapter/adapter.onnx"],
     "source": "https://huggingface.co/TencentARC/t2i-adapter-depth-midas-sdxl-1.0", "licence": "Apache-2.0",
     "obligations": "SDXL-conditioned; treated with the same OpenRAIL++ pass-through", "export": "photo_finish/export_onnx.py"},
]


def file_hash(p: Path, cache: dict) -> str:
    st = p.stat()
    k = f"{p}:{st.st_size}:{int(st.st_mtime)}"
    if k not in cache:
        h = hashlib.sha256()
        with open(p, "rb") as fh:
            for chunk in iter(lambda: fh.read(1 << 24), b""):
                h.update(chunk)
        cache[k] = h.hexdigest()
    return cache[k]


def build_lock():
    cache = json.loads(HCACHE.read_text()) if HCACHE.exists() else {}
    out = []
    for m in MODELS:
        files = {}
        for f in m["files"]:
            p = ONNX / f
            if p.exists():
                files[f] = file_hash(p, cache)
        out.append({**m, "sha256": files})
    HCACHE.write_text(json.dumps(cache, indent=1))
    LOCK.write_text(json.dumps({"models": out}, indent=1) + "\n")
    return out


def pipeline_key(embeds: str, params: dict, version: str) -> str:
    lock = json.loads(LOCK.read_text()) if LOCK.exists() else {"models": build_lock()}
    h = hashlib.sha256()
    h.update(json.dumps(lock, sort_keys=True).encode())
    e = ONNX / embeds
    for f in sorted(e.glob("*")):
        h.update(f.name.encode())
        h.update(hashlib.sha256(f.read_bytes()).digest())
    h.update(json.dumps(params, sort_keys=True).encode())
    h.update(version.encode())
    return h.hexdigest()[:16]


if __name__ == "__main__":
    for m in build_lock():
        print(m["name"], {k: v[:12] for k, v in m["sha256"].items()})
    print("wrote", LOCK)
