# Photo finish: notes (proof of concept 4)

Status: the pipeline runs end to end on CPU through ONNX Runtime. It has run on the 18 real head renders from proof of concept 2; the results and a strength sweep are in `../REPORT.md` (PoC 4 section). The timing table below comes from the earlier synthetic runs, which shared the CPU with builds; the clean timings on the real renders are in the report.

## What runs where

| Component | Runtime | File (under `poc/faces/downloads/onnx/`) |
|---|---|---|
| Vega UNet with 4 extra T2I-Adapter residual inputs `r0..r3` | **ONNX Runtime** (fp32, opset 17, external data) | `unet/unet.onnx` + `unet.onnx.data` (2.98 GB) |
| T2I-Adapter depth-midas SDXL | **ONNX Runtime** | `adapter/adapter.onnx` |
| VAE encoder (returns mean and logvar; sampling happens in numpy) | **ONNX Runtime** | `vae_encoder/vae_encoder.onnx` |
| VAE decoder | **ONNX Runtime** | `vae_decoder/vae_decoder.onnx` |
| Both CLIP text encoders | **torch, once at export time**. The prompt is fixed, so the embeddings are saved as `.npy` | `embeds/*.npy`, `embeds/prompt.json` |
| Euler scheduler, CFG, img2img noising | numpy in `finish.py` | n/a |

Nothing is blocked. The full UNet exported with the TorchScript exporter (`torch.onnx.export(..., dynamo=False)`). The export took 58 s, plus about 30 s to consolidate the external data, with a peak RSS of 4.8 GB. H, W and batch are dynamic axes.

`export_onnx.py --check` compared ORT with torch, using shapes different from the export shapes:
- UNet at 768: max abs diff 3e-6
- adapter: max abs diff 1e-4 or less
- VAE decoder: max abs diff 4e-6
- VAE encoder mean: max abs diff 6e-3 on random input, where the largest value is 13

### Adapter wiring

I read the wiring from the installed diffusers 0.40.0 source, not from memory:
- `UNet2DConditionModel.forward(..., down_intrablock_additional_residuals=[r0,r1,r2,r3])`.
- r0 (320 ch, latent/2) is added after `down_blocks[0]`.
- r1 (640 ch, latent/2) is added inside `down_blocks[1]` before its downsampler.
- r2 (1280 ch, latent/4) is added inside `down_blocks[2]`.
- r3 (1280 ch, latent/4) is added after the mid block.
- `StableDiffusionXLAdapterPipeline` multiplies every adapter output by `adapter_conditioning_scale`. `finish.py` does the same, with a default of 0.8.
- The adapter input is the depth image repeated to 3 channels and scaled to [0,1], with no normalisation, as in `_preprocess_adapter_image`.

### Scheduler

The Euler scheduler is reproduced from Vega's `scheduler/scheduler_config.json`:
- `EulerDiscreteScheduler`, `scaled_linear` betas from 0.00085 to 0.012
- 1000 training timesteps, epsilon prediction
- `timestep_spacing: leading`, `steps_offset: 1`, linear sigma interpolation, final sigma 0

The img2img strength logic is copied from diffusers' `StableDiffusionXLImg2ImgPipeline.get_timesteps` and `add_noise`:
- `init_timestep = int(steps * strength)`
- the start index is `steps - init_timestep`
- `x = z0 * 0.13025 + sigma_start * noise`

Time IDs are `(res, res, 0, 0, res, res)`. CFG runs as a batch of 2 (negative prompt, then prompt), with guidance 5.0.

Prompt: "studio headshot photograph of a professional footballer, natural skin texture, soft studio lighting, neutral grey background, sharp focus".
Negative prompt: "cartoon, 3d render, cgi, painting, illustration, plastic skin, waxy, doll, blurry, deformed, distorted face, extra eyes, makeup, text, watermark, lowres".

## Hardware and timings

Hardware:
- `lscpu`: Intel(R) Xeon(R) Processor @ 2.80GHz (family 6, model 85, stepping 7, which is Cascade Lake with AVX-512 VNNI)
- 4 cores, 1 thread per core, KVM guest
- 15 GB RAM, no swap
- onnxruntime 1.30.0 with CPUExecutionProvider, 4 intra-op threads, all fp32

**Caveat:** other agents' `cargo`/`rustc` builds ran on the same 4 cores during every measurement, with a load average of about 3.8. The numbers are therefore noisy and pessimistic: the same 512 run measured between 32 s and 49 s. These times do not predict consumer PCs.

All runs used strength 0.3, 20 scheduler steps (6 UNet evaluations) and CFG at batch 2. Model load takes a further 8 s once per process.

| Internal res | s / image | UNet s / step | VAE enc | VAE dec | Adapter | Peak RSS |
|---|---|---|---|---|---|---|
| 512 | **32 – 49** | 3.4 – 5.3 | 3 – 7 s | 8 – 10 s | 0.2 s | 4.6 GB (arena off) / 5.6 GB (arena on) |
| 768 | 80 – 86 | 8.6 – 9.0 | 9 – 10 s | 19 – 21 s | 0.5 s | 5.7 GB (arena off) |
| 1024 | 217 | 21.6 | 32 s | 54 s | 0.9 s | 7.4 GB (arena off); **OOM-killed at 13.8 GB with the ORT memory arena on** |

These choices follow from the timings:
- `finish.py` turns the ORT CPU memory arena off by default. `--arena` turns it back on.
- The default internal resolution is **512**. On the synthetic pair, 512 and 768 look comparable at strength 0.3, and 1024 takes 4 to 7 times longer with no clear gain.
- The lead should compare 512 and 768 on real renders (`--res 768`). A synthetic blob cannot judge skin quality.
- At strength 0.4 and 512, the synthetic face drifted towards a stylised, painted look. Start in the range 0.25 to 0.3.

The test outputs are in `poc/faces/downloads/test/` (not committed):
- `test_color.png` and `test_depth.png` are the synthetic test pair.
- `final_r512.png` and `final_r768.png` are finished images.
- Each output has a `*.timing.json`, and `timings.jsonl` holds the log.
- `cmp2.png` shows, left to right: input, 512, 768, 1024, and 512 at strength 0.4.

## Licences confirmed from the model cards

Both licences come from the YAML front matter of the model card. `fetch_models.py` saved each card as `README.md` next to its weights and printed the value. Neither repo is gated, and no login or token was used.

- **segmind/Segmind-Vega**: `license: apache-2.0`. See https://huggingface.co/segmind/Segmind-Vega (card: `downloads/hf/Segmind-Vega/README.md`).
- **TencentARC/t2i-adapter-depth-midas-sdxl-1.0**: `license: apache-2.0`, and the card body says "**License:** Apache 2.0". See https://huggingface.co/TencentARC/t2i-adapter-depth-midas-sdxl-1.0 (card: `downloads/hf/t2i-adapter-depth-midas-sdxl-1.0/README.md`).

Files downloaded (fp16 safetensors, upcast to fp32 at export):
- Vega: `unet/`, `vae/`, `text_encoder/`, `text_encoder_2/`, `tokenizer*/`, `scheduler/`, `model_index.json`
- adapter: `diffusion_pytorch_model.fp16.safetensors`, `config.json`

## Open questions on provenance (not resolved)

1. **Vega's lineage.** The card says Vega is "a distilled version of the Stable Diffusion XL (SDXL)" that "leverages the teachings of several expert models, including SDXL, ZavyChromaXL, and JuggernautXL". It was trained on `zzliang/GRIT` and `wanng/midjourney-v5-202304-clean`.
   - SDXL base is licensed under CreativeML Open RAIL++-M.
   - ZavyChromaXL and JuggernautXL are community fine-tunes with their own terms.
   - The Midjourney scrape dataset raises its own terms-of-use question.
   - Whether Apache-2.0 on the distilled weights is sound, or whether RAIL++-M use restrictions carry over, is an open question.
2. **Bundled text encoders and VAE.** Vega's `model_index.json` has `_name_or_path: SSD-Tiny`, and `vae/config.json` has `_name_or_path: SSD-Tiny/vae`. Their architecture matches SDXL's CLIP ViT-L and OpenCLIP ViT-bigG encoders and the SDXL VAE (scaling factor 0.13025). They are very likely the SDXL components or derived from them. SDXL is RAIL++-M; the OpenAI CLIP weights are MIT and OpenCLIP is MIT, but their training data (LAION) is a separate question. I have not verified that they are byte-identical.
3. **T2I-Adapter's lineage.** It was trained on top of SDXL. The config says `_name_or_path: valhalla/t2i-depth-midas`. The card's example uses a MiDaS detector from `controlnet_aux`. We do not use MiDaS; our depth comes from the renderer.

## Python packages installed (venv at `poc/faces/downloads/venv`)

I read the licences from the package metadata (`License-Expression`, `License` or the classifiers).

**Direct installs:**

| Package | Licence |
|---|---|
| torch 2.14.0+cpu | BSD-3-Clause (metadata: Apache-2.0 AND Apache-2.0 WITH LLVM-exception AND BSD-2-Clause AND BSD-3-Clause AND BSL-1.0 AND MIT, covering bundled components) |
| diffusers 0.40.0 | Apache-2.0 |
| transformers 5.17.0 | Apache-2.0 |
| huggingface_hub 1.33.0 | Apache-2.0 |
| safetensors 0.8.0 | Apache-2.0 |
| onnx 1.23.0 | Apache-2.0 |
| onnxruntime 1.30.0 | MIT |
| numpy 2.4.6 | BSD-3-Clause (with bundled 0BSD, MIT, Zlib, CC0-1.0) |
| pillow 12.3.0 | **MIT-CMU** (HPND-style permissive, not literally MIT/BSD) |

**Transitive dependencies:**
- MIT: annotated-doc 0.0.5, anyio 4.15.1, charset-normalizer 3.5.1, filelock 3.32.3, h11 0.16.0, markdown-it-py 4.2.0, mdurl 0.1.2, PyYAML 6.0.3, rich 15.0.0, typer 0.27.2, urllib3 2.8.0, zipp 4.1.0, pip 26.2.1, setuptools 79.0.1
- BSD: click 8.5.0, fsspec 2026.7.0, httpcore 1.0.9, httpx 0.28.1, idna 3.20, Jinja2 3.1.6, MarkupSafe 3.0.3, mpmath 1.3.0, networkx 3.6.1, protobuf 7.36.2, Pygments 2.21.0, sympy 1.14.0
- Apache-2.0: flatbuffers 25.12.19, hf-xet 1.6.0, importlib_metadata 9.0.1, ml_dtypes 0.6.0, requests 2.34.2, tokenizers 0.23.2
- Mixed: packaging 26.3 (Apache-2.0 OR BSD-2-Clause), regex 2026.9.10 (Apache-2.0 AND CNRI-Python)
- **Outside the MIT/Apache/BSD list, flagged:**
  - certifi 2026.7.22 (MPL-2.0)
  - tqdm 4.70.1 (MPL-2.0 AND MIT)
  - typing_extensions 4.16.0 (PSF-2.0)
  - shellingham 1.5.4 (ISC)
  - regex's CNRI-Python part
  - pillow (MIT-CMU)

All the flagged packages are pulled in by huggingface_hub, transformers, diffusers or torch, and they are used only at fetch or export time. The exception is pillow, which `finish.py` uses for PNG input and output. At run time `finish.py` needs only onnxruntime, numpy and pillow, plus their dependencies flatbuffers, protobuf, packaging and sympy.

## Disk

- `downloads/hf`: 3.3 GB (fp16 weights)
- `downloads/onnx`: 3.4 GB (fp32)
- `downloads/venv`: 1.4 GB

Total: about 8.2 GB. `poc/faces/.gitignore` already ignores `downloads/`, which `git check-ignore` confirms.
