# Proof-of-concept brief: regen faces that age

This brief hands four proofs of concept to an agent. The decisions behind them are in `.ai/workflows/brainstorm-regen-faces-and-aging-20260924/01-brainstorm.md`. Read `steer.md` in the same folder first. Its rules outrank this brief.

## Hard rules

- Build proofs of concept only. Do not build production code.
- Work only on the branch `poc/regen-faces`. This branch starts from `main` and holds no engine code.
- Put all code in one standalone crate at `poc/faces/`. The crate declares its own `[workspace]` so that it builds alone.
- Do not create or edit any file outside `poc/faces/`. Do not add a root `Cargo.toml`, and do not import or copy engine code.
- Use only dependencies whose licences are MIT, Apache-2.0, BSD, zlib, CC0, or CC-BY-4.0. Record every dependency and every downloaded asset with its licence in `poc/faces/LICENSES.md`.
- Do not create accounts, accept licence terms, or register on any website. When an asset needs registration, stop that step and record it in the report.
- Do not commit downloaded model weights or large assets. Commit the scripts that fetch them. Add the download folder to `poc/faces/.gitignore`.
- Commit the output images that prove each result. Keep each image under 500 KB.

## Background in five lines

- Each regen is a face genome: a small, seeded parameter record that fully decides his face.
- A 3D head carries identity and age. The target is FLAME 2023 Open for shape plus MakeHuman CC0 targets for age and ancestry.
- Age runs from 15 to 70. Updates happen at career stages: 15 to 17, 18 to 20, breakthrough, peak, veteran, retired, and staff decades.
- Hair and beards are hair cards. Coiled hair gets its own methods.
- An optional AI finish paints skin and light over the render. It must not change bones or age.

## Proof of concept 1: the genome and ancestry

1. Define a `Genome` type: shape coefficients, skin tone as a continuous value, hair type, hair colour, hairline, eye colour, birth year, and an ancestry mix.
2. Make generation deterministic from a `u64` seed. The same seed must give the same genome on every run.
3. Define four sample regional gene pools with invented, clearly labelled numbers. Each gene draws from the pools in the ancestry mix.
4. Add a binary that prints 20 sample genomes for three sample nations as JSON.
5. Add tests for determinism and for a serialised size under 1 KB per genome.

The proof passes when the tests pass and the sample output shows varied, mixed ancestries.

## Proof of concept 2: the head, age, and render

1. Fetch the MakeHuman base mesh and its age and ancestry targets from `github.com/makehumancommunity` (CC0). Use FLAME 2023 Open only if the person placed it in `poc/faces/assets/flame/` by hand. It needs registration, so never download it yourself.
2. Map genome values to target weights.
3. Apply age rules from 15 to 70: growth until about 21, fat moving lower from the mid-30s, and greying from the mid-30s to mid-40s by ancestry.
4. Render off-screen with `wgpu` into 512 by 512 PNG files. A software adapter is acceptable.
5. Write a contact sheet: 6 genomes across the rows, and the ages 16, 19, 24, 30, 36, 45, 60, and 70 across the columns.

The proof passes when each row reads as one person who ages from left to right.

## Proof of concept 3: hair cards

1. Generate hair-card geometry and alpha textures in code for four styles: short straight, medium wavy, a tight fade, and short locs or twists.
2. Place the cards on the rendered head from proof of concept 2.
3. Write a contact sheet of the four styles across three skin tones.

The proof passes when the coiled styles read as coiled hair and not as flat cards.

## Proof of concept 4: the photo finish

1. Fetch Segmind Vega and a T2I-Adapter depth model (both reported as Apache-2.0) with a script. Confirm each licence on its model card before you use it, and record the result.
2. Export the depth and colour render from proof of concept 2.
3. Run a low-strength, depth-guided image-to-image pass with a fixed prompt through ONNX Runtime on CPU. A Python script is acceptable for this proof.
4. Write before-and-after pairs for 6 faces at 3 ages.
5. Record the time per image and the hardware. These times do not predict consumer PCs.

The proof passes when the finished faces keep the same person and the same age as their renders.

## Report

Write `poc/faces/REPORT.md` with these sections:
- What each proof of concept showed, with image paths.
- What failed or was blocked, and why.
- Every licence you confirmed, with its source link.
- Open questions for the person.
