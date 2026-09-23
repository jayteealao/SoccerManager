---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: distribution
status: complete
stage-number: 5
created-at: "2026-09-23T18:22:08Z"
updated-at: "2026-09-23T18:22:08Z"
metric-files-changed: 18
metric-lines-added: 1293
metric-lines-removed: 10
metric-deviations-from-plan: 8
metric-review-fixes-applied: 0
commit-sha: "6a5d3472c171efe3e55c58c1d871defd835c45f6"
commits:
  - "6a5d3472c171efe3e55c58c1d871defd835c45f6"
steering-honored:
  - "distribution OQ-1 = B: the slice ships a Windows setup file and a Linux x86_64 .tar.gz built and smoke-tested in WSL Ubuntu-24.04; macOS is not built and stays a pre-registered deferral."
  - "distribution OQ-2 = A: the Start-menu shortcut runs `engine-cli launch --open` (packaging/windows/installer.nsi:67)."
  - "Design direction: this slice changes no page; the installed page is the unchanged web/ folder without web/tests/."
  - "Dark-path counter definition: untouched; the computer manager's queuing behaviour did not change."
  - "Output boundary: the commit message, code comments, script comments, installer strings, and README text use product language; a search of every added line found no workflow vocabulary."
tags: [distribution, installer, packaging, nsis, windows-sandbox, linux, wsl, launcher, release-version, deferred]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-distribution.md
  plan: 04-plan-distribution.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/distribution/
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md, 05-implement-commentary.md, 05-implement-calibration.md, 05-implement-viewer-match-day.md, 05-implement-viewer-lineup-tactics.md, 05-implement-viewer-reports-recovery.md, 05-implement-integration.md, 05-implement-extra-time-penalties.md, 05-implement-experiment-flags.md, 05-implement-scripting-runtime.md]
  verify: 06-verify-distribution.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine distribution"
---

# Implement: Distribution and Installers

## The Implementation

The build started from a game that ran only from a development build. `engine-cli launch` existed as the reports-and-recovery work left it, and it required `--seed` and `--web`. The Windows release binary imported `VCRUNTIME140.dll`, and no installer or archive existed. `LaunchOpts` and `launch.rs` matched the plan's description, so Step 3 went ahead without a re-plan.

The launcher now finds the page folder the same way the engine finds its content folder: the flag, then `SM_WEB_DIR`, then `./web`, then `web/` beside the program. It picks a seed from the clock when none is given, and `--open` opens the page in the default browser. The Windows program links the C runtime statically. A per-user NSIS setup installs the program with its `content/` and `web/` folders and a Start-menu entry that runs `launch --open`. A Linux archive holds the same layout and a `soccermanager` start script. The build made 8 minor, in-scope deviations from the plan. The largest is that the Windows clean-machine check proves that a match starts through its own socket client. The page holds kick-off until the manager confirms a lineup, so a script cannot press that button. All 17 autonomous decisions are `class: implementation-detail`.

Both paths ran once on the reference laptop. In Windows Sandbox, all 11 checks passed on a clean Windows 11 image that has no `vcruntime140.dll`: install, shortcut, engine running, page screenshot, kick-off, hello version, and uninstall. In WSL Ubuntu 24.04, all 7 archive checks passed. 403 Rust tests pass with 0 failed (5 new), 127 page tests pass, and fmt and clippy (`-D warnings`) are clean. Verify can re-run both clean-machine checks. The top open risk is that the Start-menu entry did not open a browser in Windows Sandbox. The sandbox image cannot open any `http` link from any program, so the browser path is proven only on a machine with a default browser.

## Summary of Changes

- Static C runtime for the MSVC target (`.cargo/config.toml`). The release binary now has 0 `VCRUNTIME140.dll` imports (`grep -a -c` returned `0`).
- `web::resolve_web_dir` and `SM_WEB_DIR`: the page-folder search mirrors `ContentDir::resolve`, and the refusal lists every folder tried.
- `launch`: `--seed` and `--web` are optional, and `--open` is new. The launcher picks a clock seed and prints `seed <n>` to stderr, so stdout still carries only the page address. It opens the browser per platform, and a failure logs `launch.open_failed` and keeps running. `serve --web` and `replay --web` are unchanged.
- Tests: `install_layout.rs` has 2 cases, and the first runs against a packaged folder when `SM_INSTALL_UNDER_TEST` is set. `release_version.rs` has 3 cases.
- Windows packaging: `build.ps1`, `installer.nsi`, `smoke.ps1`, and `run-sandbox.ps1`.
- Unix packaging: `build.sh`, `soccermanager`, and `smoke.sh`, all POSIX `sh` that name the platform from `uname`.
- Docs: `packaging/README.md`, an Install section in `README.md`, and the new `launch` options and `SM_WEB_DIR` in `docs/reference/cli.md`.

## Files Changed

- `.cargo/config.toml` (new): `+crt-static` for `x86_64-pc-windows-msvc` only.
- `.gitattributes` (new): `packaging/unix/*` checks out with LF endings.
- `.gitignore`: ignores `dist/`.
- `README.md`: an Install section for both platforms, and `launch --open` for a development build.
- `docs/reference/cli.md`: the `SM_WEB_DIR` row; `--seed` defaults to the clock; `--web` has a search order; `--open` is listed.
- `crates/engine-cli/src/cli.rs`: `LaunchOpts.seed: Option<u64>`, `web: Option<PathBuf>`, `open: bool`, with help lines under 80 columns.
- `crates/engine-cli/src/launch.rs`: finds the page folder, adds `clock_seed`, and adds `open_browser`.
- `crates/engine-cli/src/web.rs`: `WEB_DIR_ENV` and `resolve_web_dir`.
- `crates/engine-cli/tests/install_layout.rs` (new): the copied or packaged layout run from an unrelated folder; the missing-page refusal.
- `crates/engine-cli/tests/release_version.rs` (new): `--version`, `hello`, the workspace version, and the build scripts hold one version.
- `packaging/README.md` (new): build, install, remove, the clean-machine checks, the glibc floor, SmartScreen, the sandbox link limit, and macOS.
- `packaging/windows/build.ps1` (new): builds, stages, runs `makensis`, and writes the setup file and its `.sha256`.
- `packaging/windows/installer.nsi` (new): per-user MUI2 setup, the shortcut, the uninstall key, and the uninstaller.
- `packaging/windows/smoke.ps1` (new): the clean-machine check (Windows PowerShell 5.1).
- `packaging/windows/run-sandbox.ps1` (new): writes `dist/smoke.wsb`, starts the sandbox, waits for `results.json`, and exits with the verdict.
- `packaging/unix/build.sh` (new, mode 755): the versioned archive and its `.sha256`.
- `packaging/unix/soccermanager` (new, mode 755): `exec engine-cli launch --open "$@"`.
- `packaging/unix/smoke.sh` (new, mode 755): the fresh-home check.

## Shared Files (also touched by sibling slices)

- `crates/engine-cli/src/launch.rs` and `cli.rs` (`LaunchOpts`): created by viewer-reports-recovery. This slice changes only the seed and page-folder resolution and adds `--open`. Supervision, restart, and abandon are unchanged, and the 5 `tests/launch.rs` cases still pass.
- `crates/engine-cli/src/web.rs`: a sibling file of stream-protocol and viewer-reports-recovery. This slice adds one free function and changes no request handling.
- `docs/reference/cli.md` and `README.md`: from integration. `tests/docs.rs` (3 cases) still passes.

## Notes on Design Choices

- The installed layout describes itself. The program finds `content/` and `web/` beside itself, so the installer and the archive copy files and add a start entry, with no logic of their own.
- The uninstaller deletes what the setup wrote, by name, then removes the folder only if it is empty. The plan said "removes `$INSTDIR`". A recursive delete of a folder the player chose could empty an unrelated folder.
- The version enters the build only through `engine-cli --version`. `release_version.rs:87` fails if a build script ever contains the literal version.
- `open_browser` returns early on success and logs only a failure. On Windows, `cmd /C start` exits 0 even when the shell then cannot open the link, so a Windows failure of that kind is not logged (see Known Risks).
- The Linux target folder is `$HOME/.cache/soccermanager-target` in both `build.sh` and `smoke.sh`, so the test build in the smoke check does not write Linux objects into the Windows `target/`.

## Verification Seams Built

- AC-1 (clean Windows 11, installer, match starts without configuration) → `packaging/windows/smoke.ps1` writes `results.json` (line 262) with the checks `match-kick-off` and `hello-version` (lines 234–235), plus `desktop.png` and `page.png`. `packaging/windows/run-sandbox.ps1` drives it in Windows Sandbox. This enables verify to observe it with a Windows Sandbox run (infra-1) and to read `results.json`.
- AC-1 development proxy → `install_layout.rs:148` runs the copied layout from an unrelated folder with the path variables removed. This enables `cargo test` (cli-1) to observe it.
- AC-2 (release version equals `hello`) → `release_version.rs:49`, `:54`, and `:87`; smoke check `hello-version` (`smoke.ps1:235`). On Linux, `install_layout.rs:60` reads the `SM_INSTALL_UNDER_TEST` seam, and `smoke.sh:126` drives it. This enables `cargo test` (cli-1) and both smoke runs to observe it.
- Linux archive scope (OQ-1 = B) → `packaging/unix/smoke.sh` writes `results.json` (line 138). This enables verify to observe it with a WSL shell drive (cli-1).
- macOS (deferred) → the `uname` platform naming in `build.sh` and the `target_os = "macos"` branch of `open_browser` (`launch.rs:162`). Nothing here can observe macOS; the deferral stays pre-registered.

## Deviations from Plan

1. **Hello client (Step 4).** The test reads `hello` with the workspace's `stream::Client` (`crates/stream/src/client.rs:32`, which wraps `tungstenite` 0.30), not with a raw `tungstenite` client. `engine-cli` has no direct `tungstenite` dev-dependency, and `tests/stream_cli.rs` and `tests/launch.rs` already use `stream::Client`. Reuse; no new dependency.
2. **Where the smoke check proves a match starts (Step 9 d and f).** Check (d) does not wait for a kick-off after the shortcut. The page holds kick-off until the manager confirms a lineup (`web/main.mjs:589-592`: `startMatch` sends `start` only after `set-lineup` is accepted), so an unattended page never kicks off. Check (d) proves the shortcut's launcher with two checks: `engine.port` is written, and the launcher's own `/engine.json` reports `running` (it finds the port from the launcher's listening sockets). Check (f) sends `{"type":"start"}` over its own socket and waits for the `kick-off` record in `events.jsonl`, which is where "a match starts" is proven.
3. **Browser fact (Step 9 d).** Added the fact `shortcut-browser`, the count of `msedge` processes 20 seconds after the shortcut starts, as a fact and not a check. A diagnostic sandbox run (`implement-evidence/distribution/windows-sandbox-links/`) showed that Windows Sandbox cannot open any `http` link. PowerShell's `Start-Process http://…` also showed "We can't open this 'http' link" (`desk2.png`), with the `http` UserChoice set to `MSEdgeHTM`.
4. **Headless Edge flags (Step 9 e).** Added `--no-first-run`, a throw-away `--user-data-dir`, and `--virtual-time-budget=8000`, so the first-run page and the profile of the sandbox user do not interfere.
5. **Uninstaller (Step 7).** The uninstaller deletes by name, then runs a non-recursive `RMDir $INSTDIR`, instead of a recursive delete (see Notes). It also writes `InstallLocation` and `QuietUninstallString` to the uninstall key.
6. **`.gitattributes` (new file, not in the plan).** `core.autocrlf` is `true` in this repository. Without the rule, a Windows checkout gives the Unix scripts CRLF endings, and `sh` reads `set -eu\r` as an error.
7. **Executable bit.** `core.filemode` is off on this Windows checkout, so the mode 755 of the three Unix scripts was set in the index (`git update-index --chmod=+x`). `build.sh` also runs `chmod 755` on the staged copies.
8. **`build.ps1` passes `/DOUTFILE`.** The setup file is written straight to `dist/`. The installer script has a fallback name when the define is absent.

The plan named 16 files. The change touches 18: the planned 16, plus `.gitattributes`, plus `docs/reference/cli.md`. The plan's Step 13 already made `docs/reference/cli.md` conditional on the file existing, and it exists.

## Anything Deferred

- **macOS build.** Deferred by the product owner (OQ-1 = B). `build.sh`, `smoke.sh`, and the `open` branch are written for it, and nothing here ran on macOS. Clearing event: an operator with a Mac or a macOS CI runner runs `build.sh`, then `smoke.sh`. The deferral is pre-registered in `04-plan-distribution.md`; verify applies it.
- **Code signing and notarization.** Out of scope beyond what the platform requires to run (slice Scope). SmartScreen warns on the unsigned setup file, and `packaging/README.md` and `README.md` say what to click.
- **Auto-update.** Out of scope (slice Scope).

## Known Risks / Caveats

- **The browser open is unproven on Windows.** In Windows Sandbox, the shortcut starts the engine and serves the page, and no browser opens, because the sandbox image cannot open `http` links at all. `cmd /C start` exits 0 in that case, so `launch.open_failed` is not logged. On a normal Windows 11 machine with a default browser, `start` opens the link. Verify or the product owner can confirm this on the reference laptop by running the Start-menu entry.
- **SmartScreen and antivirus.** The setup file is unsigned. The sandbox copy has no Mark of the Web, so the warning was not exercised. Windows Defender was on in the sandbox and did not block the setup file.
- **glibc floor.** The Linux archive links against glibc 2.39 (`ldd` fact in `implement-evidence/distribution/linux/results.json`). Older distributions refuse to start it.
- **`crt-static` applies to every MSVC build in the workspace**, including tests and benches. Benchmark numbers can move slightly. This stage did not re-measure them; verify compares them against the `05c-benchmark.md` tripwires.
- **One match per start.** After full time, the launcher reports `finished`. The player starts the game again for a second match, and the README says so.

## Freshness Research

- **Installed NSIS 3.12** (`C:\Program Files (x86)\NSIS\makensis.exe`, `Include\MUI2.nsh`). `MUI_FINISHPAGE_RUN_FUNCTION`, `CreateShortcut` with `SW_SHOWMINIMIZED`, `RequestExecutionLevel user`, and `VIProductVersion` (four-part) were used as the installed headers define them. `makensis /V2` built the setup file with no warning.
- **Windows Sandbox (this run).** `.wsb` `MappedFolders` with absolute host paths, `Networking Disable`, `MemoryInMB`, and `LogonCommand` worked as the plan's Microsoft Learn reading described. Only one sandbox runs at a time, so a finished sandbox must be closed before the next run. Loopback sockets work with networking disabled. The image has no `vcruntime140.dll` and cannot open `http` links.
- **WSL Ubuntu-24.04 (this run).** `cargo 1.92.0` built the archive in 35.94 s into `~/.cache/soccermanager-target`. `ldd` shows only `libc`, `libm`, and `libgcc_s`. `xdg-open` is absent, so the smoke check exercised the `launch.open_failed` path.
- **Dependencies.** No new crate.

## Assumptions

- **D1** (class: implementation-detail): `LaunchOpts` and `launch.rs` as landed match `04-plan-viewer-reports-recovery.md` Steps 5 and 9 (`--seed`, `--minutes`, `--team-a`, `--team-b`, `--web` required, `--engine`), so Step 3's stop condition did not fire.
- **D2** (class: implementation-detail): The refusal wording is `cannot read page folder (tried …): no folder holds index.html`, the shape of the content-folder refusal.
- **D3** (class: implementation-detail): An empty `SM_WEB_DIR` counts as unset, which matches the launcher's `SM_ENGINE_PATH` handling.
- **D4** (class: implementation-detail): The clock seed folds the 128-bit nanosecond count into 64 bits with XOR.
- **D5** (class: implementation-detail): `open_browser` waits for the opener's exit status. `cmd /C start`, `open`, and `xdg-open` all return at once.
- **D6** (class: implementation-detail): The `hello` client in the tests is `stream::Client` (Deviation 1).
- **D7** (class: implementation-detail, ac: "clean Windows 11 machine … match starts without configuration", classification: runtime-evidence): "A match starts" is proven by the smoke check's own `start` command and the `kick-off` record. The shortcut path is proven by `engine.port` and a `running` `/engine.json` (Deviation 2). The AC's user opens the page and presses Kick off; a script cannot press it, and the protocol command is the same.
- **D8** (class: implementation-detail): The browser count after the shortcut is a fact, not a check (Deviation 3), because Windows Sandbox cannot open links and a check would fail on the environment, not on the code.
- **D9** (class: implementation-detail): The uninstaller removes by name (Deviation 5).
- **D10** (class: implementation-detail): `.gitattributes` forces LF for `packaging/unix/*` (Deviation 6).
- **D11** (class: implementation-detail): The executable bit is recorded in the index. The code commit was re-recorded once with the same message and tree plus the modes, before any push (Deviation 7).
- **D12** (class: implementation-detail): The publisher string is "Soccer Manager contributors". No legal entity is named in the repository.
- **D13** (class: implementation-detail, ac: "version string matches the engine hello message", classification: build-capability): One version source, held by `release_version.rs` and by both smoke checks.
- **D14** (class: implementation-detail): The run evidence (both `results.json`, the screenshots, the smoke log, the diagnostic run, and both `.sha256` files) is copied into `implement-evidence/distribution/`, because `dist/` is git-ignored. The setup file and the archive stay in `dist/` and are not committed.
- **D15** (class: implementation-detail): The finished sandboxes were closed by stopping `WindowsSandboxRemoteSession.exe` and `WindowsSandboxServer.exe`. Each was a sandbox this run started, and the image is discarded on close.
- **D16** (class: implementation-detail): The code commit uses the repository's product-language form (`feat(release): …`), not the stage's default subject, per the output boundary. It includes only this slice's paths, by pathspec. Four paths another session had already staged (`crates/engine/tests/zz_stall_probe.rs` and three `docs/design/realism/` files) stay staged and uncommitted.
- **D17** (class: implementation-detail): No second-opinion consult ran. No trigger in `_consult-triggers.md` holds (no reviews mode, no significant plan drift, no suppression written), and the product owner excluded `consult` (`00-index.md` `stack.excluded-by-po`).

No decision touched a `carried` intent risk: every RIM in `00-index.md` is `adjudicated`. No lint suppression and no `sdlc-debt:` marker was added.

## Checks Run This Stage

- `cargo fmt --all --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace` (Windows, debug): 58 result lines, 403 passed, 0 failed. New: `install_layout` 2, `release_version` 3.
- `cargo test -p engine-cli --release --test install_layout --test release_version` (Windows): 5 passed.
- The same two tests in WSL (`CARGO_TARGET_DIR=~/.cache/soccermanager-target`): 5 passed.
- `node --test "web/tests/*.test.mjs"`: 127 passed, 0 failed.
- `grep -a -c VCRUNTIME140.dll target/release/engine-cli.exe`: `0` (it was `1` before the change).
- `pwsh packaging/windows/build.ps1`: wrote `SoccerManager-0.1.0-windows-x64-setup.exe` (2,586,951 bytes) and its `.sha256`. The first attempt failed (`Get-Command makensis` under strict mode) and was fixed in `build.ps1`.
- `pwsh packaging/windows/run-sandbox.ps1`, run 1 (`20260923T181519Z`): 10 of 11 checks passed. `hello-version` failed, because the script read `engine_version`, and the wire name is `engine.version` (`crates/protocol/src/message.rs:120-121`). Fixed in `smoke.ps1`.
- `pwsh packaging/windows/run-sandbox.ps1`, run 2 (`20260923T181753Z`): overall PASS, 11 of 11 checks, 4 facts (`vcruntime140-present: False`, `data-folder-before: False`, `installed-version: 0.1.0`, `shortcut-browser: 0`). Evidence: `implement-evidence/distribution/windows/`.
- `wsl -d Ubuntu-24.04 -- sh packaging/unix/build.sh`: wrote `SoccerManager-0.1.0-linux-x86_64.tar.gz` and its `.sha256`.
- `sh packaging/unix/smoke.sh …` in WSL (`20260923T181608Z`): overall PASS, 7 of 7 checks, 3 facts (`ldd`, `glibc 2.39`, `browser: launch.open_failed logged`). Evidence: `implement-evidence/distribution/linux/`.
- Output-boundary search of the added lines of the code commit: no match.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine distribution`. The slice has testable behaviour on two platforms. Verify re-runs `run-sandbox.ps1` (the capability probe is `Test-Path $env:SystemRoot\System32\WindowsSandbox.exe`) and `smoke.sh` in WSL (the probe is `wsl -d Ubuntu-24.04 -- cargo --version`), and applies the pre-registered macOS deferral. Consider compacting the session before `/wf verify`; workflow state lives in artifact files on disk and the SessionStart hook re-reads it after compaction.
- **Option B:** `/wf review football-manager-match-engine distribution`: skip verify. Not recommended, because AC-1 is an observable install experience.
- **Option C:** `/wf plan football-manager-match-engine distribution`: only if the product owner wants the browser to open inside Windows Sandbox, or wants a fallback when no default browser is set. Either change would alter the OQ-2 mechanism.
