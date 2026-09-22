# Design

## Colors
Strategy: Committed, on a light scene. All colors are OKLCH. Neutrals are tinted toward the brand hue. Never pure black or pure white.

Working brand hue: 250 (blue). This is a working choice, not a brand decision. The generated palette may replace it.

| Role | Value | Note |
|---|---|---|
| Paper (background) | `oklch(0.985 0.006 250)` | tinted toward the brand hue |
| Ink (text) | `oklch(0.24 0.02 250)` | never pure black |
| Brand | `oklch(0.55 0.17 250)` | headers and controls |
| Brand hover | `oklch(0.49 0.17 250)` | lightness minus 6 |
| Brand pressed | `oklch(0.43 0.17 250)` | lightness minus 12 |
| On brand | `oklch(0.985 0.006 250)` | label on brand |
| Pitch turf | `oklch(0.62 0.13 145)` | the most saturated object |
| Pitch markings | `oklch(0.97 0.01 145)` | |
| Pitchside overlay | `oklch(0.28 0.03 250)` | score bug and overlays on the pitch |
| Focus ring | `oklch(0.55 0.17 250 / 0.35)` | must measure 3:1 against paper |
| Panel | `oklch(0.995 0.003 250)` | panel surface, quieter than paper |
| Muted ink | `oklch(0.48 0.02 250)` | secondary label |
| Hairline | `oklch(0.90 0.008 250)` | divider |
| Success | `oklch(0.52 0.14 150)` | applied state |
| Warning | `oklch(0.55 0.13 75)` | lag notice |
| Danger | `oklch(0.50 0.19 25)` | rejected, error |
| Caution card | `oklch(0.85 0.16 95)` | the yellow card |
| Skeleton | `oklch(0.93 0.01 250)` | skeleton fill while content loads |

- Team kit colors come from team data. Use them on the pitch and in the score only. Map each kit color to a contrast-safe on-pitch variant.
- Semantic states to define before the first screen ships: hover, focus, active, disabled, selected, loading, error, warning, success, info. One meaning per color on every screen.

## Typography
- Barlow Condensed 700 for the score bug, the goal banner, and report titles. IBM Plex Sans 400/500/600 for text and numbers. Both ship from `web/fonts/` as `woff2` under the SIL Open Font License; no font service is contacted. IBM Plex Sans ships as one variable file covering 400 to 700.
- Numerals: `font-variant-numeric: tabular-nums` for the clock, the score, and the statistics.
- Scale: fixed rem scale, ratio 1.125 to 1.2. Body text 16 px minimum, line height 1.5 or more. Control labels 14 px at weight 600. Numbers 14 px at weight 500. The statistics panel at 14 px must pass contrast on paper before it ships.

## Elevation
- Shadows over borders for panels: `0 1px 2px oklch(0.24 0.02 250 / 0.12)`.
- Concentric radius: outer radius equals inner radius plus padding.

## Components
- No component library. Plain HTML, CSS, and JavaScript.
- Component of record: `match-control`, the button primitive for play, pause, speed, confirm substitution, and kick-off.
  - Sizes: sm 28 px, md 36 px, lg 44 px, xl 52 px (height).
  - States: default, hover, pressed, focus, disabled.
  - Themes: light, pitchside.
- Radii: 4, 6, 8, 10 px. Spacing base 4 px. Control padding 14 px horizontal, 8 px vertical. Icon-to-label gap 8 px.
- Loading uses skeletons (a skeleton pitch), never spinners in content areas. Empty states teach: kick-off with no events and 0 to 0.

## Motion
- Easing: `cubic-bezier(0.32, 0.72, 0, 1)` (ease-out). Product transitions run 150 to 250 ms.
- Goal: the score bug pulses; the banner enters from the pitch edge and leaves within 1.5 s.
- No bounce and no elastic easing. `@media (prefers-reduced-motion: reduce)` disables every animation.

## Tokens
- `web/tokens.css` is the token file. It holds all 36 tokens in one `:root` block and is the single source for CSS; this file carries the same values so the two never disagree. `web/tests/colour.test.mjs` reads `web/tokens.css` and fails when the four values the canvas draws with drift from the module that draws them.
- Prefix: `--tl-`. Names: `--tl-bg`, `--tl-panel`, `--tl-fg`, `--tl-fg-muted`, `--tl-line`, `--tl-brand`, `--tl-brand-hover`, `--tl-brand-pressed`, `--tl-on-brand`, `--tl-pitch`, `--tl-pitch-line`, `--tl-pitchside-bg`, `--tl-ring`, `--tl-success`, `--tl-warning`, `--tl-danger`, `--tl-card-yellow`, `--tl-skeleton`, `--tl-radius-sm`, `--tl-radius-md`, `--tl-radius-lg`, `--tl-radius-xl`, `--tl-padx-md`, `--tl-pady-md`, `--tl-gap`, `--tl-font-display`, `--tl-font-md`, `--tl-font-num`, `--tl-ease`, `--tl-shadow-1`, `--tl-header-h`, `--tl-col-left`, `--tl-col-mid`, `--tl-col-right`, `--tl-pitch-h`, `--tl-controls-h`.
- Layout tokens fix the 1280 by 800 screen: header 56, columns 336 / 616 / 296, gutter 8, pitch 411 tall, control strip 40.

## Brand Assets
- No asset files exist and none are added. Generate the logo, the palette, and any illustration programmatically in JavaScript.
- `web/mark.mjs` is that function. **The mark is the touchline.** One function draws three zones into an SVG or a canvas: the field above, a white line at 58 percent of the tile height, and the pitchside band below it, where the manager stands. Two objects sit on the field: one marker and the ball, with one faint pitch marking. Inputs: tile size, corner radius (tile size x 0.21), and the tokens `--tl-pitch`, `--tl-pitch-line`, `--tl-pitchside-bg`.
- Four variants: primary (turf tile), reversed (paper tile, brand line), pitchside (turf tile with a paper ring, for dark surfaces), mono (outline, for the favicon and print).
- Club crests call the same function with the club's kit colours in place of the turf and a marker position seeded from the club id.
- Schedule the generator as its own feature before the first public screen.

## Notes
- Themes: light is primary. Pitchside governs overlays drawn on the pitch. No dark theme for the panels in the first release.
- Icon set: not chosen. Decide before the first control ships.
- Accessibility: focus rings visible at 3:1 against the adjacent background. Color is never the only indicator for cards, fatigue, or errors.
- Rendering: the pitch is a canvas at 60 frames per second with 23 moving markers (22 players and the ball).
