// The interim light look's colours for code that draws on a canvas or into an SVG string,
// where a CSS custom property cannot reach. `tests/colour.test.js` holds CANVAS equal to
// this skin's tokens.css through CANVAS_SOURCE.

/// The colours the canvas draws with: the four it measures kit colours against, then the
/// turf stripes, the ball's ring and the two keepers.
export const CANVAS = {
  pitch: 'oklch(0.62 0.13 145)',
  pitchLine: 'oklch(0.97 0.01 145)',
  onBrand: 'oklch(0.985 0.006 250)',
  fg: 'oklch(0.24 0.02 250)',
  stripe: 'oklch(0.63 0.13 145)',
  stripe2: 'oklch(0.59 0.13 145)',
  ballRing: 'oklch(0.24 0.02 250)',
  keeperHome: 'oklch(0.85 0.16 95)',
  keeperHomeInk: 'oklch(0.24 0.02 250)',
  keeperAway: 'oklch(0.52 0.17 295)',
  keeperAwayInk: 'oklch(0.985 0.006 250)',
};

/// The token each CANVAS entry mirrors.
export const CANVAS_SOURCE = {
  pitch: '--pitch',
  pitchLine: '--pitch-line',
  onBrand: '--cyan-ink',
  fg: '--ink',
  stripe: '--pitch-stripe',
  stripe2: '--pitch-stripe-2',
  ballRing: '--ball-ring',
  keeperHome: '--keeper-home',
  keeperHomeInk: '--keeper-home-ink',
  keeperAway: '--keeper-away',
  keeperAwayInk: '--keeper-away-ink',
};

/// The touchline mark's four variants. Each is three colours: the field above the line, the
/// line itself, and the pitchside band below it.
export const MARK = {
  /// The primary mark, on the turf.
  primary: {
    field: 'oklch(0.62 0.13 145)',
    line: 'oklch(0.97 0.01 145)',
    band: 'oklch(0.28 0.03 250)',
  },
  /// On paper: a light tile with the brand carrying the line.
  reversed: {
    field: 'oklch(0.985 0.006 250)',
    line: 'oklch(0.55 0.17 250)',
    band: 'oklch(0.90 0.008 250)',
  },
  /// On a dark surface, where a light tile would glare.
  pitchside: {
    field: 'oklch(0.28 0.03 250)',
    line: 'oklch(0.97 0.01 145)',
    band: 'oklch(0.62 0.13 145)',
  },
  /// One ink, for the favicon at small sizes and for print.
  mono: {
    field: 'oklch(0.24 0.02 250)',
    line: 'oklch(0.985 0.006 250)',
    band: 'oklch(0.48 0.02 250)',
  },
};
