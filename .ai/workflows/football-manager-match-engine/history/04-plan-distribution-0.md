---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: distribution
status: awaiting-input
stage-number: 4
created-at: "2026-09-22T22:28:27Z"
updated-at: "2026-09-22T22:28:27Z"
metric-files-to-touch: 13
metric-step-count: 12
has-blockers: true
revision-count: 0
revisions: []
consult-runs: []
tags: [distribution, installer, packaging, deferred, awaiting-input]
stack-source: confirmed
open-questions:
  - id: OQ-1
    class: intent-bearing
    question: "Which operating systems does this slice ship beyond Windows (U-3)?"
    options: [A-windows-only-others-stay-parked, B-windows-plus-linux-macos-deferred, C-windows-linux-and-macos]
  - id: OQ-2
    class: intent-bearing
    question: "How does an installed user start a match: a Start-menu shortcut that runs the launcher, which starts the engine and opens the page; a registered link scheme the page launches; or an engine that starts at sign-in?"
    options: [A-shortcut-runs-launcher-opens-page, B-registered-link-scheme, C-engine-starts-at-sign-in]
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-distribution.md
  siblings: [04-plan-integration.md, 04-plan-viewer-reports-recovery.md, 04-plan-viewer-pitch.md, 04-plan-stream-protocol.md, 04-plan-data-schemas-generator.md]
  implement: 05-implement-distribution.md
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine distribution"
---

# Plan: Distribution and Installers

## The Plan

The game runs today only from a development build. A person builds the engine with Cargo, then runs `engine-cli serve --seed <n> --web web` from the repository root. The engine finds its `content/` folder beside the binary (`crates/engine/src/data/mod.rs:49-77`). It serves the page itself and gives the page its socket port through `/engine.json` (`crates/engine-cli/src/web.rs:37-40`). The reports-and-recovery plan adds `engine-cli launch` before this slice. The launcher serves the page, supervises the engine, and finds the engine binary through `--engine`, then `SM_ENGINE_PATH`, then its own executable (`04-plan-viewer-reports-recovery.md` Steps 5 and 9). Three gaps stop a one-download install. First, the launcher requires `--web <DIR>` and `--seed`. Second, the release binary imports `VCRUNTIME140.dll`, and a clean Windows 11 machine does not carry that file (checked in this run). Third, no installer exists.

For Windows, this plan changes 13 files in 12 steps and adds no new command. The runtime is linked statically. The launcher finds the page folder beside the binary and chooses a seed when none is given. It also gets `--open`, which opens the page in the default browser. A per-user NSIS installer puts the game under `%LOCALAPPDATA%\Programs\SoccerManager` and adds a Start-menu shortcut to `engine-cli.exe launch --open`. NSIS 3.12 is already installed on the reference laptop. The installer file name comes from `engine-cli --version`, and a test holds that string equal to the version in the `hello` message. The clean-machine criterion runs in Windows Sandbox. This Windows 11 feature starts a new, clean Windows image each time, and it is present on this laptop. A smoke script inside the sandbox installs silently, starts the shortcut, and writes its evidence to a mapped folder.

The plan stops before implementation on two product decisions. The first is which operating systems ship beyond Windows (U-3). The slice gives this decision to the product owner. The second is how an installed user starts a match, because a browser page cannot start a native program. The Windows steps below do not change with either answer. The answer to OQ-1 can add a Linux build and smoke-test step, or a pre-registered macOS deferral. The top open risk after that is SmartScreen. It warns on a downloaded installer that is not signed, and the sandbox run does not reproduce that warning.

## Current State

- **Dependency.** `integration` is `defined`, not implemented (`00-index.md` `slices:`). Its plan (`04-plan-integration.md`) is `complete`. This slice cannot start implementation until `integration` is verified (`03-slice-distribution.md` `depends-on`).
- **Launcher (planned, not built).** Step 5 of `04-plan-viewer-reports-recovery.md` adds `Launch(LaunchOpts)` with these options: `--seed`, `--minutes`, `--team-a`, `--team-b`, `--web DIR` (required), and `--engine FILE` (default `SM_ENGINE_PATH`, else `current_exe()`). Step 9 of that plan adds `crates/engine-cli/src/launch.rs`. The launcher prints the page address, and nothing else, on stdout. It shows a first-run state when the engine binary is missing. Once that slice lands, engine discovery needs no configuration.
- **Content discovery.** `ContentDir::resolve` tries the flag, then `SM_CONTENT_DIR`, then `./content`, then `content/` beside the binary (`crates/engine/src/data/mod.rs:49-77`).
- **Page discovery.** On `serve` and `replay`, `--web <DIR>` is optional and has no fallback (`crates/engine-cli/src/cli.rs:118-120`, `web.rs:42-50`). On `launch`, it is required. The socket binds `127.0.0.1:0` and writes `engine.port` to the data folder (`crates/stream/src/server.rs:18,47-58`).
- **Seed.** `serve --seed` is required (`cli.rs:100-103`).
- **Data folder.** The data folder is `SM_DATA_DIR`, else `%LOCALAPPDATA%\SoccerManager` (`crates/engine/src/observe/identity.rs:19-30`). It is created on first use.
- **Version.** `engine::version()` is `CARGO_PKG_VERSION` (`crates/engine/src/lib.rs:54-56`), and `hello.engine_version` uses it (`crates/engine-cli/src/serve.rs:33-35`). The clap `version` attribute reads the `engine-cli` `CARGO_PKG_VERSION` (`cli.rs:9-13`). Both crates inherit the workspace version `0.1.0` (`Cargo.toml` `[workspace.package]`). `target/release/engine-cli.exe --version` prints `engine-cli 0.1.0` (this run).
- **C runtime.** `target/release/engine-cli.exe` (3.7 MB) contains the import name `VCRUNTIME140.dll` and five `api-ms-win-crt-*` import names (`grep -a`, this run). The `api-ms-win-crt` set ships with Windows 10 and 11. `VCRUNTIME140.dll` does not. No `.cargo/config.toml` exists.
- **Build tooling on the reference laptop (this run).**
  - NSIS `v3.12` is at `C:\Program Files (x86)\NSIS\makensis.exe`.
  - Inno Setup, WiX, `cargo-wix`, `cargo-dist`, and `cargo-packager` are absent.
  - Only `x86_64-pc-windows-msvc` and Android targets are installed through rustup.
  - `WindowsSandbox.exe` and `vmms.exe` (Hyper-V) are present in `System32`.
  - Microsoft Edge is installed.
  - WSL `Ubuntu-24.04` is present, with `cargo 1.92.0`, `rustc 1.92.0`, and `gcc`.
  - No git remote exists, so there is no CI.
- **Tests.** The command line has spawn-based tests (`crates/engine-cli/tests/stream_cli.rs:20`, `web_cli.rs:29-66`). Their `bin()`, `temp()`, and `get()` helpers are the pattern for the new tests.

## Simplicity Ladder

- Launching and engine discovery → rung 3 reuse. Reuse `engine-cli launch` (`04-plan-viewer-reports-recovery.md` Steps 5 and 9), which is an exact match. Recommendation: reuse it with modification (optional `--web` and `--seed`, and a new `--open`). The change is backward compatible, because an explicit `--web` or `--seed` behaves as before. A separate `play` command was rejected because it would duplicate the launcher.
- Page-folder discovery → rung 3 reuse (pattern). Mirror `ContentDir::resolve` (`crates/engine/src/data/mod.rs:49-77`): the same order of candidates and the same refusal that names every folder tried. Implement it fresh in `web.rs`, because `ContentDir` checks for `attributes.json`, not `index.html`.
- Static C runtime → rung 2 native-platform. Set the `crt-static` target feature of the MSVC target (Rust Reference, "Static and dynamic C runtimes") in `.cargo/config.toml`. Bundling the VC++ redistributable instead would add a sub-installer that needs elevation.
- Opening the default browser → rung 1 stdlib. Use `std::process::Command::new("cmd").args(["/C", "start", "", url])`. The URL has the form `http://127.0.0.1:<port>/`, so it contains no shell metacharacters. `ShellExecuteW` through `windows-sys` is not used, because it needs a new `Win32_UI_Shell` feature.
- Seed when absent → rung 1 stdlib. Fold the nanoseconds of `SystemTime::now()` into a `u64`. The seed is printed.
- Installer → rung 3 reuse (tool already installed). Use NSIS 3.12, which is already installed. Inno Setup and WiX are absent.
- Version string in the installer → rung 3 reuse. Read it from `engine-cli --version` at build time, so no second literal exists.
- Clean Windows 11 machine → rung 2 native-platform. Use Windows Sandbox with a `.wsb` configuration: `MappedFolders` with `ReadOnly`, `LogonCommand`, and `Networking` set to `Disable` (Microsoft Learn, "Use and configure Windows Sandbox").
- Page screenshot inside the sandbox → rung 2 native-platform. Use Edge `--headless --screenshot --window-size=1280,800`.
- Reading `hello` inside the sandbox → rung 2 native-platform. Use .NET `System.Net.WebSockets.ClientWebSocket` from PowerShell.

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist, and no `.ai/sdlc-config.json` sets a global directory (checked in this run).

Repeat-deferral tripwire: `00-index.md` has `runtime-evidence-deferrals: []`, so no wall repeats.

## Likely Files / Areas to Touch

- `.cargo/config.toml` (new): link the C runtime statically for the MSVC target.
- `crates/engine-cli/src/cli.rs` (modified): make `--seed` and `--web` optional on `launch`, and add `--open`.
- `crates/engine-cli/src/launch.rs` (modified; created by the reports-and-recovery work): find the page folder, choose a seed, and open the browser.
- `crates/engine-cli/src/web.rs` (modified): add `resolve_web_dir`.
- `crates/engine-cli/tests/install_layout.rs` (new): run the installed layout from an unrelated working folder.
- `crates/engine-cli/tests/release_version.rs` (new): check that the `--version` string, the `hello` engine version, and the workspace version are one string, and that the build script derives the installer name.
- `packaging/windows/installer.nsi` (new): the per-user installer, the shortcut, and the uninstaller.
- `packaging/windows/build.ps1` (new): build the release binary, stage the files, and write the versioned setup file and its hash.
- `packaging/windows/smoke.ps1` (new): drive the clean machine and collect its evidence.
- `packaging/windows/run-sandbox.ps1` (new): start the sandbox from the host.
- `packaging/README.md` (new): build, install, smoke test, and the SmartScreen note.
- `.gitignore` (modified): ignore `dist/`.
- `README.md` (modified): add an Install section.

## Proposed Change Strategy

The installed layout describes itself: the binary sits with `content/` and `web/` beside it. The binary finds everything relative to itself, so no configuration file and no registry read are needed. The installer copies files and adds a shortcut, and it holds no logic of its own. For this reason, `crates/engine-cli/tests/install_layout.rs` can prove most of the no-configuration criterion from a temporary folder on the development machine. The sandbox run proves what a temporary folder cannot prove:

- No runtime DLL is missing.
- No state is left over from an earlier installation.
- The shortcut works.

The per-user installation (`RequestExecutionLevel user`) needs no administrator prompt. It matches the data folder under `%LOCALAPPDATA%` (PO answer, data-schemas-generator Q10). The loopback bind avoids a firewall prompt (stream-protocol pre-fill: `127.0.0.1`, never `0.0.0.0`). No NFR is the rationale for a mechanism choice. NFR-9 (Windows 11 first) sets the order, not a mechanism.

The Windows work does not depend on the answer to OQ-1. Only the shortcut target in Step 7 and the shortcut start in Step 9 depend on OQ-2 option A, the recommended option. Implementation starts only after the answers to both questions are recorded.

## Step-by-Step Plan

1. **Link the C runtime statically.**
   - Add `.cargo/config.toml` with `[target.x86_64-pc-windows-msvc] rustflags = ["-C", "target-feature=+crt-static"]`.
   - Rebuild the release binary.
   - Check that the binary no longer contains `VCRUNTIME140.dll` (`grep -a -c`).
   - Check that the Rust and page test suites still pass.
2. **Find the page folder.**
   - In `web.rs`, add `resolve_web_dir(flag: Option<&Path>) -> anyhow::Result<PathBuf>`.
   - Try these folders in order: the flag, `SM_WEB_DIR`, `./web`, and `web/` beside `current_exe()`.
   - Accept a folder only when it holds `index.html`.
   - If no folder is accepted, refuse with a message that lists every folder tried, in the wording of `ContentDir::resolve`.
   - Do not change `serve --web` and `replay --web`: they keep their explicit path.
3. **Extend the launcher.**
   - In `cli.rs`, make `--seed` an `Option<u64>` and `--web` an `Option<PathBuf>` on `LaunchOpts`.
   - Add `--open`, with the help line "Open the page in the default browser."
   - In `launch.rs`, find the page folder with the function from Step 2.
   - If no seed is given, fold the `SystemTime::now()` nanoseconds into a `u64` and print `seed <n>` to stderr. Stdout stays the page address only.
   - After the launcher prints the address, run `cmd /C start "" <address>` if `--open` is set.
   - If the browser does not open, log `launch.open_failed { reason }` and keep the launcher running, because the address is already printed.
   - Do not change worker supervision.
   - Keep every help line under 80 columns (`stream_cli.rs:121`, `cli_args.rs`).
4. **Test the installed layout.** In `crates/engine-cli/tests/install_layout.rs`, write two cases.
   - Case 1 prepares the layout:
     - Copy `target/<profile>/engine-cli.exe`, `content/`, and `web/` into a temporary `install/` folder.
     - Clear `SM_CONTENT_DIR`, `SM_WEB_DIR`, and `SM_ENGINE_PATH`, and set `SM_DATA_DIR` to a temporary folder.
     - Run `install/engine-cli.exe launch --minutes 1`, with the working folder set to a second temporary folder.
   - Case 1 then checks the result:
     - Read the address line.
     - Check that `GET /` returns `index.html`.
     - Poll `GET /engine.json` until `engine.state` is `running` and `socket.port` is present.
     - Connect a `tungstenite` client to that port with Origin `http://127.0.0.1`, and check that the client receives `hello`.
   - Case 2 removes `web/` and checks that the refusal names all four folders.
5. **Test the version.** In `crates/engine-cli/tests/release_version.rs`, check four things:
   - `engine-cli --version` prints `engine-cli <v>`.
   - `hello.engine_version` from `serve`, read over a real socket, equals `<v>`.
   - `<v>` equals the `[workspace.package] version` parsed from `Cargo.toml`.
   - `packaging/windows/build.ps1` contains the `--version` read and no literal version string.
6. **Write the build script.** `packaging/windows/build.ps1` does these tasks:
   - Run `cargo build --release --locked -p engine-cli`.
   - Take `$version` from the second word of `engine-cli --version`. Stop if the value is empty.
   - Stage `dist/stage/` with `engine-cli.exe`, `content/`, `web/` (without `web/tests/`), `LICENSE-MIT`, and `LICENSE-APACHE`.
   - Run `makensis /DVERSION=$version /DSTAGE=<abs> packaging/windows/installer.nsi`. Find `makensis` on `PATH` or in `${env:ProgramFiles(x86)}\NSIS`.
   - Write `dist/SoccerManager-$version-windows-x64-setup.exe` and a `.sha256` file from `Get-FileHash`.
   - Copy `smoke.ps1` into `dist/`.
7. **Write the installer.** Write `packaging/windows/installer.nsi`:
   - Use NSIS 3 with `MUI2.nsh` and `Unicode true`, and set `RequestExecutionLevel user`.
   - Set `InstallDir` to `$LOCALAPPDATA\Programs\SoccerManager`.
   - Show these pages: welcome, licence (from `LICENSE-MIT`), directory, install, and finish. The finish page has a "Start Soccer Manager" checkbox.
   - Copy the staged tree.
   - Create the shortcut `$SMPROGRAMS\Soccer Manager.lnk`. Its target is `"$INSTDIR\engine-cli.exe" launch --open`, its working folder is `$INSTDIR`, and its window is minimised.
   - Write `Uninstall.exe`.
   - Add the key `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\SoccerManager` with `DisplayName`, `DisplayVersion=${VERSION}`, `Publisher`, `UninstallString`, `NoModify`, and `NoRepair`.
   - Make the uninstaller remove `$INSTDIR`, the shortcut, and the key. It keeps `%LOCALAPPDATA%\SoccerManager`, which holds the player's matches, and the finish text says so.
   - NSIS gives a silent install with `/S` by default.
8. **Ignore the build output.** Add `dist/` to `.gitignore`.
9. **Write the clean-machine smoke script.** `packaging/windows/smoke.ps1` runs inside the clean machine. It takes `-Setup <file> -Evidence <dir>` and does these checks in order:
   - (a) Record whether `vcruntime140.dll` is in `System32`. Record this as a fact, not a precondition.
   - (b) Run `setup.exe /S` and record the exit code.
   - (c) Check that the installed files and the shortcut exist.
   - (d) Start the shortcut `.lnk`, which is the user's path.
     - Wait up to 60 seconds for `%LOCALAPPDATA%\SoccerManager\engine.port`.
     - Wait up to 120 seconds for a kick-off record in a `matches\*\events.jsonl` file. The engine writes this record only after a viewer connects, so the record shows that the page found the engine and the match started.
     - Take a desktop screenshot with `System.Drawing` `CopyFromScreen`.
   - (e) Stop the launcher and its worker. Run `engine-cli.exe launch --minutes 1` from `$INSTDIR` without `--open`. Take a headless Edge screenshot at 1280×800 of the printed address.
   - (f) Run a third `launch` without `--open`.
     - Read the socket port from `/engine.json`.
     - Read `hello` with `ClientWebSocket`, using Origin `http://127.0.0.1`.
     - Compare `engine_version` with the output of `engine-cli.exe --version`.
   - (g) Run the uninstaller silently. Check that `$INSTDIR` no longer exists.

   The script writes `results.json`, with one entry per check (`name`, `pass`, `detail`) and an overall `pass`. It exits non-zero if any check fails.
10. **Write the sandbox launcher.** `packaging/windows/run-sandbox.ps1` does these tasks:
    - Write `dist/smoke.wsb` with absolute paths, because the sandbox does not support relative paths.
    - Map `dist/` read-only to `C:\smoke\in`, and map `dist/evidence/<utc>/` writable to `C:\smoke\out`.
    - Set `Networking` to `Disable`, because the game uses loopback only.
    - Set `LogonCommand` to `powershell -ExecutionPolicy Bypass -File C:\smoke\in\smoke.ps1 -Setup … -Evidence C:\smoke\out`.
    - Start the `.wsb` and poll for up to 10 minutes for `results.json`.
    - Print the verdict and exit with its code.
    - If `WindowsSandbox.exe` is absent, exit with code 3 and this message: "Windows Sandbox is not enabled; see packaging/README.md for the Hyper-V fallback".
11. **Write the documentation.**
    - `packaging/README.md` covers these topics:
      - the prerequisites (Rust and NSIS 3)
      - `build.ps1`
      - what the setup file installs, and where
      - the Start-menu entry
      - uninstall (the match data stays)
      - `run-sandbox.ps1` and its evidence folder
      - the Hyper-V fallback
      - the SmartScreen warning for an unsigned download ("More info → Run anyway")
    - In `README.md`, add an "Install" section. Document `engine-cli launch --open` for a development build.
    - If `docs/reference/cli.md` exists (integration Step 13), add the new `launch` options there, so that `docs.rs` continues to pass.
12. **Run the whole path once on the reference laptop.**
    - Run `packaging/windows/build.ps1`, then `run-sandbox.ps1`.
    - Keep `dist/evidence/<utc>/` for verify.
    - Fix each failure in the file that owns it.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| On a clean Windows 11 machine, the installer runs and the user opens the page. The engine is found and a match starts without configuration (observable). | Windows Sandbox drive (infra-1): `run-sandbox.ps1` runs `smoke.ps1` and collects a desktop screenshot, a headless Edge screenshot, the kick-off record, and `results.json`. The development-machine proxy is `install_layout.rs` (cli-1). | A clean Windows 11 image. Windows Sandbox (`WindowsSandbox.exe`) is present on the reference laptop, and each launch is a fresh image. Yes, but not launched in this run. Hyper-V (`vmms.exe`) is present for the fallback. | Steps 1–10: the static runtime, the launcher options, page-folder discovery, the installer, the smoke script, and the sandbox launcher. `results.json` is the evidence seam. | Windows Sandbox → Hyper-V VM from the Microsoft Windows 11 evaluation image, running the same `smoke.ps1` → pre-registered deferral (below) |
| A release build's version string matches the engine `hello` message (not observable). | `cargo test -p engine-cli --release --test release_version` (cli-1), plus smoke check (f) on the installed build | The reference laptop. Yes. | `release_version.rs` and smoke check (f) | None needed |

- AC-1 (clean machine): `constraint-resolution: prerequisite-slice: distribution`. Steps 9 and 10 scope the harness into this slice, and Windows Sandbox is native to the host. `wall-ownership: environment-negotiable`.
  - Pre-registered deferral: it applies if verify finds that Windows Sandbox is disabled. Clearing event: "the operator enables the Windows Sandbox optional feature, or creates a Hyper-V VM from the Windows 11 evaluation image, and runs `packaging/windows/run-sandbox.ps1` (or `smoke.ps1` inside the VM) against the built setup file".
  - Verify runs this capability probe first: `Test-Path $env:SystemRoot\System32\WindowsSandbox.exe`.
- Other operating systems depend on OQ-1 and are not resolved here.
  - With OQ-1 option B or C: the Linux row builds in `wsl -d Ubuntu-24.04` and runs a headless smoke test there (`launch`, plus a `curl` of the page and of `/engine.json`). WSL is present, with cargo 1.92.0 and gcc (checked in this run).
  - With OQ-1 option C: the macOS row needs a pre-registered deferral that names the provisioning step (a Mac or a macOS CI runner). No macOS environment or Apple toolchain exists.

## Test / Verification Plan

### Automated checks

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo test --workspace` and `node --test web/tests/`.
- `cargo test -p engine-cli --release --test release_version --test install_layout`.
- Static-runtime check: `grep -a -c VCRUNTIME140.dll target/release/engine-cli.exe` returns `0`.
- `packaging/windows/build.ps1` exits 0. It writes the setup file with the version in its name, and the setup file's `.sha256`.

### Interactive verification (human-in-the-loop)

- **AC-1 clean machine.**
  - Platform: Windows (cli + web).
  - Tool: Windows Sandbox, run through `packaging/windows/run-sandbox.ps1`. This is a native OS feature, not a new driver.
  - Browser: inside the sandbox, the page opens in the machine's default browser. The headless screenshot uses Edge.
  - Steps:
    1. Run `pwsh packaging/windows/build.ps1`.
    2. Run `pwsh packaging/windows/run-sandbox.ps1`.
    3. Read `dist/evidence/<utc>/results.json`, and look at `desktop.png` and `page.png`.
  - Pass criteria:
    - Every check in `results.json` passes.
    - The desktop screenshot shows the page in the browser with the score bug.
    - `events.jsonl` holds a kick-off record.
    - The `hello` version equals the installed `--version`.
  - Evidence: copy `dist/evidence/<utc>/` to `verify-evidence/distribution/`.
- Companion skills: none from `stack.available-skills` are needed. The Claude_Browser pane cannot see inside the sandbox, so it is not used for AC-1.

## Risks / Watchouts

- **SmartScreen on an unsigned download.** Code signing is out of scope beyond what the platform requires to run. SmartScreen warns but does not block. The sandbox copy has no Mark of the Web, so verify does not see the warning. `packaging/README.md` documents the warning.
- **Antivirus heuristics on NSIS installers.** Some scanners flag unsigned NSIS setup files. Windows Defender is on in the sandbox, so the smoke run exercises it.
- **Default browser inside the sandbox.** The Edge first-run page can open in front of the game page. The pass signal for "a match starts" is the kick-off record. The screenshot is supporting evidence only.
- **One match per launch.** The worker ends after full time (`serve.rs:115-150`), and the launcher reports `finished`. To start a second match, the user starts the shortcut again. The README says this.
- **The launcher is not built yet.** Steps 3 and 4 bind to `launch.rs` and `LaunchOpts` as the reports-and-recovery plan describes them. Run an auto-review of this plan after that slice is verified. Re-read the launcher options before Step 3.
- **`crt-static` for test builds.** The flag applies to every MSVC build in the workspace, including the `criterion` benches. Benchmark numbers can move slightly. Verify compares them against the tripwires as usual.
- **Output boundary.** Script comments, installer strings, and README text use product language only.

## Dependencies on Other Slices

- `integration`: the complete, verified product (`03-slice-distribution.md`). The installer packages whatever `web/` and `content/` hold at that point, without `web/tests/`.
- `viewer-reports-recovery`, reached through `integration`: it creates `engine-cli launch`, `launch.rs`, and `tests/launch.rs`. Steps 3 and 4 extend them.
- `integration`: `docs/reference/cli.md` and `docs.rs` (`04-plan-integration.md` Step 13) must list the new `launch` options. Step 11 adds them.

## Assumptions

- **A1** (class: implementation-detail): NSIS 3.12 is the installer compiler.
  - Why: NSIS is already installed. Inno Setup and WiX are absent, and WiX needs a .NET tool install. The installer format is not a user contract, because the user sees a setup `.exe` in every case.
  - Stack: `stack.build` lists only `cargo`. NSIS is a packaging tool on the reference laptop, not a verify driver. Adding it to `stack.build` is recorded for the PO answer round.
- **A2** (class: implementation-detail): The install is per user, into `%LOCALAPPDATA%\Programs\SoccerManager`, with no administrator prompt. Why: this has the smallest blast radius, and it matches the data folder under `%LOCALAPPDATA%` (data-schemas-generator Q10).
- **A3** (class: implementation-detail, ac: "clean Windows 11 machine … match starts without configuration", classification: runtime-evidence): The clean machine is Windows Sandbox. The fallback is a Hyper-V VM, then a pre-registered deferral. Why: the feature is present on the host, each launch is a clean Windows 11 image, and `.wsb` makes it scriptable.
- **A4** (class: implementation-detail): The C runtime is linked statically through `.cargo/config.toml`. Why: the current binary imports `VCRUNTIME140.dll` (checked in this run). Bundling the redistributable would need elevation and a second installer.
- **A5** (class: implementation-detail): The page folder is found in this order: the flag, `SM_WEB_DIR`, `./web`, then `web/` beside the binary. Why: this mirrors content discovery exactly.
- **A6** (class: implementation-detail): This plan extends the planned `engine-cli launch` with optional `--seed` and `--web` and a new `--open`, instead of adding a `play` command. Why: the launcher already owns page serving, engine discovery, and the first-run state (`04-plan-viewer-reports-recovery.md` Steps 5 and 9), so a second entry point would duplicate it. The change is additive, and explicit flags behave as before.
- **A7** (class: implementation-detail): When no seed is given, `launch` derives it from the clock and prints it. Why: a player does not have to supply a seed, and the printed seed keeps the match reproducible.
- **A8** (class: implementation-detail, ac: "version string matches the engine hello message", classification: build-capability): The version has one source. The workspace `version` feeds `--version`, `hello.engine_version`, the installer file name, and `DisplayVersion`, and a test holds them equal.
- **A9** (class: implementation-detail): The release artifacts are `SoccerManager-<version>-windows-x64-setup.exe` and a `.sha256` file, in `dist/`, which git ignores. Why: this is the "versioned release artifacts" scope line, with nothing extra.
- **A10** (class: implementation-detail): The uninstaller keeps `%LOCALAPPDATA%\SoccerManager`. Why: deleting a player's matches cannot be undone. The finish text says where the matches stay.
- **A11** (class: implementation-detail): This run does not update the master `04-plan.md`. Why: this plan is `awaiting-input`, and the sibling plans that are awaiting input are not listed there either. The master is updated when this plan completes.
- **A12** (class: implementation-detail): No second-opinion consult ran, although `appetite-medium-or-larger` and `unknowns-present` hold. Why: the product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`).

## Blockers

- **OQ-1: operating systems beyond Windows (U-3).** Class: intent-bearing. It changes the user-visible scope, and the slice records it as a PO decision: "macOS and Linux builds as the product owner decides at plan". The options:
  - **A.** Windows only. macOS and Linux stay parked as a later named slice. This narrows the slice's In-scope line, so the PO must ratify it.
  - **B (recommended).** Windows, plus a Linux `x86_64` `.tar.gz` with a `soccermanager` launcher script. The Linux build is built and smoke-tested natively in the WSL `Ubuntu-24.04` install on the reference laptop (cargo 1.92.0 and gcc present). macOS gets a pre-registered deferral. Its clearing event: "an operator provides a Mac or a macOS CI runner and runs the build and smoke scripts". This option adds about 3 files and 2 steps.
  - **C.** Windows, Linux, and macOS in this slice. Cross-compiling for Apple targets from Windows needs the Apple SDK and a linker, and neither is present (`clang` and `zig` are absent, `00-index.md` `stack.toolchains-absent`). macOS verification would be a deferral from the start.

  No git remote or CI exists, so every build is local.
- **OQ-2: how an installed user starts a match.** Class: intent-bearing. It is user-visible behaviour, and the slice wording "launch from the page" cannot be built as written, because a browser page cannot start a native program. The options:
  - **A (recommended).** A Start-menu shortcut runs `engine-cli launch --open`. The launcher starts the engine and opens the page in the default browser. This keeps viewer-pitch Q1 ("the engine serves the page") and reuses the launcher from the reports-and-recovery plan. Steps 3, 7, and 9 build this option.
  - **B.** The installer registers a `soccermanager:` link scheme, and a bookmarked page link starts the engine. This adds a registry contract, and the browser asks for confirmation on every start.
  - **C.** The engine starts at sign-in and waits in the background on a fixed port. This conflicts with stream-protocol Q9, which uses a port that the operating system assigns, and adds a process that always runs.

The plan is complete for Windows except for these two answers. Implementation does not start until both answers are in `po-answers.md`.

## Freshness Research

- **Rust Reference, "Linkage — Static and dynamic C runtimes"** (doc.rust-lang.org/reference/linkage.html, fetched in this run). The MSVC targets link the C runtime dynamically by default. `-C target-feature=+crt-static` selects the static runtime, and `RUSTFLAGS` can set it. A `rustflags` entry in `.cargo/config.toml` is the checked-in equivalent.
- **Microsoft Learn, "Use and configure Windows Sandbox"** (updated 2026-03-31, fetched in this run).
  - A `.wsb` file supports `MappedFolders` (with `HostFolder`, `SandboxFolder`, and `ReadOnly`), `LogonCommand`, `Networking`, `vGPU`, and `MemoryInMB`.
  - Relative host paths are not supported.
  - Writes to a writable mapped folder stay after the sandbox closes. The sandbox discards everything else.
  - The default account is `WDAGUtilityAccount`, an administrator.
  - The window size cannot be configured. For this reason, the page screenshot uses headless Edge at a fixed 1280×800.
- **NSIS.** The installed version reports `v3.12` (`makensis -VERSION`, this run). `MUI2.nsh`, `MultiUser.nsh`, `FileFunc.nsh`, and `x64.nsh` are present in `Include/`. This run did not check the latest upstream release, because the PO excluded the web-search tool. The version is recorded as installed, not as the latest.
- **Dependencies.** No new crate dependency is added. `tungstenite` 0.30 is already in the workspace and serves as the test client (`Cargo.toml` `[workspace.dependencies]`).

## Recommended Next Stage

- **Option A (after the PO answers OQ-1 and OQ-2):** `/wf plan football-manager-match-engine distribution <answers>`. This directed fix records the answers and adds the Linux or macOS rows if they are chosen. After that, run `/wf implement football-manager-match-engine distribution` once `integration` is verified.
- **Option B:** `/wf slice football-manager-match-engine`, if the PO wants macOS and Linux as their own slice (OQ-1 option A with a new slice).
