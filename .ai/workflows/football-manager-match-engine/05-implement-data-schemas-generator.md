---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: data-schemas-generator
status: complete
stage-number: 5
created-at: "2026-09-22T07:18:04Z"
updated-at: "2026-09-22T07:18:04Z"
metric-files-changed: 47
metric-lines-added: 5895
metric-lines-removed: 210
metric-deviations-from-plan: 8
metric-review-fixes-applied: 0
commit-sha: ""
tags: [engine, data, generator, modding, rust, identity]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-data-schemas-generator.md
  plan: 04-plan-data-schemas-generator.md
  instrument: 04b-instrument.md
  benchmark: 05c-benchmark.md
  probe: 03-slice-probe-engine-core.md
  siblings: [05-implement-engine-core.md]
  verify: 06-verify-data-schemas-generator.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine data-schemas-generator"
---

# Implement: Data Schemas and Team Generator

## The Implementation

The plan handed over a working engine whose two teams were built in code with every attribute at 60, a `Tuning` with no bounds, an owner identifier fixed at `local-cli`, and 36 reserved bytes in the tick-file header. It fixed twenty steps, eleven product-owner answers, `garde` 0.23 as the validator, JSON as the file format, and one rule above the rest: the loader fails closed and names the file and the field. No commit and no source change landed between the plan and this build, so the research pass was a direct re-read of every seam at its recorded line.

The build touched 47 files in the plan's order and added 5,895 lines. One generic loader (`load_json`) serves the four file kinds: it reads, peeks `schema_version`, deserializes with `deny_unknown_fields`, validates with `garde`, and turns the first report line into `EngineError::Data` with the kind, the relative path, the field path, and the reason. Attributes became a fixed array of 50 bytes with six derived values computed once at load, so the hot path reads `derived.tackling` instead of a field by name and `Player` stays `Copy`. The generator draws a bell curve from twelve uniform draws, and the binary wrote the two shipped default clubs itself (`generate --seed 1` and `--seed 2`), so the defaults and the generator cannot diverge. Identity follows the observability contract: `owner.id` is created once under `SM_DATA_DIR`, and `match.id` rides in `stats.json` and in header schema 2 at bytes 28 and 44. Eight steps deviated from the plan in detail, none in contract: two error variants were reshaped so the command line prints a failure chain once, the generator's `per_position` map is keyed by the position code string because `garde` validates maps only with string-like keys, and the tick reader returns a named `TickFile { header, records }` instead of a tuple. All 65 tests pass, `rustfmt` and `clippy -D warnings` are clean, and the benchmark compare is inside every tripwire: 390 ms median against 393, 384 ms CPU per match against a 426 tripwire, and 5.69 MB peak against a 6.33 tripwire.

Verify next re-runs the gates, the six automated criteria, the benchmark compare, and the instrument key check against the live records. The top open risk is unchanged from the plan: per-player attributes and derived values now feed the arithmetic every tick, and determinism rests on the byte-identical test and the same-seed generator test, both of which pass today on one machine.

## Summary of Changes

- `garde` 0.23 added with `default-features = false, features = ["derive"]`; `rust-version` raised to 1.87.
- `crates/engine/src/data/`: one fail-closed loader, four schema types (`AttributeSchema`, `TuningFile`, `RulePack`, `TeamFile`), the content folder resolver, the team generator, and the name tables.
- `EngineError` gains `Data`, `Version`, and `Read`; `Io` no longer repeats its source in its display.
- `Attributes` is a fixed 50-byte array in schema order; `Derived` holds the six values the hot path reads; `Team` carries club id, name, kit, and player ids; `Team::builtin` and `Attributes::uniform` are deleted.
- `MatchConfig::new(seed, minutes, &Content, [&TeamFile; 2])` builds both teams and the content hash; `Simulation::new` consumes them.
- `EngineRng::bell` and `EngineRng::attribute` (Irwin-Hall, clamped to 1..=100).
- Tick-file header schema 2: `owner_id` at byte 28, `match_millis` at byte 44; `read_ticks` refuses version 1 naming both versions.
- `observe::identity`: `data_dir`, `load_or_create_owner_id`, `MatchId`; `MatchStats` gains `owner.id`, `content.hash`, `teams`; `write_stats` and `read_stats`.
- `content/`: `attributes.json` (36 names), `tuning.json`, `rules/default.json`, `teams/default-a.json`, `teams/default-b.json`, `README.md`.
- Command line: `--content-dir`, `--team-a`, `--team-b`, `generate {--seed, --clubs, --out, --force}`; colour on stderr only on a terminal; the error chain printed once; every help line at most 78 columns.
- Tests: 65 (44 unit, 16 engine integration, 5 command line), up from 32.

## Files Changed

- `Cargo.toml`: `rust-version = "1.87"`; `garde` in the workspace dependencies.
- `Cargo.lock`: `garde`, `garde_derive`, `compact_str`, `smallvec` resolved.
- `README.md`: content, runtime folder, generate, and the content hash in the bench report.
- `content/attributes.json`: the 36-name schema in four groups.
- `content/tuning.json`: the engine block equal to `Tuning::default()`, the generator block, the fatigue block.
- `content/rules/default.json`: the default rule pack with all nine stoppage kinds.
- `content/teams/default-a.json`, `content/teams/default-b.json`: generated by the binary from seeds 1 and 2.
- `content/README.md`: every field, unit, default, and bound; the position codes; the messages a bad file produces.
- `crates/engine/Cargo.toml`: `garde` dependency.
- `crates/engine/src/lib.rs`: `pub mod data`; re-exports `Content`, `ContentDir`, `TickHeader`.
- `crates/engine/src/error.rs`: `Data`, `Version`, `Read` variants; `Io` display without its source.
- `crates/engine/src/data/mod.rs`: `ContentDir::resolve`, `relative`, `load_json`, `Loaded`, `Content::load`, `Content::load_team`, `hex12`.
- `crates/engine/src/data/attributes.rs`: `Group`, `AttributeDef`, `AttributeSchema` with the length, dive, and required-name rules.
- `crates/engine/src/data/tuning.rs`: `TuningFile`, `GeneratorTuning`, `GroupDist`, `Dist`, `FatigueTuning`.
- `crates/engine/src/data/rules.rs`: `StoppageKind`, `Stoppage`, `Substitutions`, `RulePack` with the every-kind-once rule.
- `crates/engine/src/data/team.rs`: `Position`, `Kit`, `Club`, `PlayerEntry`, `TeamFile` with the schema context and the squad check.
- `crates/engine/src/data/generator.rs`: `generate_league` and the `generator.league` signal.
- `crates/engine/src/data/names.rs`: syllable tables, club suffixes, twelve kit colours.
- `crates/engine/src/player.rs`: `Attributes` array, `Derived`, `Player::derived`; `max_speed()` and `max_accel()` read `derived`.
- `crates/engine/src/team.rs`: `Team::new`, `Team::from_file`, club fields; `builtin` and `players` deleted.
- `crates/engine/src/tuning.rs`: serde and `garde` derives, one bound per field, `PartialEq`.
- `crates/engine/src/rng.rs`: `bell`, `attribute`, and their test.
- `crates/engine/src/sim.rs`: `MatchConfig` with teams, players, and `content_hash`; `club_ids`; hot-path reads through `derived`; `is_multiple_of`.
- `crates/engine/src/decision.rs`: passing skill through `derived`.
- `crates/engine/src/steering.rs`: `max_speed()` without a tuning argument; test player through `test_support`.
- `crates/engine/src/validate.rs`: tests use `team::test_support::bare`.
- `crates/engine/src/record.rs`: `SCHEMA_VERSION` 2, `TickHeader`, `FileSink::create(path, &TickHeader)`, `TickFile { header, records }`, version-1 refusal test.
- `crates/engine/src/observe/mod.rs`: `Record::owner_id`, `TeamRef`, `MatchStats` and `RunReport` fields, `write_stats`, `read_stats`; `OWNER_ID` and `match_id` deleted.
- `crates/engine/src/observe/identity.rs`: `data_dir`, `load_or_create_owner_id`, `owner_bytes`, `owner_hex`, `MatchId`.
- `crates/engine/tests/common/mod.rs`: `content_dir`, `content`, `default_teams`, `fixture_path`, `full_match` on the shipped content, `header`.
- `crates/engine/tests/content.rs`: AC-a, AC-c, tuning pinned to the default, missing file names its path.
- `crates/engine/tests/team_files.rs`: AC-b.
- `crates/engine/tests/generator.rs`: AC-d and the same-seed test.
- `crates/engine/tests/identity.rs`: AC-e through `stats.json` and through the tick header.
- `crates/engine/tests/determinism.rs`, `full_match.rs`: the new header; `owner_id` asserted on read-back.
- `crates/engine/tests/fixtures/team-bad-attribute.json`, `rules-unknown-version.json`: the two bad fixtures.
- `crates/engine/benches/tick_step.rs`: loads the shipped content and the default teams.
- `crates/engine-cli/src/cli.rs`: `--content-dir`, `--team-a`, `--team-b`, `Generate`; explicit `long_help` line breaks.
- `crates/engine-cli/src/main.rs`: `generate` dispatch, `with_ansi(is_terminal)`, the chain printed once.
- `crates/engine-cli/src/content.rs`: shared content and team loading.
- `crates/engine-cli/src/simulate.rs`: content, teams, identity, header schema 2, `stats.json`, `teams` and `content.hash` in the record.
- `crates/engine-cli/src/bench.rs`: content and teams loaded once before the warm-up; `owner.id` and `content.hash` in the report.
- `crates/engine-cli/src/generate.rs`: `generate` command.
- `crates/engine-cli/tests/cli_args.rs`: AC-f, missing content folder, help width.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/sim.rs`, `record.rs`, `observe/mod.rs`, `tuning.rs`, `player.rs`, `team.rs`, `error.rs`, `lib.rs`, and the command line: written by `engine-core`, extended here. No sibling implementation is in progress, so no conflict exists.

## Notes on Design Choices

- One loader for four kinds: `load_json` is generic over `DeserializeOwned + garde::Validate` and takes the context by reference, so `TeamFile` validates against the schema and the other three against `()`.
- The version peek is a two-field struct read before the full deserialize; a tagged enum could not phrase "newer than this build" (plan freshness research).
- `Attributes::from_entry` fills the array in schema order; `Derived::from_attributes` reads the six required indices. No `HashMap` exists in `crates/engine`; the only map is the `BTreeMap` inside a team file entry, read at load.
- `MatchConfig` owns the two `Team`s and 22 `Player`s; `Simulation::new` clones them. `bench` therefore builds one config and clones it per match, and the timed path reads no file.
- `content_hash` is SHA-256 over the content digest and the JSON of the two team files, so a generated team that was never written to disk hashes the same way as a loaded one.
- `owner.id` is a field on every record and `to_json` copies it into the envelope; the constant `OWNER_ID` is gone.
- The `teams` array in `match-stats` is emitted as an additive extra; the observability audit settles the vocabulary (plan R4, instrument note).
- Help text uses explicit `long_help` strings with line breaks because `clap` without the `wrap_help` feature joins doc-comment lines and does not wrap (`clap_builder-4.6.7/src/output/textwrap/mod.rs:25`).
- The `--json` help now states the rule the binary applies: the extension is replaced by `.jsonl` (probe finding 1).

## Verification Seams Built

- AC-a (shipped schema loads with 30 to 50 grouped names) → the shipped folder reached through `CARGO_MANIFEST_DIR` at `crates/engine/tests/common/mod.rs:14` and the loader at `crates/engine/src/data/mod.rs:115` (enables `cargo test -p engine --test content`).
- AC-b (attribute of 120 refused naming file, player, attribute; valid file loads) → the committed fixture `crates/engine/tests/fixtures/team-bad-attribute.json` reached through `fixture_path` at `crates/engine/tests/common/mod.rs:34`; the squad check at `crates/engine/src/data/team.rs:129`; the message asserted verbatim in `crates/engine/tests/team_files.rs` (enables `cargo test -p engine --test team_files`).
- AC-c (unknown rule-pack version refused naming the version; current version loads) → `crates/engine/tests/fixtures/rules-unknown-version.json` and the version peek at `crates/engine/src/data/mod.rs:138-154` (enables `cargo test -p engine --test content`).
- AC-d (20 clubs, 22 players each, legal positions, per-position means inside the tuning bounds) → `generate_league` at `crates/engine/src/data/generator.rs:13` on the shipped content; `Position` is an enum so an illegal code fails deserialization (enables `cargo test -p engine --test generator`).
- AC-e (owner and match identifiers round-trip) → `write_stats` and `read_stats` at `crates/engine/src/observe/mod.rs:178` and `:192`; `TickHeader` at `crates/engine/src/record.rs:80` with the identity bytes at offsets 28 and 44 (`record.rs:26-28`); a fixed test identity through `header()` at `crates/engine/tests/common/mod.rs:48` (enables `cargo test -p engine --test identity`).
- AC-f (`simulate --team-a --team-b` on generated output completes with the engine-core tick count) → `CARGO_BIN_EXE_engine-cli` with `SM_DATA_DIR` and `SM_CONTENT_DIR` set per test at `crates/engine-cli/tests/cli_args.rs:10-13`; `generate` writes to a temp folder and `simulate` reads it back (enables `cargo test -p engine-cli`).
- Benchmark compare (`05c-benchmark.md`) → the release binary loads content once before the warm-up at `crates/engine-cli/src/bench.rs:21-27`; the measurement of this build is filed under `implement-evidence/data-schemas-generator/` (`bench.stdout.txt`: `bench.match_wall_ms` 390, `bench.cpu_ms` 1922 over 5 matches, `bench.peak_mem_mb` 5.6875, `engine.ticks_per_s` 692,308, `budget.pass` true, exit 0, `content.hash` `c33b2b9b247e`; `criterion.txt`: `tick_step` 1.4410 µs, `steering_pass_22` 880.84 ns, "No change in performance detected").
- Instrument key check (`04b-instrument.md` §2) → `content.loaded` at `crates/engine/src/data/mod.rs:171`, `content.refused` at `:130` and `:142`, `generator.league` at `crates/engine/src/data/generator.rs:71`, `identity.owner_created` at `crates/engine/src/observe/identity.rs:61`, `tickfile.header` with `owner_id` and `match_id` at `crates/engine/src/record.rs:178`; `content.hash` and `teams` in the live record (`implement-evidence/data-schemas-generator/bench.stdout.txt`, `refused.stderr.txt`).
- Probe carry-overs → finding 1 at `crates/engine-cli/src/cli.rs:49`; finding 3 at `crates/engine-cli/src/main.rs:33` with `crates/engine/src/error.rs` (`implement-evidence/data-schemas-generator/error-chain.stderr.txt` shows one chain); finding 4 at `crates/engine-cli/src/main.rs:21` (`bench.stderr.txt` holds no escape byte); finding 5 asserted by `no_help_line_exceeds_eighty_columns` in `crates/engine-cli/tests/cli_args.rs` (longest line 78).

## Deviations from Plan

- Step 2 and step 3: `EngineError::Read { path, source }` was added for a file that cannot be read, and `Io`'s display dropped its inner text. With `#[from]` the source is also the chain's next link, so `{err:#}` printed the operating-system text twice (probe finding 3). The chain now prints once: `cannot create <path>: io error: The system cannot find the path specified. (os error 3)`.
- Step 3: `Content.hash()` covers the three content files; `MatchConfig.content_hash` chains that digest with the JSON of the two team files instead of hashing five raw files, so a generated team never written to disk hashes the same way.
- Step 4: `AttributeSchema::index` is a linear scan over at most 50 names at load time; no `BTreeMap` is built.
- Step 5: `GeneratorTuning::per_position` is `BTreeMap<String, GroupDist>` keyed by the position code, not `BTreeMap<Position, GroupDist>`. `garde` validates a map only when the key implements `PathComponentKind`, which exists for `String`, `&str`, `Cow<str>`, `CompactString`, and `usize` (source: `.scratch/sources/rust/garde-0.23.0/src/error.rs:138-142`). A custom rule checks that every one of the ten codes is present and no other key exists. `bench_positions` is a `Vec<Position>` whose length plus eleven must equal `squad_size`.
- Step 7: `club` and `players` carry `#[garde(dive(()))]` so the nested types keep the unit context while `TeamFile` carries `AttributeSchema` (source: `.scratch/sources/rust/garde_derive-0.23.0/src/emit.rs:434-441`, the `dive` branch with an explicit context expression).
- Step 8 and step 9: `Player::max_speed()` and `max_accel()` take no `Tuning`; `Team::players` is deleted in favour of `Team::from_file`, and `Team::new` builds the identity. Unit tests that need a bare team or a flat player use `team::test_support::bare` and `player::test_support::flat_player`, both `#[cfg(test)]`.
- Step 14: `read_ticks` returns `TickFile { header, records }`, not a tuple, so callers name the fields.
- Step 16: `MatchStats::outcome` is a `String` so the record deserializes; `owner.id` is a field on each record and the envelope copies it.
- Step 18: `bench` runs the two default clubs only; the plan named no team flags for it and none were added.
- Step 20: `clippy` under rust-version 1.87 flagged `self.tick % n == 0`; it is now `self.tick.is_multiple_of(n)`.

## Anything Deferred

- Probe finding 2 (no structured record on a failed run): not fixed here, as the plan recorded. `content.refused` covers the loader's failure path on stderr; `/wf observability init` settles the `outcome: error` record (U-2).
- `FatigueTuning` and `RulePack` are loaded and validated but not read by the simulation; `tactics-and-ai` and `match-rules` consume them.
- The starting eleven are the first eleven entries in file order; lineup selection belongs to `viewer-lineup-tactics`.
- Engine-core deferrals stand: the wall bounce (`crates/engine/src/sim.rs:221`) and the restart heuristic (`crates/engine/src/validate.rs:77`).

## Known Risks / Caveats

- Peak memory rose from 5.06 MB to 5.69 MB (12.5 percent, inside the 25 percent tripwire). The rise is the parsed content and the two 23 KB team files held for the run; the timed path allocates nothing new.
- `owner.id` is created under the real `%LOCALAPPDATA%\SoccerManager` on any machine that runs `simulate` or `bench` without `SM_DATA_DIR`; the tests set `SM_DATA_DIR` to a temp folder.
- A team file with more than eleven players starts the first eleven in file order, whatever their positions; nothing checks that slot 0 is a goalkeeper.
- Determinism holds on one machine and one build; a different platform may differ in transcendental results (unchanged from engine-core).

## Freshness Research

- Source: `.scratch/sources/rust/garde-0.23.0/src/error.rs:124-145` and `src/validate.rs:280-320`. Why it matters: whether a map keyed by an enum can be validated with `dive`. Takeaway: `PathComponentKind` is implemented for string types and `usize` only; the map is keyed by the code string.
- Source: `.scratch/sources/rust/garde_derive-0.23.0/src/emit.rs:426-452`. Why it matters: nesting a unit-context struct inside a struct with a typed context. Takeaway: `dive(<expr>)` passes `&<expr>` as the nested context; `dive(())` works.
- Source: `.scratch/sources/rust/garde-0.23.0/src/i18n.rs:315-329`. Why it matters: the exact refusal text. Takeaway: `length is lower than {min}`, `length is greater than {max}`, `lower than {min}`, `greater than {max}`.
- Source: `~/.cargo/registry/src/*/clap_builder-4.6.7/src/output/textwrap/mod.rs:25` and `src/builder/command.rs:1375-1443`. Why it matters: help lines over 80 columns (probe finding 5). Takeaway: without the `wrap_help` feature clap never wraps and joins doc-comment lines; explicit `long_help` strings with `\n` keep every line short.
- Source: `std::io::IsTerminal` (stable since Rust 1.70). Why it matters: colour on stderr only on a terminal (probe finding 4). Takeaway: `std::io::stderr().is_terminal()` feeds `with_ansi`.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine data-schemas-generator` — the slice has testable behaviour, a benchmark compare, and an instrument key check. Consider compacting the session before `/wf verify`; workflow state lives in artifact files on disk and the SessionStart hook re-reads it after compaction.
- **Option B:** `/wf review football-manager-match-engine data-schemas-generator` — not recommended; the slice is behaviour, and verify captures the compare evidence the ledger needs.
- **Option C:** `/wf plan football-manager-match-engine data-schemas-generator` — not needed; the eight deviations are recorded and none changed a contract or a product-owner answer.
- **Option D:** Blocked — not applicable; every gate passes.
