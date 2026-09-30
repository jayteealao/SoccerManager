// Broadcast Blue's colours for code that draws on a canvas or into an SVG string, where a
// CSS custom property cannot reach. `tests/colour.test.js` holds CANVAS equal to this skin's
// tokens.css through CANVAS_SOURCE.

/// The colours the canvas draws with: the four it measures kit colours against, then the
/// turf stripes, the ball's ring and the two keepers.
export const CANVAS = {
  pitch: '#1f8a2e',
  pitchLine: '#f7f8fa',
  onBrand: '#f7f8fa',
  fg: '#0a2250',
  stripe: '#21912f',
  stripe2: '#1d842c',
  ballRing: '#15161a',
  keeperHome: '#e3d34a',
  keeperHomeInk: '#111214',
  keeperAway: '#7c56d6',
  keeperAwayInk: '#f7f8fa',
};

/// The token each CANVAS entry mirrors.
export const CANVAS_SOURCE = {
  pitch: '--pitch',
  pitchLine: '--pitch-line',
  onBrand: '--ink',
  fg: '--cyan-ink',
  stripe: '--pitch-stripe',
  stripe2: '--pitch-stripe-2',
  ballRing: '--ball-ring',
  keeperHome: '--keeper-home',
  keeperHomeInk: '--keeper-home-ink',
  keeperAway: '--keeper-away',
  keeperAwayInk: '--keeper-away-ink',
};
