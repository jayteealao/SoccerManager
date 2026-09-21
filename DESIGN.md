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

- Team kit colors come from team data. Use them on the pitch and in the score only. Map each kit color to a contrast-safe on-pitch variant.
- Semantic states to define before the first screen ships: hover, focus, active, disabled, selected, loading, error, warning, success, info. One meaning per color on every screen.

## Typography
- Working choice: one system sans family for every element: `-apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif`.
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
- No token file exists yet. This file is the source of the values until a stylesheet defines them.
- Prefix: `--mv-`. Names: `--mv-bg`, `--mv-fg`, `--mv-brand`, `--mv-brand-hover`, `--mv-brand-pressed`, `--mv-on-brand`, `--mv-pitch`, `--mv-pitch-line`, `--mv-pitchside-bg`, `--mv-ring`, `--mv-radius-sm`, `--mv-radius-md`, `--mv-radius-lg`, `--mv-radius-xl`, `--mv-padx-md`, `--mv-pady-md`, `--mv-gap`, `--mv-font-md`, `--mv-font-num`, `--mv-ease`, `--mv-shadow-1`.

## Brand Assets
- None exist. Generate the logo, the palette, and any illustration programmatically in JavaScript. Schedule that work as its own feature before the first public screen.

## Notes
- Themes: light is primary. Pitchside governs overlays drawn on the pitch. No dark theme for the panels in the first release.
- Icon set: not chosen. Decide before the first control ships.
- Accessibility: focus rings visible at 3:1 against the adjacent background. Color is never the only indicator for cards, fatigue, or errors.
- Rendering: the pitch is a canvas at 60 frames per second with 23 moving markers (22 players and the ball).
