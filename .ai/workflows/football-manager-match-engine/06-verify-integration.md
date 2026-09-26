---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: integration
status: complete
stage-number: 6
created-at: "2026-09-23T16:07:18Z"
updated-at: "2026-09-23T16:07:18Z"
result: pass
metric-checks-run: 12
metric-checks-passed: 12
metric-acceptance-met: 6
metric-acceptance-total: 6
metric-acceptance-user-observable: 4
metric-acceptance-code-only: 2
metric-interactive-checks-run: 4
metric-interactive-checks-passed: 4
metric-issues-found: 0
metric-issues-found-initial: 1
metric-issues-found-final: 0
fix-rounds-run: 1
convergence: converged
verify-owned-fix-commit: "f6f36b6"
regression-tests-added: 1
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [web, cli]
bootstrap-failures:
  - {adapter: web, step: "in-app browser pane drive (Claude_Browser)", remediation: "The pane is hidden, so the page draws only when a screenshot is taken and the served engine, which the page paces, stops: the header clock read 00:03 before and after a 10 s wait at 8x, and events.jsonl held 1 row (in-app-pane-drive.txt). Steps 1 to 4 were driven in the pane; every step was driven to the end in headless Chromium (the browser suite and tutorial-walk.mjs), the next rung in the plan's fallback chain."}
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/integration/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: pass
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped — the page is served unbundled; no bundler exists"
ac-staleness-checked: true
ac-stale-count: 0
longitudinal-baseline-compared: true
stability-check-flaky-count: 0
adversarial-tests-run: 0
adversarial-tests-failed: 0
failure-mode-probes-run: 0
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
tags: [integration, e2e, playwright, docs, tutorial, benchmark, observability, license-audit, second-yellow]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-integration.md
  plan: 04-plan-integration.md
  implement: 05-implement-integration.md
  review: 07-review-integration.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine integration"
---

# Verify: Integration and Charter Scenario

## The Verification

Implement left the browser suite, the document set, the license audit, and the `serve` statistics fix on `e71ebfd`, with 21 browser tests, 310 Rust tests, and 126 page tests green. It named one open risk: three behaviours stay human checks, and the tutorial had not yet been followed from a new clone. This run re-ran every check on `b64dc0e`. The browser suite passed 21 of 21 in 24.4 minutes, including the twelve-step whole match against the live release engine. A new clone was then built and the tutorial followed from the top: steps 1 to 5 in the in-app pane, and steps 3 to 9 in headless Chromium with `tutorial-walk.mjs`. Every step did what the document says, through to the saved `touchline-<match id>.smfx`, and the engine then ended with exit 0.

The half-time screenshot of the whole-match run showed one defect. Halan Morez was booked twice at 3' and was still on the pitch at 45+4'. The cause is in `show_card`. A caution held back for advantage was judged at the foul, so a player booked in between got it as a first yellow. The single fix round, triaged Fix by the autonomous policy, adds a failing unit test, patches `crates/engine/src/rules/mod.rs`, and commits `f6f36b6`. The re-checks all pass. The Rust suite passes 311 tests, and the slow release tests, including the 1000-match realism bands, pass. The browser suite passes 21 of 21 again. Its full-time report now shows "a second booking means a sending-off" at 3'. The benchmark on the fixed engine gives 418 to 421 ms per match and about 422.5 s for 1000 matches, against 2 s and 30 minutes.

All 6 criteria are met, so this slice can go to review. The top open risk is not in this slice. With the player now sent off at 3', the same whole-match run ended 14–0, with 54 offsides for the ten-man side. The realism bands still hold, so this is recorded as an out-of-scope observation for the engine and not as a failure of any criterion here. One more item stays open. The `viewer-match-day` legibility deferral names this slice as `needed-by`, and no person has recorded that reading yet.

## Verification Summary

| Area | Result |
|---|---|
| Static checks (fmt, clippy `-D warnings`) | pass, before and after the fix |
| Rust tests | 310 passed / 4 ignored before; 311 passed / 4 ignored after (1 new regression test) |
| Page unit tests | 126 / 126 |
| Browser suite (headless Chromium 1243, 1280 by 800) | 21 / 21 before (24.4 min) and 21 / 21 after the fix (25.1 min) |
| Tutorial from a new clone | 10 / 10 recorded steps PASS (`tutorial/tutorial-walk.log`) |
| Benchmark | one match 418–421 ms; 1000 matches about 422.5 s |
| License audit | no GPL or AGPL dependency |
| Fix loop | 1 issue found, 1 fixed, 0 remaining; commit `f6f36b6` |

## Automated Checks Run

- `cargo fmt --all --check`: pass (exit 0; `fmt.txt`, `fmt-after-fix.txt`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass (exit 0; `clippy.txt`, `clippy-after-fix.txt`).
- `cargo build --release -p engine-cli`: pass (`build-release.txt`).
- `cargo test --workspace`: pass. 310 passed, 0 failed, 4 ignored (`cargo-test.txt`). After the fix: 311 passed, 0 failed, 4 ignored (`cargo-test-after-fix.txt`).
- `cargo test --test licenses` (inside the workspace run): pass. `no_dependency_is_gpl_or_agpl` and `expressions_are_judged_by_their_best_alternative`.
- `cargo test --test docs` (inside the workspace run): pass. `every_document_in_the_set_exists`, `the_reference_names_every_flag_of_every_command`, `every_command_in_the_guides_is_a_real_command`.
- `node --test "web/tests/*.test.mjs"`: pass, 126 / 126 (`web-unit.txt`).
- `npx playwright test` in `e2e/`: pass, 21 / 21 (`e2e-full-run.txt`, report in `playwright-report/`). After the fix: 21 / 21 (`e2e-full-run-after-fix.txt`, `playwright-report-after-fix/`).
- `engine-cli bench --seed 42 --matches 5 --json` three times: pass. Median wall 420 / 421 / 418 ms per match, `budget.pass: true` (`bench-1.json` to `bench-3.json`).
- `engine-cli bench --seed 42 --matches 1000 --json`: pass. `bench.cpu_ms` 419172, `cpu_wall_ratio` 0.99222, so the wall time is about 422.5 s (`thousand.json`, `thousand.wall.txt`).
- Secret scan of the slice diff (`git diff db33cd3..e71ebfd`, key, secret, password, and token assignments): pass. The only hits are the license parser's `token` variables.
- `npm audit` in `e2e/`: pass, "found 0 vulnerabilities".
- Re-check after the fix: `cargo test -p engine --lib rules::tests` failed before the patch (`left: [Some(Yellow), Some(Yellow)]`, `fix-card-test-before.txt`) and passed after it (`fix-card-test-after.txt`). `cargo test --release -p engine -- --ignored`: 3 / 3 (`engine-ignored-release-after-fix.txt`). `cargo test --release -p engine-cli --test calibrate -- --ignored`: `a_thousand_matches_hold_the_realism_bands` ok in 159.5 s (`realism-bands-after-fix.txt`).

## Interactive Verification Results

**Criterion:** Charter scenario, all 12 steps with their checkpoints against the live engine.
- **Platform & tool:** web. In-app pane (Claude_Browser) for steps 1 to 4, then Playwright 1.63.0 headless Chromium for all 12 steps.
- **Steps performed:** In the pane, a fresh clone's release `serve --seed 42` with the seed-2026 league: the header was read, slot 4 was emptied and refilled, mentality and pressing were changed, and the match kicked off. The pane stopped drawing at 8x (see `bootstrap-failures`). Then `e2e/tests/first-match.spec.mjs` ran all 12 steps against the release engine, before and after the fix.
- **Evidence:** `verify-evidence/integration/in-app-pane-drive.txt`; `e2e-full-run.txt` and `e2e-full-run-after-fix.txt` (test 1 ok, 13.2 min and 13.9 min); the step screenshots in `playwright-report/data/` and `playwright-report-after-fix/data/`.
- **Observation:** Pane: "Engine connected · v0.1.0", "Ten starters; a match needs eleven." with Kick off disabled, then "The lineup is ready.", mentality 3, and the phase live. Suite: all 12 `test.step` blocks passed both times. The half-time screenshot of the first run showed two 3' yellow cards for one player who was not sent off; this is issue DEF-1, which the fix loop fixed. The final screenshot after the fix shows the sending-off.
- **Result:** pass.

**Criterion:** Benchmark: one match on one thread under 2 s; 1000 matches under 30 minutes.
- **Platform & tool:** cli, `target/release/engine-cli.exe bench`.
- **Steps performed:** Three 5-match runs and one 1000-match run on the fixed engine.
- **Evidence:** `bench-1.json`, `bench-2.json`, `bench-3.json`, `thousand.json`, `thousand.wall.txt`.
- **Observation:** Wall time per match 420, 421, and 418 ms. 1000 matches used 419.2 s of processor time at a processor-to-wall ratio of 0.992, which is about 422.5 s of wall time. The file times bound the run under 435 s. Peak memory is 6.68 MB.
- **Result:** pass.

**Criterion:** Records of a browser-driven match reach the observability sink within the stated latency.
- **Platform & tool:** web and file sink. Playwright headless plus a read of `SM_DATA_DIR`.
- **Steps performed:** `e2e/tests/observability.spec.mjs` (event rows by the first stoppage; `stats.json` within 2 s of full time; record kinds, required keys, and schema version). The tutorial walk's data folder was also read after full time.
- **Evidence:** `e2e-full-run.txt` and `e2e-full-run-after-fix.txt` test 10 ok (1.2 min). The tutorial walk's data folder held `events.jsonl`, `snapshot.smsn`, and a `match-stats` `stats.json` with `"outcome"` and `"cards.yellow":[1,5]`.
- **Observation:** Both records are present and in the contract's shape. The latency tolerance is the one the sink contract states: at every stoppage and at match end, with 2 s at full time.
- **Result:** pass.

**Criterion:** The documentation set exists, and the tutorial's steps run as written.
- **Platform & tool:** cli plus web. A fresh `git clone` at `b64dc0e`, the in-app pane, and `tutorial-walk.mjs` in headless Chromium at 1280 by 800.
- **Steps performed:** Step 1 `cargo build --release` (Finished). Step 2 `generate --seed 2026 --clubs 2 --out my-league`, which wrote `club-000007ea-00.json` and `club-000007ea-01.json`. Steps 3 to 9 as written: start `serve`, open the page, pick the fourth slot, empty it, refill it, set Mentality to Positive, kick off, select 8x, queue a substitution, watch Queued become Applied, Continue at half-time, and Save replay at full time.
- **Evidence:** `tutorial/tutorial-walk.log` (10 PASS lines), `tutorial/tutorial-step-*.png`, `tutorial/tutorial-half-time.txt`, `tutorial/tutorial-full-time.txt`, `tutorial/saved-replay.txt` (20,001,508 bytes, SHA-256 `7a87176c…`), and `docs.rs` 3 / 3.
- **Observation:** Every step did what the tutorial says. The clock ran at 7.99x at 8x. The chip read "Queued" and then "Applied". The feed showed "Substitution applied", and the substitutions left went from 5 to 4. The half-time report counts goals, cards, fouls, and set pieces. The download was named `touchline-000000000000002a-1790176637954.smfx`, the engine exited with 0 at full time, and "Open a replay" was present. One wording difference: the generate command prints `wrote 2 team files to my-league`, and the tutorial quotes the first part of that line. This is a friction note, not a failure.
- **Result:** pass.

## Acceptance Criteria Status

| # | Criterion | Kind | Status | Method | Evidence | Evidence rung |
|---|---|---|---|---|---|---|
| 1 | Charter scenario: all 12 steps execute with their checkpoints against the live engine | user-observable | met | interactive | `e2e-full-run.txt` / `e2e-full-run-after-fix.txt` test 1; step screenshots; `in-app-pane-drive.txt` (steps 1–4) | headless |
| 2 | The Playwright suite runs headless, and every viewer interactive criterion has a passing test or a pre-registered human check | code-only | met | automated | 21 / 21 twice; the map in `e2e/README.md` covers every row, and its three human checks are pre-registered | n-a |
| 3 | Benchmark: one match under 2 s; 1000 matches under 30 min | user-observable | met | interactive (cli) | `bench-*.json`, `thousand.json`, `thousand.wall.txt` | live |
| 4 | Statistics record and event stream reach the sink within the stated latency | user-observable | met | interactive | observability spec ok twice; the tutorial data folder | headless |
| 5 | Every planned document exists, and the tutorial's steps run as written | user-observable | met | interactive + automated | `docs.rs` 3 / 3; `tutorial/tutorial-walk.log` from a new clone | headless |
| 6 | No dependency carries a GPL or AGPL license | code-only | met | automated | `licenses.rs` 2 / 2 | n-a |

evidence: live 1 / headless 3 / n-a 2. `metric-acceptance-mock-rung`: 0.

## Issues Found

- severity: high — DEF-1 (fixed). A caution held back for advantage and shown to a player already booked was a first yellow, so the player stayed on the pitch with two cautions. Seen in the whole-match run: two 3' yellow cards for Halan Morez, still on the pitch at 45+4'. Location: `crates/engine/src/rules/mod.rs` `show_card`, with the card judged at `foul()` by `fouls::card_outcome(..., offender.yellow, ...)`. Triage: Fix. Fixed in `f6f36b6`.
- severity: info — Out of scope, not an issue of this slice. After the fix, the same whole-match run ends 14–0 with 54 offsides for the side reduced to ten at 3' (`playwright-report-after-fix`, full-time screenshot). The realism bands still hold. This is filed as a separate investigation of how a ten-man team plays.

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| DEF-1 | unmet-ac (charter step 10 report showed a law violation) | Fix | Patched | `crates/engine/src/rules/mod.rs` `rules::tests::a_held_back_caution_for_a_booked_player_is_a_second_yellow` | Pass |

Commit: f6f36b6
Regression tests added: 1

No sub-agent tool was available in this run, so the orchestrator applied the minimal patch itself with the prescribed method. It wrote the test first, saw it fail, patched `show_card`, and saw the test pass. The patch changes only the card shown when a held-back Yellow meets a player already booked.

## Augmentation Verification

- Mock fidelity (`02c-craft.md`): this slice changed only the text in the existing `#engine-version` span. The screenshots show "Engine connected · v0.1.0" and "Engine finished · v0.1.0" in the header style, and no layout change. The owning viewer slices verified the inventory items; none was touched.
- Instrumentation (`04b-instrument.md`): `match-stats` and `match-event` records were observed on disk for a served match (the observability spec and the tutorial data folder). `darkpath.change_never_applied` and `change.expired_at_full_time` are present in the headless record.
- Benchmark compare (`05c-benchmark.md`): the median wall time is 418–421 ms, against 418–420 ms before this slice and 420–421 ms after implement. Processor time per tick is 1.449–1.493 µs, against 1.471–1.493 µs, well inside the 10 percent tripwire. Peak memory is 6.46–6.68 MB, against 6.44–6.54 MB, inside the 25 percent tripwire.
- Experiment (`04c-experiment.md`): deferred to experiment-flags; no re-check.

## Security Scan

- CVE scan: no Rust scanner is installed (`cargo audit` and `cargo deny` are not found). `npm audit` in `e2e/` found 0 vulnerabilities. The license audit test passes.
- Secret detection: no findings in the slice diff.
- SAST: none installed. Clippy with `-D warnings` is clean.

## Accessibility Gate

The browser test "every slot, control, and picker is reached with Tab and shows a focus ring" passed twice. This slice added no new control. New violations: 0. The human legibility reading belongs to `viewer-match-day` AC-7, and its deferral is still open.

## Performance Gate

No bundle (the page is served unbundled). The release build of `engine-cli` took 9.5 s incrementally. The benchmark is in the table above.

## Cross-Slice Regression

Siblings checked: every earlier slice, through the whole Rust suite (311 / 311 after the fix), the page tests (126 / 126), the viewer browser specs (20 / 20), and the slow release tests (AI trailing change, attacking against defensive, stronger team wins, and the realism bands). Regressions found: 0.

## Longitudinal Delta

- Whole-match run: the baseline is the implement run (`implement-evidence/integration/e2e-full-run1.txt`, 21 / 21, 24.4 min). This run gave 21 / 21 in 24.4 min before the fix and 25.1 min after it. The match content changed after the fix, which is expected: a player is now sent off at 3'.
- Benchmark: the baseline is `bench-baseline/integration/after-*.json`. The delta is within ±1 percent.

## Friction Notes

- The tutorial quotes the generate output as `wrote 2 team files`, and the program prints `wrote 2 team files to my-league`. A reader sees the quoted text at the start of the line.
- The first-match run after the fix ended 14–0. A manager watching this match would read it as unrealistic (see Issues Found, info).

## Free Exploration Notes

- The engine paces a served match by the page. With the pane hidden, the match stopped at 00:03 and the engine wrote nothing more. This is expected behaviour, not a defect — informational.
- `crates/engine/tests/zz_stall_probe.rs` is staged as added in the index but deleted in the working tree. It is not part of this slice or of `f6f36b6` — informational.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| Empty submission (a lineup with ten starters) | pass | Kick off stays disabled, with "Ten starters; a match needs eleven." (pane and tutorial walk) |
| Max-length input | n-a | no free-text input |
| Double-click / rapid repeat | n-a | covered by the sixth-substitution refusal in the suite |
| Mid-flow interruption | n-a | covered by the crash and drop specs of reports-recovery in the suite |
| Offline / network failure | n-a | covered by the dropped-connection spec in the suite |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Slow response | n-a | local socket only |
| Concurrent session | n-a | one viewer per served match by design |
| Session expiry mid-flow | n-a | no sessions |

## Cross-Browser Delta

Primary browser: Playwright Chromium 1243, headless. The in-app pane (Chromium) agreed on steps 1 to 4. Divergences found: none.

## Web Vitals

Not measured. The page is a local canvas application, and the frame-rate test (60 fps at 1x) is the gating performance signal; it passed.

## Gaps / Unverified Areas

- Human checks 1 and 2 in `e2e/README.md` (legibility and focus; the goal-moment motion) are pre-registered human checks for viewer criteria. They are not criteria of this slice. Check 1 is the open `viewer-match-day` AC-7 deferral, whose `needed-by` is this slice. Its clearing probe `grep -ciE "legibility|read the match screen" po-answers.md` still reads 0, so the product owner must record the reading. This run does not clear it.
- Human check 3 (the tutorial from a new clone) was performed by this run as an automated operator. The README asks for a person's name on the record, and none is recorded.

## Freshness Research

- Playwright 1.63.0 (`npx playwright --version`) with chromium-1243 already cached in `%LOCALAPPDATA%\ms-playwright`.
- Node on this host is v24.14.0 (fnm). The suite declares `>=20`.

## Recommendation

Proceed to review. Every criterion has direct evidence, and the one defect found was fixed with a regression test and re-checked in the same run. Raise the ten-man collapse with the engine owner, and ask the product owner for the legibility reading that the `viewer-match-day` deferral waits on.

## Recommended Next Stage

- **Option A: Review.** `/wf review football-manager-match-engine integration`. Convergence converged with `result: pass`. Consider compacting first, because this run was long.
- **Option D: Skip review.** `/wf handoff football-manager-match-engine integration` only if the slug-wide review ledger is accepted as covering this slice.
- **Option G: Slug-wide runtime probe.** `/wf probe football-manager-match-engine` once the product owner records the legibility reading, to clear the `viewer-match-day` deferral before ship.

## Assumptions

- A-V1 (class: implementation-detail, ac: "tutorial's steps run as written", classification: runtime-evidence): an automated operator following the tutorial from a new clone exercises exactly the steps the criterion names. The criterion is met on that evidence. The README's named-person record for human check 3 is a separate bookkeeping item.
- A-V2 (class: implementation-detail): `SM_DATA_DIR` was set to a scratch folder for the tutorial run so the user's own data folder stayed untouched. The tutorial does not mention the variable, and the engine's behaviour is otherwise the same.
- A-V3 (class: implementation-detail, ac: "benchmark", classification: runtime-evidence): the 1000-match wall time is derived from the engine's own processor time and its processor-to-wall ratio, because `bc` is absent for the shell subtraction. The file times bound it under 435 s.

## Triage Decisions

- DEF-1 (class: implementation-detail): Fix, as the autonomous policy selects. The engine's own module documentation states that a second caution sends a player off, so the patch restores stated behaviour and does not change intent.
- Ten-man collapse (class: implementation-detail): not triaged into this slice's fix loop. No criterion of this slice fails, and the realism bands pass. It was filed as a separate investigation.
- `viewer-match-day` deferral (class: implementation-detail): neither cleared nor absorbed. Its `needed-by` escalation is written above for the product owner.
