// Broadcast Blue's colours for code that draws on a canvas or into an SVG string, where a
// CSS custom property cannot reach. `tests/colour.test.js` holds CANVAS equal to this skin's
// tokens.css through CANVAS_SOURCE.

/// The four colours the canvas measures kit colours against.
export const CANVAS = {
  pitch: '#1f8a2e',
  pitchLine: '#f7f8fa',
  onBrand: '#f7f8fa',
  fg: '#0a2250',
};

/// The token each CANVAS entry mirrors.
export const CANVAS_SOURCE = {
  pitch: '--pitch',
  pitchLine: '--pitch-line',
  onBrand: '--ink',
  fg: '--cyan-ink',
};
