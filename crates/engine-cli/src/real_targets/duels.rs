//! The duel targets from the Wyscout open events: a duel is a contest between two players,
//! and Wyscout writes one row for each of them, so two rows of opposite teams with matching
//! kinds (ground attacking with ground defending, air with air, loose ball with loose ball)
//! within one second pair into one contest. A row left without a partner counts as one
//! contest, and the unpaired rows are counted.
//!
//! Only England, France, Italy and Spain 2017/18 are read. Germany 2017/18, the World Cup
//! 2018 and Euro 2016 are held out: no builder opens them.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

use super::football_data::mean_sd;
use super::{EngineCount, Label, Layer, Target};

/// The leagues the duel targets read.
pub const LEAGUES: [&str; 4] = ["England", "France", "Italy", "Spain"];
/// The held-out sets no builder opens.
pub const HELD_OUT: [&str; 3] = ["Germany", "World_Cup", "European_Championship"];
/// The longest gap between the two rows of one contest, in seconds.
pub const PAIR_WINDOW_S: f64 = 1.0;

/// The provider's name.
pub const PROVIDER: &str = "Wyscout";
/// The source label.
pub const SOURCE: &str = "Wyscout open events 2017/18 (Pappalardo et al. 2019)";

/// The fields of a Wyscout event the duel count reads.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub match_id: u64,
    pub match_period: String,
    pub event_sec: f64,
    pub event_id: u32,
    #[serde(default, deserialize_with = "sub_event")]
    pub sub_event_id: u32,
    pub team_id: u64,
}

/// Wyscout writes an empty string for a missing sub-event.
fn sub_event<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(v.as_u64().map_or(0, |n| u32::try_from(n).unwrap_or(0)))
}

/// The duel event and its sub-events.
const DUEL: u32 = 1;
const AIR: u32 = 10;
const GROUND_ATTACKING: u32 = 11;
const GROUND_DEFENDING: u32 = 12;
const LOOSE_BALL: u32 = 13;

fn compatible(a: u32, b: u32) -> bool {
    matches!(
        (a, b),
        (AIR, AIR)
            | (LOOSE_BALL, LOOSE_BALL)
            | (GROUND_ATTACKING, GROUND_DEFENDING)
            | (GROUND_DEFENDING, GROUND_ATTACKING)
    )
}

/// The contests of one match: `(contests, unpaired rows)`.
pub fn contests(events: &[&Event]) -> (u32, u32) {
    let mut by_period: BTreeMap<&str, Vec<&Event>> = BTreeMap::new();
    for e in events.iter().filter(|e| e.event_id == DUEL) {
        by_period.entry(&e.match_period).or_default().push(e);
    }
    let (mut contests, mut unpaired) = (0, 0);
    for list in by_period.values_mut() {
        list.sort_by(|a, b| a.event_sec.total_cmp(&b.event_sec));
        let mut used = vec![false; list.len()];
        for i in 0..list.len() {
            if used[i] {
                continue;
            }
            used[i] = true;
            contests += 1;
            let partner = (i + 1..list.len())
                .take_while(|&j| list[j].event_sec - list[i].event_sec <= PAIR_WINDOW_S)
                .find(|&j| {
                    !used[j]
                        && list[j].team_id != list[i].team_id
                        && compatible(list[i].sub_event_id, list[j].sub_event_id)
                });
            match partner {
                Some(j) => used[j] = true,
                None => unpaired += 1,
            }
        }
    }
    (contests, unpaired)
}

/// The contests per match of one league's events: one value per match, and the unpaired
/// rows.
pub fn per_match(events: &[Event]) -> (Vec<f64>, u32) {
    let mut by_match: BTreeMap<u64, Vec<&Event>> = BTreeMap::new();
    for e in events {
        by_match.entry(e.match_id).or_default().push(e);
    }
    let mut unpaired = 0;
    let values = by_match
        .values()
        .map(|m| {
            let (c, u) = contests(m);
            unpaired += u;
            f64::from(c)
        })
        .collect();
    (values, unpaired)
}

/// Reads `wyscout/events_<league>.json` of `data`, refusing a held-out set; the path read is
/// added to `opened`.
pub fn read(data: &Path, league: &str, opened: &mut Vec<String>) -> anyhow::Result<Vec<Event>> {
    anyhow::ensure!(
        LEAGUES.contains(&league) && !HELD_OUT.contains(&league),
        "the Wyscout set {league} is not one the duel targets read"
    );
    let rel = format!("wyscout/events_{league}.json");
    let bytes = std::fs::read(data.join(&rel)).with_context(|| format!("cannot read {rel}"))?;
    let events = serde_json::from_slice(&bytes).with_context(|| format!("{rel} cannot be read"))?;
    opened.push(rel);
    Ok(events)
}

fn target(values: &[f64], layer: Layer, leagues: Option<Vec<String>>) -> Option<Target> {
    let (mean, sd) = mean_sd(values)?;
    Some(Target {
        value: Some(mean),
        shares: None,
        bins: None,
        width: None,
        se: sd / (values.len() as f64).sqrt(),
        n: values.len() as u64,
        seasons: vec!["1718".into()],
        between_season_sd: None,
        between_league_sd: None,
        layer,
        leagues,
        provider: PROVIDER.into(),
        source_label: SOURCE.into(),
        event_definition: "duel contests: two duel rows (eventId 1) of opposite teams with \
                           matching kinds within 1 second are one contest; an unpaired row \
                           is one contest"
            .into(),
        denominator: "matches".into(),
        unit_of_observation: "match".into(),
        label: Label::Calibration,
        engine_count: EngineCount {
            text: "contests between two players for the ball (tackles and aerial \
                   challenges) per match, each counted once"
                .into(),
            band: None,
            measure: None,
        },
        context: None,
        extrapolated: None,
        formula: None,
        per_match: None,
    })
}

/// The per-league and pooled duel targets, `wy.<league>.duel_contests_per_match` and
/// `wy.pooled.duel_contests_per_match`; returns the unpaired rows per league and the files
/// opened.
pub fn targets(
    data: &Path,
    targets: &mut BTreeMap<String, Target>,
    absent: &mut BTreeMap<String, String>,
) -> anyhow::Result<(BTreeMap<String, u32>, Vec<String>)> {
    let mut opened = Vec::new();
    let mut unpaired = BTreeMap::new();
    let mut pooled = Vec::new();
    for league in LEAGUES {
        let events = read(data, league, &mut opened)?;
        let (values, u) = per_match(&events);
        unpaired.insert(league.to_string(), u);
        let key = format!("wy.{league}.duel_contests_per_match");
        match target(&values, Layer::League, None) {
            Some(t) => {
                targets.insert(key, t);
            }
            None => {
                absent.insert(key, "fewer than two matches".into());
            }
        }
        pooled.extend(values);
    }
    if let Some(t) = target(
        &pooled,
        Layer::Pooled,
        Some(LEAGUES.iter().map(|l| (*l).to_string()).collect()),
    ) {
        targets.insert("wy.pooled.duel_contests_per_match".into(), t);
    }
    Ok((unpaired, opened))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(sec: f64, team: u64, sub: u32) -> Event {
        Event {
            match_id: 1,
            match_period: "1H".into(),
            event_sec: sec,
            event_id: DUEL,
            sub_event_id: sub,
            team_id: team,
        }
    }

    #[test]
    fn paired_rows_count_one_contest_and_a_lone_row_counts_one() {
        let rows = [
            e(1.0, 1, GROUND_ATTACKING),
            e(1.2, 2, GROUND_DEFENDING),
            e(5.0, 2, AIR),
            e(5.1, 1, AIR),
            e(9.0, 1, LOOSE_BALL),
            e(9.4, 2, LOOSE_BALL),
            // Alone: its would-be partner is 1.5 seconds later.
            e(20.0, 1, AIR),
            e(21.5, 2, AIR),
        ];
        let refs: Vec<&Event> = rows.iter().collect();
        assert_eq!(contests(&refs[..6]), (3, 0));
        assert_eq!(contests(&refs), (5, 2));
    }

    #[test]
    fn a_held_out_set_is_refused_and_never_opened() {
        let mut opened = Vec::new();
        for held in HELD_OUT {
            assert!(read(Path::new("."), held, &mut opened).is_err());
        }
        assert!(opened.is_empty());
    }
}
