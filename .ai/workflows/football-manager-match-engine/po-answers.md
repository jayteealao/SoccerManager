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
