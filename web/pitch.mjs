// The pitch renderer. Nothing else in the page draws to the canvas.
//
// It owns one mapping: the engine's centred metre space, where x runs -52.5 to 52.5 and y
// runs -34 to 34, onto canvas pixels. The mapping is uniform in both axes, because the
// tile's 616 by 411 is slightly squarer than the pitch's 105 by 68 and stretching one axis
// would put a marker at a position the engine never computed. The playing area therefore
// sits inside the tile with a band of surround, which is also what the mark draws.

import { PLAYER_COUNT } from './decode.mjs';
import { safeKit } from './colour.mjs';

/// Pitch dimensions in metres, ported from `crates/engine/src/pitch.rs`.
export const LENGTH = 105;
export const WIDTH = 68;
export const GOAL_WIDTH = 7.32;
const HALF_LENGTH = LENGTH / 2;
const HALF_WIDTH = WIDTH / 2;

/// A sent-off player stands beside the pitch, past the touchline, at `parking_spot()` in
/// `crates/engine/src/pitch.rs`. These two constants and the test below are ported from there.
const PARKING_OFFSET = 3;

/// `true` when a position in metres is a parking spot, to the precision of the wire's
/// centimetres. A port of `is_parking_spot()` in `crates/engine/src/pitch.rs`: a sent-off
/// player is not on the pitch and is not drawn.
export function isParkingSpot(x, y) {
  const ax = Math.abs(x);
  return Math.abs(y + HALF_WIDTH + PARKING_OFFSET) < 0.01 && ax >= 9.99 && ax <= 20.01;
}

/// Markings, in metres, from the laws of the game.
const CENTRE_CIRCLE = 9.15;
const PENALTY_DEPTH = 16.5;
const PENALTY_WIDTH = 40.32;
const GOAL_AREA_DEPTH = 5.5;
const GOAL_AREA_WIDTH = 18.32;
const PENALTY_SPOT = 11;
const CORNER_ARC = 1;

/// The surround inside the tile, in pixels.
const MARGIN = 8;

/// Marker and ball sizes are fixed pixels rather than scaled metres: a player drawn to
/// scale is three pixels across and carries no shirt number.
const MARKER_RADIUS = 10;

/// The shirt number, in pixels. It is set here and not in the stylesheet, because a canvas
/// takes a font string; the family comes from the page.
const SHIRT_PX = 10;
const BALL_RADIUS = 4;
const TRAIL_TICKS = 12;

/// The token values the canvas draws with, read once from the stylesheet. They are never
/// re-read inside `draw()`: `getComputedStyle` forces a style recalculation, and a colour
/// that cannot change does not belong on a path that runs sixty times a second.
function readTokens() {
  const styles = getComputedStyle(document.documentElement);
  const value = (name) => {
    const found = styles.getPropertyValue(name).trim();
    if (!found) {
      throw new Error(`web/tokens.css does not define ${name}`);
    }
    return found;
  };
  return {
    turf: value('--tl-pitch'),
    line: value('--tl-pitch-line'),
    surround: value('--tl-pitchside-bg'),
    ink: value('--tl-fg'),
    // A canvas takes a font string and not a token, so the family is read back from the
    // page rather than written a second time here.
    face: getComputedStyle(document.body).fontFamily || 'sans-serif',
  };
}

export class Pitch {
  /// `canvas` is sized in CSS pixels by the stylesheet; the backing store is multiplied by
  /// the device pixel ratio here so the markings stay crisp on a scaled display.
  constructor(canvas, kits) {
    this.canvas = canvas;
    this.cssWidth = canvas.clientWidth || 616;
    this.cssHeight = canvas.clientHeight || 411;
    this.ratio = Math.max(1, Math.min(3, globalThis.devicePixelRatio || 1));
    canvas.width = Math.round(this.cssWidth * this.ratio);
    canvas.height = Math.round(this.cssHeight * this.ratio);

    // `alpha: false` lets the browser skip compositing a transparent layer every frame.
    this.ctx = canvas.getContext('2d', { alpha: false });
    this.ctx.scale(this.ratio, this.ratio);

    this.scale = Math.min(
      (this.cssWidth - MARGIN * 2) / LENGTH,
      (this.cssHeight - MARGIN * 2) / WIDTH
    );
    this.originX = (this.cssWidth - LENGTH * this.scale) / 2;
    this.originY = (this.cssHeight - WIDTH * this.scale) / 2;

    this.tokens = readTokens();
    this.kits = [safeKit(kits[0]), safeKit(kits[1])];
    this.trail = [];
    this.markings = this.renderMarkings();
  }

  /// Metres to canvas pixels. Centimetres arrive from the wire, so callers divide by 100.
  x(metres) {
    return this.originX + (metres + HALF_LENGTH) * this.scale;
  }

  y(metres) {
    return this.originY + (metres + HALF_WIDTH) * this.scale;
  }

  /// The static markings, drawn once into their own canvas and blitted each frame.
  renderMarkings() {
    const off = document.createElement('canvas');
    off.width = this.canvas.width;
    off.height = this.canvas.height;
    const ctx = off.getContext('2d', { alpha: false });
    ctx.scale(this.ratio, this.ratio);

    const { turf, line, surround } = this.tokens;

    ctx.fillStyle = surround;
    ctx.fillRect(0, 0, this.cssWidth, this.cssHeight);
    ctx.fillStyle = turf;
    ctx.fillRect(
      this.originX,
      this.originY,
      LENGTH * this.scale,
      WIDTH * this.scale
    );

    ctx.strokeStyle = line;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    // Touchlines and goal lines.
    ctx.rect(this.originX, this.originY, LENGTH * this.scale, WIDTH * this.scale);
    // Halfway line.
    ctx.moveTo(this.x(0), this.y(-HALF_WIDTH));
    ctx.lineTo(this.x(0), this.y(HALF_WIDTH));
    ctx.stroke();

    ctx.beginPath();
    ctx.arc(this.x(0), this.y(0), CENTRE_CIRCLE * this.scale, 0, Math.PI * 2);
    ctx.stroke();

    ctx.beginPath();
    ctx.arc(this.x(0), this.y(0), 1.5, 0, Math.PI * 2);
    ctx.fillStyle = line;
    ctx.fill();

    for (const side of [-1, 1]) {
      const goalLine = HALF_LENGTH * side;
      ctx.beginPath();
      ctx.rect(
        this.x(Math.min(goalLine, goalLine - PENALTY_DEPTH * side)),
        this.y(-PENALTY_WIDTH / 2),
        PENALTY_DEPTH * this.scale,
        PENALTY_WIDTH * this.scale
      );
      ctx.rect(
        this.x(Math.min(goalLine, goalLine - GOAL_AREA_DEPTH * side)),
        this.y(-GOAL_AREA_WIDTH / 2),
        GOAL_AREA_DEPTH * this.scale,
        GOAL_AREA_WIDTH * this.scale
      );
      ctx.stroke();

      ctx.beginPath();
      ctx.arc(this.x(goalLine - PENALTY_SPOT * side), this.y(0), 1.5, 0, Math.PI * 2);
      ctx.fill();

      // The goal itself, drawn just outside the goal line.
      ctx.beginPath();
      ctx.rect(
        this.x(Math.min(goalLine, goalLine + 1.8 * side)),
        this.y(-GOAL_WIDTH / 2),
        1.8 * this.scale,
        GOAL_WIDTH * this.scale
      );
      ctx.stroke();

      for (const edge of [-1, 1]) {
        ctx.beginPath();
        ctx.arc(
          this.x(goalLine),
          this.y((HALF_WIDTH - 0) * edge),
          CORNER_ARC * this.scale,
          0,
          Math.PI * 2
        );
        ctx.stroke();
      }
    }
    return off;
  }

  /// Draws one frame from wire components: ball x, y, height, then two per player.
  draw(components) {
    const ctx = this.ctx;
    ctx.drawImage(this.markings, 0, 0, this.cssWidth, this.cssHeight);

    const ballX = this.x(components[0] / 100);
    const ballY = this.y(components[1] / 100);
    this.trail.push(ballX, ballY);
    if (this.trail.length > TRAIL_TICKS * 2) {
      this.trail.splice(0, this.trail.length - TRAIL_TICKS * 2);
    }

    // One `fillStyle` change per team rather than one per marker, and one path for the
    // whole team's discs. The shirt numbers follow in a second pass for the same reason.
    // A sent-off player parked beside the pitch is skipped in both passes.
    const onPitch = (i) => !isParkingSpot(components[3 + i * 2] / 100, components[4 + i * 2] / 100);
    // The shirt-number face is the same for both teams: set once, parsed once a frame.
    ctx.font = `600 ${SHIRT_PX}px ${this.tokens.face}`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (const team of [0, 1]) {
      const kit = this.kits[team];
      const first = team * 11;
      ctx.fillStyle = kit.fill;
      ctx.beginPath();
      for (let i = first; i < first + 11; i += 1) {
        if (!onPitch(i)) {
          continue;
        }
        const px = this.x(components[3 + i * 2] / 100);
        const py = this.y(components[4 + i * 2] / 100);
        ctx.moveTo(px + MARKER_RADIUS, py);
        ctx.arc(px, py, MARKER_RADIUS, 0, Math.PI * 2);
      }
      ctx.fill();

      ctx.strokeStyle = kit.ring;
      ctx.lineWidth = 2;
      ctx.stroke();

      ctx.fillStyle = kit.number;
      for (let i = first; i < first + 11; i += 1) {
        if (!onPitch(i)) {
          continue;
        }
        const px = this.x(components[3 + i * 2] / 100);
        const py = this.y(components[4 + i * 2] / 100);
        ctx.fillText(String(i - first + 1), px, py + 0.5);
      }
    }

    // The ball's trail is a few short segments, each stroked at its own falling alpha (a
    // canvas path has one alpha, so the fade needs one stroke a segment). It belongs to the
    // ball alone: a trail on a player is arcade styling, not a broadcast view.
    const { line, ink } = this.tokens;
    ctx.strokeStyle = line;
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

    ctx.fillStyle = line;
    ctx.beginPath();
    ctx.arc(ballX, ballY, BALL_RADIUS, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = ink;
    ctx.lineWidth = 1;
    ctx.stroke();
  }

  /// Drops the trail, so a rewind does not draw a streak the engine never produced.
  clearTrail() {
    this.trail.length = 0;
  }
}

export { PLAYER_COUNT };
