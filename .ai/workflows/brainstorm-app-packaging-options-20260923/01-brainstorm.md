---
schema: sdlc/v1
type: brainstorm
slug: brainstorm-app-packaging-options-20260923
topic: "what are our app packaging options is unity an option for us with html assets, is electron or taurior something else an option"
status: open
board: brainstorm-board.json
page: "https://claude.ai/artifact/84Qd7oJcnUWzUnfw6y1aR1"
created-at: "2026-09-23T21:55:55Z"
updated-at: "2026-09-25T08:02:45Z"
sessions: 3
batches: 45
consult-runs:
- at: "2026-09-25T06:56:49Z"
  trigger: claim-contradicted
  provider: codex
  verdict: "The Unreal probe can justify more investigation, but its bar cannot justify Unreal as the long-run host."
revisions:
- rev: 1
  at: "2026-09-23T22:37:31Z"
  trigger: manual
  because: done
  changed: the person chose done after every area was explored; scoping the work begins
- rev: 2
  at: "2026-09-24T03:07:25Z"
  trigger: resume
  because: the person asked to evaluate GPUI, the Zed interface framework, and its community fork
  changed: board reopened; a new area and thread for a native Rust interface
- rev: 3
  at: "2026-09-24T06:18:03Z"
  trigger: manual
  because: done
  changed: GPUI area closed; the piece of work now carries a GPUI pitch probe beside the Tauri trial
- rev: 4
  at: "2026-09-25T06:16:10Z"
  trigger: resume
  because: the person asked to consider Unreal Engine as a packaging option
  changed: board reopened; a new area and thread for Unreal Engine as the host
---

# Brainstorm: app packaging options

## The Brainstorm
We started from one question: how should Touchline reach a player’s desktop? Today the installed game starts the engine and opens the match page in the player’s default browser tab. The candidates were Unity, Electron, Tauri, and others.

We now believe that reach, not the browser tab, drives the choice. The next release is one app window that can grow to Steam and to touch. The choice leans to Tauri 2, with Electron as the fallback. Unity, Godot, and Bevy are off the list: the Unity HTML plugin has no Linux build, and a rebuilt page loses every page test. The page talks to the engine through one interface with two paths: direct calls in the app, and the socket in a plain browser for tests, development, and replays. Engine errors are caught at the boundary, so a match restarts from the last stoppage.

Around the shell, the app updates itself and registers the replay file type. Saves stay in today’s folder. Signing waits until the game has players, and the Mac app ships unsigned. Hosted CI also waits, so the first trial proves Windows and Linux only: a full match, a suite test in the app, and the same event stream on both paths. In a second session, GPUI, the Zed interface framework, and its community fork were weighed. It cannot keep the page or its tests, so it does not replace Tauri. A small GPUI pitch probe runs beside the Tauri trial instead, and the web page rule can change if the probe shows a smoother pitch and a working test story. The sharpest risk is the Mac: its web engine differences show up late, and most Mac players give up at the unsigned-app block.

A third session reopened the board to weigh Unreal Engine, and it changed the plan. One engine for the long run draws you most, then a 3D match view, then Steam and consoles; in three years Touchline is a 3D match-day game. The licence rule now allows a proprietary engine host, and the 3D anti-reference is rewritten too. A research run found the Unreal browser widget too weak to carry the page, so the Unreal probe uses no web page: a flat 2D pitch and one dense native screen, such as the squad list. The Rust engine links into Unreal in-process, the bridge consoles would need, and hands Unreal the same byte stream the web page reads; engine errors are caught at the boundary, as in the Tauri app. The probe must play a full live match with a tactics change, a pause, and a second match, read recorded matches byte for byte, and pass Unreal tests. It runs first, at roughly four to six weeks for one owner, and the Tauri release waits for it. The sharpest risk is that wait: no player-facing change for weeks, against the earlier decision that the next release is one app window.

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

### A native Rust interface with no web page (GPUI and its community fork) — solution side — explored
Decided: Tauri stays the lean; a GPUI pitch probe runs beside the Tauri trial, against a bar of a smoother pitch and a working test story; the web page rule can change if it passes. Open: the GPUI base, picked by a licence audit. Scope: kept.

### Unreal Engine as the host — solution side — explored
Core: the probe shape (Unreal only, a 2D pitch, one dense native screen, an in-process bridge passing the same byte stream). First version: the whole probe after the Tauri release, and the licence-rule change. Later: the long-run 3D match-day goal and the reasons behind it. Top risk: the bridge and the toolchain. Scope: mixed.

### The product rules and a 3D match-day game — problem side — open
Opened in the third session. Not yet explored.

### Console targets and their terms — solution side — open
Opened in the third session. Not yet explored.

### The art and the people a 3D view needs — problem side — open
Opened in the third session. Not yet explored.

## Open now
- Top risk with "the in-process bridge": it fights the Windows and Linux toolchains, and a hard crash still closes the game.
- Accepted risk with "a 3D match-day game": the product identity changes, because the product rules call the Football Manager 3D view an anti-reference.
- The recorded piece of work is out of date: it drops the GPUI probe and ends at the release, and the Unreal probe becomes its own workflow.
- Accepted risk with "an engine exception to the licence rule": Unity comes back, a royalty applies above 1 million dollars gross, and the product rules change.
- Accepted risk with "a GPUI pitch probe": a second trial at the same time splits the attention of one owner.
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

### A native Rust interface (GPUI)
- A small GPUI probe rebuilds only the pitch view against the engine interface, beside the Tauri trial. Why: to measure the feel and the work of a native interface on a small piece. Accepted risk: a second trial at the same time splits the attention of one owner.
- The rule that the interface is plain HTML, CSS, and JavaScript can change if the probe shows GPUI is clearly better.
- GPUI counts as clearly better only if its pitch runs smoother than the web pitch on the same machine and match, and its pitch tests run on Windows and Linux with a way to drive it from outside.
- The GPUI probe is one more part of the app-shell workflow, next to the Tauri trial, and both end at the same go or no-go.

### Unreal Engine as the host
- Three things draw you to Unreal: a rich 3D match view, reach to Steam and to consoles, and one engine for the long run so the game does not move a second time. Why: chosen as the reasons to weigh Unreal.
- The reasons rank in this order: one engine for the long run, then the 3D match view, then Steam and consoles.
- The licence rule gains an exception for a proprietary engine host; code dependencies stay MIT-compatible or Apache-2.0-compatible. Accepted risk: Unity comes back into the comparison, a royalty applies above 1 million dollars gross, and the product rules and the open-source promise of the project change.
- An Unreal probe takes the place of the GPUI pitch probe beside the Tauri trial, and both end at the same go or no-go. Why: GPUI and Unreal both test a move away from the web page, so one probe answers the long-run question. This replaced: a GPUI pitch probe beside the Tauri trial.
- In three years Touchline is a 3D match-day game: a broadcast-style 3D view with animated players, and the screens serve the match. Accepted risk: the product rules name the Football Manager 3D view as an anti-reference and set a flat, light pitch, so the product identity changes, not only the package. This replaced: a 3D view as a someday wish.
- The probe uses Unreal only; Godot and Unity are not built. Accepted risk: the go or no-go compares Unreal only with Tauri, so a lighter engine such as Godot is never measured.
- The Unreal probe rebuilds one real screen with Unreal's own interface tools; that screen sends a change the engine applies, and no browser widget is used. Why: the browser widget fails local pages on packaged Linux builds and likely cannot run on a console. Accepted risk: on the Unreal path the web page and its browser suite are replaced by Unreal screens, so a later full move to Unreal rebuilds every screen. This replaced: a live 3D pitch plus one page screen in the browser widget.
- Unreal wins the go or no-go only if the native screen sends a change the engine applies, and Unreal Automation or Gauntlet tests drive that screen and check the 3D pitch in a packaged build. Accepted risk: the browser suite covers the page in the Tauri app only; the Unreal side needs a second test stack, and a go still says nothing about Linux. This replaced: a bar with a page screen in the widget and a test from outside.
- The Unreal bar adds a full live match in the packaged probe through the real Rust bridge, with a tactics change, a pause, and a second match after the first. Why: the bridge is the part most likely to change the answer. Accepted risk: the probe takes longer, and a go still says nothing about Linux.
- The native Unreal screen in the probe is a dense one, for example the squad list with sorting, selection, and keyboard use. Why: it shows the real cost of moving every screen to Unreal.
- The Unreal probe draws today's flat 2D pitch, not a 3D one, beside the dense native screen; 3D comes later on the same engine. Why: it tests the bridge, the dense screen, and Unreal as the one engine with no 3D art work. Accepted risk: the probe does not test the 3D look, which is the second reason for Unreal. This replaced: a live 3D pitch in the probe, and your idea to test Unreal with a 2D pitch.
- The Unreal probe links the Rust engine into Unreal as a C library, in-process. Why: it is the bridge consoles would need, proven now. Accepted risk: about 8 to 15 days of bridge work in place of 5 to 8, the toolchain fights on Windows and Linux come first, and a Rust crash takes Unreal down unless the crash restart is rebuilt inside Unreal.
- The Unreal bar adds a replay check: Unreal must read recorded matches exactly as the web page does, byte for byte. Why: it catches reader bugs early for about a day of work.
- In the Unreal probe, engine errors are caught at the C boundary, and the match restarts from the last stoppage inside Unreal, as in the Tauri app. Accepted risk: a hard crash, such as running out of memory, still closes the game, and the probe grows.
- The in-process bridge hands Unreal the existing byte stream and takes the existing commands; Unreal reads it with a C++ reader of about 600 to 900 lines. Why: the same-stream rule then holds on every path with no exception, and the replay check works as is. This replaced: new direct function calls for Unreal.
- The Unreal probe budget is five to eight weeks for one owner, and the probe keeps every part. Why: the parts add up to about 24 to 38 working days. Accepted risk: the Unreal answer comes up to eight weeks after the Tauri release. This replaced: a budget of four to six weeks.
- The first version rewrites both the licence rule (an engine exception) and the 3D anti-reference in the product rules. Accepted risk: the product rules point at a 3D game that is not yet a decision, and the flat-pitch rule no longer guides screens built before the 3D goal is decided.
- The Tauri app window ships as the next release; if Unreal wins later, the Tauri app is a stepping stone. Why: the one-app-window release stands over Unreal first. This replaced: the Unreal probe first, with the Tauri release waiting.
- The Unreal probe is its own workflow, started after the Tauri release; the app-shell workflow drops the GPUI probe and ends at the release.
- The Unreal probe starts after the Tauri app release ships. Why: the release comes soonest, and one owner does one thing at a time. Accepted risk: the answer on Unreal waits about four to six weeks after the release.

## Ideas still open
- The browser chrome makes the game feel like a website. Raised by you.
- A closed tab or a browser crash ends the match while the engine keeps running. Raised by you.
- The default browser can be any browser at any version. Raised by you.
- All four drivers hold at once: a Mac and Linux build, trust and install, one choice that grows to Steam and touch, and a richer view later. Raised by you.
- A 3D view can live in the page itself with WebGL and a permissive library, with no game engine host. Raised by the agent.
- The finished distribution piece of the match engine workflow needs an update for the app shell. Raised by the agent.
- A Windows installer near today's size was not chosen as a proof. Raised by you.
- Unreal Engine is worth weighing as the package for the game. Raised by you.
- Avoid the cost of an in-process bridge by having Unreal read the engine's existing socket. Raised by the agent.

## Ideas still open (GPUI)
- GPUI draws you for one Rust codebase with no transport, and for the speed and feel of a native GPU-drawn interface. Raised by you.
- The one engine interface can take a third client later, so a native GPUI interface can come after Tauri without undoing it. Raised by the agent.
- A small probe rebuilds only the pitch view in GPUI, to measure the feel and the work. Raised by the agent.
- GPUI can be learned first on a developer tool, for example a calibration dashboard. Raised by the agent.

## What we found
- The page is about 5,300 lines of JavaScript, 1,600 lines of CSS, and 290 lines of HTML, with 23 unit test files (about 2,200 lines) and 6 browser suite files (about 1,200 lines). Source: web/ and e2e/tests/.
- The socket, recovery, pending-change, and decode code on the page plus the protocol crate come to about 2,700 lines. Source: web/socket.mjs, web/recovery.mjs, web/pending.mjs, web/decode.mjs, crates/protocol/src/.
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

- The official gpui crate (Apache-2.0) was last published in October 2025 and is pre-1.0 with frequent breaking changes. Source: https://crates.io/crates/gpui (research run).
- An open report says the main GPUI code pulls GPL-3.0 crates through its tracing crates. Source: https://github.com/zed-industries/zed/issues/55470 (research run, not confirmed).
- The community fork gpui-ce has its own releases apart from the editor, is Apache-2.0 or MIT, and is active in 2026, with yanked and renumbered releases. Source: https://github.com/gpui-ce/gpui-ce (research run).
- GPUI Kit (Apache-2.0) offers more than 75 components and runs a commercial trading desktop app. Source: https://github.com/longbridge/gpui-component (research run).
- GPUI does not render HTML; its only web view crate is experimental, covers Windows and macOS only, and draws over the window. Source: https://github.com/longbridge/gpui-kit (research run).
- GPUI tests are Rust tests with simulated input; screenshot tests run on macOS only, and outside automation needs an unmerged accessibility change. Source: https://gpui-kit.com/docs/test/ and https://github.com/gpui-ce/gpui-ce/pull/274 (research run).
- GPUI runs on Windows, Linux, and macOS; mobile is experimental and partly GPL or AGPL; web is basic. Source: https://github.com/longbridge/gpui-mobile (research run).
- No shipped game is built on GPUI. Source: https://github.com/zed-industries/awesome-gpui (research run).
- GPUI on Windows needs a DirectX 11 GPU and a Windows SDK compiler for release builds. Source: https://zed.dev/docs/windows (research run).

- Unreal ships a Web Browser widget on the Chromium Embedded Framework, but on an old Chromium: about 90 up to Unreal 5.5, and 128 only from 5.7. Source: https://forums.unrealengine.com/t/request-for-guidance-on-updating-cef-in-unreal-webbrowser-plugin/2617726 (research run).
- In packaged Unreal 5.6 Linux builds, the browser widget cannot load a local file or a localhost page, and Epic has not answered the report. Source: https://forums.unrealengine.com/t/ue-5-6-0-web-browser-widget-cef-on-native-linux-builds-is-broken-and-unable-to-load-local-files-at-all/2685525 (research run).
- The page in the widget can call the game through window.ue, but only through a small C++ subclass of the widget. Source: https://docs.unrealengine.com/5.1/en-US/API/Runtime/WebBrowser/SWebBrowserView/BindUObject/ (research run).
- No confirmed way exists to attach Playwright to the browser widget in a packaged Unreal game; it likely needs a plugin change, or a move to the Unreal Automation and Gauntlet tests. Source: https://www.magpcss.org/ceforum/viewtopic.php?f=6&t=19950 (research run).
- No public evidence shows the browser widget on PlayStation, Xbox, or Switch, and Chromium does not target consoles. Source: https://forums.unrealengine.com/t/porting-a-html5-game-to-console-using-unreals-web-browser-widget/120338 (research run).
- Unreal takes a 5 percent royalty above 1 million dollars gross per game, 3.5 percent with a same-day Epic Games Store launch, and Epic store revenue is excluded. Source: https://www.unrealengine.com/license (research run).
- The game code, the Rust crate, and the page can stay public under MIT or Apache-2.0, but any engine change cannot be public, and every contributor must accept the Epic licence. Source: https://www.unrealengine.com/eula/unreal (research run).
- A Rust library with a C interface loads from a small Unreal C++ plugin, with several examples; the Linux build fights the Unreal clang toolchain. Source: https://github.com/MaikKlein/unreal-rust (research run).
- An empty Unreal 5 Windows build is about 320 MB, about half when trimmed, and the browser plugin adds about 150 MB; today the setup file is about 2.6 MB. Source: https://forums.unrealengine.com/t/what-is-libcef-so-and-why-is-it-gigantic/1439586 (research run).
- The product rules allow MIT-compatible or Apache-2.0-compatible dependencies only. Source: PRODUCT.md:39.

- The event stream may not carry enough timing, facing, ball flight, and contact detail for believable 3D animation, so the 3D view may invent movement that contradicts the match. Source: a second opinion (the reviewer could not read the files, so this rests on the summary it was given).
- A 3D engine leaves two possible owners of the ball, player positions, and contact; the Rust engine must stay the owner, or the design changes. Source: a second opinion (the reviewer could not read the files, so this rests on the summary it was given).
- One simple Unreal screen does not predict the cost of the dense manager screens: sortable tables, text entry, keyboard use, and scaling. Source: a second opinion (the reviewer could not read the files, so this rests on the summary it was given).
- The Tauri trial tests packaging today's game, and the Unreal probe tests a future 3D rewrite, so one go or no-go mixes two questions with different bars. Source: a second opinion (the reviewer could not read the files, so this rests on the summary it was given).
- Unreal tests prove little unless they drive the packaged game through the real Rust bridge and compare the result with a known engine run. Source: a second opinion (the reviewer could not read the files, so this rests on the summary it was given).

- The engine stream is JSON control messages plus small binary position frames (a 99-byte keyframe and a 48-byte delta), with 9 message kinds to the client and 6 JSON commands back. Source: docs/reference/protocol.md:57 and crates/protocol/src/codec.rs:1.
- A C++ client for the stream needs roughly 600 to 900 lines: about 80 for the binary frames, most of the rest for the nested JSON messages. Source: web/decode.mjs:1-78 and docs/reference/protocol.md:72-147 (research run).
- The engine allows one viewer per match, and each engine process plays one match. Source: docs/reference/protocol.md:47 and packaging/README.md:23.
- An in-process bridge (Rust built as a C library inside Unreal) takes about 8 to 15 days for the probe: a new C API over the engine, Windows and Linux toolchain fights, and a Rust panic takes Unreal down, losing the crash restart; the Rust-in-Unreal crates are proofs of concept or unmaintained. Source: https://github.com/MaikKlein/unreal-rust and https://github.com/viniciusmorgado/rusteal (research run).
- An out-of-process bridge, where Unreal starts the unchanged engine and reads its existing socket, takes about 5 to 8 days: a C++ decoder, starting and stopping the engine process, and shipping a second program; no Rust enters the Unreal build and the crash restart keeps working. Source: https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/WebSockets/IWebSocket/OnRawMessage (research run).
- Consoles likely do not allow a game to start a second program, so the socket bridge is for PCs only; a later console build can carry the same byte stream through an in-process library and keep the C++ decoder. Source: platform documentation is under NDA; not confirmed (research run).

## What we are assuming
- The existing HTML page stays the user interface, whatever the package is. Confirmed: the game engine hosts are off the list.
- The reasons that took Unity, Godot, and Bevy off the list apply to Unreal unless Unreal answers them. Still open.

## Tensions
- A Unity host against the permissive-licence rule and the plain-web-page rule. Resolved: Unity is off the list.
- Unity with its HTML plugin against the Linux target. Resolved: Unity is off the list.
- Direct engine calls against the browser suite and replay viewing, which use the socket. Resolved: two paths behind one interface.
- A full match on macOS as a proof against signing later. Resolved: the Mac app ships unsigned.
- The full suite in the app on every system against Tauri's test tools. Resolved: you lean to Tauri and accept a second test tool.
- A touch target later against shells that build for desktops only, such as Electron. Resolved: touch does not shape the choice now.
- Direct engine calls against crash recovery, which needs a separate process. Resolved: catch engine errors at the boundary.
- CI later with no Mac against the core proofs on all three systems. Resolved: the first trial proves Windows and Linux, and the Mac comes later.
- The pull to a 3D view and to consoles against the earlier decisions that 3D is a someday wish and that Steam is someday. Resolved: the long-run goal is a 3D match-day game.
- The page inside the Unreal widget against consoles as a reason for Unreal. Resolved: the probe uses a native Unreal screen, not the widget.
- The bar that a test reaches the widget screen against the finding that Playwright cannot reach a packaged Unreal game. Resolved: Unreal tools test the Unreal side.
- An Unreal host against the licence rule of MIT-compatible or Apache-2.0-compatible dependencies. Resolved: the rule gains an exception for an engine host.

- Unreal first with the Tauri release waiting against the earlier decision that the next release delivers one app window. Resolved: the app window ships first.
- An in-process Unreal bridge against the crash restart from the last stoppage. Resolved: errors are caught at the C boundary, as in the Tauri app.
- New direct calls for Unreal against the same event stream on every path. Resolved: the bridge hands Unreal the existing byte stream.
- Rewriting the product rules now against the long-run 3D goal left for later. Resolved: both parts are rewritten now.
- The kept probe parts (about 24 to 38 days) against the four to six week budget. Resolved: the budget is five to eight weeks.

## Questions for the plan
- Which GPUI base does the probe use, gpui-ce or the snapshots with GPUI Kit? A licence audit picks it, because the product rule allows permissive licences only.
- Is Tauri 2 confirmed as the shell, and does a trial against the three proofs settle it?

## Scope
One piece of work carries every kept decision. Nothing is left for later, and nothing is cut.

The piece holds three parts, in order:
- **The Tauri app-window trial.** The engine link behind one interface with two paths (direct calls in the app, the socket in a plain browser). Direct calls with engine errors caught at the boundary, and a restart from the last stoppage. The full browser suite inside the app. It proves three things on Windows and Linux: a full match, a suite test in the app, and the same event stream on both paths. It ends with a go or no-go for Tauri, and Electron is the fallback. The idea of a 3D view in the page itself, with no game engine host, rides along as a note.
- **The documentation update, after a go.** The product rules still say "a browser page served by a local engine process", and the finished distribution piece of the match engine workflow describes the NSIS setup. Both change for the app shell.
- **The app release.** The app installer in place of the NSIS setup, self-update, the replay file type, and saves in today’s folder.

A GPUI pitch probe joins the trial part, against its bar, and the GPUI base is picked by a licence audit in the plan.

These decisions are kept as waits, with no work now: code signing on both systems, the unsigned Mac release, hosted CI, the Mac proof and Mac suite runs, playing again without a restart (with the season layer), Steam, and touch.

## Work
1. **Ship Touchline as a Tauri 2 app window.** One workflow. Part 1: the Tauri trial on Windows and Linux, with the two-path engine interface and the three proofs. Part 1b: a GPUI pitch probe against the same interface, measured against its bar (a smoother pitch and a working test story). Both end at one go or no-go: Tauri or Electron for the shell, and whether a native interface is worth a later rebuild. Part 2: the documentation update after a go. Part 3: the app release. Its shape stage holds the go or no-go as a gate. Form: a feature to build. Entry: `/wf intake desktop app shell — a Tauri 2 trial and a GPUI pitch probe with one go or no-go, then the app release from brainstorm-app-packaging-options-20260923`

## How to continue
- Resume: `/wf intake brainstorm brainstorm-app-packaging-options-20260923`
- Control words: `park <thread>` · `pull <thread>` · `drop <thread>` · `board` · `look it up` · `second opinion` · `done`
- Retire when no thread is live: `/wf close brainstorm-app-packaging-options-20260923`
