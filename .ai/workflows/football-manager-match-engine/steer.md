# Steering — football-manager-match-engine

Standing constraints for this workflow. Live chat instructions outrank this file.

## Design direction (2026-09-22)

- Image gate for every viewer slice: the north-star mock is the Design canvas
  "Match Viewer Design", https://claude.ai/artifact/5gcPdykgjpXMuRuGzeHEG2.
  Treat the direction as confirmed. Record `image-gate: pass` with source
  `steer.md`. Do not generate a competing mock.
- The canvas link is private and sub-agents cannot open it. Every constraint a
  sub-agent needs is written as text in this file.
- Screens on that canvas define layout at 1280 by 800: header 56 px carrying the
  score bug; body grid 336 / 616 / 296 with an 8 px gutter; pitch 616 by 411;
  playback controls 40 px under the pitch; statistics below the controls;
  match feed and the tactics panel in the right column.
- Typography: Barlow Condensed 700 for the score bug, the goal banner, and
  report titles; IBM Plex Sans 400/500/600 for text and numbers, with
  `font-variant-numeric: tabular-nums`. Both faces ship from the local bundle,
  never from a font service.
- Palette: the OKLCH tokens in DESIGN.md at hue 250, plus these additions —
  `--tl-success` oklch(0.52 0.14 150), `--tl-warning` oklch(0.55 0.13 75),
  `--tl-danger` oklch(0.50 0.19 25), `--tl-card-yellow` oklch(0.85 0.16 95),
  `--tl-fg-muted` oklch(0.48 0.02 250), `--tl-line` oklch(0.90 0.008 250),
  `--tl-panel` oklch(0.995 0.003 250), `--tl-skeleton` oklch(0.93 0.01 250).
- Kit colors on the pitch: the marker ring is the kit secondary when that colour
  measures 3:1 on turf, otherwise pitch-line white; the shirt number is white or
  ink, whichever contrasts more with the fill.
- The mark is the touchline: the field above, a white line at 58 percent of the
  tile height, and the pitchside band below it. One marker and the ball sit on
  the field. One JavaScript function draws it from the tile size, the corner
  radius (tile size x 0.21), and the tokens `--tl-pitch`, `--tl-pitch-line`, and
  `--tl-pitchside-bg`. Never load a logo from an image file. Club crests call the
  same function with the club's kit colours in place of the turf.
- Token prefix is `--tl-`, not `--mv-`. This supersedes the `--mv-` prefix in
  `02b-design.yaml` and `02b-design.md`, which were written before the product
  was named. Treat the prefix in those two artifacts as stale, not as a conflict.
- Pending-change chips carry the state word (Queued, Applies now, Applied,
  Rejected). Colour never carries the state alone. The same rule governs cards,
  fatigue bands, and injury markers.
- Loading uses skeletons and a step list. Never a spinner in a content area.

## Dark-path counter definition (product owner, 2026-09-23)

- A queued change that is still waiting at full time, because no stoppage that
  admits it came after it was queued, is "expired at full time". Count it in
  `change.expired_at_full_time` (int, unit engine), reported in `match-stats` and
  the calibration run report, with no zero rule.
- `darkpath.change_never_applied` keeps its zero rule and counts only a change
  that an admitting stoppage should have applied and did not.
- Amend `.ai/observability.md` (key list, `dark-paths`, `match-stats`) to match.
  Do not change the computer manager's queuing behaviour.

## Product-owner answers to plan questions (2026-09-23)

All recommended options were chosen. Full text is in po-answers.md.
- viewer-match-day Q-1 = A; viewer-lineup-tactics OQ-1 = 1 and OQ-2 = 1
  (protocol version 3: squads, lineup hold, set-lineup, tactics schema in the
  opening message, roster, per-second stats, and a condition message).
- extra-time-penalties Q-1 = B (kicks on the pitch, 10-round allowance).
- distribution OQ-1 = B (Windows plus Linux; macOS deferred) and OQ-2 = A
  (Start-menu shortcut runs the launcher).
- probe-engine-core Q-1 = A (a failed run emits an error record on stdout).
- probe-engine-core Q-2 = C (unify on `error`: calibration writes `error`, not
  `failure`; the schemas accept `success` and `error` only; statistic keys are
  required only when the outcome is not `error`). Earlier run folders are not a
  compatibility target.

## Moved criteria and slice order (product owner, 2026-09-24)

- Verify `defending-and-discipline` on discipline, the acting keeper, and the
  eight formation pairings that hold (Q-I2). Do not judge it on "No advantage
  from a red card" or on the 4-4-1-1 and 3-4-3 pairings. Those criteria belong
  to `lone-forward`, with their limits unchanged.
- Build order: `defending-and-discipline` verify, `tuning-loop`,
  `lone-forward`, then `keeper-and-shots` (Q-X4). `keeper-and-shots` does not
  plan before `lone-forward` is complete.
- A targeted calibrate run is an inner loop. A slice gate still runs the full
  suites: five seeds for equal and strength, one seed for formations (Q-I1).

## Red-card criterion fixtures (product owner, 2026-09-24, Q-LF1)

- Measure "No advantage from a red card" on balanced fixtures: two equal
  clubs, or both home and away orders of the default clubs, pooled. Apply the
  same fixtures to the slow test and the calibrate red-card suite.
- Do not change any limit. Do not add a lever. If an arm still fails on
  balanced fixtures, stop and report the arm and its figures.

## Red-card criterion moves to keeper-and-shots (product owner, 2026-09-24, Q-LF2)

- `lone-forward`: ship the formations fix with the two levers tuned within
  their bounds. Judge the slice on three criteria: the lone-forward
  formations hold, nothing that passes now regresses, and a lone forward uses
  his team-mates. Do not judge it on "No advantage from a red card".
- `keeper-and-shots`: add "No advantage from a red card" to its criteria, on
  the balanced fixtures of Q-LF1, with every limit unchanged. The measured
  cause is that a side with ten men attacks as if it had eleven. If the save
  and block model does not fix the criterion, stop and report the arms and
  their figures. Do not raise a limit.

## keeper-and-shots criteria changes (product owner, 2026-09-24, Q-KS1, Q-KS2)

- `keeper-and-shots`: do not judge it on "No advantage from a red card". That
  criterion moves to `realism-tuning`, on the balanced fixtures of Q-LF1,
  with every limit unchanged. Do not add a lever for a side with ten men.
- `keeper-and-shots`: the corner floor is 1.2 per team over 200 matches, not
  3.0. Goal kicks stay at least 10 per match, and every corner still follows a
  defending touch over the goal line. Do not turn parries toward the goal line
  and do not use a hold share that is not sourced.
- `tempo-and-restarts`: add corners from clearances and blocked crosses that
  go over a side's own goal line. A corner still needs a real crossing of the
  line after a defending touch (RIM-9).
- `realism-tuning`: it owns the red-card criterion and the 3.5–6.5 corners
  band. If the red-card criterion fails at every in-bounds setting, stop and
  report each arm. Do not raise a limit.

## Heavy runs go to the Contabo server (product owner, 2026-09-25)

This PC crashed twice (stop code 0x9F) during long all-core runs. Run every
heavy command on the server through `.scratch/remote/vps.sh` (git-ignored; it
syncs the working tree, uncommitted edits included, and runs at nice 19):

- Use the server for: `cargo test --workspace`, `cargo clippy`, `cargo fmt
  --check`, every release slow test (`--include-ignored`), and every
  `engine-cli calibrate` run. Example:
  `.scratch/remote/vps.sh run 'cargo build --release -q -p engine-cli && ./target/release/engine-cli --content-dir content calibrate --seed 42 --suite equal --matches 1000 --out ../out/<name>'`
- Write calibrate output under `../out/<name>` on the server. Copy back only
  what the record needs: `.scratch/remote/vps.sh fetch <name>/report.json
  <local evidence dir>`. Do not copy `events/` or `stats/`.
- A command that can run longer than 10 minutes runs in the background.
- Keep on this PC: `engine-cli bench` (CPU time and peak memory are measured
  on Windows only), the page tests (`node --test web/tests/*.test.mjs`) and the
  browser suite. Run any local cargo command with `-j 6` and
  `-- --test-threads 6`.
- Two tests are Windows-only and fail on Linux by design; run them on this PC:
  `data::tests::relative_paths_never_leak_the_root` (`cargo test -p engine
  --lib relative_paths`) and `a_small_run_writes_a_record_per_match_and_a_report_that_validate`
  (`bench.cpu_us_per_tick` is empty on Linux). Every other server failure is
  real.
- Server results match Windows results: 240 matches compared gave identical
  goals, cards, shots, passes and bands (only `ball.max_speed` differs, below
  1e-9). Baselines recorded on this PC stay valid.
- The server is about half as fast as this PC: a full formations suite takes
  about 2.6 hours. Plan the gate runs for that.

## Defending lever in tempo-and-restarts (product owner, 2026-09-25, Q-TR1)

- `tempo-and-restarts` may add bounded, tunable defending values that contest
  a ball carrier: the chance to win the ball from a dribbler, and how close
  pressers come. It may also tune the `clear` weight for throw-ins.
- Keep every criterion of the slice with its limits unchanged, and keep
  `every_formation_holds` passing. Shots must stay realistic, about 10 to 16
  per team per match.
- Also measure the red-card suite (balanced fixtures) on the chosen setting
  and record it. `realism-tuning` still owns that criterion, but this lever
  targets its measured cause.
- If the levers at their bounds cannot meet a criterion, stop and report. Do
  not raise a limit.

## tempo-and-restarts ships b0 (product owner, 2026-09-25, Q-TR2)

- Ship setting `b0`, including the foul chance per tackle attempt at 0.04.
  Do not change the card chance per foul.
- Judge "The ball is in play for about an hour" in this slice at 52 to 68
  minutes per 90. The band in `realism-bands.json` stays 52 to 65, and
  `realism-tuning` must reach it.
- The shipped setting must still pass `every_formation_holds`, the discipline
  criterion and the earlier slices' criteria. If it does not, stop and report.

## tempo-and-restarts ships the counting only (product owner, 2026-09-25, Q-TR3)

This entry replaces the Q-TR2 entry above.
- Do not ship `b0`. Keep the contest levers at the committed values that leave
  play unchanged.
- Judge `tempo-and-restarts` on "Only real passes count" and on no regression
  of the earlier criteria. Do not judge it on passes, ball in play or
  throw-ins.
- `realism-tuning` owns passes 350–550 per team at 75–88% accuracy, ball in
  play 52–65 minutes and throw-ins 35–55, in addition to the red-card
  criterion and the bands it already owns. It may tune the contest levers and
  the foul chance per tackle attempt. The `b0` patch
  (`implement-evidence/tempo-and-restarts/b0-ship/b0-ship.patch`) and its
  measurements are its starting evidence: b0 passed passes, throw-ins and the
  formations test, and failed discipline (0.160 second yellows, 30% sending-off
  share) and the stronger-team test (94 wins in 200).

## VETO: stop after tempo-and-restarts (product owner, 2026-09-25)

- Do not plan, implement, verify or review `realism-tuning`, and do not run
  the slug-wide review. When `tempo-and-restarts` is complete, end the run.
  This veto stands until the product owner removes it.

## Veto lifted for planning realism-tuning (product owner, 2026-09-25)

- The product owner ran `/wf plan football-manager-match-engine realism-tuning`.
  Planning `realism-tuning` is allowed.
- Implement, verify and review of `realism-tuning`, and the slug-wide review,
  run only when the product owner runs them. A driver does not start them.

## realism-tuning plan answers (product owner, 2026-09-25, Q-RT1 to Q-RT4)

- Formations: gate on `every_formation_holds` tightened to 2.4–3.2 goals per
  match for each formation against 4-4-2. Run the 55-pairing suite and record
  it. Do not gate on it.
- Corners: code may add a corner source from clearances and deflected crosses
  over the defenders' own goal line. Keep RIM-9. Keep the 3.5–6.5 band.
- Tunable: contest levers, `foul_base`, the sourced restart delays, shot
  weights, `shot_range`, shot and aim noise, the xG coefficients,
  `foul_booked_factor`, `foul_cooldown_ticks`, carrier decision weights and
  the carry window. Do not change the card chance per foul, the save curve,
  the hold share, any band or any limit.
- Gate: equal and strength on seeds 42, 1, 7, 99, 2026; formations on seed 42.
- If no setting passes every gated band, stop and report the closest result
  and the conflicting pair. Never widen a band.

## realism-tuning stops at the formations wall (product owner, 2026-09-25, Q-RT5)

- Stop `realism-tuning` after its feasibility stage. Keep the committed corner
  source (`clearances.wide_chance` 0.0, `clearances.wide_depth` 16.5). Write no
  tuned value.
- A separate slice for formation behaviour comes before the tuning resumes.
  The product owner starts that slice. A driver does not start it, and does not
  resume `realism-tuning`.
