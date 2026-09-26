---
schema: sdlc/v1
type: brainstorm
slug: brainstorm-app-packaging-options-20260923
topic: "what are our app packaging options is unity an option for us with html assets, is electron or taurior something else an option"
status: distilled
board: brainstorm-board.json
page: "https://claude.ai/artifact/84Qd7oJcnUWzUnfw6y1aR1"
created-at: "2026-09-23T21:55:55Z"
updated-at: "2026-09-23T22:45:49Z"
sessions: 1
batches: 19
consult-runs: []
revisions:
- rev: 1
  at: "2026-09-23T22:37:31Z"
  trigger: manual
  because: done
  changed: the person chose done after every area was explored; scoping the work begins
---

# Brainstorm: app packaging options

## The Brainstorm
We started from one question: how should Touchline reach a player’s desktop? Today the installed game starts the engine and opens the match page in the player’s default browser tab. The candidates were Unity, Electron, Tauri, and others.

We now believe that reach, not the browser tab, drives the choice. The next release is one app window that can grow to Steam and to touch. The choice leans to Tauri 2, with Electron as the fallback. Unity, Godot, and Bevy are off the list: the Unity HTML plugin has no Linux build, and a rebuilt page loses every page test. The page talks to the engine through one interface with two paths: direct calls in the app, and the socket in a plain browser for tests, development, and replays. Engine errors are caught at the boundary, so a match restarts from the last stoppage.

Around the shell, the app updates itself and registers the replay file type. Saves stay in today’s folder. Signing waits until the game has players, and the Mac app ships unsigned. Hosted CI also waits, so the first trial proves Windows and Linux only: a full match, a suite test in the app, and the same event stream on both paths. The sharpest risk is the Mac: its web engine differences show up late, and most Mac players give up at the unsigned-app block.

## Map
### What feels wrong with a browser tab today — problem side — explored
Decided: every pain applies, but reach drives the choice. Scope: kept.

### Which platforms and stores the game must reach — problem side — explored
Decided: Steam, macOS, Linux, and touch later are targets; Steam is someday; touch does not shape the choice now. Scope: kept.

### Install, trust, and updates — problem side — explored
Decided: the app updates itself; signing comes later on both systems; the Mac app ships unsigned with steps to allow it; saves stay in today’s folder. Core: all three. Scope: kept.

### Stay in the browser, but make it feel like an app — solution side — explored
A chromeless browser window or an installable web app is off the list. Scope: kept.

### A thin shell on the system web view (Tauri and similar) — solution side — explored
The choice leans to Tauri 2. Core: lean Tauri. Open: confirm it with a trial against the three proofs. Scope: kept.

### A shell that ships its own browser (Electron and similar) — solution side — explored
Electron is the fallback: one browser and one test tool everywhere, but a large download and no touch. Scope: kept.

### A game engine as the host (Unity, Godot) — solution side — explored
Unity, Godot, and Bevy are off the list: no Linux HTML plugin for Unity, and a rebuilt page loses every page test. 3D is a wish. Scope: kept.

### How the engine and the page talk inside a package — both sides — explored
Two paths behind one interface: direct calls in the app, the socket in a plain browser. Core: two paths and the same stream on both. Scope: kept.

### Testing and iteration speed — both sides — explored
The whole browser suite also runs inside the app on every system. Core: the full suite and the three proofs. Open: the macOS build needs a Mac and an Apple account. Scope: kept.

### A crash inside the app — problem side — explored
Decided: direct calls stay, and engine errors are caught at the boundary with a restart from the last stoppage. Core. Risk: a hard crash still closes the app. Scope: kept.

### One match per start — problem side — explored
Decided: playing again without a restart waits for the season layer. Core. Scope: kept.

### Builds and test runs on three systems — solution side — explored
Decided: local builds for Windows and Linux now, hosted CI later; the first trial proves Windows and Linux, and the Mac proof comes later. Core. Scope: kept.

### Replay files open the app — solution side — explored
Decided: the app registers the replay file type. Core. Scope: kept.

## Open now
- Accepted risk with "prove on two, Mac later": web engine differences on a Mac show up late.
- Accepted risk with "catch engine errors": a hard crash, for example running out of memory, still closes the app window.
- Accepted risk with "play again waits for the season": until then, the app plays one match per start.
- Accepted risk with "CI later": the Mac proof and the full suite on macOS wait with it.
- Accepted risk with "the Mac app ships unsigned": most Mac players give up at the block, so the Mac release reaches few players until it is signed.
- Accepted risk with "the app updates itself": a host for update files and an update signing key; a lost key strands every installed copy on its version.
- Accepted risk with "signing later": Windows keeps its warnings, and on macOS most players cannot open the app without a trip to the system settings.
- Accepted risk with "lean Tauri": a second test tool on Linux and macOS, web engine differences between systems, and a workaround for the Steam overlay.
- Accepted risk with "full suite everywhere": with Tauri, a second test tool on Linux and macOS, and the slowest test runs.
- Accepted risk with "two transport paths": the two paths must stay equal, and a bug can live in one path only.
- Accepted risk with "a shell first": the next release adds repackaging work with no new player feature, and the unsigned-setup warning stays.
- Accepted risk with "Steam someday": a package chosen without Steam in mind can mean a second package when Steam comes.
- Accepted risk with "touch not now": a desktop-only shell means a second shell when touch comes.
- For the plan: is Tauri 2 confirmed as the shell, and does a trial against the three proofs settle it?

<!-- The front ends here. The record follows. -->

## Decisions
### What feels wrong with a browser tab today
- Reach to stores and platforms drives the package choice, not the pains of the browser tab. Why: the tab itself is tolerable.
- The next release delivers one app window that can later grow to Steam and touch, before a Mac build, a signed install, or a richer view. Why: you chose it as the one thing the next release delivers. Accepted risk: the next release adds repackaging work with no new player feature, and the unsigned-setup warning stays.

### Which platforms and stores the game must reach
- The game must be able to ship on Steam. Why: you chose it as a target.
- The game must run on macOS and Linux desktops as well as Windows. Why: you chose it as a target.
- A tablet or phone is a real future target. Why: you chose it as a target.
- Steam is a wish for someday, not a plan. Accepted risk: a package chosen without Steam in mind can mean a second package when Steam comes.
- Touch does not shape the package choice now. Accepted risk: a desktop-only shell means a second shell when touch comes.

### Install, trust, and updates
- The app checks for a new version and installs it itself. Core. Accepted risk: a host for update files and an update signing key; a lost key strands every installed copy on its version.
- Code signing comes later on both Windows and macOS, when the game has players. Core. Accepted risk: Windows keeps the SmartScreen and antivirus warnings, and on macOS most players cannot open an unsigned app without a trip to the system settings.
- The app keeps saves in today’s data folder, so old matches carry over. Core.
- The Mac app ships to players unsigned, with steps to allow it in the system settings. Accepted risk: most Mac players give up at the block, so the Mac release reaches few players until it is signed.

### Stay in the browser, but make it feel like an app
- A chromeless browser app window or an installable web app is off the list. Why: it has no store path.

### The shells and the game engine hosts
- The choice leans to Tauri 2 over Electron. Core. Why: installer size, a path to touch, and direct calls into the Rust engine with no Node addon. Accepted risk: a second test tool on Linux and macOS, web engine differences between systems, and a workaround for the Steam overlay.
- Unity, Godot, and Bevy leave the list, and the comparison is Tauri 2 against Electron. Why: the Unity HTML plugin has no Linux build, and a rebuilt page loses every page test. This replaced: a list of five hosts with Unity as a full candidate.
- A 3D view is a someday wish and does not weigh on the package now.

### How the engine and the page talk inside a package
- One transport interface with two paths: direct engine calls in the app window, and the socket in a plain browser for tests, development, and replays. Core. Why: most page tests survive only if the page still runs in a plain browser over the socket. Accepted risk: two transport paths must stay equal, and a bug can live in one path only. This replaced: direct engine calls only.
- One match must give the same event stream over direct calls and over the socket. Core.

### Testing and iteration speed
- The whole browser suite also runs inside the packaged app on Windows, Linux, and macOS. Core. Accepted risk: with Tauri this needs a second test tool on Linux and macOS, and it gives the slowest test runs.
- The first trial proves the app on Windows and Linux; the macOS proof and the macOS suite runs come with CI or a Mac. Core. Why: Windows and Linux can prove the shell now, and there is no Mac and no CI. Accepted risk: web engine differences on a Mac show up late. This replaced: a full match on Windows, Linux, and macOS in the first trial.
- At least one full browser-suite test must pass inside the app on each system. Core.

### A crash inside the app
- Direct calls stay; engine errors are caught at the boundary and the match restarts from the last stoppage. Core. Accepted risk: a hard crash, for example running out of memory, still closes the app window.

### One match per start
- Playing the next match without a restart waits for the season layer. Core. Why: the season layer changes what the next match is anyway. Accepted risk: until then, the app plays one match per start and feels like a one-shot launcher.

### Builds and test runs on three systems
- Hosted CI comes later; Windows and Linux build locally now, and the Mac waits. Core. Accepted risk: the Mac proof and the full suite on macOS wait with it.

### Replay files open the app
- The app registers the replay file type, so a double-click on a replay opens it in the app. Core.

## Ideas still open
- The browser chrome makes the game feel like a website. Raised by you.
- A closed tab or a browser crash ends the match while the engine keeps running. Raised by you.
- The default browser can be any browser at any version. Raised by you.
- All four drivers hold at once: a Mac and Linux build, trust and install, one choice that grows to Steam and touch, and a richer view later. Raised by you.
- A 3D view can live in the page itself with WebGL and a permissive library, with no game engine host. Raised by the agent.
- The finished distribution piece of the match engine workflow needs an update for the app shell. Raised by the agent.
- A Windows installer near today's size was not chosen as a proof. Raised by you.

## What we found
- The installed game starts the engine, serves the page, and opens it in the player's default browser. Each start plays one match. Source: packaging/README.md:23.
- The page reaches the engine over a WebSocket on the local machine, and the engine serves the page itself with cross-origin isolation headers. Source: web/socket.mjs:12 and crates/engine-cli/src/web.rs:429.
- The Windows setup file is about 2.6 MB and is not signed, so SmartScreen warns before it runs. Source: dist/SoccerManager-0.1.0-windows-x64-setup.exe and packaging/README.md:57.
- The product rules say plain HTML, CSS, and JavaScript, permissive-licence dependencies only, and Windows 11 first as a browser page served by a local engine process. Source: PRODUCT.md:37, PRODUCT.md:40, PRODUCT.md:42.
- Installers for other operating systems were set aside as a later piece of work in the match engine workflow. Source: .ai/workflows/football-manager-match-engine/po-answers.md:194.
- The match engine workflow has a finished distribution piece that built the setup file and the Linux archive, and an app shell changes it. Source: .ai/workflows/football-manager-match-engine/03-slice-distribution.md.
- The browser-tab release already ships for Linux, and a Mac build needs Apple signing and notarization whichever shell is chosen. Source: packaging/README.md:7-9.
- The product names the Football Manager 3D view as an anti-reference, because cluttered overlays hide the play. Source: PRODUCT.md:26.
- The browser test suite drives the page against a live engine it starts, over the local socket. Source: README.md:85 and e2e/README.md:3.
- A direct call means something different in each host: a Rust command in Tauri, a Node native addon in Electron, a native plugin in Unity, and a rewrite of the page in Godot or Bevy. Source: https://v2.tauri.app/develop/calling-rust/, https://www.electronjs.org/docs/latest/tutorial/native-code-and-electron, https://docs.unity3d.com/Manual/ios-native-plugin-create.html (research run).
- The official Tauri WebDriver runs on Windows and Linux only; macOS needs WebdriverIO with an embedded plugin or a paid driver. Source: https://v2.tauri.app/develop/tests/webdriver/ (research run).
- Our Playwright suite can drive the page in a Tauri window on Windows through the WebView2 debugging port. Source: https://playwright.dev/docs/webview2 (research run).
- Tauri ships mocks for its engine calls, so page unit tests can run with no app window. Source: https://v2.tauri.app/develop/tests/mocking/ (research run).
- Tauri development mode reloads the page on a change and rebuilds the Rust side on a change. Source: https://v2.tauri.app/develop/ (research run).
- Playwright drives Electron apps directly, but that support is still marked experimental. Source: https://playwright.dev/docs/api/class-electron (research run).
- An Electron installer is usually about 50 to 150 MB. A Tauri app on Windows can be under 1 MB, plus about 1.8 MB with the WebView2 bootstrapper. Source: https://v2.tauri.app/distribute/windows-installer/ (research run).
- Tauri uses a different web engine on each system, so the page needs testing on all three. Source: https://v2.tauri.app/develop/tests/webdriver/ (research run).
- The Steam overlay does not draw over WebView2; a Tauri plugin adds a surface for it as a workaround. In Electron the overlay works with extra flags, and Vampire Survivors first shipped on Electron. Source: https://github.com/tauri-apps/tauri/issues/6196, https://github.com/ceifa/steamworks.js/ (research run).
- The main Unity plugin that shows HTML (Vuplex 3D WebView) is paid per platform and has no Linux build. Source: https://store.vuplex.com/webview/overview/ (research run).
- Unity Personal is free up to 200,000 dollars of revenue, the runtime fee is cancelled, and the runtime stays proprietary. Source: https://unity.com/products/pricing-updates (research run).
- Rebuilding the page in Godot, Bevy, or Unity loses the Playwright suite and the page unit tests, because no browser automation reaches an engine-drawn interface. A permissive Godot web view plugin exists for desktops only. Source: https://github.com/doceazedo/godot_wry (research run).

- The page today shows an engine crash and restarts the engine from the last stoppage with the same score and clock. That works because the engine is a separate process. Source: e2e/README.md:62.
- The repository has no CI configuration; builds and test runs happen on the owner’s machine and in WSL. Source: no .github/workflows folder; packaging/README.md:44.

- A season layer design document exists in the realism programme. Source: docs/design/realism/03-season-layer.md.

## What we are assuming
- The existing HTML page stays the user interface, whatever the package is. Confirmed: the game engine hosts are off the list.

## Tensions
- A Unity host against the permissive-licence rule and the plain-web-page rule. Resolved: Unity is off the list.
- Unity with its HTML plugin against the Linux target. Resolved: Unity is off the list.
- Direct engine calls against the browser suite and replay viewing, which use the socket. Resolved: two paths behind one interface.
- A full match on macOS as a proof against signing later. Resolved: the Mac app ships unsigned.
- The full suite in the app on every system against Tauri's test tools. Resolved: you lean to Tauri and accept a second test tool.
- A touch target later against shells that build for desktops only, such as Electron. Resolved: touch does not shape the choice now.
- Direct engine calls against crash recovery, which needs a separate process. Resolved: catch engine errors at the boundary.
- CI later with no Mac against the core proofs on all three systems. Resolved: the first trial proves Windows and Linux, and the Mac comes later.

## Questions for the plan
- Is Tauri 2 confirmed as the shell, and does a trial against the three proofs settle it?

## Scope
One piece of work carries every kept decision. Nothing is left for later, and nothing is cut.

The piece holds three parts, in order:
- **The Tauri app-window trial.** The engine link behind one interface with two paths (direct calls in the app, the socket in a plain browser). Direct calls with engine errors caught at the boundary, and a restart from the last stoppage. The full browser suite inside the app. It proves three things on Windows and Linux: a full match, a suite test in the app, and the same event stream on both paths. It ends with a go or no-go for Tauri, and Electron is the fallback. The idea of a 3D view in the page itself, with no game engine host, rides along as a note.
- **The documentation update, after a go.** The product rules still say "a browser page served by a local engine process", and the finished distribution piece of the match engine workflow describes the NSIS setup. Both change for the app shell.
- **The app release.** The app installer in place of the NSIS setup, self-update, the replay file type, and saves in today’s folder.

These decisions are kept as waits, with no work now: code signing on both systems, the unsigned Mac release, hosted CI, the Mac proof and Mac suite runs, playing again without a restart (with the season layer), Steam, and touch.

## Work
1. **Ship Touchline as a Tauri 2 app window.** One workflow: a trial with a go or no-go on Windows and Linux, then the app release. The documentation update goes into its documentation plan. Its shape stage holds the go or no-go as a gate: on a no-go, the release is planned again for Electron. Form: a feature to build. Entry: `/wf intake desktop app shell — a Tauri 2 trial with a go or no-go, then the app release from brainstorm-app-packaging-options-20260923`

## How to continue
- Resume: `/wf intake brainstorm brainstorm-app-packaging-options-20260923`
- Control words: `park <thread>` · `pull <thread>` · `drop <thread>` · `board` · `look it up` · `second opinion` · `done`
- Retire when no thread is live: `/wf close brainstorm-app-packaging-options-20260923`
