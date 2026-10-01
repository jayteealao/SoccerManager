# Packaging

This folder builds the release files a player downloads, and checks each one on a clean machine.

| Platform | Release file | Start |
|---|---|---|
| Windows 11 x64 | `dist/SoccerManager-<version>-windows-x64-setup.exe` | The **Soccer Manager** entry in the Start menu |
| Linux x86_64 | `dist/SoccerManager-<version>-linux-x86_64.tar.gz` | `./soccermanager` in the unpacked folder |
| macOS | Not built yet (see [macOS](#macos)) | |

Each release file has a `.sha256` file beside it. The version in every file name is the version `engine-cli --version` prints, which is also the version in the engine's `hello` message. A test holds these equal (`crates/engine-cli/tests/release_version.rs`).

## What a release holds

Every release holds one folder:

- `engine-cli` (`engine-cli.exe` on Windows): the engine and the launcher.
- `content/`: the attributes, tuning, rules, teams, commentary, and script packs.
- `web/`: the match viewer, as the viewer's release build (`npm run build:release` in `viewer/`): the viewer page, the handshake check page, their scripts and fonts, and the font licence texts in `web/fonts/`.
- `LICENSE-MIT` and `LICENSE-APACHE`.
- `soccermanager` (Linux and macOS only): the start script.

The program finds `content/` and `web/` beside itself, so an installed game needs no path, no flag, and no environment variable. The start entry runs `engine-cli launch --open`. The launcher picks a seed from the clock, starts the engine, serves the page, and opens it in the default browser. Each start plays one match. To play again, start the game again.

The Windows program links the C runtime statically (`.cargo/config.toml`), so it runs on a machine without the Visual C++ redistributable.

## Prerequisites

- Rust 1.87 or later.
- Node 22.12 or later, with `npm`. Both build scripts run `npm ci` and `npm run build:release` in `viewer/` and copy `viewer/dist-release/` to `web/`.
- Windows: NSIS 3 (`makensis` on `PATH`, or installed in `Program Files (x86)\NSIS`), and PowerShell 7 or Windows PowerShell 5.1.
- Linux archive: a Linux shell with Rust, `tar`, `curl`, and `sha256sum`. On Windows, WSL with Ubuntu 24.04 works.

## Build

Windows setup file:

```powershell
pwsh packaging/windows/build.ps1
```

To build only the installed layout, in `dist/stage`, without the setup file (no NSIS needed):

```powershell
pwsh packaging/windows/build.ps1 -StageOnly
```

Linux archive (from the repository folder, in a Linux shell or WSL):

```bash
sh packaging/unix/build.sh
# on Windows:
wsl -d Ubuntu-24.04 -- sh packaging/unix/build.sh
```

The Linux build uses the target folder `~/.cache/soccermanager-target` (set `CARGO_TARGET_DIR` to change it), so Linux objects stay out of the Windows `target/` folder.

The archive is built against the glibc of the build machine. An archive built on Ubuntu 24.04 needs glibc 2.39 or later on the player's machine; an older distribution refuses to start the program.

## Install and remove

**Windows.** Run the setup file. It installs for the current user into `%LOCALAPPDATA%\Programs\SoccerManager` with no administrator prompt, and adds the Start-menu entry. `/S` installs silently. To remove the game, use **Settings > Apps**, or run `Uninstall.exe /S` in the install folder. The uninstaller removes the program and keeps the player's matches in `%LOCALAPPDATA%\SoccerManager`.

The setup file is not signed. A browser download carries the Mark of the Web, so SmartScreen warns before it runs: choose **More info**, then **Run anyway**. Some antivirus products also flag unsigned installers.

**Linux.** Unpack the archive anywhere and run `./soccermanager`. Extra arguments go to `engine-cli launch`, for example `./soccermanager --seed 42`. When no browser opener (`xdg-open`) is available, the program logs `launch.open_failed` and keeps running; open the address it prints. To remove the game, delete the folder. Matches stay in `~/.local/share/SoccerManager`.

## Check a release on a clean machine

### Windows: Windows Sandbox

```powershell
pwsh packaging/windows/build.ps1
pwsh packaging/windows/run-sandbox.ps1
```

`run-sandbox.ps1` starts Windows Sandbox, a fresh Windows 11 image, with networking off. It maps `dist/` read-only and a new evidence folder `dist/evidence/windows/<utc>/` writable. Inside the sandbox, `smoke.ps1`:

1. records whether the Visual C++ runtime is present, as a fact;
2. installs silently and checks the installed files (with the font licence texts) and the Start-menu entry;
3. starts the Start-menu entry, checks that the engine runs, and takes `desktop.png`;
4. runs `engine-cli launch` from the install folder, checks that the page's module script is served as JavaScript, and takes `page.png` with headless Microsoft Edge at 1280 by 800;
5. runs a third launch, reads the engine's `hello` over the socket, compares its version with `engine-cli --version`, starts the match, and waits for the kick-off record in `events.jsonl`;
6. uninstalls silently and checks that the program is gone and the matches stay.

Windows Sandbox has no working handler for web links: any `http` link, from any program, shows "We can't open this 'http' link". So the Start-menu entry starts the engine there, but the browser does not open. The check records the number of browser processes as a fact, not as a pass or a failure. On a Windows 11 machine with a default browser, the page opens in that browser.

It writes `results.json`, with one entry per check and an overall `pass`. `run-sandbox.ps1` prints the checks and exits 0 on a pass, 1 on a failure, 2 when no results arrive within ten minutes, and 3 when Windows Sandbox is not enabled. Close the sandbox window afterwards; the image is discarded.

**Hyper-V fallback.** When Windows Sandbox is not available, create a Hyper-V virtual machine from the Windows 11 evaluation image, copy the setup file and `smoke.ps1` into it, and run:

```powershell
powershell -ExecutionPolicy Bypass -File smoke.ps1 -Setup <setup file> -Evidence <folder>
```

### Linux: a fresh home

```bash
sh packaging/unix/smoke.sh dist/SoccerManager-<version>-linux-x86_64.tar.gz dist/evidence/linux/<utc>
```

`smoke.sh` unpacks the archive into a fresh temporary home and starts `soccermanager` from an unrelated folder with only `HOME` and `PATH` set. It checks the page address, the page, that the page's module script is served as JavaScript, that `web/fonts/OFL-Saira.txt` ships, `engine.json` reporting a running engine, `engine.port` in the fresh home's data folder, and the program's version against the archive name. Then it runs the install-layout test against the unpacked folder (`SM_INSTALL_UNDER_TEST`), which reads the engine's `hello` and compares its version. It writes `results.json` in the same shape as the Windows check.

### The browser suite against a packaged folder

The browser suite in `e2e/` can drive the packaged program and its `web/` instead of the repository build. Point `SM_E2E_INSTALL` at the unpacked or staged folder:

```bash
SM_INSTALL_UNDER_TEST=<folder> cargo test --release --locked -p engine-cli --test install_layout
cd e2e && SM_E2E_INSTALL=<folder> npx playwright test --project=viewer
```

The release workflow runs both against the Linux archive and the Windows staged folder before it drafts a release.

## macOS

macOS is not built yet. The Unix scripts name the platform from `uname`, and `--open` uses `open` on macOS, so the same two scripts are expected to work there. An operator with a Mac runs:

```bash
sh packaging/unix/build.sh
sh packaging/unix/smoke.sh dist/SoccerManager-<version>-macos-<arch>.tar.gz dist/evidence/macos/<utc>
```

A macOS release also needs code signing and notarization before Gatekeeper lets a downloaded program run; that is not part of these scripts.
