---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: distribution
status: complete
stage-number: 6
created-at: "2026-09-23T18:35:00Z"
updated-at: "2026-09-23T18:35:00Z"
result: partial
metric-checks-run: 14
metric-checks-passed: 14
metric-acceptance-met: 3
metric-acceptance-total: 4
metric-acceptance-user-observable: 3
metric-acceptance-code-only: 1
metric-acceptance-mock-rung: 0
metric-interactive-checks-run: 4
metric-interactive-checks-passed: 4
metric-issues-found: 0
metric-issues-found-initial: 1
metric-issues-found-final: 0
fix-rounds-run: 1
convergence: converged
verify-owned-fix-commit: "89fe959"
regression-tests-added: 0
constraint-resolution-missing: []
interactive-verification: deferred
interactive-verification-defer-reason: "macOS build scope row only (product owner OQ-1 = B: macOS is a pre-registered deferral). Rungs tried: (1) the platform-neutral proxy — packaging/unix/build.sh and smoke.sh ran on Linux x86_64 in WSL Ubuntu-24.04 this run, 7 of 7 checks twice, and the macOS branch of --open shares the start script; (2) cross-compile — `rustup target list --installed` -> `aarch64-linux-android armv7-linux-androideabi x86_64-linux-android x86_64-pc-windows-msvc` (no apple-darwin target), and `command -v xcrun ld64 ld64.lld zig clang` on Windows and `xcrun ld64 zig clang o64-clang` in WSL -> `not found` for every tool (no Apple SDK or linker); (3) a macOS runner — `ls .github/workflows` -> `No such file or directory` and the repository has no remote, so no CI runner exists; (4) a macOS VM — no Apple hardware, and macOS licensing binds it to Apple hardware. Residual: building, installing, and running the macOS archive on a Mac (build.sh, smoke.sh, and `open` launching the default browser)."
interactive-verification-wall-ownership: external
adapters-used: [cli, web]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/distribution/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped"
ac-staleness-checked: true
ac-stale-count: 0
longitudinal-baseline-compared: true
stability-check-flaky-count: 0
adversarial-tests-run: 4
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
steering-honored:
  - "distribution OQ-1 = B: Windows and the Linux x86_64 archive were built and checked on clean targets this run; macOS stays the product owner's pre-registered deferral and is recorded as one."
  - "distribution OQ-2 = A: the Start-menu shortcut runs the launcher (sandbox check shortcut-engine-running), and `launch --open` on the reference laptop opened the page in the default browser, which connected to the engine 0.29 s after start."
  - "Design direction: this slice changes no page; the installed page is the unchanged web/ folder, and the page suite passes 127 of 127."
  - "Dark-path counter definition: untouched; no engine or queuing code changed."
  - "Output boundary: the fix commit 89fe959 and the two script comments use product language; a vocabulary scan of every added line found 0 workflow terms."
tags: [distribution, installer, packaging, windows-sandbox, linux, wsl, launcher, release-version, deferred]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-distribution.md
  plan: 04-plan-distribution.md
  implement: 05-implement-distribution.md
  benchmark: 05c-benchmark.md
  review: 07-review-distribution.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine distribution"
---

# Verify: Distribution and Installers

## The Verification

Implement handed over commit `6a5d347` and its record commit `fddf5ac`. The slice adds a statically linked Windows program, a per-user setup file with a Start-menu entry that runs `launch --open`, and a Linux x86_64 archive with a `soccermanager` start script. AC-1 (clean Windows 11, install, match starts without configuration) and the Linux scope row are user-observable. AC-2 (release version equals `hello`) is code-only. The macOS scope row is the product owner's pre-registered deferral (OQ-1 = B). This run did every check itself, in sequence, because cargo serialises on one target folder.

Every check passed. In a fresh Windows Sandbox image with no `vcruntime140.dll`, all 11 checks passed: silent install, Start-menu entry, engine running, page screenshot, kick-off, `hello` version, and uninstall. In WSL Ubuntu 24.04, all 7 archive checks passed. On the reference laptop, `launch --open` from a copied install folder opened the page in the default browser, and the page connected to the engine 0.29 s after start. That closes the implement record's top open risk. The run found one issue. The documented Windows command for the Linux build, `wsl -d Ubuntu-24.04 -- sh packaging/unix/build.sh`, stopped with `cargo: not found`, because a non-login shell does not put `~/.cargo/bin` on PATH. The fix round patched `build.sh` and `smoke.sh` (commit `89fe959`), and the literal documented commands then passed. The result is `partial` only because the macOS row is deferred.

Review can proceed. Ship stays blocked until an operator runs `build.sh` and `smoke.sh` on a Mac. The top open risk is SmartScreen on a downloaded, unsigned setup file. The sandbox copy carries no Mark of the Web, so this run did not exercise the warning.

## Verification Summary

- Slice: `distribution`, standard mode, on the dedicated branch `feat/football-manager-match-engine`. The code under test is `6a5d347`, and the fix is `89fe959`.
- Stack: `stack.user-confirmed: true`, platforms `[web, cli]`, and the plan has no unconfirmed stamp.
- Constraint-resolution gate: AC-1 and the Linux row carry `constraint-resolution: prerequisite-slice: distribution`. The macOS row carries `proxy+deferral`. `constraint-resolution-missing: []`.
- No earlier `06-verify-distribution.md` or verify evidence existed. This is evidence run 1, so nothing was archived. The driver said that an earlier round had applied one fix pass, but nothing on disk shows it. This run's fix round is round 1 for this slice.
- Capability probes this run: `Test-Path $env:SystemRoot\System32\WindowsSandbox.exe` gave `True`. `wsl -d Ubuntu-24.04 -- bash -lc 'cargo --version'` gave `cargo 1.92.0 (344c4567c 2025-10-21)`. The macOS probe is in `macos-probe.txt`.
- Issues: 1 initial (TOOL-1) and 0 final. Convergence: `converged`.

## Automated Checks Run

1. `cargo fmt --all -- --check`: pass, exit 0 (`fmt.txt`).
2. `cargo clippy --workspace --all-targets -- -D warnings`: pass, exit 0 (`clippy.txt`).
3. `cargo test --workspace --no-fail-fast`: pass. 403 passed, 0 failed, and 4 ignored over 58 result lines, exit 0 (`test-workspace.txt`). This is the same count as the implement record.
4. `cargo build --release -p engine-cli`: pass, exit 0 (`release-build.txt`).
5. `cargo test -p engine-cli --release --test install_layout --test release_version` on Windows: 5 of 5 passed (`test-ac-windows.txt`).
6. The same two tests in WSL (`CARGO_TARGET_DIR=~/.cache/soccermanager-target`, `--locked`): 5 of 5 passed (`test-ac-wsl.txt`). This ran after the fix, so `the_build_scripts_read_the_version_and_never_write_it` also covers the patched scripts.
7. `node --test web/tests/*.test.mjs`: 127 passed, 0 failed, 0 skipped (`web-tests.txt`).
8. Static runtime: `grep -a -c VCRUNTIME140.dll target/release/engine-cli.exe` gave `0` (`vcruntime.txt`).
9. `pwsh packaging/windows/build.ps1`: exit 0. It wrote `SoccerManager-0.1.0-windows-x64-setup.exe` (2,587,652 bytes, SHA-256 `729d77e8…412f`) (`windows-build.txt`).
10. `pwsh packaging/windows/run-sandbox.ps1`: overall PASS, 11 of 11 checks and 4 facts, exit 0 (`windows-sandbox.txt`, `windows/results.json`).
11. `wsl -d Ubuntu-24.04 -- bash -lc 'sh packaging/unix/build.sh'`: exit 0. It wrote the archive (15,352,174 bytes) (`linux-build.txt`).
12. `sh packaging/unix/smoke.sh` in WSL: overall PASS, 7 of 7 checks and 3 facts, exit 0 (`linux-smoke.txt`, `linux/results.json`).
13. Benchmark against the `05c-benchmark.md` gate (`engine-cli bench --seed 42 --matches 5 --json` ×3, `SM_DATA_DIR` set to a scratch folder): 418.8, 409.4, and 415.6 ms per match (median 415.6, tripwire 460.7). Peak memory was 6.617, 6.617, and 6.785 MB (median 6.617, tripwire 6.82). Pass (`bench-summary.txt`, `bench-{1,2,3}.json`). The static C runtime did not move the gate.
14. Security, debt, and boundary scan over the 1,305 added lines of `6a5d347` plus the fix: 0 secret-pattern hits, 0 added `unsafe` blocks, 0 `sdlc-debt` markers, 0 workflow-vocabulary hits, and no `Cargo.lock` change (`security-boundary.txt`).

Before the fix, one more command was run: the literal documented command `wsl -d Ubuntu-24.04 -- sh packaging/unix/build.sh`. It failed with `packaging/unix/build.sh: 19: cargo: not found`, exit 127 (`linux-build-readme-literal.txt`). This is TOOL-1. The run then read the WSL PATH with `wsl -d Ubuntu-24.04 --exec sh -c 'command -v cargo || …'`, which gave `cargo not on non-login PATH` (`wsl-path-probe.txt`). The re-checks after the fix are listed under Verify-Owned Fixes.

## Interactive Verification Results

- **Criterion:** AC-1, "Given a clean Windows 11 machine, When the installer runs and the user opens the page, Then the engine is found and a match starts without configuration."
  - **Platform & tool:** Windows Sandbox, a fresh Windows 11 image with networking off, driven by `packaging/windows/run-sandbox.ps1` and `smoke.ps1` (cli adapter, infra-1). Headless Microsoft Edge took the page screenshot.
  - **Steps performed:** build the setup file. Start the sandbox. Install silently, and check the files and the Start-menu entry. Start the Start-menu entry, and check `engine.port` and a `running` `/engine.json`. Run a second launch and take a 1280 by 800 page screenshot. Run a third launch, read `hello` over the socket, send `start`, and wait for the kick-off record. Uninstall silently. Close the sandbox.
  - **Evidence:** `verify-evidence/distribution/windows/results.json`, `page.png`, `desktop.png`, `events.jsonl`, `smoke.log`, and the launch logs.
  - **Observation:** facts: `vcruntime140-present: False`, `data-folder-before: False`, `installed-version: 0.1.0`, and `shortcut-browser: 0`. Checks that passed: `install-exit-code`, `installed-files`, `shortcut-engine-port`, `shortcut-engine-running` (launcher port 49670, socket 49671), `launch-address`, `launch-engine-running`, `page-screenshot`, `match-kick-off` (match `18d806b39b13dcd8-1790188220353`), `hello-version` (`0.1.0` = `0.1.0`), `uninstall-removes-program`, and `uninstall-keeps-matches`. `page.png` shows the installed page with the Touchline header and score bug at 0 – 0. Its step list reads "Starting the engine: Done" and "Connecting to the match: In progress". `desktop.png` shows the sandbox dialog "We can't open this 'http' link". This is the known limit of the sandbox image, and the item below covers it.
  - **Result:** pass.
- **Supplementary to AC-1 (the OQ-2 browser path on the reference laptop):**
  - **Platform & tool:** Windows 11 on the reference laptop, with the default `http` handler `MSEdgeHTM`, driven by a PowerShell drive (`host-open-drive.txt`).
  - **Steps performed:** copy the staged install layout (`dist/stage`) to a new temporary folder. Remove every `SM_*` variable. Point `SM_DATA_DIR` at a scratch folder, so the player's data folder is not touched. Run `engine-cli launch --open --minutes 1` from an unrelated folder. After 8 s, read `/engine.json` and the launcher log. Stop the launcher, and delete the folder.
  - **Observation:** the address is `http://127.0.0.1:53373`, and `engine.state` is `running`. The log has `web.serving … dir=web files=42`, then `socket.listening port=53374`, and then `socket.client origin=http://127.0.0.1:53373` at 0.29 s. So the default browser opened the page, and the page connected to the engine. No `launch.open_failed` was logged. Edge was already running, so it opened the page as a tab and started no new process (new `msedge` processes: 0).
  - **Result:** pass.
- **Criterion:** Linux scope row (OQ-1 = B), "the archive runs from a fresh home with no configuration."
  - **Platform & tool:** WSL Ubuntu-24.04 (glibc 2.39), `packaging/unix/smoke.sh` (cli adapter, cli-1).
  - **Steps performed:** unpack the archive into a fresh `HOME`. Start `./soccermanager` with `env -i` and `PATH=/usr/bin:/bin` from an unrelated folder. Fetch the page and `/engine.json`. Check the data folder, the version, and the install-layout test against the packaged folder.
  - **Evidence:** `verify-evidence/distribution/linux/` (drive 1, before the fix) and `linux-recheck/` (drive 2, after the fix, with the literal documented command).
  - **Observation:** both drives passed all 7 checks: `extract`, `address`, `page`, `engine-running`, `data-folder`, `version`, and `hello-version`. The facts are `ldd` (libc, libm, and libgcc_s only), `glibc 2.39`, and `browser: launch.open_failed logged`. WSL has no `xdg-open`, and the game kept running.
  - **Result:** pass.
- **Criterion:** macOS scope row (OQ-1 = B: deferred). This row was not driven. The platform does not exist on this host (`macos-probe.txt`). It is deferred as recorded below.

## Acceptance Criteria Status

| # | Criterion | kind | status | method | evidence | evidence-rung |
|---|-----------|------|--------|--------|----------|---------------|
| AC-1 | "Given a clean Windows 11 machine, When the installer runs and the user opens the page, Then the engine is found and a match starts without configuration." | user-observable (explicit `observable: true`) | met | interactive | Windows Sandbox 11 of 11 (`windows/results.json`, `page.png`, `events.jsonl`). Supplement: the host `--open` drive (`host-open-drive.txt`). | emulator-or-container |
| AC-2 | "Given a release build, Then its version string matches the engine hello message." | code-only (explicit `observable: false`) | met | automated | `release_version` 3 of 3 and `install_layout` 2 of 2 on Windows and in WSL. Sandbox `hello-version` `0.1.0` = `0.1.0`. Linux smoke `version` and `hello-version`. | n-a |
| SCOPE-linux | Linux x86_64 archive runs from a fresh home with no configuration (plan Verification Strategy, OQ-1 = B) | user-observable (plan: "the install experience on Linux") | met | interactive | `linux/results.json` and `linux-recheck/results.json`, 7 of 7 each | emulator-or-container |
| SCOPE-macos | macOS build (plan Verification Strategy, OQ-1 = B: deferred) | user-observable | runtime-evidence deferred | none (deferred) | (none — deferred; probe receipt in `macos-probe.txt` and in the frontmatter defer-reason) | n-a |

Rollup: evidence emulator-or-container 2, n-a 2 (1 code-only, 1 deferred). User-observable 3, mock-rung 0.

Deferral (recorded once here, and in the `00-index.md` ledger): SCOPE-macos. The wall is `external`. The clearing event is: "an operator provides a Mac or a macOS CI runner, runs `sh packaging/unix/build.sh` then `sh packaging/unix/smoke.sh dist/SoccerManager-<v>-macos-<arch>.tar.gz .ai/workflows/football-manager-match-engine/verify-evidence/distribution/macos`, and re-runs verify for this slice or `/wf probe`". The clearing probe is `test -f .ai/workflows/football-manager-match-engine/verify-evidence/distribution/macos/results.json`.

## Issues Found

None open. TOOL-1 was found and fixed in this run (see Verify-Owned Fixes).

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|----|------|--------|-------------------|-----------------|-----------------|
| TOOL-1 | check-failure (tooling): the documented `wsl -d Ubuntu-24.04 -- sh packaging/unix/build.sh` exits 127 with `cargo: not found`, because a non-login WSL shell lacks `~/.cargo/bin` on PATH (`packaging/unix/build.sh:19`, `packaging/README.md:46`) | Fix (autonomous) | Patched: `build.sh` and `smoke.sh` add `$HOME/.cargo/bin` to PATH only when `cargo` is missing and rustup's binary exists there. The game's own `env -i` run in `smoke.sh` is unchanged. | exempt: tooling fix to a build script. The re-run of the literal documented commands is the check that caught it. | Pass: the literal build command exited 0 (`recheck-linux-build-literal.txt`). The literal smoke command passed 7 of 7 (`recheck-linux-smoke-literal.txt`). `release_version` in WSL passed 3 of 3 after the patch. |

Commit: `89fe959` (`fix(release): find cargo when the Linux build runs in a non-login shell`). It holds only `packaging/unix/build.sh` and `packaging/unix/smoke.sh`, committed by pathspec, and mode 100755 is kept.
Regression tests added: 0.

## Augmentation Verification

- **Benchmark (`05c-benchmark.md`, default-match gate):** median 415.6 ms per match against the 460.7 ms tripwire, and median 6.617 MB against the 6.82 MB tripwire. Pass. This answers the implement record's open question about `crt-static` moving the numbers.
- **Instrumentation (`04b-instrument.md`):** the slice adds `launch.open_failed`. It fired on Linux, where WSL has no `xdg-open`. It did not fire on the Windows host, where the open succeeded. The launcher's existing signals (`web.serving`, `launch.worker_started`, `socket.listening`, `socket.client`) fired in the host drive.
- **Experiment (`04c-experiment.md`):** no flag is involved in this slice.
- **Craft (`02c-craft.md`):** no page file changed, so no mock-fidelity item applies. The installed page renders the unchanged viewer (`page.png`).

## Security Scan

- CVE: `cargo audit` is not installed (`error: no such command: audit`). `Cargo.lock` did not change in this slice, so no new dependency exists.
- Secrets: 0 hits in the pattern scan of the added lines.
- SAST: clippy `-D warnings` is clean, and the change adds 0 `unsafe` blocks. `open_browser` passes only the launcher's own `http://127.0.0.1:<port>/` address to `cmd /C start "" <url>`, `open`, or `xdg-open`, with no user-supplied text. The installer runs per user with `RequestExecutionLevel user`, and its uninstaller deletes files by name. New HIGH+ issues: 0.

## Accessibility Gate

Not automatable for this slice. No UI element changed. The installer uses the stock NSIS MUI2 pages. New WCAG AA violations: 0.

## Performance Gate

- Bundle size delta: skipped, because no page file changed. The absolute artifact sizes are 2,587,652 bytes for the setup file and 15,352,174 bytes for the archive.
- Processor time and memory: see the benchmark gate above.

## Cross-Slice Regression

The workspace suite covers every sibling slice's Rust tests: 403 pass and 0 fail. The 5 `tests/launch.rs` cases from viewer-reports-recovery and the 3 `tests/docs.rs` cases from integration are in that count. The page suite passes 127 of 127. Regressions found: 0.

## Longitudinal Delta

- Windows Sandbox: compared with the implement record's run 2. It is again 11 of 11, with the same 4 facts.
- Linux smoke: compared with the implement run. It is again 7 of 7, with the same glibc 2.39 and `ldd` set.
- Benchmark: the median of 415.6 ms sits inside the 412.6–421.8 ms range that the scripting-runtime verify recorded for the build before `crt-static` (its no-pack drives). No delta is attributable to the static runtime.

## Friction Notes

- The sandbox page screenshot shows the page still "Connecting to the match" at the 8 s headless budget. The kick-off is proven by the socket check in the same run, not by the page. The page holds kick-off until a lineup is confirmed, as the implement record explains, and the viewer slices verified that part of the flow in the page. Informational.
- The host `--open` drive opened a tab in the reference laptop's Edge at a local address. The launcher was stopped afterwards, and the tab stays until it is closed. Informational.

## Free Exploration Notes

- The archive hash changes on every build, because tar records file times. The `.sha256` is written beside each archive, so it stays correct for that archive. Informational.
- `git status` showed the two patched scripts as staged immediately after the edit. The commit used a pathspec, and 4 paths that another session had staged (`crates/engine/tests/zz_stall_probe.rs` and three `docs/design/realism/` files) stay staged and uncommitted. Informational.

## Adversarial Tests

| Test | Result | Finding |
|------|--------|---------|
| Start from an unrelated folder with every `SM_*` variable removed (Windows host) | pass | the launcher found `web/` and `content/` beside the program |
| Start with a cleared environment (`env -i`, `PATH=/usr/bin:/bin`) in a fresh home (Linux) | pass | smoke `address`, `page`, `engine-running` |
| No browser opener available (Linux) | pass | `launch.open_failed` logged; the game kept running |
| Install folder missing `web/` | pass | `a_layout_without_the_page_folder_is_refused_naming_both_folders_tried` |

## Failure Mode Probes

| Probe | Result | Finding |
|-------|--------|---------|
| Slow response / concurrent session / session expiry | n-a | a local installer and launcher with no session or remote network surface |

## Gaps / Unverified Areas

- macOS: deferred (see Acceptance Criteria Status).
- SmartScreen and the Mark of the Web: the setup file in the sandbox came from a mapped folder, not from a download, so the warning path was not exercised. The README explains what to click.
- The Start-menu entry opening a browser was proven on the reference laptop through the same `launch --open` command, not through the installed shortcut itself. The host run used the staged layout that the setup file packs. The sandbox proves the shortcut runs that command.
- `cargo audit` is not installed.

## Freshness Research

No dependency changed, and the plan and implement ran on 2026-09-22 and 2026-09-23. This run observed the NSIS 3.12 setup, Windows Sandbox (`.wsb`, networking off, loopback working, no `http` handler), and WSL Ubuntu-24.04 (cargo 1.92.0, glibc 2.39) directly. AC staleness: 0.

## Assumptions

Each entry is an autonomous decision of this run, with its class per `_decision-classes.md`.

- **V-1** AC-1 is user-observable (explicit `observable: true`) and classified `runtime-evidence`. It was produced in Windows Sandbox, which is a clean Windows 11 image and not a mock. The rung is `emulator-or-container`. `class: implementation-detail`.
- **V-2** AC-2 is code-only (explicit `observable: false`) and classified `build-capability`. It is met by the version tests on both platforms and by both smoke checks. `class: implementation-detail`.
- **V-3** The plan's two scope rows (Linux, macOS) are counted as criteria, because the plan's Verification Strategy gives each a method, and the slice's Risks section asks for a named deferral per platform. `metric-acceptance-total` is 4. `class: implementation-detail`.
- **V-4** "A match starts" is proven by the smoke check's `start` command and the kick-off record, as in implement D7. The page sends the same protocol command after the lineup is confirmed. `class: implementation-detail`.
- **V-5** The OQ-2 browser path was proven on the reference laptop with `SM_DATA_DIR` set to a scratch folder, so the player's match data was not touched. The data folder is not what that drive tests. `class: implementation-detail`.
- **V-6** SCOPE-macos is deferred with `result: partial`. This applies the product owner's recorded answer (`po-answers.md`, OQ-1 = B: "macOS is a pre-registered deferral"). It is not a new scope decision, and the wall was re-probed this run. `class: implementation-detail`, `ac: SCOPE-macos`, `classification: runtime-evidence`.
- **V-7** TOOL-1 was triaged Fix without asking. It is a tooling defect in this slice's own script, and the fix is minimal and changes no behaviour of the game. `class: implementation-detail`.
- **V-8** The fix was applied inline, not by a write-isolated sub-agent. It is a 6-line change to two scripts that no other check was running, and this run has no sub-agent dispatch tool. `class: implementation-detail`.
- **V-9** The consult trigger `ac-deferred` holds, but no consult ran. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`). `class: implementation-detail`.
- **V-10** This slice's roster status is set to `in-progress` in `03-slice.md` and `00-index.md`, per verify Step 10: a deferral-only `partial` is not complete. `class: implementation-detail`.
- **V-11** The Windows Sandbox that this run started was closed by stopping its two processes, as in implement D15. `class: implementation-detail`.

## Triage Decisions

- TOOL-1: **Fix** (autonomous; this replaces the gate question). The patch landed, and the re-check passed.

## Recommendation

Ready for review with one open deferral. Both acceptance criteria and the Linux scope row are met by runtime evidence from clean machines, and the browser-open path is proven on the reference laptop. macOS waits for an operator with a Mac, and ship blocks until then.

## Recommended Next Stage

- **Option A (default): Review** → `/wf review football-manager-match-engine distribution`. Convergence is `converged` and the result is `partial` only for the deferred macOS row. Review may proceed with a soft warning.
- **Option F: Re-verify in a capable environment** → `/wf verify football-manager-match-engine distribution` on a Mac, or `/wf probe football-manager-match-engine` after the macOS clearing event. This clears the deferral that blocks ship.
- **Option G: Slug-wide runtime probe** → `/wf probe football-manager-match-engine`. Use it for a cross-slice sweep.
