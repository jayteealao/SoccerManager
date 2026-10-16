//! The substitution report of a calibration run: how many substitutions each team makes in a
//! match, the share of team matches by count, the quartiles of the substitution minute, the
//! reasons the AI manager gave, and the share of substitutes who played out of their listed
//! position; beside them the real values of the era whose substitution limit the run's rule
//! pack has (3: the Wyscout 2017/18 leagues; 5: the StatsBomb tournaments), from the
//! signature table's `substitutions` part.
//!
//! A row keeps each substitution packed in one number ([`pack`]): its minute in tenths, the
//! reason, and the out-of-position bit. The minute counts like the real data's: the first
//! half from 0 with its added time, the second half from 45, and a substitution made at the
//! half-time break at 45.

use std::collections::BTreeMap;

use engine::sim::{EngineEvent, EngineEventKind, EventDetail};
use engine::tactics::change::ChangeKind;
use engine::team::PLAYERS_PER_TEAM;
use engine::{Simulation, TICKS_PER_SECOND};
use serde::Serialize;

use crate::calibrate::fixtures::FixtureKey;
use crate::calibrate::rows::{Row, SUB_SLOTS};

/// The reasons a substitution can carry, by number: 0 for a substitution no AI decision
/// names (a person's), then the AI manager's substitution codes.
pub const REASONS: [&str; 7] = [
    "other",
    "sub-injury",
    "sub-fatigue",
    "sub-keeper",
    "sub-booked",
    "sub-form",
    "sub-knock",
];

/// One substitution packed: bit 0 to 15 the minute in tenths plus 1 (so 0 is no
/// substitution), bit 16 to 19 the reason's number in [`REASONS`], bit 20 set when the
/// substitute's listed position is not the slot's.
pub fn pack(minute: f64, reason: usize, out_of_position: bool) -> u32 {
    let tenths = (minute * 10.0).round().clamp(0.0, 65_534.0) as u32;
    (tenths + 1) | ((reason.min(15) as u32) << 16) | (u32::from(out_of_position) << 20)
}

/// A packed substitution: its minute, its reason's number and its out-of-position bit;
/// `None` for 0.
pub fn unpack(v: u32) -> Option<(f64, usize, bool)> {
    let tenths = v & 0xffff;
    (tenths > 0).then(|| {
        (
            f64::from(tenths - 1) / 10.0,
            ((v >> 16) & 0xf) as usize,
            v & (1 << 20) != 0,
        )
    })
}

/// Each side's substitutions of a finished match, packed, in the order they were made: the
/// minute from the half's kick-off tick, the reason of the AI decision that asked for it (the
/// k-th substitution decision of a side asked for its k-th substitution verdict, since the
/// queue judges its substitutions in the order they joined it), and whether the substitute's
/// listed position differs from the position of his slot in the side's formation at full
/// time.
pub fn of_match(events: &[EngineEvent], sim: &Simulation) -> [[u32; SUB_SLOTS]; 2] {
    let teams = sim.teams();
    let schema = &sim.config().tactics;
    let tpm = f64::from(60 * TICKS_PER_SECOND);
    // Per side: the reason of each substitution decision in order, and of each verdict.
    let mut asked: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
    let mut applied: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
    let mut verdicts = [0usize; 2];
    for e in events {
        let Some(team) = e.team else { continue };
        match (e.kind, e.detail) {
            (EngineEventKind::AiDecision, Some(EventDetail::Ai { code })) => {
                if let Some(r) = REASONS.iter().position(|c| *c == code.code()) {
                    asked[team].push(r);
                }
            }
            (
                EngineEventKind::ChangeApplied | EngineEventKind::ChangeRejected,
                Some(EventDetail::Change {
                    kind: ChangeKind::Substitution,
                    ..
                }),
            ) => {
                let reason = asked[team].get(verdicts[team]).copied().unwrap_or(0);
                verdicts[team] += 1;
                if e.kind == EngineEventKind::ChangeApplied {
                    applied[team].push(reason);
                }
            }
            _ => {}
        }
    }
    let mut out = [[0u32; SUB_SLOTS]; 2];
    let mut made = [0usize; 2];
    let (mut half, mut start, mut break_on) = (0u32, None::<u32>, false);
    for e in events {
        match e.kind {
            EngineEventKind::KickOff if start.is_none() || break_on => {
                if break_on {
                    half += 1;
                }
                start = Some(e.tick);
                break_on = false;
            }
            EngineEventKind::HalfTime => break_on = true,
            EngineEventKind::Substitution => {
                let (Some(team), Some(EventDetail::Substitution { on, .. })) = (e.team, e.detail)
                else {
                    continue;
                };
                let n = made[team];
                made[team] += 1;
                if n >= SUB_SLOTS {
                    continue;
                }
                let minute = if break_on {
                    45.0 * f64::from(half + 1)
                } else {
                    45.0 * f64::from(half)
                        + f64::from(e.tick.saturating_sub(start.unwrap_or(0))) / tpm
                };
                let side = &teams[team];
                let slot = e.player.map_or(0, |p| p % PLAYERS_PER_TEAM);
                let position =
                    schema.formations[usize::from(side.tactics.formation)].slots[slot].position;
                let away = side.squad.get(on).is_some_and(|p| p.position != position);
                let reason = applied[team].get(n).copied().unwrap_or(0);
                out[team][n] = pack(minute, reason, away);
            }
            _ => {}
        }
    }
    out
}

/// The real values of one era, as the signature table holds them.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct RealEra {
    pub source: String,
    pub competitions: Vec<String>,
    pub matches: u64,
    pub per_team: f64,
    pub per_team_range: [f64; 2],
    pub minute_quartiles: [f64; 3],
    pub minute_quartile_ranges: [[f64; 2]; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injury_share: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injury_share_range: Option<[f64; 2]>,
}

/// The substitution report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SubstitutionReport {
    /// The rule pack's substitution limit (regulation time).
    pub limit: u8,
    pub matches: usize,
    pub substitutions: usize,
    /// Substitutions a team makes in a match.
    pub per_team: f64,
    /// The share of team matches by the number of substitutions made, from 0.
    pub count_shares: Vec<f64>,
    /// The minute at 25, 50 and 75 percent of the substitutions, and their mean.
    pub minute_quartiles: [f64; 3],
    pub mean_minute: f64,
    /// The share of substitutions by reason.
    pub reasons: BTreeMap<String, f64>,
    /// The share made for an injury or a knock, beside the real injury share.
    pub injury_knock_share: f64,
    /// Substitutes who played out of their listed position, a match (both sides) and as a
    /// share.
    pub out_of_position_per_match: f64,
    pub out_of_position_share: f64,
    /// The real era of the same limit, when the signature table has it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real: Option<RealEra>,
    /// Inside the real era's range: the rate a team, and each quartile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_team_inside: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quartiles_inside: Option<[bool; 3]>,
}

/// The value at share `p` of the sorted values `v`, linear between the two nearest ranks, as
/// the signature table takes it.
fn quantile(v: &[f64], p: f64) -> f64 {
    let at = p * (v.len() - 1) as f64;
    let i = at.floor() as usize;
    let j = (i + 1).min(v.len() - 1);
    v[i] + (v[j] - v[i]) * (at - i as f64)
}

fn round4(x: f64) -> f64 {
    (x * 1e4).round() / 1e4
}

/// The era of `limit` in the signature table `table`, when it holds one.
pub fn real_era(table: &serde_json::Value, limit: u8) -> Option<RealEra> {
    let era = table["substitutions"]["eras"]
        .as_array()?
        .iter()
        .find(|e| e["limit"].as_u64() == Some(u64::from(limit)))?;
    serde_json::from_value(serde_json::json!({
        "source": era["source"],
        "competitions": era["competitions"],
        "matches": era["matches"],
        "per_team": era["per_team"],
        "per_team_range": era["per_team_range"],
        "minute_quartiles": era["minute_quartiles"],
        "minute_quartile_ranges": era["minute_quartile_ranges"],
        "injury_share": era.get("injury_share"),
        "injury_share_range": era.get("injury_share_range"),
    }))
    .ok()
}

/// The report of `rows` played under a rule pack of substitution limit `limit`, against the
/// era of that limit in the signature table `table`. `None` when no row holds a match.
pub fn report(
    rows: &BTreeMap<FixtureKey, Row>,
    limit: u8,
    table: Option<&serde_json::Value>,
) -> Option<SubstitutionReport> {
    let mut minutes = Vec::new();
    let mut counts: Vec<usize> = Vec::new();
    let mut reasons = [0usize; REASONS.len()];
    let mut away = 0usize;
    let mut matches = 0usize;
    for row in rows.values() {
        if row.outcome != crate::calibrate::rows::Outcome::Success {
            continue;
        }
        matches += 1;
        for side in &row.subs {
            let made: Vec<(f64, usize, bool)> = side.iter().filter_map(|&v| unpack(v)).collect();
            if counts.len() <= made.len() {
                counts.resize(made.len() + 1, 0);
            }
            counts[made.len()] += 1;
            for (minute, reason, out) in made {
                minutes.push(minute);
                reasons[reason.min(REASONS.len() - 1)] += 1;
                away += usize::from(out);
            }
        }
    }
    if matches == 0 {
        return None;
    }
    let n = minutes.len();
    minutes.sort_by(f64::total_cmp);
    let sides = (2 * matches) as f64;
    let share = |k: usize| {
        if n == 0 {
            0.0
        } else {
            round4(k as f64 / n as f64)
        }
    };
    let quartiles = if n == 0 {
        [0.0; 3]
    } else {
        [0.25, 0.5, 0.75].map(|p| round4(quantile(&minutes, p)))
    };
    let per_team = round4(n as f64 / sides);
    let real = table.and_then(|t| real_era(t, limit));
    let inside = |v: f64, [lo, hi]: [f64; 2]| v >= lo && v <= hi;
    Some(SubstitutionReport {
        limit,
        matches,
        substitutions: n,
        per_team,
        count_shares: counts.iter().map(|&c| round4(c as f64 / sides)).collect(),
        minute_quartiles: quartiles,
        mean_minute: if n == 0 {
            0.0
        } else {
            round4(minutes.iter().sum::<f64>() / n as f64)
        },
        reasons: REASONS
            .iter()
            .zip(reasons)
            .filter(|(_, k)| *k > 0)
            .map(|(name, k)| (name.to_string(), share(k)))
            .collect(),
        injury_knock_share: share(reasons[1] + reasons[6]),
        out_of_position_per_match: round4(away as f64 / matches as f64),
        out_of_position_share: share(away),
        per_team_inside: real.as_ref().map(|r| inside(per_team, r.per_team_range)),
        quartiles_inside: real
            .as_ref()
            .map(|r| [0, 1, 2].map(|q| inside(quartiles[q], r.minute_quartile_ranges[q]))),
        real,
    })
}

/// The report as lines for the terminal.
pub fn render(r: &SubstitutionReport) -> String {
    let mut out = format!(
        "substitutions (limit {}): {} a team over {} matches; minute quartiles {:.1} / {:.1} \
         / {:.1} (mean {:.1}); injury or knock {:.3}; out of position {:.2} a match\n",
        r.limit,
        r.per_team,
        r.matches,
        r.minute_quartiles[0],
        r.minute_quartiles[1],
        r.minute_quartiles[2],
        r.mean_minute,
        r.injury_knock_share,
        r.out_of_position_per_match
    );
    if let Some(real) = &r.real {
        out.push_str(&format!(
            "  real ({}, {} matches): {} a team (range {} to {}); quartiles {:.1} / {:.1} / \
             {:.1}; inside: rate {}, quartiles {:?}\n",
            real.source,
            real.matches,
            real.per_team,
            real.per_team_range[0],
            real.per_team_range[1],
            real.minute_quartiles[0],
            real.minute_quartiles[1],
            real.minute_quartiles[2],
            r.per_team_inside.unwrap_or(false),
            r.quartiles_inside.unwrap_or([false; 3])
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_substitution_packs_and_unpacks() {
        assert_eq!(unpack(0), None);
        let v = pack(72.35, 4, true);
        assert_eq!(unpack(v), Some((72.4, 4, true)));
        assert_eq!(unpack(pack(0.0, 0, false)), Some((0.0, 0, false)));
    }

    /// Two matches: the home side makes three substitutions at 60, 70 and 80 (one for an
    /// injury, one out of position) and the away side none; then both make one at 45.
    #[test]
    fn the_report_counts_team_matches_minutes_and_reasons() {
        let mut rows = BTreeMap::new();
        let mut a = Row {
            key: FixtureKey::from_u64(1),
            ..Row::default()
        };
        a.subs[0][..3].copy_from_slice(&[
            pack(60.0, 2, false),
            pack(70.0, 1, true),
            pack(80.0, 2, false),
        ]);
        let mut b = Row {
            key: FixtureKey::from_u64(2),
            ..Row::default()
        };
        b.subs[0][0] = pack(45.0, 2, false);
        b.subs[1][0] = pack(45.0, 2, false);
        rows.insert(a.key, a);
        rows.insert(b.key, b);
        let table = serde_json::json!({"substitutions": {"eras": [{
            "limit": 3, "source": "wyscout_2017_18", "competitions": ["England"], "matches": 10,
            "per_team": 1.25, "per_team_range": [1.2, 1.3], "minute_quartiles": [45.0, 52.5, 72.5],
            "minute_quartile_ranges": [[40.0, 50.0], [61.0, 70.0], [70.0, 75.0]]
        }]}});
        let r = report(&rows, 3, Some(&table)).unwrap();
        assert_eq!((r.matches, r.substitutions), (2, 5));
        assert_eq!(r.per_team, 1.25);
        assert_eq!(r.count_shares, vec![0.25, 0.5, 0.0, 0.25]);
        assert_eq!(r.minute_quartiles, [45.0, 60.0, 70.0]);
        assert_eq!(r.reasons["sub-fatigue"], 0.8);
        assert_eq!(r.injury_knock_share, 0.2);
        assert_eq!(r.out_of_position_per_match, 0.5);
        assert_eq!(r.per_team_inside, Some(true));
        assert_eq!(r.quartiles_inside, Some([true, false, true]));
        assert!(report(&rows, 5, Some(&table)).unwrap().real.is_none());
    }
}
