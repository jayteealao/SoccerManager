//! The pace check: fast running and sprints of a tracked match on its native frames and on
//! the report's own frames after conversion, by one rule, so a gap between the two is the
//! converter's and a gap that both share comes with the data.
//!
//! Every outfield player's distance between two frames of a half that both hold him goes
//! to the running zone of its speed, with the shared report's zone bounds: fast running
//! 5.5 to 7 m/s, sprint 7 m/s and over. A native step longer than 100 ms, or a converted
//! step longer than one report frame, adds nothing (the player was not tracked between);
//! neither does a step faster than the report's track break (1 m in one report frame,
//! 25 m/s). The native figures also come from the rows the provider saw on camera only.
//!
//! `engine-cli tracking pace` prints the figures of each match and of all, and whether the
//! converted figures sit within 5 percent of the native ones.

use std::path::Path;

use super::{Group, TrackedMatch};
use crate::report::tracked_shape::{BREAK_M, FRAMES_PER_SECOND, Frame, SPRINT_MPS, ZONE_TOPS};

/// The fastest step counted: the report's track break, in m/s.
const MAX_MPS: f64 = BREAK_M * FRAMES_PER_SECOND;
/// The longest native step counted, in milliseconds.
const MAX_GAP_MS: f64 = 100.0;
/// How far the converted figures may sit from the native ones.
pub const TOLERANCE: f64 = 0.05;

/// Distance by running zone (walk, jog, run, high-speed, sprint) and the player time it
/// took.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pace {
    pub zone_m: [f64; 5],
    pub seconds: f64,
}

impl Pace {
    /// A step of `metres` in `seconds`; one faster than the track break adds nothing.
    fn step(&mut self, metres: f64, seconds: f64) {
        let speed = metres / seconds;
        if seconds <= 0.0 || speed > MAX_MPS {
            return;
        }
        let z = if speed >= SPRINT_MPS {
            ZONE_TOPS.len()
        } else {
            ZONE_TOPS.iter().take_while(|&&t| speed >= t).count()
        };
        self.zone_m[z] += metres;
        self.seconds += seconds;
    }

    /// Fast running, 5.5 to 7 m/s, in metres.
    pub fn fast_m(&self) -> f64 {
        self.zone_m[3]
    }

    /// Sprinting, 7 m/s and over, in metres.
    pub fn sprint_m(&self) -> f64 {
        self.zone_m[4]
    }

    /// `metres` per player-minute of the steps counted.
    pub fn per_minute(&self, metres: f64) -> f64 {
        if self.seconds > 0.0 {
            metres * 60.0 / self.seconds
        } else {
            0.0
        }
    }

    pub fn add(&mut self, o: &Pace) {
        for (a, b) in self.zone_m.iter_mut().zip(o.zone_m) {
            *a += b;
        }
        self.seconds += o.seconds;
    }
}

/// The pace of `m` on its native frames, `times` the frames' times in milliseconds per
/// half: from every row, and from the rows that `estimated` (per half, per person, per
/// frame) does not mark as estimated.
pub fn native(m: &TrackedMatch, times: &[Vec<f64>], estimated: &[Vec<Vec<bool>>]) -> [Pace; 2] {
    let mut all = Pace::default();
    let mut seen = Pace::default();
    for (h, half) in m.halves.iter().enumerate() {
        for (p, track) in half.positions.iter().enumerate() {
            if m.persons[p].group == Some(Group::Keeper) {
                continue;
            }
            let est = estimated.get(h).and_then(|e| e.get(p));
            for k in 1..track.len() {
                let (Some(a), Some(b)) = (track[k - 1], track[k]) else {
                    continue;
                };
                let dt = times[h][k] - times[h][k - 1];
                if dt > MAX_GAP_MS {
                    continue;
                }
                let d = f64::from(b[0] - a[0]).hypot(f64::from(b[1] - a[1]));
                all.step(d, dt / 1000.0);
                let hidden = est.is_some_and(|e| e[k - 1] || e[k]);
                if !hidden {
                    seen.step(d, dt / 1000.0);
                }
            }
        }
    }
    [all, seen]
}

/// The pace on the report's frames of a converted match: every outfield player held by
/// two frames one report frame apart.
pub fn converted(frames: &[Frame]) -> Pace {
    let mut out = Pace::default();
    let dt = 1.0 / FRAMES_PER_SECOND;
    for w in frames.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if a.half != b.half || b.tick != a.tick + 2 {
            continue;
        }
        for i in 0..a.on.len() {
            let (Some(x), Some(y)) = (a.on[i], b.on[i]) else {
                continue;
            };
            if x != y || x.group == Group::Keeper {
                continue;
            }
            let (p, q) = (a.players[i], b.players[i]);
            out.step(f64::from(q[0] - p[0]).hypot(f64::from(q[1] - p[1])), dt);
        }
    }
    out
}

/// One match's pace: native from every row, native from rows seen on camera, converted.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchPace {
    pub match_id: String,
    pub native: Pace,
    pub visible: Pace,
    pub converted: Pace,
}

/// The pace of PFF match `id` of `data`.
pub fn pff_match(data: &Path, id: &str) -> anyhow::Result<MatchPace> {
    let n = super::pff::read_native(data, id)?;
    let [native, visible] = native(&n.m, &n.times, &n.estimated);
    let r = super::resample::resample(&n.m, &n.times, n.fps)?;
    let c = super::convert::to_ticks(&r)?;
    let frames = super::measure::frames_of(&c.records, &c.side)?;
    Ok(MatchPace {
        match_id: n.m.match_id.clone(),
        native,
        visible,
        converted: converted(&frames),
    })
}

/// The pace of a DFL match: its 25-a-second frames, which carry no estimated rows.
pub fn dfl_match(files: &super::dfl::Files) -> anyhow::Result<MatchPace> {
    let m = super::dfl::read(files)?;
    let times: Vec<Vec<f64>> = m
        .halves
        .iter()
        .map(|h| h.frames.iter().map(|f| f.t_ms as f64).collect())
        .collect();
    let [native, visible] = native(&m, &times, &[]);
    let c = super::convert::to_ticks(&m)?;
    let frames = super::measure::frames_of(&c.records, &c.side)?;
    Ok(MatchPace {
        match_id: m.match_id.clone(),
        native,
        visible,
        converted: converted(&frames),
    })
}

/// `f` of each item, `jobs` at a time, in the items' order.
pub fn each<T: Sync>(
    items: &[T],
    jobs: usize,
    f: impl Fn(&T) -> anyhow::Result<MatchPace> + Sync,
) -> Vec<anyhow::Result<MatchPace>> {
    let out: std::sync::Mutex<Vec<Option<anyhow::Result<MatchPace>>>> =
        std::sync::Mutex::new((0..items.len()).map(|_| None).collect());
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..jobs.clamp(1, items.len().max(1)) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(item) = items.get(i) else {
                        break;
                    };
                    let done = f(item);
                    if let Ok(mut o) = out.lock() {
                        o[i] = Some(done);
                    }
                }
            });
        }
    });
    out.into_inner()
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.unwrap_or_else(|| Err(anyhow::anyhow!("a pace thread panicked"))))
        .collect()
}

/// The converted figure's difference from the native one, as a share of it.
pub fn difference(native: f64, converted: f64) -> f64 {
    if native > 0.0 {
        (converted - native) / native
    } else {
        0.0
    }
}

/// The printed lines of `matches`: one per match, the totals and the two verdicts.
pub fn lines(matches: &[MatchPace]) -> Vec<String> {
    let line = |name: &str, p: &MatchPace| {
        let (n, v, c) = (&p.native, &p.visible, &p.converted);
        format!(
            "{name}: fast running native {:.0} m ({:.2} m a player-minute), converted {:.0} m \
             ({:.2}), {:+.1}%; sprint native {:.0} m ({:.2}), converted {:.0} m ({:.2}), \
             {:+.1}%; rows seen on camera only: fast {:.2}, sprint {:.2} m a player-minute",
            n.fast_m(),
            n.per_minute(n.fast_m()),
            c.fast_m(),
            c.per_minute(c.fast_m()),
            100.0 * difference(n.fast_m(), c.fast_m()),
            n.sprint_m(),
            n.per_minute(n.sprint_m()),
            c.sprint_m(),
            c.per_minute(c.sprint_m()),
            100.0 * difference(n.sprint_m(), c.sprint_m()),
            v.per_minute(v.fast_m()),
            v.per_minute(v.sprint_m()),
        )
    };
    let mut out: Vec<String> = matches.iter().map(|p| line(&p.match_id, p)).collect();
    let mut total = MatchPace {
        match_id: String::new(),
        native: Pace::default(),
        visible: Pace::default(),
        converted: Pace::default(),
    };
    for p in matches {
        total.native.add(&p.native);
        total.visible.add(&p.visible);
        total.converted.add(&p.converted);
    }
    out.push(line(
        &format!("totals of {} matches", matches.len()),
        &total,
    ));
    let within = |n: f64, c: f64| {
        if difference(n, c).abs() <= TOLERANCE {
            "yes"
        } else {
            "no"
        }
    };
    out.push(format!(
        "fast running converted within 5 percent of native: {}",
        within(total.native.fast_m(), total.converted.fast_m())
    ));
    out.push(format!(
        "sprint converted within 5 percent of native: {}",
        within(total.native.sprint_m(), total.converted.sprint_m())
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::super::{Frame as NativeFrame, Half, Notes, Person};
    use super::*;
    use crate::report::tracked_shape::Holder;

    /// A match of one outfield player running along x at `speed` m/s for `secs` seconds,
    /// at `fps` native frames a second.
    fn running(speed: f64, secs: f64, fps: f64) -> (TrackedMatch, Vec<Vec<f64>>) {
        let n = (secs * fps).round() as usize + 1;
        let times: Vec<f64> = (0..n).map(|k| k as f64 * 1000.0 / fps).collect();
        let frames = times
            .iter()
            .enumerate()
            .map(|(k, t)| NativeFrame {
                n: k as u32,
                t_ms: t.round() as i64,
                ball: None,
                live: None,
                possession: None,
            })
            .collect();
        let positions = vec![
            times
                .iter()
                .map(|t| Some([(speed * t / 1000.0) as f32, 0.0]))
                .collect(),
        ];
        let m = TrackedMatch {
            source: "test".into(),
            match_id: "M".into(),
            pitch: [105.0, 68.0],
            teams: ["A".into(), "B".into()],
            persons: vec![Person {
                id: "1".into(),
                team: 0,
                starting: true,
                position: None,
                group: Some(Group::Cm),
            }],
            halves: vec![Half {
                section: "firstHalf".into(),
                frames,
                positions,
            }],
            events: Vec::new(),
            ticks_per_frame: 0,
            resampled_from: None,
            notes: Notes::default(),
        };
        (m, vec![times])
    }

    /// Report frames 25 a second of one outfield player running along x at `speed`.
    fn report_frames(speed: f64, secs: f64) -> Vec<Frame> {
        let n = (secs * FRAMES_PER_SECOND).round() as u32 + 1;
        (0..n)
            .map(|k| {
                let mut players = [[0.0f32; 2]; 22];
                players[3] = [(speed * f64::from(k) / FRAMES_PER_SECOND) as f32, 0.0];
                let mut on = [None; 22];
                on[3] = Some(Holder {
                    person: 7,
                    group: Group::Cm,
                });
                Frame {
                    tick: 2 * k,
                    half: 0,
                    ball: None,
                    players,
                    on,
                    live: Some(true),
                    possession: None,
                    restarts: Vec::new(),
                    attack_x: [1.0, -1.0],
                }
            })
            .collect()
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() <= 0.01 * b
    }

    /// 6 m/s for 10 s is 60 m of fast running on native frames at 29.97 a second and on
    /// report frames; 8 m/s is sprinting.
    #[test]
    fn a_steady_run_puts_its_distance_in_its_zone_on_both_frame_sets() {
        let (m, t) = running(6.0, 10.0, 29.97);
        let [all, seen] = native(&m, &t, &[]);
        assert!(near(all.fast_m(), 60.0), "{all:?}");
        assert_eq!(all.sprint_m(), 0.0);
        assert_eq!(all, seen);
        assert!(near(all.per_minute(all.fast_m()), 360.0));
        let c = converted(&report_frames(6.0, 10.0));
        assert!(near(c.fast_m(), 60.0), "{c:?}");
        let (m, t) = running(8.0, 10.0, 29.97);
        let [all, _] = native(&m, &t, &[]);
        assert!(near(all.sprint_m(), 80.0) && all.fast_m() == 0.0, "{all:?}");
        let c = converted(&report_frames(8.0, 10.0));
        assert!(near(c.sprint_m(), 80.0) && c.fast_m() == 0.0, "{c:?}");
    }

    /// A native gap over 100 ms adds no distance; a step faster than 25 m/s adds none.
    #[test]
    fn a_gap_or_a_jump_adds_no_distance() {
        let (mut m, mut t) = running(6.0, 1.0, 29.97);
        let before = native(&m, &t, &[])[0].fast_m();
        // A 150 ms gap at the end: the player is 0.9 m on.
        let last = *t[0].last().unwrap();
        t[0].push(last + 150.0);
        let x = m.halves[0].positions[0].last().unwrap().unwrap()[0];
        m.halves[0].positions[0].push(Some([x + 0.9, 0.0]));
        assert_eq!(native(&m, &t, &[])[0].fast_m(), before);
        // A 30 m/s step: 1 m in 33 ms.
        t[0].push(last + 183.4);
        m.halves[0].positions[0].push(Some([x + 1.9, 0.0]));
        let after = native(&m, &t, &[])[0];
        assert_eq!(after.zone_m.iter().sum::<f64>(), before);
        let mut frames = report_frames(6.0, 1.0);
        let base = converted(&frames).fast_m();
        let mut jump = frames.last().unwrap().clone();
        jump.tick += 2;
        jump.players[3][0] += 1.2;
        frames.push(jump);
        assert_eq!(converted(&frames).zone_m.iter().sum::<f64>(), base);
    }

    /// A step to or from a row the provider estimated leaves the figure of rows seen on
    /// camera, not the figure of every row.
    #[test]
    fn estimated_rows_leave_the_figure_seen_on_camera() {
        let (m, t) = running(6.0, 10.0, 29.97);
        let frames = t[0].len();
        let est = vec![vec![(0..frames).map(|k| k < frames / 2).collect()]];
        let [all, seen] = native(&m, &t, &est);
        assert!(near(all.fast_m(), 60.0));
        assert!(seen.fast_m() < 0.55 * all.fast_m() && seen.fast_m() > 0.45 * all.fast_m());
    }

    /// The verdict reads yes within 5 percent of native, no beyond.
    #[test]
    fn the_verdict_reads_the_totals() {
        let mut a = MatchPace {
            match_id: "1".into(),
            native: Pace::default(),
            visible: Pace::default(),
            converted: Pace::default(),
        };
        a.native.zone_m = [0.0, 0.0, 0.0, 100.0, 50.0];
        a.native.seconds = 600.0;
        a.converted = a.native;
        a.converted.zone_m[3] = 96.0;
        a.converted.zone_m[4] = 40.0;
        let l = lines(&[a]);
        assert_eq!(l.len(), 4);
        assert!(l[0].starts_with("1: fast running native 100 m"), "{}", l[0]);
        assert!(
            l[0].contains("-4.0%") && l[0].contains("-20.0%"),
            "{}",
            l[0]
        );
        assert!(l[1].starts_with("totals of 1 matches"));
        assert!(l[2].ends_with("yes") && l[3].ends_with("no"), "{l:#?}");
    }
}
