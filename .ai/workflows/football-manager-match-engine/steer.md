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
