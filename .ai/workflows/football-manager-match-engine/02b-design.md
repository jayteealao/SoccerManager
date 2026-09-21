---
schema: sdlc/v1
type: design
slug: football-manager-match-engine
title: "Design brief: 2D match viewer and match-day screen"
status: complete
created-at: "2026-09-21T19:34:57Z"
updated-at: "2026-09-21T19:34:57Z"
register: product
color-strategy: committed
recommended-references: [typeset, animate, colorize, layout, optimize, harden]
component: match-control
refs:
  index: 00-index.md
  intake: 01-intake.md
  shape: 02-shape.md
---

# Design brief

## The Design

The match viewer inherits a fixed frontend (HTML, CSS, JavaScript), a full match-day content inventory, and a product owner who wants broadcast energy on a light screen. Round 3b answered every discovery question, so this brief records the direction without a further user round.

Four decisions shape the surface. The register is product, with an expressive broadcast treatment: the interface serves a 90-minute management task, and its energy comes from the score bug, event banners, and team colors on the pitch, not from decoration around the panels. The color strategy is committed on a light scene: light panels, one brand blue on headers and controls, team kit colors only on the pitch and in the score. Three anchor references bound the look: the classic Football Manager 2D view for the pitch, Opta and Second Spectrum tracking graphics for markers and trails, and a telemetry dashboard for the numbers. Four states carry design weight and each has a note below.

Plan resolves the image gate and writes the visual contract. The top risk is contrast: broadcast energy on a light scene competes with legibility of small tabular numbers, so the contract must prove the statistics panel at 14px against the paper background before any banner motion is added.

## 1. Feature summary

A match-day screen where a manager watches a 2D replay of a simulated match and intervenes with tactics and substitutions. The screen replaces a text-only result with continuous, believable movement and a live data surface.

## 2. User and context

A football-management player, at a desk in daylight, watching one match at a time for 20 to 90 minutes. Frequency: several matches per session. State of mind: engaged and mildly tense; the manager reads the pitch and the numbers together, and reaches for the tactics panel when the match turns.

Scene sentence (confirmed by the product owner in Round 3b as a light scene): the manager plays at a desk in a lit room during the day; the screen must read against ambient light, so panels are light and the pitch is a saturated green field with high-contrast markers.

## 3. Content inventory

Content elements on the match screen:

- 2D pitch: 105 m by 68 m, 22 player markers with shirt numbers, the ball, a short ball trail, the referee optional.
- Header: clock (tabular numerals), score, team names, kit color chips, playback speed indicator.
- Event feed: commentary lines with minute stamps; goals, cards, substitutions, injuries highlighted.
- Lineups: two columns of 11 plus bench, each row with position, name, fatigue bar, condition marker, cards.
- Statistics panel: possession, shots, shots on target, expected goals, passes, pass accuracy, fouls, corners, offsides.
- Tactics and substitution panel: formation, mentality, team instructions, roles and duties, substitution picker, pending-change list.
- Playback controls: play, pause, speed 1x to 8x, skip to next stoppage, rewind scrubber.

Realistic ranges: 0 events at kick-off, 40 to 120 events by full time; 22 markers always; statistics update every tick; the rewind scrubber spans up to 120 minutes of extra-time play.

Edge cases and state variants:

- Empty: kick-off with no events and 0 to 0 in the header.
- Loading: engine process starting; socket connecting.
- Error: engine crashed or socket dropped; recovery offer from the last stoppage.
- First run: no engine installed or the binary path is missing.
- Power user: 8x playback with many events per second in the feed.
- Degraded: the engine cannot sustain the chosen speed; the notice shows the sustained speed.

Dynamic content: positions change 50 times per simulated second; the clock, score, statistics, and fatigue bars change during play; the event feed appends.

## 4. Visual direction

- Color strategy: Committed. Light neutral panels tinted toward the brand hue, one brand blue for headers and controls, team kit colors on the pitch and in the score. The pitch is the most saturated object. Numbers use ink on paper for maximum legibility.
- Scene sentence: see section 2. Light theme is primary; a pitchside theme governs overlays drawn on the pitch (score bug, goal banner).
- Register: product. The task is a tool the manager operates for long stretches. The product owner chose an expressive broadcast treatment inside that register: motion and identity live in the score bug, goal banners, and event feed highlights; the panels stay calm.
- Anchor references:
  - Football Manager 2D classic view: top-down markers on a plain pitch; the intake named it.
  - Opta and Second Spectrum broadcast tracking graphics: numbered markers, thin trails, muted pitch, clean data overlays.
  - Telemetry dashboard: dense tabular figures, sober panel chrome, strong number contrast.
- Anti-goals: the cluttered overlays of the Football Manager 3D view; television sponsor clutter; arcade or cartoon styling; purple-blue gradients; nested cards; side accent stripes.

## 5. Scope and fidelity

Shipped quality for the match screen and its four weighted states. Sketch quality is acceptable for the half-time and full-time reports in the first deliverable.

State notes (each weighted state gets a deliberate treatment):

- Live play and goal moments: markers move on tick-bounded interpolation; a goal triggers a score-bug pulse and a banner that enters from the pitch edge and leaves within 1.5 seconds; the event feed highlights the goal row. Motion respects `prefers-reduced-motion`.
- Stoppage intervention: the tactics panel is available at all times; at a stoppage the pending-change list shows which queued changes apply now. A queued change shows a pending chip until it applies.
- Pre-match lineup screen: formation slots on a small pitch diagram; players dragged or clicked into slots; role fit and fitness shown per slot; validation messages inline; the kick-off control disabled until the lineup is legal.
- Loading, error, and recovery: a skeleton pitch while the engine starts; an error panel that names the failure and offers restart from the last stoppage or abandon; no spinner in content areas.

Component of record for the visual contract: `match-control`, the button primitive used for play, pause, speed, confirm substitution, and kick-off. Four sizes, three states, two themes; tokens in `02b-design.yaml`.

## 6. Recommended references

- `typeset.md`: always; the numbers need tabular numerals and a product scale ratio of 1.125 to 1.2.
- `animate.md`: goal banners, score-bug pulse, panel entry; sub-300 ms product motion, no bounce.
- `colorize.md`: committed strategy, OKLCH tokens, team kit colors mapped to safe on-pitch contrast.
- `layout.md`: the match screen is layout-heavy (pitch, header, feed, lineups, statistics).
- `optimize.md`: 60 frames per second on a canvas with 23 moving markers; off-main-thread rendering candidates.
- `harden.md`: focus rings, color never the only indicator (cards, fatigue), reduced-motion.

## Image gate

Unresolved. Plan resolves the image gate and writes `02c-craft.md`.
