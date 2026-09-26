---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: distribution
status: complete
stage-number: 4
created-at: "2026-09-22T22:28:27Z"
updated-at: "2026-09-23T08:41:11Z"
metric-files-to-touch: 16
metric-step-count: 14
has-blockers: false
revision-count: 2
revisions:
  - rev: 1
    at: "2026-09-23T07:03:04Z"
    trigger: manual
    because: "auto-review — 5 issues found"
    changed: "page-folder discovery matches content discovery; stale citations and binary facts refreshed; OQ-1 and OQ-2 still open"
  - rev: 2
    at: "2026-09-23T08:41:11Z"
    trigger: answers-returned
    because: "product-owner answers OQ-1 = B and OQ-2 = A recorded 2026-09-23T08:37:28Z; auto-review — 6 issues found"
    changed: "Linux archive build and smoke steps and a macOS pre-registered deferral added; --open opens the browser per platform; install-layout test can target a packaged folder; stale citations refreshed; status complete"
consult-runs: []
tags: [distribution, installer, packaging, deferred, windows, linux]
stack-source: confirmed
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-distribution.md
  siblings: [04-plan-integration.md, 04-plan-viewer-reports-recovery.md, 04-plan-viewer-pitch.md, 04-plan-stream-protocol.md, 04-plan-data-schemas-generator.md]
  implement: 05-implement-distribution.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine distribution"
---

# Plan: Distribution and Installers

## The Plan

The game runs today only from a development build. A person builds the engine with Cargo, then runs `engine-cli serve --seed <n> --web web` from the repository root. The engine finds its `content/` folder beside the binary (`crates/engine/src/data/mod.rs:55-82`). It serves the page itself and gives the page its socket port through `/engine.json` (`crates/engine-cli/src/web.rs:39-43`). The reports-and-recovery plan adds `engine-cli launch` before this slice. The launcher serves the page, supervises the engine, and finds the engine binary through `--engine`, then `SM_ENGINE_PATH`, then its own executable (`04-plan-viewer-reports-recovery.md` Steps 5 and 9). Three gaps stop a one-download install. First, the launcher requires `--web <DIR>` and `--seed`. Second, the Windows release binary imports `VCRUNTIME140.dll`, and a clean Windows 11 machine does not carry that file (checked in this run). Third, no installer or archive exists.

The product owner answered both open questions on 2026-09-23. The slice ships Windows and a Linux x86_64 archive; macOS is a pre-registered deferral (OQ-1, option B). An installed user starts a match from a Start-menu shortcut that runs `engine-cli launch --open` (OQ-2, option A). So this plan changes 16 files in 14 steps and adds no new command. The Windows runtime is linked statically. The launcher finds the page folder beside the binary, chooses a seed when none is given, and with `--open` opens the page in the default browser on each platform. A per-user NSIS installer puts the game under `%LOCALAPPDATA%\Programs\SoccerManager` with the shortcut. Windows Sandbox is the clean Windows 11 machine, and a smoke script inside it installs silently, starts the shortcut, and writes evidence to a mapped folder. The Linux archive is built and smoke-tested in the WSL `Ubuntu-24.04` install on the reference laptop, with a `soccermanager` start script as the Linux equivalent of the shortcut. One version string names every artifact, and a test holds it equal to the version in the `hello` message.

This enables the implement stage once `integration` is verified. The top open risk is SmartScreen: it warns on a downloaded installer that is not signed, and the sandbox run does not reproduce that warning. The Linux archive needs glibc 2.39 or newer, because WSL Ubuntu 24.04 builds it.

## Current State

- **Dependency.** `integration` is `defined`, not implemented (`00-index.md` `slices:`). Its plan (`04-plan-integration.md`) is `complete`. This slice cannot start implementation until `integration` is verified (`03-slice-distribution.md` `depends-on`).
- **Launcher (planned, not built).** Step 5 of `04-plan-viewer-reports-recovery.md` adds `Launch(LaunchOpts)` with these options: `--seed`, `--minutes`, `--team-a`, `--team-b`, `--web DIR` (required), and `--engine FILE` (default `SM_ENGINE_PATH`, else `current_exe()`). Step 8 of that plan changes `web::start` to take a status source. Step 9 adds `crates/engine-cli/src/launch.rs`. The launcher prints the page address, and nothing else, on stdout. `launch.rs` does not exist yet (checked in this run).
- **Content discovery.** `ContentDir::resolve` uses the flag alone when it is given, else `SM_CONTENT_DIR` alone when it is set. Only when neither is present does it try `./content`, then `content/` beside the binary. The refusal lists the folders tried (`crates/engine/src/data/mod.rs:55-82`).
- **Page discovery.** On `serve` and `replay`, `--web <DIR>` is optional and has no fallback (`crates/engine-cli/src/cli.rs:122,157`; `web.rs:43`). On `launch`, it will be required. The socket binds `127.0.0.1:0` and writes `engine.port` to the data folder (`crates/stream/src/server.rs:18,46-48`).
- **Seed.** `serve --seed` is required (`cli.rs:107`).
- **Data folder.** The data folder is `SM_DATA_DIR`, else `%LOCALAPPDATA%\SoccerManager`, else `$HOME/.local/share/SoccerManager`, else `./SoccerManager` (`crates/engine/src/observe/identity.rs:17-31`). The `$HOME` branch serves the Linux build without change.
- **Version.** `engine::version()` is `CARGO_PKG_VERSION` (`crates/engine/src/lib.rs:63-65`), and `hello.engine_version` uses it (`crates/engine-cli/src/serve.rs:37`). The clap `version` attribute reads the `engine-cli` `CARGO_PKG_VERSION` (`cli.rs:11`). Both crates inherit the workspace version `0.1.0` (`Cargo.toml` `[workspace.package]`).
- **C runtime.** `target/release/engine-cli.exe` (5.1 MB, rebuilt 2026-09-23 08:23) contains the import name `VCRUNTIME140.dll` (`grep -a -c VCRUNTIME140.dll` returns `1`, this run). No `.cargo/config.toml` exists.
- **Portability.** The only platform-specific code is `crates/engine/src/observe/process.rs`, which has `#[cfg(windows)]` measurements and `#[cfg(not(windows))]` fallbacks (lines 7, 35, 61, 66). `windows-sys` is a `cfg(windows)` target dependency (`crates/engine/Cargo.toml:21-22`). Nothing else blocks a Linux build.
- **Build tooling on the reference laptop (this run and the previous revision).**
  - NSIS `v3.12` is at `C:\Program Files (x86)\NSIS\makensis.exe`. Inno Setup, WiX, `cargo-wix`, `cargo-dist`, and `cargo-packager` are absent.
  - `WindowsSandbox.exe` and `vmms.exe` (Hyper-V) are present in `System32`. Microsoft Edge is installed.
  - WSL `Ubuntu-24.04` (version 2) is present, with `cargo 1.92.0`, `rustc 1.92.0`, `gcc 13.3.0`, glibc `2.39`, `curl`, `tar`, `sha256sum`, and `python3`. `xdg-open` is absent (checked in this run).
  - No Apple SDK, `clang`, or `zig` (`00-index.md` `stack.toolchains-absent`). No git remote, so there is no CI.
- **Tests.** The command line has spawn-based tests (`crates/engine-cli/tests/stream_cli.rs`, `web_cli.rs`). A shared `temp(prefix, name)` helper exists in `crates/engine-cli/tests/common/mod.rs:10`. The help-width rule is asserted at `stream_cli.rs:132`.
- **Licences.** `LICENSE-MIT` and `LICENSE-APACHE` are at the repository root.

## Simplicity Ladder

- Launching and engine discovery → rung 3 reuse. Reuse `engine-cli launch` (`04-plan-viewer-reports-recovery.md` Steps 5 and 9), an exact match. Recommendation: reuse with modification (optional `--web` and `--seed`, and a new `--open`). Backward compatible, because an explicit `--web` or `--seed` behaves as before.
- Page-folder discovery → rung 3 reuse (pattern). Mirror `ContentDir::resolve` (`crates/engine/src/data/mod.rs:55-82`). Implement fresh in `web.rs`, because `ContentDir` checks for `attributes.json`, not `index.html`.
- Static C runtime (Windows) → rung 2 native-platform. The `crt-static` target feature of the MSVC target (Rust Reference, "Static and dynamic C runtimes") in `.cargo/config.toml`, scoped to `[target.x86_64-pc-windows-msvc]`.
- Opening the default browser → rung 1 stdlib. `std::process::Command`: `cmd /C start "" <url>` on Windows, `open <url>` on macOS, `xdg-open <url>` elsewhere. The URL is `http://127.0.0.1:<port>/`, with no shell metacharacters. No `opener`-style crate is added.
- Seed when absent → rung 1 stdlib. Fold the nanoseconds of `SystemTime::now()` into a `u64`, and print it.
- Windows installer → rung 3 reuse (tool already installed). NSIS 3.12.
- Linux package → rung 2 native-platform. A `.tar.gz` built with `tar` and a POSIX start script. No `.deb` or AppImage tooling is added.
- Version string in every artifact → rung 3 reuse. Read it from `engine-cli --version` at build time.
- Clean Windows 11 machine → rung 2 native-platform. Windows Sandbox with a `.wsb` configuration (Microsoft Learn, "Use and configure Windows Sandbox").
- Clean Linux user → rung 2 native-platform. A fresh temporary `HOME` and `env -i` inside WSL.
- Page screenshot inside the sandbox → rung 2 native-platform. Edge `--headless --screenshot --window-size=1280,800`.
- Reading `hello` inside the sandbox → rung 2 native-platform. .NET `System.Net.WebSockets.ClientWebSocket` from PowerShell.
- Reading `hello` from the Linux archive → rung 3 reuse. `install_layout.rs` with `SM_INSTALL_UNDER_TEST` pointing at the extracted folder, so no second WebSocket client is written.

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist, and no `.ai/sdlc-config.json` sets a global directory.

Repeat-deferral tripwire: `00-index.md` has `runtime-evidence-deferrals: []`, so no wall repeats.

## Likely Files / Areas to Touch

- `.cargo/config.toml` (new): link the C runtime statically for the MSVC target only.
- `crates/engine-cli/src/cli.rs` (modified): make `--seed` and `--web` optional on `launch`, and add `--open`.
- `crates/engine-cli/src/launch.rs` (modified; created by the reports-and-recovery work): find the page folder, choose a seed, and open the browser per platform.
- `crates/engine-cli/src/web.rs` (modified): add `resolve_web_dir`.
- `crates/engine-cli/tests/install_layout.rs` (new): run the installed layout from an unrelated working folder, on Windows and Linux.
- `crates/engine-cli/tests/release_version.rs` (new): one version string across `--version`, `hello`, the workspace, and both build scripts.
- `packaging/windows/installer.nsi` (new): the per-user installer, the shortcut, and the uninstaller.
- `packaging/windows/build.ps1` (new): build, stage, and write the versioned setup file and its hash.
- `packaging/windows/smoke.ps1` (new): drive the clean machine and collect its evidence.
- `packaging/windows/run-sandbox.ps1` (new): start the sandbox from the host.
- `packaging/unix/build.sh` (new): build, stage, and write the versioned archive and its hash.
- `packaging/unix/soccermanager` (new): the start script at the archive root.
- `packaging/unix/smoke.sh` (new): extract into a fresh home and check the archive.
- `packaging/README.md` (new): build, install, smoke tests, the glibc minimum, the SmartScreen note, and the macOS status.
- `.gitignore` (modified): ignore `dist/`.
- `README.md` (modified): add an Install section.

## Proposed Change Strategy

The installed layout describes itself on every platform: the binary sits with `content/` and `web/` beside it. The binary finds everything relative to itself, so no configuration file, registry read, or environment variable is needed. The installer and the archive copy files and add a start entry; they hold no logic of their own. For this reason, `crates/engine-cli/tests/install_layout.rs` proves most of the no-configuration criterion from a temporary folder, on Windows and in WSL. The sandbox run proves what a temporary folder cannot prove on Windows: no runtime DLL is missing, no state is left from an earlier installation, and the shortcut works. The Linux smoke proves that the packaged archive, run from a fresh home with a cleared environment, serves the page and the engine.

The per-user installation (`RequestExecutionLevel user`) needs no administrator prompt and matches the data folder under `%LOCALAPPDATA%` (data-schemas-generator Q10). The loopback bind avoids a firewall prompt (stream-protocol: `127.0.0.1`, never `0.0.0.0`). No NFR is the rationale for a mechanism choice. NFR-9 (Windows 11 first) sets the order, not a mechanism.

The Linux build uses a target folder inside the WSL file system (`CARGO_TARGET_DIR=$HOME/.cache/soccermanager-target`), so it does not mix Linux objects into the Windows `target/` folder and does not compile across the `/mnt/c` bridge.

## Step-by-Step Plan

1. **Link the Windows C runtime statically.**
   - Add `.cargo/config.toml` with `[target.x86_64-pc-windows-msvc] rustflags = ["-C", "target-feature=+crt-static"]`.
   - Rebuild the release binary. Check that `grep -a -c VCRUNTIME140.dll target/release/engine-cli.exe` returns `0`.
   - Check that the Rust and page test suites still pass.
2. **Find the page folder.**
   - In `web.rs`, add `resolve_web_dir(flag: Option<&Path>) -> anyhow::Result<PathBuf>`.
   - If the flag is given, use only that folder. Else, if `SM_WEB_DIR` is set, use only that folder.
   - If neither is present, try `./web`, then `web/` beside `current_exe()`.
   - Accept a folder only when it holds `index.html`.
   - If no folder is accepted, refuse with a message that lists every folder tried, in the wording of `ContentDir::resolve`.
   - Do not change `serve --web` and `replay --web`.
3. **Extend the launcher.**
   - Re-read `LaunchOpts` and `launch.rs` as landed. If they differ from `04-plan-viewer-reports-recovery.md` Steps 5 and 9, stop and re-run this plan's auto-review.
   - In `cli.rs`, make `--seed` an `Option<u64>` and `--web` an `Option<PathBuf>` on `LaunchOpts`. Add `--open`, with the help line "Open the page in the default browser."
   - In `launch.rs`, find the page folder with the function from Step 2.
   - If no seed is given, fold the `SystemTime::now()` nanoseconds into a `u64` and print `seed <n>` to stderr. Stdout stays the page address only.
   - After the launcher prints the address, if `--open` is set, run `cmd /C start "" <address>` under `cfg(windows)`, `open <address>` under `cfg(target_os = "macos")`, and `xdg-open <address>` otherwise.
   - If the browser does not open (spawn error or non-zero exit), log `launch.open_failed { reason }` and keep running, because the address is already printed.
   - Do not change worker supervision. Keep every help line under 80 columns (`stream_cli.rs:132`, `cli_args.rs`).
4. **Test the installed layout.** In `crates/engine-cli/tests/install_layout.rs`, write two cases. Use `env!("CARGO_BIN_EXE_engine-cli")`, so the binary name is right on each platform.
   - Case 1 prepares the layout:
     - If `SM_INSTALL_UNDER_TEST` is set, use that folder as the layout. Otherwise copy the built binary, `content/`, and `web/` into a temporary `install/` folder.
     - Clear `SM_CONTENT_DIR`, `SM_WEB_DIR`, and `SM_ENGINE_PATH`, and set `SM_DATA_DIR` to a temporary folder.
     - Run `install/engine-cli launch --minutes 1` with the working folder set to a second temporary folder.
   - Case 1 then checks the result:
     - Read the address line. Check that `GET /` returns `index.html`.
     - Poll `GET /engine.json` until `engine.state` is `running` and `socket.port` is present.
     - Connect a `tungstenite` client to that port with Origin `http://127.0.0.1`. Check that `hello` arrives and that its `engine_version` equals the second word of `install/engine-cli --version`.
   - Case 2 (copied layout only) removes `web/` and checks that the refusal names both fall-through folders.
5. **Test the version.** In `crates/engine-cli/tests/release_version.rs`, check these things:
   - `engine-cli --version` prints `engine-cli <v>`.
   - `hello.engine_version` from `serve`, read over a real socket, equals `<v>`.
   - `<v>` equals the `[workspace.package] version` parsed from `Cargo.toml`.
   - `packaging/windows/build.ps1` and `packaging/unix/build.sh` each read `--version` and contain no literal version string.
6. **Write the Windows build script.** `packaging/windows/build.ps1`:
   - Run `cargo build --release --locked -p engine-cli`.
   - Take `$version` from the second word of `engine-cli --version`. Stop if it is empty.
   - Stage `dist/stage/` with `engine-cli.exe`, `content/`, `web/` (without `web/tests/`), `LICENSE-MIT`, and `LICENSE-APACHE`.
   - Run `makensis /DVERSION=$version /DSTAGE=<abs> packaging/windows/installer.nsi`. Find `makensis` on `PATH` or in `${env:ProgramFiles(x86)}\NSIS`.
   - Write `dist/SoccerManager-$version-windows-x64-setup.exe` and a `.sha256` file from `Get-FileHash`. Copy `smoke.ps1` into `dist/`.
7. **Write the installer.** `packaging/windows/installer.nsi`:
   - NSIS 3 with `MUI2.nsh` and `Unicode true`; `RequestExecutionLevel user`; `InstallDir` `$LOCALAPPDATA\Programs\SoccerManager`.
   - Pages: welcome, licence (from `LICENSE-MIT`), directory, install, and finish. The finish page has a "Start Soccer Manager" checkbox.
   - Copy the staged tree. Create `$SMPROGRAMS\Soccer Manager.lnk` with target `"$INSTDIR\engine-cli.exe" launch --open`, working folder `$INSTDIR`, and a minimised window.
   - Write `Uninstall.exe`. Add `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\SoccerManager` with `DisplayName`, `DisplayVersion=${VERSION}`, `Publisher`, `UninstallString`, `NoModify`, and `NoRepair`.
   - The uninstaller removes `$INSTDIR`, the shortcut, and the key. It keeps `%LOCALAPPDATA%\SoccerManager`, which holds the player's matches, and the finish text says so.
   - `/S` gives a silent install.
8. **Ignore the build output.** Add `dist/` to `.gitignore`.
9. **Write the clean-machine smoke script.** `packaging/windows/smoke.ps1` takes `-Setup <file> -Evidence <dir>` and runs in order:
   - (a) Record whether `vcruntime140.dll` is in `System32`, as a fact, not a precondition.
   - (b) Run `setup.exe /S` and record the exit code.
   - (c) Check that the installed files and the shortcut exist.
   - (d) Start the shortcut `.lnk`. Wait up to 60 seconds for `%LOCALAPPDATA%\SoccerManager\engine.port`. Wait up to 120 seconds for a kick-off record in a `matches\*\events.jsonl` file; the engine writes it only after a viewer connects, so it shows that the page found the engine and the match started. Take a desktop screenshot with `System.Drawing` `CopyFromScreen`.
   - (e) Stop the launcher and its worker. Run `engine-cli.exe launch --minutes 1` from `$INSTDIR` without `--open`. Take a headless Edge screenshot at 1280×800 of the printed address.
   - (f) Run a third `launch` without `--open`. Read the socket port from `/engine.json`, read `hello` with `ClientWebSocket` (Origin `http://127.0.0.1`), and compare `engine_version` with `engine-cli.exe --version`.
   - (g) Run the uninstaller silently. Check that `$INSTDIR` no longer exists.
   - Write `results.json` with one entry per check (`name`, `pass`, `detail`) and an overall `pass`. Exit non-zero if any check fails.
10. **Write the sandbox launcher.** `packaging/windows/run-sandbox.ps1`:
    - Write `dist/smoke.wsb` with absolute paths. Map `dist/` read-only to `C:\smoke\in` and `dist/evidence/windows/<utc>/` writable to `C:\smoke\out`. Set `Networking` to `Disable`.
    - Set `LogonCommand` to `powershell -ExecutionPolicy Bypass -File C:\smoke\in\smoke.ps1 -Setup … -Evidence C:\smoke\out`.
    - Start the `.wsb` and poll for up to 10 minutes for `results.json`. Print the verdict and exit with its code.
    - If `WindowsSandbox.exe` is absent, exit with code 3 and the message "Windows Sandbox is not enabled; see packaging/README.md for the Hyper-V fallback".
11. **Write the Linux build and start script.**
    - `packaging/unix/soccermanager` (mode 755): `#!/bin/sh`, then `exec "$(dirname "$0")/engine-cli" launch --open "$@"`.
    - `packaging/unix/build.sh` (POSIX `sh`, `set -eu`):
      - Run `cargo build --release --locked -p engine-cli` with `CARGO_TARGET_DIR` defaulting to `$HOME/.cache/soccermanager-target`.
      - Take the version from the second word of `engine-cli --version`. Stop if it is empty.
      - Name the platform from `uname -s` and `uname -m`: `linux-x86_64` here, `macos-<arch>` on a Mac.
      - Stage `dist/stage-<platform>/SoccerManager-<version>-<platform>/` with `engine-cli`, `content/`, `web/` (without `web/tests/`), both licences, and `soccermanager`.
      - Write `dist/SoccerManager-<version>-<platform>.tar.gz` with `tar -czf`, and a `.sha256` file with `sha256sum` (else `shasum -a 256`).
    - Run it on the reference laptop with `wsl -d Ubuntu-24.04 -- sh packaging/unix/build.sh` from the repository folder.
12. **Write the Linux smoke script.** `packaging/unix/smoke.sh` takes `<archive> <evidence-dir>` and runs in order:
    - (a) Make a fresh `HOME` with `mktemp -d`. Extract the archive into `$HOME/opt`.
    - (b) Record `ldd engine-cli` output and `ldd --version`, when `ldd` exists, as facts.
    - (c) From a second temporary folder, run `env -i HOME=$HOME PATH=/usr/bin:/bin <extracted>/soccermanager --minutes 1` in the background, with stdout and stderr to files. Read the address line within 30 seconds.
    - (d) `curl` the address and check that the body is the page. Poll `/engine.json` with `curl` for up to 60 seconds until `engine.state` is `running` and `socket.port` is present. Check that `$HOME/.local/share/SoccerManager/engine.port` exists. Record whether stderr holds `launch.open_failed` (expected where no browser opener exists).
    - (e) Check that `engine-cli --version` equals the version in the archive name.
    - (f) Stop the launcher. Run `SM_INSTALL_UNDER_TEST=<extracted> cargo test --release -p engine-cli --test install_layout` from the repository folder, which reads `hello` from the packaged binary and compares its version.
    - Write `results.json` in the same shape as the Windows smoke. Exit non-zero if any check fails.
13. **Write the documentation.**
    - `packaging/README.md`: prerequisites (Rust; NSIS 3 on Windows; WSL or a Linux shell for the archive); `build.ps1` and `build.sh`; what each artifact installs and where; the Start-menu entry and `./soccermanager`; uninstall on Windows (match data stays) and removal on Linux (delete the folder; data stays in `~/.local/share/SoccerManager`); `run-sandbox.ps1` and `smoke.sh` with their evidence folders; the Hyper-V fallback; the glibc 2.39 minimum; the SmartScreen warning for an unsigned download ("More info → Run anyway"); macOS is not yet built, and an operator with a Mac runs `build.sh` then `smoke.sh`.
    - In `README.md`, add an "Install" section for both platforms and `engine-cli launch --open` for a development build.
    - If `docs/reference/cli.md` exists (integration Step 13), add the new `launch` options there, so `docs.rs` keeps passing.
14. **Run both paths once on the reference laptop.**
    - Run `packaging/windows/build.ps1`, then `run-sandbox.ps1`. Keep `dist/evidence/windows/<utc>/`.
    - Run `packaging/unix/build.sh`, then `packaging/unix/smoke.sh dist/SoccerManager-<v>-linux-x86_64.tar.gz dist/evidence/linux/<utc>`, both in WSL.
    - In WSL, also run `cargo test -p engine-cli --release --test release_version --test install_layout`.
    - Fix each failure in the file that owns it.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-1: On a clean Windows 11 machine, the installer runs and the user opens the page. The engine is found and a match starts without configuration (observable). | Windows Sandbox drive (infra-1): `run-sandbox.ps1` runs `smoke.ps1` and collects a desktop screenshot, a headless Edge screenshot, the kick-off record, and `results.json`. Development-machine proxy: `install_layout.rs` (cli-1). | A clean Windows 11 image. `WindowsSandbox.exe` is present on the reference laptop; each launch is a fresh image. Yes, but not launched in this run. Hyper-V (`vmms.exe`) is present for the fallback. | Steps 1–10: the static runtime, the launcher options, page-folder discovery, the installer, the smoke script, and the sandbox launcher. `results.json` is the evidence seam. | Windows Sandbox → Hyper-V VM from the Windows 11 evaluation image, running the same `smoke.ps1` → pre-registered deferral (below) |
| AC-2: A release build's version string matches the engine `hello` message (not observable). | `cargo test -p engine-cli --release --test release_version` (cli-1) on Windows and in WSL; smoke check (f) on Windows and `install_layout` against the extracted archive on Linux | The reference laptop and its WSL install. Yes. | `release_version.rs`, the version check in `install_layout.rs`, smoke check (f) | None needed |
| Scope: a smoke test for the Linux build (OQ-1 = B). The archive runs from a fresh home with no configuration (observable: the install experience on Linux). | WSL shell drive (cli-1): `packaging/unix/smoke.sh` in `Ubuntu-24.04`, plus `install_layout.rs` with `SM_INSTALL_UNDER_TEST` | WSL `Ubuntu-24.04` with cargo 1.92.0, gcc, curl, and tar. Yes (checked in this run). | Steps 11–12: the build script, the start script, the smoke script, and the `SM_INSTALL_UNDER_TEST` seam in `install_layout.rs` | Any Linux x86_64 host with glibc ≥ 2.39 running the same `smoke.sh` → pre-registered deferral (below) |
| Scope: macOS build (OQ-1 = B: deferred). | Not built in this slice. Proxy: the platform-neutral `build.sh` and `smoke.sh` pass on Linux (cli-1). | A Mac or a macOS CI runner. No (no Apple hardware, SDK, or linker). | The `uname`-based platform naming in `build.sh`; the `open` branch of `--open` | Pre-registered deferral (below) |

- AC-1 (clean Windows machine): `constraint-resolution: prerequisite-slice: distribution`. Steps 9 and 10 scope the harness into this slice, and Windows Sandbox is native to the host. `wall-ownership: environment-negotiable`.
  - Pre-registered deferral, if verify finds Windows Sandbox disabled. Clearing event: "the operator enables the Windows Sandbox optional feature, or creates a Hyper-V VM from the Windows 11 evaluation image, and runs `packaging/windows/run-sandbox.ps1` (or `smoke.ps1` inside the VM) against the built setup file".
  - Verify capability probe: `Test-Path $env:SystemRoot\System32\WindowsSandbox.exe`.
- Linux smoke: `constraint-resolution: prerequisite-slice: distribution`. Steps 11 and 12 scope the harness; WSL is present on the host. `wall-ownership: environment-negotiable`.
  - Pre-registered deferral, if verify finds WSL unavailable. Clearing event: "the operator runs `wsl --install -d Ubuntu-24.04` (or uses any Linux x86_64 host with glibc 2.39 or newer), then runs `packaging/unix/build.sh` and `packaging/unix/smoke.sh`".
  - Verify capability probe: `wsl -d Ubuntu-24.04 -- cargo --version`.
- macOS build: `constraint-resolution: proxy+deferral: an operator provides a Mac or a macOS CI runner and runs packaging/unix/build.sh then packaging/unix/smoke.sh, copying dist/evidence/ into the verify evidence`. `wall-ownership: external` (Apple hardware and SDK, not code in this repository). The proxy is the Linux run of the same scripts. This is the product owner's answer to OQ-1, recorded in `po-answers.md` at 2026-09-23T08:37:28Z.

## Test / Verification Plan

### Automated checks

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo test --workspace` and `node --test web/tests/`.
- `cargo test -p engine-cli --release --test release_version --test install_layout`, on Windows and in WSL.
- Static-runtime check: `grep -a -c VCRUNTIME140.dll target/release/engine-cli.exe` returns `0`.
- `packaging/windows/build.ps1` exits 0 and writes the setup file with the version in its name and its `.sha256`.
- `packaging/unix/build.sh` exits 0 in WSL and writes the archive with the version in its name and its `.sha256`.

### Interactive verification (human-in-the-loop)

- **AC-1 clean Windows machine.**
  - Platform: Windows (cli + web). Tool: Windows Sandbox, through `packaging/windows/run-sandbox.ps1`, a native OS feature, not a new driver.
  - Steps: (1) `pwsh packaging/windows/build.ps1`; (2) `pwsh packaging/windows/run-sandbox.ps1`; (3) read `dist/evidence/windows/<utc>/results.json` and look at `desktop.png` and `page.png`.
  - Pass: every check in `results.json` passes; the desktop screenshot shows the page with the score bug; `events.jsonl` holds a kick-off record; the `hello` version equals the installed `--version`.
  - Evidence: copy `dist/evidence/windows/<utc>/` to `verify-evidence/distribution/windows/`.
- **Linux archive smoke.**
  - Platform: cli + web in WSL `Ubuntu-24.04`. Tool: `packaging/unix/smoke.sh`, `curl`, and `cargo test`.
  - Steps: (1) `wsl -d Ubuntu-24.04 -- sh packaging/unix/build.sh`; (2) `wsl -d Ubuntu-24.04 -- sh packaging/unix/smoke.sh dist/SoccerManager-<v>-linux-x86_64.tar.gz dist/evidence/linux/<utc>`; (3) read `results.json`.
  - Pass: every check passes; the page body and a `running` `/engine.json` were fetched from the packaged binary; `hello` arrived with the archive's version.
  - Evidence: copy `dist/evidence/linux/<utc>/` to `verify-evidence/distribution/linux/`.
- Companion skills: none from `stack.available-skills` are needed. The Claude_Browser pane cannot see inside the sandbox or WSL, so it is not used here.

## Risks / Watchouts

- **SmartScreen on an unsigned download.** Code signing is out of scope beyond what the platform requires to run. SmartScreen warns but does not block. The sandbox copy has no Mark of the Web, so verify does not see the warning. `packaging/README.md` documents it.
- **Antivirus heuristics on NSIS installers.** Some scanners flag unsigned NSIS setup files. Windows Defender is on in the sandbox, so the smoke run exercises it.
- **Default browser inside the sandbox.** The Edge first-run page can open in front of the game page. The pass signal for "a match starts" is the kick-off record; the screenshot is supporting evidence.
- **glibc floor on Linux.** The archive is built against glibc 2.39 (Ubuntu 24.04). Older distributions refuse the binary. The README states the floor; a static musl build is a later option.
- **No browser opener in WSL.** `xdg-open` is absent, so `soccermanager` logs `launch.open_failed` there and keeps running. This exercises the failure path; the smoke does not depend on a browser.
- **One match per launch.** The worker ends after full time, and the launcher reports `finished`. To start a second match, the user starts the shortcut or script again. The README says this.
- **The launcher is not built yet.** Steps 3 and 4 bind to `launch.rs` and `LaunchOpts` as the reports-and-recovery plan describes them. Step 3 re-reads them first and stops on a difference.
- **`crt-static` for test builds.** The flag applies to every MSVC build in the workspace, including the `criterion` benches. Benchmark numbers can move slightly; verify compares them against the tripwires as usual.
- **Output boundary.** Script comments, installer strings, and README text use product language only.

## Dependencies on Other Slices

- `integration`: the complete, verified product (`03-slice-distribution.md`). The installer and the archive package whatever `web/` and `content/` hold at that point, without `web/tests/`.
- `viewer-reports-recovery`, reached through `integration`: it creates `engine-cli launch`, `launch.rs`, and `tests/launch.rs`. Steps 3 and 4 extend them.
- `integration`: `docs/reference/cli.md` and `docs.rs` (`04-plan-integration.md` Step 13) must list the new `launch` options. Step 13 adds them.

## Assumptions

- **A1** (class: implementation-detail): NSIS 3.12 is the Windows installer compiler. Why: it is installed; Inno Setup and WiX are absent. The user sees a setup `.exe` in every case. NSIS is a packaging tool, not a verify driver, so `stack.build` is not changed.
- **A2** (class: implementation-detail): The Windows install is per user, into `%LOCALAPPDATA%\Programs\SoccerManager`, with no administrator prompt. Why: smallest blast radius, and it matches the data folder (data-schemas-generator Q10).
- **A3** (class: implementation-detail, ac: "clean Windows 11 machine … match starts without configuration", classification: runtime-evidence): The clean machine is Windows Sandbox, with a Hyper-V VM fallback and then a pre-registered deferral. Why: present on the host, each launch is a clean image, and `.wsb` makes it scriptable.
- **A4** (class: implementation-detail): The C runtime is linked statically for the Windows target only, through `.cargo/config.toml`. Why: the release binary imports `VCRUNTIME140.dll` (this run); bundling the redistributable needs elevation.
- **A5** (class: implementation-detail): Page-folder discovery mirrors content discovery: a given flag or `SM_WEB_DIR` is used alone, then `./web`, then `web/` beside the binary. Why: `crates/engine/src/data/mod.rs:55-82`.
- **A6** (class: implementation-detail): The planned `engine-cli launch` is extended with optional `--seed` and `--web` and a new `--open`; no `play` command is added. Why: the launcher already owns page serving, engine discovery, and the first-run state; the change is additive. This is the mechanism the product owner chose in OQ-2 option A.
- **A7** (class: implementation-detail): When no seed is given, `launch` derives it from the clock and prints it. Why: a player does not supply a seed, and the printed seed keeps the match reproducible.
- **A8** (class: implementation-detail, ac: "version string matches the engine hello message", classification: build-capability): One version source. The workspace `version` feeds `--version`, `hello.engine_version`, both artifact names, and `DisplayVersion`, and tests hold them equal.
- **A9** (class: implementation-detail): The release artifacts are `SoccerManager-<version>-windows-x64-setup.exe`, `SoccerManager-<version>-linux-x86_64.tar.gz`, and a `.sha256` for each, in the git-ignored `dist/`. Why: the "versioned release artifacts" scope line with nothing extra.
- **A10** (class: implementation-detail): The Windows uninstaller keeps `%LOCALAPPDATA%\SoccerManager`. Why: deleting a player's matches cannot be undone.
- **A11** (class: implementation-detail): The Linux package is a `.tar.gz` with a POSIX start script, built with the GNU target in WSL Ubuntu 24.04 (glibc 2.39 floor). Why: this is the product owner's OQ-1 option B as written ("a Linux x86_64 `.tar.gz`, built and smoke-tested in WSL Ubuntu-24.04"); a musl build would need a new rustup target, and `.deb` or AppImage tooling is absent.
- **A12** (class: implementation-detail): `--open` uses `cmd /C start` on Windows, `open` on macOS, and `xdg-open` elsewhere, through `std::process::Command`; a failure is logged and not fatal. Why: rung 1 stdlib, no new crate; the macOS branch costs one line and lets the deferred macOS run use the same start script.
- **A13** (class: implementation-detail): The Linux build uses `CARGO_TARGET_DIR=$HOME/.cache/soccermanager-target` inside WSL. Why: it keeps Linux objects out of the Windows `target/` folder and avoids compiling across `/mnt/c`.
- **A14** (class: implementation-detail): The Linux smoke reads `hello` through `install_layout.rs` with a new `SM_INSTALL_UNDER_TEST` test-only variable, not through a second WebSocket client in shell. Why: reuse; the test already connects a `tungstenite` client. The variable is read only by the test, not by the product.
- **A15** (class: implementation-detail): The scripts in `packaging/unix/` are platform-neutral POSIX `sh` that name the platform from `uname`. Why: the macOS deferral's clearing event is then an act ("run these two scripts on a Mac"), not new work.
- **A16** (class: implementation-detail): No second-opinion consult ran, although `appetite-medium-or-larger` holds. Why: the product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`).
- **A17** (class: implementation-detail): This re-run is an auto-review with returned answers, not a new plan, so no discovery interview ran. The review ran inline in this agent, because no sub-agent dispatch tool is available in this run. It re-checked every cited file against the current tree on 2026-09-23 and fixed six issues: OQ-1 and OQ-2 folded in, the Linux build and smoke steps and the macOS deferral added, `--open` made per platform, the install-layout test given a packaged-folder seam and a version check, the help-width citation moved from `stream_cli.rs:121` to `:132`, and the binary size and page-discovery citations refreshed.
- **A18** (class: implementation-detail): The product-owner answers are read from `po-answers.md` (entry 2026-09-23T08:37:28Z) and `steer.md` ("distribution OQ-1 = B … OQ-2 = A"). Why: both are the workflow's recorded answer channels; the two agree.
- **A19** (class: implementation-detail): This run does not edit `00-index.md` or the global index. Why: the run driver records stage state as the single writer. The macOS deferral is pre-registered here; verify adds it to `runtime-evidence-deferrals` when it applies it.

## Blockers

None. OQ-1 and OQ-2 are answered (`po-answers.md`, 2026-09-23T08:37:28Z). Implementation waits only on the `integration` slice being verified, which is the slice's declared dependency, not a planning blocker.

## Freshness Research

- **Rust Reference, "Linkage — Static and dynamic C runtimes"** (doc.rust-lang.org/reference/linkage.html, read in the first revision). MSVC targets link the C runtime dynamically by default; `-C target-feature=+crt-static` selects the static runtime; a `rustflags` entry under a `[target.<triple>]` table in `.cargo/config.toml` applies it to that target only.
- **Microsoft Learn, "Use and configure Windows Sandbox"** (updated 2026-03-31, read in the first revision). `.wsb` supports `MappedFolders` (`HostFolder`, `SandboxFolder`, `ReadOnly`), `LogonCommand`, `Networking`, `vGPU`, and `MemoryInMB`. Relative host paths are not supported. Writes to a writable mapped folder persist after the sandbox closes. The default account is `WDAGUtilityAccount`. The window size cannot be configured, so the page screenshot uses headless Edge at 1280×800.
- **NSIS.** Installed `v3.12` (`makensis -VERSION`). `MUI2.nsh`, `MultiUser.nsh`, `FileFunc.nsh`, and `x64.nsh` are in `Include/`. The latest upstream release was not checked, because the product owner excluded the web-search tool.
- **Linux toolchain (this run).** WSL `Ubuntu-24.04`: `cargo 1.92.0`, `rustc 1.92.0`, `gcc 13.3.0`, glibc `2.39` (`ldd --version`), `x86_64`. A binary linked against glibc 2.39 requires 2.39 or newer at run time.
- **Dependencies.** No new crate dependency. `tungstenite` 0.30 is already in the workspace and serves as the test client (`Cargo.toml` `[workspace.dependencies]`).

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine distribution` — the plan is complete, but implementation starts only after `integration` is verified (declared dependency). Compact the session first; the artifacts are re-read after compaction.
- **Option C:** `/wf slice football-manager-match-engine` — only if the product owner later wants macOS as its own slice instead of a deferral.
