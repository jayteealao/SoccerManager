// The pitch renderer. Nothing else in the viewer draws to the pitch canvas.
//
// It owns one mapping: the engine's centred metre space, where x runs along the ground's
// length and y across its width (-52.5 to 52.5 and -34 to 34 on the default 105 by 68
// ground), onto the box its canvas gives it. The box follows its column at a fixed aspect
// (742 by 312 on the match screen, 752 by 290 on the replay), and `resize()` takes each new
// size. The length maps with one uniform scale; the width maps with one fixed vertical
// factor, the sketch's top-down tilt. Both come from the default ground in the box, so a
// smaller ground draws smaller and centred, at the same tilt, and every drawn position stays
// a pure function of the engine's metres: the tilt projects a position, it never moves one.
//
// Every colour and the shirt-number face are read once, from the active skin's custom
// properties, when the renderer is built; `draw()` never reads a style.

import { PLAYER_COUNT } from './decode.js';
import { safeKit } from './colour.js';
import { CANVAS_SOURCE as BROADCAST_BLUE } from '../skins/broadcast-blue/palette.js';
import { CANVAS_SOURCE as INTERIM_LIGHT } from '../skins/interim-light/palette.js';

/// The default ground in metres, as `crates/engine/src/pitch.rs` gives it. A match plays on
/// the home team's ground, which the hello names when it is not 105 by 68.
export const LENGTH = 105;
export const WIDTH = 68;
export const DEFAULT_GROUND = Object.freeze({ length: LENGTH, width: WIDTH });

/// The ground a hello names: its `ground.length` and `ground.width`, each 105 or 68 when the
/// hello leaves it out.
export function groundOf(hello) {
  return Object.freeze({
    length: hello?.['ground.length'] ?? LENGTH,
    width: hello?.['ground.width'] ?? WIDTH,
  });
}
export const GOAL_WIDTH = 7.32;

/// The match screen's pitch box at the standard step, in CSS pixels: the reference aspect and
/// the default size. A canvas may name another (the replay's is 752 by 290), and every box
/// follows its column at its aspect.
export const BOX = Object.freeze({ width: 742, height: 312 });

/// A sent-off player stands beside the pitch, past the touchline, at `parking_spot()` in
/// `crates/engine/src/pitch.rs`. This constant and the test below are ported from there.
const PARKING_OFFSET = 3;

/// `true` when a position in metres is a parking spot beside a ground `width` metres wide, to
/// the precision of the wire's centimetres. A port of `is_parking_spot()` in
/// `crates/engine/src/pitch.rs`: a sent-off player is not on the pitch and is not drawn.
export function isParkingSpot(x, y, width = WIDTH) {
  const ax = Math.abs(x);
  return Math.abs(y + width / 2 + PARKING_OFFSET) < 0.01 && ax >= 9.99 && ax <= 20.01;
}

/// Markings, in metres, from the laws of the game.
const CENTRE_CIRCLE = 9.15;
const PENALTY_DEPTH = 16.5;
const PENALTY_WIDTH = 40.32;
const GOAL_AREA_DEPTH = 5.5;
const GOAL_AREA_WIDTH = 18.32;
const PENALTY_SPOT = 11;
const PENALTY_ARC = 9.15;
const CORNER_ARC = 1;

/// The inset of the touchlines inside the box, in pixels: the outer line sits on the first
/// pixel, as the sketch draws it.
const PAD = 1;
/// The stripes mown across the length.
const STRIPES = 14;
/// The markings are drawn at this opacity of the pitch-line colour.
const LINE_ALPHA = 0.55;
const LINE_WIDTH = 1.3;

/// Markers are fixed pixels rather than scaled metres: a player drawn to scale is a few
/// pixels across and carries no shirt number. 17 px across with a 1.5 px ring, as drawn.
const MARKER_RADIUS = 8.5;
const RING_WIDTH = 1.5;
const SHIRT_PX = 8.5;
const BALL_RADIUS = 4.5;
const BALL_RING = 2;
const TRAIL_TICKS = 12;

/// The squad slot of each side's keeper in the wire's player order.
export const KEEPERS = Object.freeze([0, 11]);

/// The mapping from metres to box pixels for a box of `width` by `height` and a ground of
/// `length` by `breadth` metres. The scale and the tilt are those that make the default
/// ground fill the box, so every ground keeps the sketch's top-down tilt and a smaller ground
/// draws smaller; a ground too large for the box at that scale shrinks to fit, its tilt
/// kept. The drawn ground is centred in the box; `rect` is where it lies, in box pixels.
/// `tilt` is the vertical factor against the horizontal scale.
export function projection(width = BOX.width, height = BOX.height, length = LENGTH, breadth = WIDTH) {
  const inner = { width: width - PAD * 2, height: height - PAD * 2 };
  const fit = Math.min(1, LENGTH / length, WIDTH / breadth);
  const sx = (inner.width / LENGTH) * fit;
  const sy = (inner.height / WIDTH) * fit;
  const rect = Object.freeze({
    left: PAD + (inner.width - length * sx) / 2,
    top: PAD + (inner.height - breadth * sy) / 2,
    width: length * sx,
    height: breadth * sy,
  });
  return {
    sx,
    sy,
    tilt: sy / sx,
    rect,
    x: (metres) => rect.left + (metres + length / 2) * sx,
    y: (metres) => rect.top + (metres + breadth / 2) * sy,
  };
}

/// The token each canvas colour mirrors, per skin: each skin's palette names them (the four
/// `safeKit` measures against differ between skins), and `colour.test.js` holds the palette
/// equal to that skin's tokens.css. The line and the shirt-number face share one name.
const SOURCES = { 'broadcast-blue': BROADCAST_BLUE, 'interim-light': INTERIM_LIGHT };
const DRAWN = { line: '--pitch-line', face: '--fd' };

/// The token values the canvas draws with, read once from the active skin: the skin that
/// `data-theme` on the root element names.
export function readTokens(doc = globalThis.document) {
  const styles = getComputedStyle(doc.documentElement);
  const value = (name) => {
    const found = styles.getPropertyValue(name).trim();
    if (!found) {
      throw new Error(`the active skin does not define ${name}`);
    }
    return found;
  };
  const source = SOURCES[doc.documentElement.dataset.theme] ?? BROADCAST_BLUE;
  const tokens = {};
  for (const [key, name] of Object.entries({ ...DRAWN, ...source })) {
    tokens[key] = value(name);
  }
  return tokens;
}

/// The most backing-store pixels per CSS pixel, as on a 3x display.
const MAX_RATIO = 3;

export class Pitch {
  /// `canvas` is sized in CSS pixels by its component; the backing store holds the box in
  /// device pixels, so the markings stay crisp on a scaled display and in a zoomed page.
  /// `tokens` are the active skin's values (`readTokens`); `kits` the two clubs' kit
  /// colours; `width` and `height` the box in CSS pixels, the match screen's unless given;
  /// `deviceWidth` and `deviceHeight` the box in device pixels, the CSS box times `ratio`
  /// unless given; `ground` the match's ground in metres (`groundOf`), 105 by 68 unless given.
  constructor(
    canvas,
    kits,
    tokens,
    {
      ratio = globalThis.devicePixelRatio || 1,
      width = BOX.width,
      height = BOX.height,
      deviceWidth,
      deviceHeight,
      ground = DEFAULT_GROUND,
    } = {}
  ) {
    this.canvas = canvas;
    // `alpha: false` lets the browser skip compositing a transparent layer every frame.
    this.ctx = canvas.getContext('2d', { alpha: false });
    this.ground = ground;
    this.tokens = tokens;
    this.kits = [safeKit(kits[0], tokens), safeKit(kits[1], tokens)];
    this.trail = [];
    /// The frame on show, which a resize draws again; null before the first frame.
    this.shown = null;
    this.frame = null;
    const r = Math.max(1, Math.min(MAX_RATIO, ratio));
    this.size({
      width,
      height,
      deviceWidth: deviceWidth ?? Math.round(width * r),
      deviceHeight: deviceHeight ?? Math.round(height * r),
    });
  }

  /// Takes the box's new size, in CSS pixels and in device pixels: the backing store becomes
  /// the device pixels (at most three per CSS pixel), the mapping is rebuilt for the box by
  /// the same `projection()`, and the markings are drawn again. The trail moves with the
  /// ground, and the frame on show is drawn again at the new size, so a paused, full-time or
  /// replay frame stays on the pitch.
  resize(box) {
    const before = this.map;
    this.size(box);
    for (let i = 0; i < this.trail.length; i += 2) {
      this.trail[i] = this.map.rect.left + (this.trail[i] - before.rect.left) * (this.map.sx / before.sx);
      this.trail[i + 1] = this.map.rect.top + (this.trail[i + 1] - before.rect.top) * (this.map.sy / before.sy);
    }
    if (this.shown) {
      this.draw(this.shown, { repaint: true });
    } else {
      this.ctx.drawImage(this.markings, 0, 0, this.cssWidth, this.cssHeight);
    }
  }

  size({ width, height, deviceWidth, deviceHeight }) {
    this.cssWidth = width;
    this.cssHeight = height;
    this.ratioX = Math.min(MAX_RATIO, deviceWidth / width);
    this.ratioY = Math.min(MAX_RATIO, deviceHeight / height);
    this.canvas.width = Math.round(width * this.ratioX);
    this.canvas.height = Math.round(height * this.ratioY);
    // Setting the canvas's size cleared its transform.
    this.ctx.setTransform(this.ratioX, 0, 0, this.ratioY, 0, 0);
    this.map = projection(width, height, this.ground.length, this.ground.width);
    this.markings = this.renderMarkings();
  }

  /// Metres to box pixels. Centimetres arrive from the wire, so callers divide by 100.
  x(metres) {
    return this.map.x(metres);
  }

  y(metres) {
    return this.map.y(metres);
  }

  /// Where the ground is drawn: the ground in metres, the box, the drawn rectangle in box
  /// pixels, the two scales and the tilt, and the backing store's size in device pixels.
  /// The browser tests read it through the test hook.
  geometry() {
    const { sx, sy, tilt, rect } = this.map;
    return {
      ground: { length: this.ground.length, width: this.ground.width },
      box: { width: this.cssWidth, height: this.cssHeight },
      backing: { width: this.canvas.width, height: this.canvas.height },
      rect: { ...rect },
      sx,
      sy,
      tilt,
    };
  }

  /// The turf and the markings, drawn once into their own canvas and blitted each frame. The
  /// turf fills the box; the lines are the ground's.
  renderMarkings() {
    const off = this.canvas.ownerDocument.createElement('canvas');
    off.width = this.canvas.width;
    off.height = this.canvas.height;
    const ctx = off.getContext('2d', { alpha: false });
    ctx.scale(this.ratioX, this.ratioY);
    const { stripe, stripe2, line } = this.tokens;
    const { sx, sy } = this.map;

    const band = this.cssWidth / STRIPES;
    for (let i = 0; i < STRIPES; i += 1) {
      ctx.fillStyle = i % 2 === 0 ? stripe : stripe2;
      ctx.fillRect(Math.floor(i * band), 0, Math.ceil(band) + 1, this.cssHeight);
    }

    ctx.globalAlpha = LINE_ALPHA;
    ctx.strokeStyle = line;
    ctx.fillStyle = line;
    ctx.lineWidth = LINE_WIDTH;
    const { length, width } = this.ground;
    const half = { l: length / 2, w: width / 2 };
    ctx.beginPath();
    // Touchlines and goal lines, then the halfway line.
    ctx.rect(this.x(-half.l), this.y(-half.w), length * sx, width * sy);
    ctx.moveTo(this.x(0), this.y(-half.w));
    ctx.lineTo(this.x(0), this.y(half.w));
    ctx.stroke();

    // The centre circle, projected: the tilt makes it an ellipse.
    ctx.beginPath();
    ctx.ellipse(this.x(0), this.y(0), CENTRE_CIRCLE * sx, CENTRE_CIRCLE * sy, 0, 0, Math.PI * 2);
    ctx.stroke();
    ctx.beginPath();
    ctx.arc(this.x(0), this.y(0), 2, 0, Math.PI * 2);
    ctx.fill();

    for (const side of [-1, 1]) {
      const goalLine = half.l * side;
      ctx.beginPath();
      ctx.rect(
        this.x(Math.min(goalLine, goalLine - PENALTY_DEPTH * side)),
        this.y(-PENALTY_WIDTH / 2),
        PENALTY_DEPTH * sx,
        PENALTY_WIDTH * sy
      );
      ctx.rect(
        this.x(Math.min(goalLine, goalLine - GOAL_AREA_DEPTH * side)),
        this.y(-GOAL_AREA_WIDTH / 2),
        GOAL_AREA_DEPTH * sx,
        GOAL_AREA_WIDTH * sy
      );
      ctx.stroke();

      // The penalty arc: the part of a 9.15 m circle round the spot outside the area.
      const spot = goalLine - PENALTY_SPOT * side;
      const reach = Math.acos((PENALTY_DEPTH - PENALTY_SPOT) / PENALTY_ARC);
      const facing = side < 0 ? 0 : Math.PI;
      ctx.beginPath();
      ctx.ellipse(this.x(spot), this.y(0), PENALTY_ARC * sx, PENALTY_ARC * sy, 0, facing - reach, facing + reach);
      ctx.stroke();

      ctx.beginPath();
      ctx.arc(this.x(spot), this.y(0), 1.5, 0, Math.PI * 2);
      ctx.fill();

      for (const edge of [-1, 1]) {
        ctx.beginPath();
        ctx.ellipse(this.x(goalLine), this.y(half.w * edge), CORNER_ARC * sx, CORNER_ARC * sy, 0, 0, Math.PI * 2);
        ctx.stroke();
      }
    }
    ctx.globalAlpha = 1;
    return off;
  }

  /// Draws one frame from wire components: ball x, y, height, then two per player.
  draw(components, { repaint = false } = {}) {
    const ctx = this.ctx;
    ctx.drawImage(this.markings, 0, 0, this.cssWidth, this.cssHeight);
    if (!repaint) {
      // One buffer, filled again each frame, so drawing allocates nothing.
      this.frame ??= new Int16Array(components.length);
      this.frame.set(components);
      this.shown = this.frame;
    }

    const ballX = this.x(components[0] / 100);
    const ballY = this.y(components[1] / 100);
    if (!repaint) {
      this.trail.push(ballX, ballY);
      if (this.trail.length > TRAIL_TICKS * 2) {
        this.trail.splice(0, this.trail.length - TRAIL_TICKS * 2);
      }
    }

    // One `fillStyle` change per team rather than one per marker, and one path for the
    // whole team's outfield discs; the keeper takes the skin's keeper colour. The shirt
    // numbers follow in a second pass. A sent-off player parked beside the pitch is skipped.
    const onPitch = (i) =>
      !isParkingSpot(components[3 + i * 2] / 100, components[4 + i * 2] / 100, this.ground.width);
    const at = (i) => [this.x(components[3 + i * 2] / 100), this.y(components[4 + i * 2] / 100)];
    const t = this.tokens;
    ctx.font = `700 ${SHIRT_PX}px ${t.face}`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.lineWidth = RING_WIDTH;
    for (const team of [0, 1]) {
      const kit = this.kits[team];
      const first = team * 11;
      const keeper = KEEPERS[team];
      ctx.strokeStyle = kit.ring;
      ctx.fillStyle = kit.fill;
      ctx.beginPath();
      for (let i = first; i < first + 11; i += 1) {
        if (i === keeper || !onPitch(i)) {
          continue;
        }
        const [px, py] = at(i);
        ctx.moveTo(px + MARKER_RADIUS, py);
        ctx.arc(px, py, MARKER_RADIUS, 0, Math.PI * 2);
      }
      ctx.fill();
      ctx.stroke();
      if (onPitch(keeper)) {
        const [px, py] = at(keeper);
        ctx.fillStyle = team === 0 ? t.keeperHome : t.keeperAway;
        ctx.beginPath();
        ctx.arc(px, py, MARKER_RADIUS, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
      }

      ctx.fillStyle = kit.number;
      for (let i = first; i < first + 11; i += 1) {
        if (!onPitch(i)) {
          continue;
        }
        if (i === keeper) {
          ctx.fillStyle = team === 0 ? t.keeperHomeInk : t.keeperAwayInk;
        }
        const [px, py] = at(i);
        ctx.fillText(String(i - first + 1), px, py + 0.5);
        if (i === keeper) {
          ctx.fillStyle = kit.number;
        }
      }
    }

    // The ball's trail is a few short segments, each stroked at its own falling alpha. It
    // belongs to the ball alone: a trail on a player is arcade styling, not a broadcast view.
    ctx.strokeStyle = t.line;
    ctx.lineWidth = 2;
    ctx.lineCap = 'round';
    for (let i = 2; i < this.trail.length; i += 2) {
      ctx.globalAlpha = (i / this.trail.length) * 0.5;
      ctx.beginPath();
      ctx.moveTo(this.trail[i - 2], this.trail[i - 1]);
      ctx.lineTo(this.trail[i], this.trail[i + 1]);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;

    ctx.fillStyle = t.line;
    ctx.strokeStyle = t.ballRing;
    ctx.lineWidth = BALL_RING;
    ctx.beginPath();
    ctx.arc(ballX, ballY, BALL_RADIUS, 0, Math.PI * 2);
    ctx.stroke();
    ctx.fill();
  }

  /// The turf and markings alone, with no player: the pitch before the first tick.
  clear() {
    this.ctx.drawImage(this.markings, 0, 0, this.cssWidth, this.cssHeight);
    this.trail.length = 0;
    this.shown = null;
  }

  /// Drops the trail, so a rewind does not draw a streak the engine never produced.
  clearTrail() {
    this.trail.length = 0;
  }
}

export { PLAYER_COUNT };
