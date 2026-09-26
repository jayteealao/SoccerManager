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
- The interim light palette (OKLCH, brand hue 250) is still in `web/tokens.css` and on today's match screen. It stays there until a workflow rebuilds a screen on these values.

## Typography
- Saira Semi Condensed 500 to 800 for display: the header band, section headers, the score bug, the goal banner, labels in capitals, and attribute numbers. Saira 400 to 700 for text and numbers. Both are under the SIL Open Font License and ship from `web/fonts/` as `woff2`; no font service is contacted. The sketch loads them from a font service only because it is a sketch.
- Fallbacks: `'Barlow Semi Condensed', 'Arial Narrow', system-ui, sans-serif` for display, and `system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif` for text.
- Numerals: `font-variant-numeric: tabular-nums` for the clock, the score, tables, attributes, and statistics.
- Scale: fixed rem scale, ratio 1.125 to 1.2. Body text 16 px minimum, line height 1.5 or more. Control labels 14 px at weight 600. Numbers 14 px at weight 500. Every text size must pass contrast on its dark ground before it ships.
- Today `web/fonts/` holds Barlow Condensed and IBM Plex Sans for the interim screen. The Saira files are added when a workflow rebuilds a screen.

## Elevation
- Shadows over borders for panels: `0 1px 2px oklch(0.24 0.02 250 / 0.12)`.
- Concentric radius: outer radius equals inner radius plus padding.

## Components
- No component library. Plain HTML, CSS, and JavaScript.
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
- `web/tokens.css` is the token file and the single source for CSS. Today it holds the 36 interim light tokens, which no longer match the colours above; the next workflow that rebuilds a screen moves it to the Broadcast Blue values, and from then on the two never disagree. `web/tests/colour.test.mjs` reads `web/tokens.css` and fails when the four values the canvas draws with drift from the module that draws them.
- Prefix: `--tl-`. Names: `--tl-bg`, `--tl-panel`, `--tl-fg`, `--tl-fg-muted`, `--tl-line`, `--tl-brand`, `--tl-brand-hover`, `--tl-brand-pressed`, `--tl-on-brand`, `--tl-pitch`, `--tl-pitch-line`, `--tl-pitchside-bg`, `--tl-ring`, `--tl-success`, `--tl-warning`, `--tl-danger`, `--tl-card-yellow`, `--tl-skeleton`, `--tl-radius-sm`, `--tl-radius-md`, `--tl-radius-lg`, `--tl-radius-xl`, `--tl-padx-md`, `--tl-pady-md`, `--tl-gap`, `--tl-font-display`, `--tl-font-md`, `--tl-font-num`, `--tl-ease`, `--tl-shadow-1`, `--tl-header-h`, `--tl-col-left`, `--tl-col-mid`, `--tl-col-right`, `--tl-pitch-h`, `--tl-controls-h`.
- Layout tokens fix the 1280 by 800 screen: header 56, columns 336 / 616 / 296, gutter 8, pitch 411 tall, control strip 40.

## Brand Assets
- No asset files exist and none are added. Generate the logo, the palette, and any illustration programmatically in JavaScript.
- `web/mark.mjs` is that function. **The mark is the touchline.** One function draws three zones into an SVG or a canvas: the field above, a white line at 58 percent of the tile height, and the pitchside band below it, where the manager stands. Two objects sit on the field: one marker and the ball, with one faint pitch marking. Inputs: tile size, corner radius (tile size x 0.21), and the tokens `--tl-pitch`, `--tl-pitch-line`, `--tl-pitchside-bg`.
- Four variants: primary (turf tile), reversed (paper tile, brand line), pitchside (turf tile with a paper ring, for dark surfaces), mono (outline, for the favicon and print).
- Club crests call the same function with the club's kit colours in place of the turf and a marker position seeded from the club id. A facsimile club's crest and kit echo the real club's colours and shape, never the real badge.
- Schedule the generator as its own feature before the first public screen.

## Notes
- Themes: the dark Broadcast Blue look is primary for every surface, and it must read in a lit room during the day. No light theme is planned.
- Icon set: not chosen. Decide before the first control ships.
- Accessibility: focus rings visible at 3:1 against the adjacent background. Color is never the only indicator for cards, fatigue, or errors.
- Rendering: the pitch is a canvas at 60 frames per second with 23 moving markers (22 players and the ball).
