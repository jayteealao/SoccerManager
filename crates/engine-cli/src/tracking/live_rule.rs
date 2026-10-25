//! The event-data live-time rule and its check against tracked ball status.
//!
//! The rule: in each half, from the half's first event to its last, the time from a stop to
//! the next restart is dead and the rest is live. A second stop before the restart changes
//! nothing; a restart with no stop before it opens nothing. Each source maps its events to a
//! stop, a restart or neither, so the rule is one rule for every source:
//!
//! - DFL: a foul, an offside and a goal stop play; a throw-in, free kick, goal kick, corner,
//!   kick-off after a goal and penalty restart it, and the referee's decision time a restart
//!   carries is a stop at that time (DFL logs the ball going out only through the restart's
//!   decision). A half-opening kick-off and the final whistle bound the half.
//! - Wyscout (the event ids of its public data set): a foul (2), an interruption (5: ball
//!   out, whistle) and an offside (6) stop play; a free kick of any kind (3: free kick,
//!   corner, throw-in, goal kick, penalty) restarts it.
//!
//! `engine-cli tracking live-check` applies the rule to each DFL match's events and compares
//! it with the tracked ball status: the frames whose ball is in play, at 25 a second. The
//! check passes only when every match is within 2.0 minutes; otherwise event-data rates
//! from Wyscout stay per match, never per live minute.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::{EventKind, TrackedMatch};

/// How far the rule may be from the tracked ball status on any match, in minutes.
pub const TOLERANCE_MIN: f64 = 2.0;
/// The check file's schema.
pub const SCHEMA: u32 = 1;
/// The rule in words, as the check file states it.
pub const RULE: &str = "in each half, from its first event to its last (on DFL events, its \
                        kick-off to its final whistle), the time from a stop to the next \
                        restart is dead";

/// What an event does to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Stop,
    Restart,
    Other,
}

/// One event of a half on the rule's clock, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Marked {
    pub t_ms: i64,
    pub mark: Mark,
}

/// The dead spans of a half's events, in time order: each from a stop to the next restart.
/// A stop with no restart after it is dead to the half's last event.
pub fn dead_spans(events: &[Marked]) -> Vec<(i64, i64)> {
    let mut sorted = events.to_vec();
    sorted.sort_by_key(|e| e.t_ms);
    let mut out = Vec::new();
    let mut since: Option<i64> = None;
    for e in &sorted {
        match (e.mark, since) {
            (Mark::Stop, None) => since = Some(e.t_ms),
            (Mark::Restart, Some(s)) => {
                out.push((s, e.t_ms));
                since = None;
            }
            _ => {}
        }
    }
    if let (Some(s), Some(last)) = (since, sorted.last()) {
        out.push((s, last.t_ms));
    }
    out
}

/// The live minutes of the halves by the rule.
pub fn live_minutes(halves: &[Vec<Marked>]) -> f64 {
    let mut live_ms = 0i64;
    for h in halves {
        let (Some(first), Some(last)) = (
            h.iter().map(|e| e.t_ms).min(),
            h.iter().map(|e| e.t_ms).max(),
        ) else {
            continue;
        };
        let dead: i64 = dead_spans(h).iter().map(|(a, b)| b - a).sum();
        live_ms += last - first - dead;
    }
    live_ms as f64 / 60_000.0
}

/// The DFL mapping: each half's marked events, the halves split at each half-opening
/// kick-off, each bounded by its kick-off and its final whistle (an event the provider logs
/// before the kick-off or after the whistle, such as a line-up change, is no play).
pub fn dfl_halves(m: &TrackedMatch) -> Vec<Vec<Marked>> {
    let halves_of = super::event_halves(&m.events);
    let mut bounds: Vec<(Option<i64>, Option<i64>)> = Vec::new();
    for (e, &h) in m.events.iter().zip(&halves_of) {
        if bounds.len() <= h {
            bounds.resize(h + 1, (None, None));
        }
        match e.kind {
            EventKind::KickOff { half_start: true } => bounds[h].0 = bounds[h].0.or(e.time_ms),
            EventKind::FinalWhistle => bounds[h].1 = e.time_ms.or(bounds[h].1),
            _ => {}
        }
    }
    let mut halves: Vec<Vec<Marked>> = Vec::new();
    for (e, h) in m.events.iter().zip(super::event_halves(&m.events)) {
        let Some(t) = e.time_ms else {
            continue;
        };
        if halves.len() <= h {
            halves.resize(h + 1, Vec::new());
        }
        let half = &mut halves[h];
        let restart = matches!(
            e.kind,
            EventKind::ThrowIn
                | EventKind::FreeKick
                | EventKind::GoalKick
                | EventKind::Corner
                | EventKind::Penalty
                | EventKind::KickOff { half_start: false }
        );
        if restart {
            if let Some(d) = e.decision_ms {
                half.push(Marked {
                    t_ms: d,
                    mark: Mark::Stop,
                });
            }
            half.push(Marked {
                t_ms: t,
                mark: Mark::Restart,
            });
            continue;
        }
        let mark = match e.kind {
            EventKind::Foul | EventKind::Offside | EventKind::Shot { goal: true } => Mark::Stop,
            _ => Mark::Other,
        };
        half.push(Marked { t_ms: t, mark });
    }
    for (h, half) in halves.iter_mut().enumerate() {
        let (from, to) = bounds.get(h).copied().unwrap_or((None, None));
        half.retain(|e| from.is_none_or(|f| e.t_ms >= f) && to.is_none_or(|z| e.t_ms <= z));
    }
    halves
}

/// The Wyscout mapping of one event id. The mapping serves the rule's fixture test today:
/// no command reads Wyscout events while the check's verdict keeps Wyscout per match.
#[cfg(test)]
pub fn wyscout_mark(event_id: u32) -> Mark {
    match event_id {
        2 | 5 | 6 => Mark::Stop,
        3 => Mark::Restart,
        _ => Mark::Other,
    }
}

/// The Wyscout mapping: a match's events (`eventId`, `eventSec`, `matchPeriod`) as each
/// half's marked events, in the order of their periods.
#[cfg(test)]
pub fn wyscout_halves(events: &[serde_json::Value]) -> anyhow::Result<Vec<Vec<Marked>>> {
    let mut periods: Vec<(String, Vec<Marked>)> = Vec::new();
    for e in events {
        let id = e["eventId"]
            .as_u64()
            .context("a Wyscout event has no eventId")?;
        let sec = e["eventSec"]
            .as_f64()
            .context("a Wyscout event has no eventSec")?;
        let period = e["matchPeriod"]
            .as_str()
            .context("a Wyscout event has no matchPeriod")?;
        let marked = Marked {
            t_ms: (sec * 1000.0).round() as i64,
            mark: wyscout_mark(u32::try_from(id).unwrap_or(u32::MAX)),
        };
        match periods.iter_mut().find(|(p, _)| p == period) {
            Some((_, h)) => h.push(marked),
            None => periods.push((period.to_string(), vec![marked])),
        }
    }
    Ok(periods.into_iter().map(|(_, h)| h).collect())
}

/// The tracked ball-in-play minutes of a match: its frames with the ball in play, at 25 a
/// second.
pub fn tracked_minutes(m: &TrackedMatch) -> f64 {
    let live: usize = m
        .halves
        .iter()
        .map(|h| h.frames.iter().filter(|f| f.live == Some(true)).count())
        .sum();
    live as f64 / (25.0 * 60.0)
}

/// One match of the check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchCheck {
    pub match_id: String,
    pub rule_minutes: f64,
    pub tracked_minutes: f64,
    /// The rule's minutes less the tracked minutes.
    pub difference: f64,
}

/// The check file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveCheck {
    pub schema: u32,
    pub rule: String,
    pub tolerance_minutes: f64,
    pub matches: Vec<MatchCheck>,
    /// `pass` when every match is within the tolerance, else `fail`.
    pub verdict: String,
    /// How Wyscout event rates are taken: `per live minute` only on a pass, else
    /// `per match`.
    pub wyscout_rates: String,
}

/// The verdict on the matches.
pub fn verdict(matches: Vec<MatchCheck>) -> LiveCheck {
    let pass = !matches.is_empty() && matches.iter().all(|m| m.difference.abs() <= TOLERANCE_MIN);
    LiveCheck {
        schema: SCHEMA,
        rule: RULE.into(),
        tolerance_minutes: TOLERANCE_MIN,
        matches,
        verdict: if pass { "pass" } else { "fail" }.into(),
        wyscout_rates: if pass { "per live minute" } else { "per match" }.into(),
    }
}

/// The check of one match.
pub fn check_match(m: &TrackedMatch) -> MatchCheck {
    let rule = live_minutes(&dfl_halves(m));
    let tracked = tracked_minutes(m);
    MatchCheck {
        match_id: m.match_id.clone(),
        rule_minutes: round3(rule),
        tracked_minutes: round3(tracked),
        difference: round3(rule - tracked),
    }
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// The check of every DFL match of `data`.
pub fn check_dfl(data: &Path) -> anyhow::Result<LiveCheck> {
    let mut out = Vec::new();
    for files in super::dfl::matches(data)? {
        out.push(check_match(&super::dfl::read(&files)?));
    }
    anyhow::ensure!(!out.is_empty(), "no DFL match in {}", data.display());
    Ok(verdict(out))
}

impl LiveCheck {
    pub fn write(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("cannot create {}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(self)? + "\n";
        std::fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))
    }
}

/// The check file's default place: `gate/tracking/live-time-check.json` of the checkout.
pub fn default_out(content_root: &Path) -> PathBuf {
    crate::real_targets::gate_dir(content_root).join("tracking/live-time-check.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: f64, mark: Mark) -> Marked {
        Marked {
            t_ms: (s * 1000.0) as i64,
            mark,
        }
    }

    /// A stop and a restart 30 s apart make 30 s dead; a second stop and an unmatched
    /// restart change nothing.
    #[test]
    fn a_stop_and_a_restart_30_s_apart_make_30_s_dead() {
        let half = vec![
            at(0.0, Mark::Restart),
            at(100.0, Mark::Stop),
            at(110.0, Mark::Stop),
            at(130.0, Mark::Restart),
            at(200.0, Mark::Restart),
            at(600.0, Mark::Other),
        ];
        assert_eq!(dead_spans(&half), [(100_000, 130_000)]);
        assert!((live_minutes(&[half]) - 9.5).abs() < 1e-12);
        // A stop with no restart is dead to the half's last event.
        let open = vec![
            at(0.0, Mark::Other),
            at(50.0, Mark::Stop),
            at(80.0, Mark::Other),
        ];
        assert_eq!(dead_spans(&open), [(50_000, 80_000)]);
    }

    fn check(id: &str, difference: f64) -> MatchCheck {
        MatchCheck {
            match_id: id.into(),
            rule_minutes: 50.0 + difference,
            tracked_minutes: 50.0,
            difference,
        }
    }

    /// One match 2.5 minutes off fails the check, and Wyscout stays per match; every match
    /// within 2.0 passes.
    #[test]
    fn the_check_passes_only_when_every_match_is_within_two_minutes() {
        let fail = verdict(vec![check("a", 1.0), check("b", -2.5), check("c", 0.0)]);
        assert_eq!(fail.verdict, "fail");
        assert_eq!(fail.wyscout_rates, "per match");
        let pass = verdict(vec![check("a", 1.9), check("b", -2.0), check("c", 0.0)]);
        assert_eq!(pass.verdict, "pass");
        assert_eq!(pass.wyscout_rates, "per live minute");
        assert_eq!(verdict(Vec::new()).verdict, "fail");
    }

    /// The Wyscout mapping turns its events into the same spans as the same play mapped by
    /// hand: an interruption and a foul stop, a throw-in and a free kick restart.
    #[test]
    fn the_wyscout_mapping_gives_the_same_spans() {
        let events: Vec<serde_json::Value> = [
            (8, 2.0, "1H"),
            (5, 100.0, "1H"),
            (3, 130.0, "1H"),
            (8, 140.0, "1H"),
            (2, 300.0, "1H"),
            (3, 320.5, "1H"),
            (10, 600.0, "1H"),
            (8, 1.0, "2H"),
            (6, 50.0, "2H"),
            (3, 60.0, "2H"),
            (8, 90.0, "2H"),
        ]
        .into_iter()
        .map(|(id, s, p)| serde_json::json!({ "eventId": id, "eventSec": s, "matchPeriod": p }))
        .collect();
        let halves = wyscout_halves(&events).unwrap();
        assert_eq!(halves.len(), 2);
        assert_eq!(
            dead_spans(&halves[0]),
            [(100_000, 130_000), (300_000, 320_500)]
        );
        let by_hand = vec![
            at(2.0, Mark::Other),
            at(100.0, Mark::Stop),
            at(130.0, Mark::Restart),
            at(300.0, Mark::Stop),
            at(320.5, Mark::Restart),
            at(600.0, Mark::Other),
        ];
        assert_eq!(dead_spans(&halves[0]), dead_spans(&by_hand));
        assert_eq!(dead_spans(&halves[1]), [(50_000, 60_000)]);
        let live = live_minutes(&halves);
        assert!((live - ((598.0 - 50.5) + (89.0 - 10.0)) / 60.0).abs() < 1e-9);
    }

    /// The DFL mapping: a restart's decision time stops play, and the half-opening kick-off
    /// starts a half.
    #[test]
    fn the_dfl_mapping_reads_the_decision_time_as_the_stop() {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracking/dfl");
        let mut m =
            super::super::dfl::read(&super::super::dfl::find(&data, "DFL-MAT-FIX001").unwrap())
                .unwrap();
        let halves = dfl_halves(&m);
        assert_eq!(halves.len(), 2);
        let base = m.events[0].time_ms.unwrap();
        let throw = m
            .events
            .iter_mut()
            .find(|e| e.kind == EventKind::ThrowIn)
            .unwrap();
        throw.decision_ms = throw.time_ms.map(|t| t - 200);
        let t = throw.time_ms.unwrap();
        let halves = dfl_halves(&m);
        assert!(
            dead_spans(&halves[0]).contains(&(t - 200, t)),
            "{:?} {base}",
            dead_spans(&halves[0])
        );
        let c = check_match(&m);
        assert!((c.difference - (c.rule_minutes - c.tracked_minutes)).abs() < 2e-3);
    }
}
