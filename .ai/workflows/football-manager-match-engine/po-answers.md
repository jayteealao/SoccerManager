# Product-owner answers — football-manager-match-engine

Cumulative log. Each entry carries a timestamp, the stage, the question, the answer, and a `scope:` line.

## 2026-09-21T16:46:41Z · intake · raw request

**Request (verbatim):** i want to build a football manager game with a html css and js frontend, the game engine and match engine do not need to be js, we can choose the most performative language or engine for hour use case. to start we need to build a very complex and detailed match engine like FM and a 2D match viewer

scope: states the product vision and the first deliverable; does NOT decide the engine language, the viewer rendering technology, or the success criteria.

## 2026-09-21T19:05:33Z · intake · Batch B (substance)

**Q1 Outcome and audience.** Who plays, and what is the first playable moment?
**A:** Multiple users who choose a lineup and play against a computer team. Football teams are modeled. Tactics and substitution are important parts of the gameplay experience.
scope: decides the actor (a human manager versus a computer team) and the core loop (choose lineup, play, substitute); does NOT decide multiplayer or league structure.

**Q2 What "like FM" means.** Which engine features must the first version have?
**A:** All of (a) attributes, (b) tactical instructions, (c) continuous positions every tick, (d) realistic statistics, (e) fatigue, injuries, substitutions, (f) set pieces, (g) commentary — and more. A realistic match experience.
scope: decides the feature floor of the match engine; does NOT decide the order of delivery.

**Q3 Success criteria.** How do you know the engine is good enough?
**A:** A reasonably good simulation. A debug observability pipeline will be created with `/wf observability`. The PO asked: what do we need to monitor to model a good simulation?
scope: decides that success is measured through an observability pipeline; does NOT yet give falsifiable thresholds — parked as an open question with proposed metrics.

**Q4 Constraints and decided choices.** Where must the engine run, language preference, timeline, project type?
**A:** No preferences. Priorities: performance, maintainability, depth, customizable — not limited to these.
scope: decides the priority set; does NOT decide runtime location (browser, native, server), language, timeline, or commercial status.

**Q5 Stack confirmation.**
**A:** Stack will be updated as languages and frameworks are decided. Not needed: zread, web-search-prime, consult. Needed: the Claude design artifact skill (artifact-design, artifact-diagramming, artifact-capabilities).
scope: decides which session tooling is in and out of use; does NOT decide the product stack.

## 2026-09-21T19:06:11Z · intake · Batch A (process gates)

**Q Branch strategy.** Dedicated / Shared / None.
**A:** Dedicated. Rung 1 (AskUserQuestion).
scope: decides git management for this workflow; does NOT decide slice boundaries.

**Q Appetite.** Large / Medium / Small.
**A:** Large. Rung 1 (AskUserQuestion).
scope: decides the pre-mortem horizon, slice-count expectation, and consult trigger; does NOT decide a deadline.

**Q Base branch.** main / develop.
**A:** main. Feature branches named `feat/<slice>`. Rung 1 (AskUserQuestion).
scope: decides the merge target; does NOT create the branch (the first commit does).

## 2026-09-21T19:09:35Z · intake · Charter ratification and success metrics (Step 6b)

**Q Charter.** Confirm or correct the commitments C1–C6.
**A:** All six confirmed: C1, C2, C3, C4, C5, C6. Rung 1 (AskUserQuestion, multi-select).
scope: ratifies the charter; does NOT decide delivery order or slice boundaries.

**Q Metrics.** Which proposed metrics are the initial success criteria?
**A:** All four accepted: realism statistics, performance, viewer fidelity, positional sanity. Thresholds are starting values; shape refines them.
scope: decides what the observability pipeline measures first; does NOT fix the final thresholds.

**Consult note.** Triggers `new-capability` and `appetite-medium-or-larger` held. The PO excluded `consult` in Batch B, so no consult ran. Recorded, not fired.

## 2026-09-21T19:18:09Z · shape · Round 1 (what the feature does)

**Q1 Live or replay.** How does the match run in relation to in-match decisions?
**A (freeform):** Changes (tactics or substitutions) are applied at the next stoppage: throw-in, corner, goal kick, and normal stoppages in play that allow a substitution or where coaches pass tactical information. The engine advances tick by tick until the next stoppage.
scope: decides when manager changes take effect (stoppage-gated); does NOT decide whether the viewer plays ticks as the engine produces them or after a segment completes (asked in Round 2).

**Q2 Lineup input.** Eleven plus bench from a squad.
scope: decides the pre-match input; does NOT decide squad size beyond "about 20 to 25".

**Q3 Tactics depth.** Tier 2: formation, mentality, 6 to 8 team instructions, per-player roles and duties.
scope: decides the v1 tactics model; does NOT exclude Tier 3 later; the data model must leave room.

**Q4 AI opponent.** Reactive AI manager: pre-match setup plus in-match mentality changes and substitutions; one module manages both teams in headless runs.
scope: decides that an AI manager module exists in v1; does NOT decide its decision quality.

## 2026-09-21T19:19:47Z · shape · Round 2 (how the feature behaves)

**Q5 Segment play.** Stream ticks live; the viewer draws each tick as the engine produces it. The engine must run faster than the fastest playback speed.
scope: decides the transport model (live tick stream); does NOT decide the wire format.

**Q6 Determinism.** Best effort. Matches are random each run; replays store the full tick stream.
scope: decides that seed-replay is not a v1 requirement; does NOT decide the random-number source. Consequence to confirm: rewind and observability re-runs depend on stored tick streams, not on seeds.

**Q7 Tick rate.** 50 ticks per second (20 ms per tick). 270,000 ticks for a 90-minute match.
scope: decides simulation fidelity and stream size; does NOT decide the viewer frame rate.

**Q8 Playback controls.** All four: speed 1x to 8x, pause and resume, skip to next stoppage, rewind and replay a goal.
scope: decides the viewer control set; does NOT decide how much tick history the viewer keeps (asked in Round 4).

## 2026-09-21T19:22:15Z · shape · Round 3 (what the feature looks like)

**Q9 Engine home.** Local native process plus browser page. The engine is a native program on the user's machine; it serves ticks to the page over a local socket. Desktop only; users install and run a program.
scope: decides the runtime location (resolves OQ-1 and RIM-3); does NOT decide the language (asked in an extension round) or the socket protocol.

**Q10 Match screen.** Full match-day screen: pitch, clock and score, event feed, lineups with fatigue and condition, live statistics panel.
scope: decides the viewer content inventory; does NOT decide layout or visual direction (Round 3b).

**Q11 States.** Pre-match lineup and tactics; stoppage tactics and substitution panel; half-time and full-time reports; error and recovery; PLUS (PO correction): a pause state, and the manager can change tactics and make substitutions while paused or while the game runs. Changes apply at the next stoppage in the game, and the qualifying stoppage depends on whether the change is a tactics change or a substitution.
scope: decides the state inventory and that the tactics panel is available at any time; does NOT yet name which stoppage kinds admit a substitution versus a tactics change (asked in Round 4).

**Q12 Team data.** Fictional generated teams with realistic attribute distributions; the calibration runs use the same generator.
scope: decides the v1 data source and removes the real-names licensing risk; does NOT decide the attribute schema.

## 2026-09-21T19:24:55Z · shape · Round 3b (visual direction)

**Q13 Register.** Expressive broadcast: television-graphics energy, bold score bugs, animated event banners, team colors.
scope: decides the design register for the match screen; does NOT decide the register for later manager screens.

**Q14 Color and scene.** Committed, light scene: light panels, a committed brand color on headers and controls, team colors on the pitch. Scene sentence changed from the draft: the manager plays in daylight; the screen reads well in a lit room.
scope: decides the color strategy and light theme; does NOT decide the brand hue.

**Q15 References.** Football Manager 2D classic view; broadcast tracking graphics (Opta, Second Spectrum); trading terminal or telemetry dashboard for the numbers. Anti-goals: cluttered FM 3D overlays, television sponsor clutter, arcade or cartoon styling.
scope: decides anchor references and anti-goals; does NOT lock a specific mock.

**Q16 State weight.** All four: live play and goal moments; stoppage intervention; pre-match lineup screen; loading, error, and recovery.
scope: decides which states get a design note; does NOT specify the treatments.

## 2026-09-21T19:27:06Z · shape · Round 4 (what can go wrong)

**Q17 Engine crash.** Restart from the last stoppage. The engine snapshots match state at every stoppage; the viewer restarts the engine and resumes from the snapshot.
scope: decides the recovery behavior and requires a snapshot format; does NOT decide the snapshot storage location.

**Q18 Engine lag.** Auto-reduce playback speed to the highest sustained rate and show a notice.
scope: decides the degraded-speed behavior; does NOT set the minimum supported hardware.

**Q19 Substitution windows.** PO answer: "based on real life", with all listed stoppage kinds selected. Real-life rule: a substitution may happen at any stoppage in play once the ball is dead (throw-in, goal kick, corner, free kick, goal, injury stoppage, half-time), with the referee's permission. Tactics changes apply at any stoppage.
scope: decides which stoppages admit a substitution; does NOT decide the substitution count limit (asked in Round 5).

**Q20 History.** Whole match kept in a compact binary encoding; rewind to any moment; the full-time report can save the replay.
scope: decides viewer memory policy and that replays are full tick streams; does NOT decide the encoding.

## 2026-09-21T19:28:28Z · shape · Round 5 (boundaries)

**Q21 Rules in v1.** All four: offside; fouls with yellow and red cards; substitution limit of 5 changes in 3 windows plus half-time; stoppage time and extra time with penalties.
scope: decides the laws the engine enforces in v1; does NOT decide VAR or other officiating aids (out of scope).

**Q22 Commentary.** Context-aware lines: templates plus score state, minute, in-match form, and repeated-event awareness.
scope: decides commentary depth; does NOT add languages other than English.

**Q23 Project type.** Open source. Dependencies must be license-compatible; contributors expect docs. No timeline given.
scope: decides licensing posture (resolves OQ-4 type); does NOT set a deadline (none held).

**Q24 Augmentations.** Benchmark the engine; instrument for observability; feature flags for experiments. Profiling not reserved.
scope: decides `augmentations-needed`; does NOT design the signals or flags.

## 2026-09-21T19:31:17Z · shape · Extension round 1

**Q25 Language.** Rust. Native process; the same code can compile to WebAssembly later.
scope: decides the engine language (resolves AMB-2); does NOT decide crates or the socket protocol.

**Q26 Attributes.** PO answer: about 30 to 50 attributes, rated 1 to 100, custom schema, data-driven. The attribute list lives in a configuration file; the engine maps roles to attributes through data and ships with a default set.
scope: decides the attribute model shape and scale; does NOT name the default attribute list.

**Q27 Multi-user.** Networked multiplayer later, design for it now: independent single-player sessions in v1; the data model reserves user identity and match ownership so a server can be added without a rewrite.
scope: decides v1 is single-player and reserves identity fields (resolves OQ-3, RIM-2); does NOT put a server or accounts in v1.

**Q28 Customize.** Full modding: scripts and rule packs. Users can script behaviors and swap rule packs.
scope: decides the customization ceiling; does NOT decide whether scripting ships in the first deliverable (asked in extension round 2).

## 2026-09-21T19:33:24Z · shape · Extension round 2 and verification tooling (Step 3)

**Q29 Verification tooling.** In-app browser (Claude_Browser) now; Playwright added to the repository once the viewer has stable markup; engine slices verified by cargo tests and headless benchmark runs.
scope: decides the verification drivers per phase; does NOT decide the CI provider.

**Q30 License.** MIT or Apache-2.0 (permissive). GPL code (for example FootballEngine) may be read, not copied.
scope: decides the license posture and the dependency rule; does NOT pick between MIT and Apache-2.0 (plan or handoff picks; both are acceptable).

**Q31 Modding timing.** Boundary now, scripts later: the first deliverable exposes tuning constants, team data, and rule packs as data files and defines the plugin interface; a scripting runtime is its own later slice.
scope: decides what customization ships first; does NOT drop full modding (it stays a later slice).

**Q32 Seeded RNG.** PO answer: matches can be seeded for testing purposes; real games are open to non-deterministic randomness. The engine owns a seedable random-number generator; a test or calibration run passes a seed; a real match draws a fresh seed.
scope: decides the RNG design (seedable, engine-owned); does NOT promise cross-machine identical replays.

## 2026-09-21T19:34:57Z · shape · Pre-mortem adjudication and consult record (Step 9)

**Blind pre-mortem returned:** two post-mortems and three candidates.
- Post-mortem 1 (interpolation invents movement) → RIM-5, adjudicated by the named mechanism "tick-bounded interpolation" (02-shape.md#desired-behavior, AC-21).
- Post-mortem 2 (modding wall never clears) → RIM-6, adjudicated by a named later slice and the Definition of Done wording (02-shape.md#dependencies-sequencing-notes).
- Candidate: identity fields never threaded → dismissed with citation AC-18.
- Candidate: performance metric gamed by parallel runs → dismissed with citation AC-19 (one match, one thread).
- Candidate: stoppage gating hard-coded → dismissed with citation: named mechanism "rule pack" (02-shape.md#desired-behavior).

**Consult record.** Triggers `new-capability`, `multi-slice`, and `rim-severity-high` held this stage. The PO excluded `consult` (intake Batch B), so no consult ran. Recorded, not fired.

## 2026-09-21T19:50:41Z · slice · Round 1 (slicing strategy)

**Q1 Order.** Engine core first, then the viewer on a recorded fixture. The performance risk retires first; nothing is on screen until slice 3.
scope: decides delivery order; does NOT decide slice contents.

**Q2 Granularity.** Thin: about 12 buildable slices plus deferred ones.
scope: decides slice size and review cadence; does NOT decide review scope (asked next).

**Q3 Visible milestone.** After engine core, protocol, and viewer pitch: crude play on screen driven by the live engine, before rules and tactics.
scope: decides which slice carries the charter-scenario milestone AC; does NOT change the final slice's full-scenario AC.

**Q4 Defer.** Deferred as named later slices: scripting runtime; installer and packaging for other operating systems; extra time and penalty shoot-outs; feature flags for calibration experiments.
scope: narrows the first release only; every deferred item stays in the roster as a named slice. Extra time and penalties narrow Q21 for the first release by this explicit PO choice.

## 2026-09-21T19:50:41Z · slice · Round 2 (review scope and coupling)

**Q5 Review scope.** Slug-wide: one review report against the cumulative branch diff at handoff.
scope: sets `review-scope: slug-wide` and `review-scope-confirmed: true`; does NOT change per-slice verify.

**Q6 Coupling.** Commentary is its own slice; the snapshot-at-every-stoppage mechanism rides with the match-rules slice.
scope: decides two slice boundaries; does NOT change acceptance criteria.

**Consult record.** Slice triggers held (roster larger than 3, a slice carries the charter-scenario AC). The PO excluded `consult` at intake; no consult ran.

## 2026-09-21T20:48:16Z · design setup · Round 1 (design context)

**Q1 Brand words.** Pitchside, tabular, broadcast. Rung 1 (AskUserQuestion).
scope: decides the brand-personality words in PRODUCT.md; does NOT decide the palette or the typeface.

**Q2 First five seconds.** The match is live and I am in charge: the pitch moves at once, the clock runs, the tactics control is visible without scrolling.
scope: decides the tone statement and the first-screen priority; does NOT decide the layout.

**Q3 Brand assets and hue.** First answer: "Assets exist". Follow-up answer: "None but need to plan to build and generate one fully in javascript" and "Not files, you create them". Recorded as: no brand assets exist; the logo, the palette, and any illustration are generated programmatically in JavaScript as part of the product. The design brief hue 250 stays a working choice, not a brand decision.
scope: decides that no assets exist and that generation is programmatic in JavaScript; does NOT decide the brand hue, and does NOT schedule the generation work in the slice roster.

**Q4 Product name.** No name yet; PRODUCT.md carries a `[TODO]` marker by PO choice.
scope: decides that the name stays open; does NOT decide the page title. Note: the design preflight treats a `[TODO]` marker as invalid context, so the plan stage for the first viewer slice stops until the marker is removed.

**Pre-filled answers (not asked; source recorded):**
- Register: product — 02b-design.md `register: product`; shape Round 3b Q13.
- Product purpose and users: a human manager picks a lineup and tactics and plays one match against a computer team — intake Batch B Q1; 02b-design.md section 2.
- User context and expertise: at a desk in daylight, 20 to 90 minutes per match, fluent in football vocabulary — 02b-design.md section 2; "like FM" in the raw request.
- Emotional tone: expressive broadcast energy inside a product register; engaged and mildly tense — shape Round 3b Q13; 02b-design.md section 2.
- Positive references: Football Manager 2D classic view; Opta and Second Spectrum tracking graphics; telemetry or trading-terminal dashboard — shape Round 3b Q15.
- Anti-references: Football Manager 3D overlays; television sponsor clutter; arcade or cartoon styling — shape Round 3b Q15; 02b-design.md section 4.
- Technical constraints: plain HTML, CSS, and JavaScript frontend; no framework or component library chosen; Rust engine over a local socket — raw request; 00-index.md `stack.ui`; shape Round 3 Q9.
- Existing design documentation: none in the repository; the design brief and its token file are the only design records — codebase inspection (empty repository).
- Off-limits: no real team names (shape Round 3 Q12); permissive licenses only, no GPL copying (02-shape.md NFR-5); light scene (shape Round 3b Q14).

**Q5 Confirmation.** PRODUCT.md and DESIGN.md confirmed as written; the [TODO] name marker stays by PO choice. Rung 1 (AskUserQuestion).
scope: makes both files the design context for every later design command; does NOT remove the [TODO] marker, so the plan stage for the first viewer slice stops until a name is written.

## 2026-09-21T21:34:34Z · observability init · Round 1 (inventory and forks)

**Q1 Inventory.** Confirmed: the repository holds no source, no logger, no dashboard; the specified signals come from the shape. Rung 1 (AskUserQuestion).
scope: confirms the discovery state; does NOT add signals.

**Q2 Backend.** Local files plus an in-repo debug dashboard: JSON Lines in a local data folder; a browser dashboard page in the plain JavaScript stack; DuckDB SQL as an optional analyst path.
scope: decides Block F (backend and query layer); does NOT decide the data folder default path.

**Q3 Transport.** File sink plus socket feed: the engine writes files at every stoppage snapshot and at match end, and sends every event over the existing local socket; headless runs use the file sink only. Resolves shape U-2.
scope: decides Block E (pipeline) and closes U-2; does NOT decide the wire encoding (U-1 stays with the stream-protocol plan).

**Q4 Retention.** Keep every record; cap the folder at 2 GB; prune the oldest tick histories first; calibration runs keep the report, every statistics record, and the event streams of outlier and failed matches only.
scope: decides Block D (sampling and cost posture); does NOT decide the pruning schedule.

## 2026-09-21T21:34:34Z · observability init · Round 2 (confirmation)

**Q5 Confirm.** Confirmed: Blocks A (four record kinds, dotted keys, owner.id as the actor key), B (records at 100 percent, per-tick telemetry summarized), C (opaque owner.id, hashed machine, no absolute paths, no free text), F (six dashboard analyses), G (emit-iac ceiling: files in the repository only), H (viewer in scope over the socket), and four additional contracts (realism bands, benchmark tripwires, dark paths, schema versioning). Rung 1 (AskUserQuestion).
scope: locks `.ai/observability.md` plan-version 1; does NOT update the workflow index (U-2 closure is recorded here for the stream-protocol and calibration plans to cite).

**Consult record.** Triggers (new vendor or cost; unredacted identity; conflict with a ship-plan) did not hold: no vendor, owner.id is opaque, no ship-plan exists. The PO excluded `consult` at intake. Not fired.

## 2026-09-21T21:57:49Z · plan (engine-core) · Round 1 (data formats and command line)

**Q1 Tick file format.** Binary records with 32-bit floats: a small header plus one fixed-size record per tick, written with the standard library; about 51 MB per match; a `--json` flag dumps a readable form. Rung 1 (AskUserQuestion).
scope: decides the provisional tick file layout for this slice; does NOT decide the socket wire encoding (U-1 stays with the stream-protocol plan).

**Q2 Ball height.** Yes: the ball state holds x, y, and z with gravity and bounce restitution from the first slice; the tick record carries z.
scope: decides the ball model dimensionality; does NOT decide spin (still out of scope) or how the viewer draws height.

**Q3 Float width.** 64-bit inside the simulation, 32-bit in the tick file.
scope: decides numeric precision for the engine and the file; does NOT promise cross-machine reproduction (NFR-4 unchanged).

**Q4 CLI parsing.** clap with derive (cached 4.5.60; current 4.6.7; MIT OR Apache-2.0).
scope: decides the argument parser; does NOT decide the command set beyond simulate and bench.

## 2026-09-21T21:57:49Z · plan (engine-core) · Round 2 (performance levers and the benchmark)

**Q5 Decision rate.** 50 decisions per second: every agent re-decides every tick. PO chose the most responsive and most expensive option over the recommended 10 per second.
scope: sets the decision cadence constant in the tuning defaults and the benchmark baseline; does NOT forbid lowering the rate if the benchmark misses (that reopens with the PO under NFR-1 yields-to C2).

**Q6 Machine identity.** Hashed hostname (`machine.hash`, SHA-256 of the computer name, 12 hex characters) plus the CPU model string. The slice criterion "machine name" is read as satisfied by the hashed identity; the observability contract Block C stays intact.
scope: resolves the conflict between the slice AC wording and `.ai/observability.md` Block C for the benchmark report; does NOT change either document's other fields.

**Q7 Benchmark harness.** Both: the `engine-cli bench` command (standard-library clock plus Win32 CPU-time and peak-memory calls through windows-sys, one JSON run-report record) for the acceptance criterion, and a criterion bench target for the tick step and the steering step.
scope: decides the benchmark tooling; does NOT change the NFR-1 budget or the tripwires.

**Q8 Test corpus.** Full match in integration tests: unit tests use 5-minute runs; one integration test per criterion runs the full 270,000 ticks with seed 42.
scope: decides test scope and suite duration; does NOT decide the CI runner (none exists).

## 2026-09-21T21:57:49Z · plan (engine-core) · Round 3 (conventions)

**Q9 Edition and lints.** Edition 2024; `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` block verify.
scope: sets the workspace edition and the lint gates; does NOT add a CI service.

**Q10 License.** Dual MIT OR Apache-2.0: two license files and the SPDX expression in every Cargo.toml.
scope: fixes the repository license (NFR-5); does NOT decide attribution for calibration data sources.

**Q11 Vector math.** glam 0.33 `DVec2` and `DVec3` (f64, scalar, MIT OR Apache-2.0). PO chose the library over the recommended hand-written type.
scope: decides the vector math dependency; does NOT decide the physics integrator.

**Q12 Errors.** thiserror in the library (typed `EngineError`), anyhow in the binary.
scope: decides the error model; does NOT decide message wording.

**Pre-filled (not asked; source recorded):** crate layout `crates/engine` + `crates/engine-cli` (03-slice-engine-core.md Scope; `.ai/observability.md` Block B paths); RNG `rand_chacha::ChaCha8Rng::seed_from_u64` with rand 0.10 (research: the Rand Book names ChaCha as the reproducible generator; StdRng is not version-pinned); ordered collections `BTreeMap` or `Vec` with explicit index order, never `HashMap` iteration in the simulation path (research finding 3; slice risk 3); spatial queries brute force over 22 agents (484 pairs per tick; a grid is a later lever); build hash from `build.rs` running `git rev-parse --short HEAD` with a `-dirty` suffix (research capability 7); dependency versions latest stable (serde_json is not cached, so the first build needs crates.io either way).

**Consult record.** Trigger `appetite-medium-or-larger` holds (appetite large). The PO excluded `consult` at intake; not fired.

**Design contract.** `02b-design.md` exists without `02c-craft.md`. The contract is not authored in this plan: the slice has no user-interface surface, and the contract build gate requires PRODUCT.md without a `[TODO]` marker, which the PO chose to keep (design setup Q4/Q5). The `viewer-pitch` plan authors the contract once a name is written.

## 2026-09-22T06:26:14Z · plan · data-schemas-generator Round 1 (loading and identity)

**Q1 Validation.** PO answer: garde declarative validation. Structs derive `Deserialize` and carry `#[garde(...)]` range and length rules; garde 0.23 (MIT/Apache-2.0) reports the field path; the loader wraps garde's report with the file path. Rung 1 (AskUserQuestion).
scope: decides the validation mechanism and adds one dependency; does NOT decide the file format (JSON is fixed by the slice criterion `--team-a a.json`) and does NOT decide error wording.

**Q2 Default data.** PO answer: on-disk data folder only. The binary reads every content file (attribute schema, tuning, teams, rule pack) from a content folder on disk; no embedded copy; a missing file exits non-zero naming the path. Rung 1.
scope: decides where shipped content lives and that a missing file fails; does NOT fix the folder's name or the flag that points at it (plan Round 3 confirms `content/` versus `data/` to keep the observability contract's "data folder" for SM_DATA_DIR only).

**Q3 Match record.** PO answer: also write the identifiers into the tick-file header. The match-stats record is saved as `stats.json` and loaded back, and `owner.id` and `match.id` also enter the reserved bytes of the tick-file header and are read back by `read_ticks`. Rung 1.
scope: decides that both the stats file and the tick header carry identity; does NOT decide the wire encoding (U-1 stays with stream-protocol) and does NOT change the 192-byte record layout.

**Q4 Owner id.** PO answer: generate and persist an id now. On first run the binary creates `owner.id` in the runtime data folder (SM_DATA_DIR) with a random opaque value and reuses it on later runs. Rung 1.
scope: decides that headless runs carry a real per-machine identifier from this slice; does NOT decide accounts, servers, or the viewer's handling of the id (RIM-2 stays adjudicated as single-player).

## 2026-09-22T06:31:26Z · plan · data-schemas-generator Round 2 (generator and tests)

**Q5 Distributions.** PO answer: bell curve from summed uniform draws, no new dependency. `EngineRng` gains a `bell(mean, spread)` method (Irwin-Hall approximation, clamped to 1..100); per-position means and spreads live in the tuning file. Rung 1 (AskUserQuestion).
scope: decides the sampling method and keeps the dependency set unchanged; does NOT fix the per-position numbers (the calibration slice tunes them).

**Q6 Generator CLI.** PO answer: library API plus `engine-cli generate`. The library exposes league generation; the binary adds `generate --seed N --clubs 20 --out <folder>` writing one team file per club. Rung 1.
scope: decides the generator's two entry points; does NOT decide league or season structure (out of scope per the shape).

**Q7 Existing tests.** PO answer: default team files replace the built-ins everywhere. `Team::builtin` and `Attributes::uniform(60)` are deleted; `simulate` with no team flags loads the two shipped default teams; the three engine-core tests re-record their bounds; the benchmark re-baselines. Rung 1.
scope: decides one code path for teams and accepts a moved benchmark line; does NOT relax any engine-core criterion (270,000 ticks, byte-identical output, zero violations still hold).

**Q8 Tuning bounds.** PO answer: bounds in code, values in the file. A garde range rule per field holds the bound; the shipped file holds values only; a companion reference page lists field, unit, default, and bound. Rung 1.
scope: decides where bounds live; does NOT decide the bound values (the plan carries the engine-core defaults with a stated margin).

## 2026-09-22T06:33:39Z · plan · data-schemas-generator Round 3 (vocabulary, folders, augmentations)

**Q9 Positions.** PO answer: ten detailed positions: GK, CB, LB, RB, DM, CM, AM, LW, RW, ST. Each formation slot names one position; the generator draws per-position means. Rung 1 (AskUserQuestion).
scope: decides the position vocabulary in team files and the rule that a slot names one position; does NOT decide roles or duties (the tactics slice maps roles onto these codes).

**Q10 Folders.** PO answer: `content/` beside the binary (flag `--content-dir`) for shipped files; SM_DATA_DIR for runtime writes, defaulting to `%LOCALAPPDATA%\SoccerManager` on Windows. Rung 1.
scope: decides the two folder names and defaults; does NOT decide packaging or other operating systems (U-3).

**Q11 Augmentations.** PO answer: re-author both `04b-instrument.md` and `05c-benchmark.md` for this slice, with the engine-core versions byte-copied into `history/` first; the benchmark re-baselines on the measured engine-core numbers. Rung 1.
scope: decides the augmentation artifacts for this slice; does NOT change the tripwire thresholds (10 percent CPU, 25 percent memory).

**Pre-filled (not asked; source recorded):** file format JSON with `serde_json` (the slice criterion names `a.json`; `toml` is not in the registry); schema version as an integer field peeked before full deserialization so an unknown version reports "file X carries schema version N; this build reads version M" (research: a tagged enum cannot phrase a newer-than-loader message); the default attribute list fixed by the plan at 36 attributes in four groups (the slice assigns the list to plan); kit colors as hex strings; identifiers as strings (`sha2` already installed; `uuid` not in the registry); the tick-file record layout unchanged at 192 bytes.

**Consult record.** Trigger `appetite-medium-or-larger` holds (appetite large). The PO excluded `consult` at intake; not fired.

**Design contract.** `02b-design.md` exists without `02c-craft.md`. Not authored in this plan: the slice has no user-interface surface and PRODUCT.md keeps its `[TODO]` marker by PO choice; the `viewer-pitch` plan authors it.

## 2026-09-22T11:28:51Z · plan (stream-protocol) · Round 1 (transport, encoding, layout, pacing)

**Q1 Transport.** `tungstenite` 0.30, the blocking WebSocket library, on `std::net`. A browser page reaches a local program over WebSocket or HTTP only; a raw TCP stream and a Windows named pipe are unreachable from page code, so the "TCP on loopback" default in the slice needs WebSocket framing above it. The product owner added: decide synchronous against asynchronous now (answered in Round 2 Q5). Rung 1 (AskUserQuestion).
scope: decides the wire transport and the one new dependency tree; does NOT decide the tick encoding (Q2) or the concurrency model (Q5).

**Q2 Tick encoding.** Binary keyframe plus delta, closing shape unknown U-1. A 98-byte keyframe every 50 ticks (tick `u32`, ball three `i16` in centimetres, 22 players two `i16` in centimetres) and 47-byte delta frames of `i8` centimetre steps between them; about 48 bytes per tick and 13 MB per match against 51.8 MB raw and a 300 MB budget (NFR-3). Rung 1 (AskUserQuestion).
scope: closes U-1 and fixes the tick frame layout; does NOT decide the control-channel encoding (JSON by pre-fill) or the fixture file layout (Q6).

**Q3 Crate layout.** Two new crates. `crates/protocol` holds the message types, the tick codec, the version constant, and the document-enumeration test, with no input or output and no network dependency. `crates/stream` holds the server, the socket sink, the bounded buffer, the recorder, and the replayer. `crates/engine` stays network-free, which keeps the later WebAssembly build open (RIM-3) and honours NFR-8. Rung 1 (AskUserQuestion).
scope: decides package boundaries; does NOT decide the module names inside either crate.

**Q4 Pacing.** Client-pull, bounded. The engine simulates as fast as the client reads, up to a bounded lead; the read rate of the client is the only pace. `set speed` is stored and echoed in acknowledgements so the viewer paces its own playback; `pause` stops production and `start` resumes it. The mock replayer paces itself at speed times 50 frames per second because it has no simulation to run. Rung 1 (AskUserQuestion).
scope: decides the live-stream pacing semantic; does NOT decide the bound (Round 3 Q10) or the buffer inside the viewer.

## 2026-09-22T11:28:51Z · plan (stream-protocol) · Round 2 (concurrency, fixture, events, commands)

**Q5 Concurrency.** Blocking, one thread per client, `tungstenite` on `std::net::TcpListener`, `std::sync::mpsc::sync_channel` as the bounded buffer. Version 1 serves one viewer per match and the observability contract adds the debug dashboard later, so two or three connections in total; `tokio` would add about twenty-five crates and an executor for concurrency the engine does not have, and the simulation loop would run on `spawn_blocking` regardless. Rung 1 (AskUserQuestion).
scope: decides the concurrency model and closes the Round 1 Q1 follow-up; does NOT forbid a later change, which the split into `crates/protocol` and `crates/stream` contains to `crates/stream`.

**Q6 Fixture format.** Capture the wire bytes. `engine-cli record` writes every frame exactly as the server would send it, hello and control messages included, each with its tick index, in one new file; replay writes the stored bytes back out, so byte-identity holds by construction and no re-encoding happens on replay. Rung 1 (AskUserQuestion).
scope: decides the fixture layout and the replay mechanism; does NOT change the `.ticks` file, which keeps its validator and benchmark job.

**Q7 Event stream.** Envelope plus the events that exist. The event message follows the key vocabulary of the observability contract, and the engine emits the three it can produce today: kick-off or restart, goal, and full-time. The restart event retires the `sdlc-debt:` marker at `crates/engine/src/validate.rs:77`, where the validator reads any ball jump above two metres as a restart because the tick record carries no restart flag. Rung 1 (AskUserQuestion).
scope: decides which event types this slice emits; does NOT decide fouls, cards, or set-piece events, which `match-rules` owns, nor commentary text.

**Q8 Change queue.** Hold the queue, apply nothing. The server validates a change type against the loaded rule pack, assigns a queue identifier, keeps the change in an ordered queue, and reports it as pending; nothing reaches play until `match-rules` lands, and the identifiers are then already in place. Rung 1 (AskUserQuestion).
scope: decides the queue semantics of the command channel; does NOT decide when a change applies, which is the stoppage-gated queue `match-rules` builds.

## 2026-09-22T11:28:51Z · plan (stream-protocol) · Round 3 (discovery, bound, version, benchmark)

**Q9 Port and access.** Operating-system-assigned port with an Origin allowlist. The server binds `127.0.0.1:0`, writes the chosen port to `SM_DATA_DIR/engine.port` and to stderr, and refuses any handshake whose `Origin` header is not `null`, `file://`, or `http://localhost`. No port clash, and a stray page in the same browser cannot drive a match. Rung 1 (AskUserQuestion).
scope: decides port discovery and the connection guard; does NOT add a session token, which the Origin check covers on a single-player local machine.

**Q10 Buffer bound.** 500 ticks, held in `content/tuning.json`. Ten simulated seconds of lead, about 50 kilobytes per client; a browser stall never pauses the engine needlessly, and the throttled-client test still exercises the pause-and-resume path. A modder or a later slice changes the value without a rebuild. Rung 1 (AskUserQuestion).
scope: decides the bound and its home; does NOT decide the socket write-buffer bound inside `tungstenite`, which is an implementation detail set to a few keyframes.

**Q11 Protocol version.** An independent `PROTOCOL_VERSION` integer, sent in the hello message, with a fail-closed refusal that names both versions in the wording the tick file already uses. The reference document is `docs/reference/protocol.md`, the target the Documentation Plan of the shape names, and the enumeration test parses its message tables. Rung 1 (AskUserQuestion).
scope: decides the version constant and the document location; does NOT decide the prose structure of the document beyond a parseable message table.

**Q12 Benchmark.** Re-baseline and add a stream target. The four existing targets are re-measured on the current commit before any code change, and a fifth is added: ticks per second delivered over the socket to an unthrottled client. Verify compares all five; the stream number is the evidence for the 8x-playback criterion and the baseline for `viewer-pitch`. Rung 1 (AskUserQuestion).
scope: decides the benchmark scope for this slice; does NOT change the tripwires (10 percent processor time, 25 percent memory).

**Pre-filled (not asked; source recorded):** the control channel is JSON text frames through the installed `serde_json` (research: MessagePack and CBOR add a dependency and a second wire format for a low-rate channel); `TCP_NODELAY` on every accepted connection (research: Nagle coalesces small frames); `tungstenite` default features only, no transport-layer security, because the server binds loopback; the bind address is `127.0.0.1`, never `0.0.0.0`, which also avoids a Windows Firewall prompt; the hello message reuses `engine::version()`, `engine::build_hash()`, `owner.id`, and `MatchId` rather than new identifiers.

**Consult record.** Triggers `appetite-medium-or-larger` and `unknowns-present` (U-1) hold. The PO excluded `consult` at intake; not fired.

**Design contract.** `02b-design.md` exists without `02c-craft.md`. Not authored in this plan: the slice ships no user-interface surface; the `viewer-pitch` plan authors it.

## 2026-09-22T14:41:32Z · plan (viewer-pitch) · Round 1 (the page-to-engine boundary)

**Q1 Page delivery.** The engine serves it. Add a `--web <dir>` option to `engine-cli serve` and `engine-cli replay` so the Rust binary serves the page files beside the socket it already opens. Research fact that forced the question: a browser blocks JavaScript module scripts outright over `file://` (CORS, opaque origin `null`), so the page cannot be a double-clicked file. One program, one address, and `http://127.0.0.1:<port>` is already on the server's Origin allowlist (`crates/stream/src/server.rs:181`). Rung 1 (AskUserQuestion).
scope: decides how the page reaches the browser and which program serves it; does NOT decide the page folder name, the MIME table, or the installer (U-3 keeps packaging parked).

**Q2 Lag notice.** The page measures, and `engine-cli replay` gains a `--sustain <f32>` cap option for testing. The page times tick arrival against the speed it asked for, drops its own playback speed to the rate it is really receiving, and names that rate in the notice. No wire-format change. Research fact that forced the question: `ReplayOpts` has only `--fixture` and `--speed` (`crates/engine-cli/src/cli.rs:106-114`); `Replayer::serve` never reads the socket (`crates/stream/src/replay.rs:64-105`); no `ServerMessage` announces lag (`crates/protocol/src/lib.rs:85-185`); and the browser exposes no inbound-queue depth, because `WebSocket.bufferedAmount` counts outgoing bytes only. Rung 1 (AskUserQuestion).
scope: decides who detects lag and how the notice gets its number, and closes the AC-5 force-scope wall; does NOT decide the threshold at which the notice appears, nor the notice wording.

**Q3 Kit colours.** Add them to the opening message. `TeamRef` gains the club's primary and secondary kit colour beside `team.id` and `team.name`. Research fact that forced the question: `TeamRef` carries identifier and name only (`crates/protocol/src/message.rs:11-18`), so no kit colour reaches the page today, while `steer.md` requires the marker ring to use the kit secondary. Rung 1 (AskUserQuestion).
scope: decides where the page learns kit colours; does NOT decide how a colour is mapped to a safe on-pitch value (Round 3 Q11), and does NOT bump `PROTOCOL_VERSION`, which stays a separate judgement in the plan steps.

**Q4 Skip target.** Both, merged. Build one stoppage index from restart-flagged tick frames (`KIND_RESTART = 0x03`, `crates/protocol/src/frame.rs:16`) and from event-message ticks, and jump to whichever comes first, with a rule that collapses a goal and its restart into one entry. Rung 1 (AskUserQuestion).
scope: decides what "next stoppage" means to the skip control; does NOT decide the collapse window in ticks, which the plan fixes as an implementation detail.

## 2026-09-22T14:41:32Z · plan (viewer-pitch) · Round 2 (inside the page)

**Q5 Where drawing runs.** Main thread now, with the renderer in its own module so a later move to a worker is contained. Research fact: 22 circles, 22 shirt numbers, a ball and a trail at 616 by 411 pixels is sub-millisecond drawing against a 16.6-millisecond frame budget, and decoding 400 frames per second is also sub-millisecond; `transferControlToOffscreen()` is one-way, after which the main thread can never draw to that canvas, which would push the three test-hook criteria across a message boundary. Rung 1 (AskUserQuestion).
scope: decides where rendering runs in this slice; does NOT forbid a later worker move, which the module split keeps contained, and does NOT decide the canvas pixel-ratio handling.

**Q6 Rewind history.** Decoded positions. Every tick is decoded into absolute `Int16Array` arrays: 270,000 ticks x 47 components x 2 bytes = 24.2 MB, twelvefold under the 300 MB budget (NFR-3), and rewind to any tick is a direct index. The wire form would be 12.6 MB but would need a keyframe walk of up to 49 steps on every scrub. Rung 1 (AskUserQuestion).
scope: decides the in-memory history shape and the rewind mechanism; does NOT decide the saved replay file format, which `viewer-reports-recovery` owns.

**Q7 JavaScript tests.** No `package.json`; shared logic files carry the `.mjs` extension and run under `node --test`. The same files load unchanged in the browser, because the browser resolves a module by the served MIME type, not the extension, and the Rust server from Q1 controls its own MIME table. The repository stays a pure Cargo workspace. Rung 1 (AskUserQuestion).
scope: decides the JavaScript module and test shape and keeps npm out of the repository; does NOT decide the test folder layout or which modules are extracted.

**Q8 Memory gauge.** Serve the cross-origin-isolation headers and ask the browser. The page reports `performance.measureUserAgentSpecificMemory()` as its gauge, which requires `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` on every response, headers the Q1 Rust server sends and a `file://` page never could. Rung 1 (AskUserQuestion).
scope: decides what the gauge reports; does NOT by itself decide how AC-7 is evidenced, which Round 3 Q9 settles, and does NOT authorise dropping the exact byte count.

## 2026-09-22T14:41:32Z · plan (viewer-pitch) · Round 3 (evidence, scope, and the visual contract)

**Q9 Memory evidence (follow-up to Q8).** Both numbers, browser is the gauge. The page shows the browser's whole-page figure as the gauge and keeps an exact history-byte count beside it. The browser figure is evidenced by a driven full match; the byte count is evidenced headlessly under `node --test`. Asked because the Q8 answer foreclosed the headless path the slice's AC-7 names, and the requirement, a testable 300 MB assertion, is a second decision the product owner had not made. Rung 1 (AskUserQuestion).
scope: decides how AC-7 is evidenced and keeps both numbers; does NOT change the 300 MB threshold, and does NOT re-open Q8.

**Q10 Page shell.** Full frame, empty regions. This slice lays down the whole 1280 by 800 grid from `steer.md`, that is a 56-pixel header and columns 336 / 616 / 296 with an 8-pixel gutter, with the header and both side columns present and empty, so the pitch lands at its final 616 by 411 size and the frame-rate and clock evidence is measured at the shipping layout. Rung 1 (AskUserQuestion).
scope: decides the layout scope of this slice; does NOT move any panel content into this slice, because the header, feed, lineups, statistics, and tactics panel stay with `viewer-match-day` and `viewer-lineup-tactics`.

**Q11 Kit colour rule.** Pull extreme colours back into range. Each kit hex is converted to OKLCH and its lightness clamped into a safe band before the 3:1 turf test decides between the kit secondary and pitch-line white. Research fact that forced the question: `content/teams/default-a.json` sets `club.kit.secondary` to `#000000`, and DESIGN.md bans pure black; pure black would pass the 3:1 test, so the rule as written would put a banned value on the pitch. Rung 1 (AskUserQuestion).
scope: decides how any kit colour becomes a safe on-pitch value, for shipped, generated, and modder teams alike; does NOT change the shipped team files, and does NOT decide the club-crest treatment, which reuses the same function later.

**Q12 Measurement scope.** Options 2 and 3 together, not the recommended option 1. Author a browser benchmark artifact with browser targets and thresholds, AND extend the instrumentation plan with page-side signals. The product owner accepted the stated cost of option 3: the observability pipeline has no browser transport, so the page-side signals have nowhere to go until one is built. Rung 1 (AskUserQuestion, free-text answer "2 and 3").
scope: decides that this slice re-authors both augmentation artifacts with browser content; does NOT build a browser transport for the signals, and does NOT retire the engine benchmark targets, which stay as the cross-slice regression check.

**Pre-filled (not asked; source recorded):**
- Image gate: `pass`, source `steer.md`. The standing steering file names the Design canvas "Match Viewer Design" as the north-star mock, says to treat the direction as confirmed, and forbids generating a competing mock. No `imagery` run was made.
- Brief-confirm gate (`shape=pass`): satisfied by a user-confirmed PRODUCT.md, because design setup Round 1 Q5 confirmed PRODUCT.md and DESIGN.md as the design context.
- Product name: `Touchline` (PRODUCT.md), which removes the `[TODO]` marker that design setup Q4 left blocking this stage.
- Token prefix `--tl-`, superseding the `--mv-` prefix in `02b-design.md` and `02b-design.yaml`. `steer.md` fixes this directly.
- Typefaces: Barlow Condensed 700 and IBM Plex Sans 400/500/600, shipped as local `woff2` from the repository, never from a font service (`steer.md`). Both carry the SIL Open Font License, and IBM Plex Sans carries real OpenType tabular figures, so `font-variant-numeric: tabular-nums` works as intended. The two faces land in this slice because the canvas draws shirt numbers and the frame-time counter needs tabular figures.
- Binary receive shape: `WebSocket.binaryType = 'arraybuffer'` with a `DataView` over each frame. The `'blob'` default would force an asynchronous step per frame at 400 frames per second.
- Reduced motion: `@media (prefers-reduced-motion: reduce)` disables the control-strip and notice transitions. It never disables the simulation redraw, which is content, not decoration.
- Canvas accessibility: `role="img"` with a label, plus a visually hidden `aria-live="polite"` region mirroring clock, speed, and lag state in text.

**Consult record.** Triggers `appetite-medium-or-larger` and `unknowns-present` hold. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`). Not fired.

## 2026-09-22T19:29:33Z · plan (match-rules) · Round 1 (how a match flows)

**Q1 Change queue.** Stoppage hook only. This slice announces every stoppage with its kind through one engine hook (`TickSink::on_stoppage`); nothing applies a queued change yet, and the next slice drains the queue through that hook. Research fact that forced the question: `crates/protocol/src/command.rs:1-3` says this slice applies queued changes, while `03-slice-match-rules.md` gives substitution windows to `tactics-and-ai`. Rung 1 (AskUserQuestion).
scope: decides that this slice applies no queued change and corrects the stale code comment; does NOT decide how or when the next slice applies tactics changes or substitutions.

**Q2 Match length.** Announce a maximum. The rule pack caps added time per half; the file header and the opening message announce regulation ticks plus both caps as the maximum; the file trailer and the final statistics carry the real count. The page reserves its rewind memory once. Research fact: `web/main.mjs:66` sizes the history from `hello.ticks_expected`, and `crates/engine/tests/full_match.rs:22` pins 270,000. Rung 1.
scope: decides that `ticks_expected` and `expected_ticks` mean "at most" from this slice on; does NOT decide the cap value (the plan sets 900 seconds per half as the shipped default) and does NOT change the 300 MB memory budget.

**Q3 Dead ball.** Players walk into place. The ball sits at the restart spot; players steer to restart positions over a delay per restart kind from the tuning file; then the restart is taken. Rung 1.
scope: decides that players move by steering during a dead ball and that the dead-ball delay is tuning data; does NOT decide the delay values, which the plan sets as tuning defaults, and does NOT apply to the opening kick-off and the second-half kick-off, which place players at once as the break allows.

**Q4 Advantage.** Advantage when possession kept, not the recommended "no advantage". If the fouled team keeps the ball after the foul, play continues, a `foul` event records the advantage, and any card is shown at the next stoppage. Rung 1.
scope: decides that the engine plays advantage outside the penalty area; does NOT remove the free kick when the fouled team loses the ball, which the slice criterion requires, and does NOT decide advantage inside the penalty area (the plan awards the penalty there).

## 2026-09-22T19:29:33Z · plan (match-rules) · Round 2 (fouls, cards, and added time)

**Q5 Foul model.** Every tackle rolls three ways: a clean win, a foul, or a miss, from one draw. Foul chance rises with the tackler's aggression and falls with tackling skill; the tuning file holds the base rate, set for about 10 fouls per team per match. Research fact: the only tackle today is the possession roll at `crates/engine/src/sim.rs:362-365`. Rung 1.
scope: decides where fouls come from and which attributes drive them; does NOT decide the calibrated rate, which the `calibration` slice owns.

**Q6 Discipline.** Require aggression. `aggression` joins the required attributes; a schema or team file without it is refused at load naming the file and field. Research fact: `aggression` exists in the shipped schema (`content/README.md:29`) and nothing reads it. Rung 1.
scope: decides that aggression is required and drives foul and card chances; does NOT add a new attribute or change the shipped content files.

**Q7 Added time.** Free-text answer: "second per stoppage kind within expected limits and with variance". Read as: the rule pack gives seconds per stoppage kind; the engine sums them per half, adds a bounded variance drawn from the engine's seeded generator, and clamps the result between the rule pack's minimum and cap. The engine plays the exact seconds and reports the rounded-up minute. Rung 1 (free text mapped to option 1 plus variance).
scope: decides the formula shape and that variance comes from the seeded generator, so a seed reproduces the added time; does NOT decide the per-kind seconds, the variance bound, or the minimum, which the plan sets as rule-pack defaults near the real 3 to 4 and 5 to 6 minutes.

**Q8 Card events.** One event type `card` plus a field `card.kind` (`yellow`, `second-yellow`, `red`) and the player's id. Research fact: the observability contract reserves `card` and no colour key. Rung 1.
scope: decides the card event shape; does NOT amend the observability contract, where `card.kind` rides as an additive extra until the audit settles it.

## 2026-09-22T19:29:33Z · plan (match-rules) · Round 3 (snapshot, tests, and the page)

**Q9 Resume.** Exact continuation. A resumed match produces tick for tick the same match as one that never stopped, on the same machine and build; the test compares both tick files byte for byte from the stoppage onward. Research fact: `rand_chacha` 0.10 exposes `get_seed`, `get_stream`, `get_word_pos`, and `set_word_pos` (read in the installed source, `rand_chacha-0.10.0/src/chacha.rs:134-198`). Rung 1.
scope: decides the resume guarantee; does NOT promise cross-machine or cross-build resume (NFR-4), and a snapshot from another build is refused with a named reason.

**Q10 Snapshot file.** Binary like the tick file, not the recommended JSON. A fixed binary layout with magic bytes, a version, and a trailer; the plan adds a SHA-256 digest in the trailer, following the `.smfx` fixture's pattern, so corruption has a named reason. Rung 1.
scope: decides the snapshot encoding; does NOT decide the file location or how many snapshots are kept (the plan keeps the latest per match, replaced atomically).

**Q11 Test seam.** Scenario builder behind a Cargo feature that only tests enable; it places players and the ball, sets the carrier, and scripts the next random draws. Release builds never contain it. Rung 1.
scope: decides how criterion tests build a scene; does NOT make the builder a public or modder-facing API.

**Q12 Viewer skip.** Filter in this slice. The page's stoppage index takes restart frames plus only the event types that stop play. Research fact: `web/main.mjs:122-126` adds every event message, including `tactics-change`, to the index. Rung 1.
scope: decides that this slice changes one page module and its test; does NOT reopen the viewer-pitch Q4 decision ("both, merged"), which the filter keeps.

## 2026-09-22T19:29:33Z · plan (match-rules) · Round 4 (the benchmark gate)

**Q13 Benchmark gate.** Judge per tick, report both. Verify compares processor time per tick against the 10 percent tripwire and reports per-match time beside it with the tick counts. Asked because Q2 and Q7 add about 9 percent more ticks per match, so the per-match gate could fire from match length alone. Rung 1.
scope: decides how the slice's benchmark criterion is judged; does NOT change the 10 percent processor and 25 percent memory tripwires, and does NOT change the 2-second per-match budget (NFR-1), which stays per match.

**Pre-filled (not asked; source recorded):**
- Offside is judged only when a team-mate plays the ball (`03-slice-match-rules.md` Risks: "check only on forward passes"); involvement is the first touch by a player in the cached offside set. No offside from a goal kick, a throw-in, or a corner (IFAB Law 11).
- Restart spots follow IFAB Laws 1 and 13 to 17 on the 105 m by 68 m pitch: penalty mark 11 m, goal area 5.5 m, penalty area 16.5 m, corner arc 1 m, 9.15 m distance; the ball is out when its whole body crosses the line (radius 0.11 m).
- A team below seven players ends the match as abandoned (IFAB Law 3); the rule pack holds `min_players: 7`.
- A sent-off player's record slot parks at a fixed point beside the pitch, because the 192-byte tick record and the wire frame hold 22 positions and no status bit; the validator exempts that point.
- A shortened match (`--minutes` below the rule pack's regulation length) plays no added time, so short test matches keep a fixed tick count.
- `PROTOCOL_VERSION` rises to 2 and the tick-file schema to 4, because `ticks_expected` and `expected_ticks` change meaning from "exactly" to "at most" (the rule at `crates/protocol/src/lib.rs:21-28`). The page reads the version from the engine, so no page logic changes for the bump.
- Seven code comments name workflow slices (`sim.rs:273`, `data/rules.rs:2`, `command.rs:1-2`, `command.rs:55`, `event.rs:3`, `event.rs:12`, `message.rs:90`); the plan rewrites them in product language per the output boundary.

**Consult record.** Triggers `appetite-medium-or-larger` and `touches-migration` (snapshot and tick-file schema) hold. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`). Not fired.

## 2026-09-22T22:14:45Z · plan (viewer-lineup-tactics) · awaiting input (autonomous run stopped)

**OQ-1 (awaiting-input, intent-bearing: public contract and session behaviour).** How do the squad and the chosen lineup cross between engine and page before kick-off? Options: (1) squads in the opening message, a new `set-lineup` command, engine holds until lineup or explicit `start`; (2) separate squad message and `set-lineup` with a timeout to the default lineup; (3) lineup chosen over the page server before the match process starts. Research: `crates/engine-cli/src/serve.rs:17-20`, `crates/stream/src/control.rs:32`, `crates/engine/src/team.rs:88-92`.
scope: would decide the pre-match protocol contract and whether a session holds before kick-off; not decided by the autonomous run.

**OQ-2 (awaiting-input, intent-bearing: public contract shared with tactics-and-ai).** Where does the tactics panel read its schema (opening message, page-server JSON, or page code), and what does a queued change's `detail` hold?
scope: would decide the tactics schema delivery and change payload; not decided by the autonomous run.

## 2026-09-22T22:14:50Z · plan (viewer-match-day) · awaiting input (autonomous run stopped)

**Q-1 Match-day data source — AWAITING INPUT (intent-bearing).** The stream carries no player roster, no fatigue or condition value, and no live statistics: `stats` is sent once at full time with `possession.changes`, `ball.max_speed`, `ball.idle_ticks` (`crates/protocol/src/message.rs:51-66`, `docs/reference/protocol.md:116`); the hello names clubs only (`message.rs:11-23`); the engine has no fatigue model (`crates/engine/src/data/tuning.rs:100`). Options: (A) scope the wire additions and engine counters into this slice, with expected goals and fatigue shown as not modelled until later slices; (B, recommended) re-sequence this slice after the tactics, commentary, and calibration slices, the order `04-plan.md` already records, and add them to its dependencies; (C) build the panels against a proposed contract exercised by a synthetic fixture, with engine producers later.
scope: decides the wire contract and sequencing for the lineup, fatigue, and statistics panels; does NOT decide panel layout, the goal moment, the feed, or the empty state, which the plan settles as implementation detail.

## 2026-09-22T22:23:31Z · plan (extra-time-penalties) · awaiting input (autonomous run stopped)

**Q-1 Shoot-out simulation — AWAITING INPUT (intent-bearing: core-loop mechanism and the match-length contract).** How is the penalty shoot-out simulated? Research: the attribute schema already has `composure`, `finishing`, `reflexes`, `one_on_ones` (`content/attributes.json:16-34`); the penalty restart and shot mechanics exist (`crates/engine/src/rules/restart.rs:212`, `:259`); `ticks_expected` means "at most" (`crates/protocol/src/lib.rs:21-28`), and sudden death has no upper bound in law. Options: (A) seeded draws from attributes after the last tick, with events only and the pitch standing still, bounded at 480,000 ticks; (B, recommended) kicks played on the pitch through the penalty restart, an announced shoot-out allowance, and the page growing history past it; (C) kicks on the pitch, knockout length announced as unknown, `PROTOCOL_VERSION` 3.
scope: decides how shoot-out kicks are simulated and what the announced match length means for a knockout match; does NOT decide extra-time periods, the rule pack fields, kicker order, or event fields, which the plan settles as implementation detail.

## 2026-09-22T22:24:14Z · plan (viewer-reports-recovery) · autonomous run, no question asked

No product-owner question was asked. Every implementation question is settled as `class: implementation-detail` and recorded in `04-plan-viewer-reports-recovery.md` § Assumptions A-1 to A-14. Summary:
- A-1 The launcher `engine-cli launch` serves the page and supervises the engine worker. A page cannot start a process, and the engine process serves the page, so Q17 ("the viewer restarts the engine") needs a process that survives the engine.
- A-2 No protocol change: a reconnect repeats the same hello, and the first keyframe marks the resume tick.
- A-3 The replay file is the existing `.smfx` layout, written by the page from the wire bytes it received.
- A-4 `serve` keeps its end-on-drop behaviour unless it is given `--reconnect-wait`.
- A-5 The snapshot is still captured at every stoppage, but the serving path writes it only after the socket has written past it. The file name, layout, and location are unchanged.
- A-6 to A-14: the launcher passes the match stamp; the engine path comes from `--engine`, then `SM_ENGINE_PATH`, then the launcher's own executable; restart and abandon are same-origin POST routes; the half-time report pauses the page's own playback; saving is a blob download, verified through a read-only hook; a one-minute golden file is tracked; the shared augmentation artifacts are not rewritten; loading and first-run copy; without the launcher, a crash offers abandon only.
scope: decides how this slice's recovery, reports, and replay files are built inside the committed behaviour; does NOT change the protocol, the snapshot file, or any charter commitment.

## 2026-09-22T22:28:27Z · plan (distribution) · awaiting input (autonomous run stopped)

**OQ-1 Operating systems beyond Windows (U-3) — AWAITING INPUT (intent-bearing: user-visible scope; the slice gives this decision to the product owner at plan).** Options: (A) Windows only, macOS and Linux stay parked as a later slice; (B, recommended) Windows plus a Linux x86_64 archive built and smoke-tested natively in the WSL Ubuntu 24.04 install on the reference laptop (cargo 1.92.0 and gcc present), macOS pre-registered as a deferral cleared when an operator provides a Mac or a macOS CI runner; (C) Windows, Linux, and macOS, with macOS verification deferred from the start (no Apple SDK or linker; clang and zig absent).
scope: would decide which platform builds this slice ships; not decided by the autonomous run.

**OQ-2 How an installed user starts a match — AWAITING INPUT (intent-bearing: user-visible behaviour; a browser page cannot start a native program, so "launch from the page" is not buildable as written).** Options: (A, recommended) a Start-menu shortcut runs `engine-cli launch --open`, the launcher the reports-and-recovery plan adds, which starts the engine and opens the page in the default browser, keeping the viewer-pitch Q1 answer; (B) a registered `soccermanager:` link scheme the page launches; (C) an engine that starts at sign-in on a fixed port, which conflicts with stream-protocol Q9.
scope: would decide the installed launch path; not decided by the autonomous run.

**Settled as implementation detail (recorded in the plan's Assumptions):** NSIS 3.12 (installed) as the installer compiler; per-user install under `%LOCALAPPDATA%\Programs\SoccerManager`; static C runtime (the release binary imports VCRUNTIME140.dll today); Windows Sandbox as the clean Windows 11 machine with a Hyper-V fallback and a pre-registered deferral; page-folder discovery beside the binary; the planned launcher extended with optional seed and page folder and an --open flag instead of a new command; one version source for the binary, the hello message, and the installer name.

**Consult record.** Triggers `appetite-medium-or-larger` and `unknowns-present` hold. The product owner excluded `consult` at intake; not fired.

## 2026-09-22T22:29:40Z · plan (probe-engine-core) · awaiting input (autonomous run stopped)

**Q-1 Failure record — AWAITING INPUT (intent-bearing: the headless data contract of charter C4 and command-line output).** A failed `simulate` or `bench` run prints prose on stderr and no record. This run reproduced it: `simulate --minutes 0`, a directory as `--ticks-out`, and `bench --matches 0` each exit 1 with 0 bytes on stdout (`crates/engine-cli/src/main.rs:39-45`). The contract defines `outcome` values `success`, `error`, and `abandoned` and the keys `error.type`, `error.code`, and `error.retriable` (`.ai/observability.md` Block A). No contract gives the record shape for a run that fails before kick-off. Options: (A, recommended) one record on stdout with `outcome: error`, the three `error.*` keys, the success `record.kind` of the command, no statistic key, nothing persisted, exit 1 with the prose line kept; (B) narrow the contract so a failed run stays prose-only; (C) option A plus a failure record saved under `SM_DATA_DIR`.
scope: decides whether and how a failed run reaches the observability pipeline; does NOT decide the help-text fixes or the regression tests for findings 1, 3, 4, and 5, which the plan settles as implementation detail (`04-plan-probe-engine-core.md` § Assumptions A-1 to A-9).

## 2026-09-22T22:32:34Z · plan · tactics-and-ai (autonomous run; no product-owner answers)

No question was put to the product owner. The discovery round was resolved autonomously. Each of the 23 settled choices is recorded with its reason and a `class: implementation-detail` stamp in `04-plan-tactics-and-ai.md` § Assumptions (entries 1, 2, and 4 to 24).
scope: records where the autonomous choices live; does NOT stand in for a product-owner answer.

**Carried, not answered:** the wire format of a queued change's `detail` (tactics and substitution), and with it the socket bridge into the engine's queue. It is intent-bearing and already open as OQ-2 in `04-plan-viewer-lineup-tactics.md`. This plan does not answer it, and none of its criteria depend on it.
scope: points to the owning open question; does NOT decide it.

## 2026-09-22T22:34:05Z · plan (scripting-runtime) · autonomous run, no question asked

No product-owner question was asked. Every implementation question is settled as `class: implementation-detail` and recorded in `04-plan-scripting-runtime.md` § Assumptions A-1 to A-16. Summary:
- A-1 The runtime is Rhai 1.26 (pure Rust, MIT OR Apache-2.0). C-backed runtimes cannot build without a C compiler.
- A-2 The engine owns a runtime-free plugin interface, and a separate crate implements it.
- A-3 and A-4 Decisions still run every tick. The script returns option-score offsets, cached and refreshed on a carrier change, after a stoppage, or every 25 ticks.
- A-5 to A-8 The time budget is an operation count with a 2 ms backstop. Imports and unexposed functions are denied and recorded. Scripts are stateless and get no random function. Three consecutive failures disable a hook for the match.
- A-9 to A-13 The pack is a folder with `pack.json` and one `.rhai` file, selected by `--script-pack`. The pack hash is folded into the content hash, so the snapshot does not change. An additive `script` event type keeps protocol version 2. The rule hook covers cards, and the commentary hook covers lines.
- A-14 to A-16 The benchmark baseline is captured at implement time. No consult is run (excluded by the product owner). There is no page surface.
scope: decides how the slice's delegated scope (runtime, sandbox, hooks, pack format) is built; does NOT change any charter commitment, the decision interval, or the snapshot file.

## 2026-09-23T07:00:32Z · verify · calibration · product-owner answer

**Calibration AC-d: do changes that expire at full time count toward `darkpath.change_never_applied`? — ANSWERED: option 1, do not count them.**
A change that is still queued at full time because no stoppage that admits it came after it was queued is "expired at full time". It is not "never applied". The engine counts it in a separate counter, `change.expired_at_full_time` (int, unit engine). That counter is reported in `match-stats` and in the calibration run report, and it has no zero rule. `darkpath.change_never_applied` keeps its zero rule. It counts only a change that an admitting stoppage should have applied and did not. The observability contract (`.ai/observability.md`: the key list, the `dark-paths` block, and the `match-stats` record) is amended to match. The computer manager's queuing behaviour from the tactics slice does not change.
scope: decides the counter definition for DARKPATH-2 (105 over 2000 matches); does NOT change the computer manager's behaviour or the other two dark-path counters.

## 2026-09-23T07:03:28Z · plan (viewer-match-day) · awaiting input (autonomous re-run, Q-1 narrowed)

**Q-1 Match-day wire shape — STILL AWAITING INPUT (intent-bearing: versioned protocol and recorded match files).** The earlier "re-sequence" option is now satisfied: the engine keeps all nine statistics live (`crates/engine/src/sim.rs:289-328`), each player's energy (`crates/engine/src/player.rs:137`), injury and substitution events, and a commentary line on every play event. The wire still carries no roster, no running statistics, and no energy (`crates/protocol/src/message.rs:28-66`; `stats` is sent once at full time from `crates/engine-cli/src/stream_run.rs:103`). Options: (A, recommended) a `roster` on each hello team, the nine panel fields added to `stats` with it also sent once per simulated second, and a `condition` message with the 22 energy values, version rule per `crates/protocol/src/lib.rs:21-28`; (B) three new messages (`roster`, `live-stats`, `condition`) at protocol version 3; (C) roster read by the page from the team files, only statistics and energy on the wire. The roster part is the same decision as `viewer-lineup-tactics` OQ-1.
scope: decides the wire shape for the lineup, fatigue, and statistics panels; does NOT decide panel layout, the feed, the goal moment, or the empty state, which the plan settles as implementation detail (`04-plan-viewer-match-day.md` § Assumptions A-1 to A-13).

## 2026-09-23T07:30:49Z · plan (viewer-match-day) · awaiting input (autonomous auto-review, Q-1 unchanged)

**Q-1 Match-day wire shape — STILL AWAITING INPUT (intent-bearing: versioned protocol and recorded match files).** No answer has been recorded since 07:03:28Z. The question and its three options are unchanged (see the 07:03:28Z entry). The auto-review added one planning risk: a later event-contract redesign (`docs/design/realism/02-event-contract-redesign.md`) names this slice as a consumer to migrate; it does not answer Q-1.
scope: records that the question remains open; decides nothing about the wire shape.

## 2026-09-23T08:37:28Z · plan · product-owner answers to the open plan questions

Each answer picks the option the plan marked as recommended, as written in that plan.

- **viewer-match-day Q-1: ANSWERED, option A** (additive fields and one periodic message). Each hello team gains a `roster`. The `stats` message gains the nine panel fields per team and is also sent once per simulated second. A new `condition` message carries the 22 energy values. The version follows the rule at `crates/protocol/src/lib.rs`. Because viewer-lineup-tactics OQ-1 option 1 raises the protocol to version 3, these additions ship in version 3 with it.
- **viewer-lineup-tactics OQ-1: ANSWERED, option 1** (hello carries the squads, a new `set-lineup` command, and the engine holds before kick-off). A client that sends no lineup plays the computer manager's pre-match setup after an explicit `start`. `PROTOCOL_VERSION` rises to 3. This also answers the roster part of viewer-match-day Q-1.
- **viewer-lineup-tactics OQ-2: ANSWERED, option 1** (the tactics schema rides in the opening message). A queued change's `detail` is `{ "patch": <tactics patch> }` for tactics or `{ "off": <squad index>, "on": <squad index> }` for a substitution, and the socket bridge routes it into `Simulation::queue_change`.
- **extra-time-penalties Q-1: ANSWERED, option B** (on the pitch, with a bounded announcement). Each kick is played through the penalty restart and shot mechanics, tick by tick. The announced maximum includes a 10-round shoot-out allowance, and the page grows its history if sudden death runs past it.
- **distribution OQ-1 (U-3): ANSWERED, option B.** The slice ships Windows and a Linux x86_64 `.tar.gz`, built and smoke-tested in WSL Ubuntu-24.04. macOS is a pre-registered deferral that clears when an operator provides a Mac or a macOS CI runner.
- **distribution OQ-2: ANSWERED, option A.** A Start-menu shortcut runs `engine-cli launch --open`, which starts the engine and opens the page in the default browser.
- **probe-engine-core Q-1: ANSWERED, option A.** A failed `simulate` or `bench` run emits one stdout record with `outcome: "error"`, `error.type`, `error.code`, `error.retriable`, and the command's success `record.kind`, with no statistic key. Nothing is persisted. The run exits 1 and keeps the prose line on stderr.

scope: settles every intent-bearing open question in the five plans awaiting input; does NOT decide any implementation detail those plans leave open.

## 2026-09-23T08:44:00Z · plan (probe-engine-core) · awaiting input (autonomous auto-review, Q-1 answered, new Q-2)

**Q-1 recorded as answered (option A)** from the 2026-09-23T08:37:28Z entry above. No new answer was taken in this run.
scope: records the answer in the plan; does NOT extend it to `calibrate` or the other subcommands.

**Q-2 Record schemas — AWAITING INPUT (intent-bearing: a published record schema and the outcome word).** The Q-1 record (`outcome: error`, no statistic key) fails `schemas/observability/match-stats.schema.json` (outcome enum `success`/`failure`; 32 required keys) and `run-report.schema.json` (same enum; 16 required keys). These schemas shipped with calibration after Q-1 was asked. Options: (A, recommended) amend both schemas additively so an `error` record needs the three `error.*` keys and no statistic key, calibration keeps `failure`; (B) the failure record uses `outcome: failure`, schemas change only their required keys; (C) option A plus calibration switches `failure` to `error`, unifying on the contract word.
scope: decides the schema and outcome-word change for a failed `simulate` or `bench` run; does NOT decide the help-text fixes or the regression tests, which the plan settles as implementation detail (`04-plan-probe-engine-core.md` § Assumptions A-1 to A-12).

## 2026-09-23T08:42:40Z · plan (viewer-lineup-tactics) · complete (autonomous re-run on the recorded answers)

stage: plan. The product-owner answers OQ-1 = 1 and OQ-2 = 1 (08:37:28Z entry) clear both blockers. The discovery interview did not run on this autonomous run. Its implementation questions were settled as implementation details and are recorded as A8 to A14 in `04-plan-viewer-lineup-tactics.md`:
- Only `serve` holds before kick-off. `record`, `bench`, `resume`, and the stream tests keep producing at once.
- Field names inside the ratified options: `squad`, `setup`, `tactics`, `set-lineup { lineup, bench, patch? }`, and a detail patch of schema indices keyed by squad index.
- The opening message also carries `substitutions { limit, windows }` for the remaining-count display. This is additive inside the ratified version 3.
- Role fit comes from the engine's `ai::role_fit`. Fitness before kick-off is `natural_fitness`.
- The socket queue id is carried to the change event by a driver-side map.
- A malformed detail is refused at queue time. The engine refuses an illegal change at the stoppage.
- Pre-match tactics ride in `set-lineup`. A `queue-change` before the first `start` is refused with a reason.

## 2026-09-23T08:46:29Z · plan (viewer-match-day) · autonomous re-plan on the returned answer, no question asked

Q-1 was answered with option A (2026-09-23T08:37:28Z entry above); the plan is now complete. No new question was put to the product owner. The details inside the answered wire shape are settled as `class: implementation-detail` in `04-plan-viewer-match-day.md` § Assumptions A-16 to A-23:
- A-16 This slice raises `PROTOCOL_VERSION` to 3; the lineup editor adds its fields under the same number.
- A-17 A roster entry holds `player.id`, `player.name`, `player.shirt`, `player.position`, and `player.squad_index`; the 11 starters in wire-slot order, then the named bench.
- A-18 Stats and condition are sent every 50 ticks (one simulated second) after the tick's events, plus a final stats message at full time; values come from `MatchFigures::new` and `Summary`, so they equal the `match-stats` record.
- A-19 Energy is rounded to three decimals; no condition message at full time.
- A-20 One `hello_teams(&Simulation)` helper for serve, record, and bench; the simulation is created before the hello.
- A-21 No engine crate change: energy is read through `Simulation::players()`.
scope: decides the details inside the answered wire shape; does NOT change the shape the product owner chose or any charter commitment.

## 2026-09-23T18:39:54Z · plan (probe-engine-core) · awaiting input (autonomous auto-review, Q-2 still open)

**Q-2 Record schemas — STILL AWAITING INPUT (intent-bearing: a published record schema and the outcome word).** No answer has been recorded since 08:44:00Z. The question and its three options are unchanged (see the 08:44:00Z entry). This run re-read both schemas at HEAD `89fe959`: `outcome` is still `enum: [success, failure]`, and 32 (`match-stats`) and 16 (`run-report`) keys are still required. No question was put to the product owner.
scope: records that the question remains open and re-grounds the plan on the current code (a tenth help surface `launch`, 47 over-width `-h` lines, the command reference in `docs/reference/cli.md`); decides nothing about the record contract. The new implementation choices are A-13 to A-15 in `04-plan-probe-engine-core.md` § Assumptions.

## 2026-09-23T18:44:06Z · plan · probe-engine-core · product-owner answer

**Q-2 How the failure record fits the record schemas — ANSWERED: option C, unify on `error`.** Option A applies: `outcome` accepts `error`, the statistic and benchmark keys are required only when `outcome` is not `error`, and an `error` record requires `error.type`, `error.code`, and `error.retriable`. In addition, calibration writes `error` in place of `failure` (the worker, the run report, the report filters, and their tests). The schemas accept `success` and `error` only, which matches the observability contract.
Run folders written before this change are not a compatibility target, because the product has not shipped. The coordinator recorded this point when the answer was relayed, and the product owner can override it.
scope: settles the outcome word and the schema shape for failed runs; does NOT change any other record key.

## 2026-09-23T18:52:56Z · plan (probe-engine-core) · complete (autonomous re-run on the recorded answer, no question asked)

stage: plan. The product-owner answer Q-2 = C (18:44:06Z entry above) clears the last blocker; the plan is complete. No question was put to the product owner. The details inside the answered contract are settled as `class: implementation-detail` in `04-plan-probe-engine-core.md` § Assumptions A-8 to A-19:
- The envelope and correlation keys (`owner.id`, `match.id`, `run.id`, `machine.hash`, `content.hash`) stay required; only the statistic and benchmark keys become conditional.
- The failure record takes `owner.id` from the existing identity file (created as on any run); with no identity, no record is printed. `content.hash` is empty on a failed `simulate` or `bench` record, as on calibration's failed match.
- A calibration run report with a failed worker carries `error.type` `worker`, `error.code` `worker-failed`, `error.retriable` `false`.
- A hidden test seam `calibrate --inject-failure <match|worker>` lets verify drive both calibration error paths; `schema.version` stays 1; `abandoned` is not added.
scope: decides the details inside the answered record contract; does NOT change the shape the product owner chose or any other record key.

## 2026-09-23T22:05:38Z · stage: extend · extension round 1 · product-owner answers

Request: fix the ten-player problem and the red-card issues, add a band on the share of matches with ten or more goals, lower the second-yellow rate and add a red-card band, recalibrate shots on target, pass volume, corners and xG against new bands, investigate shots, passes, pass completion, corners and throw-ins, then run the browser suite and hand off.

- **Q-E1 Band values — ANSWERED: the sourced bands.** 10 or more goals at most 0.5% of matches; a sending-off in 8–22% of matches; yellow cards per team 1.2–2.6; shots on target 30–42% of shots; goals per xG 0.85–1.15; passes per team 350–550; pass accuracy 75–88%; corners per team 3.5–6.5; throw-ins per match 35–55; goal kicks per match 12–22; goalless draws 4–12%. Sources: Wyscout and StatsBomb figures recorded in `docs/design/realism/01-engine-realism.md` and its local sources. This answer adds to the confirmed realism-bands contract; it widens or removes no existing band.
- **Q-E2 Slicing — ANSWERED: split into slices, not one fix.** The cause is wider than ten against eleven (defending has no marking, so formations other than 4-4-2 fail at 11 against 11).
- **Q-E3 Slice list and order — ANSWERED: bands first, and a fifth slice if needed to get things within band.** Order: `realism-bands-v2`, `defending-and-discipline`, `keeper-and-shots`, `tempo-and-restarts`, `realism-tuning`.
- **Q-E4 How verify judges the interim slices — ANSWERED: a final tuning slice.** `realism-bands-v2`, `defending-and-discipline`, `keeper-and-shots` and `tempo-and-restarts` are judged on their own acceptance criteria; their band misses are recorded, not treated as failures. `realism-tuning` must pass every band.
- **Q-E5 The two ship-blocking deferrals — ANSWERED: leave both for later.** The legibility reading and the macOS build stay open; handoff may proceed; ship stays blocked until both are cleared.
scope: defines extension round 1 (five slices) and its bands; does NOT change any completed slice or any existing band value.

## 2026-09-23T22:28:02Z · stage: plan · slice: realism-bands-v2 · product-owner answers

Three rounds of plan questions (rung 1, structured question tool).

- **Q-P1 What the formations suite checks — ANSWERED (free text): "there are more possible formations than that in football, simulate 1000 matches per scenario and check goal bands, check all formations against each other".** The formations suite pairs every formation against every other formation, mirrors included, with 1,000 matches per pairing. Each pairing is checked against the goal bands: goals per match (2.4–3.2), the share of matches with 10 or more goals (at most 0.5%), and the share of goalless matches (4–12%).
  scope: decides the formations suite's pairings and checks; does NOT change any band value, and does NOT decide which new formations ship (Q-P5).
- **Q-P2 Matches per pairing — ANSWERED: 1,000 per pairing (Recommended).** `--matches` applies to each pairing.
  scope: decides the sample size per pairing; does NOT decide whether a default run includes the suite (Q-P6).
- **Q-P3 The ignored slow test — ANSWERED: every band, fails now (Recommended).** The slow test asserts every band and exit 0. It fails until `realism-tuning`.
  scope: decides the slow test's assertions; does NOT make band misses fail this slice (Q-E4 stands).
- **Q-P4 Paired-run verdict for bands whose low end is 0 — ANSWERED (free text): "what's best for our scenario, realism and player enjoyment".** The product owner delegated the choice. The plan takes the recommended option: for a band whose low end is 0, the distance counts only how far the value is above the high end. Other bands keep today's formula.
  scope: decides the paired-run distance for floor bands only; does NOT change the distance of any existing band.
- **Q-P5 New formations — ANSWERED: add 6 in this slice (Recommended).** 4-1-4-1, 4-4-1-1, 4-1-2-1-2 (diamond), 3-4-3, 5-3-2 and 5-4-1 join the four shipped formations, for 10 in total. They appear in the lineup editor and the tactics panel at once.
  scope: decides the shipped formation list; does NOT change the engine's defending (that is `defending-and-discipline`), and does NOT change the computer manager's default formation (4-4-2).
- **Q-P6 Run time — ANSWERED (free text): "always in --suite all we need to plan to run tests in parallel or other ideas to reduce the time".** `--suite all` includes the formations suite. The plan must cut run time without cutting matches.
  scope: decides that every full run includes the formations suite; does NOT authorize fewer matches per pairing (Q-P2, Q-P8).
- **Q-P7 An older bands file — ANSWERED: refuse, version 2 (Recommended).** `content/realism-bands.json` moves to schema version 2 and every band is required. An older file is refused with a message that names the file and the missing band.
  scope: decides bands-file compatibility; does NOT change any other content file's version.
- **Q-P8 Early stopping of a clearly failing pairing — ANSWERED: always 1,000.** Every pairing plays all 1,000 matches, even when it clearly fails.
  scope: forbids early stopping; does NOT forbid the other time savings (shared outlier decision in the worker, fewer file writes).
- **Q-P9 The baseline — ANSWERED: five seeds, every suite.** The implement record holds a baseline of every suite, formations included, on seeds 42, 1, 7, 99 and 2026 (about 6 hours on the reference machine).
  scope: decides the baseline's seeds and suites; does NOT make the baseline a pass or fail gate for this slice.

## 2026-09-24T00:28:46Z · stage: implement · slice: realism-bands-v2

- **Q-I1 Seeds for the formations suite — ANSWERED: yes, one seed.** Asked after seed 42 of the baseline: "We really don't need more than one seed for formation pairs testing right?" Agreed to this rule: every suite runs on five seeds except the formations suite, which runs on one seed (1,000 matches per pairing). A pairing within about two sampling errors of a band edge (for example 3.0 to 3.2 goals per match) is rerun on the other four seeds, that pairing only. For this slice's baseline, seed 42 is the formations seed; seeds 1, 7, 99 and 2026 run the equal and strength suites only. Reason: seed 42 missed by far more than sampling noise (pairings from 1.45 to 38.5 goals per match against 2.4 to 3.2), and a second seed only adds 1,000 more matches of the same pairing.
  scope: changes Q-P9 (the baseline of this slice) and the formations part of the realism-tuning criterion "every band passes on every seed" (03-slice-realism-tuning.md, acceptance criteria), which its plan re-states; does NOT change matches per pairing (Q-P2, Q-P8), does NOT remove the formations suite from --suite all (Q-P6), does NOT change the five seeds of the equal and strength suites, and does NOT widen any band.

## 2026-09-24T02:46:11Z · stage: plan · slice: defending-and-discipline · autonomous run, no question asked

No product owner was present, so the plan asked no discovery question. Each implementation choice is recorded as A1 to A19 in `04-plan-defending-and-discipline.md` § Assumptions, each stamped `class: implementation-detail`. None changes the slice's scope, a public contract, or an earlier answer.
scope: records that the plan settled only implementation details; does NOT answer any product question, does NOT change a band or a criterion limit, and does NOT change the card chances per foul (RIM-7: the card rate is fixed separately).

## 2026-09-24T03:50:37Z · stage: implement · slice: defending-and-discipline · AWAITING INPUT (autonomous run stopped)

- **Q-I2 Two criteria fail at every in-bounds tuning — AWAITING INPUT.** Implement built every planned mechanism (commit `f7fe35b`). Discipline passes (0.085 second yellows per match, 16.5% of matches with a sending-off, no same-tick pair over 200 matches), and the acting keeper passes (six scripted scenes). Two criteria fail at every setting of the five new tuning values within their bounds:
  - **No advantage from a red card** (120 seeds, cards off): the reduced side outscores the full side in every arm. Keeper 5.82 against 2.06, centre-back 2.39 against 0.71, striker 3.84 against 0.95. The full side's limit is 1.24, from a control of 0.78 for home and 2.23 for away.
  - **Every formation holds** (120 seeds): 4-4-1-1 scores 5.18 and 3-4-3 scores 4.17 against 4-4-2, above the 4.0 limit. The other eight pairings pass; at the baseline, seven of nine failed.
  - The measured cause is a lone central forward. With no forward team-mate to pass to, the ball carrier dribbles and shoots about twice as often (40.6 shots against 20.8) and is never offside. Contact near the ball also gives about four fouls for each won tackle. The carrier's decision weights and the tackle odds are outside the five values this slice may tune (plan step 14 and A19).
  - Options for the product owner. None is taken until you answer.
    1. Widen this slice to the carrier's decision with no forward team-mate, and to the odds of a won tackle against a foul; then re-plan and tune them.
    2. Move the red-card criterion and the two pairings to `realism-tuning`, and verify this slice on the other criteria and the improvement already measured.
    3. Revisit the slice definition, for example by splitting out a "lone forward" slice before `keeper-and-shots`.
  scope: records a stop under the plan's own rule; does NOT change any limit, band, card chance or goal tuning, and does NOT choose an option.

## 2026-09-24T06:19:45Z · stage: implement · slice: defending-and-discipline

- **Q-I2 Two criteria fail at every in-bounds tuning — ANSWERED: a new lone-forward slice.** Verify `defending-and-discipline` on what passes now: discipline, the acting keeper, and the formation pairings that hold, with the improvement already measured (7 of 9 pairings failed at the baseline; 2 of 10 fail now). Add a separate lone-forward slice before `keeper-and-shots` through an intake extension. That slice owns the lone central forward's decisions when he has no forward team-mate, the odds of a won tackle against a foul, the red-card criterion ("no advantage from a red card", all three arms) and the two failing pairings (4-4-1-1 at 5.18 and 3-4-3 at 4.17 against 4-4-2, limit 4.0).
  scope: moves the red-card criterion and the two failing pairings out of defending-and-discipline into the new slice, and orders that slice before keeper-and-shots; does NOT change any limit, band, card chance or goal tuning, does NOT widen defending-and-discipline, and does NOT move work into realism-tuning.

## 2026-09-24T06:26:59Z · stage: extend · extension round 2 · product-owner answers

- **Q-X1 Tuning-loop capabilities — ANSWERED: all four.** Targeted runs (named suites, pairings or bands on one seed); a stored baseline with a per-band diff and its sampling error; the red-card experiment as a calibrate suite; and a faster build profile, kept only if matches per second rise and every result is identical per seed.
  scope: defines `tuning-loop`; does NOT split runs across machines, does NOT add early stopping, and does NOT change any band or tuning value.
- **Q-X2 Lone-forward levers — ANSWERED: both.** The lone central forward's decisions when he has no forward team-mate (pass, dribble, shoot, hold-up, offside line), and the odds of a won tackle against a foul. Discipline is measured again, because fewer fouls move the card figures.
  scope: defines the levers of `lone-forward`; does NOT change the card chance per foul (RIM-7), the keeper model or any band.
- **Q-X3 Lone-forward criteria — ANSWERED: the moved criteria plus no regression.** "No advantage from a red card" (all three arms, limits unchanged) and 4-4-1-1 and 3-4-3 against 4-4-2 at most 4.0 goals per side (Q-I2); the eight pairings that pass now and the discipline criterion of `defending-and-discipline` still pass.
  scope: sets the acceptance criteria of `lone-forward`; does NOT change any limit.
- **Q-X4 Order — ANSWERED: tuning-loop, then lone-forward, then keeper-and-shots.** `defending-and-discipline` verify may run at any time before `lone-forward`.
  scope: orders the two new slices; does NOT reorder the slices after `keeper-and-shots`.
- **Q-X5 Loop speed — ANSWERED: under 2 minutes.** One targeted suite or pairing at 1,000 matches, or the full red-card experiment, finishes with its diff against the baseline in under 2 minutes on the 8-core reference machine.
  scope: sets the speed criterion of `tuning-loop`.
- **Q-X6 Baseline location — ANSWERED: a report given by path.** `calibrate --baseline <report.json>` compares to any earlier run. Nothing new is committed; a slice copies the baseline it used into its evidence folder.
  scope: sets how `tuning-loop` stores a baseline; does NOT commit run folders.
- **Q-X7 Confirm — ANSWERED: confirmed.** `tuning-loop` (M) and `lone-forward` (L) are added in extension round 2. Existing slices are not modified. `defending-and-discipline` is verified without the two criteria Q-I2 moved.

## 2026-09-24T07:09:11Z · stage: plan · slice: tuning-loop (autonomous run)

- **Plan discovery — ANSWERED AUTONOMOUSLY (class: implementation-detail).** No product owner was present. Each implementation question is answered in `04-plan-tuning-loop.md` § Assumptions A1-A21 (full-list fixture indices for a selected pairing, either-order pairing names, `--band` narrowing the suites, a red-card suite outside `--suite all` on engine seeds used directly, a public send-off before kick-off, independent-run sampling error, additive report keys at schema version 1, and a profile measurement that never commits a native CPU target).
  scope: decides how the agreed tuning-loop scope is built; does NOT change a band, a criterion limit or a tuning value.
- **Q-P1 Baseline identity — AWAITING INPUT (class: intent-bearing).** The criterion refuses a baseline on a different content hash, but `content.hash` covers `tuning.json` and every flag state, so the guard as written would refuse every tuning change the loop exists to measure. Options: 1 (recommended) guard on a fixtures hash (seed, match count, the attributes, rules and tactics files, the generator block and the default clubs) and print a changed content hash as the change under test; 2 guard on `content.hash` as written (the loop measures engine code changes only); 3 guard on `content.hash` with an `--allow-content-change` override.
  scope: decides the refusal key of "A wrong baseline is refused"; does NOT change any other step of the plan.

## 2026-09-24T07:20:00Z · stage: plan · slice: tuning-loop · product-owner answer

- **Q-P1 Baseline identity — ANSWERED: the fixtures hash (Recommended).** A baseline is refused only when its fixtures differ from the run: the seed, the match count, the attributes, rules and tactics files, the generator block and the default clubs. A changed content hash (tuning values or flag states) is not refused; the diff prints it as the change under test.
  scope: sets the refusal key of the tuning-loop criterion "A wrong baseline is refused", which the plan re-states as "a baseline with different fixtures is refused"; does NOT change any other criterion, band, limit or tuning value.

## 2026-09-24T08:26:23Z · stage: plan · slice: tuning-loop (autonomous re-run)

- **Q-P1 answer folded in (class: implementation-detail).** The plan re-run builds the product owner's 07:20:00Z answer without widening or narrowing it: a new additive `fixtures.hash` report key over the attributes, rules and tactics content, the generator block after flag states, and the two default team files; the guard refuses a different seed, match count or fixtures hash, and a baseline without the key; a different content hash is printed in the diff header as the change under test. Recorded in `04-plan-tuning-loop.md` § Assumptions A22-A27, which also adds the red-card suite to the jobs clamp (auto-review finding).
  scope: completes the tuning-loop plan; does NOT change any criterion, band, limit or tuning value.

## 2026-09-24T11:20:00Z · stage: plan · slice: lone-forward (autonomous run)

- **Plan discovery — ANSWERED AUTONOMOUSLY (class: implementation-detail).** No product owner was present. The implementation questions are answered in the plan draft's Assumptions A1-A20: a lone carrier is one with no active outfield team-mate ahead of him; three weighted terms (hold-up, lay-off behind him, a cost on a pressed dribble) and no scripted rule (RIM-12); no shot or clearance term; the tackle-odds lever is a new bounded `tackle_win_base` (today's 0.05 as its neutral value) plus the existing `foul_base`, with the card chances untouched (RIM-7); the lone forward off the ball holds the offside line from the same line the referee uses; the seams land at neutral values with a zero-change red-card diff before any tuning; the benchmark is re-baselined on a447ff1 at 1.5422 us per tick.
  scope: decides how the agreed lone-forward scope is built; does NOT change a criterion, a limit, a card chance, the keeper model or a band.
- **Q-L1 The design is not settled — AWAITING INPUT (class: intent-bearing).** The plan could not be written: the pre-write hook refuses every plan file for this workflow because `02c-craft.md` carries `image-gate: pass` but no `direction-confirmed-by`. The design lane now makes the design stage human-only, so an autonomous run may not record a person's confirmation or turn the gate off. `steer.md` (2026-09-22) already says to treat the viewer direction as confirmed; a person records that confirmation through `/wf design football-manager-match-engine`, or records that design is skipped with a reason. This slice changes no page; the gate is workflow-wide. The full plan draft is held outside the workflow folder until then.
  scope: unblocks every plan write for this workflow; does NOT change the lone-forward scope, criteria or levers.

## 2026-09-24T11:22:17Z · stage: design · product-owner answer

- **Design confirmation — ANSWERED: approve, interim.** The product owner's words: "Approve for now, this is an interim design only in place for working on the engine". The Touchline match-screen direction (the Design canvas "Match Viewer Design" and `02c-craft.md`) is confirmed in session with no drawing changed. A final design replaces it later.
  scope: settles the design gate for the remaining engine slices; does NOT make this design final, and does NOT change any surface, token or layout.

## 2026-09-24T11:53:45Z · stage: plan · slice: lone-forward (autonomous run)

- **Plan written (class: implementation-detail).** The product owner's design confirmation at 11:22:17Z cleared Q-L1, so the plan is written as `04-plan-lone-forward.md` with no blocker. The implementation questions are answered there in § Assumptions A1-A18, in the same direction as the 11:20:00Z draft: a lone carrier has no active outfield team-mate ahead; three weighted terms (lay-off, hold-up when pressed, a cost on a pressed dribble) and no scripted rule; no shot or clearance term; the tackle-odds lever is a new bounded `tackle_win_base` (neutral 0.05) plus the existing `foul_base`, with the card chances untouched; the team's lone forward holds the referee's own offside line off the ball; the five values land at neutral values with a zero-change red-card diff before tuning; the three slow criterion tests are reused unchanged.
  scope: decides how the agreed lone-forward scope is built; does NOT change a criterion, a limit, a card chance, the keeper model or a band.

## 2026-09-24T12:41:48Z · stage: implement · slice: lone-forward · AWAITING INPUT (autonomous run stopped)

- **Build — ANSWERED AUTONOMOUSLY (class: implementation-detail).** Both levers are built at commit `c3cc96f` with the five tuning values at neutral values. Play is unchanged: the red-card diff shows 0 on all 6 rows, and 489 workspace tests pass. The decisions are recorded in `05-implement-lone-forward.md` § Assumptions and § Deviations from Plan.
  scope: builds the agreed levers; does NOT ship a tuned setting, and does NOT change a criterion, limit, card chance, keeper value or band.
- **Q-LF1 The red-card criterion fails at every realistic in-bounds setting — AWAITING INPUT (class: intent-bearing).** The run tried 17 settings on the criterion's seeds and arms (`implement-evidence/lone-forward/tuning-log.txt`). The two pairings pass with `tackle_win_base` 0.5 (4-4-1-1 2.71, 3-4-3 2.92 over 200 matches). No setting with realistic play passes "no advantage from a red card". At the tackle bound, the reduced side scores 2.77, 1.82 and 2.17 against a full side's 1.68, 0.74 and 1.16 (keeper, centre-back, striker). The only passes come where shots fall below one per team per match, which RIM-12 rules out. The measured cause: in the experiment the away club (Eldstead City) scores about 2.8 times the home club at 11 against 11. With the clubs swapped, the control is 2.19–0.80 instead of 0.78–2.23. In the centre-back arm the reduced side has no lone forward. Options:
  1. Keep the criterion and allow a third lever, for example less pressing and running by a side with ten men.
  2. Measure the criterion on equal clubs, or on both home and away orders of the default clubs, with the limits unchanged.
  3. Narrow "lone" to the team's structural lone forward, ship the tackle lever for the pairings, and judge each arm against the reduced side's own control.
  4. Ship the formations fix alone (tackle lever) and move the red-card criterion to a later slice.
  scope: decides how the red-card criterion is met or moved; does NOT change the pairing limits, the discipline criterion, the card chances or the keeper model.

## 2026-09-24T13:19:59Z · stage: implement · slice: lone-forward · product-owner answer

- **Q-LF1 The red-card criterion — ANSWERED: option 2, equal clubs or both orders (Recommended).** Measure "No advantage from a red card" without the club-strength imbalance: either on two equal clubs, or on both home and away orders of the default clubs, pooled. Every limit is unchanged: in each arm the reduced side's goals per match do not exceed the full side's, and the full side's goals per match are at most 1.6 times the no-send-off control on the same seeds and clubs. The reason: in the current test the sent-off player is always on the away club, which scores about 2.8 times the home club at 11 against 11, so the test measured club strength and not the red card.
  scope: changes how the red-card criterion's fixtures are set up (clubs and home-away order) in the slow test and the calibrate red-card suite; does NOT change any limit, the pairing criteria, the discipline criterion, the card chances or the keeper model, and does NOT add a lever. If the centre-back arm still fails on balanced fixtures, implement stops and reports again.

## 2026-09-24T13:49:15Z · stage: implement · slice: lone-forward · AWAITING INPUT (autonomous run stopped)

- **Balanced fixtures — ANSWERED AUTONOMOUSLY (class: implementation-detail; within the Q-LF1 answer).** Of the two fixture forms the answer allowed, the run chose both home and away orders of the default clubs, pooled: each seed from 1 to 120 plays twice, the second time with the clubs swapped. This is in the slow test and in the calibrate red-card suite (`--seed 1 --matches 240`), at commit `7bd5fa5`. At the shipped neutral values both give the same figures: control 1.4833–1.5167.
  scope: builds the Q-LF1 answer; does NOT change a limit, an arm, a lever or a band.
- **Q-LF2 The red-card criterion still fails on balanced fixtures — AWAITING INPUT (class: intent-bearing).** The run tried the neutral values and 10 settings of the two levers on seeds 1–120 in both orders (`implement-evidence/lone-forward/balanced/`). The keeper and striker arms fail at every setting with realistic play (13.6 or more shots per team per match). At `tackle_win_base` 0.5: keeper, the reduced side 2.46 against the full side 1.90 (+0.56, se 0.12), and the full side is 1.66 times the control against the 1.6 limit; striker 1.67 against 1.25 (+0.42, se 0.10); centre-back 1.21 against 1.09 (+0.13, se 0.10). The only pass is the centre-back arm at 7.7 shots per team, which RIM-12 rules out. The Q-LF1 answer said to stop and report in this case. Options:
  1. Allow a third lever that acts on the side with ten men, for example less pressing and running.
  2. Move the red-card criterion to `keeper-and-shots`, which owns the save model (the keeper arm is the worst arm). Ship the formations fix here (`tackle_win_base` 0.5 passed both pairings in the inner loop) and judge this slice on the other three criteria.
  3. Judge each arm against a relative target: the reduced side scores less than it does at 11 against 11.
  4. Keep the criterion and the levers, and allow a setting below the shot floor. RIM-12 rules this out today.
  scope: decides how the red-card criterion is met, moved or re-judged; does NOT change the pairing limits, the discipline criterion, the card chances or the keeper model.

## 2026-09-24T14:15:39Z · stage: implement · slice: lone-forward · product-owner answer

- **Q-LF2 The red-card criterion on balanced fixtures — ANSWERED: move it to keeper-and-shots.** `lone-forward` ships the formations fix: the two levers, tuned within their bounds (the inner loop found `tackle_win_base` 0.5 passes both pairings). This slice is judged on its other three criteria: the lone-forward formations hold (4-4-1-1 and 3-4-3 against 4-4-2, at most 4.0 goals per side), nothing that passes now regresses (the eight pairings and discipline), and a lone forward uses his team-mates. "No advantage from a red card" moves to `keeper-and-shots` on the balanced fixtures of Q-LF1, with every limit unchanged.
  scope: moves the red-card criterion a second time, from `lone-forward` to `keeper-and-shots`; does NOT change any limit, does NOT add a lever to `lone-forward`, does NOT allow play below the shot floor (RIM-12), and does NOT change the card chances. The ten-man side's behaviour (the measured cause: it attacks as if at full strength) is open for `keeper-and-shots` to address within its own scope, or to report.

## 2026-09-24T16:36:13Z · stage: plan · slice: keeper-and-shots (autonomous run)

- **Plan discovery — ANSWERED AUTONOMOUSLY (class: implementation-detail).** No product owner was present. The implementation questions are answered in `04-plan-keeper-and-shots.md` § Assumptions A1-A25: on target is decided at the kick from the trajectory (a copy of the ball stepped to the goal line) and counted then; the keeper attempts a save only on a shot on target, on the sourced curve (0.891 at quality 0.05 to 0.272 at 0.40) read from a fixed quality model so the xG refit cannot feed back; a save is held with a one-in-three share or parried; an outfield defender near the ball can block once per flight; parries and blocks are deflections with the defending side as the last touch, so corners come only from a crossing (RIM-9); the loft range lets shots rise over the bar; penalties and the shoot-out use the same keeper model with a penalty xG of 0.76, and the three hand-set shoot-out values move into the tuning file; no event kind, statistics field, snapshot field, report key or protocol message is added; the benchmark is re-baselined on a30313b at 1.613 us per tick.
  scope: decides how the agreed keeper-and-shots scope is built, including the red-card criterion moved here by Q-LF2; does NOT change a criterion, a limit, a band, a card chance or the ten-man side's behaviour.

## 2026-09-24T17:26:30Z · stage: implement · slice: keeper-and-shots · AWAITING INPUT (autonomous run stopped)

- **Implementation choices — ANSWERED AUTONOMOUSLY (class: implementation-detail).** The planned model is built at commit `8fd407b`: on target from the trajectory at the kick, saves only on target on the sourced curve with the sourced hold share (0.333), parries and blocks as deflections with the defending side as the last touch, and penalties and the shoot-out on the same save model. The tuned values are `shot_noise` 0.5, `parry_speed` 0.8, `block_reach` 3.0, `block_chance` 0.5, `block_speed` 0.8, `block_spread` 3.2 and `penalty_spread` 0.25. One xG refit gave intercept −4.191, distance −0.0441 and angle 6.3836. On seed 42 over 1,000 matches: on target 0.387, goals per xG 1.098, goal kicks 13.1 per match. 500 penalty scenes convert 0.796. The wide and over scenes pass. Details are in `05-implement-keeper-and-shots.md`.
  scope: builds the agreed scope with tuned values inside their bounds; does NOT change a criterion, a limit, a band, a card chance or a sourced value.
- **Q-KS1 The red-card criterion still fails after the save and block model — AWAITING INPUT (class: intent-bearing).** On the balanced fixtures (seeds 1–120 in both club orders), with every limit unchanged, all three arms fail. Control home 0.73, limit 1.17. Keeper: the reduced side 2.10 against the full side 1.18, and the full side is also above the limit. Centre-back: 0.83 against 0.77. Striker: 1.27 against 1.01. Before this slice the figures were 2.67 against 1.64, 1.08 against 0.88 and 1.79 against 1.21. The steer entry for Q-LF2 says to stop and report in this case. Options:
  1. Allow a lever on the side with ten men, for example less pressing, running or attacking width. This is the measured cause.
  2. Move the criterion to `tempo-and-restarts` or `realism-tuning`.
  3. Judge each arm against the reduced side's own 11-against-11 figure.
  scope: decides how the red-card criterion is met, moved or re-judged; does NOT change the card chances or the save model.
- **Q-KS2 Corners per team stay under 3.0 — AWAITING INPUT (class: intent-bearing).** The criterion "at least 3.0 corners per team over 200 matches" fails. It reached 1.21 per team in the slow test (1.59 on seed 42), with goal kicks at 11.2 per match. Every corner followed a defending touch over the goal line. With every block and parry value at its bound, the model's ceiling is 1.88–2.33 per team, and goal kicks then fall under 10. Shots give about 4 parries and 2.6 blocks per match, and about 36% of those end in a corner. An experiment that turned every parry toward the goal line reached 2.64 at the sourced hold share, and 3.18 at a hold share of 0.1, with goal kicks at 9.4. Options:
  1. Lower this slice's corner floor, for example to 1.2 per team, and leave the 3.5–6.5 band to later slices.
  2. Allow parries turned toward the goal line and a non-sourced hold share.
  3. Add corner sources from clearances and crosses, here or in `tempo-and-restarts`. That is a scope change.
  scope: decides how the corner criterion is met, lowered or moved; does NOT change the on-target, xG, wide-shot or penalty criteria.

## 2026-09-24T18:15:55Z · stage: implement · slice: keeper-and-shots · product-owner answers

- **Q-KS1 The red-card criterion after the save and block model — ANSWERED: move it to realism-tuning.** `keeper-and-shots` is not judged on "No advantage from a red card". The criterion moves to `realism-tuning`, on the balanced fixtures of Q-LF1, with every limit unchanged. `realism-tuning` already lists this criterion. The product owner was told that the measured cause (a side with ten men attacks as if it had eleven) has no lever in `realism-tuning` either, and that it will probably stop again; the product owner chose to move it.
  scope: moves the red-card criterion a third time, from `keeper-and-shots` to `realism-tuning`; does NOT add a ten-man lever to any slice, does NOT change any limit, and does NOT change the card chances or the save model.
- **Q-KS2 Corners per team — ANSWERED: lower this slice's floor to 1.2, and give clearance and cross corners to a later slice.** `keeper-and-shots` is judged on at least 1.2 corners per team over 200 matches (it reached 1.205), with goal kicks at least 10 per match and every corner still following a defending touch over the goal line. Corners from clearances and blocked crosses over a side's own goal line move to `tempo-and-restarts`. The 3.5–6.5 corners band is unchanged, and `realism-tuning` must still pass it.
  scope: lowers the corner floor of `keeper-and-shots` from 3.0 to 1.2, and adds clearance and cross corners to the scope of `tempo-and-restarts`; does NOT change the corners band, the goal-kick floor, or the on-target, xG, wide-shot and penalty criteria, and does NOT allow parries turned toward the goal line or a hold share that is not sourced.

## 2026-09-25T03:12:00Z · stage: plan · slice: tempo-and-restarts (autonomous run)

- **Plan discovery — ANSWERED AUTONOMOUSLY (class: implementation-detail).** No product owner was present. The implementation questions are answered in `04-plan-tempo-and-restarts.md` § Assumptions A1-A24: tempo is a soft carry window on the pass and clearance scores of an unpressed carrier (a shot and a pressed carrier are exempt), so every agent still decides every tick; the throw-in, goal-kick, corner and free-kick delays move to the sourced medians and never above them, with kick-off, penalty, drop ball and added time unchanged; a clearance is its own kick and the first kick of a restart taker is a restart kick, each under its own key, and `stats.passes` and pass accuracy cover open-play passes only; `stats.ball_in_play_s` is added to match-stats and the calibrate report shows minutes per 90 as a figure, not a band; the snapshot moves to version 6 for the new counters; corners from clearances and crosses (Q-KS2) come from a defender clearing a fast pass inside his own penalty area once per flight, by deflection; throw-ins, measured at 20-24 per match on shipped play, are raised only by physical levers (the clearance aim spread and the cross clearance); if a criterion cannot pass with every lever in bounds, implement stops and reports.
  scope: decides how the agreed tempo-and-restarts scope is built, including the clearance and cross corners moved here by Q-KS2; does NOT change a criterion, a limit, a band, the tick rate, the decision interval, added time, or the protocol.

## 2026-09-25T04:24:16Z · stage: implement · slice: tempo-and-restarts · AWAITING INPUT (autonomous run stopped)

- **Implementation choices — ANSWERED AUTONOMOUSLY (class: implementation-detail).** The counting and the new mechanisms are built and committed with play left as it was: a clearance is its own kick, the first kick of a restart taker is a restart kick, `stats.passes` and pass accuracy count open-play passes only, `stats.clearances`, `stats.restart_kicks` and `stats.ball_in_play_s` are in match-stats, the calibrate report shows `ball_in_play_min_per_90_mean`, the snapshot is version 6, and the carry window and the cross clearance are tuning values that ship off (`carry_cost` 0, `cross_chance` 0). The restart delays stay at 3, 8, 6 and 8 s until the tuning below is settled. On seed 42 over 200 matches the counting alone leaves play identical (throw-ins 21.1, goal kicks 11.9, goals 1.79) and moves passes from 1,225.8 to 1,193.8 per team; the ball is in play 89.6 minutes per 90. Details are in `05-implement-tempo-and-restarts.md`.
  scope: builds the counting and the levers of the agreed scope; does NOT change a criterion, a limit, a band, the tick rate, the decision interval, added time or the protocol.
- **Q-TR1 Fewer passes make the match a shooting gallery — AWAITING INPUT (class: intent-bearing).** The run tried 60 settings on seed 42 over 200 matches (`implement-evidence/tempo-and-restarts/inner/sweeps.txt`). Two walls:
  1. Throw-ins. The two levers the plan named (the clearance aim spread and the cross clearance, both at their bounds) reach 18.3 throw-ins per match. 35–55 needs the carrier to clear the ball more often (the `clear` weight, 18–30 clearances per team).
  2. Passes against shots. 22 of the 60 settings reach 550 or fewer passes per team. In all of them carriers dribble forward, and they give 27–61 shots per team (13.2 on shipped play; about 12 in real matches) and 4.0–14.8 goals per match. The one at 27 shots leaves the ball in play for 49.6 minutes, under 52. The best setting for the four criteria (carry window 1.5 s at cost 1.0, delays at the medians, aim spread 1.6, `min_lane` 0, `lane` 0, `clear` −0.45) gives on seed 42: passes 557, accuracy 83.7%, throw-ins 43.9, ball in play 52.6 minutes, shots 56.4 per team, goals 7.85. On the test seeds 1–200 it gives passes 603 (fails), accuracy 81.3%, throw-ins 40.0, ball in play 55.4 minutes. At that setting `every_formation_holds` fails in 6 of 10 pairings (4-4-1-1 v 4-4-2 5.17, 3-4-3 v 4-4-2 5.12, the 4-4-2 side against 3-5-2 5.59). The settings that keep shots near real (12.5 per team) leave passes near 700 and the ball in play near 68 minutes.
  The measured cause: in this engine a carried ball advances faster and more safely than a passed one, and a defender rarely stops a dribbler, so fewer passes means more attacks that end in a shot. No lever in the plan acts on that. The plan said to stop and report in this case. Options:
  1. Allow a defending lever in this slice so that a carrier is contested (for example the tackle chance against a dribbler, or how close pressers come), and keep every criterion.
  2. Ship the counting, the ball-in-play figure, the clearances and the restart delays here, with tuning that keeps shots realistic, and move "Passes are realistic" (350–550 per team) to `realism-tuning`, which already owns goals and shots.
  3. Accept the higher shots and goals as band misses (Q-E4) and also relax or move the formations criterion. RIM-10 makes that criterion yours to change.
  scope: decides how the pass-volume criterion is met, moved or re-judged; does NOT change the counting rule, the sourced restart medians, the ball-in-play or throw-in bands, added time or the tick rate.

## 2026-09-25T05:51:30Z · stage: implement · slice: tempo-and-restarts · product-owner answer

- **Q-TR1 Fewer passes make the match a shooting gallery — ANSWERED: option 1, a defending lever in this slice (Recommended).** `tempo-and-restarts` may add bounded, tunable defending values so that a ball carrier is contested: for example the chance to win the ball from a dribbler, and how close pressers come to the carrier. Every criterion of the slice stays, with its limits unchanged: passes 350–550 per team, pass accuracy 75–88%, ball in play 52–65 minutes, throw-ins 35–55, only real passes counted, and no regression in `every_formation_holds`. The `clear` weight may also be tuned for the throw-in floor. The reason: in this engine a dribbler advances faster and more safely than a pass, and defenders rarely stop him. This is the same root cause as the red-card failure, and no later slice has a lever for it.
  scope: widens `tempo-and-restarts` by defending levers that contest a carrier; does NOT change any limit or band, the card chance per foul (RIM-7), the save model, the sourced restart medians, added time or the tick rate. If the levers at their bounds still cannot meet a criterion with realistic shots (about 10–16 per team), implement stops and reports again.

## 2026-09-25T08:45:00Z · stage: implement · slice: tempo-and-restarts · awaiting product-owner answer

- **Q-TR2 The ball stays in play too long once shots are realistic — AWAITING INPUT (class: intent-bearing).** Under Q-TR1 the run added three defending values that contest a carrier (tackle reach, how close pressers come, and an extra win chance against a running carrier). It tried 181 settings on seed 42 over 200 matches (`implement-evidence/tempo-and-restarts/contest/`). The levers fixed the shots: 28 settings give 10–16 shots per team at 373–545 passes. But no setting meets every limit together. With passes 350–550, accuracy 75–88%, throw-ins 35–55 and 10–16 shots, the lowest ball-in-play figure is 65.5 minutes on seed 42, over the 65 limit. The best setting that also keeps sending-offs at or under 25% (`b0`) gives on the test seeds 1–200: passes 493.6 per team, accuracy 85.28%, throw-ins 52.95, ball in play 66.22 minutes, and on seed 42 14.7 shots per team.
  The measured cause: the restart delays already sit at the sourced medians, and throw-ins are at their cap. The missing dead time is corners, 1.98 per team at `b0` against the 3.5–6.5 band that `realism-tuning` owns. A fit over the runs gives about 0.27 minutes of play less for each extra corner per team. The discipline limit also binds: with realistic shots, fouls rise from 6.4 to 12–17 per team, and the sending-off share sits at 0.18–0.32 against 0.25. That needed the foul chance of one tackle attempt below its shipped 0.1 (0.04 at `b0`). The card chance per foul was not changed. Options:
  1. Ship `b0` here, and move "The ball is in play for about an hour" (52–65) to `realism-tuning`, which owns the corners band that would close the gap. (Recommended.)
  2. Accept 66–67 minutes as a band miss here (Q-E4) and keep the criterion in this slice at a limit you set.
  3. Allow a restart delay above its sourced median, or a longer kick-off delay (no sourced median), to add dead time.
  scope: decides how the ball-in-play criterion is met, moved or re-judged, and whether the foul chance per tackle attempt may ship at 0.04; does NOT change the counting rule, the throw-in or pass limits, the card chance per foul, the save model, added time or the tick rate.

## 2026-09-25T08:44:13Z · stage: implement · slice: tempo-and-restarts · product-owner answer

- **Q-TR2 The ball stays in play too long once shots are realistic — ANSWERED: option 2, ship `b0` and accept a band miss, with a slice limit of 52–68 minutes.** Ship setting `b0`, including the foul chance per tackle attempt at 0.04 (shipped 0.1); the card chance per foul is unchanged. In `tempo-and-restarts`, "The ball is in play for about an hour" is judged at 52–68 minutes per 90 over 200 matches (b0 measured 66.22 on seeds 1–200). The real-football band stays 52–65 minutes, and `realism-tuning` must still reach it (the corners band it owns closes the gap). Every other criterion of the slice keeps its limit.
  scope: sets this slice's ball-in-play limit to 52–68 minutes and allows the foul chance per tackle attempt to ship at 0.04; does NOT change the 52–65 band in `realism-bands.json` or any other band, the pass, accuracy, throw-in or counting criteria, the card chance per foul, the sourced restart medians, added time or the tick rate. The shipped setting must still pass `every_formation_holds`, the discipline criterion and the earlier slices' criteria; if it does not, implement stops and reports.

## 2026-09-25T09:15:00Z · stage: implement · slice: tempo-and-restarts · awaiting product-owner answer

- **Q-TR3 Setting `b0` fails the discipline criterion and the stronger-team criterion — AWAITING INPUT (class: intent-bearing).** Under Q-TR2 the run applied `b0` to the shipped tuning and ran the slice criteria and the earlier slices' slow criteria on the server (200 matches each, seeds 1–200, `implement-evidence/tempo-and-restarts/b0-ship/criteria.txt`). The slice's own criteria pass: passes 493.6 per team at 85.28%, throw-ins 52.95, ball in play 66.22 minutes (inside the Q-TR2 limit of 68). `every_formation_holds` passes, and so do the set-piece floor (1.465 corners per team, 13.705 goal kicks), the lone-forward tests, the trailing-AI test and the mentality test. Two earlier criteria fail:
  - `discipline_is_realistic`: 0.160 second yellows per match (limit 0.10) and a sending-off in 30.0% of matches (limit 25%).
  - `a_stronger_team_wins_more_than_half_its_matches`: won 94, drew 81, lost 25 of 200 (needs more than 100 wins). Scoring is low at `b0`: in the red-card suite the control sides scored 0.63 and 0.69 goals per match.
  The red-card criterion (owned by `realism-tuning`) was measured as asked: the full side scored 1.32 (keeper sent off), 1.13 (centre-back) and 1.37 (striker) against a limit of 1.01. Q-TR2 says to stop in this case, so `b0` is not shipped: the tree stays at the committed play and the `b0` change is kept as a patch (`b0-ship/b0-ship.patch`). Options:
  1. Allow the rerun to leave `b0` and tune inside the Q-TR1 levers (the foul chance per tackle attempt below 0.04 included) with the discipline and stronger-team criteria added to the inner loop, keeping the Q-TR2 limits. If no in-bounds setting passes, stop again. (Recommended.)
  2. Ship `b0` and move the discipline and stronger-team criteria to `realism-tuning`, which owns goals.
  3. Ship the counting only (the levers stay off, as now) and move the passes, ball-in-play and throw-in criteria to `realism-tuning`.
  scope: decides whether the shipped setting may differ from `b0` and which slice judges discipline and the stronger team; does NOT change any limit, band, the card chance per foul, the sourced restart medians, added time or the tick rate.

## 2026-09-25T09:11:19Z · stage: implement · slice: tempo-and-restarts · product-owner answer

- **Q-TR3 Setting `b0` fails the discipline and stronger-team criteria — ANSWERED: option 3, ship the counting only.** `tempo-and-restarts` ships what is committed: the pass counting (clearances and restart kicks counted apart from passes), the ball-in-play figure, the clearances and the restart delays, with the contest levers built but at values that leave play unchanged (as at `4022e47`). `b0` is not shipped. The slice is judged on "Only real passes count" and on no regression of the earlier criteria. "Passes are realistic" (350–550 per team, 75–88%), "The ball is in play for about an hour" (the 52–65 band) and "Throw-ins stay in band" (35–55) move to `realism-tuning`. The Q-TR2 slice limit of 52–68 minutes no longer applies. The contest levers and the foul chance per tackle attempt are tuning values, so `realism-tuning` may tune them; the `b0` patch and its measurements are its starting evidence.
  scope: narrows `tempo-and-restarts` to the counting and moves three criteria to `realism-tuning`; does NOT change any limit or band, the card chance per foul, the sourced restart medians, added time or the tick rate, and does NOT ship `b0`.

## 2026-09-25T18:12:40Z · stage: plan · slice: realism-tuning · product-owner answers

- **Q-RT1 Formations target — ANSWERED: 10 v 4-4-2 (Recommended).** Judge formations on `every_formation_holds` (each formation against 4-4-2, 120 matches), tightened from "at most 4.0 goals per side" to the goals-per-match band 2.4–3.2. The 55-pairing formations suite runs and is recorded, but it is not a gate. A later slice owns the full 55 pairings.
  scope: narrows the slice criterion "every band passes in every suite, including the formations suite" to the equal and strength suites plus the tightened 10-pairing test; does NOT widen any band.
- **Q-RT2 Corners — ANSWERED: add a corner source (Recommended).** The slice may add code for a new corner source: defensive clearances and deflected crosses that go over the defenders' own goal line. A corner still needs a real crossing of the line after a defending touch (RIM-9). The corners band stays 3.5–6.5 per team.
  scope: allows engine code beyond `tuning.json` for corners; does NOT change the save curve, the hold share or RIM-9.
- **Q-RT3 Extra levers — ANSWERED: all four.** Besides the contest levers and `foul_base` (Q-TR3), the slice may: ship the sourced restart delays (throw-in 13.8 s, corner 31.8 s, goal kick 23.2 s, free kick 32.5 s); tune the shot decision weights, `shot_range`, the shot and aim noise, and refit the xG coefficients; tune `foul_booked_factor` and `foul_cooldown_ticks`; tune the carrier decision weights and the carry window.
  scope: widens the tunable set; does NOT change the card chance per foul (`yellow_base`, `yellow_aggression_weight`, `red_base`), the save curve, the hold share, any band or any limit.
- **Q-RT4 Gate seeds — ANSWERED: formations on seed 42 (Recommended).** The full gate runs equal and strength on seeds 42, 1, 7, 99 and 2026, and the formations suite on seed 42 only (Q-I1). About 3.5 hours on the server.
  scope: replaces "formations on every seed" in the slice text; does NOT change the equal and strength seed set.

## 2026-09-25T20:21:36Z · stage: implement · slice: realism-tuning · product-owner answer

- **Q-RT5 The ten formations against 4-4-2 cannot all reach 2.4–3.2 goals per match with the tunable values — ANSWERED: stop; new slice later.** The feasibility stage screened 13 settings (`implement-evidence/realism-tuning/screens/g4.log`, `fm1.log`). 5-3-2 never scored more than 1.38 goals per match against 4-4-2, and every setting that raised the back-five shapes pushed attacking shapes above 3.2. Keep the committed corner source (`bdb6626`, `e6708aa`, shipped at values that leave play unchanged). `realism-tuning` stops here, blocked. A separate slice for formation behaviour comes before the tuning resumes.
  scope: stops `realism-tuning` with no tuned value written; does NOT widen any band, raise any limit, or change the Q-RT1 formations gate; does NOT create the new slice (the product owner starts it).
