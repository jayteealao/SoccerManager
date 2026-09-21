# Product

## Name
[TODO] — the working name is not decided. The repository folder is named SoccerManager.

## Register
product

## Users
Football-management players who already know games such as Football Manager. Each player picks a lineup and tactics, then watches one match at a time against a computer-managed team, at a desk, in daylight, for 20 to 90 minutes. Players are fluent in football vocabulary (formations, roles, mentality, substitutions). No programming knowledge is assumed.

## Brand Personality
- **Pitchside** — the pitch is the most saturated object on the screen. Every other surface stays quieter than the turf.
- **Tabular** — numbers are the second voice: tabular numerals, ink on paper, dense but aligned.
- **Broadcast** — energy lives in the score bug, the goal banner, and the team colors. The panels do not compete with them.

## Tone
The match is live and the player is in charge. In the first five seconds the pitch moves, the clock runs, and the tactics control is visible without scrolling. The design feels engaged and mildly tense, serious about the numbers and warm about the game. It must not feel like an arcade game, a cartoon, a television sponsor board, or a spreadsheet with a pitch attached.

## Positive References
- Football Manager 2D classic view — top-down markers on a plain pitch; the view the player already reads.
- Opta and Second Spectrum tracking graphics — numbered markers, thin trails, a muted pitch, clean data overlays.
- Telemetry and trading-terminal dashboards — dense tabular figures, sober panel chrome, strong number contrast.

## Anti-references
- Football Manager 3D view — cluttered overlays hide the play.
- Television sponsor graphics — logos and clutter compete with the score.
- Arcade and cartoon football games — styling that undercuts a serious simulation.

## Strategic Principles
1. The pitch and the numbers share the screen. Neither hides behind a tab.
2. Legibility of small tabular numbers on a light background wins over banner motion. Prove the contrast before you add the motion.
3. Team kit colors appear only on the pitch and in the score. The brand color owns headers and controls.
4. Motion conveys state: a goal, a card, a substitution applied. Product transitions run under 300 ms; the goal banner leaves within 1.5 s. The reduced-motion preference disables all of it.

## Constraints
- Frontend: plain HTML, CSS, and JavaScript. No UI framework and no component library is chosen.
- Brand assets: none exist. Generate the logo, the palette, and any illustration programmatically in JavaScript as part of the product. Do not source them from image files.
- Team names, crests, and players are fictional and generated.
- Licenses: MIT-compatible or Apache-2.0-compatible dependencies only. Do not copy from GPL code.
- Light scene first: the screen must read in a lit room during the day. A darker pitchside treatment applies only to overlays drawn on the pitch.
- First platform: Windows 11 desktop, a browser page served by a local engine process.
