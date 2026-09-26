// Kit colours, made safe before they reach the pitch.
//
// A team file holds any `#rrggbb`, and the shipped home club's trim colour is pure black,
// which the design canon bans outright. Every kit colour is therefore converted to OKLCH,
// its lightness pulled into a range that is neither black nor white, and converted back
// before it is drawn. The ring colour is then chosen by measurement, not by hope: the trim
// colour is used only when it measures 3:1 against the turf, and the pitch line is the
// fallback. Colour is never the only thing separating two teams.

/// The lightness band a kit colour is pulled into. Below it a marker reads as black on a
/// dark screen; above it, as white against the pitch lines.
export const LIGHTNESS_MIN = 0.2;
export const LIGHTNESS_MAX = 0.88;

/// The contrast a trim colour must reach against the turf to serve as the marker ring.
export const RING_MIN_CONTRAST = 3;

/// The token values the canvas needs. `web/tokens.css` is the single source for CSS, and
/// `colour.test.mjs` reads that file and fails when these three drift from it.
export const TOKENS = {
  pitch: 'oklch(0.62 0.13 145)',
  pitchLine: 'oklch(0.97 0.01 145)',
  onBrand: 'oklch(0.985 0.006 250)',
  fg: 'oklch(0.24 0.02 250)',
};

const DEG = Math.PI / 180;

/// Parses `#rgb` or `#rrggbb` into three channels in 0..1.
function parseHex(hex) {
  const text = String(hex).trim().toLowerCase();
  const digits = text.startsWith('#') ? text.slice(1) : text;
  const full =
    digits.length === 3
      ? digits
          .split('')
          .map((d) => d + d)
          .join('')
      : digits;
  if (!/^[0-9a-f]{6}$/.test(full)) {
    throw new Error(`not a hex colour: ${hex}`);
  }
  return [
    parseInt(full.slice(0, 2), 16) / 255,
    parseInt(full.slice(2, 4), 16) / 255,
    parseInt(full.slice(4, 6), 16) / 255,
  ];
}

/// Parses `oklch(L C H)`, with an optional `/ alpha` this page never needs.
function parseOklch(text) {
  const match = /^oklch\(\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)/i.exec(String(text).trim());
  if (!match) {
    throw new Error(`not an oklch colour: ${text}`);
  }
  return { l: Number(match[1]), c: Number(match[2]), h: Number(match[3]) };
}

const toLinear = (v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
const toGamma = (v) => (v <= 0.0031308 ? v * 12.92 : 1.055 * v ** (1 / 2.4) - 0.055);
const clamp01 = (v) => Math.min(1, Math.max(0, v));

/// A hex colour as OKLCH.
export function hexToOklch(hex) {
  const [r, g, b] = parseHex(hex).map(toLinear);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const lightness = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const a1 = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const b1 = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  const chroma = Math.hypot(a1, b1);
  const hue = chroma < 1e-6 ? 0 : (Math.atan2(b1, a1) / DEG + 360) % 360;
  return { l: lightness, c: chroma, h: hue };
}

/// An OKLCH colour as a hex string, with every channel held inside the sRGB cube.
export function oklchToHex({ l, c, h }) {
  const a = c * Math.cos(h * DEG);
  const b = c * Math.sin(h * DEG);
  const lc = (l + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const mc = (l - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const sc = (l - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const channels = [
    4.0767416621 * lc - 3.3077115913 * mc + 0.2309699292 * sc,
    -1.2684380046 * lc + 2.6097574011 * mc - 0.3413193965 * sc,
    -0.0041960863 * lc - 0.7034186147 * mc + 1.707614701 * sc,
  ];
  return `#${channels
    .map((v) => Math.round(clamp01(toGamma(clamp01(v))) * 255))
    .map((v) => v.toString(16).padStart(2, '0'))
    .join('')}`;
}

/// Pulls a lightness into the safe band.
export function clampLightness(l) {
  return Math.min(LIGHTNESS_MAX, Math.max(LIGHTNESS_MIN, l));
}

/// A kit colour with its lightness pulled into the safe band, as hex.
export function safeHex(colour) {
  const oklch = colour.startsWith('oklch') ? parseOklch(colour) : hexToOklch(colour);
  return oklchToHex({ ...oklch, l: clampLightness(oklch.l) });
}

/// The relative luminance of a colour, for the contrast ratio below.
function luminance(colour) {
  const hex = colour.startsWith('oklch') ? oklchToHex(parseOklch(colour)) : colour;
  const [r, g, b] = parseHex(hex).map(toLinear);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/// The contrast ratio between two colours, from 1 to 21.
export function contrast(a, b) {
  const la = luminance(a);
  const lb = luminance(b);
  const light = Math.max(la, lb);
  const dark = Math.min(la, lb);
  return (light + 0.05) / (dark + 0.05);
}

/// The three colours one club's markers are drawn with.
///
/// The fill is the shirt colour, made safe. The ring is the trim colour, made safe, but
/// only when it measures 3:1 against the turf; otherwise the pitch line carries it, so the
/// ring is always visible and the two teams are never told apart by hue alone. The shirt
/// number takes whichever of paper and ink contrasts more with the fill.
export function safeKit({ primary, secondary }) {
  const fill = safeHex(primary);
  const trim = safeHex(secondary);
  const pitch = oklchToHex(parseOklch(TOKENS.pitch));
  const ringIsTrim = contrast(trim, pitch) >= RING_MIN_CONTRAST;
  const paper = oklchToHex(parseOklch(TOKENS.onBrand));
  const ink = oklchToHex(parseOklch(TOKENS.fg));
  return {
    fill,
    ring: ringIsTrim ? trim : oklchToHex(parseOklch(TOKENS.pitchLine)),
    ringIsTrim,
    number: contrast(paper, fill) >= contrast(ink, fill) ? paper : ink,
  };
}
