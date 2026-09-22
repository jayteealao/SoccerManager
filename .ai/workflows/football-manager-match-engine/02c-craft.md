---
schema: sdlc/v1
type: design-contract
slug: football-manager-match-engine
title: "Touchline match screen visual contract"
status: ready
created-at: "2026-09-22T14:41:32Z"
updated-at: "2026-09-22T14:41:32Z"
component: match-control
based-on: 02b-design.md
tokens: [--tl-bg, --tl-panel, --tl-fg, --tl-fg-muted, --tl-line, --tl-brand, --tl-brand-hover, --tl-brand-pressed, --tl-on-brand, --tl-pitch, --tl-pitch-line, --tl-pitchside-bg, --tl-ring, --tl-success, --tl-warning, --tl-danger, --tl-card-yellow, --tl-skeleton, --tl-radius-sm, --tl-radius-md, --tl-radius-lg, --tl-radius-xl, --tl-padx-md, --tl-pady-md, --tl-gap, --tl-font-display, --tl-font-md, --tl-font-num, --tl-ease, --tl-shadow-1, --tl-header-h, --tl-col-left, --tl-col-mid, --tl-col-right, --tl-pitch-h, --tl-controls-h]
states: [default, hover, focus, active, disabled, loading, empty, error]
sizes: [desktop]
themes: [light, pitchside]
refs:
  design: 02b-design.md
  index: 00-index.md
  steer: steer.md
  plan: 04-plan-viewer-pitch.md
register: product
image-gate: pass
north-star-mock: "Design canvas \"Match Viewer Design\" — https://claude.ai/artifact/5gcPdykgjpXMuRuGzeHEG2 (private; every constraint transcribed as text in steer.md)"
references-loaded: [typeset, animate, colorize, layout, optimize, harden, polish, product]
---

# Visual contract: Touchline match screen

## The Contract

The design brief left two gates open and named plan as their owner. Both close here without an image run. The standing steering file `steer.md` names the Design canvas "Match Viewer Design" as the north-star mock, instructs that the direction is confirmed, and forbids a competing mock, so `image-gate` reads `pass` with `steer.md` as its source. The brief-confirm gate is satisfied by a user-confirmed PRODUCT.md, recorded at design setup Round 1 Q5. The product now has a name, Touchline, which clears the `[TODO]` marker that blocked this stage.

The canvas is private and no sub-agent can open it, so `steer.md` carries every constraint as text and this contract is built from that text, not from pixels. Thirty-six tokens, one grid, two typefaces, and one drawing function make up the whole surface. The load-bearing decision is the token prefix: `steer.md` fixes `--tl-`, which supersedes the `--mv-` prefix written into `02b-design.md` and `02b-design.yaml` before the product was named. Those two files are stale on that one point, not in conflict.

Ten fidelity items survive into implementation, and four of them belong to `viewer-pitch`: the grid, the pitch tile, the control strip, and the marker treatment. The other six are pinned here so the three later viewer slices inherit them rather than re-deciding. The top risk is the one the brief named and the product owner ranked second in PRODUCT.md: broadcast energy on a light scene competes with 14-pixel tabular numbers, so the contrast of the numeric row is proved before any banner motion is written.

## 1. Visual direction confirmed

Product register, committed colour strategy, light scene. Light panels tinted toward hue 250, one brand blue on headers and controls, and team kit colours confined to the pitch and the score. The pitch is the most saturated object on the screen and every other surface stays quieter than the turf. Energy lives in the score bug, the goal banner, and the event highlights; the panels stay calm and tabular.

Three deviations from the brief emerged while writing this contract, each recorded rather than silently applied:

1. **Token prefix.** The brief and its YAML use `--mv-`. `steer.md` fixes `--tl-` (Touchline). This contract uses `--tl-` throughout and treats the brief's prefix as stale.
2. **Typeface names.** The brief left typography as "one system sans family". `steer.md` names Barlow Condensed 700 for display and IBM Plex Sans 400/500/600 for text and numbers. `design/typeset.md` carries IBM Plex on its reflex-reject list, which governs *brand*-register font discovery; this is a product-register surface where the product owner has already named the faces, so the named faces stand. Recorded as a deliberate deviation, not an oversight.
3. **Kit colour safety.** `steer.md` says the marker ring is the kit secondary when it measures 3:1 on turf. The shipped team file `content/teams/default-a.json` sets that colour to `#000000`, which DESIGN.md bans. Plan Round 3 Q11 resolved this: every kit hex is converted to OKLCH and its lightness clamped into `0.20 … 0.88` before the 3:1 test runs.

## 2. North-star mock

- **Path:** the Design canvas "Match Viewer Design", `https://claude.ai/artifact/5gcPdykgjpXMuRuGzeHEG2`. The link is private. No local image file exists and none is generated.
- **Scene sentence:** the manager plays at a desk in a lit room during the day; the screen must read against ambient light, so panels are light and the pitch is a saturated green field with high-contrast markers.

Annotated callouts, transcribed from `steer.md`:

1. **Header band, 56 pixels.** Carries the score bug. Full page width. Implementation note: `viewer-pitch` builds the band empty; `viewer-match-day` fills it.
2. **Body grid, 336 / 616 / 296 with an 8-pixel gutter.** Three columns under the header at a 1280 by 800 viewport. Implementation note: CSS Grid with fixed track widths; no breakpoint behaviour, because the target is one desktop size.
3. **Pitch, 616 by 411.** Fills the middle column. Implementation note: a canvas sized in CSS pixels at 616 by 411, with the backing store multiplied by `devicePixelRatio` and the context scaled to match, so markings stay crisp.
4. **Playback controls, 40 pixels, directly under the pitch.** Implementation note: `match-control` buttons at the `sm` size (28 pixels) inside a 40-pixel strip, so the strip height is the token, not the button height.
5. **Statistics below the controls.** Implementation note: `viewer-match-day`. `viewer-pitch` leaves the region empty.
6. **Match feed and tactics panel in the right column.** Implementation note: `viewer-match-day` and `viewer-lineup-tactics`.
7. **Player marker.** A filled disc in the club's safe kit primary, a ring in the safe kit secondary when it measures 3:1 against turf and pitch-line white otherwise, and a shirt number in white or ink, whichever contrasts more with the fill.
8. **Ball with a short trail.** Implementation note: the trail is the last N tick positions at falling alpha, drawn as one path, never as N separate strokes.
9. **The mark is the touchline.** One JavaScript function draws three zones into a canvas or an SVG: the field above, a white line at 58 percent of the tile height, and the pitchside band below it. Inputs are the tile size, a corner radius of tile size times 0.21, and the tokens `--tl-pitch`, `--tl-pitch-line`, `--tl-pitchside-bg`. One marker and the ball sit on the field. No logo is ever loaded from an image file. Club crests call the same function with the club's kit colours in place of the turf.
10. **State words on every chip.** Queued, Applies now, Applied, Rejected. Colour never carries the state alone. The same rule governs cards, fatigue bands, and injury markers.

## 3. Mock fidelity inventory

Each item names the slice that owns it. `viewer-pitch` carries a plan step for every item it owns; the rest are pinned here so the later viewer slices inherit them.

1. **The 1280 by 800 grid** — header 56, columns 336 / 616 / 296, gutter 8 — page shell — non-negotiable because the pitch's final size and position depend on it, and evidence measured at any other layout does not describe the shipping screen. Owner: `viewer-pitch`.
2. **Pitch tile at exactly 616 by 411** with the full markings drawn from the engine's own 105 by 68 metre coordinate space — middle column — non-negotiable because the aspect ratio ties rendered position to simulated position, and a mismatch would put markers outside the touchline. Owner: `viewer-pitch`.
3. **Playback control strip, 40 pixels, immediately under the pitch** — middle column — non-negotiable because the product owner's first-five-seconds statement requires the controls visible without scrolling. Owner: `viewer-pitch`.
4. **Marker treatment: safe kit-primary fill, conditional ring, contrast-chosen shirt number** — on the pitch — non-negotiable because it is the only place team identity appears on the pitch, and because the ring rule is the accessibility guarantee that colour is not the only signal separating two teams. Owner: `viewer-pitch`.
5. **Ball with a short falling-alpha trail** — on the pitch — non-negotiable because the trail is the Opta and Second Spectrum reference that separates this from an arcade view. Owner: `viewer-pitch`.
6. **Barlow Condensed 700 for display, IBM Plex Sans 400/500/600 for text and numbers, both from a local bundle, with `font-variant-numeric: tabular-nums` on every changing number** — everywhere — non-negotiable because a number that changes width as it changes value makes the whole numeric row twitch, and because a font service is forbidden. Owner: `viewer-pitch` lands the bundle and the numeric treatment; later slices reuse it.
7. **The `--tl-` OKLCH token set at hue 250, with no pure black and no pure white anywhere** — everywhere — non-negotiable because it is the single source of every colour and because two files still carry the stale `--mv-` prefix. Owner: `viewer-pitch` writes the token file.
8. **The touchline mark drawn programmatically, never loaded from an image** — favicon, header mark, club crests — non-negotiable because the product owner ruled that no asset files exist and every mark is generated in JavaScript. Owner: `viewer-pitch` writes the function and uses it for the favicon and the empty header mark; `viewer-match-day` uses it for club crests.
9. **State words beside every state colour** — chips, cards, fatigue bands, injury markers — non-negotiable because NFR-6 forbids colour as the only indicator. Owner: `viewer-lineup-tactics` (chips) and `viewer-match-day` (cards, fatigue). `viewer-pitch` applies it to the lag notice, which names the sustained speed in words and digits.
10. **Skeletons and a step list while the engine starts, never a spinner in a content area** — loading state — non-negotiable because the brief weights the loading state and the product owner named skeletons directly. Owner: `viewer-reports-recovery`. `viewer-pitch` reserves the skeleton token and draws the skeleton pitch before the first tick arrives.

## 4. Implementation contract

### Token choices

Write `web/tokens.css` as the single `:root` block. Every value is OKLCH. No pure black and no pure white appears anywhere.

| Token | Value | Use |
|---|---|---|
| `--tl-bg` | `oklch(0.985 0.006 250)` | page paper |
| `--tl-panel` | `oklch(0.995 0.003 250)` | panel surface |
| `--tl-fg` | `oklch(0.24 0.02 250)` | ink |
| `--tl-fg-muted` | `oklch(0.48 0.02 250)` | secondary label |
| `--tl-line` | `oklch(0.90 0.008 250)` | hairline divider |
| `--tl-brand` | `oklch(0.55 0.17 250)` | header and control fill |
| `--tl-brand-hover` | `oklch(0.49 0.17 250)` | hover |
| `--tl-brand-pressed` | `oklch(0.43 0.17 250)` | pressed |
| `--tl-on-brand` | `oklch(0.985 0.006 250)` | label on brand |
| `--tl-pitch` | `oklch(0.62 0.13 145)` | turf, the most saturated object |
| `--tl-pitch-line` | `oklch(0.97 0.01 145)` | pitch markings, fallback marker ring |
| `--tl-pitchside-bg` | `oklch(0.28 0.03 250)` | overlays drawn on the pitch |
| `--tl-ring` | `oklch(0.55 0.17 250 / 0.35)` | focus ring, 3:1 against paper |
| `--tl-success` | `oklch(0.52 0.14 150)` | applied state |
| `--tl-warning` | `oklch(0.55 0.13 75)` | lag notice |
| `--tl-danger` | `oklch(0.50 0.19 25)` | rejected, error |
| `--tl-card-yellow` | `oklch(0.85 0.16 95)` | caution card |
| `--tl-skeleton` | `oklch(0.93 0.01 250)` | skeleton fill |
| `--tl-radius-sm` … `-xl` | `4px`, `6px`, `8px`, `10px` | radii |
| `--tl-padx-md` / `--tl-pady-md` | `14px` / `8px` | control padding |
| `--tl-gap` | `8px` | icon to label, and the grid gutter |
| `--tl-font-display` | `"Barlow Condensed", sans-serif` at 700 | score bug, goal banner, report titles |
| `--tl-font-md` | `14px / 600 "IBM Plex Sans"` | control label |
| `--tl-font-num` | `14px / 500 "IBM Plex Sans"`, `tabular-nums` | clock, score, statistics, frame counter |
| `--tl-ease` | `cubic-bezier(0.32, 0.72, 0, 1)` | every transition |
| `--tl-shadow-1` | `0 1px 2px oklch(0.24 0.02 250 / 0.12)` | panel elevation, shadows over borders |
| `--tl-header-h` | `56px` | header band |
| `--tl-col-left` / `-mid` / `-right` | `336px` / `616px` / `296px` | body grid tracks |
| `--tl-pitch-h` | `411px` | pitch tile height |
| `--tl-controls-h` | `40px` | playback strip |

**New tokens added by this contract** beyond DESIGN.md: `--tl-fg-muted`, `--tl-line`, `--tl-panel`, `--tl-skeleton`, `--tl-success`, `--tl-warning`, `--tl-danger`, `--tl-card-yellow` (all from `steer.md`), `--tl-font-display`, and the six layout tokens. DESIGN.md must gain the same rows when `viewer-pitch` writes the token file, so the two never disagree.

### Component decisions

- **Create** `match-control`, the button primitive of record, in `web/components/match-control.css`. Four sizes (28 / 36 / 44 / 52 pixels high), five states, two themes. `viewer-pitch` ships the `sm` and `md` sizes in the light and pitchside themes; the remaining sizes ship unused but defined, so later slices add no new geometry.
- **Create** `web/mark.mjs`, the touchline-mark function. One exported function, three tokens and two numbers in, one drawn tile out. The favicon, the empty header mark, and every later club crest call it.
- **Create** `web/pitch.mjs`, the pitch renderer. It owns the coordinate mapping from the engine's centred metre space to canvas pixels, and nothing else draws to the canvas.
- **Do not create** a component library, a CSS framework, or a build step. PRODUCT.md forbids all three.

### Layout structure

CSS Grid. One page-level grid of two rows (`--tl-header-h`, then the rest) and, in the second row, three columns of `--tl-col-left`, `--tl-col-mid`, `--tl-col-right` with `gap: var(--tl-gap)`. No breakpoints and no fluid tracks: the target is a single 1280 by 800 desktop viewport, and `stack.platforms` carries no mobile or tablet. The middle column stacks the pitch (`--tl-pitch-h`) above the control strip (`--tl-controls-h`) above the statistics region, which stays empty in this slice.

### Type scale

Product register, fixed rem scale, ratio 1.125 to 1.2. `12px` caption, `14px` control label and number, `16px` body minimum, `18px` and `20px` for panel headings, `30px` and above for the score bug in Barlow Condensed 700. Shirt numbers on the canvas are drawn at 10 pixels in IBM Plex Sans 600, which is below the 12-pixel floor for CSS text and is permitted only because a canvas shirt number is a graphic label with a redundant ring and position, not readable prose.

### Colour application

- Turf, pitch markings, and the marker fills are the only saturated objects. Panels never exceed chroma 0.01.
- Brand blue appears on the header band, the playback controls, and the focus ring. It never appears on the pitch.
- Kit colours appear on the pitch and, later, in the score. Nowhere else.
- The lag notice uses `--tl-warning` **and** the words "Playing at 3x — the engine sustains 3x", so the colour is never the only signal.
- Every numeric readout is `--tl-fg` on `--tl-panel` and must measure at least 4.5:1 before any motion is written. This is the brief's named top risk and PRODUCT.md's second strategic principle.

### Motion

Apply `design/animate.md`'s frequency rule. The playback controls are pressed tens of times per match, so they get a colour transition only, never a press-scale bounce; `match-control` ships a `static` variant and the playback strip uses it.

| Interaction | Animate? | Timing | Easing |
|---|---|---|---|
| Control hover and press | yes, colour only | 150 ms | `--tl-ease` |
| Focus ring appearance | no | — | — |
| Lag notice enter | yes, opacity and 8-pixel rise | 200 ms | `--tl-ease` |
| Lag notice exit | yes, opacity and 8-pixel fall | 150 ms | `ease-in` |
| Speed change | no | — | — |
| Scrubber drag | no transition; the value follows the pointer directly | — | — |
| Skeleton pitch to first tick | yes, cross-fade | 250 ms | `--tl-ease` |

`bounce: 0` everywhere. No keyframe animation on anything re-triggerable; transitions only, so a rapidly toggled notice retargets instead of restarting. Animate `opacity` and `transform` only. `@media (prefers-reduced-motion: reduce)` sets every transition to `0.01ms`; it never touches the canvas redraw, which is content.

### Finish and detail

- **Concentric radius.** The pitch tile sits in a panel with 8 pixels of padding, so the panel radius is the tile radius plus 8. The control strip's buttons at `--tl-radius-sm` inside a strip with 6 pixels of padding give the strip `--tl-radius-md`.
- **Shadows over borders.** Panels lift with `--tl-shadow-1`. The pitch-tile edge and the divider under the header stay real 1-pixel borders in `--tl-line`.
- **Hit areas.** Every playback control is at least 40 by 40 pixels. The `sm` button is 28 pixels tall, so it carries a centred pseudo-element extending the hit area to 40 by 40, and adjacent hit areas never overlap.
- **Optical alignment.** The play triangle shifts 2 pixels right of geometric centre.
- **Tabular figures** on the clock, the speed readout, the frame-time counter, and the memory gauge. Not on the shirt numbers, which are static per player.

### State coverage

| Element | Required states |
|---|---|
| `match-control` button | default, hover, focus, active, disabled |
| Speed selector | default, hover, focus, active, disabled, selected |
| Rewind scrubber | default, hover, focus, active, disabled |
| Lag notice | hidden, shown |
| Pitch canvas | loading (skeleton), empty (kick-off, no motion), default (playing), error (socket dropped) |
| Page | loading, default, error |

The `error` state for the canvas and the page is stubbed in this slice and completed by `viewer-reports-recovery`; the stub renders the panel and the text, with no recovery action.

## 5. Anti-patterns to avoid

Specific to this surface, drawn from the brief's anti-goals and the absolute bans:

- **No Football Manager 3D clutter.** No overlays on the pitch beyond the score bug and the goal banner. Nothing is drawn over a player marker.
- **No sponsor-board clutter.** No logos, no advertising boards drawn on or beside the pitch.
- **No arcade or cartoon styling.** No drop shadows on markers, no glow, no motion trails on players — the trail belongs to the ball alone.
- **No purple-blue gradient** anywhere, including the header band, which is flat `--tl-brand`.
- **No nested cards.** The pitch panel holds the canvas directly; it does not contain a second card.
- **No decorative side accent stripe** on any panel. Use a full hairline or a background tint.
- **No bounce or elastic easing**, including on the scrubber handle.
- **No pure black and no pure white**, including in a kit colour taken from a team file — clamp it first.
- **No spinner in a content area.** The pitch shows a skeleton.
- **No font service.** Both faces load from `web/fonts/` as `woff2`.
- **No logo image file.** The mark is drawn by `web/mark.mjs`.
- **No colour-only state.** The lag notice, and later every chip and card, carries its state word.

## 6. Implementation references

`/wf implement` loads, from `skills/wf/reference/design/`: `product.md` (register), `typeset.md` (type scale, tabular figures, the reflex-reject deviation recorded in section 1), `animate.md` (the frequency rule, strong easing, interruptible transitions, `bounce: 0`), `colorize.md` (OKLCH, committed strategy, kit-colour mapping), `layout.md` (the fixed three-column grid), `optimize.md` (canvas at 60 frames per second), `harden.md` (focus rings at 3:1, colour never the only indicator, canvas accessibility), and `polish.md` (concentric radius, shadows over borders, hit areas, optical alignment).

The authoritative list is the `references-loaded:` array in this file's frontmatter. A reference named here and missing there is not loaded.

## 7. Carry the contract into the plan

`04-plan-viewer-pitch.md` carries a concrete step for every inventory item owned by `viewer-pitch` — items 1, 2, 3, 4, 5, 6, 7, and 8 — and one step that writes the token file and the `match-control` primitive in full, so items 9 and 10 and the four sizes this slice does not use are already defined when `viewer-match-day`, `viewer-lineup-tactics`, and `viewer-reports-recovery` arrive. The implementation contract's token, component, layout, type, colour, motion, finish, and state decisions are pointers from those steps, not restatements.

`00-index.md` moves `current-stage` to `plan`. There is no hand-back to a separate design command.
