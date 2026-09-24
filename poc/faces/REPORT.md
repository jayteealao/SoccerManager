# Regen faces that age: proof-of-concept report

These are proofs of concept only. The crate is standalone (`poc/faces`, with its own `[workspace]`) and touches nothing outside `poc/faces`. Model weights and downloaded assets live in `downloads/`, which git ignores; the scripts that fetch them are committed.

## How to reproduce

```sh
cd poc/faces
./scripts/fetch_makehuman.sh                       # MakeHuman CC0 mesh and targets, pinned commit
cargo test --release                               # PoC 1 tests
cargo run --release --bin genomes > out/genomes.json
cargo run --release --bin age_sheet -- --finish-inputs   # PoC 2 (+ PoC 4 inputs)
cargo run --release --bin hair_sheet               # PoC 3
cargo run --release --bin measure [-- --fit]       # proportion check / refit calibration
# PoC 4 (setup in photo_finish/README.md):
downloads/venv/bin/python photo_finish/finish.py --manifest out/tmp/pairs.csv --strength 0.2 --steps 30 --adapter-scale 0.9 --log out/poc4_timings.jsonl
downloads/venv/bin/python photo_finish/export_onnx.py prompt_sets   # v2 prompt embeddings
cargo run --release --bin finish_sheet
# PoC 5 (Anny; see LICENSES.md): git clone https://github.com/naver/anny downloads/anny-src && downloads/venv/bin/pip install -e downloads/anny-src
cargo run --release --bin anny_sheet -- landmarks
ANNY_CACHE_DIR=downloads/anny-cache downloads/venv/bin/python anny/anny_fit.py out/tmp/anny
cargo run --release --bin anny_sheet -- jobs --anny-calibrated --face-age
ANNY_CACHE_DIR=downloads/anny-cache downloads/venv/bin/python anny/anny_heads.py out/tmp/anny
cargo run --release --bin anny_sheet -- render
# PoC 6 (FLAME; place FLAME2023Open.zip unpacked at assets/flame/open/):
downloads/venv/bin/python flame/flame_transfer.py
cargo run --release --bin measure -- --fit --flame
cargo run --release --bin flame_sheet -- ladder
cargo run --release --bin flame_sheet -- ages
downloads/venv/bin/python photo_finish/finish.py --manifest out/tmp/flame/pairs.csv --strength 0.2 --steps 30 --adapter-scale 0.9
cargo run --release --bin flame_sheet -- compare
downloads/venv/bin/python photo_finish/landmarks2d.py out/tmp/finish_inputs out/tmp/finished
```

On a machine without a GPU, install Mesa lavapipe (`apt install mesa-vulkan-drivers`). wgpu then renders through software Vulkan.

## What each proof of concept showed

### PoC 1: the genome and ancestry. **Passes.**

- `src/genome.rs`, `src/pools.rs` and `src/rng.rs` define the genome: 24 shape coefficients, skin tone as a continuous value plus undertone, hair type, hair colour (darkness and red), hairline (height, recession, onset), greying (onset and span), eye colour and shade, beard density, fat tendency, birth year, the ancestry mix, and a MakeHuman macro mix.
- **Determinism.** Generation is deterministic from a `u64` seed. Each gene reads its own forked SplitMix64 stream, so adding a gene later does not change the genes that already exist. The generator is written in the crate rather than taken from `rand`, so a dependency upgrade cannot change what a seed means.
- **Pools and nations.** There are four sample gene pools, A to D. Every number in them is **invented** and labelled as invented in the code. The three nations are fictional.
- **How genes draw from pools.** Continuous genes blend the pools by ancestry share. Discrete genes pick one ancestral pool, weighted by share. Curl is treated as polygenic: each pool contributes a draw and the draws are blended, so an A/B mix tends to get curly or wavy hair.
- **Sample output.** `out/genomes.json` holds 20 genomes each for 3 nations. It shows single-pool, two-pool and trace three-pool mixes.
- **Tests.** `tests/genome.rs` checks that:
  - the same seed gives the same genome;
  - a pinned seed is guarded against drift;
  - different seeds differ;
  - the largest JSON over 6,000 genomes is under 1 KB;
  - values stay in range and mixes sum to 1;
  - every nation produces mixed ancestries.

  All 6 pass.

### PoC 2: head, age and render. **Passes, with caveats.**

Images:
- [`out/poc2_age_sheet.jpg`](out/poc2_age_sheet.jpg): 6 genomes (rows) at ages 16, 19, 24, 30, 36, 45, 60 and 70 (columns).
- [`out/poc2/CVD_4_age19.png`](out/poc2/CVD_4_age19.png), [`out/poc2/CVD_4_age70.png`](out/poc2/CVD_4_age70.png), [`out/poc2/CVD_4_age70_depth.png`](out/poc2/CVD_4_age70_depth.png): full-size 512×512 samples.
- [`out/poc2/landmarks_check.png`](out/poc2/landmarks_check.png): the landmark picks used for the proportion check.
- [`out/poc2_measurements.txt`](out/poc2_measurements.txt): the proportion table.

How it works:
- **Mesh.** MakeHuman CC0 base mesh (`src/head.rs`). The head and neck are cut out, and the eyeball helpers are kept.
- **Macro targets.** The genome's MakeHuman ancestry mix is combined with the age classes (child, young, old) and with muscle and weight.
- **Shape genes.** These map to 24 pairs of face targets.
- **Anchor.** Every head is moved so the eyes sit at a fixed point, because macro targets change body height.
- **Age rules** (`src/age.rs`):
  - growth finishes by 21;
  - from the mid-30s, fat moves lower: double chin, jowls, eye bags, thinner upper cheeks and laugh lines;
  - greying runs from the genome's onset over its span, and the onset comes from the ancestry blend, mid-30s to mid-40s;
  - the hairline recedes, with crown thinning when recession is strong;
  - forehead lines and crow's feet deepen in the shader.
- **Render.** Off-screen wgpu 30 (`src/render.rs`, `src/shader.wgsl`), 4× MSAA, with a colour target plus a depth target (near is white). It ran on **Mesa lavapipe (llvmpipe, software Vulkan)**, at about 0.3 s per 512×512 portrait including mesh build and readback.

**Proportions.** Uncorrected MakeHuman heads looked off. I measured them against published adult male norms (`src/measure.rs`, `bin/measure`):

| Measure | Uncorrected (mm) | Norm (mm) |
|---|---|---|
| Mouth width | 38–46 | 54.5 |
| Nose base to chin | 63–69 | 72.6–77.5 |
| Jaw width | 104–114 | 97–100 |
| Hairline to nasion | 79–86 | 67–72 |

- **Calibration.** A bounded least-squares fit (`--fit`) writes `src/calibration.rs`: per-ancestry target weights that move neutral heads to the norms. It also sets each shape gene's range from the population SD, so one gene SD moves its measure by about one population SD. Afterwards, neutral heads sit within about 1 SD on almost every measure.
- **Passing criterion.** Each row reads as one person who ages from left to right.

Caveats:
- The ages 16, 19 and 24 look alike.
- Faces vary less than real faces do.
- The skin shading is basic.

### PoC 3: hair cards. **Passes for the coiled styles; straight and wavy are weaker.**

Images:
- [`out/poc3_hair_sheet.jpg`](out/poc3_hair_sheet.jpg): 4 styles (short straight, medium wavy, tight fade, short twists) across 3 skin tones, same head, age 26.
- [`out/poc3/tight_fade_closeup.png`](out/poc3/tight_fade_closeup.png) and [`out/poc3/short_twists_closeup.png`](out/poc3/short_twists_closeup.png): full-size close-ups.

`src/cards.rs` and `src/hair.rs` generate all geometry and textures in code:
- **Straight and wavy.** Classic hair cards. Each ribbon is walked over the scalp along a growth field that flows from the crown whorl, collides with an offset surface, has gravity, and optionally waves. The strand alpha texture is generated in code. Its red channel carries a per-strand id, so greying is salt-and-pepper per strand.
- **Coiled hair has its own methods:**
  - **Fade and short curly** use 16–24 stacked shells. The shader places one helix per jittered 3D cell, then draws the ring of that helix at each shell's height. Coils clump into naps and lean with the growth direction. A fade profile shortens the hair to skin on the sides and back.
  - **Twists** are a short shell base plus tubes. Each tube is transported along a drooping, collision-aware centreline and shaded with a two-strand helical groove.
- **Result.** The fade and the twists read as coiled hair, not as flat cards.

Weaknesses:
- The wavy cards show a grey haze at the fringe. It needs mip-mapped textures and sorted blending.
- Straight cards look spiky at the hairline.
- Coils are close to pixel size at portrait framing, so they read as texture rather than as individual coils.

### PoC 4: photo finish. **v2 passes on "same person" and "same age" for these 18 faces. v1 failed on both.**

Images:
- [`out/poc4_finish_pairs.jpg`](out/poc4_finish_pairs.jpg): v2 before and after pairs for 6 faces at ages 19, 30 and 60.
- [`out/poc4_v1_vs_v2.jpg`](out/poc4_v1_vs_v2.jpg): render, v1 and v2 side by side at ages 30 and 60.
- [`out/poc4_strength_sweep.jpg`](out/poc4_strength_sweep.jpg): the v1 strength sweep (0.3, 0.2 and 0.15).
- [`out/poc4/`](out/poc4/): full-size pairs.
- [`out/poc4_timings.jsonl`](out/poc4_timings.jsonl): per-image timings, prompt and settings.

Pipeline (`photo_finish/`):
- **Models.** Segmind Vega plus the TencentARC T2I-Adapter depth-midas SDXL model. The depth input comes from our renderer, not from MiDaS.
- **ONNX Runtime on CPU:** the UNet (with 4 adapter residual inputs), the adapter, and the VAE encoder and decoder.
- **Other steps.** The Euler img2img loop runs in numpy. The prompt embeddings were computed once with torch.

**v1, the brief's fixed prompt, at strength 0.3:**
- It kept pose, outline and hair.
- It made 60-year-olds look about 45.
- It pulled the East-Asian-like and mixed faces towards European features.
- It repainted brown eyes grey-blue.

**v2 tunes the input to Vega in two places.**

1. **The render.** MediaPipe Face Landmarker (Apache-2.0; `photo_finish/landmarks2d.py`) measured 2D proportions on each render and on its v1 finish. The corrections Vega made consistently across the 18 pairs showed where the renders were off:

   | Measure | Vega's change | Pairs agreeing | Our 2D value | Norm |
   |---|---|---|---|---|
   | Brow height above eye | −40% | 100% | | |
   | Mouth width | −4% | 83% | 0.633 | 0.598 |
   | Lower face | +6% | 89% | | |
   | Nose height | −8% | 100% | | |

   What was changed in the render as a result:
   - Brows were moved down and made fuller.
   - The 3D mouth-corner landmark was re-picked to include the visible corner. The mouth calibration had overshot because of it, and the refit lowered it.
   - The inner-canthus term was dropped from the fit, because the 3D landmark disagrees with the 2D detector. The outer eye span now drives eye placement, and it lands within the norms.
   - A dark lash line was added, so the eye opening reads as an eye.

   After these changes, Vega's corrections shrank:

   | Measure | Before | After |
   |---|---|---|
   | Brow | −40% | −6.7% |
   | Mouth | −4% | −0.2% |
   | Lower face | +6% | +1.7% |
   | Nose height | −8% | −1.5% |

   Our renders now match the 2D norms for mouth width (0.606 against 0.598) and lower face (0.796 against 0.796). Full numbers are in [`out/poc4_landmarks_v2.txt`](out/poc4_landmarks_v2.txt).
2. **The conditioning.** There are 12 precomputed prompt sets (`photo_finish/prompts.py`): age band (19, 30, 60) × eye colour from the genome. For age 60, the prompt names wrinkles, eye bags and jowls, and the negative prompt names "young, smooth skin". Ancestry is never put in the prompt. Strength drops from 0.3 to 0.2, and the adapter scale rises from 0.8 to 0.9.

What v2 achieves:
- At 60, faces now read as about 60, with wrinkles, eye bags and sagging.
- KSI 2 keeps its East Asian eye shape at 30 and 60.
- The mixed faces stay close to their renders.
- Eye colour is kept.

What is still weak:
- The upper lip is slightly thick. Vega thins it by about 10%, with 67% of pairs agreeing.
- Some finishes add stubble.
- The fixed seed means every face shares the same noise.

**Timing (v2):** 31.5 s per image on average (range 30.5–37.9 s), 6 UNet evaluations at about 3.4 s each, peak RSS 4.7 GB.
- Hardware: Intel Xeon @ 2.80 GHz (Cascade Lake), 4 vCPU, 15 GB RAM, onnxruntime 1.30, fp32, no GPU.
- **These times do not predict consumer PCs.**

Suggested next steps:
- Close the loop automatically: render, finish, landmark, adjust.
- Mask the finish to the skin.
- Test an identity metric that is not InsightFace, whose weights are non-commercial.

### PoC 5: Anny as the head generator. **Works as a drop-in, with no clear visual gain; worth it as an engineering base.**

Images:
- [`out/poc5_makehuman_vs_anny.jpg`](out/poc5_makehuman_vs_anny.jpg): our MakeHuman-direct heads against Anny heads for the same genomes at ages 19, 30 and 60.
- [`out/poc5_anny_age_sheet.jpg`](out/poc5_anny_age_sheet.jpg): the full Anny age sheet.
- [`out/poc5_finish_compare.jpg`](out/poc5_finish_compare.jpg): the MakeHuman-direct v2 finish, the Anny render and the Anny finish, at ages 30 and 60.
- [`out/poc5_anny_age_sheet_native.jpg`](out/poc5_anny_age_sheet_native.jpg) and [`out/poc5_makehuman_vs_anny_native.jpg`](out/poc5_makehuman_vs_anny_native.jpg): Anny with its own WHO age mapping and no facial calibration.
- Measurements: [`out/poc5_measurements.txt`](out/poc5_measurements.txt) and [`out/poc5_measurements_native.txt`](out/poc5_measurements_native.txt).

**What Anny is.** [Anny](https://github.com/naver/anny) (NAVER LABS, Apache-2.0) is a differentiable PyTorch model built on MakeHuman's CC0 data (via MPFB2). It has:
- 6 overall controls (gender, age, muscle, weight, height, proportions) plus african, asian and caucasian weights;
- 254 signed local targets, 137 of them on the face and neck;
- UVs, a rig, and CC0 facial expressions.

No registration is needed. Its optional "smplx" and "smpl" topologies download non-commercial data; they are never used here.

**Integration.** The genome and the renderer are unchanged. Only the head generator changes.
1. `anny_sheet jobs` writes each genome and age as Anny inputs: ancestry, muscle, weight, and the shape and age targets by MakeHuman name.
2. `anny/anny_heads.py` evaluates Anny at 0.03 s per head on CPU. It converts metres with Z up to MakeHuman decimetres with Y up; that mapping fits exactly (rms 1.5e-7). It writes the vertices in base-mesh indexing, using Anny's `base_mesh_vertex_indices`.
3. `anny_sheet render` cuts, masks, adds hair and renders exactly as PoC 2 does.

`head-oval` and `head-square` have no Anny label and are skipped.

Findings:
1. **Same data, same look.** At the same weights, Anny heads are nearly identical to our MakeHuman heads at 19 and 30. The underlying targets are the same MakeHuman assets, in a newer (MPFB2) release.
2. **Anny's age mapping is for bodies, not faces.** It is fitted to WHO height-for-age, so a 24-year-old gets Anny age 0.78, which is half "old". From 30 to 70 the value moves only from 0.786 to 0.852, so faces barely age. We keep our own face-age rule, converted to Anny's linear age scale (child 1/9, young 5/9, old 1).
3. **Calibration does not transfer between versions.** Our MakeHuman calibration applied to Anny left lower faces about 10 mm short, because the MPFB2 targets and body controls differ from MakeHuman 1.1.
4. **Differentiable calibration.** `anny/anny_fit.py` fits the facial calibration by gradient descent through Anny: Adam, 250 steps, about 13 s per ancestry. It uses the same landmark definitions, exported from Rust with `anny_sheet landmarks`, and the same norms.
   - Unsourced measures get a weak pull (one-third weight) towards the North American value. Without it, the African-like jaw drifted to 115 mm.
   - After the fit, calibrated Anny heads sit at the norms about as closely as the MakeHuman-direct heads, with slightly narrower jaws.
5. **Ageing is a little weaker.** At 60, Anny heads show less jowl and neck fullness than the MakeHuman-direct ones, because Anny's weight and muscle controls act differently on the head. The age-aware finish still reads as about 60.
6. **Finish timing:** 31.4 s per image on average (range 30.5–37.7 s), the same pipeline as PoC 4 v2. Details are in `out/poc5_finish_timings.jsonl`.

**Where the unhuman look comes from** ([`out/poc5_ladder.jpg`](out/poc5_ladder.jpg)). Anny heads were built up one layer at a time and rendered in plain grey clay, so geometry is judged without shading. The stages are: Anny default, then ancestry, then facial calibration, then shape genes, then adult structure, then the full look.
- **Overall proportions are human.** Measured on these heads:

  | Ratio | Ours | Published |
  |---|---|---|
  | Skull breadth | 138–149 mm | about 140–151 mm |
  | Length-to-breadth index | 72–80 | about 76–80 |
  | Face width / skull width | 0.90–0.97 | about 0.91 |
  | Jaw width / face width | 0.71–0.79 | about 0.71 |

- **What reads as unhuman is detail.** The MakeHuman mesh that Anny wraps is an idealised, smoothed face: no brow ridge, flat cheek planes, a soft jaw, and no fold from nose to mouth corner. My placeholder shading adds to it: painted brows, flat lips, uniform skin, and eyeballs without a cornea or lid thickness.
- **Adult structure helps a little but does not fix it.** That stage adds a forward brow ridge, cheekbones, slight lower-lid fullness and a small nasal hump.
- **Conclusion.** Tuning targets on this mesh, whether through MakeHuman or Anny, will not make raw renders read as human. That needs:
  - scan-derived facial form, such as FLAME 2023 Open, which is CC-BY-4.0 but needs registration;
  - scanned skin detail;
  - a real eye model;
  - or leaning on the photo finish, which does make the faces read as human (PoC 4 v2).

**Should we use it?** For portrait quality, Anny adds nothing over using the MakeHuman targets directly. Its value is as an engineering base:
- a maintained, permissively licensed package;
- exact gradients for calibration and for fitting shape spaces to data;
- batched evaluation;
- UVs, a rig and expressions for later 3D use;
- WHO-calibrated body sampling for the match engine's bodies.

**Recommended pattern:**
- Use Anny offline, to calibrate and fit.
- Bake the resulting blendshapes into our own runtime format, so the game never needs PyTorch.
- Keep our face-age rule, and keep the facial calibration tied to the exact asset version.

### PoC 6: FLAME 2023 Open on the MakeHuman mesh. **Clear gain in facial structure; this is the base to keep.**

Images:
- [`out/poc6_flame_ladder.jpg`](out/poc6_flame_ladder.jpg): four heads in grey clay, then the full look. Columns: MakeHuman with hand-made shape genes, then plus FLAME's mean form, then plus FLAME identity, then the full look.
- [`out/poc6_flame_age_sheet.jpg`](out/poc6_flame_age_sheet.jpg): the six-genome age sheet with FLAME.
- [`out/poc6_finish_compare.jpg`](out/poc6_finish_compare.jpg): the MakeHuman v2 finish, the FLAME render and the FLAME finish, at ages 30 and 60.
- [`out/poc6_measurements.txt`](out/poc6_measurements.txt): measurements.

**Source.** FLAME 2023 Open, CC-BY-4.0, confirmed from the readme inside the zip. It is a statistical head model learned from real 3D scans: 5,023 vertices, 300 shape directions and 100 expression directions. The person registered and downloaded it; I received it through their Google Drive.

Only `flame2023_Open.pkl` is used. `FLAME_masks.zip` is not used because its licence is unconfirmed.

**How it works** (`flame/flame_transfer.py`, then `src/flame.rs`). FLAME is transferred onto the MakeHuman topology rather than replacing it, so eyes, hair, masks, age rules, the jersey and the Vega finish all work unchanged.
1. **Alignment.** Umeyama similarity from four landmarks (both eye centres, nose tip, chin), refined by ICP on the face. The result is scale 9.79 from metres to decimetres and a 4.6° tilt.
2. **Embedding.** Each MakeHuman head vertex is attached to its nearest FLAME triangle, with barycentric coordinates.
3. **Two layers, applied as pseudo-targets:**
   - `flame/conform` moves MakeHuman's base head onto FLAME's mean head. This adds the brow ridge, cheek planes and jaw definition that MakeHuman's idealised mesh lacks.
   - `flame/beta0..99` carry FLAME's first 100 shape directions onto our vertices. A genome draws its betas from a standard normal on its own seeded stream. These replace the hand-made shape genes.
4. **Keeping the two meshes compatible** took most of the work:
   - FLAME has holes at the eyes, so each eye zone keeps MakeHuman's lids and eyeball and moves them as one unit with the ring of face around them.
   - Interior surfaces (mouth cavity, nostrils) are carried along by diffusion and kept beneath FLAME's skin.
   - Offsets that disagree with their neighbours, or that flip triangles, are relaxed.
   - Both offset fields are low-pass filtered: FLAME's value is medium-scale structure, and its fine crease detail clashes with MakeHuman's folds.
   - At build time, FLAME is faded out locally wherever a triangle turns more than 60° against the same head built without FLAME. That is where FLAME's base-mesh offsets conflict with the ancestry and age targets. The 12 sheet heads, ages 24 and 60, have no remaining folds.
5. **Calibration.** FLAME changes proportions, so it has its own fitted table: `measure --fit --flame` writes `src/calibration_flame.rs`. Neutral FLAME heads sit at the norms. The genome-driven identities spread realistically across the six test faces: nose width 30–42 mm, face height 113–135 mm.

**Result.**
- In clay, FLAME heads read as adult men, where MakeHuman heads read as mannequins, and identities differ much more.
- Each row of the age sheet still reads as one person ageing.
- One tiny speck of a pixel or two remains beside the left nostril crease in close-ups. It is not visible at portrait size.

## What failed or was blocked, and why

- **FLAME 2023 Open** needs registration, which the rules forbid me to do. The person registered, downloaded it, and handed it over through Google Drive; it is used in PoC 6. The separate `FLAME_masks.zip` is not used, because its licence is unconfirmed.
- **No reference photos.** Wikimedia Commons refused direct file downloads from this container (an error page instead of the image). The proportion work uses published anthropometric numbers instead, and no images of real people are used or committed.
- **No GPU.** Rendering ran on Mesa lavapipe, installed with apt, outside the repo. The PoC 4 timings are CPU-only.
- **v1 of the photo finish drifted in age and ancestry.** v2 fixes this for these 18 faces by tuning the render and the prompt (see PoC 4). It departs from the brief's single fixed prompt: v2 uses a fixed prompt per age band and eye colour.
- **The first head proportions were wrong:** shape genes were over-scaled, and two landmark picks were initially wrong. Both were fixed and checked with rendered landmark markers.

## Licences confirmed, with sources

The full record is in [`LICENSES.md`](LICENSES.md).

| Item | Licence | Where confirmed |
|---|---|---|
| MakeHuman base mesh and targets (commit `a8bc2d5`) | CC0 1.0 | `LICENSE.md` §C and `LICENSE.ASSETS.md` in https://github.com/makehumancommunity/makehuman, plus each file's header ("explicitly released as CC0 in september 2020") |
| segmind/Segmind-Vega | Apache-2.0 | Card YAML, https://huggingface.co/segmind/Segmind-Vega (not gated) |
| TencentARC/t2i-adapter-depth-midas-sdxl-1.0 | Apache-2.0 | Card YAML and card body, https://huggingface.co/TencentARC/t2i-adapter-depth-midas-sdxl-1.0 (not gated) |
| Rust crates (82 resolved) | MIT / Apache-2.0 / BSD / Zlib, except `libloading` (ISC) and `unicode-ident` (adds Unicode-3.0) | `cargo metadata`; table in `LICENSES.md` |
| Mesa lavapipe (system, not redistributed) | MIT | Ubuntu package `mesa-vulkan-drivers` 25.2.8 |
| Python packages for PoC 4 (venv, not committed) | Mostly MIT/Apache/BSD; exceptions listed below | Package metadata; see `photo_finish/NOTES.md` |
| FLAME 2023 Open | CC-BY-4.0; cite Li et al., ACM ToG 2017 | The readme inside `FLAME2023Open.zip`, and https://flame.is.tue.mpg.de/modellicense.html |
| Anny (code and bundled MPFB2 data) | Apache-2.0 code; CC0 MakeHuman/MPFB2 data | Repository `LICENSE` and README licence section, https://github.com/naver/anny (commit `ee5b909`) |
| Facial norms | Published numbers, cited | Wamalwa et al. 2019, https://pmc.ncbi.nlm.nih.gov/articles/PMC6384287/ |

The `jpeg-encoder` crate (which adds an IJG term) was replaced with a small in-crate baseline encoder (`src/jpeg.rs`) to stay inside the allowed set.

## Open questions for you

1. **Licences outside the list that cannot be avoided without dropping wgpu:**
   - `libloading` (ISC), which wgpu uses to load the GPU driver;
   - `unicode-ident` (Unicode-3.0), used at build time only.

   Both are permissive. Can they be added to the allowed list?
2. **PoC 4 Python exceptions:**
   - pillow (MIT-CMU), which `finish.py` uses at run time;
   - certifi and tqdm (MPL-2.0), typing_extensions (PSF-2.0), shellingham (ISC) and regex (part CNRI-Python), used at fetch and export time only.

   Can they be accepted, or should `finish.py` drop pillow (for example by writing PNGs with numpy and zlib)?
3. **Vega's provenance.** Its card says it was distilled from SDXL (Open RAIL++-M) and two community models, trained partly on a Midjourney scrape, and bundles SDXL-like encoders and a VAE. Is its Apache-2.0 label acceptable to your legal review, or should the finish use a model with cleaner lineage?
4. **Prompt per age and eye colour.** v2 uses 12 fixed prompt sets instead of one, which fixed the age and eye-colour drift. Is that acceptable against the brief's "fixed prompt"?
5. **Ancestry drift.** v2 holds ancestry for these faces at strength 0.2. A wider test across many more genomes is needed before trusting it. Should that test be the next step?
6. **FLAME as the base (PoC 6).** Should FLAME become the identity source, with MakeHuman kept only for ancestry and age targets, topology, and attachments? That needs the CC-BY attribution (the FLAME paper) credited wherever the game credits third-party assets.
7. **Face variety.** Proportions are now calibrated, but faces vary less than real ones do. Should the next step add more shape genes (eye spacing, lip shape, brow ridge, cheek fat)? Or should it fit a proper shape space to measured data, which would need a licensed dataset?
8. **East Asian norms.** I found no sourced norms, so for that ancestry only face height and mouth width are constrained. Do you have a preferred source?
9. **Anny as the base.** Anny does not improve portraits over the MakeHuman targets used directly, but it is a better engineering base: differentiable, maintained, with UVs, rig and expressions. Should the next step bake Anny's blendshapes into a Rust runtime format, and use Anny only offline for calibration?
10. **The invented gene pools** need real, reviewed numbers before any use beyond this proof of concept. Who should own that review?
