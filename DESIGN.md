# Design

## Colors
Strategy: the dark Broadcast Blue look, chosen as the main look on 2026-09-26 (brainstorm `brainstorm-realism-additions-20260922`). The values below are read from the whole-game sketch canvas (https://claude.ai/artifact/5shzExrW9n9qWAVcPxD3Kr) and are hex values as the sketch defines them. The screen must still read in a lit room during the day, so every text size proves its contrast before a screen ships.

| Role | Value | Sketch token | Note |
|---|---|---|---|
| Page | `#141517` | `--page` | outer background |
| Well | `#0e0f11` | `--well` | the frame behind a screen |
| Ground | `#232427` | `--ground` | panel surface |
| Ground 2 | `#2a2b2f` | `--ground-2` | odd table rows, option rows |
| Ground 3 | `#26272a` | `--ground-3` | even table rows |
| Rail | `#1b1c1f` | `--rail` | the left navigation rail |
| Rule | `#3a3c41` | `--rule` | divider |
| Rule 2 | `#303236` | `--rule-2` | quieter divider, card border |
| Ink | `#ffffff` | `--ink` | primary text |
| Ink 2 | `#c3c6cc` | `--ink-2` | secondary text |
| Ink 3 | `#8a8d94` | `--ink-3` | labels, muted text |
| Ink 4 | `#5d6067` | `--ink-4` | faint text |
| Navy 900 | `#082a66` | `--navy-900` | deepest brand band |
| Navy 700 | `#0b3a8c` | `--navy-700` | header band start, date block |
| Navy 600 | `#0d47a8` | `--navy-600` | section headers, strips, buttons |
| Navy 500 | `#0f4fb8` | `--navy-500` | header band end |
| Navy sub | `#aebbd6` | `--navy-sub` | text on navy |
| Cyan | `#19c6f0` | `--cyan` | the action colour: continue, selected, focus ring |
| On cyan | `#0a2250` | `--cyan-ink` | text on cyan |
| Star | `#f5d33a` | `--star` | ratings stars, highlight button |
| Good | `#4ad66d` | `--good` | success, high attribute band |
| Middle | `#e3d34a` | `--mid` | middle attribute band |
| Warning | `#f0923a` | `--warn` | warning, low attribute band |
| Bad | `#e5484d` | `--bad` | error, loss, lowest band |
| Pitch | `#1f8a2e` | `--pitch` | turf, the most saturated object |
| Pitch card | `#17722a` | `--pitch-card` | cards drawn on the pitch |
| Pitch deep | `#0f5a1d` | `--pitch-deep` | pitch edge |
| Link | `#72f04e` | `--link` | passing links drawn on the pitch |
| Magenta | `#d62ad6` | `--magenta` | a second overlay colour on the pitch |
| Chip | `#17181a` | `--chip` | name chips under player markers |
| Attack duty | `#f59e3b` | `--at` | duty marker |
| Support duty | `#4aa8ff` | `--su` | duty marker |
| Defend duty | `#4ad66d` | `--de` | duty marker |

- The sketch also defines club colours for its fictional example clubs (`--club`, `--kel`, `--kel-away`, `--var`). They are sample data, not tokens.
- Team kit colours come from team data. Use them on the pitch and in the score only. Map each kit colour to a contrast-safe on-pitch variant.
- Attribute values use four bands: 15 and up good, 11 to 14 middle, 7 to 10 warning, below 7 bad.
- Semantic states to define before the first screen ships: hover, focus, active, disabled, selected, loading, error, warning, success, info. One meaning per colour on every screen.
- The interim light palette (OKLCH, brand hue 250) is the viewer's test skin, in `viewer/src/skins/interim-light/`. It proves that a skin swap needs no code change; players are not offered it.

## Typography
- Saira Semi Condensed 500 to 800 for display: the header band, section headers, the score bug, the goal banner, labels in capitals, and attribute numbers. Saira 400 to 700 for text and numbers. Both are under the SIL Open Font License and ship as `woff2` with their licence texts from the skin's own `fonts/` folder (`viewer/src/skins/broadcast-blue/fonts/`); no font service is contacted. The sketch loads them from a font service only because it is a sketch.
- Fallbacks: `'Barlow Semi Condensed', 'Arial Narrow', system-ui, sans-serif` for display, and `system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif` for text.
- Numerals: `font-variant-numeric: tabular-nums` for the clock, the score, tables, attributes, and statistics.
- Scale: the Touchline Full Game sketch's scale, adopted on 2026-09-29. Base text 11 px/1.35; tables and section labels 10.5 px; sublabels 9.5 px; strip facts 12 px at weight 600; header name 14.5 px display 700 in capitals; score 28 to 30 px display 800. Every text size must pass contrast on its dark ground before it ships.
- The interim light test skin keeps Barlow Condensed and IBM Plex Sans in `viewer/src/skins/interim-light/fonts/`.

## Elevation
- Shadows over borders for panels: `0 1px 2px oklch(0.24 0.02 250 / 0.12)`.
- Concentric radius: outer radius equals inner radius plus padding.

## Components
- Every screen ports its closest screen from the Touchline Full Game sketch (https://claude.ai/artifact/5shzExrW9n9qWAVcPxD3Kr): the 40 px icon rail, the 52 px slanted navy header with the date block and the cyan action block, the sub-navigation tabs, the navy info strip, and sections split by 1 px rules instead of cards. Parts not yet built stay as marked stubs.
- No component library.
- Component of record: `match-control`, the button primitive for play, pause, speed, confirm substitution, and kick-off.
  - Sizes: sm 28 px, md 36 px, lg 44 px, xl 52 px (height).
  - States: default, hover, pressed, focus, disabled.
  - Themes: dark broadcast, pitchside.
- Radii: 4, 6, 8, 10 px. Spacing base 4 px. Control padding 14 px horizontal, 8 px vertical. Icon-to-label gap 8 px.
- Loading uses skeletons (a skeleton pitch), never spinners in content areas. Empty states teach: kick-off with no events and 0 to 0.

## Motion
- Easing: `cubic-bezier(0.32, 0.72, 0, 1)` (ease-out). Product transitions run 150 to 250 ms.
- Goal: the score bug pulses; the banner enters from the pitch edge and leaves within 1.5 s.
- No bounce and no elastic easing. `@media (prefers-reduced-motion: reduce)` disables every animation.

## Tokens
- Each skin's `tokens.css`, in `viewer/src/skins/<skin>/`, is the token file and the single source for CSS. The Broadcast Blue skin holds the values above. The canvas reads its colours from the active skin (`palette.js` beside the tokens), and `viewer/tests/colour.test.js` fails when the two drift apart.
- No prefix: the tokens use the sketch's names, as in the table above (`--ground`, `--navy`, `--cyan`, `--ink`, and the rest), plus the radii `--radius-sm`, `--radius-md` and `--radius-lg`. Every skin defines the same names.
- `npm run scan` (in `viewer/`) fails on a literal colour or font value outside the skins, and on `--ink-4` outside inactive stub text. `npm run contrast` checks every text and boundary pair that `contrast-pairs.json` lists against WCAG 2.2 AA.
- `npm run scan` also holds the window-step rules: no fluid font size, no layout measured in script outside the pitch canvas, and no `@media` width other than the window steps below.

## Layout
- The page fills the browser window. The window picks one of five steps:

  | Step | Window width | Scale | Controls |
  |---|---|---|---|
  | Compact | 768 to 1023 px, or under 600 px high | 1 | 44 px hit areas; the tabs scroll sideways |
  | Standard | 1024 to 1599 px | 1 | 24 px hit areas; as drawn at 1280 by 800 |
  | Wide | 1600 to 1919 px | 1 | 24 px; side panels in two columns |
  | Large | 1920 to 2559 px | 1.125 | 24 px times the scale |
  | Huge | 2560 px and up | 1.375 | 24 px times the scale |

- The compact layout scales down only below 768 px wide, to fit the window's width. Under 600 px high the page does not shrink: it scrolls up and down, so browser zoom still grows the text.
- Media queries on the window pick the step: `min-width` 1024, 1600, 1920 and 2560 px, `max-width` 1023 px and `max-height` 599 px are the only widths and heights a media query uses. Each panel (the side groups, the tables, the report columns) is a size container and changes its columns with its own width.
- The scale is CSS `zoom` on the page's top box, so type, spacing, controls and radii grow by one factor. Text never takes a fluid size.
- The pitch canvas is the only layout measured in script: a `ResizeObserver` on its box, which draws at the device pixel ratio and keeps the ground's proportions.
- A screen's body scrolls up and down in a tall window. The page never scrolls sideways.
- At every step the header band takes the spare width and its title ends in an ellipsis; below 1280 px the stub header icons hide, and Menu stays.

## Brand Assets
- No asset files exist and none are added. Generate the logo, the palette, and any illustration programmatically in JavaScript.
- `viewer/src/lib/mark.js` is that function. **The mark is the touchline.** One function draws three zones into an SVG or a canvas: the field above, a white line at 58 percent of the tile height, and the pitchside band below it, where the manager stands. Two objects sit on the field: one marker and the ball, with one faint pitch marking. Inputs: tile size, corner radius (tile size x 0.21), and the active skin's three mark colours: the field, the line, and the pitchside band.
- Four variants: primary (turf tile), reversed (paper tile, brand line), pitchside (turf tile with a paper ring, for dark surfaces), mono (outline, for the favicon and print).
- Club crests call the same function with the club's kit colours in place of the turf and a marker position seeded from the club id. A facsimile club's crest and kit echo the real club's colours and shape, never the real badge.
- Schedule the generator as its own feature before the first public screen.

## Notes
- Themes: the dark Broadcast Blue look is primary for every surface, and it must read in a lit room during the day. No light theme is planned.
- Icon set: not chosen. Decide before the first control ships.
- Accessibility: focus rings visible at 3:1 against the adjacent background. Color is never the only indicator for cards, fatigue, or errors.
- Rendering: the pitch is a canvas at 60 frames per second with 23 moving markers (22 players and the ball).
