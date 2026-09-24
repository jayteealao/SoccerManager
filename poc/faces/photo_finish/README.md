# Photo finish (proof of concept 4)

A depth-guided image-to-image pass at low strength. It uses Segmind-Vega with the SDXL T2I-Adapter depth-midas model. The UNet, the adapter and the VAE all run through ONNX Runtime on CPU. The scheduler loop is plain numpy. See `NOTES.md` for results, timings and licences.

All weights, ONNX files, the venv and test outputs go in `poc/faces/downloads/`. Never commit that folder.

## Setup (one time, about 8 GB of disk, 15 GB of RAM is enough)

```sh
cd poc/faces
python3 -m venv downloads/venv
downloads/venv/bin/pip install torch --index-url https://download.pytorch.org/whl/cpu
downloads/venv/bin/pip install diffusers transformers huggingface_hub safetensors onnx onnxruntime numpy pillow
downloads/venv/bin/python photo_finish/fetch_models.py      # about 3.3 GB, prints license: from each model card
downloads/venv/bin/python photo_finish/export_onnx.py       # about 2 min, peak about 5 GB RSS
downloads/venv/bin/python photo_finish/export_onnx.py --check   # optional: compares ORT with torch
```

`finish.py` itself needs only `onnxruntime`, `numpy` and `pillow`. It does not need torch.

## Run

```sh
downloads/venv/bin/python photo_finish/finish.py --color render.png --depth depth.png --out finished.png \
    [--strength 0.3] [--steps 20] [--seed 0] [--guidance 5.0] [--adapter-scale 0.8] [--res 512] [--log timings.jsonl]
# many images, loading the models once (saves about 8 s per image):
downloads/venv/bin/python photo_finish/finish.py --manifest pairs.csv --log timings.jsonl   # each line: color,depth,out
```

- `--color` takes an RGB PNG, normally 512x512.
- `--depth` takes a grayscale PNG of the same framing. NEAR is WHITE and FAR or background is BLACK (the MiDaS convention).
- The output PNG has the input's size. Each output also gets `<out>.timing.json`, and `--log` appends one JSON line per image.
- UNet evaluations per image = `int(steps * strength)`. The defaults give 6.
- Synthetic test pair: `downloads/venv/bin/python photo_finish/make_test_pair.py --out-dir downloads/test`
