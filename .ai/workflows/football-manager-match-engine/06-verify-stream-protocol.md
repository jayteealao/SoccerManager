---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: stream-protocol
status: complete
stage-number: 6
created-at: "2026-09-22T13:35:00Z"
updated-at: "2026-09-22T13:35:00Z"
result: pass
metric-checks-run: 15
metric-checks-passed: 15
metric-acceptance-met: 6
metric-acceptance-total: 6
metric-acceptance-user-observable: 0
metric-acceptance-code-only: 6
metric-interactive-checks-run: 0
metric-interactive-checks-passed: 0
metric-issues-found: 0
metric-issues-found-initial: 2
metric-issues-found-final: 0
fix-rounds-run: 1
convergence: converged
verify-owned-fix-commit: "c7f7357"
regression-tests-added: 1
constraint-resolution-missing: []
interactive-verification: not-applicable
adapters-used: [cli]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/stream-protocol/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "+37.9"
ac-staleness-checked: false
ac-stale-count: 0
longitudinal-baseline-compared: true
stability-check-flaky-count: 1
adversarial-tests-run: 22
adversarial-tests-failed: 0
failure-mode-probes-run: 4
cross-browser-delta: "none"
web-vitals-lcp-ms: null
web-vitals-cls: null
web-vitals-inp-ms: null
stack-source: confirmed
debt-markers-found: 0
debt-markers-malformed: 0
debt-markers-unrecorded: 0
skipped-gating-specs: []
consult-runs: []
tags: [engine, stream, websocket, protocol, fixture, rust, benchmark]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-stream-protocol.md
  plan: 04-plan-stream-protocol.md
  implement: 05-implement-stream-protocol.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  review: 07-review-stream-protocol.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine stream-protocol"
---

# Verify: Stream Protocol and Fixture Harness

## The Verification

Implement handed over two commits, 47 files, and two new crates: `protocol`, which holds the message set and the codec and performs no input and no output, and `stream`, which adds the socket, the recorder, and the replayer. The slice carries six acceptance criteria. Every one is annotated `observable: false` and names a cargo test, so the runtime gate did not apply and no sub-agent ran. Cargo serialises on the target folder and the benchmark needs an uncontended processor, so the coordinator ran every check in sequence and drove the release binary directly for the augmentation evidence and the probes.

Fifteen checks ran. Thirteen passed on the first drive. Two issues entered the fix loop, and both are now fixed and re-checked. The first was a flaky test: the full-workspace suite failed once at `crates/stream/tests/backpressure.rs:43` with "the buffer reached 66 against a bound of 64", while five isolated drives passed. The gauge counts `entered - left`, so it reads up to two over the bound, and the live serve run reached `high_water` 501 against a bound of 500 on an idle machine. The channel is the bound the criterion names, and `sync_channel` enforces it. The second came from a failure-mode probe: a viewer that closed its page in mid-match made `serve` print "error: the viewer disconnected" and exit 1, and `socket.session_closed` was never logged. The user triaged both `Fix`. One commit, `c7f7357`, loosened the two gauge assertions to `bound + 2` and gave the closing messages the same clean-end rule the tick path already had, with one regression test that failed before the patch and passes after.

All eight designed signals fired on live records, the fixture round trip hashes to `ce483d844c17` in both directions, and the benchmark compare holds inside every tripwire: 389 milliseconds against 389, 381 milliseconds of processor time against 384, 5.97 megabytes against a 7.03 tripwire, and 1.4376 microseconds per tick step against 1.4342. The fifth target has its first value: 640,213 ticks per second delivered over the socket, 4.7 times the budget, and the socket costs 0.8 percent of engine throughput. Review is next. The top open risk moves from throughput, which is settled, to the change queue: a rejected change writes no `change.kind`, because the kind the client sent is not a known kind, and the instrument plan lists that key on every change row.

## Verification Summary

- Result: `pass`. Every criterion met by the test suite; no criterion is user-observable, so the runtime gate is `not-applicable`.
- Convergence: `converged`. Two issues entered the fix loop (TEST-FLAKE-1, ADV-1), two `Fix` decisions, one patch accepted, one re-check passed, one regression test added; `fix-rounds-run: 1`.
- Adapter: `cli`, driven directly (`cli-direct`) on the reference machine for augmentation evidence, adversarial probes, and the benchmark compare. A scratch client built against the `stream` crate drove the socket surface that no shipped command reaches.
- Sub-agents: none. One cargo command covers every criterion, the benchmark needs the processor alone, and the two fixes are six files in one commit.
- Benchmark compare mode: recorded in `05c-benchmark.md`; four targets compared, zero regressions, zero improvements, zero tripwires; one new target measured for the first time.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (exit 0; `check-fmt.exit-code`, `recheck-fmt.txt`).
- `cargo check --workspace --all-targets`: pass (exit 0; `check-types.exit-code`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass (exit 0, 0 diagnostics; `check-clippy.stderr.txt`, `recheck-clippy.txt`).
- `cargo build --release` from a clean of the four workspace crates: pass (exit 0; 5,456 ms wall, 0 warnings; binary 3,280,896 bytes; `check-build-release.txt`).
- `cargo test --workspace` (debug profile): fail on the first drive, pass on three drives after the fix (122 passed, 0 failed, 0 ignored, 22 binaries; 24,902 to 30,354 ms wall; `check-test.stdout.txt`, `recheck-test-{1,2,3}.txt`). Breakdown: `engine` 48 unit plus 16 integration; `engine-cli` 6 in `cli_args` and 5 in `stream_cli`; `protocol` 20 unit plus 2 in `document`; `stream` 16 unit plus 9 integration. The first drive stopped after 15 binaries, so its tally of 112 is short of the suite.
- Stability check: five isolated drives of `cargo test -p stream --test backpressure` all passed while the full-workspace drive failed once (`check-backpressure-drives.txt`). `stability-check-flaky-count: 1`; see Verify-Owned Fixes.
- `gitleaks detect --log-opts="333ed06..HEAD"`: pass (exit 0, "no leaks found", 3 commits, 208.62 KB; `check-gitleaks.txt`).
- Secret pattern grep over `git diff 333ed06...HEAD` (api key, secret, password, token, credential assignments): pass (0 matches; `check-secret-grep.txt`).
- Dependency advisory check: pass. `cargo-audit` is not installed; the eleven crates the slice added to the lockfile were checked one by one against the RustSec advisory database over the GitHub contents API, calibrated against a known hit. Eight have no advisory folder. Three carry four advisories between them, and the pinned version is patched in every case: `bytes` 1.12.1 against RUSTSEC-2026-0007 (patched `>= 1.11.1`), `http` 1.5.0 against RUSTSEC-2019-0033 and RUSTSEC-2019-0034 (both patched `>= 0.1.20`), and `tungstenite` 0.30.0 against RUSTSEC-2023-0065 (patched `>= 0.20.1`). `check-rustsec-packages.txt`.
- `sdlc-debt:` marker hygiene over the slice diff: pass. 0 markers added, 0 malformed, 0 unrecorded, 1 retired (the validator's restart guess, replaced by the restart flag). One marker remains in the tree, `sim.rs:272`, and it names `match-rules`. `check-debt-markers.txt`.
- Instrumentation key check (`04b-instrument.md` §2 against live records): pass with one note. 8 planned signals, 8 present, 0 absent, 2 additive signals. `instrument-key-check.txt`; details under Augmentation Verification.
- `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` three times without a reset: pass (exit 0 x 3; 389, 390, 389 ms; `bench-drive-{1,2,3}.stdout.txt`).
- `cargo bench -p engine --bench tick_step`: pass (exit 0; `tick_step` 1.4376 µs, `steering_pass_22` 888.96 ns; `check-criterion.stdout.txt`).
- `target/release/engine-cli.exe bench --seed 42 --matches 1 --stream --json` three times: pass (exit 0 x 3; 640,213, 634,857, 642,476 ticks per second delivered; `bench-stream-{1,2,3}.stdout.txt`).
- Cold start `engine-cli --version` five times: pass (37 to 41 ms; data-schemas-generator measured 36 to 37 ms; `perf-cold-start-ms.txt`).
- Cross-slice regression over `engine-core` and `data-schemas-generator`: pass (`check-cross-slice.txt`); details under Cross-Slice Regression.

## Interactive Verification Results

Automated only. Every criterion in `03-slice-stream-protocol.md` is annotated `observable: false` and names a cargo test, and the plan's `## Verification Strategy` records no environment wall. The release binary and a scratch client were driven for augmentation evidence and probes, not for a criterion.

## Acceptance Criteria Status

- **AC-a** "Given the engine simulates with the socket server on, When a client connects, Then it receives a hello with engine version, owner identifier, and match identifier before the first tick frame." kind: `code-only` (annotated `observable: false`). status: met. method: automated. evidence: the three tests in `crates/stream/tests/hello.rs` passed; the live client's first message is the hello, carrying `engine.version` `0.1.0`, `owner.id` `ca090d302c9c357be8f58dad322ded79`, and `match.id` `000000000000002a-1790081643845`, before any tick frame (`live-client-messages.txt:1`). evidence-rung: n-a.
- **AC-b** "Given a client reads 8 times faster than real time, When the engine streams a full match, Then every one of the 270,000 ticks arrives in order with no gap, and the server's buffer never exceeds its bound." kind: `code-only`. status: met. method: automated. evidence: `a_fast_client_receives_every_tick_in_order` passed on three consecutive full-workspace drives; `bench --stream` delivered every tick of three 270,000-tick matches at 640,213 ticks per second median (`bench-stream-{1,2,3}.stdout.txt`). The bound is `sync_channel(buffer_ticks)`, which holds at most `bound` frames; the gauge reads up to two higher during the hand-off, and the assertion now names that window. evidence-rung: n-a.
- **AC-c** "Given a client reads slower than the engine produces, Then the server pauses production at the buffer bound and resumes when the client drains; no tick is dropped." kind: `code-only`. status: met. method: automated. evidence: `a_slow_client_pauses_the_producer_and_loses_no_tick` passed; the live 15-minute run against a throttled client paused 18 times for 2,477 milliseconds in total and delivered all 45,000 ticks (`live-serve.stderr.txt`, `live-client-messages.txt`). evidence-rung: n-a.
- **AC-d** "Given `engine-cli record --seed 7 --out fixture.bin`, Then the fixture holds the full stream, and Given the mock server replays it, Then a client receives a byte-identical sequence." kind: `code-only`. status: met. method: automated. evidence: the two tests in `crates/stream/tests/fixture.rs` and `record_writes_a_fixture_and_prints_its_counts` passed; the live round trip recorded 6,003 frames over 6,000 ticks and hashed to `ce483d844c17`, and the replay reported the same hash and served all 6,000 ticks to a client (`live-record.stderr.txt`, `live-replay.stderr.txt`). evidence-rung: n-a.
- **AC-e** "Given a `queue change` command with an unknown change type, Then the server answers with a rejection naming the type; and Given a valid command, Then it answers with an acknowledgement carrying a queue identifier." kind: `code-only`. status: met. method: automated. evidence: the two tests in `crates/stream/tests/commands.rs` and `a_queued_change_writes_a_match_event_row` passed; the live client received `{"type":"ack","command":"queue-change","change.queue_id":"q-33-0",...}` for `tactics` and `{"type":"reject","command":"queue-change","reason":"unknown change type formation"}` for `formation` (`live-client-messages.txt`), and both verdicts are on the record in `events.jsonl` (`live-events.jsonl`). evidence-rung: n-a.
- **AC-f** "Given the protocol reference document, Then every message in the implementation appears in the document with its fields." kind: `code-only`. status: met. method: automated. evidence: the two tests in `crates/protocol/tests/document.rs` passed; they read `docs/reference/protocol.md` and require a `### <name>` heading for each of the ten `MESSAGES` entries and a table row for each field. The completeness test `messages_names_every_variant_and_nothing_else` matches exhaustively on both message enumerations, so a new variant cannot compile until `MESSAGES` and the document both name it. evidence-rung: n-a.

evidence: n-a 6. `metric-acceptance-mock-rung`: 0.

## Issues Found

None open. Two issues entered the fix loop, TEST-FLAKE-1 and ADV-1, and both were triaged `Fix`, patched, re-checked, and committed; see Verify-Owned Fixes.

## Verify-Owned Fixes

| ID | Type | Triage | Outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| TEST-FLAKE-1 | check-failure (`cargo test --workspace` failed once at `crates/stream/tests/backpressure.rs:43`: "the buffer reached 66 against a bound of 64"; five isolated drives passed) | Fix | Patched. Both gauge assertions read `bound + 2`, and the comment names the two-item hand-off window: one frame sits at a blocked sender, which counts itself in before it tries to send, and one frame is received but not yet counted out. No engine code changed. | The two existing tests are the regression test; the assertion now states what the gauge can guarantee | Pass (`cargo test --workspace` exit 0 on three consecutive drives, 122 of 122) |
| ADV-1 | check-failure (failure-mode probe FM-1: a viewer that closes its page in mid-match made `serve` print "error: the viewer disconnected" and exit 1, and `socket.session_closed` was never logged) | Fix | Patched. `closing_or_fail` gives the full-time event and the closing statistics the same clean-end rule the tick path already had, and `peer_gone` classifies a reset, an abort, a broken pipe, and a missing closing handshake as a peer that is gone, because a page that is killed sends no close frame. The run now exits 2 for a short run and logs `socket.client_gone` and `socket.session_closed`. | `crates/engine-cli/tests/stream_cli.rs::a_viewer_that_closes_in_mid_match_ends_the_run_without_an_error` (failed before the patch with `left: Some(1) right: Some(2)`; passes after) | Pass (`cargo test -p engine-cli --test stream_cli` 5 of 5; the live FM-1 probe now reports `serve exit=2`, `socket.client_gone written=4543`, and `socket.session_closed ticks=4543`; `adversarial-client.txt`) |

The gate question fired through rung 1 (`AskUserQuestion`), batched, one question per issue; the user chose `Fix` for both. Both patches landed in one commit, because they share the same surface and the same re-check.

Commit: `c7f7357`
Regression tests added: 1

## Augmentation Verification

- **instrument** (`04b-instrument.md`, status ready): pass with one note. All eight designed signals fired on live records from the release binary. `socket.listening` carried `port`, `protocol_version`, and `match.id`. `socket.client` carried `origin`, `protocol_version`, and `match.id`. `socket.refused` fired twice, once for a foreign Origin ("origin https://evil.example is not on the loopback allowlist") and once for a wrong version ("protocol version 99; this build speaks 1"), and a page with the wrong origin did not end the match for the right one. `socket.backpressure` fired 18 times in one 15-minute run, once per pause, with `tick`, `bound`, `paused_ms`, and `resumes`. `match-event.engine` opened the fourth record kind with all ten planned keys plus the five Block A envelope keys. `fixture.recorded` and `fixture.replayed` both carried the same 12-hex hash `ce483d844c17`, which proves the round trip. Types hold: `port` is an integer, `paused_ms` and `resumes` are integers, `speed` is `8.0`, and the hash is 12 lower-case hex characters. Two additive signals ship beyond the plan: `socket.session_closed` and `socket.client_gone`. Key drift, LOW, informational: `match-event.change` writes `change.kind` on a queued row and omits it on a rejected row, because the kind the client sent is not a known kind; the reason string names it verbatim. Review reconciles the plan text or the key. `instrument-key-check.txt`.
- **benchmark** (`05c-benchmark.md`, mode baseline to complete): pass. Compare mode ran the three recorded commands with the shipped defaults. Full match wall time 389 ms against 389 (0.0 percent); ticks per second 694,087 against 694,087 (0.0 percent); processor time 381 ms per match against 384 (-0.8 percent, the 422 ms tripwire not reached); peak memory 5.97 MB against 5.62 (+6.3 percent, the 7.03 MB tripwire not reached); tick step 1.4376 µs against 1.4342 (+0.24 percent, and the two criterion intervals overlap, so the change is not resolvable); `steering_pass_22` 888.96 ns against 887.68. Regressions: none. Improvements: none. Tripwires: none. The fifth target has its first value: 640,213 ticks per second delivered over the socket, against a 135,000 budget, with 0 or 1 pause per match. The plan's top risk, a copy on the hot path, is not reproduced: the encoder writes into a reused 99-byte array inside `TickFrame`, and 270,000 ticks cost 1.69 MB of extra peak memory against the same run without a socket, which is the 500-frame buffer, the client, and the tungstenite write buffer, not a per-tick allocation. `benchmark-compare.txt`.
- **experiment**: deferred to the `experiment-flags` slice per the index; no `04c-experiment.md` exists, so no re-check applies.
- Mock fidelity inventory: `02c-craft.md` does not exist; no inventory applies. Design findings: no `07-design-audit.md` or `07-design-critique.md` exists.

## Security Scan

- CVE scan: `cargo-audit` is not installed; manual check of the eleven added crates against the RustSec advisory database. New critical or high advisories: 0. Four advisories exist across `bytes`, `http`, and `tungstenite`, and every pinned version is above its patched bound. The slice binds `127.0.0.1:0` only, refuses any Origin outside `null`, `file://`, `localhost`, and `127.0.0.1`, and refuses any protocol version but 1.
- Secret detection: `gitleaks` over `333ed06..HEAD`, 3 commits, no leaks found. Pattern grep over the diff: no finding.
- SAST: `semgrep` is not installed; skipped. Clippy at `-D warnings` is the static floor and is clean.
- `trufflehog` is not installed; `gitleaks` covers the same range.

## Accessibility Gate

Not automatable: the slice ships a headless command line, a local socket, and a binary fixture format, and no user interface. New WCAG AA violations: 0 (no surface). The viewer slices own the accessibility gate.

## Performance Gate

- Bundle size: `engine-cli.exe` 3,280,896 bytes against 2,379,264 in data-schemas-generator, a 37.9 percent increase. Cause: two new crates, `tungstenite` with its `sha1` and `http` dependencies, and the handshake path. The base branch `main` holds no code, so no delta against `main` exists.
- Build time: 5,456 ms for the four workspace crates from clean against 4,686 ms for two crates in data-schemas-generator, a 16.4 percent increase, under the 30 percent WARN line.
- Cold start: 37 to 41 ms for `--version` over five runs against 36 to 37 ms in data-schemas-generator. The increase is one to four milliseconds, inside the run-to-run spread; the linked socket library is not loaded until a command needs it.
- The `benchmark` augmentation carries the engine numbers (see Augmentation Verification).

## Cross-Slice Regression

Siblings checked: `engine-core` (`result: pass`) and `data-schemas-generator` (`result: pass`). Both overlap this slice's `files-modified`, so the regression target is the whole suite. `cargo test --workspace` passed all 122 tests on three consecutive drives, including `two_runs_are_byte_identical`, `ninety_minutes_write_270_000_records`, `a_seeded_full_match_has_no_violations`, `the_same_seed_gives_the_same_league`, and the six `cli_args` tests.

Live confirmation on the release binary (`check-cross-slice.txt`):

- Two `simulate --seed 42` runs wrote 51,840,080 bytes each. Every byte from offset 64 to the end is identical, 51,840,016 bytes, records and trailer alike. The two runs differ in the header only, at the 18 bytes that hold `owner.id` and the first two bytes of `match.id`; the two runs used two different data folders, so each created its own owner identifier. The validator reported 0 violations on both.
- The tick file header now reads schema version 3. The restart flag rides in bit 31 of the stored tick number, so the record stays 192 bytes and the reader refuses version 2 under the header's own fail-closed rule.
- Two `generate --seed 9 --clubs 20` runs wrote 20 files each, identical under `diff -r`, with the same league hash `9250735be56b`.

`cross-slice-regressions-found: 0`.

The engine-core AC-b wording item that data-schemas-generator raised is unchanged and still open for review: the criterion reads "the two tick outputs are byte-identical" over the whole file, and the header carries per-run identifiers.

## Longitudinal Delta

Baseline source: prior evidence run `verify-evidence/data-schemas-generator/` (`simulate-42.stdout.txt`) and the command help tree. Surface: the `simulate --seed 42` record and the command list.

- Record (`longitudinal-simulate-42.txt`): every match fact is unchanged. `goals`, `possession.changes`, `ball.idle_ticks`, `ball.max_speed`, `validate.violations`, and `ticks.written` are equal to the prior run. Five keys differ and every one is expected: `build.hash` `4a806c8-dirty` to `65a3414-dirty`, `content.hash` `c33b2b9b247e` to `02d33ad91de5` (the tuning file gained the `stream` block), `match.id`, `owner.id` (a fresh data folder), and `duration_ms` 400 to 401 with `engine.ticks_per_s` 673,360 to 671,733. Interpretation: the slice changed no simulation behaviour.
- Help tree: added `serve`, `record`, and `replay`, and `--stream` on `bench`. No command and no flag removed. Every help line of the four streaming commands is at most 80 columns, held by `no_help_line_of_the_streaming_commands_exceeds_eighty_columns`.

## Friction Notes

- A rejected change writes no `change.kind` row, because `ChangeKind::parse` returns nothing for an unknown kind. The reason string carries the kind verbatim ("unknown change type formation"), so no fact is lost, but a reader that groups rows by `change.kind` sees only accepted ones. Class: `key-drift`. informational, LOW.
- `--team-a` with a missing file is refused as "cannot read nope.json", which names the file but not the folder the run looked in. Class: `ambiguous-copy`. informational, LOW.
- `record --out` creates missing parent folders and exits 0. Nothing is silently relocated: the fixture lands exactly at the path asked for. `generate --out` behaves the same way and says so in its help. informational.
- A second client that connects while one is being served is refused by the operating system, because `Server::accept` consumes the listener and the port file is removed. The message is "No connection could be made because the target machine actively refused it. (os error 10061)". One match serves one viewer by design; the page has no way to tell that refusal from an engine that has not started. informational.
- The build hash carries `-dirty` during every workflow run because the cost ledger is hook-appended each turn; unchanged from the prior two slices.

## Free Exploration Notes

- The socket costs 0.8 percent of engine throughput: `engine.ticks_per_s` falls from 694,087 without the socket to 688,775 with it, on the same seed and the same content hash. informational.
- `socket.session_closed` reports `high_water` 501 against a bound of 500 on an idle machine and on every drive. The one-frame overshoot is the hand-off, not the channel. informational; it is the fact that TEST-FLAKE-1 turns on.
- A 15-minute match against a throttled client paused 18 times and spent 2,477 milliseconds waiting, and delivered every one of the 45,000 ticks in order. informational.
- `set-speed` clamps to the viewer's range: 1e9 and `f32::MAX` both return 8.0, and -5 and 0 both return 0.25. The acknowledgement echoes the clamped value, so the page can show what the engine actually did. informational.
- A one-megabyte `detail` payload on `queue-change` is accepted and acknowledged with a queue identifier. No size rule exists yet; the change queue slices own one if they want it. informational.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| `replay --fixture` missing file (`adv-replay-missing`) | pass | "cannot replay <path>: cannot read <path>: The system cannot find the file specified. (os error 2)", exit 1 |
| `replay` on 400 random bytes (`adv-replay-random`) | pass | "fixture format error: bad magic", exit 1 |
| `replay` on a truncated fixture (`adv-replay-truncated`) | pass | "missing trailer: the fixture is incomplete", exit 1 |
| `replay` on a fixture with one flipped payload byte (`adv-replay-flipped-byte`) | pass | "the frame bytes do not match the trailer hash", exit 1 |
| `replay` on a fixture stamped version 99 (`adv-replay-bad-version`) | pass | "protocol version 99; this build speaks 1", exit 1 |
| `replay --fixture` pointed at a folder (`adv-replay-fixture-is-dir`) | pass | "cannot read <path>: Access is denied. (os error 5)", exit 1 |
| `record --out` into a missing folder (`adv-record-missing-dir`) | pass | creates the folders and writes the fixture, exit 0 (informational) |
| `record --out` pointed at a folder (`adv-record-out-is-dir`) | pass | "cannot create <path>: Access is denied. (os error 5)", exit 1 |
| `record --minutes 0` (`adv-record-minutes-zero`) | pass | "invalid configuration: minutes must be 1 to 200, got 0", exit 1 |
| `serve --minutes 0` (`adv-serve-minutes-zero`) | pass | same message, exit 1 |
| `record --minutes 4294967295` (`adv-record-huge-minutes`) | pass | same message naming the value, exit 1 |
| `serve --content-dir` missing folder (`adv-serve-missing-content`) | pass | "cannot read content folder (tried <path>): no folder holds attributes.json", exit 1 |
| `serve --team-a` missing file (`adv-serve-bad-team`) | pass | "cannot read nope.json: … (os error 2)", exit 1 (copy note above) |
| `serve` without `--seed` (`adv-serve-no-seed`) | pass | clap: "the following required arguments were not provided: --seed <SEED>", exit 2 |
| Handshake with `Origin: https://evil.example` | pass | 403; `socket.refused` names the origin and the rule; the match stayed open for the next client |
| Handshake with `?v=99` | pass | 400; `socket.refused` names both versions |
| `set-speed 1e9` | pass | acknowledged as 8.0 |
| `set-speed -5` | pass | acknowledged as 0.25 |
| `set-speed 0` | pass | acknowledged as 0.25 |
| `set-speed f32::MAX` | pass | acknowledged as 8.0 |
| `queue-change` with an empty kind | pass | rejected: "unknown change type ", and the rejection is on the record |
| `queue-change` with a one-megabyte detail | pass | acknowledged with `q-493-0`; no crash, no truncation |

22 probes, 0 crashes, 0 unhandled errors, 0 failures. Every refusal names the file, the field, or the rule.

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Viewer closes its page in mid-match (`FM-1`) | fail to pass | before the fix: "error: the viewer disconnected", exit 1, no `socket.session_closed`; after: exit 2, `socket.client_gone written=4543`, `socket.session_closed ticks=4543`; ADV-1 |
| Wild command values and a one-megabyte payload (`FM-2`) | pass | every value clamped or refused; the match ran to full time, exit 0; two change rows on the record |
| A second client while one is connected (`FM-3`) | pass | the operating system refuses the connection; the served match is unaffected and exits 0 |
| Port file lifecycle (`FM-4`) | pass | `engine.port` holds the printed port while the run listens and is absent after the run exits, so no page reaches an engine that is gone |

## Gaps / Unverified Areas

- No criterion is user-observable, so no interactive evidence exists; the slice ships no user interface. The viewer slices carry the rendering gate.
- A missing Origin header, which the plan writes as the literal `none`, is covered by the unit tests in `server.rs` only. Both live refusals carried a header.
- The protocol is verified against one client, the scratch client built on the `stream` crate. No browser has opened the socket yet; `viewer-pitch` is the first.
- Determinism across machines and across builds is not measured (unchanged from engine-core).
- The change queue admits and records a change; nothing applies one yet. `change.state` `Applies now` and `Applied` have no producer, by the slice's own scope.

## Freshness Research

Not run: no test failed for an external reason, the plan is under a day old, and the two facts the slice takes from a third-party library (the `assert_valid` rule on `WebSocketConfig` and the shape of `accept_hdr_with_config`) were read from the installed tungstenite 0.30.0 source during implement. `ac-staleness-checked: false`.

## Recommendation

Proceed to review. The slice passes every check and every criterion, the fix loop converged in one round with its regression test, the benchmark compare holds inside every tripwire, and the new throughput target sits 4.7 times above its budget. Review should settle three items: the missing `change.kind` on a rejected change row against the instrument plan, the engine-core AC-b wording that data-schemas-generator raised and this slice leaves unchanged, and whether a second client deserves a spoken refusal rather than the operating system's.

## Recommended Next Stage

- **Option A (recommended): `/wf review football-manager-match-engine stream-protocol`** — `result: pass`, `convergence: converged`. Compact recommended before review: this verify carried test output, live socket traffic, probe output, and a fix round.
- **Option D: `/wf handoff football-manager-match-engine stream-protocol`** — valid on `result: pass` for a solo project; not recommended because `review-scope: slug-wide` accumulates one ledger and two slice reviews are still pending on it.
- **Option G: `/wf probe football-manager-match-engine`** — a slug-wide runtime sweep after review, once a viewer exists to open the socket.
