# Research: improving face quality, genetic variation and hair, and making the pipeline production-ready

Written 2026-09-25. Four parallel research passes covered face quality, genetic variation, hair, and pipeline quality. Every recommendation was checked against primary sources: licence files, model cards and papers. Items marked **UNVERIFIED** could not be confirmed. It was the plan after PoC 7. Items 1–8 were then proved in PoC 8 (see `REPORT.md`).

Licence labels used throughout:
- **OK**: inside the allowed set (MIT, Apache-2.0, BSD, zlib, CC0, CC-BY-4.0).
- **FLAG**: outside the allowed set, or with pass-through obligations.
- **NC**: non-commercial only.

## Headline findings

1. **GNM Head (Google, July 2026) is Apache-2.0 and ships an ethnicity- and gender-conditioned identity sampler.** Verified from https://github.com/google/GNM:
   - The README says "released under the Apache 2.0 permissive license suitable for both non-commercial and commercial applications". The repository `LICENSE` is Apache-2.0.
   - `gnm/shape` provides 170 head, 3 eyeball and 80 teeth identity components, and 383 expression components.
   - `semantic_sampler.IdentitySampler().sample_identity(Gender, Ethnicity)` is included.
   - The technical report (arXiv 2607.23687) lists four broad classes: Middle Eastern, Asian, Black and White. The training set is about 5,000 scanned individuals.
   - This is the first commercially usable face model with per-group shape distributions over the *whole* face, eye region included. It directly addresses PoC 7's main weakness (East-Asian-like players looking European).
   - Caveats: the README warns the classes "do not fully represent … the full diversity of the global population". Scan consent and provenance are **UNVERIFIED**; read the report's data section before adopting.
   - Tim Bolkart, a FLAME author, is a co-author.
2. **Why PoC 7's ancestry is weak.** A ridge fit of FLAME betas to 13 linear distances leaves everything the distances do not measure at the FLAME mean:
   - lid-crease height;
   - epicanthus;
   - nasal root depth;
   - nasal tip projection;
   - midface projection.

   These are exactly the most visible differences between groups. For example, upper-lid crease height is 3.95 ± 3.57 mm in Chinese men against 8.05 ± 2.11 mm in Malays (PMC5665901, CC BY).
3. **Ancestry explains only a modest share of facial variation, and the generator should reproduce that.**
   - Craniometric diversity is about 13% between regions, 6% between local populations and 81% within populations (Relethford 2002).
   - Skin colour is the opposite: about 88% between regions.
   - In 7,342 Latin Americans, ancestry alone explained 19% of the melanin index, 8% of hair shape, 5% of eye colour, 1% of eye fold, and 0% of face size (Ruiz-Linares 2014, PMC4177621, CC BY).
4. **No permissive scan-quality skin textures, hair datasets or learned hairstyle priors exist.**
   - Skin: Texturing.xyz forbids AI use, 3D Scan Store forbids character makers, and AlbedoMM, BFM, FFHQ-UV and TRUST are all NC.
   - Hair: Hair20k, HAAR, NeuralHaircut, CT2Hair and DiffLocks are all NC.
   - The permissive route is therefore procedural shading and procedural styles, optionally bootstrapped from our own photo-finished outputs baked into UV space.
5. **The photo finish has licence obligations beyond its Apache label.**
   - Segmind Vega (and the xinsir ControlNet, the LCM-LoRA and PixArt) derive from SDXL.
   - SDXL's CreativeML OpenRAIL++-M licence defines distillation as producing a "Derivative", and requires passing the use restrictions on "as an enforceable provision".
   - Shipping Vega therefore probably needs an EULA clause plus a copy of the licence. **This needs legal review.**
   - Every face-identity adapter found depends on InsightFace models, which are NC: IP-Adapter-FaceID, InstantID, PuLID, Arc2Face and PhotoMaker V2.
6. **Industry pattern.** Ubisoft's Faceshifter, MetaHuman Creator and EA's clustering work all use a statistical shape learned from scans, with texture modelled separately, baked into a cheap runtime. FLAME (and now GNM) fits that shape. Our gap is texture and surface detail.
   - Football Manager 26 ships a **pre-rendered static PNG library** of regen faces, which its community criticises.
   - EA ships strand hair for star players, with card fallbacks.
7. **The grey haze on wavy hair cards has concrete causes in our code.**
   - The hair texture has no mipmaps (`mip_level_count: 1`).
   - Hardware alpha-to-coverage uses the *same* sample pattern for every layer, so overlapping semi-transparent cards never add up to opacity.
   - Kajiya-Kay speculars fall on partially covered fringe pixels.

   All three have small fixes (see Hair).

**Corrections between the research passes**, resolved here:
- **UVs.** The pipeline pass said FLAME ships UVs. It does not. The 2023 Open pickle has no UVs, and FLAME's UV layout is distributed with its texture space under CC BY-NC-SA. We need our own unwrap (xatlas, MIT).
- **MakeHuman proxies.** The hair pass assumed MakeHuman eyebrow and eyelash proxies fit our heads directly. That was true for PoC 6 but not PoC 7, which uses FLAME's own topology, so they would need refitting.

## Recommended direction

1. **Adopt GNM Head as the head model and shape-distribution source (PoC 8).** Keep FLAME as a fallback and a comparison.
2. **Make the raw render good on its own:** skin shading, a real eye model, micro-detail, strand hair, beards, brows and lashes. This lets the photo finish stay optional and weak, and gives low-end PCs an AI-free mode.
3. **Replace the invented gene pools with sourced, provenance-tagged numbers,** using continuous admixture and liability-threshold traits.
4. **Treat the finish as a cached, lazily applied, seeded, masked, measured layer,** under a model-provenance register and an EU AI Act Article 50 marking kit.

## Ranked roadmap (combined)

| # | Work | Area | Effort | Licence status | What it unlocks |
|---|---|---|---|---|---|
| 1 | **PoC 8: GNM Head.** Load `gnm_head.npz`; sample identities per class and blend by the genome's ancestry weights (average several class-conditional samples, or blend in latent space); render with the existing pipeline; compare against FLAME on the clay ladder, East-Asian eye region, landmark norms and the photo finish | Face, genetics | M | OK (Apache-2.0); read data-provenance section | Ancestry-specific whole-face shape; 3 eyeball components; teeth |
| 2 | **Guardrails.** Draw finish noise from the genome seed; pin model hashes into a `pipeline_version` cache key; keep a model-provenance register; decide on Vega's OpenRAIL++ obligations; email MPI-IS about the FLAME masks licence | Pipeline | S | n/a | Deterministic regeneration, legal clarity |
| 3 | **Evaluation harness v1:** MediaPipe landmark drift (render against finish, across ages); colorimetric ΔITA / Monk Skin Tone bucket per genome; KD-DINOv2 against FairFace (CC BY 4.0); in-save near-duplicate check; age band via zero-shot OpenCLIP/SigLIP2; SFace or dlib identity **internal-only**, with provenance flagged | Pipeline | M | MediaPipe Apache; DINOv2 Apache; dgm-eval MIT; FairFace CC BY 4.0; SFace Apache label but data provenance unknown | Every later change becomes measurable, including the wide ancestry-drift test |
| 4 | **Constrain the finish:** a skin-only inpaint mask from the head's regions (eyes, brows and hair stay renderer-owned); Lab tone lock after the finish; optional lineart control from the eye and lid contours | Face | S | xinsir union ControlNet has an Apache label but SDXL lineage (FLAG, as with Vega) | Less eye-shape and tone drift |
| 5 | **Skin shading:** pre-integrated skin (Penner 2011) with a curvature LUT; dual-lobe specular; cavity and roughness regions; a two-chromophore (melanin and haemoglobin) albedo calibrated so renders cover MST 1–10; a regression test against the Monk swatches | Face | S–M | Own implementation; Separable SSS reference is BSD-2; MST CC BY 4.0 | Less "painted CG skin"; fair dark-tone coverage |
| 6 | **Hair card fixes:** coverage-preserving mipmaps; a custom stochastic `@builtin(sample_mask)` per fragment instead of hardware alpha-to-coverage; 8–16 jittered accumulation passes; replace Kajiya-Kay with a Marschner approximation (Karis 2016 / Frostbite 2019) using melanin absorption from pbrt-v3 (BSD-2) | Hair | S–M | Own code; pbrt-v3 BSD-2 | Removes the haze; true salt-and-pepper greying; saturated blond and red |
| 7 | **Real eye model:** cornea bulge with refracted iris lookup; limbal ring fading with age; tear meniscus; eye-occlusion shell (the biggest fix for "doll eyes"); caruncle; lid-margin thickness; eyelashes | Face | M | Own implementation; ICT-FaceKit (MIT) eye-helper geometry as reference | Removes the staring, doll-eyed look |
| 8 | **Sourced gene pools:** neutral components (about 9 anchors) with Dirichlet ancestry; continuous traits blended plus within-group covariance; ordinal traits (curl, lid crease, epicanthus, beard, baldness) as liability thresholds; skin from ISSA spectra (CC BY 4.0) in (L\*, b\*) plus site a\*; eye colour and red hair via HERC2 and MC1R allele frequencies (1000 Genomes via Ensembl, unrestricted); baldness and greying curves by age; unit tests that sourced marginals and variance-explained are reproduced | Genetics | M | Sources listed below; several articles are CC BY-NC-ND, so their numbers are used as facts only after legal sign-off | Realistic, respectful variation; correct handling of mixed ancestry |
| 9 | **Strand hair for portraits:** 50–120k strands as pixel-wide strips with stochastic transparency; later port the MIT WebGPU software rasteriser (Scthe/frostbitten-hair-webgpu); guide-strand style grammar (part, whorl, length/fade field, curl, clump, frizz, tie points, braid map); deep opacity maps; keep cards as the low-spec fallback | Hair | M–L | MIT (Scthe, TressFX); algorithms from papers; avoid the patented hair-mesh approach | The largest hair realism jump; full style coverage |
| 10 | **Facial hair and coily hair:** beard and moustache strands from style masks × density × length genes; strand stubble on the jaw silhouette plus the painted follicle shadow; strand eyebrows; type-4 coils as super-helices with phase locking, switchbacks and curvature-space ruffle (Wu 2024; Shi 2026, CC BY 4.0), plus a silhouette fringe; procedural braids, cornrows and locs | Hair | M | Papers only; Curly-Cue code licence UNVERIFIED, so reimplement | Real beards (none today), authentic afro-textured styles |
| 11 | **Detail geometry:** our own UV unwrap (xatlas, MIT); one Loop subdivision; procedural pore, fine-wrinkle and age-crease normal maps keyed to regions and age | Face | M | OK | Close-up quality; age that reads without the finish |
| 12 | **Finish at scale:** fp16 ONNX; Segmind-VegaRT (LCM, 2–4 steps, no CFG); `ort` in Rust; Windows ML / DirectML / CPU tiers; lazy finishing on first view; career-stage caching; a no-AI tier | Pipeline | M | Same Vega lineage question | Plausibly under 1 s per portrait on 8 GB GPUs (**UNVERIFIED** estimate); thousands of regens per save |
| 13 | **Article 50 compliance kit:** C2PA manifests (`c2pa` crate, MIT/Apache), an invisible watermark (Adobe TrustMark MIT or `invisible-watermark` MIT), an in-game "AI-assisted portrait" label and toggle, and Steam's live-generated disclosure text | Pipeline | S–M | OK | Art. 50 marking by the 2 December 2026 grace deadline (per the research; confirm with counsel) |
| 14 | **Identity across a career:** reference-attention from the player's canonical portrait (reimplement; avoid GPL code) or IP-Adapter Plus Face (Apache; OpenCLIP MIT) at low scale; later, a texture-space finish (project the canonical finished portrait to UV, de-light, re-render later ages) | Pipeline | M → L | OK, but SDXL lineage for the adapters | Fewer identity breaks at 60+; fewer AI calls |
| 15 | **Offline evaluation of Apache-2.0 finishers on a GPU:** Z-Image-Turbo with its union ControlNet, and FLUX.2 klein 4B editing | Face | M | Apache-2.0 (the klein text encoder's licence is UNVERIFIED) | A cleaner-lineage, higher-quality finish tier for servers or 16 GB GPUs |
| 16 | **Perceptual studies:** HYPE-style real-vs-synthetic 2AFC; same/different identity across ages (d′); age-rating error; Bradley-Terry preference for A/B tests; a diverse rater panel reviewing ancestry plausibility | Pipeline | M, recurring | n/a | Validates the metrics; sets release gates |

Fallback:
- A pre-rendered portrait library at age bands, made offline with a heavy Apache-2.0 model, is how FM26 ships. It remains a zero-risk option if runtime generation proves infeasible.

## Face quality: key facts

**Head models**
- **GNM Head:** above.
- **ICT-FaceKit Light:** MIT per the repository: 26.7k vertices, PCA identity modes from light-stage scans, 53 ARKit expressions, a UV layout, and eye helper shells (lacrimal fluid, occlusion, blend), eyelashes, teeth and tongue. The full model uses a separate USC licence. Scan-data provenance is not documented; confirm with USC ICT before shipping derived meshes.
- **Rejected:**
  - SMPL-X, LYHM/Headspace, the NPHM dataset, Ava-256/Goliath, BFM, FaceScape and FaceVerse are NC.
  - MetaHuman is under the Epic EULA, which bans AI training and ties the calibration tooling to Epic products.
  - MB-Lab is AGPL.

**Skin**
- There is no permissive human skin scan library. Poly Haven and ambientCG have none.
- MakeHuman system skins are CC0 but low-fidelity; they are useful only for colour-range reference.
- Model skin as melanin plus haemoglobin. Keep specular independent of tone. Test coverage against the Monk Skin Tone swatches (CC BY 4.0): #f6ede4 #f3e7db #f7ead0 #eadaba #d7bd96 #a07e56 #825c43 #604134 #3a312a #292420.

**Eyes**
- At 512 px the eyes are about 25 px wide. The occlusion shell, specular highlight, limbal ring and tear line matter more than iris micro-detail (Jimenez, GDC 2013).

**Finish alternatives** (card licences verified)

| Model | Licence | Notes |
|---|---|---|
| FLUX.1 schnell | Apache-2.0 | Gated click-through, so you must download it yourself; 12B |
| FLUX.2 klein 4B | Apache-2.0 | About 13 GB VRAM |
| Z-Image-Turbo (6B) | Apache-2.0 | Has an Apache-2.0 union ControlNet |
| Qwen-Image-Edit | Apache-2.0 | 20B |
| Sana | FLAG | Gemma terms on its text encoder |
| SD 3.5 | FLAG | Stability Community licence |
| PixArt-Σ | FLAG | OpenRAIL++ |
| Kolors | FLAG | Commercial registration required |
| HunyuanDiT | FLAG | Tencent community licence |
| SDXL-Turbo, SD-Turbo | NC | |
| Hyper-SD | Excluded | No licence stated |

**Face restoration:** CodeFormer is NC. GFPGAN's code is Apache but its priors are FFHQ-derived (**UNVERIFIED**), and restorers normalise faces toward FFHQ. Do not use either.

## Genetic variation: key data

**Farkas et al. 2005 regional aggregates** (adult males, mm). These are unweighted means of 25 groups, from PMC5708327 Table 6 (CC BY-NC-ND; used as facts pending legal sign-off).
- EU is 12 European groups plus North American White; EA is Singapore Chinese, Vietnamese, Thai and Japanese; AF is Angolan, Zulu and African American; ME is Azerbaijani, Iranian, Turkish and Egyptian; IN is Indian.
- Between-group differences in Farkas are inflated by observer effects, because each country was measured by different people with about 30 subjects. Shrink go-go and zy-zy the most.
- Take deltas within one study only; never mix caliper and 3D methods.

| Measure | EU | EA | AF | ME | IN |
|---|---|---|---|---|---|
| en-en | 31.6 | 37.2 | 36.2 | 31.8 | 34.1 |
| n-gn | 119.9 | 122.8 | 119.9 | 124.5 | 112.5 |
| sn-gn | 67.7 | 71.4 | 72.8 | 68.1 | 62.7 |
| zy-zy | 136.9 | 145.7 | 135.9 | 141.5 | 135.8 |
| n-sn | 54.5 | 53.6 | 50.6 | 57.8 | 47.2 |
| al-al | 35.4 | 39.6 | 44.1 | 35.0 | 37.9 |
| ch-ch | 52.2 | 49.0 | 55.1 | 51.2 | 51.0 |
| go-go | 104.8 | 111.1 | 102.0 | 104.9 | 102.8 |

**Other sourced values**
- **Within-group coefficient of variation** (Fang 2011, 26 groups): en-en .088, ex-ex .049, en-ex .065, al-al .072, ch-ch .079, n-sn .070, sn-gn .080, zy-zy .042, go-go .056, tr-n .110.
- **Eye region and nose depth:**
  - Hong Kong Chinese: eye fissure 27.6 mm long, 11.6 mm high (PMC3730197, CC BY).
  - Chinese vs Malay lid-crease height, as above.
  - Malay nasal tip protrusion (sn-prn) 17.3 mm; nasolabial angle 105° (PMC5051712, CC BY).
  - Nasolabial angle pooled across groups: African 87.5°, Asian 94.7°, Caucasian 100.1° (Wen 2015, PMC4527668, CC BY).
- **Skin, ISSA** (about 15k spectra, 2,113 subjects, 8 groups; data CC BY 4.0, https://doi.org/10.6084/m9.figshare.28228571.v4). Pooled L\* runs from 39.6 (African) to 63.5 (Japanese), and group differences lie mainly along b\* and L\*. Also, 89.4% of samples have a perceptually indistinguishable match in another group. Recompute face-site statistics from the raw data before use.
- **Allele frequencies** (1000 Genomes via Ensembl; frequency of the effect allele, AFR / AMR / EAS / EUR / SAS):

  | Variant | Effect | AFR | AMR | EAS | EUR | SAS |
  |---|---|---|---|---|---|---|
  | HERC2 rs12913832 G | Blue eyes | .028 | .202 | .002 | .636 | .071 |
  | MC1R rs1805007 T | Red hair | .003 | .016 | .001 | .072 | .005 |
  | SLC24A5 rs1426654 A | Lighter skin | .074 | .589 | .012 | .997 | .685 |
  | EDAR rs3827760 G | Thicker, straighter hair | .003 | .392 | .873 | .011 | .013 |

  - The HERC2 frequency is clinal within Europe, from Finland .91 to Spain .32.
  - Modelling these two-allele loci fixes blue-eye over-production in mixed-ancestry players, and supports father–son inheritance.
- **Greying onset:** Caucasian 34 ± 9.6 years, Black 43.9 ± 10.3 (Tobin & Paus 2001). Panhard 2012: 6–23% of people have at least 50% grey hair at age 50, depending on origin.
- **Male-pattern baldness, Norwood III or worse**, by decade from the 20s to the 70s:
  - Korean: 2.3, 4.0, 10.8, 24.5, 34.3, 46.9%.
  - Chinese: 2.8, 13.3, 21.4, 31.9, 36.2, 41.4%.
  - US men aged 18–49: 42% overall.
- **Heritability** (Korean families): en-en .61, al-al .53, sn-prn .44 … ch-ch .23. For a father–son model:
  - a son's additive part is the parents' average plus N(0, h²/2);
  - ancestry weights are averaged;
  - loci are passed on in the Mendelian way.
- **Real nations:** there is no open source for the ancestry composition of squads, and inferring a real player's ancestry from photos or names is unacceptable. Build nation templates from census data, UN DESA migrant stock and Wikidata (CC0) birthplace and citizenship aggregates. Have them reviewed by community members. Keep component IDs neutral and internal, and never show "race" in the UI.
- **Data gaps:** open Latin American and Pacific Islander facial norms; per-country hair curl frequencies (Loussouarn 2007 is paywalled); beard density; African baldness curves; epicanthus prevalence.

## Hair: key facts

**Rendering references**
- Frostbite "Strand-based Hair Rendering" (SIGGRAPH 2019): Marschner R/TT/TRT with the lobe formulas given in the slides; four deep-opacity layers; strands drawn as strips widened to one pixel.
- Karis 2016: a practical Marschner approximation.
- pbrt-v3 hair BSDF (BSD-2): the melanin-to-absorption conversion σa = ce·(0.419, 0.697, 1.37) + cp·(0.187, 0.4, 1.05).
- Code to reuse:
  - **Scthe/frostbitten-hair-webgpu (MIT)**: a WGSL software rasteriser for strands, about 3.3 ms for the fine pass on an RTX 3060;
  - **AMD TressFX (MIT)**.
- Avoid: NVIDIA HairWorks (proprietary); Unity demoteam Hair (Unity Companion Licence); Cem Yuksel's hair meshes (**patented**).

**Datasets and priors (all NC or unavailable):** USC-HairSalon, Hair20k, the Perm weights (its code is MIT), HAAR, NeuralHaircut, CT2Hair, DiffLocks and TANGLED.

**Assets**
- Usable:
  - MakeHuman system assets (CC0): eyebrow001–012, eyelashes01–04, and styles afro01, braid01, short01–04 and others. They need refitting to FLAME or GNM.
  - Sketchfab "Hair Cards_FBX" (POLYTRICITY, CC Attribution; version UNVERIFIED).
  - Sketchfab "Cornrow Braids Hairstyle" (OCBacon, CC Attribution).
- Flagged: Open Source Afro Hair Library (BOSS licence, bans AI use); Dove "Code My Crown" (no licence); Blender "Hair Styles" demo (CC-BY-SA).

**Coily and afro-textured hair.** At 512 px the silhouette fringe, matte highlights, phase-locked clumps and switchbacks carry the look more than individual coils do. Use super-helix strands generated in curvature space.

**Sports games:** EA ships Frostbite strand hair (up to 50k strands per character, card fallback on low-end hardware). No published details exist for FM, 2K or eFootball regen hair.

## Pipeline: key facts

**Evaluation**
- Avoid clean-fid's default Inception weights, which are under the NVIDIA licence (NC). Use DINOv2 features (dgm-eval, MIT).
- Most "permissive" face-recognition weights are trained on NC datasets (VGGFace2, MS1M). Use them for internal QA only, after legal sign-off, and never ship them.
- **Do not classify race.** Measure skin colorimetry (ITA°, Monk Skin Tone bucket) and landmark ratios instead, and stratify every metric by Monk Skin Tone bucket.

**Consumer hardware (Steam survey, August 2026):** 16 GB VRAM 26.9%, 8 GB 25.7%, 12 GB 13.0%. Target 8 GB for the finish tier and fall back to raw renders below that.

**EU AI Act Article 50**
- Applies from 2 August 2026, with a marking grace period for existing systems until 2 December 2026, per the Digital Omnibus.
- Requires machine-readable marking; the Code of Practice expects signed metadata plus an imperceptible watermark.
- The deepfake label (Art. 50(4)) applies only to content resembling real people. That makes the checks against resembling real people a compliance tool.

**Steam:** live-generated AI content must be disclosed with a description of the guardrails.

**Likeness**
- Face recognition against photos of real players counts as processing biometric data (GDPR Art. 9). Do not do it without a DPIA and legal counsel.
- Prefer:
  - near-duplicate checks within generated faces only;
  - a "looks like someone" report that re-rolls cosmetic genes.

## Do-not-use list (verified)

| Reason | Items |
|---|---|
| NC | SMPL-X, LYHM/Headspace, the NPHM dataset, Ava-256/Goliath, non-Open FLAME versions and the FLAME texture space and UVs, BFM, AlbedoMM, TRUST/BalancedAlb, DECA, MICA, FFHQ and FFHQ-UV(-Intrinsics), Digital Emily 2, CodeFormer, InsightFace models and everything built on them (IP-Adapter-FaceID, InstantID, PuLID, Arc2Face, PhotoMaker V2), SDXL-Turbo/SD-Turbo, EdgeFace, the hair datasets and priors listed above, GaussianHaircut |
| FLAG | MetaHuman (Epic EULA), Unity Digital Human and Unity Hair (Unity Companion Licence), MB-Lab (AGPL), SD 3.5 (Stability Community), PixArt-Σ and LCM-LoRA (OpenRAIL++), Sana (Gemma terms), Kolors (registration), HunyuanDiT (Tencent licence), Hyper-SD (no licence), Texturing.xyz (bans AI use), 3D Scan Store (no character makers), Open Source Afro Hair Library (BOSS licence), dlib library (BSL-1.0; its weights are public domain) |
| Patent | Hair meshes (Yuksel); statistical wisp model (US 7,418,371, likely expired, UNVERIFIED) |

## Decisions (made 2026-09-25)

1. **GNM Head is the base.** PoC 8 proves roadmap items 1–8 (see REPORT.md, PoC 8).
2. **Vega's OpenRAIL++ obligations are accepted:** an EULA clause plus a licence copy (`photo_finish/OPENRAIL_NOTICE.md`).
3. **The finish runs at runtime:** seeded, cached by pipeline key, masked and measured.
4. **Legal sign-off given** on using numbers from CC BY-NC(-ND) articles as facts, and on internal-only face-recognition QA.
5. **The FLAME masks are used as they are;** no MPI-IS query.

Still open: community review of nation templates, and GNM's data-provenance section.
