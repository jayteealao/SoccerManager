---
schema: sdlc/v1
type: brainstorm
slug: brainstorm-regen-faces-and-aging-20260924
topic: "we need to brainstorm solutions to generating regen or Newgen faces but of a higher realistic quality than fm does and the possibility of aging what is the solution space research solutions and think extensively before we start brainstorming"
status: open
board: brainstorm-board.json
page: "https://claude.ai/artifact/Hv2SDBaXq7ECZs28NhpH3v"
created-at: "2026-09-24T15:21:24Z"
updated-at: "2026-09-24T18:15:20Z"
sessions: 1
batches: 32
consult-runs: []
revisions:
- rev: 1
  at: "2026-09-24T18:15:20Z"
  trigger: manual
  because: done
  changed: the person chose done after every problem area and the lead route were explored; scoping the work begins
---

# Brainstorm: regen faces that look real and age

## The Brainstorm
We started from one wish: the youth players that Touchline creates each season should have faces more realistic than Football Manager's, and those faces should age. A research pass came first. It found that no football game ages a generated face, that the games which age faces well store a face as parameters on a head, and that most photoreal AI face tools carry non-commercial licences.

We now believe in one route. Each regen is a face genome with an identity that can age from day one. FLAME 2023 Open gives the shape, MakeHuman's CC0 targets give age and ancestry, and all of age lives in the 3D head. Hair and beards are hair cards, with dedicated methods for coiled hair and 150 or more styles that change at life moments. Code builds the structure, and studio AI makes textures that ship as compact data, with you as the only reviewer. A separate Rust portrait process renders faces: followed players are baked at age-band edges, and everyone else is drawn on demand. Our best render is the default look. A painterly shader gives a painted style with no AI, and an optional download adds an AI photo finish on capable PCs, with Vega at launch and our own distilled finisher later. Regens come from real countries: each carries a mix of ancestry, spreads follow each nation's footballers, and heritage opens a second international nation.

Still open: whether lists of unfollowed players wait for a slow finisher, the details of aging (band edges, life events on the face), and player trust. The top risks are the art job for one reviewer and a finisher that runs too slowly on real PCs.

## Map
### What more realistic than Football Manager means, and where faces show — problem side — touched
Decided: the first version solves the plastic look and the clones best; photoreal by default with a painted setting; the profile sets the bar. Open: whether the youth intake reveal needs more.

### Aging, and the same person across a whole life — problem side — explored
Decided: aging runs 15 to 70, all in the 3D head; faces update at career stages with two academy stages; weight after football and stress greying mark the face; hair loss goes first if art forces a cut. Core and first version: all four.

### Faces that fit where a player comes from — problem side — explored
Decided: real countries; all four origin failures unacceptable; a mix of ancestry over about 20 regional gene pools; spreads follow footballers, from population data shifted by public heritage records; heritage gives international eligibility. Core and first version: all four. Later: family looks.

### Player trust: the backlash against AI art, store disclosure, and likeness to real people — problem side — explored
Decided: generative AI in scope with Steam disclosure; studio audits check likeness before release; no AI claim for the painted mode; open by design in the store, settings, and a public page. Core: disclosure, audits, openness. First version: all four.

### Scale, save size, and the player's hardware — problem side — touched
Open: where the photo finish runs. A genome is a few hundred bytes per player; only the photo finish needs a graphics card.

### A shipped library of finished face images — solution side — open
Not the lead: a finished image cannot age. It returns only as a studio library of textures.

### Generative AI, on the player’s PC or in the studio — solution side — explored
Decided: our best render with studio textures is the default; an optional download adds an AI finish on capable PCs (Vega with a depth adapter, then our own distilled finisher); players without it choose painted or render; studio audits check likeness. Core and first version: all four. Top risk: the finisher is too slow on real PCs.

### A face genome rendered as a 3D head — solution side — explored
Decided: FLAME 2023 Open shape with MakeHuman CC0 targets, age fully in 3D; hair cards, dedicated coiled styles, 150 or more styles changed at life moments; code structure with studio AI textures shipped as data; a Rust portrait process, followed players baked at age bands, others on demand; a painted shader with no AI. Core and first version: all. Top risk: the art job.

### Procedural 2D faces — solution side — open
Not the lead. faces.js stays a reference for how small a face record can be.

### A genome render with an AI realism pass on top — solution side — explored
Decided: the lead route. Identity and age live in the 3D head; an optional AI finish paints skin and light. Core and first version.

### Bought character middleware — solution side — open
Not the lead: FaceGen was set aside for FLAME plus MakeHuman; MetaHuman cannot create heads during play.

### Licences and the MIT or Apache rule — both sides — touched
Verified clear: FLAME 2023 Open, MakeHuman assets, faces.js. Open: FLAME skin textures and the tools that guide the AI pass.

## Open now
- Top risk: the art job for one reviewer — skin sets for every decade and skin tone, and 150 or more hairstyles, where studio AI is weakest on coiled hair and dark skin.
- Top risk: the photo finish runs too slowly on real PCs; speed is unmeasured.
- No tension is open.
- Question for the plan: which tools that guide the AI from a render allow commercial use, and which licence covers FLAME skin textures?
- Question for the plan: which game data drives weight after football and stress greying?

## Briefs
None.

<!-- The front ends here. The record follows. -->

## Decisions
### What more realistic than Football Manager means
- All four Football Manager complaints matter: the plastic look, faces that never age, clones, and faces wrong for the nation. Why: you picked every complaint.
- The first version makes faces read as real people and makes every regen look individual. Aging and the fit to a nation must be acceptable, not excellent. Why: you picked the plastic look and the clones as the two to solve best.
- The player profile, at about 200 to 300 pixels, sets the detail bar for every face. Why: you chose the profile as the key screen.
- From the first version, every face stores an identity that can age, even if the first version ages it only lightly. Why: faces in existing saves must never change when aging arrives.

### The face genome with an AI finish
- The lead identity basis is a face genome on a 3D head, aged by rules, with a light AI pass that paints photoreal skin, hair, and light. Licensed AI identity tools, a seed with an age slider, and a model we train are not taken further now. Why: you chose it from the four options. Accepted risk: the most engineering of the four; hair and the licence of the tools that make the AI follow the render are unproven.

### The head model and age
- The head model is FLAME 2023 Open for identity shape, with the MakeHuman CC0 age and body targets adapted onto it. All of it runs in our own code, so regens are created during play. Why: the most control on a commercial-safe base. Accepted risk: fitting the MakeHuman targets onto the FLAME mesh is our own work, and FLAME skin textures need a separate licence check.
- Age lives in the 3D head, both shape and surface: bone growth, fat shift, wrinkles, pores, grey hair, and hairline. The AI pass only paints and must not change the age. Why: aging stays predictable and owned. Accepted risk: wrinkle and skin sets for each decade across every skin tone are a large art job. This is the top risk.
- Skin and hair textures ship as compact data, parameters and small detail tiles, that code expands into full textures. Why: it keeps the spirit of the rule against shipped illustration files.

### The renderer
- Faces render in a separate Rust portrait process (wgpu off-screen, with the finisher beside it through ONNX Runtime), which hands images to the page. Why: keeps graphics out of the simulation. Accepted risk: a third process to ship, start, and supervise.
- The current-age face of each followed player (squad and shortlist) is baked and stored; every other face is drawn on demand and cached. Why: a small save, with no delay on the faces seen most.
- A followed player gets a new baked face at age-band edges and at life moments. Why: the fewest renders. Accepted risk: aging shows as visible steps between bands.
- Lists show the render at once, and the finisher upgrades faces in the background as each one finishes. Why: a slow finisher must never stall a list. Core.
- The painted style is a painterly shader on the render, with no AI, so it runs on every PC and gives players who avoid AI art a clean choice. This replaced the open idea of a drawn style with no AI.

### Player trust
- The painted mode is a look, not an AI choice, and the game makes no AI claim about it. Why: a false no-AI claim is the worst outcome in a backlash.
- The game is open by design about faces: the Steam disclosure, a plain explanation in settings, and a public page on how faces are made, including the likeness audits and the data rules. The wording is precise: code decides who a player is and how old he looks, studio AI made the skin and hair materials, and the optional finisher paints light and skin. This replaced a claim that identity and age are never AI. Core.

### Scope of the first version
- No limit is set on the first version: every kept decision ships in it. Accepted risk: the first version is large for one reviewer and may take long to ship.

### Aging details
- Aging runs from 15 to 70, through coaching and management. Accepted risk: the most age sets for the head.
- Band edges follow career stages: academy, breakthrough, peak, veteran, retired, and staff decades. Accepted risk: long gaps at peak, and the academy stage covers the years when a face changes most.
- The academy has two stages, 15 to 17 and 18 to 20, before the career stages. Why: a face changes most while bone growth ends.
- If the art job forces a cut, hair loss moves to later first; teenage growth, fat shift, wrinkles and skin, and greying stay.
- Two life events mark the face: weight after football, and faster greying for a manager under pressure. Injury marks and tattoos are not included. This replaced the open idea of life events on the face.

### Hair and beards
- A regen changes hairstyle at set life moments, such as a first-team debut, a transfer, or retirement, and keeps it between them. Why: fewer changes, and each one tells a story.
- Hair is built as hair cards in 3D, authored per style. Why: hair fully in 3D and predictable. Accepted risk: each style is art work, and hair cards are known to render tightly coiled hair badly.
- Beards use the same hair-card method as the hair. Why: one system to build.
- Coiled hair gets dedicated authoring: braids and locs as geometry, fades and cornrows as scalp textures. Why: coiled hair must look right, or the game repeats the wrong-for-the-nation complaint.
- Hairstyle can change at a first-team debut, a transfer, turning about 30, retirement, and club milestones (trophies, captaincy, appearance landmarks, big occasions). Each regen has his own tendency, so some change often and some never. Why: the moments are not uniform for all players.
- Code generates the structure of hair and skin, AI in the studio makes the textures, no artists are involved, and you review every result. Accepted risk: you are the only quality gate for 150 or more styles and every skin set, and studio AI trained on skewed data is weakest on coiled hair and dark skin, where the representation risk is highest. The textures also add to the Steam disclosure.
- The first version ships 150 or more hairstyles across all hair types. Why: rare repeats across a whole save. Accepted risk: the largest art job.

### Faces and nations
- Regens come from real countries, so a face must fit real populations, including diaspora and mixed heritage. Accepted risk: mapping real nations to looks carries the ethics and stereotype risks of real demographics.
- All four origin failures are equally unacceptable: a look wrong for the nation, one look per nation, missing diaspora and mixed heritage, and coarse skin tone.
- Each regen carries a mix of ancestry proportions, and each gene draws from the pools in the mix. Accepted risk: gene pool data is needed for every ancestry region.
- The regens of a nation follow the ancestry spread of its real professional footballers, not its general population. Why: the most football-true result.
- Spreads come from published data where it exists; your judgement fills the gaps and is recorded as judgement.
- Heritage shapes the face and also gives a regen a second nation for international football. Why: a real football mechanic that links faces to the season layer.
- Population data sets the base spread, and public birthplace and eligibility records of professional players shift each nation towards its diaspora. No faces of real players are used. Why: heritage evidence without looking at anyone.
- Family looks (sons, nephews, and brothers of retired players) come after the first version; the genome stores what they need from the start. This replaced the open idea that regens inherit the looks of a retired player.

### The photo finish
- An AI pass runs on capable player PCs: Segmind Vega with a T2I-Adapter depth guide first, replaced by our own distilled finisher when it beats Vega. This replaced the earlier decision to leave the placement open. Accepted risk: Steam treats the pass as live generation, which needs a written account of guardrails, and speed on consumer PCs is unmeasured.
- Our own distilled finisher, a small one-step model trained on studio FLUX.1 schnell finishes of our renders, replaces Vega after the first version. Core; later.
- The finisher is an optional download, offered in settings when the graphics card can run it. Why: a small base game, and the photo look is opt-in.
- The default look is our best render with studio textures; the finisher upgrades it to photo on capable PCs that download it; the painted style stays a setting. This replaced: photoreal by default with a painted setting. Why: an optional finisher cannot be the default.
- Likeness to real footballers is checked by studio audits of thousands of sample faces before release; nothing screens at run time. Accepted risk: a face that resembles a real footballer can still appear in a save.
- Where the photo finish is not available, the player chooses in settings between the painted style and our best render with studio textures.

### Generative AI
- Generative AI stays in scope. Generation on the player's PC and generation in the studio only are both evaluated further; a route with no generative AI is not the lead. Why: you want to weigh the two placements before you choose. Accepted risk: either placement means a Steam AI disclosure and exposure to the backlash against AI art; generation on the PC also needs written guardrails.

## Ideas still open
- A hidden style trait per regen sets how often he changes his look, linked to personality: a showman changes often, a quiet professional never. Raised by the agent.
- The art pipeline prints review sheets: a grid of every style across every hair type, skin tone, and age, so gaps and bias show at a glance. Raised by the agent.

## What we found
- Player records today carry no nationality, birth date, or appearance fields, and the season design plans no youth intake yet. A face system therefore also defines the identity data it rests on. Source: crates/engine/src (no match); docs/design/realism/03-season-layer.md.
- The product rule says brand assets and illustration are generated in code, not sourced from image files, and every dependency must be MIT- or Apache-compatible. Source: PRODUCT.md.
- Football Manager added 2D generated regen faces in 2008 and 3D faces in 2018. The 2026 edition moved to Unity, and players still call regen faces plastic. Source: https://fullerfm.com/2021/11/26/the-story-of-football-managers-newgen-faces/
- No source describes any aging of Football Manager regen faces. Source: as above.
- The popular fan mods assign pre-made GAN photos to regens from about 14 appearance folders, by player ID. The faces never age, and the tools are GPL-3.0. Source: https://github.com/Maradonna90/NewGAN-Manager
- EA FC youth players come from a small bank of heads. Players complain of clones and of faces that never grow a beard or age. Source: EA Answers HQ bug report.
- Out of the Park Baseball uses FaceGen 3D heads that age and change with weight across a career. Source: https://manuals.ootpdevelopments.com/index.php?man=ootp24&page=game_settings_facegen
- Crusader Kings 3 stores each character as a DNA string of gene sliders. Children inherit genes from both parents. Age is a separate layer of blend shapes and wrinkle and hair-loss textures over the same head. A small team built it. Source: https://gdcvault.com/play/1027354/Creating-a-Portrait-System-Based
- faces.js stores a face as a small object and is Apache-2.0 (checked at the source). Source: https://github.com/zengm-games/facesjs
- StyleGAN weights are non-commercial and the FFHQ face set is CC BY-NC-SA. Source: https://github.com/NVlabs/stylegan2-ada-pytorch
- FLUX.1 schnell is Apache-2.0 and makes an image in about four steps. FLUX.1 dev is non-commercial. SDXL and SDXL-Lightning allow commercial use under an OpenRAIL licence; SDXL-Turbo does not. Source: Hugging Face model cards.
- Almost every tool that keeps one identity across images depends on InsightFace weights, which are non-commercial without a paid licence. Source: https://www.insightface.ai/solutions/face-recognition-licensing
- The strong learned aging models are built on StyleGAN or on non-commercial face sets. Source: SAM and Lifespan Age Transformation Synthesis repositories.
- An age slider can be a small add-on of a few megabytes to any image model. Source: https://sliders.baulab.info/
- The FLAME 2023 Open head model allows commercial use under CC-BY-4.0 (checked at the source). The Basel Face Model and SMPL-X are non-commercial. Source: https://flame.is.tue.mpg.de/modellicense.html
- Since 2025, MetaHuman heads may be used in any engine under the Unreal licence terms. Character Creator content forbids an in-game character creator without an enterprise licence. Source: https://www.metahuman.com/license
- MakeHuman code is AGPL, but its bundled assets, including the morph targets, are CC0, and exports carry no limitation (checked at the source). Source: https://raw.githubusercontent.com/makehumancommunity/makehuman/master/LICENSE.md
- Regens are born during play, so the head model must create heads in our code at run time. A tool that makes heads only in its own editor, such as MetaHuman, cannot serve that. Source: https://www.metahuman.com/license
- Images that Blender renders are not covered by its GPL. Source: https://www.blender.org/about/license/
- About half of Steam players have a graphics card with 8 GB or more. A mid-range card makes a four-step portrait in seconds; integrated graphics take minutes. Source: Steam Hardware Survey, August 2026.
- Hosted four-step generation costs roughly 3 to 25 dollars per thousand images. Source: https://pricepertoken.com/flux-pricing
- A library of 50,000 faces at four ages is about 16 GB, while a face genome is a few hundred bytes per player. Source: estimate.
- Steam asks for disclosure of player-facing AI content; content generated while the game runs also needs a written account of its guardrails. Source: PC Gamer.
- Games accused of AI art on Steam in 2025 and 2026 met review bombs and lost awards and sponsors. Source: PC Gamer.
- The EU AI Act labelling duty for images that resemble real people started on 2 August 2026. Source: https://artificialintelligenceact.eu/transparency-rules-article-50/
- Diffusion models can reproduce training photos of real people. A similarity check and a hard exclusion of real footballers is the standard defence. Source: Carlini and others, USENIX Security 2023.
- Football Manager players complain of faces that do not fit a regen's nation. Source: FullerFM.
- The common face training set is about two thirds white, and models trained on it drift towards lighter faces. Source: bias study on ResearchGate.
- Facial bone growth ends at about 18 to 21 in men. Fine lines show in the 30s and sagging in the 40s. About half of men show hair loss by 50 and four in five by 70. Greying starts in the mid-30s to mid-40s depending on ancestry. Source: JCAD review; Norwood data.
- The low-drift aging pipeline makes a few anchor ages from one identity and blends between them. Source: research synthesis.

- Hair type, colour, density, and hairline are genes; hairstyle is a choice that can change while the person stays recognisable. Source: reasoning on the genome model.

- FLUX.1 schnell has about 12 billion parameters, so even a compressed copy is a download of several gigabytes; SDXL-class models are about 7 GB. Source: https://huggingface.co/black-forest-labs/FLUX.1-schnell

- Segmind Vega and T2I-Adapter depth guides are both Apache-2.0, so a render-guided photo finish can ship in about 1.5 to 2 GB. Source: https://huggingface.co/segmind/Segmind-Vega
- SD 1.5 and its distilled forms such as BK-SDM Tiny are CreativeML OpenRAIL-M, which allows commercial shipping with the use-restriction notice passed on. Source: CompVis licence.
- SD-Turbo, SDXL-Turbo, Sana-Sprint, and the pretrained img2img-turbo weights cannot ship in a sold game without a paid agreement. Source: Stability licence; img2img-turbo repository.
- No public source gives speeds for this exact pass on consumer graphics cards, so the choice needs our own benchmark. Source: research pass.

- At 3,000 to 10,000 regens a season over 30 seasons, baking six ages of every face at about 40 KB is roughly 20 to 70 GB, so a save cannot store every face in advance. Source: estimate.

## What we are assuming
- Realism matters most on the player profile and the youth intake screen; squad lists show faces too small for realism to count. Confirmed: the profile sets the bar, and lists show the render first.
- The game plays offline, so a face cannot depend on a server at the moment it appears. Confirmed: faces render locally, and the finisher is a download.

## Tensions
- The settings text says identity and age are never AI, while the wrinkle and skin textures that show age are made by studio AI. Resolved: precise wording.
- The painted style is offered as a clean choice for players who avoid AI art, while it paints textures that studio AI made. Resolved: the game makes no AI claim about the painted mode.
- Every closed area keeps every decision in the first version, while one person reviews all art and no budget limits the first version. Resolved: no limit; the size is an accepted risk.
- Career-stage bands put the academy in one stage, while a face changes most from 15 to 21. Resolved: two academy stages.
- Faces outside the followed players are drawn on demand, while the finisher may be slow, so a list could stall. Resolved: lists show the render first, and the finisher upgrades faces in the background.
- The finisher decision says Vega first and our distilled finisher later, while the area close puts the distilled finisher in the first version. Resolved: Vega at launch, the distilled finisher later.
- Faces are photoreal by default, while the photo finish is an optional download, so a player without it never sees the default look. Resolved: our best render is the default, and the finisher is an upgrade.
- The product rule says illustration is generated in code, not sourced from image files, while studio AI textures would ship as image files. Resolved: textures ship as compact data that code expands.
- Spreads should follow footballers, while published ancestry data describes populations, and inferring the ancestry of real players from photos is not acceptable. Resolved: public heritage records shift the population base.
- Hair cards suit straight and wavy hair, while a large share of footballers have tightly coiled hair that cards render badly. Resolved: coiled hair gets dedicated authoring.
- The most photoreal route is generative AI, while AI art brings store disclosure and backlash. Resolved: disclosure accepted, open by design, and our render as the default.
- A library of finished images is the cheapest realism, but a finished image cannot age. Resolved: the genome route.
- AI tools that keep one identity need face-recognition weights that the licence rule forbids without payment. Resolved: the genome route needs no face-recognition model.
- Generation on the player's PC needs no server, but about half of players lack the graphics card. Resolved: the render is the default, and the finisher is optional.
- A first version whose identity lives only in a finished image makes aging later need a new identity basis. Resolved: every face stores an aging-ready identity from the first version.

## Questions still open
- Which tools that make an image model follow a render allow commercial use with FLUX.1 schnell or SDXL?
- Which licence covers FLAME skin textures, and do we need our own skin textures?

## Questions for the plan
- Does the whole game use real countries (leagues, international football, eligibility), and does the season design record it?
- Where does heritage-based international eligibility live in the season and international design?
- How does the portrait process fit the app package (a Tauri sidecar) and the packaging plan of one interface with two paths?
- Which game data drives weight after football and stress greying, and where is it designed?
- Is stubble a hair card or a skin texture inside the one beard system?

## Scope

## Work

## How to continue
- Resume: `/wf intake brainstorm brainstorm-regen-faces-and-aging-20260924`
- Control words: `park <thread>` · `pull <thread>` · `drop <thread>` · `board` · `look it up` · `second opinion` · `done`
- Retire when no thread is live: `/wf close brainstorm-regen-faces-and-aging-20260924`
