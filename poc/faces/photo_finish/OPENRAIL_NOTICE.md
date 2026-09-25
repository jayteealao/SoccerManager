# Photo finish: OpenRAIL++ obligations (accepted)

**Decision (2026-09-25, project owner):** ship the Segmind-Vega photo finish at runtime and accept the obligations that come with its SDXL lineage. Legal sign-off was given by the project owner. This file is the working checklist; it is not legal advice.

## Why the obligations apply

- Segmind-Vega is labelled Apache-2.0 on its model card, but it is distilled from Stable Diffusion XL.
- The SDXL licence, CreativeML Open RAIL++-M (copy in `licences/CreativeML-OpenRAIL-PlusPlus-M.txt`), treats distillation as producing a "Derivative of the Model".
- Paragraph 5 of that licence requires the Attachment A use restrictions to be passed on, and requires users of a Derivative to comply with them.
- The T2I-Adapter (Apache-2.0) is SDXL-conditioned, so we treat it the same way.

## What shipping requires

1. **A copy of the licence** travels with the game: `licences/CreativeML-OpenRAIL-PlusPlus-M.txt` goes in the install's third-party notices.
2. **An enforceable EULA clause.** Draft wording:

   > *AI-assisted portraits.* The game includes a machine-learning model derived from Stable Diffusion XL, licensed under the CreativeML Open RAIL++-M License (included with the game). You agree not to use that model, or any part of it extracted from the game, for any of the uses listed in Attachment A of that licence, including to break the law, to harm minors, to generate or spread false or harmful information about real people, to defame or harass, or to discriminate against people based on protected characteristics. If you redistribute the model you must pass on these restrictions.

3. **Modifications notice.** Our ONNX export, the fixed prompt sets and the finish loop are modifications. State that in the notices ("converted to ONNX; used for img2img at low strength").
4. **No claim of endorsement** by Stability AI or Segmind.
5. **Keep the provenance register current.** `models.lock.json` pins every model file by SHA-256, and `provenance.py` regenerates it. The finish cache key includes those hashes, so swapping a model invalidates cached portraits.

## Runtime finish (decision)

The finish runs at runtime, as opposed to a pre-rendered library. PoC 8 proves the guardrails that make this reproducible:
- the finish seed is derived from the genome seed and age, so the same player always gets the same portrait;
- outputs are cached per pipeline key.

Speed and hardware tiers are roadmap item 12.
