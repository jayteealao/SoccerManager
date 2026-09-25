# Model provenance register

Machine-readable version: `models.lock.json` (SHA-256 per file; regenerate with `python photo_finish/provenance.py`). Weights are never committed.

| Model | Used for | Source | Licence | Obligations / status |
|---|---|---|---|---|
| Segmind-Vega UNet, VAE | photo finish (runtime) | huggingface.co/segmind/Segmind-Vega | Apache-2.0 card, SDXL lineage | OpenRAIL++ pass-through accepted; see OPENRAIL_NOTICE.md |
| T2I-Adapter depth-midas SDXL | photo finish depth control | huggingface.co/TencentARC/t2i-adapter-depth-midas-sdxl-1.0 | Apache-2.0 | treated like Vega |
| GNM Head v3.0 (npz + identity decoder) | head shape and identity sampler | github.com/google/GNM | Apache-2.0 | shipped (derived arrays) |
| MediaPipe Face Landmarker | evaluation (landmark drift) | Google | Apache-2.0 | tooling only |
| DINOv2-small | evaluation (near-duplicates) | huggingface.co/facebook/dinov2-small | Apache-2.0 | tooling only |
| SigLIP 2 base patch16-224 | evaluation (apparent age) | huggingface.co/google/siglip2-base-patch16-224 | Apache-2.0 | tooling only |
| YuNet 2023mar | evaluation (face detection) | github.com/opencv/opencv_zoo | MIT | tooling only |
| SFace 2021dec | evaluation (identity) | github.com/opencv/opencv_zoo | Apache-2.0 label; training-data provenance unknown | **internal QA only, never shipped** (signed off) |
