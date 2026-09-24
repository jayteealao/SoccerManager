---
schema: sdlc/v1
type: observability-plan
slug: soccer-manager
plan-version: 1
created-at: "2026-09-21T21:34:34Z"
updated-at: "2026-09-21T21:34:34Z"
project-name: "SoccerManager"
ship-plan-read: false

# === Required core ===

# Block A — Wide-event schema (language-agnostic canonical field vocabulary)
schema-core:
  correlation: [match.id, owner.id, run.id, session.id, seed]
  service:     [service, version, env]
  outcome:     [operation, duration_ms, outcome, status]
  actor:       [owner.id, manager.kind]
  error:       [error.type, error.code, error.retriable]
schema-domain:
  - { key: "record.kind", type: "enum(match-event, match-stats, run-report, viewer-session)", unit: "all" }
  - { key: "schema.version", type: "string", unit: "all" }
  - { key: "build.hash", type: "string", unit: "all" }
  - { key: "rules.pack_version", type: "string", unit: "engine" }
  - { key: "tick", type: "int", unit: "engine" }
  - { key: "minute", type: "int", unit: "engine" }
  - { key: "event.type", type: "enum(kick-off, goal, shot, pass, foul, card, corner, throw-in, goal-kick, free-kick, penalty, offside, injury, substitution, tactics-change, ai-decision, snapshot, lag, half-time, full-time)", unit: "engine" }
  - { key: "team.id", type: "string", unit: "engine" }
  - { key: "player.id", type: "string", unit: "engine" }
  - { key: "player.secondary_id", type: "string", unit: "engine" }
  - { key: "home.score", type: "int", unit: "engine" }
  - { key: "away.score", type: "int", unit: "engine" }
  - { key: "stats.possession_pct", type: "float", unit: "engine" }
  - { key: "stats.shots", type: "int", unit: "engine" }
  - { key: "stats.shots_on_target", type: "int", unit: "engine" }
  - { key: "stats.xg", type: "float", unit: "engine" }
  - { key: "stats.passes", type: "int", unit: "engine" }
  - { key: "stats.pass_accuracy_pct", type: "float", unit: "engine" }
  - { key: "stats.fouls", type: "int", unit: "engine" }
  - { key: "stats.corners", type: "int", unit: "engine" }
  - { key: "stats.throw_ins", type: "int", unit: "engine" }
  - { key: "stats.goal_kicks", type: "int", unit: "engine" }
  - { key: "stats.offsides", type: "int", unit: "engine" }
  - { key: "stats.goals", type: "int", unit: "engine" }
  - { key: "fatigue.mean_pct", type: "float", unit: "engine" }
  - { key: "injury.count", type: "int", unit: "engine" }
  - { key: "ai.decision", type: "string", unit: "engine" }
  - { key: "change.kind", type: "enum(tactics, substitution)", unit: "engine" }
  - { key: "change.queued_tick", type: "int", unit: "engine" }
  - { key: "change.applied_tick", type: "int", unit: "engine" }
  - { key: "change.rejected_reason", type: "string", unit: "engine" }
  - { key: "snapshot.tick", type: "int", unit: "engine" }
  - { key: "engine.lag_ms", type: "float", unit: "engine" }
  - { key: "engine.sustained_speed", type: "float", unit: "engine" }
  - { key: "engine.ticks_per_s", type: "float", unit: "engine" }
  - { key: "bench.match_wall_ms", type: "int", unit: "engine-cli" }
  - { key: "bench.cpu_ms", type: "int", unit: "engine-cli" }
  - { key: "bench.peak_mem_mb", type: "float", unit: "engine-cli" }
  - { key: "bench.matches", type: "int", unit: "engine-cli" }
  - { key: "machine.hash", type: "string", unit: "engine-cli" }
  - { key: "darkpath.change_never_applied", type: "int", unit: "engine" }
  - { key: "change.expired_at_full_time", type: "int", unit: "engine" }
  - { key: "darkpath.match_without_stats", type: "int", unit: "engine-cli" }
  - { key: "darkpath.viewer_dropped_ticks", type: "int", unit: "viewer" }
  - { key: "viewer.frame_ms_p50", type: "float", unit: "viewer" }
  - { key: "viewer.frame_ms_p95", type: "float", unit: "viewer" }
  - { key: "viewer.speed", type: "float", unit: "viewer" }
  - { key: "viewer.socket_drops", type: "int", unit: "viewer" }
  - { key: "viewer.recoveries", type: "int", unit: "viewer" }
key-normalization: "One canonical key per concept, dotted and lower-case: owner.id (never user.id, userId, or owner_id), match.id, run.id, session.id. The same key carries the same meaning in every record kind and every unit."

# Block B — Emit layer (per unit / language)
emit:
  - unit: "engine"
    language: "rust"
    mechanism: "A match-scoped builder accumulates context from kick-off; one match-stats record is emitted once at full time or abandonment. Every event appends one match-event record as it happens. Emission uses serde to JSON Lines; the tracing crate carries developer diagnostics only and never the records."
    builder-location: "crates/engine/src/observe/ (planned)"
  - unit: "engine-cli"
    language: "rust"
    mechanism: "A run-scoped builder accumulates every match-stats record of a calibrate or benchmark run; one run-report record is emitted once at run end, with the aggregate statistics, the band checks, the tripwire checks, and the dark-path counters."
    builder-location: "crates/engine-cli/src/report/ (planned)"
  - unit: "viewer"
    language: "javascript"
    mechanism: "A session-scoped builder accumulates frame timing, dropped ticks, socket drops, and recoveries; one viewer-session record is emitted once on match end or page unload, sent over the existing socket to the engine, which writes it into the match folder. Errors emit a match-event of type error at once."
    builder-location: "web/src/observe/ (planned)"

# Block C — Redaction / PII
redaction:
  deny: [password, token, api_key, authorization, cookie, ssn, card_number, hostname, absolute_path, free_text]
  hash: [machine]
  policy: "owner.id is an opaque identifier generated on the player's machine; no name or email exists in any record. machine is hashed (SHA-256, first 12 hex characters) before it enters a run-report. Absolute file paths are dropped; only paths relative to the data folder are recorded. Free text typed by the player never enters a record. Team and player names are fictional and generated, and are allowed."

# Block D — Sampling
sampling:
  always-keep: [errors, slow-matches, band-outliers, dark-path-hits, seeded-runs, lag-events, slow-frames]
  slow-threshold-ms: 2000
  base-rate: 1.0
  cost-posture: "Records (match-event, match-stats, run-report, viewer-session) are kept at 100 percent. Per-tick telemetry is summarized per simulated minute (p50 and p95) and kept in full only when engine lag exceeds one tick (20 ms) or a viewer frame exceeds 33 ms. Disk, not ingest, is the cost: the data folder is capped at 2 GB; the oldest tick histories are pruned first; calibration runs keep the run-report, every match-stats record, and the match-event streams of outlier and failed matches only."

# Block E — Collection pipeline
pipeline:
  transport: direct-sdk-export
  collector-config-target: "none"
  notes: "Direct file writes by the engine process: no SDK, no collector, no agent. The engine writes records to the data folder at every stoppage snapshot and at match end, and sends every match-event over the existing local socket to connected clients (the viewer and the debug dashboard). Headless runs use the file sink only. The engine serves the data folder read-only over the same local endpoint so the dashboard page loads records without file-system access."

# Block F — Backend + query layer
backend:
  platform: local-files-and-in-repo-dashboard
  storage: "SM_DATA_DIR/matches/<match.id>/events.jsonl (match-event), stats.json (match-stats), session.json (viewer-session), ticks.bin (compact tick history for rewind); SM_DATA_DIR/runs/<run.id>/report.json (run-report) and stats/<match.id>.json"
  query-dialect: "JavaScript over JSON Lines in the dashboard page; DuckDB SQL over the same files as an optional analyst path"
  retention: "Real matches: every record kept until the 2 GB cap, then the oldest ticks.bin files are pruned first, then the oldest match folders. Calibration runs: report.json and stats/ kept without limit; events.jsonl kept for outlier and failed matches only."
  lock-in-note: "No vendor, no infrastructure, no recurring cost, no data leaving the player's machine. The tradeoff is query power: no full-text index and no cross-machine aggregation. Migration path: ship the same JSON Lines files through an OpenTelemetry Collector or Vector into Grafana or a vendor later without changing the emit layer."

# Block G — Dashboards
dashboards:
  target: in-repo-dashboard-page
  analyses:
    - { id: realism-bands, query-intent: "distributions of goals per match, shots per team, possession, and win rate of the stronger team per run-report, the eleven bands of version 2, and the goal bands of every formation pairing (calib.formations), drawn against the accepted bands" }
    - { id: performance-tripwires, query-intent: "engine.ticks_per_s, bench.match_wall_ms, bench.cpu_ms, and bench.peak_mem_mb per build.hash, with the 10 percent CPU and 25 percent memory tripwires" }
    - { id: dark-paths, query-intent: "the three dark-path counters per run and per match; any non-zero value is highlighted" }
    - { id: match-timeline, query-intent: "one match's match-event stream by minute with score, xG, fatigue, cards, substitutions, and queued versus applied ticks of every change" }
    - { id: ai-decisions, query-intent: "ai-decision events by minute and score state; the share of runs with a tactical change before full time when the AI team trails after minute 70" }
    - { id: viewer-health, query-intent: "viewer.frame_ms_p95, darkpath.viewer_dropped_ticks, viewer.socket_drops, and viewer.recoveries per session and per playback speed" }

# Block H — Provisioning + environments
provisioning:
  ceiling: emit-iac
  never-store-credentials: true
  environments:
    - { name: "dev", backend-endpoint-env-var: "SM_DATA_DIR" }
    - { name: "play", backend-endpoint-env-var: "SM_DATA_DIR" }
    - { name: "calibration", backend-endpoint-env-var: "SM_DATA_DIR" }

# Client/edge (optional)
client-edge:
  in-scope: true
  notes: "The viewer (browser page) emits one viewer-session record per session and a match-event of type error for every caught error, over the existing local socket. No sendBeacon and no remote endpoint. Frame timing and dropped-tick counters are the viewer's own signals for the dark path 'a viewer that drops ticks silently'."

# === Extensions — open schema ===
additional-contracts:
  - id: realism-bands
    purpose: "The accepted realism thresholds live as data so the calibration harness, the dashboard, and the audit read one source."
    fields: { schema_version: 2, goals_per_match: "2.4-3.2", shots_per_team: "8-16", possession_pct: "35-65", stronger_team_win_rate: ">0.50 with 15 percent higher attributes", ten_plus_goals_share: "0-0.005", sending_off_share: "0.08-0.22", yellow_cards_per_team: "1.2-2.6", shots_on_target_share: "0.30-0.42", goals_per_xg: "0.85-1.15", passes_per_team: "350-550", pass_accuracy_pct: "75-88", corners_per_team: "3.5-6.5", throw_ins_per_match: "35-55", goal_kicks_per_match: "12-22", goalless_share: "0.04-0.12", formation_pairings: "goals_per_match, ten_plus_goals_share and goalless_share for every pairing of the shipped formations", sample_size: 1000 }
    enforced-by: "calibration harness assertion (signal calibrate.band_failed names each miss on stderr); audit lens schema-and-emit"
  - id: benchmark-tripwires
    purpose: "Every engine slice after the first reruns the benchmark and fails on regression."
    fields: { budget_match_wall_ms: 2000, budget_1000_matches_min: 30, cpu_regression_pct: 10, memory_regression_pct: 25, threads: 1 }
    enforced-by: "benchmark harness compare; verify"
  - id: dark-paths
    purpose: "Three counters must read zero over every calibration run. change_never_applied counts only a change that a stoppage admitting its kind should have applied and did not; a change still queued at full time because no admitting stoppage came after it was queued is expired, counted in change.expired_at_full_time, with no zero rule."
    fields: { change_never_applied: 0, match_without_stats: 0, viewer_dropped_ticks_silent: 0 }
    enforced-by: "calibration harness assertion; dashboard dark-paths; audit"
  - id: schema-versioning
    purpose: "Every record carries schema.version; a reader that meets an unknown version refuses the file with a named error."
    fields: { version_field: "schema.version", current: "1", reader_policy: "fail-closed", schema_files: "schemas/observability/*.schema.json (planned)" }
    enforced-by: "cargo test; audit lens schema-and-emit"
---

# Observability Plan — SoccerManager

## The Observability Posture

The repository holds no source, no logger, and no dashboard. What exists is a specification: the engine must emit one structured event stream and one statistics record per match, the calibration harness must prove realism over 1000 matches, and three dark paths must never stay silent. The product owner named this pipeline as the measure of success, so the contract exists before the first line of engine code.

The chosen path is local files plus an in-repo dashboard page. The game runs on the player's own machine with no server, so a collector, a vendor, or a container stack would add operations without adding signal. The engine writes JSON Lines records to a data folder and feeds every event over the local socket it already serves; the dashboard page, written in the same plain JavaScript as the viewer, reads the folder through the engine and draws six analyses. Records are kept at 100 percent because one match is the unit of work and a match is small; per-tick telemetry is the only stream that is summarized.

The one decision that most shapes debuggability is the schema: four record kinds share one dotted key vocabulary, and every record carries a schema version that a fail-closed reader checks. The top risk is drift between the engine's serde structs and the schema files once slices ship in parallel; the calibration slice validates every record against the schema files, and the audit re-checks it.

## Wide-event schema

Four record kinds carry the vocabulary in Block A:

1. `match-event`: one record per engine event, appended as the event happens. Keys: correlation, service, `tick`, `minute`, `event.type`, `team.id`, `player.id`, `player.secondary_id`, the score, and the change and snapshot keys where the event type uses them.
2. `match-stats`: one wide event per match, emitted once at full time or abandonment. Keys: correlation, service, outcome, error, both teams' statistics (throw-ins and goal kicks included), fatigue and injury summaries, the counts of queued, applied, and rejected changes, the snapshot count, engine lag summary, the dark-path counter `darkpath.change_never_applied`, and `change.expired_at_full_time` (changes still queued at full time with no admitting stoppage after them; no zero rule).
3. `run-report`: one record per calibrate or benchmark run. Keys: correlation (`run.id`, `seed`), service, outcome, `bench.*`, `machine.hash`, `fixtures.hash` (the inputs that decide the fixtures: attributes, rules, tactics, the generator block, and the default clubs), the aggregate of every `match-stats` record per suite (`equal`, `strength`, `formations`, and `red-card` when named), the band checks (each with its sampling error `se`; a check of the formations suite carries its `pairing`, and a check of the red-card suite its arm), the figures of every formation pairing played (`calib.formations`), the red-card experiment (`calib.red_card`), what the run selected (`calib.selection`), and, with `--baseline`, the baseline (`calib.baseline`) and the band-by-band diff (`calib.diff`), the tripwire checks, and the dark-path counters. Each failing band is also one `warn` line on stderr with `signal` `calibrate.band_failed`; a diff is also one `info` line with `signal` `calibrate.diff`.
4. `viewer-session`: one wide event per viewer session. Keys: correlation (`session.id`, `match.id`, `owner.id`), service, outcome, `viewer.*`, and `darkpath.viewer_dropped_ticks`.

Every record carries `record.kind`, `schema.version`, `build.hash`, `service`, `version`, and `env`. `owner.id` is the actor key; `manager.kind` is `human` or `ai`. Keys are dotted and lower-case; one key per concept in every unit.

## Emit layer

- Engine (Rust): a match-scoped builder starts at kick-off and accumulates context; `match-stats` is emitted once at the end. `match-event` records append as events happen. Serialization is `serde` to JSON Lines. The `tracing` crate carries developer diagnostics to stderr and never carries the records. Planned location: `crates/engine/src/observe/`.
- Engine CLI (Rust): a run-scoped builder collects every `match-stats` record and emits one `run-report` at run end. Planned location: `crates/engine-cli/src/report/`.
- Viewer (JavaScript, one adapter among three): a session-scoped builder accumulates frame timing and counters and emits one `viewer-session` record on match end or page unload, over the socket to the engine. Caught errors emit a `match-event` of type `error` at once. Planned location: `web/src/observe/`.

## Redaction & PII

- `owner.id` is an opaque identifier generated on the player's machine. No name, email, or account exists in any record.
- `machine` is hashed with SHA-256 and truncated to 12 hex characters before it enters a `run-report`. The hash keeps benchmark reproducibility without the hostname.
- Absolute file paths are dropped. Only paths relative to the data folder are recorded.
- Free text typed by the player never enters a record.
- The standard denylist (passwords, tokens, API keys, authorization headers, cookies, government and card numbers) applies to every unit even though no such field exists today.
- Team and player names are fictional and generated, and are allowed.

## Sampling

- Records are kept at 100 percent: `match-event`, `match-stats`, `run-report`, `viewer-session`.
- Per-tick telemetry (engine lag, viewer frame time) is summarized per simulated minute as p50 and p95. Full per-tick detail is kept only when engine lag exceeds one tick (20 ms) or a viewer frame exceeds 33 ms.
- Always keep: errors, matches slower than 2000 ms headless, matches outside the realism bands, dark-path hits, seeded runs, lag events, slow frames.
- Cost is disk: the data folder is capped at 2 GB; the oldest `ticks.bin` files are pruned first, then the oldest match folders. Calibration runs keep the `run-report`, every `match-stats` record, and the `match-event` streams of outlier and failed matches only.

## Collection pipeline

Direct file writes by the engine: no SDK, no collector, no agent. The engine writes records at every stoppage snapshot and at match end, so a crash loses at most the play since the last stoppage. Every `match-event` also travels over the existing local socket to connected clients: the viewer and the dashboard page. Headless runs use the file sink only. The engine serves the data folder read-only over the same local endpoint, so the dashboard page loads records without file-system access. This resolves the open question on sink and transport: file and socket, both.

## Backend & query layer

Platform: local files plus an in-repo dashboard page. Storage layout under `SM_DATA_DIR`: `matches/<match.id>/events.jsonl`, `stats.json`, `session.json`, `ticks.bin`; `runs/<run.id>/report.json` and `stats/<match.id>.json`. Query dialect: JavaScript over JSON Lines in the dashboard; DuckDB SQL over the same files as an optional analyst path. No vendor, no infrastructure, no recurring cost, and no data leaves the player's machine. The tradeoff is query power: no full-text index and no cross-machine aggregation. The migration path is to ship the same JSON Lines files through an OpenTelemetry Collector or Vector into Grafana or a vendor later, without changing the emit layer.

## Dashboards

Target: an in-repo dashboard page (dashboards as code, plain JavaScript, the viewer's stack). Six analyses:

1. Realism bands: distributions of goals, shots, possession, and the stronger team's win rate per run against the accepted bands.
2. Performance and tripwires: ticks per second, match wall time, CPU time, and peak memory per build hash, with the 10 percent CPU and 25 percent memory tripwires.
3. Dark paths: the three counters per run and per match; any non-zero value is highlighted.
4. Match timeline: one match's events by minute with score, expected goals, fatigue, cards, substitutions, and the queued and applied ticks of every change.
5. AI decisions: tactical changes by minute and score state; the share of runs with a change before full time when the AI team trails after minute 70.
6. Viewer health: frame time p95, dropped ticks, socket drops, and recoveries per session and per playback speed.

## Provisioning & environments

Ceiling: `emit-iac`, read here as files in the repository only: the schema files, the dashboard page, and the data folder layout. Nothing is applied to any remote; there is no remote. Credentials are never stored, because none exist. Three environments (`dev`, `play`, `calibration`) share one variable, `SM_DATA_DIR`, which names the data folder; the default is a per-user application-data folder on Windows.

## Additional contracts

### realism-bands
The accepted thresholds as data, file version 2: goals per match 2.4 to 3.2; shots per team 8 to 16; possession 35 to 65 percent; the stronger team (15 percent higher attributes) wins more than 50 percent; sample size 1000. Version 2 adds eleven bands from real-match data, checked in the equal suite: matches with 10 or more goals at most 0.5 percent; matches with a sending-off 8 to 22 percent; yellow cards per team 1.2 to 2.6; shots on target 30 to 42 percent of shots; goals per expected goal 0.85 to 1.15; passes per team 350 to 550; pass accuracy 75 to 88 percent; corners per team 3.5 to 6.5; throw-ins per match 35 to 55; goal kicks per match 12 to 22; goalless matches 4 to 12 percent. The formations suite checks goals per match, the 10-or-more-goals share and the goalless share for every pairing of the shipped formations. A bands file of version 1 is refused. Enforced by the calibration harness and the audit.

### benchmark-tripwires
One match under 2000 ms on one thread; 1000 matches under 30 minutes; more than 10 percent CPU or 25 percent memory regression between builds fails. Enforced by the benchmark harness compare and by verification.

### dark-paths
Three counters must read zero over every calibration run: a change that never applied, a match without a statistics record, viewer ticks dropped without a notice. Enforced by the calibration harness, the dark-paths dashboard, and the audit.

### schema-versioning
Every record carries `schema.version`, currently `1`. Schema files are planned at `schemas/observability/*.schema.json`. A reader that meets an unknown version refuses the file with a named error. Enforced by the engine tests and the audit.
