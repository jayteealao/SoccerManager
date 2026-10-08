//! The equality check: the fast model against the full engine on the held-out check batch.
//!
//! For each check-batch match the fast model plays `draws` seeded matches from the same
//! kick-off. Compared, full engine against fast model, in two families:
//!
//! - the score: per pairing the home win, draw and away win shares and the home and away
//!   goals per match (9 x 5 = 45 figures); over the equal-strength pairings the season
//!   figures goals per match, the goalless share and the share of matches with ten or more
//!   goals; and over 1.15 v 1.00 in both orders the stronger side's win rate (4 figures);
//! - the events: per pairing each side's fouls, offsides, yellow cards, corners, throw-ins,
//!   goal kicks, free kicks, penalties, injuries and substitutions per match, the share of
//!   matches with a sending-off, and the seconds each half added (9 x 23 = 207 figures);
//!   over the equal-strength pairings the five season band figures sending-off share, yellow
//!   cards and corners per team, and throw-ins and goal kicks per match (5 figures).
//!
//! A figure passes when the two differ by at most max(floor, z x the combined standard
//! error), each standard error from that side's own distribution. Each family keeps a 1
//! percent chance that a fast model equal to the full engine fails any of its figures:
//! z = 3.7 over the 49 score figures, z = 4.07 over the 212 event figures (Bonferroni). A
//! figure with no events on either side passes and says so.

use engine::modules::fast_model::{FastFit, FastModel};
use serde::{Deserialize, Serialize};

use super::batch::{Counts, EventTally, PAIRINGS, Row, pairing_name, tally};
use crate::calibrate::fixtures::splitmix64;

/// The z of every score tolerance.
pub const Z: f64 = 3.7;
/// The z of every event tolerance.
pub const EVENT_Z: f64 = 4.07;
/// The smallest tolerance of a share.
pub const SHARE_FLOOR: f64 = 0.01;
/// The smallest tolerance of a mean.
pub const MEAN_FLOOR: f64 = 0.03;
/// The score figures the check compares.
pub const SCORE_FIGURES: usize = PAIRINGS.len() * 5 + 4;
/// The event figures the check compares.
pub const EVENT_FIGURES: usize = PAIRINGS.len() * (COUNTED.len() * 2 + 3) + SEASON_EVENTS;
/// Every figure the check compares.
pub const FIGURES: usize = SCORE_FIGURES + EVENT_FIGURES;
/// The season band figures of the events.
const SEASON_EVENTS: usize = 5;

/// A count of one side's events in a match.
pub type Count = fn(&Counts) -> u32;

/// The counts compared per side, by name.
pub const COUNTED: [(&str, Count); 10] = [
    ("fouls", |c| c.fouls),
    ("offsides", |c| c.offsides),
    ("yellow_cards", |c| c.yellow),
    ("corners", |c| c.corners),
    ("throw_ins", |c| c.throw_ins),
    ("goal_kicks", |c| c.goal_kicks),
    ("free_kicks", |c| c.free_kicks),
    ("penalties", |c| c.penalties),
    ("injuries", |c| c.injuries),
    ("substitutions", |c| c.substitutions),
];

/// A realism band's range, shown beside a season figure for information only.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub lo: f64,
    pub hi: f64,
}

/// One compared figure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Figure {
    /// `1.150 v 1.000` for a pairing, `season` for a season figure.
    pub group: String,
    pub name: String,
    pub full: f64,
    pub fast: f64,
    pub tolerance: f64,
    /// The z of the figure's family.
    pub z: f64,
    pub pass: bool,
    /// `true` when neither side has an event: the figure passes with nothing to compare.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_events: bool,
    /// The realism band's range, for information; the check never requires it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band: Option<Range>,
}

/// A fast match as the check compares it: the score and the events counted.
#[derive(Debug, Clone, PartialEq)]
pub struct FastRow {
    pub goals: [u32; 2],
    pub tally: EventTally,
}

/// The fast model's matches for every row: `draws` per row, in row order.
pub fn play_fast(
    model: &dyn FastModel,
    fit: &FastFit,
    rows: &[Row],
    draws: u32,
    seed: u64,
) -> anyhow::Result<Vec<Vec<FastRow>>> {
    rows.iter()
        .enumerate()
        .map(|(i, r)| {
            (0..draws)
                .map(|d| {
                    let s = splitmix64(
                        (0xfa57 << 48)
                            ^ (seed << 40)
                            ^ (i as u64 * u64::from(draws) + u64::from(d)),
                    );
                    let m = model.play(fit, &r.kick_off, s)?;
                    let mut tally = tally(&m.events);
                    tally.bins = Vec::new();
                    Ok(FastRow {
                        goals: m.scores,
                        tally,
                    })
                })
                .collect()
        })
        .collect()
}

/// A sample of values: its mean and the standard error of the mean.
fn stats(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    if n == 0.0 {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<f64>() / n;
    let var = if n > 1.0 {
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)
    } else {
        0.0
    };
    (mean, (var / n).sqrt())
}

/// A share's standard error is sqrt(p (1 - p) / n); a mean's is s / sqrt(n).
fn figure(group: String, name: &str, full: &[f64], fast: &[f64], share: bool, z: f64) -> Figure {
    let (f, se_f) = if share {
        share_stats(full)
    } else {
        stats(full)
    };
    let (m, se_m) = if share {
        share_stats(fast)
    } else {
        stats(fast)
    };
    let floor = if share { SHARE_FLOOR } else { MEAN_FLOOR };
    let tolerance = floor.max(z * (se_f * se_f + se_m * se_m).sqrt());
    let no_events = full.iter().chain(fast).all(|v| *v == 0.0);
    Figure {
        group,
        name: name.to_string(),
        full: f,
        fast: m,
        tolerance,
        z,
        pass: no_events || (f - m).abs() <= tolerance,
        no_events,
        band: None,
    }
}

fn share_stats(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    if n == 0.0 {
        return (0.0, 0.0);
    }
    let p = values.iter().sum::<f64>() / n;
    (p, (p * (1.0 - p) / n).sqrt())
}

/// The season figures' band ranges, by name, when the bands file is at hand.
pub type Bands = [(&'static str, Option<Range>); 9];

/// One match as a figure reads it: the score and the events counted.
type Match<'a> = (&'a [u32; 2], &'a EventTally);
/// A figure's values for one match of a pairing: one per match, or one per team.
type Values = Box<dyn Fn(usize, Match) -> Vec<f64>>;
/// A per-pairing figure: its name, its value for one match, and whether it is a share.
type PairingRule = (String, Values, bool);
/// A season figure: its name, the pairings it pools, its values, and whether it is a share.
type SeasonRule<'a> = (&'static str, &'a dyn Fn(usize) -> bool, Values, bool);

fn flag(b: bool) -> f64 {
    if b { 1.0 } else { 0.0 }
}

/// One value per match from `f`.
fn one(f: impl Fn(Match) -> f64 + 'static) -> Values {
    Box::new(move |_, m| vec![f(m)])
}

/// The per-pairing figures of the score.
fn score_rules() -> Vec<PairingRule> {
    vec![
        (
            "home_win_share".into(),
            one(|(g, _)| flag(g[0] > g[1])),
            true,
        ),
        ("draw_share".into(), one(|(g, _)| flag(g[0] == g[1])), true),
        (
            "away_win_share".into(),
            one(|(g, _)| flag(g[0] < g[1])),
            true,
        ),
        (
            "home_goals_per_match".into(),
            one(|(g, _)| f64::from(g[0])),
            false,
        ),
        (
            "away_goals_per_match".into(),
            one(|(g, _)| f64::from(g[1])),
            false,
        ),
    ]
}

/// The per-pairing figures of the events.
fn event_rules() -> Vec<PairingRule> {
    let mut out: Vec<PairingRule> = Vec::new();
    for (side, word) in ["home", "away"].into_iter().enumerate() {
        for (kind, count) in COUNTED {
            out.push((
                format!("{word}_{kind}_per_match"),
                one(move |(_, t)| f64::from(count(&t.counts[side]))),
                false,
            ));
        }
    }
    out.push((
        "sending_off_share".into(),
        one(|(_, t)| flag(t.sent_off)),
        true,
    ));
    for (half, word) in ["first", "second"].into_iter().enumerate() {
        out.push((
            format!("added_s_{word}_half"),
            one(move |(_, t)| f64::from(t.added_s[half])),
            false,
        ));
    }
    out
}

/// Compares the full engine's `rows` with the fast model's matches (`fast[i]` for
/// `rows[i]`): the score figures first, then the event figures.
pub fn compare(rows: &[Row], fast: &[Vec<FastRow>], bands: &Bands) -> Vec<Figure> {
    let mut out = Vec::with_capacity(FIGURES);
    let full_of = |keep: &dyn Fn(usize) -> bool| -> Vec<(usize, Match)> {
        rows.iter()
            .filter(|r| keep(r.pairing))
            .map(|r| (r.pairing, (&r.goals, &r.tally)))
            .collect()
    };
    let fast_of = |keep: &dyn Fn(usize) -> bool| -> Vec<(usize, Match)> {
        rows.iter()
            .zip(fast)
            .filter(|(r, _)| keep(r.pairing))
            .flat_map(|(r, list)| list.iter().map(|f| (r.pairing, (&f.goals, &f.tally))))
            .collect()
    };
    let values = |list: &[(usize, Match)], f: &Values| -> Vec<f64> {
        list.iter().flat_map(|(p, m)| f(*p, *m)).collect()
    };
    let per_pairing = |out: &mut Vec<Figure>, rules: &[PairingRule], z: f64| {
        for pairing in 0..PAIRINGS.len() {
            let this = |p: usize| p == pairing;
            let (full, quick) = (full_of(&this), fast_of(&this));
            for (name, f, share) in rules {
                out.push(figure(
                    pairing_name(pairing),
                    name,
                    &values(&full, f),
                    &values(&quick, f),
                    *share,
                    z,
                ));
            }
        }
    };
    let season = |out: &mut Vec<Figure>, rules: Vec<SeasonRule>, z: f64| {
        for (name, keep, f, share) in rules {
            let mut fig = figure(
                "season".into(),
                name,
                &values(&full_of(keep), &f),
                &values(&fast_of(keep), &f),
                share,
                z,
            );
            fig.band = bands.iter().find(|(n, _)| *n == name).and_then(|(_, r)| *r);
            out.push(fig);
        }
    };

    let equal = |p: usize| PAIRINGS[p][0] == PAIRINGS[p][1];
    let top = PAIRINGS
        .iter()
        .map(|p| p[0].max(p[1]))
        .max()
        .unwrap_or_default();
    let stronger = move |p: usize| {
        let [h, a] = PAIRINGS[p];
        (h == top && a == 0) || (h == 0 && a == top)
    };
    per_pairing(&mut out, &score_rules(), Z);
    season(
        &mut out,
        vec![
            (
                "goals_per_match",
                &equal,
                one(|(g, _)| f64::from(g[0] + g[1])),
                false,
            ),
            (
                "goalless_share",
                &equal,
                one(|(g, _)| flag(*g == [0, 0])),
                true,
            ),
            (
                "ten_plus_goals_share",
                &equal,
                one(|(g, _)| flag(g[0] + g[1] >= 10)),
                true,
            ),
            (
                "stronger_team_win_rate",
                &stronger,
                Box::new(|p, (g, _)| {
                    let side = usize::from(PAIRINGS[p][1] > PAIRINGS[p][0]);
                    vec![flag(g[side] > g[1 - side])]
                }),
                true,
            ),
        ],
        Z,
    );
    per_pairing(&mut out, &event_rules(), EVENT_Z);
    let per_team = |f: Count| -> Values {
        Box::new(move |_, (_, t)| t.counts.iter().map(|c| f64::from(f(c))).collect())
    };
    let per_match = |f: Count| one(move |(_, t)| t.counts.iter().map(|c| f64::from(f(c))).sum());
    season(
        &mut out,
        vec![
            (
                "sending_off_share",
                &equal,
                one(|(_, t)| flag(t.sent_off)),
                true,
            ),
            (
                "yellow_cards_per_team",
                &equal,
                per_team(|c| c.yellow),
                false,
            ),
            ("corners_per_team", &equal, per_team(|c| c.corners), false),
            (
                "throw_ins_per_match",
                &equal,
                per_match(|c| c.throw_ins),
                false,
            ),
            (
                "goal_kicks_per_match",
                &equal,
                per_match(|c| c.goal_kicks),
                false,
            ),
        ],
        EVENT_Z,
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fast_model::batch::Counts;
    use engine::modules::fast_model::KickOff;

    /// A hand-made tally: every side with the same counts, `fouls` fouls each.
    fn tally_of(fouls: u32) -> EventTally {
        let c = Counts {
            fouls,
            offsides: 1,
            yellow: 1,
            corners: 2,
            throw_ins: 11,
            goal_kicks: 7,
            free_kicks: 5,
            injuries: 0,
            substitutions: 5,
            ..Counts::default()
        };
        EventTally {
            counts: [c, c],
            sent_off: false,
            added_s: [90, 150],
            added_goals: [0, 0],
            bins: Vec::new(),
        }
    }

    fn rows_from(scores: &dyn Fn(usize, usize) -> [u32; 2], per: usize) -> Vec<Row> {
        (0..PAIRINGS.len())
            .flat_map(|p| {
                (0..per).map(move |k| Row {
                    pairing: p,
                    kick_off: KickOff::even([10.0, 10.0]),
                    goals: scores(p, k),
                    goal_minutes: Vec::new(),
                    tally: tally_of(6 + (k % 3) as u32),
                })
            })
            .collect()
    }

    fn base(_: usize, k: usize) -> [u32; 2] {
        match k % 10 {
            0..=3 => [1, 0],
            4..=6 => [1, 1],
            7 => [0, 0],
            _ => [0, 2],
        }
    }

    const NO_BANDS: Bands = [
        ("goals_per_match", None),
        ("goalless_share", None),
        ("ten_plus_goals_share", None),
        ("stronger_team_win_rate", None),
        ("sending_off_share", None),
        ("yellow_cards_per_team", None),
        ("corners_per_team", None),
        ("throw_ins_per_match", None),
        ("goal_kicks_per_match", None),
    ];

    fn same(rows: &[Row]) -> Vec<Vec<FastRow>> {
        rows.iter()
            .map(|r| {
                vec![
                    FastRow {
                        goals: r.goals,
                        tally: r.tally.clone(),
                    };
                    20
                ]
            })
            .collect()
    }

    #[test]
    fn equal_tables_pass_every_figure() {
        let rows = rows_from(&base, 1000);
        let figures = compare(&rows, &same(&rows), &NO_BANDS);
        assert_eq!(figures.len(), FIGURES);
        assert_eq!((SCORE_FIGURES, EVENT_FIGURES), (49, 212));
        assert!(figures.iter().all(|f| f.pass), "{figures:?}");
        // The score figures keep z 3.7, the event figures use z 4.07.
        assert!(figures[..SCORE_FIGURES].iter().all(|f| f.z == Z));
        assert!(figures[SCORE_FIGURES..].iter().all(|f| f.z == EVENT_Z));
        // Nobody is injured or sent off, nobody takes a penalty and no match has ten goals:
        // those figures pass with no events, and every other figure has some.
        let empty: Vec<&str> = figures
            .iter()
            .filter(|f| f.no_events)
            .map(|f| f.name.as_str())
            .collect();
        assert!(
            empty.iter().all(|n| n.contains("injuries")
                || n.contains("penalties")
                || n.contains("sending_off")
                || *n == "ten_plus_goals_share"),
            "{empty:?}"
        );
        assert_eq!(empty.len(), PAIRINGS.len() * 5 + 2);
    }

    /// A fast model whose draw share is 0.1 lower in one pairing fails, and the failing
    /// figure names the pairing.
    #[test]
    fn a_planted_draw_share_shift_fails_and_names_the_pairing() {
        let rows = rows_from(&base, 1000);
        let mut fast = same(&rows);
        for (i, (r, list)) in rows.iter().zip(fast.iter_mut()).enumerate() {
            // In pairing 6, one draw in ten turns into a home win.
            if r.pairing == 6 && i % 10 == 4 {
                for f in list.iter_mut() {
                    f.goals = [1, 0];
                }
            }
        }
        let figures = compare(&rows, &fast, &NO_BANDS);
        let failed: Vec<&Figure> = figures.iter().filter(|f| !f.pass).collect();
        assert!(
            failed
                .iter()
                .any(|f| f.group == "1.150 v 1.000" && f.name == "draw_share"),
            "{failed:?}"
        );
        assert!(
            failed
                .iter()
                .all(|f| f.group == "1.150 v 1.000" || f.group == "season")
        );
    }

    /// A fast model whose home side fouls once more a match in one pairing fails that
    /// figure, which names the pairing; every other figure passes.
    #[test]
    fn a_planted_foul_shift_fails_and_names_the_pairing_and_the_figure() {
        let rows = rows_from(&base, 1000);
        let mut fast = same(&rows);
        for (r, list) in rows.iter().zip(fast.iter_mut()) {
            if r.pairing == 2 {
                for f in list.iter_mut() {
                    // 6, 7 or 8 fouls become 7, 8 or 9.
                    let c = &mut f.tally.counts[0];
                    c.fouls = (c.fouls * 11).div_ceil(10);
                }
            }
        }
        let figures = compare(&rows, &fast, &NO_BANDS);
        let failed: Vec<(&str, &str)> = figures
            .iter()
            .filter(|f| !f.pass)
            .map(|f| (f.group.as_str(), f.name.as_str()))
            .collect();
        assert_eq!(failed, [("1.000 v 1.150", "home_fouls_per_match")]);
    }
}
