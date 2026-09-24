# Licences — regen faces proofs of concept

The allowed set is MIT, Apache-2.0, BSD, zlib, CC0 and CC-BY-4.0. Anything outside
that set is marked **OUTSIDE** below and is also listed as an open question in `REPORT.md`.

## Downloaded assets (fetched by script, never committed)

| Asset | Source | Licence | Confirmed from |
|---|---|---|---|
| MakeHuman base mesh `base.obj` and targets (`macrodetails`, `head`, `chin`, `nose`, `mouth`, `cheek`, `eyebrows`, `forehead`, `ears`, `neck`, `eyes`) | https://github.com/makehumancommunity/makehuman at commit `a8bc2d54ff0ac92e78ff71431b1023eda42bf482`, via `scripts/fetch_makehuman.sh` | CC0 1.0 | `LICENSE.md` section C ("These assets have been released under CC0 1.0 Universal"), `LICENSE.ASSETS.md` (full CC0 text), and the header of every file ("explicitly released as CC0 in september 2020"). The MakeHuman *code* is AGPL; this PoC uses none of it. |
| FLAME 2023 Open | not downloaded | needs registration | Blocked by the brief's rules; see REPORT.md |

PoC 4 model weights are listed in the PoC 4 section below.

## Rust crates (PoCs 1-3)

The table was generated from `cargo metadata` for x86_64 Linux. Every crate that `Cargo.lock` resolves for that target is listed.

Code in this crate that replaces a dependency:
- `src/rng.rs` implements SplitMix64, a public-domain algorithm. It keeps genome seeds stable across dependency upgrades.
- `src/jpeg.rs` is a baseline JPEG encoder written from ITU-T T.81, using that standard's Annex K tables. It replaces the `jpeg-encoder` crate, whose licence adds an IJG term.

Crates outside the allowed set. All three reach the build only through `wgpu` and `serde`:
- **OUTSIDE:** `libloading` (ISC). `wgpu-hal` uses it to load the Vulkan or GL driver at run time. ISC reads the same as MIT, but ISC is not on the allowed list.
- **OUTSIDE (partly):** `unicode-ident` is `(MIT OR Apache-2.0) AND Unicode-3.0`. It is a build-time dependency of the proc-macros used by `serde_derive` and `thiserror`. Its Unicode data tables carry the Unicode-3.0 licence.

| Crate | Version | Licence (SPDX, from Cargo metadata) | Direct |
|---|---|---|---|
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |  |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 |  |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 |  |
| ash | 0.38.0+1.3.281 | MIT OR Apache-2.0 |  |
| autocfg | 1.5.1 | Apache-2.0 OR MIT |  |
| bit-set | 0.10.0 | Apache-2.0 OR MIT |  |
| bit-vec | 0.9.1 | Apache-2.0 OR MIT |  |
| bitflags | 2.13.2 | MIT OR Apache-2.0 |  |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT | yes |
| bytemuck_derive | 1.12.1 | Zlib OR Apache-2.0 OR MIT |  |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |  |
| cfg_aliases | 0.2.2 | MIT |  |
| codespan-reporting | 0.13.1 | Apache-2.0 |  |
| crc32fast | 1.5.2 | MIT OR Apache-2.0 |  |
| dlib | 0.5.3 | MIT |  |
| document-features | 0.2.12 | MIT OR Apache-2.0 |  |
| equivalent | 1.0.2 | Apache-2.0 OR MIT |  |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 |  |
| flate2 | 1.1.10 | MIT OR Apache-2.0 |  |
| foldhash | 0.2.0 | Zlib |  |
| glam | 0.33.9 | MIT OR Apache-2.0 | yes |
| glow | 0.17.0 | MIT OR Apache-2.0 OR Zlib |  |
| gpu-allocator | 0.28.0 | MIT OR Apache-2.0 |  |
| half | 2.7.1 | MIT OR Apache-2.0 |  |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 |  |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |  |
| indexmap | 2.14.2 | Apache-2.0 OR MIT |  |
| itoa | 1.0.18 | MIT OR Apache-2.0 |  |
| khronos-egl | 6.0.0 | MIT/Apache-2.0 |  |
| libc | 0.2.189 | MIT OR Apache-2.0 |  |
| libloading | 0.8.9 | ISC |  |
| libm | 0.2.16 | MIT |  |
| litrs | 1.0.0 | MIT OR Apache-2.0 |  |
| lock_api | 0.4.14 | MIT OR Apache-2.0 |  |
| log | 0.4.34 | MIT OR Apache-2.0 |  |
| memchr | 2.8.3 | Unlicense OR MIT |  |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 |  |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 |  |
| naga | 30.0.1 | MIT OR Apache-2.0 |  |
| naga-types | 30.0.1 | MIT OR Apache-2.0 |  |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |  |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |  |
| ordered-float | 5.5.0 | MIT |  |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 |  |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 |  |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 |  |
| png | 0.18.1 | MIT OR Apache-2.0 | yes |
| pollster | 1.0.1 | Apache-2.0/MIT | yes |
| presser | 0.3.1 | MIT OR Apache-2.0 |  |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |  |
| profiling | 1.0.18 | MIT OR Apache-2.0 |  |
| quote | 1.0.47 | MIT OR Apache-2.0 |  |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib |  |
| renderdoc-sys | 1.1.0 | MIT OR Apache-2.0 |  |
| rustc-hash | 1.1.0 | Apache-2.0/MIT |  |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 |  |
| serde | 1.0.229 | MIT OR Apache-2.0 | yes |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |  |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |  |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | yes |
| simd-adler32 | 0.3.10 | MIT |  |
| smallvec | 1.16.1 | MIT OR Apache-2.0 |  |
| spirv | 0.4.0+sdk-1.4.341.0 | Apache-2.0 |  |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 |  |
| syn | 2.0.119 | MIT OR Apache-2.0 |  |
| syn | 3.0.6 | MIT OR Apache-2.0 |  |
| termcolor | 1.4.1 | Unlicense OR MIT |  |
| thiserror | 2.0.21 | MIT OR Apache-2.0 |  |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 |  |
| unicode-ident | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 |  |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 |  |
| wayland-sys | 0.31.11 | MIT |  |
| wgpu | 30.0.1 | MIT OR Apache-2.0 | yes |
| wgpu-core | 30.0.1 | MIT OR Apache-2.0 |  |
| wgpu-core-deps-windows-linux-android | 30.0.1 | MIT OR Apache-2.0 |  |
| wgpu-hal | 30.0.1 | MIT OR Apache-2.0 |  |
| wgpu-naga-bridge | 30.0.1 | MIT OR Apache-2.0 |  |
| wgpu-types | 30.0.1 | MIT OR Apache-2.0 |  |
| zerocopy | 0.8.58 | BSD-2-Clause OR Apache-2.0 OR MIT |  |
| zerocopy-derive | 0.8.58 | BSD-2-Clause OR Apache-2.0 OR MIT |  |
| zlib-rs | 0.6.8 | Zlib |  |
| zmij | 1.0.23 | MIT |  |

## System software used at run time (not redistributed)

| Software | Licence | Note |
|---|---|---|
| Mesa lavapipe (`mesa-vulkan-drivers` 25.2.8, Ubuntu) | MIT | Software Vulkan driver used for the headless renders. It is installed with apt and is not part of the crate. |

## PoC 4: model weights (fetched by `photo_finish/fetch_models.py` into `downloads/`, never committed)

| Model | Licence on the model card | Source | Note |
|---|---|---|---|
| segmind/Segmind-Vega | `license: apache-2.0` (card YAML front matter) | https://huggingface.co/segmind/Segmind-Vega | Not gated. **Open provenance question**: the card says Vega is distilled from SDXL (CreativeML Open RAIL++-M), ZavyChromaXL and JuggernautXL, and that it was trained on GRIT and a Midjourney scrape. Its bundled text encoders and VAE look like SDXL's own. |
| TencentARC/t2i-adapter-depth-midas-sdxl-1.0 | `license: apache-2.0` (card YAML front matter, and the card body reads "License: Apache 2.0") | https://huggingface.co/TencentARC/t2i-adapter-depth-midas-sdxl-1.0 | Not gated. It was trained on top of SDXL. We feed it our renderer's depth, not MiDaS. |

No account, token or licence click-through was used. Both cards are saved next to the weights in `downloads/hf/*/README.md`.

## PoC 4: Python packages (venv in `downloads/venv`, never committed)

The full list with versions is in `photo_finish/NOTES.md`. Each licence was read from the package metadata.

| Needed for | Packages | Licence |
|---|---|---|
| Running `finish.py` | onnxruntime | MIT |
| | numpy | BSD-3-Clause |
| | flatbuffers; protobuf; packaging | Apache-2.0; BSD-3-Clause; Apache-2.0 OR BSD-2-Clause |
| | sympy | BSD |
| | **pillow** | **OUTSIDE: MIT-CMU** (HPND-style permissive) |
| Fetch and export only | torch (CPU) | BSD-3-Clause with bundled Apache/MIT/BSL parts |
| | diffusers, transformers, huggingface_hub, safetensors, onnx | Apache-2.0 |
| | **certifi** | **OUTSIDE: MPL-2.0** |
| | **tqdm** | **OUTSIDE: MPL-2.0 AND MIT** |
| | **typing_extensions** | **OUTSIDE: PSF-2.0** |
| | **shellingham** | **OUTSIDE: ISC** |
| | **regex** | **OUTSIDE (part): CNRI-Python** |

## Reference data (numbers only, no images)

| Data | Source | Use |
|---|---|---|
| Adult male facial norms (Farkas NAW; Kenyan and African American means and SDs) | Wamalwa et al. 2019, https://pmc.ncbi.nlm.nih.gov/articles/PMC6384287/; Farkas norms (zy-zy 137, n-gn 121.3) | Only the published means and SDs are cited, in `src/measure.rs`. No data files were downloaded. |
