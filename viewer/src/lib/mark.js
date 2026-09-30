// The touchline mark, drawn rather than loaded.
//
// No asset file exists in this product and none is added: every mark is generated here.
// The mark is the touchline itself — the field above, the white line across it, and the
// pitchside band below — with one marker and the ball on the field. A club crest is the
// same drawing with the club's kit colours in place of the turf, so the crest and the
// markers on the pitch cannot drift apart.

import { MARK } from '../skins/interim-light/palette.js';

/// The line sits at this fraction of the tile height, measured from the top.
export const LINE_AT = 0.58;

/// The corner radius as a fraction of the tile size.
export const RADIUS_AT = 0.21;

/// The four variants DESIGN.md names. Each is three colours: the field above the line,
/// the line itself, and the pitchside band below it.
///
/// The values live in the interim light skin's `palette.js`, beside its tokens.
/// `colours(document)` below reads the active skin's tokens instead, which is what the page
/// does; these are the fallback for a caller with no document, such as a test.
export const PRIMARY = MARK.primary;

/// On paper: a light tile with the brand carrying the line.
export const REVERSED = MARK.reversed;

/// On a dark surface, where a light tile would glare.
export const PITCHSIDE = MARK.pitchside;

/// One ink, for the favicon at small sizes and for print.
export const MONO = MARK.mono;

/// The primary variant built from the live token values, so the mark and the pitch can
/// never drift apart.
export function colours(doc = globalThis.document) {
  if (!doc) {
    return PRIMARY;
  }
  const styles = doc.defaultView.getComputedStyle(doc.documentElement);
  const value = (name, fallback) => styles.getPropertyValue(name).trim() || fallback;
  return {
    field: value('--pitch', PRIMARY.field),
    line: value('--pitch-line', PRIMARY.line),
    band: value('--chip', PRIMARY.band),
  };
}

/// Every number the mark is made of, from the tile size alone. Pure, so a test can hold
/// the drawing to a shape without a canvas.
export function markGeometry(size) {
  return {
    size,
    radius: size * RADIUS_AT,
    lineY: size * LINE_AT,
    lineHeight: Math.max(1, size * 0.055),
    marker: { x: size * 0.34, y: size * 0.33, r: size * 0.11 },
    ball: { x: size * 0.66, y: size * 0.41, r: size * 0.055 },
    // One faint pitch marking, as DESIGN.md asks: the arc of a centre circle at the top
    // edge of the field, which reads as a pitch at 16 pixels and at 256.
    arc: { x: size * 0.5, y: size * 0.02, r: size * 0.26 },
  };
}

/// Draws the mark into a 2D canvas context, with its top-left corner at `x`, `y`.
export function drawMark(ctx, x, y, size, colours = PRIMARY) {
  const g = markGeometry(size);
  ctx.save();
  ctx.translate(x, y);
  ctx.beginPath();
  ctx.roundRect(0, 0, size, size, g.radius);
  ctx.clip();

  ctx.fillStyle = colours.field;
  ctx.fillRect(0, 0, size, g.lineY);
  ctx.fillStyle = colours.band;
  ctx.fillRect(0, g.lineY, size, size - g.lineY);
  ctx.fillStyle = colours.line;
  ctx.fillRect(0, g.lineY - g.lineHeight, size, g.lineHeight);

  ctx.strokeStyle = colours.line;
  ctx.globalAlpha = 0.35;
  ctx.lineWidth = Math.max(1, size * 0.03);
  ctx.beginPath();
  ctx.arc(g.arc.x, g.arc.y, g.arc.r, 0, Math.PI * 2);
  ctx.stroke();
  ctx.globalAlpha = 1;

  ctx.fillStyle = colours.line;
  ctx.beginPath();
  ctx.arc(g.marker.x, g.marker.y, g.marker.r, 0, Math.PI * 2);
  ctx.fill();
  ctx.beginPath();
  ctx.arc(g.ball.x, g.ball.y, g.ball.r, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();
}

/// The same mark as an SVG document, for the favicon and for any place that wants an
/// image without a file.
export function markSvg(size, colours = PRIMARY) {
  const g = markGeometry(size);
  return [
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}" width="${size}" height="${size}">`,
    `<clipPath id="c"><rect width="${size}" height="${size}" rx="${g.radius}"/></clipPath>`,
    `<g clip-path="url(#c)">`,
    `<rect width="${size}" height="${g.lineY}" fill="${colours.field}"/>`,
    `<rect y="${g.lineY}" width="${size}" height="${size - g.lineY}" fill="${colours.band}"/>`,
    `<rect y="${g.lineY - g.lineHeight}" width="${size}" height="${g.lineHeight}" fill="${colours.line}"/>`,
    `<circle cx="${g.arc.x}" cy="${g.arc.y}" r="${g.arc.r}" fill="none" stroke="${colours.line}" stroke-opacity="0.35" stroke-width="${Math.max(1, size * 0.03)}"/>`,
    `<circle cx="${g.marker.x}" cy="${g.marker.y}" r="${g.marker.r}" fill="${colours.line}"/>`,
    `<circle cx="${g.ball.x}" cy="${g.ball.y}" r="${g.ball.r}" fill="${colours.line}"/>`,
    `</g></svg>`,
  ].join('');
}

/// Points the document's icon at a freshly drawn mark. No image file is ever fetched.
export function setFavicon(document, size = 32, colours = PRIMARY) {
  const link = document.querySelector('link[rel="icon"]') ?? document.createElement('link');
  link.rel = 'icon';
  link.type = 'image/svg+xml';
  link.href = `data:image/svg+xml,${encodeURIComponent(markSvg(size, colours))}`;
  if (!link.parentNode) {
    document.head.appendChild(link);
  }
  return link;
}
