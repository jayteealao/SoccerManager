// Tick-bounded interpolation.
//
// The engine computes every position; the viewer shows those positions and nothing else.
// A frame drawn between two ticks must lie on the segment joining them, never beyond the
// next one, because a position past the newest tick is movement the engine never computed.
// This is expressed as a constraint rather than as a comment: `t` is clamped before it is
// used, so the function cannot express a position outside the segment, whatever it is
// given. When ticks arrive faster than frames the scheduler moves the cursor; the fraction
// never leaves `[0, 1]`.

/// Writes the position at fraction `t` between ticks `a` and `b` into `out`.
///
/// `a`, `b` and `out` are wire-order component arrays. `t` outside `[0, 1]` is clamped,
/// so a late or a re-ordered frame slows the motion rather than inventing any.
export function between(a, b, t, out) {
  // A fraction that is not a number is treated as the start of the segment. It can only
  // arrive from a broken clock, and drawing the earlier tick again is the one answer that
  // shows nothing the engine did not compute.
  const f = Number.isFinite(t) ? (t < 0 ? 0 : t > 1 ? 1 : t) : 0;
  for (let i = 0; i < out.length; i += 1) {
    const from = a[i];
    out[i] = Math.round(from + (b[i] - from) * f);
  }
  return out;
}
