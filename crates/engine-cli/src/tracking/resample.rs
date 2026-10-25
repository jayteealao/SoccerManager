//! Resampling a tracked match from its native frame rate to the engine's 50 ticks a second.
//!
//! Each half starts at its first native frame with a ball position, instant 0; instant `k`
//! lies `20 k` ms later, up to the half's last native frame, never past it. A person's
//! position at an instant is the straight line between the two native frames around it when
//! both hold him and lie at most 100 ms apart, the frame's own position when the instant
//! falls on a frame, and none otherwise: the converter then parks him for that tick and lists
//! the spell. The ball the same, height included; without one the converter repeats the last
//! ball and lists the tick. The ball status and possession are those of the nearer frame, the
//! earlier on a tie, as an event goes to its nearest frame. Times are the native frames'
//! unrounded milliseconds.

use anyhow::ensure;

use super::{Frame, Half, TrackedMatch};

/// Milliseconds between two instants: 50 a second.
pub const STEP_MS: f64 = 20.0;
/// The widest gap between two native frames that a position is drawn across, in ms.
pub const MAX_GAP_MS: f64 = 100.0;

/// The value between `a` at `ta` and `b` at `tb` at time `t`; `a` itself on its own frame.
fn between<const N: usize>(
    a: Option<[f32; N]>,
    b: Option<[f32; N]>,
    (ta, tb, t): (f64, f64, f64),
) -> Option<[f32; N]> {
    if t == ta {
        return a;
    }
    let (a, b) = (a?, b?);
    if tb - ta > MAX_GAP_MS {
        return None;
    }
    let w = (t - ta) / (tb - ta);
    let mut out = [0.0f32; N];
    for (o, (x, y)) in out.iter_mut().zip(a.iter().zip(&b)) {
        *o = (f64::from(*x) + w * (f64::from(*y) - f64::from(*x))) as f32;
    }
    Some(out)
}

/// `m` at 50 frames a second; `times` holds each native frame's unrounded time per half and
/// `fps` the native rate.
pub fn resample(m: &TrackedMatch, times: &[Vec<f64>], fps: f64) -> anyhow::Result<TrackedMatch> {
    ensure!(
        times.len() == m.halves.len(),
        "match {}: {} halves but times for {}",
        m.match_id,
        m.halves.len(),
        times.len()
    );
    let mut halves = Vec::with_capacity(m.halves.len());
    for (half, t) in m.halves.iter().zip(times) {
        ensure!(
            t.len() == half.frames.len(),
            "match {}: {} has {} frames but {} times",
            m.match_id,
            half.section,
            half.frames.len(),
            t.len()
        );
        let Some(first) = half.frames.iter().position(|f| f.ball.is_some()) else {
            anyhow::bail!(
                "match {}: {} has no frame with a ball position",
                m.match_id,
                half.section
            );
        };
        let (t0, last) = (t[first], t[t.len() - 1]);
        let count = ((last - t0) / STEP_MS).floor() as usize + 1;
        let mut frames = Vec::with_capacity(count);
        let mut positions = vec![Vec::with_capacity(count); m.persons.len()];
        let mut i = first;
        for k in 0..count {
            let at = t0 + STEP_MS * k as f64;
            while i + 1 < t.len() && t[i + 1] <= at {
                i += 1;
            }
            let j = (i + 1).min(t.len() - 1);
            let span = (t[i], t[j], at);
            let (a, b) = (&half.frames[i], &half.frames[j]);
            let near = if t[j] - at < at - t[i] { b } else { a };
            frames.push(Frame {
                n: a.n,
                t_ms: at.round() as i64,
                ball: between(a.ball, b.ball, span),
                live: near.live,
                possession: near.possession,
            });
            for (p, out) in half.positions.iter().zip(&mut positions) {
                out.push(between(p[i], p[j], span));
            }
        }
        halves.push(Half {
            section: half.section.clone(),
            frames,
            positions,
        });
    }
    Ok(TrackedMatch {
        halves,
        ticks_per_frame: 1,
        resampled_from: Some(fps),
        ..m.clone()
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::pff;
    use super::*;

    fn native() -> pff::Native {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracking/pff");
        pff::read_native(&data, "9001").unwrap()
    }

    fn person(m: &TrackedMatch, id: &str) -> usize {
        m.persons.iter().position(|p| p.id == id).unwrap()
    }

    fn fixture() -> (pff::Native, TrackedMatch) {
        let n = native();
        let r = resample(&n.m, &n.times, n.fps).unwrap();
        (n, r)
    }

    /// The player running at a constant 5 m/s sits on his straight line at every instant.
    #[test]
    fn the_constant_speed_player_sits_on_his_line_at_every_instant() {
        let (n, r) = fixture();
        let p = person(&r, "90009");
        let t0 = n.times[0][2];
        for (k, at) in r.halves[0].positions[p].iter().enumerate() {
            let t = t0 + STEP_MS * k as f64;
            let want = -12.0 + 5.0 * (t - 100_000.0) / 1000.0;
            let got = at.unwrap();
            assert!((f64::from(got[0]) - want).abs() < 1e-4, "instant {k}");
            assert!((f64::from(got[1]) - 5.0).abs() < 1e-4, "instant {k}");
        }
        assert_eq!(r.ticks_per_frame, 1);
        assert_eq!(r.resampled_from, Some(29.97));
    }

    /// Instant 0 is each half's first ball frame; a half has floor(duration / 20 ms) + 1
    /// instants and none lies past its last frame.
    #[test]
    fn each_half_runs_from_its_first_ball_frame_to_its_last_frame() {
        let (n, r) = fixture();
        assert_eq!(r.halves[0].frames[0].n, 3002);
        assert_eq!(r.halves[1].frames[0].n, 12000);
        for (h, half) in r.halves.iter().enumerate() {
            let t = &n.times[h];
            let first = if h == 0 { 2 } else { 0 };
            let duration = t[t.len() - 1] - t[first];
            assert_eq!(
                half.frames.len(),
                (duration / 20.0).floor() as usize + 1,
                "half {h}"
            );
            let end = t[first] + 20.0 * (half.frames.len() - 1) as f64;
            assert!(end <= t[t.len() - 1]);
            assert_eq!(half.frames[0].t_ms, t[first].round() as i64);
        }
        assert_eq!(r.halves[0].frames.len(), 146);
        assert_eq!(r.halves[1].frames.len(), 149);
    }

    /// A player missing for 6 frames (0.2 s) has no position at the instants between his
    /// last frame before the gap and his first after it.
    #[test]
    fn a_gap_wider_than_the_limit_leaves_its_instants_empty() {
        let (n, r) = fixture();
        let p = person(&r, "90106");
        let t = &n.times[0];
        let t0 = t[2];
        for (k, at) in r.halves[0].positions[p].iter().enumerate() {
            let when = t0 + 20.0 * k as f64;
            let inside = when > t[29] && when < t[36];
            assert_eq!(at.is_none(), inside, "instant {k}");
        }
    }

    /// A pair of frames 33.4 ms apart is drawn across: the instant between them takes the
    /// weighted mean of the two positions; the ball status is the nearer frame's.
    #[test]
    fn an_instant_between_two_frames_takes_their_weighted_mean() {
        let (n, r) = fixture();
        let p = person(&r, "90102");
        let t = &n.times[0];
        let at = t[2] + 20.0;
        assert!(t[2] < at && at < t[3] && t[3] - t[2] < 33.4);
        let w = (at - t[2]) / (t[3] - t[2]);
        let (a, b) = (
            n.m.halves[0].positions[p][2].unwrap(),
            n.m.halves[0].positions[p][3].unwrap(),
        );
        let want = f64::from(a[0]) + w * (f64::from(b[0]) - f64::from(a[0]));
        let got = r.halves[0].positions[p][1].unwrap();
        assert!((f64::from(got[0]) - want).abs() < 1e-5);
        assert_eq!(r.halves[0].frames[1].live, n.m.halves[0].frames[3].live);
        // The OUT at frame 40 of half 1 is dead from the instant nearest it.
        let out = t[40];
        for (k, f) in r.halves[0].frames.iter().enumerate() {
            let when = t[2] + 20.0 * k as f64;
            if (when - out).abs() < 10.0 {
                assert_eq!(f.live, Some(false), "instant {k}");
            }
        }
        // The ball is missing at frames 40 and 41 of half 2: no ball between 39 and 42.
        let t2 = &n.times[1];
        for (k, f) in r.halves[1].frames.iter().enumerate() {
            let when = t2[0] + 20.0 * k as f64;
            let gone = when > t2[39] && when < t2[42];
            assert_eq!(f.ball.is_none(), gone, "instant {k}");
        }
    }
}
