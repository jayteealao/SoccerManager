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
# PoC 7 (FLAME on its own; also unpack FLAME_masks.zip to assets/flame/masks/):
downloads/venv/bin/python flame/flame_prep.py
cargo run --release --bin flame7_sheet -- identity
cargo run --release --bin flame7_sheet -- ages
cargo run --release --bin flame7_sheet -- hair
downloads/venv/bin/python photo_finish/finish.py --manifest out/tmp/flame7/pairs.csv --strength 0.2 --steps 30 --adapter-scale 0.9
cargo run --release --bin flame7_sheet -- compare
downloads/venv/bin/python photo_finish/landmarks2d.py out/tmp/finish_inputs out/tmp/finished
# PoC 8 (GNM Head, Apache-2.0; git clone --depth 1 https://github.com/google/GNM downloads/GNM):
downloads/venv/bin/python gnm/gnm_prep.py --check        # asset + decoder check against Keras
downloads/venv/bin/python gnm/gnm_norms.py               # sampler vs Farkas / Fang, landmark picks
downloads/venv/bin/python gnm/issa_skin.py               # ISSA face-skin statistics -> data/
cargo test --release                                     # includes the sourced-genome tests
cargo run --release --bin gnm8_sheet -- identity         # also: classes ages hair eyes mst family
downloads/venv/bin/python photo_finish/provenance.py     # model hashes -> models.lock.json
downloads/venv/bin/python photo_finish/finish.py --manifest out/tmp/gnm8/pairs.csv --strength 0.2 --adapter-scale 0.9 --cache downloads/finish_cache
cargo run --release --bin gnm8_sheet -- compare
downloads/venv/bin/python eval/evaluate.py               # evaluation harness v1
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

### PoC 7: FLAME on its own, with FLAME's masks. **The most human-looking base so far; ancestry is its weak point.**

Images:
- [`out/poc7_flame_identity.jpg`](out/poc7_flame_identity.jpg): six genomes at age 24, in clay and in the full look.
- [`out/poc7_flame_age_sheet.jpg`](out/poc7_flame_age_sheet.jpg): the age sheet.
- [`out/poc7_flame_hair_sheet.jpg`](out/poc7_flame_hair_sheet.jpg): four hair styles across three skin tones.
- [`out/poc7_finish_pairs.jpg`](out/poc7_finish_pairs.jpg): render and finish pairs for six faces at 19, 30 and 60.
- [`out/poc7_finish_timings.jsonl`](out/poc7_finish_timings.jsonl): finish timings.

The person asked for FLAME to be used freshly, with no MakeHuman base, and for FLAME's masks to be used. Everything here comes from FLAME 2023 Open plus `FLAME_masks.pkl`. The masks' licence is unstated; they are used at the person's direction and recorded in LICENSES.md.
- **Asset** (`flame/flame_prep.py`, then `downloads/flame_asset/`):
  - FLAME's mean head and eyeballs, scaled to decimetres and anchored at the shared eye point, so the camera and hair code carry over;
  - 100 shape directions;
  - the 14 mask regions as per-vertex bits;
  - landmarks for the 13 measures.
- **Identity:** 100 betas per genome, drawn from the seed (SD 0.85) around a per-ancestry mean. That mean is fitted by ridge least squares to the published norms, using FLAME's linearity; no MakeHuman targets are involved.
  - FLAME's surface landmarks for face width and jaw width sit at the edge of the soft-tissue mask, not at bony zygion and gonion, so they read wide against the norms. They are not comparable, and are noted rather than forced.
- **Masks used:**
  - the scalp mask sets the resting hairline, which recedes with age;
  - face, ear and neck exclude hair;
  - the lip mask is feathered and tints the lips;
  - the forehead mask places forehead lines;
  - the shoulder band becomes the jersey;
  - the lash line comes from head vertices on the eyeball surface. FLAME's lids are closed surfaces wrapped round the eyeball, with no holes.
- **Eyes:** FLAME's mean has wide-open lids and a 14.3 mm eyeball. The upper-lid band is rotated about each eyeball centre, by 11° plus up to 5° in late life, so the lid covers the top of the iris. The iris angle is set for a real ~11.7 mm iris.
- **Ageing** (rule-based, because FLAME has no age space and no permissive ageing data is available). All regions come from FLAME's masks and landmarks and are feathered over the mesh to avoid seams.
  - Jowls drop and spread, and the front of the neck fills.
  - Eye bags form, and the cheek beside the nasolabial fold fills.
  - Lips thin toward the stomion.
  - The nose and ears grow slightly.
  - Cheeks are fuller at 15–20.
  - Upper lids droop a little more late in life.
  - Skin creases, greying, recession and the age-aware finish add the rest.
- **Photo finish:** the PoC 4 v2 settings (age and eye-colour prompts, strength 0.2, adapter 0.9). 30.6 s per image on average (range 29.7–37.5 s) on the same 4-vCPU Xeon.

Result:
- In clay and in full, FLAME heads read as real people, with far more natural structure than MakeHuman or Anny.
- After the finish, the faces look photographic. Each person keeps their identity across 19, 30 and 60, age reads correctly, and eye colour holds.

Weak points:
- **Ancestry.** It comes only from anthropometric means. Proportions carry some of it, but eye-region shape does not. The East-Asian-like genome (KSI 2) reads fairly European, both before and after the finish. There are no sourced East Asian norms. A licensed FLAME-topology dataset with ancestry labels would fix this, but those found (LYHM, FaceWarehouse, BP4D+) are non-commercial.
- **Ageing geometry is modest.** The finish carries much of the visible age.
- **Hair.** Hair reaches down the back of the neck, because FLAME's scalp mask extends low at the nape. The medium-wavy cards are still hazy.
- **Resolution.** FLAME's head is only 3,931 vertices. A subdivision step would help close-ups.

### PoC 8: GNM Head, sourced genome, and roadmap items 1–8. **The best base so far; every roadmap item 1–8 has a working proof.**

Decisions the person made before this PoC:
- adopt GNM Head;
- accept Vega's OpenRAIL++ obligations;
- run the finish at runtime;
- use the FLAME masks as they are;
- legal sign-off given, including for using numbers from CC BY-NC(-ND) articles as facts and for internal-only face-recognition QA.

Images:
- [`out/poc8_gnm_identity.jpg`](out/poc8_gnm_identity.jpg): six genomes at 24, clay and full look.
- [`out/poc8_gnm_classes.jpg`](out/poc8_gnm_classes.jpg): three latents across GNM's four ethnicity conditions. Only the condition changes.
- [`out/poc8_gnm_age_sheet.jpg`](out/poc8_gnm_age_sheet.jpg): six genomes at eight ages.
- [`out/poc8_gnm_eyes.jpg`](out/poc8_gnm_eyes.jpg): eye close-ups without and with the PoC 8 eye model.
- [`out/poc8_gnm_hair_sheet.jpg`](out/poc8_gnm_hair_sheet.jpg): four styles × four hair colours, old hair path (alpha-to-coverage, Kajiya-Kay) against new (stochastic coverage, Marschner).
- [`out/poc8_gnm_mst.jpg`](out/poc8_gnm_mst.jpg): the same face with its skin set to each Monk Skin Tone swatch.
- [`out/poc8_gnm_family.jpg`](out/poc8_gnm_family.jpg): two fathers at 45 and three sons each at 22.
- [`out/poc8_finish_pairs.jpg`](out/poc8_finish_pairs.jpg): render and constrained-finish pairs at 19, 30 and 60.
- Numbers:
  - [`out/poc8_gnm_norms.txt`](out/poc8_gnm_norms.txt): the sampler against published norms;
  - [`out/poc8_eval.txt`](out/poc8_eval.txt) and [`.json`](out/poc8_eval.json): the evaluation harness;
  - [`out/poc8_mst.txt`](out/poc8_mst.txt): skin-model fit per MST swatch;
  - [`out/poc8_finish_timings.jsonl`](out/poc8_finish_timings.jsonl): finish timings.

**1. GNM Head (roadmap 1).**
- `gnm/gnm_prep.py` reads `gnm_head.npz` and the identity decoder `.h5` from google/GNM (Apache-2.0). It writes plain arrays to `downloads/gnm_asset/`:
  - 17,821 vertices: skin, eyeballs with separate interior and cornea, teeth, tongue;
  - 253 identity components;
  - 46 named vertex groups;
  - iBUG-68 landmarks.
- **No GNM code runs** (running GNM's Python package was refused by the session's safety check). The rest-pose forward pass is linear, and the identity decoder is a 5-layer MLP. Both are reimplemented in `src/gnm.rs`; the numpy version matches Keras to 1e-6.
- **Identity per genome.** Inputs:
  - GNM's condition is [male] + the genome's ancestry shares mapped to GNM's four classes;
  - a 64-d latent and a 253-d residual come from the genome.

  The residual (×0.6) restores within-group spread. The decoder alone gives about half the published coefficient of variation, because a conditional VAE decodes towards the mean. An extra class contrast (×1.0) moves between-group contrasts towards Farkas.
- **Result against norms** (`out/poc8_gnm_norms.txt`, 300 samples per class):
  - all seven between-group contrasts have the same sign as Farkas 2005, at 40–95% of its size (Farkas gaps are inflated by observer effects);
  - within-class CVs are 0.04–0.07 against Fang's 0.04–0.09;
  - nearest-class-mean accuracy is 0.91.
- **Lids.** GNM's template is a scan average with relaxed lids: a 7.2 mm aperture against the usual 9–10 mm. The smallest-norm combination of GNM's own eye expression components opens it by 1.8 mm.
- **Ageing:** PoC 7's rules, moved onto GNM's named regions (infraorbital, cheek, zygomatic, lips, nose, ears) and made gentler at the jowls.
- **Result:**
  - GNM heads in clay look like real people, with far more natural eye regions, noses and lips than FLAME, and much better than MakeHuman or Anny;
  - the classes sheet shows the ancestry condition changing eye-region, nasal and lip shape;
  - the per-person latent still dominates, as the research predicts.

**2. Guardrails (roadmap 2).**
- The finish seed comes from the genome seed and age (manifest column 6), so regeneration is bit-identical.
- `photo_finish/provenance.py` writes `models.lock.json`, which pins every model file by SHA-256. `photo_finish/MODELS.md` is the human-readable register.
- The finish cache key hashes:
  - the lock;
  - the prompt set;
  - the parameters;
  - `FINISH_VERSION`.

  Outputs are cached under `<cache>/<key>/`. Changing any model invalidates the cache; a rerun hits it.
- The Vega OpenRAIL++ obligations are accepted and written up in `photo_finish/OPENRAIL_NOTICE.md`: a draft EULA clause, a licence copy in `photo_finish/licences/`, and the modification notice.
- The FLAME masks stay as they are. PoC 8 does not use them.

**3. Evaluation harness v1 (roadmap 3, `eval/evaluate.py`).** Images only; no race classification. Results on this set:
- **Landmark drift, render → finish** (MediaPipe): 0.7–2.5% on face width, jaw, nose and eye spacing, and 4.8% on eye aperture. The lips move most (11–13% on vermilion height), because the finish redraws lips. That points at the next mask refinement.
- **Colorimetry:**
  - mean |ΔITA| render → finish is 1.1°, with ΔE ≤ 0.8 on the skin mask;
  - the MST bucket is kept in 100% of images;
  - the tone lock shifted L\*a\*b\* by under 1 unit on average.
- **Near-duplicates** (DINOv2-small): 5 cross-genome pairs flagged, all "same skin, same hair style, same age" pairs such as CVD 1 and CVD 4. SFace separates those easily. DINOv2 alone is a coarse duplicate check; pair it with an identity or landmark vector.
- **Apparent age** (SigLIP 2 zero-shot):
  - renders: MAE 9.8 years, r = 0.80;
  - finished: MAE 6.1 years, r = 0.93.

  The finish carries age well. Renders at 19 still read about 27.
- **Identity** (SFace, internal QA only):
  - same genome across ages: 0.90 cosine; 19 against 60: 0.85;
  - different genomes: 0.55; d′ = 3.5;
  - render against finish: 0.78 (SFace's match threshold is 0.36).
- Not done: FairFace KD-DINOv2 distance. It needs the FairFace images, which are not downloaded here.

**4. Constrained finish (roadmap 4).**
- The renderer writes a region mask. R marks skin the finish may change. G marks eyes and B marks hair, brows and lashes, which stay renderer-owned.
- `finish.py` re-noises the latent from the render outside the mask at every step, composites the result over the render with a feathered mask, and locks mean skin CIELAB to the render.
- Eyes, brows, lashes and hair come back pixel-exact. Iris colour cannot drift, and neither can hairline or brows.
- 31.8 s per image on average (22.6–53.1 s) on the 4-vCPU Xeon, with 4 UNet steps (strength 0.2 of 20). A parallel render job slowed some images.

**5. Skin shading (roadmap 5).**
- **Albedo:** a two-chromophore model (eumelanin with a pheomelanin share, and haemoglobin), fitted so it reaches all ten Monk swatches within ΔE76 4.3. There is a unit test.
- **Per genome:** the skin CIELAB is inverted to melanin and blood concentrations. Blood is then raised regionally (cheeks, nose tip, ears, lips) from GNM's regions.
- **Lighting:**
  - pre-integrated subsurface diffuse (Penner 2011), with a LUT computed from d'Eon's six-Gaussian skin profile and per-vertex curvature;
  - diffuse on the smooth normal and specular on the pore-bumped normal;
  - dual-lobe GGX with oiliness from the T-zone;
  - cavity occlusion.
- **Finding:** real genomes, whose skin comes from ISSA measurements (L\* 36–65), render at MST 5–7. The display swatches MST 1–4 (L\* 88–94) are lighter than any measured face skin. Fed in directly, they clip to near-white (strip covers buckets 2, 5, 6, 7, 8, 10).

  MST swatches are display colours, not reflectances. Exposure should therefore be calibrated against photographs, not against the swatches.

**6. Hair fixes (roadmap 6).** A new hair path behind `RenderOpts::stochastic_hair`. The old path is untouched; PoC 7 renders are byte-identical.
- coverage-preserving mipmaps for hair textures (Castaño 2010);
- stochastic per-sample coverage through `@builtin(sample_mask)` instead of hardware alpha-to-coverage, with a random sample set per fragment, layer and pass;
- 8 jittered accumulation passes;
- Marschner R/TT/TRT lobes (Karis 2016 / Frostbite 2019) with melanin absorption from the pbrt-v3 formula.

Result:
- the grey haze on the wavy fringe is gone;
- red and brown come from absorption;
- grey is per-strand salt and pepper.

Coily styles still use the PoC 3 shells, whose grey reads too speckled at 60.

**7. Eye model (roadmap 7).**
- The eye interior (sclera, iris and pupil geometry from GNM) is shaded with:
  - a refracted iris lookup (Snell at n = 1.376 over the anterior chamber depth);
  - a limbal ring that fades with age;
  - collarette and crypts;
  - a sclera that yellows with age.
- The cornea is drawn blended, with Fresnel reflection and a key-light glint.
- An eye-occlusion shell adds lid and lash shadow per fragment, from cubic fits of the lid margins.
- Tear meniscus on the lower lid, a caruncle, and eyelashes as tapered Marschner ribbons along the lid margins.
- `out/poc8_gnm_eyes.jpg` shows the "doll eye" look (bright, shadowless sclera) replaced by shadowed, wet eyes.

**8. Sourced gene pools (roadmap 8, `src/genes.rs`, genome version 2).**
- Six neutral components, K1–K6. Every number is tagged `Sourced` (with citation) or `Assumed` (with the reason).
- **Ancestry:** a Dirichlet draw around nation templates.
- **Two-allele loci per chromosome with local ancestry:**
  - HERC2 rs12913832 (eye colour);
  - MC1R R (red hair);
  - SLC24A5 (skin);
  - EDAR (hair form).

  Frequencies come from 1000 Genomes.
- **Skin:** ISSA face-site CIELAB for men, recomputed here from the raw spectra (`gnm/issa_skin.py` → `data/issa_face_skin.json`, CC BY 4.0). It includes the within-group covariance.
- **Liability thresholds** for curl and baldness.
- **Greying:** onset from Tobin and Paus, with the span fitted to Panhard 2012.
- **Baldness curves** from the Chinese Norwood data and Norwood 1975.
- **Father-to-son inheritance:** Mendelian loci, averaged ancestry, and a face latent with parent-offspring correlation h²/2.
- **Tests** (`cargo test`) check that:
  - allele frequencies reproduce their sources;
  - ISSA skin means and spread reproduce;
  - a 50/50 K1/K2 mix is 12–20% blue-eyed, and F1 sons under 6%, against 25% under PoC 1's pool-picking;
  - 3–25% are half grey at 50;
  - baldness prevalence matches the curve at 35 and 55;
  - the skin model covers MST 1–10;
  - skin R² on ancestry exceeds 0.6.

Weak points and what's next:
- **Hair** is still cards and shells. Strand hair (roadmap 9) is the largest remaining visual gap.
- **The brows** are painted strokes.
- **Renders read older than their age at 19.** This is a lighting and skin-detail issue; the finish corrects most of it.
- **The finish redraws lips.** Add lips to the renderer-owned mask, or feather lower there.
- **GNM's data provenance and consent** section should still be read against the technical report before shipping.

## What failed or was blocked, and why

- **FLAME 2023 Open** needs registration, which the rules forbid me to do. The person registered, downloaded it, and handed it over through Google Drive; it is used in PoC 6. The separate `FLAME_masks.zip` is not used, because its licence is unconfirmed.
- **No reference photos.** Wikimedia Commons refused direct file downloads from this container (an error page instead of the image). The proportion work uses published anthropometric numbers instead, and no images of real people are used or committed.
- **No GPU.** Rendering ran on Mesa lavapipe, installed with apt, outside the repo. The PoC 4 timings are CPU-only.
- **v1 of the photo finish drifted in age and ancestry.** v2 fixes this for these 18 faces by tuning the render and the prompt (see PoC 4). It departs from the brief's single fixed prompt: v2 uses a fixed prompt per age band and eye colour.
- **Running GNM's own Python package** was refused by the session's safety check (external code). PoC 8 reads GNM's published data arrays instead and reimplements the rest-pose forward pass and the identity decoder, validated against Keras.
- **FairFace distribution distance** (roadmap 3) was not run: it needs the FairFace image set, which was not downloaded.
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
| GNM Head v3.0 (npz data, identity decoder) | Apache-2.0 | Repository `LICENSE` and README ("suitable for both non-commercial and commercial applications"), https://github.com/google/GNM |
| ISSA skin spectra | CC BY 4.0 | figshare record, https://doi.org/10.6084/m9.figshare.28228571.v4 |
| DINOv2-small, SigLIP 2 base | Apache-2.0 | Model card YAML on Hugging Face |
| YuNet / SFace (OpenCV zoo) | MIT / Apache-2.0 (SFace internal QA only) | `LICENSE` files in https://github.com/opencv/opencv_zoo |
| Facial norms | Published numbers, cited | Wamalwa et al. 2019, https://pmc.ncbi.nlm.nih.gov/articles/PMC6384287/ |

The `jpeg-encoder` crate (which adds an IJG term) was replaced with a small in-crate baseline encoder (`src/jpeg.rs`) to stay inside the allowed set.

## Decisions made (2026-09-25)

- GNM Head adopted as the base (PoC 8).
- Vega's OpenRAIL++ obligations accepted (`photo_finish/OPENRAIL_NOTICE.md`).
- The finish runs at runtime, with seeded, cached, masked and measured output.
- The FLAME masks are used as they are.
- Legal sign-off given: numbers from CC BY-NC(-ND) articles may be used as facts; face-recognition QA is internal only.

## Open questions for you

These are the earlier questions. Items 3, 6, 10 and 12 are answered by the decisions above or by PoC 8.

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
10. **FLAME masks licence.** `FLAME_masks.pkl` is used in PoC 7 at your direction, but its licence is unstated. Should we ask MPI-IS (flame@tuebingen.mpg.de) to confirm whether it falls under FLAME 2023 Open's CC-BY-4.0?
11. **Ancestry in FLAME.** Only anthropometric means are available under a permissive licence. Should we commission or license a FLAME-topology scan set with ancestry labels, to fit per-ancestry shape distributions?
12. **The invented gene pools** need real, reviewed numbers before any use beyond this proof of concept. Who should own that review?
