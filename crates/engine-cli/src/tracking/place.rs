//! Event placement: each event of a tracked match onto a frame, so onto a tick.
//!
//! - An event with an event time goes to the frame nearest that time, the earlier frame on
//!   a tie.
//! - An event with only a game clock (its half and the milliseconds since that half's first
//!   ball frame) goes to the frame nearest that half's first ball frame plus the clock plus
//!   the match's clock offset.
//! - The clock offset is fitted once per match on the passes: for each pass with an event
//!   time, the offset from its event time to the first frame within 1 s either side at which
//!   the ball's frame-to-frame speed rises past 3 m/s; the match's offset is the median, so
//!   a mis-logged pass does not move it. Its pass count and interquartile range go into the
//!   side file.

use super::convert::nearest;
use super::side::ClockOffset;
use super::{Event, EventKind, TrackedMatch};

/// The window either side of a pass's event time in which its kick is looked for, in ms.
const WINDOW_MS: i64 = 1000;
/// The ball speed a kick rises past, in m/s.
const KICK_MPS: f64 = 3.0;

/// How an event was placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    EventTime,
    ClockOffset,
}

impl How {
    /// The side file's word.
    pub fn word(self) -> &'static str {
        match self {
            How::EventTime => "event-time",
            How::ClockOffset => "clock-offset",
        }
    }
}

/// Where an event went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub tick: u32,
    pub half: usize,
    pub frame: usize,
    pub how: How,
}

/// The ball's frame-to-frame speed at frame `i` of half `h`, in m/s; `None` without a ball
/// in this frame or the one before.
fn ball_speed(m: &TrackedMatch, h: usize, i: usize) -> Option<f64> {
    let frames = &m.halves[h].frames;
    let (a, b) = (frames.get(i.checked_sub(1)?)?, frames.get(i)?);
    let (pa, pb) = (a.ball?, b.ball?);
    let dt = (b.t_ms - a.t_ms) as f64 / 1000.0;
    if dt <= 0.0 {
        return None;
    }
    let (dx, dy) = (f64::from(pb[0] - pa[0]), f64::from(pb[1] - pa[1]));
    Some(dx.hypot(dy) / dt)
}

/// The match's clock offset fitted on its passes; `None` when no pass gives one.
pub fn fit_offset(m: &TrackedMatch, frames: &[(i64, u32, usize, usize)]) -> Option<ClockOffset> {
    let mut offsets: Vec<i64> = Vec::new();
    for e in &m.events {
        let (EventKind::Pass { .. }, Some(t)) = (&e.kind, e.time_ms) else {
            continue;
        };
        let from = frames.partition_point(|f| f.0 < t - WINDOW_MS);
        let kick = frames[from..]
            .iter()
            .take_while(|f| f.0 <= t + WINDOW_MS)
            .find(|&&(_, _, h, i)| {
                let now = ball_speed(m, h, i);
                let before = i.checked_sub(1).and_then(|j| ball_speed(m, h, j));
                now.is_some_and(|s| s > KICK_MPS) && before.is_some_and(|s| s <= KICK_MPS)
            });
        if let Some(&(ft, ..)) = kick {
            offsets.push(ft - t);
        }
    }
    if offsets.is_empty() {
        return None;
    }
    offsets.sort_unstable();
    Some(ClockOffset {
        ms: quantile(&offsets, 0.5),
        passes: u32::try_from(offsets.len()).unwrap_or(u32::MAX),
        iqr_ms: quantile(&offsets, 0.75) - quantile(&offsets, 0.25),
    })
}

/// The `q` quantile of sorted values, by linear interpolation.
fn quantile(sorted: &[i64], q: f64) -> f64 {
    let pos = q * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    let frac = pos - lo as f64;
    sorted[lo] as f64 * (1.0 - frac) + sorted[hi] as f64 * frac
}

/// The placement of `e`: by its event time, else by its game clock and the offset; `None`
/// when it has neither, or a clock and no offset.
pub fn place(
    m: &TrackedMatch,
    frames: &[(i64, u32, usize, usize)],
    offset: Option<&ClockOffset>,
    e: &Event,
) -> Option<Placement> {
    if let Some(t) = e.time_ms {
        let (tick, half, frame) = nearest(frames, t)?;
        return Some(Placement {
            tick,
            half,
            frame,
            how: How::EventTime,
        });
    }
    let (half, clock_ms) = e.clock?;
    let offset = offset?;
    // The half's first ball frame is its first used frame.
    let start = frames.iter().find(|f| f.2 == half)?.0;
    let t = start + clock_ms + offset.ms.round() as i64;
    // The nearest frame of that half.
    let own: Vec<(i64, u32, usize, usize)> =
        frames.iter().copied().filter(|f| f.2 == half).collect();
    let (tick, half, frame) = nearest(&own, t)?;
    Some(Placement {
        tick,
        half,
        frame,
        how: How::ClockOffset,
    })
}

#[cfg(test)]
mod tests {
    use super::super::convert::frame_ticks;
    use super::super::dfl;
    use super::*;
    use std::path::Path;

    fn fixture() -> TrackedMatch {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracking/dfl");
        dfl::read(&dfl::find(&data, "DFL-MAT-FIX001").unwrap()).unwrap()
    }

    fn timed(t_ms: i64) -> Event {
        Event {
            kind: EventKind::Foul,
            team: None,
            person: None,
            time_ms: Some(t_ms),
            clock: None,
            decision_ms: None,
            recipient: None,
        }
    }

    #[test]
    fn an_event_time_goes_to_the_nearest_frame_and_a_tie_to_the_earlier() {
        let m = fixture();
        let frames = frame_ticks(&m).unwrap();
        let t0 = m.halves[0].frames[2].t_ms; // the first ball frame: tick 0
        for (dt, tick) in [(0, 0), (19, 0), (20, 0), (21, 2), (39, 2), (60, 2), (61, 4)] {
            let p = place(&m, &frames, None, &timed(t0 + dt)).unwrap();
            assert_eq!((p.tick, p.how), (tick, How::EventTime), "{dt} ms");
        }
        // Before the first ball frame and after the last: the end frames.
        assert_eq!(place(&m, &frames, None, &timed(t0 - 5000)).unwrap().tick, 0);
        let end = m.halves[1].frames[59].t_ms;
        assert_eq!(
            place(&m, &frames, None, &timed(end + 5000)).unwrap().tick,
            234
        );
    }

    /// The fixture's passes move the ball 0.48 s after their event time.
    #[test]
    fn the_clock_offset_fitted_on_the_passes_is_the_planted_one() {
        let m = fixture();
        let frames = frame_ticks(&m).unwrap();
        let fit = fit_offset(&m, &frames).unwrap();
        assert_eq!(
            fit,
            ClockOffset {
                ms: 480.0,
                passes: 2,
                iqr_ms: 0.0
            }
        );
    }

    /// An event with only a game clock lands on the frame at the clock plus the offset: 400
    /// ms into half 1 plus 480 ms is 22 frames after the first ball frame, tick 44.
    #[test]
    fn an_event_with_only_a_game_clock_lands_at_the_clock_plus_the_offset() {
        let m = fixture();
        let frames = frame_ticks(&m).unwrap();
        let fit = fit_offset(&m, &frames);
        let mut e = timed(0);
        e.time_ms = None;
        e.clock = Some((0, 400));
        let p = place(&m, &frames, fit.as_ref(), &e).unwrap();
        assert_eq!((p.tick, p.how), (44, How::ClockOffset));
        // In half 2, 1 s after its first ball frame (tick 116) plus 0.48 s: 37 frames on.
        e.clock = Some((1, 1000));
        assert_eq!(place(&m, &frames, fit.as_ref(), &e).unwrap().tick, 116 + 74);
        // No offset, or neither a time nor a clock: no placement.
        assert_eq!(place(&m, &frames, None, &e), None);
        e.clock = None;
        assert_eq!(place(&m, &frames, fit.as_ref(), &e), None);
    }

    #[test]
    fn a_match_without_a_kick_near_its_passes_has_no_offset() {
        let mut m = fixture();
        for h in &mut m.halves {
            for f in &mut h.frames {
                if f.ball.is_some() {
                    f.ball = Some([0.0, 0.0, 0.0]);
                }
            }
        }
        let frames = frame_ticks(&m).unwrap();
        assert_eq!(fit_offset(&m, &frames), None);
    }
}
