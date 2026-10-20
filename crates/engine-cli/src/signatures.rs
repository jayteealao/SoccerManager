//! `engine-cli signatures`: the real-team signature table (`content/signatures.json`). It
//! reads the fit data of the real-data folder and writes, for each primitive's signature
//! statistic, its spread across real teams: the quintile edges, the range and the median of
//! each quintile, the lowest and the highest team. A primitive's 5 levels are fitted to the
//! medians of the 5 quintiles.
//!
//! The fit data is the Wyscout 2017/18 club data of four leagues (England, France, Italy,
//! Spain). The held-out sets (the German league, the 2018 World Cup and Euro 2016) are never
//! read: the reader skips their rows, and a run that asks for one, or an event whose match
//! belongs to one, stops with a clear error. A StatsBomb open-data check of passes per
//! possession sits beside the rows.
//!
//! Each statistic follows the definitions the engine's measures share: a possession starts at
//! a team's on-ball action (a pass, a free kick, a shot, a touch) and passes to the other team
//! at that team's first on-ball action, unless the team that had the ball acts on it again
//! within [`engine::decision::REGAIN_S`] seconds while the other team completed no pass and
//! took no shot (a short regain, merged into the possession); a dead ball (an interruption, a
//! foul, an offside) or the period's end ends it. Its passes count its passes and its free
//! kicks (every restart kick: throw-ins, goal kicks, corners, free kicks). An open-play pass is
//! forward when it gains 5 m or more toward the attacked goal; the recycle passes are the
//! open-play passes before the possession's first forward one, and the recycle share is
//! their share of the open-play passes (a restart kick is a set play and takes no part); a
//! turnover is a possession the other team takes with no shot in it. Each statistic is pooled over a team's
//! season (sum over sum) before the quintiles are taken across the teams. PPDA is the sum of
//! the opponent's passes in its build-up zone over the sum of the team's defensive actions.
//!
//! The transition rows follow [`engine::measures::transitions`]: a regain is a possession that
//! starts with the other team's possession ending in open play (no dead ball between), at the
//! side's first touch; a fast break is a regain that starts at or behind
//! [`engine::decision::COUNTER_START_M`] and reaches the attacked final third within
//! [`engine::decision::COUNTER_WINDOW_S`] (an on-ball action of the side there). The fast-break
//! row reads the Wyscout fit leagues, with Wyscout's counter-attack tag (1901) as a check. The
//! StatsBomb open data, with the same possession rule, gives the regain-to-shot time and the
//! counter-press figures: the losing side's Pressure events within
//! [`engine::decision::COUNTERPRESS_S`] of each open-play loss, its regains within that time,
//! and its last pressure within [`engine::decision::COUNTERPRESS_LAST_S`]; one value per team
//! and competition, marked as national teams (three of the four competitions are national-team
//! tournaments), with a Bundesliga-only check and StatsBomb's own counterpress flag as checks.
//!
//! The output is a pure function of the input files: keys in a fixed order, values rounded to
//! 6 decimals, and no time stamp, so two runs write the same bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use engine::ContentDir;
use engine::data::primitives::held_out;
use engine::decision::{
    COUNTER_START_M, COUNTER_WINDOW_S, COUNTERPRESS_LAST_S, COUNTERPRESS_S, FORWARD_M, REGAIN_S,
};
use serde::Serialize;

use crate::cli::SignaturesOpts;
use crate::stats::{quantile_linear, round_to};

pub mod substitutions;

/// The source name of the Wyscout club data.
pub const WYSCOUT: &str = "wyscout_2017_18";
/// The source name of the StatsBomb open data.
pub const STATSBOMB: &str = "statsbomb_open";
/// The four leagues the fit reads by default.
pub const FIT_LEAGUES: [&str; 4] = ["England", "France", "Italy", "Spain"];
/// Wyscout positions are percentages of a 105 m pitch along its length.
const PITCH_M: f64 = 105.0;

/// The table's format version. Version 2 adds the in-possession rows and the thin-sample rows
/// of the SkillCorner runs; version 3 the transition rows and their checks. Version 4's
/// goal-kick rows read the StatsBomb open data (played under the 2019 law), with the fit
/// leagues' older lengths as checks. Version 5 adds the substitution rows of the two eras of the
/// substitution law ([substitutions]).
pub const SIGNATURES_VERSION: u32 = 5;
/// The StatsBomb competitions that are held-out sets: never read.
pub const STATSBOMB_HELD_OUT: [&str; 2] = ["FIFA World Cup 2018", "UEFA Euro 2016"];
/// The StatsBomb competition of club teams; the others are national-team tournaments.
pub const STATSBOMB_CLUBS: &str = "1. Bundesliga 2023/2024";
/// The source name of the SkillCorner open data.
pub const SKILLCORNER: &str = "skillcorner_open";
/// The competition of the SkillCorner open data.
pub const SKILLCORNER_LEAGUE: &str = "A-League";
/// Wyscout positions are percentages of a 68 m pitch across.
const WIDTH_M: f64 = 68.0;
/// A pass of this length or more is a long ball, as in the engine's measures.
pub const LONG_BALL_M: f64 = 32.0;
/// A point more than this far from the centre line lies in a wide channel, as in the engine.
pub const WIDE_M: f64 = 20.0;
/// The penalty area: its depth from the goal line and its half width, in metres.
const BOX_DEPTH_M: f64 = 16.5;
const BOX_HALF_WIDTH_M: f64 = 20.16;
/// The SkillCorner positions of the full-backs.
pub const SC_FULL_BACKS: [&str; 4] = ["LB", "RB", "LWB", "RWB"];

/// One row: a primitive's statistic across the real teams.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    pub primitive: String,
    pub stat: String,
    pub source: String,
    pub competitions: Vec<String>,
    pub teams: usize,
    pub matches: usize,
    /// The values at 20, 40, 60 and 80 percent of the teams.
    pub quintile_edges: [f64; 4],
    /// Each quintile's lowest and highest value.
    pub quintile_ranges: [[f64; 2]; 5],
    /// The median of the teams in each quintile: the fit point of each level. In a
    /// thin-sample row, the five targets.
    pub medians: [f64; 5],
    pub min: f64,
    pub max: f64,
    /// A row from too few teams for quintiles: its five targets are spread evenly between
    /// the lowest and the highest team, each level's range is half a level step either side
    /// of its target, and the fit says so.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub thin_sample: bool,
    /// A row from national teams (a World Cup), not clubs.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub national_teams: bool,
}

/// A check of a statistic in another source.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Check {
    pub stat: String,
    pub source: String,
    pub competitions: Vec<String>,
    pub teams: usize,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    /// The rank correlation across the teams with the row of `against`, when the check
    /// compares a definition with a row's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank_correlation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub against: Option<String>,
}

impl Check {
    /// The check of `values` (one per team).
    fn of(stat: &str, source: &str, competitions: &[String], values: &[f64]) -> Option<Self> {
        let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
        if v.is_empty() {
            return None;
        }
        v.sort_by(f64::total_cmp);
        Some(Self {
            stat: stat.into(),
            source: source.into(),
            competitions: competitions.to_vec(),
            teams: v.len(),
            median: round6(quantile_linear(&v, 0.5)),
            min: round6(v[0]),
            max: round6(v[v.len() - 1]),
            rank_correlation: None,
            against: None,
        })
    }
}

/// The ranks of `v`, the mean rank for ties.
fn ranks(v: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..v.len()).collect();
    order.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    let mut out = vec![0.0; v.len()];
    let mut i = 0;
    while i < order.len() {
        let mut j = i;
        while j + 1 < order.len() && v[order[j + 1]] == v[order[i]] {
            j += 1;
        }
        let rank = (i + j) as f64 / 2.0;
        for &k in &order[i..=j] {
            out[k] = rank;
        }
        i = j + 1;
    }
    out
}

/// Spearman's rank correlation of `a` and `b`: the Pearson correlation of their ranks.
fn spearman(a: &[f64], b: &[f64]) -> f64 {
    let (ra, rb) = (ranks(a), ranks(b));
    let n = ra.len() as f64;
    let (ma, mb) = (ra.iter().sum::<f64>() / n, rb.iter().sum::<f64>() / n);
    let cov: f64 = ra.iter().zip(&rb).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let va: f64 = ra.iter().map(|x| (x - ma).powi(2)).sum();
    let vb: f64 = rb.iter().map(|y| (y - mb).powi(2)).sum();
    cov / (va * vb).sqrt()
}

/// The table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Table {
    pub schema_version: u32,
    /// The held-out sets the table never read, as `source:competition`.
    pub held_out: Vec<String>,
    pub rows: Vec<Row>,
    pub checks: Vec<Check>,
    /// The substitutions of each era, when the real-data folder holds their files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub substitutions: Option<substitutions::Substitutions>,
}

/// The cells of one CSV line; a quoted cell may hold a comma and a doubled quote.
pub fn cells(line: &str) -> Vec<String> {
    let line = line.trim_end_matches(['\r', '\n']);
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// The column of `name` in `header`.
fn column(header: &[String], name: &str, file: &str) -> anyhow::Result<usize> {
    header
        .iter()
        .position(|h| h == name)
        .with_context(|| format!("{file}: no column {name}"))
}

/// The lines of `path`, read through gzip when its name ends in `.gz`.
fn lines(path: &Path) -> anyhow::Result<Box<dyn Iterator<Item = std::io::Result<String>>>> {
    let file = File::open(path).with_context(|| format!("cannot read {}", path.display()))?;
    Ok(if path.extension().is_some_and(|e| e == "gz") {
        Box::new(BufReader::new(flate2::read::MultiGzDecoder::new(file)).lines())
    } else {
        Box::new(BufReader::new(file).lines())
    })
}

fn round6(x: f64) -> f64 {
    round_to(x, 6)
}

/// The row of `values` (one per team).
pub fn spread(
    primitive: &str,
    stat: &str,
    competitions: &[String],
    matches: usize,
    values: &[f64],
) -> anyhow::Result<Row> {
    if values.len() < 5 {
        bail!(
            "{primitive} {stat}: {} teams; a quintile table needs at least 5",
            values.len()
        );
    }
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let edges: [f64; 4] = std::array::from_fn(|i| quantile_linear(&v, (i + 1) as f64 / 5.0));
    let quintile_of = |x: f64| edges.iter().filter(|&&e| x > e).count();
    let mut groups: [Vec<f64>; 5] = Default::default();
    for &x in &v {
        groups[quintile_of(x)].push(x);
    }
    let lo = |q: usize| if q == 0 { v[0] } else { edges[q - 1] };
    let hi = |q: usize| if q == 4 { v[v.len() - 1] } else { edges[q] };
    let medians: [f64; 5] = std::array::from_fn(|q| {
        if groups[q].is_empty() {
            (lo(q) + hi(q)) / 2.0
        } else {
            quantile_linear(&groups[q], 0.5)
        }
    });
    Ok(Row {
        primitive: primitive.into(),
        stat: stat.into(),
        source: WYSCOUT.into(),
        competitions: competitions.to_vec(),
        teams: v.len(),
        matches,
        quintile_edges: edges.map(round6),
        quintile_ranges: std::array::from_fn(|q| [round6(lo(q)), round6(hi(q))]),
        medians: medians.map(round6),
        min: round6(v[0]),
        max: round6(v[v.len() - 1]),
        thin_sample: false,
        national_teams: false,
    })
}

/// The thin-sample row of `values` (one per team): five targets spread evenly between the
/// lowest and the highest team, the edges halfway between them, and each level's range half
/// a level step either side of its target.
pub fn thin(
    primitive: &str,
    stat: &str,
    source: &str,
    competitions: &[String],
    matches: usize,
    values: &[f64],
) -> anyhow::Result<Row> {
    if values.len() < 2 {
        bail!(
            "{primitive} {stat}: {} teams; a thin-sample row needs at least 2",
            values.len()
        );
    }
    let (min, max) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| {
            (a.min(x), b.max(x))
        });
    let step = (max - min) / 4.0;
    let targets: [f64; 5] = std::array::from_fn(|q| min + step * q as f64);
    Ok(Row {
        primitive: primitive.into(),
        stat: stat.into(),
        source: source.into(),
        competitions: competitions.to_vec(),
        teams: values.len(),
        matches,
        quintile_edges: std::array::from_fn(|i| round6(targets[i] + step / 2.0)),
        quintile_ranges: targets.map(|t| [round6(t - step / 2.0), round6(t + step / 2.0)]),
        medians: targets.map(round6),
        min: round6(min),
        max: round6(max),
        thin_sample: true,
        national_teams: false,
    })
}

/// One team's season.
#[derive(Debug, Clone, Copy, Default)]
struct Season {
    possessions: u64,
    passes: u64,
    open_passes: u64,
    recycle: u64,
    turnovers: u64,
    shot_possessions: u64,
    opp_build: f64,
    def_actions: f64,
    /// Matches the team played (rows of `wy2_team_matches.csv`).
    team_matches: f64,
    /// Possessions with an action of the team in the final third.
    final_third_possessions: f64,
    /// Open-play passes (`Pass` events): all, forward (5 m or more toward goal), long (32 m or
    /// more), those starting in the own third and the long ones among them, those ending in
    /// the middle or final third and the wide ones among them.
    op_passes: f64,
    op_forward: f64,
    op_long: f64,
    own_third: f64,
    own_third_long: f64,
    upfield_end: f64,
    wide_end: f64,
    /// Completed open-play passes from outside into the final third, by the end point's
    /// channel: left, centre, right.
    entries: [f64; 3],
    goal_kicks: f64,
    goal_kick_m: f64,
    /// The keeper's open-play passes and clearances, and the long ones (every clearance).
    keeper_actions: f64,
    keeper_long: f64,
    /// Open-play shots, those from outside the penalty area, and their summed distance.
    shots: f64,
    outside_shots: f64,
    shot_m: f64,
    takeons: f64,
    /// Open-play crosses, the high ones (tag 801), and their summed distance from the goal
    /// line.
    crosses: f64,
    high_crosses: f64,
    cross_origin_m: f64,
    /// Possessions that started with a regain, the fast breaks among them, those with a shot
    /// and the summed seconds from their start to the first shot; and the possessions with an
    /// on-ball action Wyscout tagged as a counter-attack (1901).
    regains: f64,
    fast_breaks: f64,
    regain_shots: f64,
    regain_shot_s: f64,
    tagged_counters: f64,
    /// Defensive actions (ground defending duels, sliding tackles, interceptions and fouls)
    /// anywhere, and the sum of their heights in metres from the own goal line; the same for
    /// those of defenders (role DF).
    def_all: f64,
    def_all_m: f64,
    def_df: f64,
    def_df_m: f64,
    /// Fouls and yellow cards (rows of `wy2_team_matches.csv`), over the matches whose row
    /// carries them.
    fouls: f64,
    yellows: f64,
    card_matches: f64,
    /// The matches whose opponent is known: the denominator of the opponents' offsides.
    opp_matches: f64,
    /// The opponents' completed entries into the final third by channel, their goal kicks and
    /// the sum of their lengths, and their offsides: what the team let them do.
    opp_entries: [f64; 3],
    opp_goal_kicks: f64,
    opp_goal_kick_m: f64,
    offsides_caught: f64,
}

/// One team's counts in one match that its opponent's season reads.
#[derive(Debug, Clone, Copy, Default)]
struct MatchSide {
    entries: [f64; 3],
    goal_kicks: f64,
    goal_kick_m: f64,
    offsides: f64,
}

/// A defensive action of a team's season `s`: a ground defending duel, a sliding tackle, an
/// interception (tag 1401) or a foul, at `x0` percent from its own goal line, by a defender
/// when `role` is DF.
fn count_defending(s: &mut Season, ev: &str, sub: &str, role: &str, x0: Option<f64>, tags: &str) {
    let intercept = tags.split([' ', ';']).any(|t| t == "1401");
    let action =
        matches!(sub, "Ground defending duel" | "Sliding tackle") || intercept || ev == "Foul";
    let Some(x0) = x0.filter(|_| action) else {
        return;
    };
    let m = x0 * PITCH_M / 100.0;
    s.def_all += 1.0;
    s.def_all_m += m;
    if role == "DF" {
        s.def_df += 1.0;
        s.def_df_m += m;
    }
}

/// The possession in progress.
#[derive(Debug, Clone, Copy)]
struct Possession {
    team: u64,
    passes: u64,
    open_passes: u64,
    recycle: u64,
    forward: bool,
    shot: bool,
    final_third: bool,
    /// Its first touch: the second, the depth past halfway toward the attacked goal in
    /// metres, and whether it is a regain (the other team's possession ended in open play).
    start: f64,
    start_m: f64,
    regain: bool,
    /// The second it first reached the attacked final third, and of its first shot.
    final_at: Option<f64>,
    shot_at: Option<f64>,
    /// An on-ball action tagged as a counter-attack (1901).
    tagged: bool,
}

/// One on-ball action as a possession reads it: a shot, a restart kick, or an open-play pass
/// gaining `gain` metres; `final_third` when it starts in the final third; `tagged` when
/// Wyscout tagged it as a counter-attack.
#[derive(Debug, Clone, Copy)]
struct Act {
    sec: f64,
    shot: bool,
    pass: bool,
    restart: bool,
    gain: f64,
    final_third: bool,
    tagged: bool,
}

impl Possession {
    fn new(team: u64, start: f64, start_m: f64, regain: bool) -> Self {
        Self {
            team,
            passes: 0,
            open_passes: 0,
            recycle: 0,
            forward: false,
            shot: false,
            final_third: false,
            start,
            start_m,
            regain,
            final_at: None,
            shot_at: None,
            tagged: false,
        }
    }

    fn act(&mut self, a: Act) {
        let Act {
            sec,
            shot,
            pass,
            restart,
            gain,
            final_third,
            tagged,
        } = a;
        self.final_third |= final_third;
        self.tagged |= tagged;
        if final_third && self.final_at.is_none() {
            self.final_at = Some(sec);
        }
        if shot {
            self.shot = true;
            self.shot_at.get_or_insert(sec);
        } else if pass {
            self.passes += 1;
            if restart {
                return;
            }
            self.open_passes += 1;
            if gain >= FORWARD_M {
                self.forward = true;
            } else if !self.forward {
                self.recycle += 1;
            }
        }
    }
}

/// The other team's actions since its first at `start` seconds: its possession if it keeps the
/// ball, else a short regain merged away.
#[derive(Debug, Clone, Copy)]
struct Spell {
    start: f64,
    poss: Possession,
}

/// The spell becomes the other team's possession; the open one ends as a turnover.
fn take_over(
    open: &mut Option<Possession>,
    spell: &mut Option<Spell>,
    seasons: &mut BTreeMap<u64, Season>,
) {
    if let Some(s) = spell.take() {
        close(open.take(), End::Turnover, seasons);
        *open = Some(s.poss);
    }
}

/// How a possession ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    Turnover,
    Other,
}

fn close(p: Option<Possession>, end: End, seasons: &mut BTreeMap<u64, Season>) {
    let Some(p) = p else { return };
    let s = seasons.entry(p.team).or_default();
    s.possessions += 1;
    s.passes += p.passes;
    s.open_passes += p.open_passes;
    s.recycle += p.recycle;
    if p.final_third {
        s.final_third_possessions += 1.0;
    }
    if p.shot {
        s.shot_possessions += 1;
    } else if end == End::Turnover {
        s.turnovers += 1;
    }
    if p.tagged {
        s.tagged_counters += 1.0;
    }
    if p.regain {
        s.regains += 1.0;
        if p.start_m <= COUNTER_START_M
            && p.final_at.is_some_and(|f| f - p.start <= COUNTER_WINDOW_S)
        {
            s.fast_breaks += 1.0;
        }
        if let Some(t) = p.shot_at {
            s.regain_shots += 1.0;
            s.regain_shot_s += t - p.start;
        }
    }
}

/// A Wyscout point (percent along the attack, percent across from the attacking side's
/// left) in metres: along from the own goal line, across from the centre line, left negative.
fn metres(x: f64, y: f64) -> (f64, f64) {
    (x * PITCH_M / 100.0, (y - 50.0) * WIDTH_M / 100.0)
}

/// The channel of a point `y` metres across: 0 left, 1 centre, 2 right.
fn channel(y: f64) -> usize {
    if y < -WIDE_M {
        0
    } else if y > WIDE_M {
        2
    } else {
        1
    }
}

/// The counts of one event of a team's season `s` that the in-possession rows read: `at` is
/// `x0, y0, x1, y1` in percent, `role` the player's role, `tags` the event's tags.
fn count_event(
    s: &mut Season,
    side: &mut MatchSide,
    ev: &str,
    sub: &str,
    role: &str,
    at: [Option<f64>; 4],
    tags: &str,
) {
    let has = |t: &str| tags.split([' ', ';']).any(|x| x == t);
    let [Some(x0), Some(y0)] = [at[0], at[1]] else {
        return;
    };
    let (ax, ay) = metres(x0, y0);
    let end = match (at[2], at[3]) {
        (Some(x1), Some(y1)) => Some(metres(x1, y1)),
        _ => None,
    };
    let third = PITCH_M / 3.0;
    match (ev, sub) {
        ("Pass", _) => {
            let Some((bx, by)) = end else { return };
            let length = (bx - ax).hypot(by - ay);
            s.op_passes += 1.0;
            if bx - ax >= FORWARD_M {
                s.op_forward += 1.0;
            }
            let long = length >= LONG_BALL_M;
            if long {
                s.op_long += 1.0;
            }
            if ax < third {
                s.own_third += 1.0;
                if long {
                    s.own_third_long += 1.0;
                }
            }
            if bx >= third {
                s.upfield_end += 1.0;
                if channel(by) != 1 {
                    s.wide_end += 1.0;
                }
            }
            if has("1801") && ax < 2.0 * third && bx >= 2.0 * third {
                s.entries[channel(by)] += 1.0;
                side.entries[channel(by)] += 1.0;
            }
            if role == "GK" {
                s.keeper_actions += 1.0;
                if long {
                    s.keeper_long += 1.0;
                }
            }
            if sub == "Cross" {
                s.crosses += 1.0;
                if has("801") {
                    s.high_crosses += 1.0;
                }
                s.cross_origin_m += PITCH_M - ax;
            }
        }
        ("Free Kick", "Goal kick") => {
            if let Some((bx, by)) = end {
                s.goal_kicks += 1.0;
                s.goal_kick_m += (bx - ax).hypot(by - ay);
                side.goal_kicks += 1.0;
                side.goal_kick_m += (bx - ax).hypot(by - ay);
            }
        }
        ("Others on the ball", "Clearance") if role == "GK" => {
            s.keeper_actions += 1.0;
            s.keeper_long += 1.0;
        }
        ("Shot", _) => {
            let from_line = PITCH_M - ax;
            s.shots += 1.0;
            if !(from_line <= BOX_DEPTH_M && ay.abs() <= BOX_HALF_WIDTH_M) {
                s.outside_shots += 1.0;
            }
            s.shot_m += from_line.hypot(ay);
        }
        ("Duel", "Ground attacking duel") if has("503") || has("504") => s.takeons += 1.0,
        _ => {}
    }
}

/// The competition of each match of `wy2_matches.csv`.
fn match_competitions(dir: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    let file = "wy2_matches.csv";
    let mut it = lines(&dir.join(file))?;
    let header = cells(&it.next().context("an empty wy2_matches.csv")??);
    let (comp, id) = (
        column(&header, "comp", file)?,
        column(&header, "match", file)?,
    );
    let mut out = BTreeMap::new();
    for line in it {
        let c = cells(&line?);
        out.insert(c[id].clone(), c[comp].clone());
    }
    Ok(out)
}

/// The refusal of a held-out competition.
fn refuse_held_out(competition: &str) -> anyhow::Result<()> {
    if held_out(WYSCOUT, competition) {
        bail!("held-out set: {competition} is never read");
    }
    Ok(())
}

/// The seasons of the teams of `competitions`, from the Wyscout files of `dir`, and the
/// number of matches read.
fn wyscout_seasons(
    dir: &Path,
    competitions: &[String],
) -> anyhow::Result<(BTreeMap<u64, Season>, usize)> {
    let comps = match_competitions(dir)?;
    let wanted: BTreeSet<&str> = competitions.iter().map(String::as_str).collect();
    let mut seasons: BTreeMap<u64, Season> = BTreeMap::new();

    let file = "wy2_team_matches.csv";
    let mut it = lines(&dir.join(file))?;
    let header = cells(&it.next().context("an empty wy2_team_matches.csv")??);
    let (m, team, build, def) = (
        column(&header, "match", file)?,
        column(&header, "team", file)?,
        column(&header, "opp_build_vs", file)?,
        column(&header, "def_act", file)?,
    );
    // The out-of-possession columns; a file without them gives no out-of-possession rows.
    let optional = |n: &str| header.iter().position(|h| h == n);
    let (opp, fouls, yellows) = (optional("opp"), optional("fouls"), optional("yellows"));
    // Each team's opponent in each match of the competitions read.
    let mut opponents: Vec<(String, u64, u64)> = Vec::new();
    for line in it {
        let c = cells(&line?);
        let Some(comp) = comps.get(&c[m]) else {
            continue;
        };
        if !wanted.contains(comp.as_str()) {
            continue;
        }
        let team: u64 = c[team].parse().context("a team id")?;
        let s = seasons.entry(team).or_default();
        s.opp_build += c[build].parse::<f64>().unwrap_or(0.0);
        s.def_actions += c[def].parse::<f64>().unwrap_or(0.0);
        let num = |i: Option<usize>| i.and_then(|i| c.get(i)?.parse::<f64>().ok());
        if let (Some(f), Some(y)) = (num(fouls), num(yellows)) {
            s.fouls += f;
            s.yellows += y;
            s.card_matches += 1.0;
        }
        s.team_matches += 1.0;
        if let Some(o) = opp.and_then(|i| c.get(i)?.parse::<u64>().ok()) {
            s.opp_matches += 1.0;
            opponents.push((c[m].clone(), team, o));
        }
    }
    let mut sides: BTreeMap<(String, u64), MatchSide> = BTreeMap::new();

    let file = "wy2_events.csv.gz";
    let mut it = lines(&dir.join(file))?;
    let header = cells(&it.next().context("an empty wy2_events.csv.gz")??);
    let col = |n: &str| column(&header, n, file);
    let (c_comp, c_match, c_period, c_sec, c_team, c_ev, c_sub, c_x0, c_x1, c_tags) = (
        col("comp")?,
        col("match")?,
        col("period")?,
        col("sec")?,
        col("team")?,
        col("ev")?,
        col("sub")?,
        col("x0")?,
        col("x1")?,
        col("tags")?,
    );
    let (c_role, c_y0, c_y1) = (col("role")?, col("y0")?, col("y1")?);
    let mut matches = BTreeSet::new();
    let mut half: Option<(String, String)> = None;
    let mut open: Option<Possession> = None;
    let mut spell: Option<Spell> = None;
    let mut last_sec = 0.0;
    for line in it {
        let line = line?;
        // Skip the other competitions before the rest of the row is parsed.
        let comp = line.split(',').nth(c_comp).unwrap_or_default();
        if !wanted.contains(comp) {
            continue;
        }
        let c = cells(&line);
        match comps.get(&c[c_match]) {
            Some(real) => refuse_held_out(real)?,
            None => bail!("match {} is not in wy2_matches.csv", c[c_match]),
        }
        let key = (c[c_match].clone(), c[c_period].clone());
        if half.as_ref() != Some(&key) {
            // The period's end: a spell is merged away, the possession ends.
            spell = None;
            close(open.take(), End::Other, &mut seasons);
            matches.insert(key.0.clone());
            half = Some(key);
        }
        let sec: f64 = c[c_sec].parse().unwrap_or(last_sec);
        last_sec = sec;
        if spell.is_some_and(|s| sec - s.start > REGAIN_S) {
            take_over(&mut open, &mut spell, &mut seasons);
        }
        let (ev, sub) = (c[c_ev].as_str(), c[c_sub].as_str());
        if let Ok(team) = c[c_team].parse::<u64>() {
            let x0 = c[c_x0].parse::<f64>().ok().filter(|v| v.is_finite());
            count_defending(
                seasons.entry(team).or_default(),
                ev,
                sub,
                &c[c_role],
                x0,
                &c[c_tags],
            );
            if ev == "Offside" {
                sides
                    .entry((c[c_match].clone(), team))
                    .or_default()
                    .offsides += 1.0;
            }
        }
        if matches!(ev, "Interruption" | "Foul" | "Offside") {
            spell = None;
            close(open.take(), End::Other, &mut seasons);
            continue;
        }
        let num = |i: usize| c[i].parse::<f64>().ok().filter(|v| v.is_finite());
        let at = [num(c_x0), num(c_y0), num(c_x1), num(c_y1)];
        if let Ok(team) = c[c_team].parse::<u64>() {
            count_event(
                seasons.entry(team).or_default(),
                sides.entry((c[c_match].clone(), team)).or_default(),
                ev,
                sub,
                &c[c_role],
                at,
                &c[c_tags],
            );
        }
        if !matches!(ev, "Pass" | "Free Kick" | "Shot" | "Others on the ball") {
            continue;
        }
        let team: u64 = c[c_team].parse().context("a team id")?;
        let final_third = at[0].is_some_and(|x| x >= 200.0 / 3.0);
        let shot =
            ev == "Shot" || (ev == "Free Kick" && matches!(sub, "Free kick shot" | "Penalty"));
        let pass = !shot && matches!(ev, "Pass" | "Free Kick");
        let restart = ev == "Free Kick";
        let gain = match (c[c_x0].parse::<f64>(), c[c_x1].parse::<f64>()) {
            (Ok(x0), Ok(x1)) => (x1 - x0) * PITCH_M / 100.0,
            _ => 0.0,
        };
        let act = Act {
            sec,
            shot,
            pass,
            restart,
            gain,
            final_third,
            tagged: c[c_tags].split([' ', ';']).any(|t| t == "1901"),
        };
        // The depth past halfway of the first touch; none without a position.
        let start_m = at[0].map_or(f64::INFINITY, |x| (x - 50.0) * PITCH_M / 100.0);
        match open.as_mut() {
            None => {
                let mut p = Possession::new(team, sec, start_m, false);
                p.act(act);
                open = Some(p);
            }
            Some(p) if p.team == team => {
                // The team with the ball acts again: a short regain.
                spell = None;
                p.act(act);
            }
            Some(_) => {
                let s = spell.get_or_insert(Spell {
                    start: sec,
                    poss: Possession::new(team, sec, start_m, true),
                });
                s.poss.act(act);
                let completed = pass && c[c_tags].split([' ', ';']).any(|t| t == "1801");
                if shot || completed {
                    take_over(&mut open, &mut spell, &mut seasons);
                }
            }
        }
    }
    close(open.take(), End::Other, &mut seasons);
    // What each team's opponents did against it.
    for (game, team, opp) in opponents {
        let Some(o) = sides.get(&(game, opp)).copied() else {
            continue;
        };
        let s = seasons.entry(team).or_default();
        for (a, b) in s.opp_entries.iter_mut().zip(o.entries) {
            *a += b;
        }
        s.opp_goal_kicks += o.goal_kicks;
        s.opp_goal_kick_m += o.goal_kick_m;
        s.offsides_caught += o.offsides;
    }
    seasons.retain(|_, s| s.possessions > 0);
    Ok((seasons, matches.len()))
}

/// A team's pass count and its set of (match, possession) keys.
type PassesAndPossessions = (u64, BTreeSet<(String, String)>);

/// The StatsBomb check of passes per possession: per team and competition, the passes of
/// the team in possession over its possessions; `None` without the file.
fn statsbomb_check(dir: &Path) -> anyhow::Result<Option<Check>> {
    let path = dir.join("sb2_events.csv.gz");
    if !path.exists() {
        return Ok(None);
    }
    let file = "sb2_events.csv.gz";
    let mut it = lines(&path)?;
    let header = cells(&it.next().context("an empty sb2_events.csv.gz")??);
    let col = |n: &str| column(&header, n, file);
    let (c_mid, c_type, c_team, c_poss, c_poss_team, c_comp, c_season) = (
        col("mid")?,
        col("type")?,
        col("team")?,
        col("poss")?,
        col("poss_team")?,
        col("comp")?,
        col("season")?,
    );
    // (comp season, team) -> (passes, possessions)
    let mut teams: BTreeMap<(String, String), PassesAndPossessions> = BTreeMap::new();
    let mut competitions = BTreeSet::new();
    for line in it {
        let c = cells(&line?);
        if c.len() <= c_season {
            continue;
        }
        let comp = format!("{} {}", c[c_comp], c[c_season]);
        let key = (comp.clone(), c[c_poss_team].clone());
        let entry = teams.entry(key).or_default();
        entry.1.insert((c[c_mid].clone(), c[c_poss].clone()));
        if c[c_type] == "Pass" && c[c_team] == c[c_poss_team] {
            entry.0 += 1;
        }
        competitions.insert(comp);
    }
    let values: Vec<f64> = teams
        .values()
        .filter(|(_, p)| !p.is_empty())
        .map(|(n, p)| *n as f64 / p.len() as f64)
        .collect();
    let competitions: Vec<String> = competitions.into_iter().collect();
    Ok(Check::of(
        "passes_per_possession",
        STATSBOMB,
        &competitions,
        &values,
    ))
}

/// One SkillCorner team's full-back runs: its full-backs' overlaps and underlaps, and its
/// matches.
#[derive(Debug, Clone, Default)]
struct TeamRuns {
    overlaps: f64,
    underlaps: f64,
    matches: BTreeSet<String>,
}

/// The thin-sample rows of the SkillCorner open data in `dir` (`sc_dynamic_events.csv.gz`):
/// per team, its full-backs' overlaps, underlaps and both per match. None without the file.
fn skillcorner_rows(dir: &Path) -> anyhow::Result<Vec<Row>> {
    let file = "sc_dynamic_events.csv.gz";
    let path = dir.join(file);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut it = lines(&path)?;
    let header = cells(&it.next().context("an empty sc_dynamic_events.csv.gz")??);
    let col = |n: &str| column(&header, n, file);
    let (c_match, c_team, c_type, c_sub, c_pos) = (
        col("match_id")?,
        col("team_shortname")?,
        col("event_type")?,
        col("event_subtype")?,
        col("player_position")?,
    );
    let mut teams: BTreeMap<String, TeamRuns> = BTreeMap::new();
    let mut matches = BTreeSet::new();
    for line in it {
        let c = cells(&line?);
        if c.len() <= c_pos.max(c_sub).max(c_team) {
            continue;
        }
        let t = teams.entry(c[c_team].clone()).or_default();
        t.matches.insert(c[c_match].clone());
        matches.insert(c[c_match].clone());
        if c[c_type] != "off_ball_run" {
            continue;
        }
        let pos = c[c_pos].as_str();
        let sub = c[c_sub].as_str();
        if SC_FULL_BACKS.contains(&pos) {
            match sub {
                "overlap" => t.overlaps += 1.0,
                "underlap" => t.underlaps += 1.0,
                _ => {}
            }
        }
    }
    let competitions = [SKILLCORNER_LEAGUE.to_string()];
    let per_match = |f: &dyn Fn(&TeamRuns) -> f64| -> Vec<f64> {
        teams
            .values()
            .filter(|t| !t.matches.is_empty())
            .map(|t| f(t) / t.matches.len() as f64)
            .collect()
    };
    let n = matches.len();
    Ok(vec![
        thin(
            "fullback_run_frequency",
            "fullback_runs_per_match",
            SKILLCORNER,
            &competitions,
            n,
            &per_match(&|t| t.overlaps + t.underlaps),
        )?,
        thin(
            "fullback_runs",
            "overlaps_per_match",
            SKILLCORNER,
            &competitions,
            n,
            &per_match(&|t| t.overlaps),
        )?,
        thin(
            "fullback_runs",
            "underlaps_per_match",
            SKILLCORNER,
            &competitions,
            n,
            &per_match(&|t| t.underlaps),
        )?,
    ])
}

/// The source the forwards' run rows are fitted to: the 2022 World Cup's tracking data.
pub const PFF: &str = "pff_wc2022";
/// The forwards' runs from tracking data, one per line, by `tracking_runs.py` (its header
/// names the columns the reader needs).
pub const TRACKING_RUNS: &str = "tracking_runs.csv.gz";

/// One team's forwards' runs in the tracking data: all, in behind, coming short; its matches;
/// and whether it is a national team.
#[derive(Debug, Clone, Default)]
struct TeamTrackedRuns {
    runs: f64,
    behind: f64,
    short: f64,
    matches: BTreeSet<String>,
    national: bool,
}

/// The rows and checks of the forwards' runs from tracking data in `dir`
/// ([`TRACKING_RUNS`]): per team, the share of its forwards' runs that go in behind and the
/// share that come short, by [`engine::run_kind::RunEnds`]. The PFF World Cup teams give the
/// thin-sample rows (national teams, marked as such); every other source gives a check. None
/// without the file.
fn tracking_rows(dir: &Path) -> anyhow::Result<(Vec<Row>, Vec<Check>)> {
    let path = dir.join(TRACKING_RUNS);
    if !path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut it = lines(&path)?;
    let header = cells(
        &it.next()
            .with_context(|| format!("an empty {TRACKING_RUNS}"))??,
    );
    let col = |n: &str| column(&header, n, TRACKING_RUNS);
    let (c_src, c_comp, c_nat, c_match, c_team) = (
        col("source")?,
        col("competition")?,
        col("national")?,
        col("match")?,
        col("team")?,
    );
    let facts: [usize; 8] = [
        col("depth0")?,
        col("depth1")?,
        col("line0")?,
        col("line1")?,
        col("ball0")?,
        col("ball1")?,
        col("balld0")?,
        col("balld1")?,
    ];
    // (source, competition) -> team -> runs
    let mut by: BTreeMap<(String, String), BTreeMap<String, TeamTrackedRuns>> = BTreeMap::new();
    for line in it {
        let c = cells(&line?);
        if c.len() < header.len() {
            continue;
        }
        let mut v = [0.0; 8];
        for (x, &i) in v.iter_mut().zip(&facts) {
            *x = c[i]
                .parse()
                .with_context(|| format!("{TRACKING_RUNS}: a number, not {}", c[i]))?;
        }
        let ends = engine::run_kind::RunEnds {
            depth: [v[0], v[1]],
            line: [v[2], v[3]],
            ball: [v[4], v[5]],
            ball_m: [v[6], v[7]],
        };
        let t = by
            .entry((c[c_src].clone(), c[c_comp].clone()))
            .or_default()
            .entry(c[c_team].clone())
            .or_default();
        t.runs += 1.0;
        t.behind += f64::from(u8::from(ends.in_behind()));
        t.short += f64::from(u8::from(ends.coming_short()));
        t.matches.insert(c[c_match].clone());
        t.national = c[c_nat] == "1";
    }
    let mut rows = Vec::new();
    let mut checks = Vec::new();
    for ((source, competition), teams) in &by {
        let competitions = [competition.clone()];
        let national = teams.values().all(|t| t.national);
        let matches: BTreeSet<&String> = teams.values().flat_map(|t| &t.matches).collect();
        let share = |f: &dyn Fn(&TeamTrackedRuns) -> f64| -> Vec<f64> {
            teams
                .values()
                .filter(|t| t.runs > 0.0)
                .map(|t| f(t) / t.runs)
                .collect()
        };
        let stats: [(&str, Vec<f64>); 2] = [
            ("runs_in_behind_share", share(&|t| t.behind)),
            ("coming_short_share", share(&|t| t.short)),
        ];
        for (stat, values) in stats {
            if source == PFF {
                let mut row = thin(
                    "forward_runs",
                    stat,
                    source,
                    &competitions,
                    matches.len(),
                    &values,
                )?;
                row.national_teams = national;
                rows.push(row);
            } else {
                checks.extend(Check::of(stat, source, &competitions, &values));
            }
        }
    }
    Ok((rows, checks))
}

/// One StatsBomb team's transitions in one competition (sums and counts, as the engine's
/// [`engine::measures::transitions::Transitions`]).
#[derive(Debug, Clone, Default)]
struct SbTeam {
    matches: BTreeSet<String>,
    regain_shots: f64,
    regain_shot_s: f64,
    losses: f64,
    pressures: f64,
    flagged: f64,
    flagged_in_window: f64,
    regains_in_window: f64,
    last_pressures: f64,
    last_pressure_s: f64,
}

/// An open-play loss of a StatsBomb team: when the other team first touched the ball, its
/// pressures since, and the last of them within [`COUNTERPRESS_LAST_S`].
#[derive(Debug, Clone, Copy)]
struct SbLoss {
    team: usize,
    at: f64,
    pressures: f64,
    flagged: f64,
    last: Option<f64>,
    regained: bool,
}

impl SbLoss {
    fn count(self, t: &mut SbTeam) {
        t.losses += 1.0;
        t.pressures += self.pressures;
        t.flagged_in_window += self.flagged;
        if self.regained {
            t.regains_in_window += 1.0;
        }
        if let Some(last) = self.last {
            t.last_pressures += 1.0;
            t.last_pressure_s += last - self.at;
        }
    }
}

/// A StatsBomb possession in progress, on the shared rule: its team (an index into the
/// teams), its first touch, whether it is a regain, and its first shot.
#[derive(Debug, Clone, Copy)]
struct SbPossession {
    team: usize,
    start: f64,
    regain: bool,
    shot_at: Option<f64>,
}

/// The seconds of a StatsBomb time stamp `hh:mm:ss.fff` within its period.
fn sb_seconds(ts: &str) -> Option<f64> {
    let mut it = ts.split(':');
    let (h, m, s) = (it.next()?, it.next()?, it.next()?);
    Some(h.parse::<f64>().ok()? * 3600.0 + m.parse::<f64>().ok()? * 60.0 + s.parse::<f64>().ok()?)
}

/// The StatsBomb teams, as (competition, team), and their transition counts.
type SbTeams = (Vec<(String, String)>, Vec<SbTeam>);

/// The transition counts of each team and competition of the StatsBomb open data at `path`
/// (`sb2_events.csv.gz`) on the shared possession rule; a held-out competition stops the run.
///
/// The on-ball actions are passes, shots, carries, dribbles, received balls, clearances,
/// interceptions, recoveries and miscontrols; a dead ball is a foul, an offside, an injury
/// stoppage, a substitution, a dropped ball, a half's end, or a restart pass (kick-off,
/// throw-in, free kick, goal kick, corner), which also starts the next possession. A pressure
/// is a Pressure event of the side.
fn statsbomb_teams(path: &Path) -> anyhow::Result<SbTeams> {
    let file = "sb2_events.csv.gz";
    let mut it = lines(path)?;
    let header = cells(&it.next().context("an empty sb2_events.csv.gz")??);
    let col = |n: &str| column(&header, n, file);
    let (c_mid, c_period, c_ts, c_type, c_team, c_sub, c_out, c_cp, c_comp, c_season) = (
        col("mid")?,
        col("period")?,
        col("ts")?,
        col("type")?,
        col("team")?,
        col("sub")?,
        col("out")?,
        col("cp")?,
        col("comp")?,
        col("season")?,
    );
    // (competition, team) -> its index and counts.
    let mut index: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut teams: Vec<SbTeam> = Vec::new();
    let mut keys: Vec<(String, String)> = Vec::new();
    let mut half: Option<(String, String)> = None;
    let mut open: Option<SbPossession> = None;
    let mut spell: Option<(f64, SbPossession)> = None;
    let mut losses: Vec<SbLoss> = Vec::new();
    // Ends a possession: a regain with a shot adds its time.
    let end = |p: Option<SbPossession>, teams: &mut Vec<SbTeam>| {
        if let Some(p) = p.filter(|p| p.regain)
            && let Some(s) = p.shot_at
        {
            teams[p.team].regain_shots += 1.0;
            teams[p.team].regain_shot_s += s - p.start;
        }
    };
    // A dead ball: the possession ends and every loss in progress is counted.
    let dead = |open: &mut Option<SbPossession>,
                spell: &mut Option<(f64, SbPossession)>,
                losses: &mut Vec<SbLoss>,
                teams: &mut Vec<SbTeam>| {
        *spell = None;
        end(open.take(), teams);
        for l in losses.drain(..) {
            l.count(&mut teams[l.team]);
        }
    };
    for line in it {
        let c = cells(&line?);
        if c.len() <= c_season {
            continue;
        }
        let comp = format!("{} {}", c[c_comp], c[c_season]);
        if STATSBOMB_HELD_OUT.contains(&comp.as_str()) {
            bail!("held-out set: {comp} is never read");
        }
        let key = (c[c_mid].clone(), c[c_period].clone());
        if half.as_ref() != Some(&key) {
            dead(&mut open, &mut spell, &mut losses, &mut teams);
            half = Some(key);
        }
        let Some(sec) = sb_seconds(&c[c_ts]) else {
            continue;
        };
        let ty = c[c_type].as_str();
        let team = {
            let k = (comp.clone(), c[c_team].clone());
            *index.entry(k.clone()).or_insert_with(|| {
                teams.push(SbTeam::default());
                keys.push(k);
                teams.len() - 1
            })
        };
        teams[team].matches.insert(c[c_mid].clone());
        // A spell older than the short-regain time becomes the other team's possession: the
        // team that had the ball lost it at the spell's first touch.
        if let Some((start, _)) = spell.filter(|(start, _)| sec - start > REGAIN_S) {
            let (_, p) = spell.take().expect("a spell");
            take_over_sb(&mut open, p, start, &mut losses, &mut teams, end);
        }
        let restart = ty == "Pass"
            && matches!(
                c[c_sub].as_str(),
                "Kick Off" | "Throw-in" | "Free Kick" | "Goal Kick" | "Corner"
            );
        if restart
            || matches!(
                ty,
                "Foul Committed"
                    | "Offside"
                    | "Injury Stoppage"
                    | "Substitution"
                    | "Referee Ball-Drop"
                    | "Half End"
            )
        {
            dead(&mut open, &mut spell, &mut losses, &mut teams);
            if !restart {
                continue;
            }
        }
        if ty == "Pressure" {
            for l in losses.iter_mut().filter(|l| l.team == team) {
                let since = sec - l.at;
                if since < 0.0 {
                    continue;
                }
                if since <= COUNTERPRESS_S {
                    l.pressures += 1.0;
                }
                if since <= COUNTERPRESS_LAST_S {
                    l.last = Some(sec);
                }
            }
            if c[c_cp] == "True" {
                teams[team].flagged += 1.0;
                for l in losses.iter_mut().filter(|l| l.team == team) {
                    if (0.0..=COUNTERPRESS_S).contains(&(sec - l.at)) {
                        l.flagged += 1.0;
                    }
                }
            }
            continue;
        }
        let on_ball = matches!(
            ty,
            "Pass"
                | "Shot"
                | "Carry"
                | "Dribble"
                | "Ball Receipt*"
                | "Clearance"
                | "Interception"
                | "Ball Recovery"
                | "Miscontrol"
        ) && (ty != "Ball Receipt*" || c[c_out].is_empty());
        if !on_ball {
            continue;
        }
        let shot = ty == "Shot";
        let completed = ty == "Pass" && c[c_out].is_empty();
        match open.as_mut() {
            None => {
                open = Some(SbPossession {
                    team,
                    start: sec,
                    regain: false,
                    shot_at: shot.then_some(sec),
                });
            }
            Some(p) if p.team == team => {
                spell = None;
                if shot {
                    p.shot_at.get_or_insert(sec);
                }
            }
            Some(_) => {
                let (start, s) = spell.get_or_insert((
                    sec,
                    SbPossession {
                        team,
                        start: sec,
                        regain: true,
                        shot_at: None,
                    },
                ));
                let start = *start;
                if shot {
                    s.shot_at.get_or_insert(sec);
                }
                if shot || completed {
                    let (_, p) = spell.take().expect("a spell");
                    take_over_sb(&mut open, p, start, &mut losses, &mut teams, end);
                }
            }
        }
    }
    dead(&mut open, &mut spell, &mut losses, &mut teams);
    Ok((keys, teams))
}

/// A goal kick shorter than this, in metres, is short, as the engine's measures read it.
pub const SHORT_GOAL_KICK_M: f64 = 20.0;
/// A goal kick's side kept the ball when its StatsBomb possession lasted this long after the
/// kick, in seconds, or held a shot of the side, as the engine's measures read it.
pub const KEPT_S: f64 = 10.0;
/// StatsBomb positions are yards on a 120 by 80 pitch; the engine's pitch is 105 by 68 m.
const SB_X_M: f64 = 105.0 / 120.0;
const SB_Y_M: f64 = 68.0 / 80.0;

/// The goal kicks of one StatsBomb team in one competition, and of its opponents: how many,
/// their summed length, the short ones and the short ones kept.
#[derive(Debug, Clone, Copy, Default)]
struct SbKicks {
    own: [f64; 4],
    opp: [f64; 4],
}

/// A goal kick whose possession is still open: the team, the possession, the kick's time,
/// its length, the possession's last time so far and whether the team shot in it.
#[derive(Debug, Clone)]
struct SbKick {
    team: usize,
    poss: (String, String, String),
    at: f64,
    length: f64,
    last: f64,
    shot: bool,
}

/// The goal-kick rows and checks of the StatsBomb open data in `dir` (`sb2_events.csv.gz`), all
/// played under the 2019 law that lets a goal kick be taken short inside the area: per team and
/// competition, the mean length of its goal kicks (`build_up`) and of its opponents'
/// (`stop_short_goal_kicks`), marked as national teams like the other StatsBomb rows; and as
/// checks, the opponents' share of short goal kicks ([`SHORT_GOAL_KICK_M`]) and the share of
/// those the kicking side kept ([`KEPT_S`] in StatsBomb's own possession, or a shot in it).
/// None without the file; a held-out competition stops the run.
fn statsbomb_goal_kicks(dir: &Path) -> anyhow::Result<(Vec<Row>, Vec<Check>)> {
    let path = dir.join("sb2_events.csv.gz");
    if !path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }
    let file = "sb2_events.csv.gz";
    let mut it = lines(&path)?;
    let header = cells(&it.next().context("an empty sb2_events.csv.gz")??);
    let col = |n: &str| column(&header, n, file);
    let (c_mid, c_period, c_ts, c_type, c_team, c_poss, c_sub) = (
        col("mid")?,
        col("period")?,
        col("ts")?,
        col("type")?,
        col("team")?,
        col("poss")?,
        col("sub")?,
    );
    let (c_x, c_y, c_ex, c_ey, c_comp, c_season) = (
        col("x")?,
        col("y")?,
        col("ex")?,
        col("ey")?,
        col("comp")?,
        col("season")?,
    );
    let mut index: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut keys: Vec<(String, String)> = Vec::new();
    let mut teams: Vec<SbKicks> = Vec::new();
    let mut matches: BTreeSet<String> = BTreeSet::new();
    // The match in progress, its teams, and its settled kicks (team, length, kept).
    let mut current: Option<String> = None;
    let mut sides: BTreeSet<usize> = BTreeSet::new();
    let mut settled: Vec<(usize, f64, bool)> = Vec::new();
    let mut open: Option<SbKick> = None;
    let settle = |k: SbKick, settled: &mut Vec<(usize, f64, bool)>| {
        settled.push((k.team, k.length, k.shot || k.last - k.at >= KEPT_S));
    };
    // A match ends: each kick counts for its team and for the other team of the match.
    let close = |sides: &BTreeSet<usize>,
                 settled: &mut Vec<(usize, f64, bool)>,
                 teams: &mut Vec<SbKicks>| {
        for (team, length, kept) in settled.drain(..) {
            let short = length < SHORT_GOAL_KICK_M;
            let add = |a: &mut [f64; 4]| {
                a[0] += 1.0;
                a[1] += length;
                if short {
                    a[2] += 1.0;
                    a[3] += f64::from(u8::from(kept));
                }
            };
            add(&mut teams[team].own);
            for &other in sides.iter().filter(|&&s| s != team) {
                add(&mut teams[other].opp);
            }
        }
    };
    for line in it {
        let c = cells(&line?);
        if c.len() <= c_season {
            continue;
        }
        let comp = format!("{} {}", c[c_comp], c[c_season]);
        if STATSBOMB_HELD_OUT.contains(&comp.as_str()) {
            bail!("held-out set: {comp} is never read");
        }
        if current.as_deref() != Some(c[c_mid].as_str()) {
            if let Some(k) = open.take() {
                settle(k, &mut settled);
            }
            close(&sides, &mut settled, &mut teams);
            sides.clear();
            current = Some(c[c_mid].clone());
            matches.insert(c[c_mid].clone());
        }
        if c[c_team].is_empty() {
            continue;
        }
        let team = {
            let k = (comp.clone(), c[c_team].clone());
            *index.entry(k.clone()).or_insert_with(|| {
                teams.push(SbKicks::default());
                keys.push(k);
                teams.len() - 1
            })
        };
        sides.insert(team);
        let Some(sec) = sb_seconds(&c[c_ts]) else {
            continue;
        };
        let poss = (c[c_mid].clone(), c[c_period].clone(), c[c_poss].clone());
        // An event of the kick's possession stretches it; another possession ends it.
        if let Some(k) = open.as_mut() {
            if k.poss == poss {
                k.last = k.last.max(sec);
                k.shot |= c[c_type] == "Shot" && team == k.team;
            } else if let Some(k) = open.take() {
                settle(k, &mut settled);
            }
        }
        if c[c_type] == "Pass" && c[c_sub] == "Goal Kick" {
            let num = |i: usize| c[i].parse::<f64>().ok();
            let (Some(x), Some(y), Some(ex), Some(ey)) = (num(c_x), num(c_y), num(c_ex), num(c_ey))
            else {
                continue;
            };
            let length = ((ex - x) * SB_X_M).hypot((ey - y) * SB_Y_M);
            if let Some(k) = open.take() {
                settle(k, &mut settled);
            }
            open = Some(SbKick {
                team,
                poss,
                at: sec,
                length,
                last: sec,
                shot: false,
            });
        }
    }
    if let Some(k) = open.take() {
        settle(k, &mut settled);
    }
    close(&sides, &mut settled, &mut teams);
    let competitions: Vec<String> = keys
        .iter()
        .map(|(comp, _)| comp.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let per = |f: &dyn Fn(&SbKicks) -> (f64, f64)| -> Vec<f64> {
        teams
            .iter()
            .map(f)
            .filter(|(_, d)| *d > 0.0)
            .map(|(n, d)| n / d)
            .collect()
    };
    let mut rows = Vec::new();
    let mut checks = Vec::new();
    if teams.is_empty() {
        return Ok((rows, checks));
    }
    for (primitive, stat, f) in [
        (
            "build_up",
            "goal_kick_m",
            &(|t: &SbKicks| (t.own[1], t.own[0])) as &dyn Fn(&SbKicks) -> (f64, f64),
        ),
        ("stop_short_goal_kicks", "opp_goal_kick_m", &|t| {
            (t.opp[1], t.opp[0])
        }),
    ] {
        let mut row = spread(primitive, stat, &competitions, matches.len(), &per(f))?;
        row.source = STATSBOMB.into();
        row.national_teams = true;
        rows.push(row);
    }
    checks.extend(Check::of(
        "opp_short_goal_kick_share",
        STATSBOMB,
        &competitions,
        &per(&|t| (t.opp[2], t.opp[0])),
    ));
    checks.extend(Check::of(
        "opp_short_goal_kick_kept_share",
        STATSBOMB,
        &competitions,
        &per(&|t| (t.opp[3], t.opp[2])),
    ));
    Ok((rows, checks))
}

/// The rows and checks of the StatsBomb open data in `dir` ([`statsbomb_teams`]): per team
/// and competition, the regain-to-shot time, the counter-press pressures per loss and regains
/// within [`COUNTERPRESS_S`], and the time to the last pressure within
/// [`COUNTERPRESS_LAST_S`]. The rows are marked as national teams, with a club-only check from
/// the Bundesliga, and a check of StatsBomb's own counterpress flag. None without the file.
fn statsbomb_transitions(dir: &Path) -> anyhow::Result<(Vec<Row>, Vec<Check>)> {
    let path = dir.join("sb2_events.csv.gz");
    if !path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }
    let (keys, teams) = statsbomb_teams(&path)?;
    let national: Vec<String> = keys
        .iter()
        .map(|(comp, _)| comp.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let matches: BTreeSet<&String> = teams.iter().flat_map(|t| &t.matches).collect();
    let per = |f: &dyn Fn(&SbTeam) -> (f64, f64), clubs: bool| -> Vec<f64> {
        teams
            .iter()
            .zip(&keys)
            .filter(|(_, (comp, _))| !clubs || comp == STATSBOMB_CLUBS)
            .map(|(t, _)| f(t))
            .filter(|(_, d)| *d > 0.0)
            .map(|(n, d)| n / d)
            .collect()
    };
    type Stat<'a> = (&'a str, &'a str, &'a dyn Fn(&SbTeam) -> (f64, f64));
    let stats: [Stat<'_>; 4] = [
        ("on_winning_the_ball", "regain_to_shot_s", &|t| {
            (t.regain_shot_s, t.regain_shots)
        }),
        (
            "on_losing_the_ball",
            "counterpress_pressures_per_loss",
            &|t| (t.pressures, t.losses),
        ),
        ("on_losing_the_ball", "counterpress_regain_share", &|t| {
            (t.regains_in_window, t.losses)
        }),
        ("counter_press_length", "last_counterpress_s", &|t| {
            (t.last_pressure_s, t.last_pressures)
        }),
    ];
    let mut rows = Vec::new();
    let mut checks = Vec::new();
    if teams.is_empty() {
        return Ok((rows, checks));
    }
    let clubs = [STATSBOMB_CLUBS.to_string()];
    for (primitive, stat, f) in stats {
        let mut row = spread(primitive, stat, &national, matches.len(), &per(f, false))?;
        row.source = STATSBOMB.into();
        row.national_teams = true;
        rows.push(row);
        checks.extend(Check::of(stat, STATSBOMB, &clubs, &per(f, true)));
    }
    // StatsBomb's counterpress flag: the share of each team's flagged pressures that fall
    // within the counter-press time of an open-play loss.
    checks.extend(Check::of(
        "counterpress_flag_within_window_share",
        STATSBOMB,
        &national,
        &per(&|t| (t.flagged_in_window, t.flagged), false),
    ));
    Ok((rows, checks))
}

/// The other team's spell `p`, first touch at `start`, becomes its possession: the open one
/// ends (`end`), its team lost the ball at `start`, and a loss of `p`'s team in progress ends
/// as a regain when it is within the counter-press time.
fn take_over_sb(
    open: &mut Option<SbPossession>,
    p: SbPossession,
    start: f64,
    losses: &mut Vec<SbLoss>,
    teams: &mut Vec<SbTeam>,
    end: impl Fn(Option<SbPossession>, &mut Vec<SbTeam>),
) {
    let lost = open.take();
    end(lost, teams);
    // The regaining team's own losses end now.
    let mut k = 0;
    while k < losses.len() {
        if losses[k].team == p.team {
            let mut l = losses.remove(k);
            l.regained = start - l.at <= COUNTERPRESS_S;
            l.count(&mut teams[l.team]);
        } else {
            k += 1;
        }
    }
    if let Some(lost) = lost {
        losses.push(SbLoss {
            team: lost.team,
            at: start,
            pressures: 0.0,
            flagged: 0.0,
            last: None,
            regained: false,
        });
    }
    *open = Some(p);
}

/// The SkillCorner check of the counter-press: per club, its `counter_press` engagements per
/// match (a thin sample of 20 A-League matches). None without the file.
fn skillcorner_counter_press(dir: &Path) -> anyhow::Result<Option<Check>> {
    let file = "sc_dynamic_events.csv.gz";
    let path = dir.join(file);
    if !path.exists() {
        return Ok(None);
    }
    let mut it = lines(&path)?;
    let header = cells(&it.next().context("an empty sc_dynamic_events.csv.gz")??);
    let col = |n: &str| column(&header, n, file);
    let (c_match, c_team, c_type, c_sub) = (
        col("match_id")?,
        col("team_shortname")?,
        col("event_type")?,
        col("event_subtype")?,
    );
    let mut teams: BTreeMap<String, (f64, BTreeSet<String>)> = BTreeMap::new();
    for line in it {
        let c = cells(&line?);
        if c.len() <= c_match.max(c_team).max(c_type).max(c_sub) {
            continue;
        }
        let t = teams.entry(c[c_team].clone()).or_default();
        t.1.insert(c[c_match].clone());
        if c[c_type] == "on_ball_engagement" && c[c_sub] == "counter_press" {
            t.0 += 1.0;
        }
    }
    let values: Vec<f64> = teams
        .values()
        .filter(|(_, m)| !m.is_empty())
        .map(|(n, m)| n / m.len() as f64)
        .collect();
    Ok(Check::of(
        "counter_press_engagements_per_match",
        SKILLCORNER,
        &[SKILLCORNER_LEAGUE.to_string()],
        &values,
    ))
}

/// The DFL out-of-possession shapes, one line per team, half and phase (`idsse_shape.csv`, by
/// `mine_positions.py`: the width is the distance across between the widest outfield players).
pub const DFL_SHAPE: &str = "idsse_shape.csv";
/// The source of the defensive-width row: the DFL open tracking data.
pub const DFL: &str = "dfl_open";

/// The thin-sample row of defensive width: per club, the frame-weighted mean width of its
/// outfield players out of possession. None without the file.
fn dfl_width_row(dir: &Path) -> anyhow::Result<Option<Row>> {
    let path = dir.join(DFL_SHAPE);
    if !path.exists() {
        return Ok(None);
    }
    let mut it = lines(&path)?;
    let header = cells(&it.next().context("an empty idsse_shape.csv")??);
    let col = |n: &str| column(&header, n, DFL_SHAPE);
    let (c_team, c_phase, c_frames, c_width, c_match, c_comp) = (
        col("team")?,
        col("phase")?,
        col("frames")?,
        col("width")?,
        col("mid")?,
        col("comp")?,
    );
    let mut teams: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut matches = BTreeSet::new();
    let mut comps = BTreeSet::new();
    for line in it {
        let c = cells(&line?);
        if c.len() <= c_comp.max(c_width) || c[c_phase] != "out" {
            continue;
        }
        let (Ok(frames), Ok(width)) = (c[c_frames].parse::<f64>(), c[c_width].parse::<f64>())
        else {
            continue;
        };
        let t = teams.entry(c[c_team].clone()).or_default();
        t.0 += frames * width;
        t.1 += frames;
        matches.insert(c[c_match].clone());
        comps.insert(c[c_comp].clone());
    }
    let values: Vec<f64> = teams
        .values()
        .filter(|(_, n)| *n > 0.0)
        .map(|(s, n)| s / n)
        .collect();
    let competitions: Vec<String> = comps.into_iter().collect();
    Ok(Some(thin(
        "defensive_width",
        "oop_width_m",
        DFL,
        &competitions,
        matches.len(),
        &values,
    )?))
}

/// The SkillCorner reference rows of two press triggers: per club, its on-ball engagements on
/// an opponent's possession that began with a back pass received, per such possession; and
/// the same for a possession of a full-back or one that began in a wide channel. None without
/// the file.
fn skillcorner_trigger_rows(dir: &Path) -> anyhow::Result<Vec<Row>> {
    let file = "sc_dynamic_events.csv.gz";
    let path = dir.join(file);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut it = lines(&path)?;
    let header = cells(&it.next().context("an empty sc_dynamic_events.csv.gz")??);
    // A file without the possession and engagement columns gives no trigger row.
    let col = |n: &str| header.iter().position(|h| h == n);
    let (
        Some(c_id),
        Some(c_match),
        Some(c_team),
        Some(c_type),
        Some(c_pos),
        Some(c_channel),
        Some(c_dir),
        Some(c_assoc),
    ) = (
        col("event_id"),
        col("match_id"),
        col("team_shortname"),
        col("event_type"),
        col("player_position"),
        col("channel_start"),
        col("pass_direction_received"),
        col("associated_player_possession_event_id"),
    )
    else {
        return Ok(Vec::new());
    };
    let last = [
        c_id, c_match, c_team, c_type, c_pos, c_channel, c_dir, c_assoc,
    ]
    .into_iter()
    .max()
    .unwrap_or(0);
    // Each possession's match, team and kinds; each engagement's team and possession.
    // Event ids repeat across matches: each is keyed with its match.
    let mut possessions: BTreeMap<(String, String), (String, String, bool, bool)> = BTreeMap::new();
    let mut engagements: Vec<(String, (String, String))> = Vec::new();
    let mut match_teams: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for line in it {
        let c = cells(&line?);
        if c.len() <= last {
            continue;
        }
        match_teams
            .entry(c[c_match].clone())
            .or_default()
            .insert(c[c_team].clone());
        match c[c_type].as_str() {
            "player_possession" => {
                let back = c[c_dir] == "backward";
                let wide = SC_FULL_BACKS.contains(&c[c_pos].as_str())
                    || matches!(c[c_channel].as_str(), "wide_left" | "wide_right");
                possessions.insert(
                    (c[c_match].clone(), c[c_id].clone()),
                    (c[c_match].clone(), c[c_team].clone(), back, wide),
                );
            }
            "on_ball_engagement" => {
                engagements.push((c[c_team].clone(), (c[c_match].clone(), c[c_assoc].clone())))
            }
            _ => {}
        }
    }
    // Per pressing club: [back-pass possessions faced, engagements on them, wide possessions
    // faced, engagements on them].
    let mut clubs: BTreeMap<String, [f64; 4]> = BTreeMap::new();
    for (game, team, back, wide) in possessions.values() {
        let Some(pair) = match_teams.get(game) else {
            continue;
        };
        for other in pair.iter().filter(|t| *t != team) {
            let k = clubs.entry(other.clone()).or_default();
            k[0] += f64::from(u8::from(*back));
            k[2] += f64::from(u8::from(*wide));
        }
    }
    for (team, assoc) in &engagements {
        let Some((_, owner, back, wide)) = possessions.get(assoc) else {
            continue;
        };
        if owner == team {
            continue;
        }
        let k = clubs.entry(team.clone()).or_default();
        k[1] += f64::from(u8::from(*back));
        k[3] += f64::from(u8::from(*wide));
    }
    let per = |n: usize, d: usize| -> Vec<f64> {
        clubs
            .values()
            .filter(|k| k[d] > 0.0)
            .map(|k| k[n] / k[d])
            .collect()
    };
    let competitions = [SKILLCORNER_LEAGUE.to_string()];
    let matches = match_teams.len();
    let mut rows = Vec::new();
    for (primitive, stat, values) in [
        ("press_on_back_pass", "pressures_after_back_pass", per(1, 0)),
        ("press_on_wide", "pressures_after_wide", per(3, 2)),
    ] {
        if values.len() >= 2 {
            rows.push(thin(
                primitive,
                stat,
                SKILLCORNER,
                &competitions,
                matches,
                &values,
            )?);
        }
    }
    Ok(rows)
}

/// The table of the fit data in `dir` for `competitions`.
pub fn build(dir: &Path, competitions: &[String]) -> anyhow::Result<Table> {
    for c in competitions {
        refuse_held_out(c)?;
    }
    let (seasons, matches) = wyscout_seasons(dir, competitions)?;
    let ratio = |f: &dyn Fn(&Season) -> (f64, f64)| -> Vec<f64> {
        seasons
            .values()
            .map(f)
            .filter(|(_, d)| *d > 0.0)
            .map(|(n, d)| n / d)
            .collect()
    };
    let entries = |s: &Season, share: &dyn Fn(&[f64; 3]) -> f64| {
        (share(&s.entries), s.entries.iter().sum::<f64>())
    };
    let stats: [(&str, &str, Vec<f64>); 24] = [
        (
            "tempo",
            "passes_per_possession",
            ratio(&|s| (s.passes as f64, s.possessions as f64)),
        ),
        (
            "tempo",
            "recycle_share",
            ratio(&|s| (s.recycle as f64, s.open_passes as f64)),
        ),
        (
            "mentality",
            "turnovers_per_possession",
            ratio(&|s| (s.turnovers as f64, s.possessions as f64)),
        ),
        (
            "mentality",
            "shot_share",
            ratio(&|s| (s.shot_possessions as f64, s.possessions as f64)),
        ),
        ("pressing", "ppda", ratio(&|s| (s.opp_build, s.def_actions))),
        (
            "passing_directness",
            "forward_pass_share",
            ratio(&|s| (s.op_forward, s.op_passes)),
        ),
        (
            "passing_directness",
            "long_ball_share",
            ratio(&|s| (s.op_long, s.op_passes)),
        ),
        (
            "build_up",
            "own_third_long_share",
            ratio(&|s| (s.own_third_long, s.own_third)),
        ),
        (
            "build_up",
            "keeper_long_share",
            ratio(&|s| (s.keeper_long, s.keeper_actions)),
        ),
        (
            "width",
            "wide_pass_share",
            ratio(&|s| (s.wide_end, s.upfield_end)),
        ),
        (
            "attacking_channel",
            "entries_left_share",
            ratio(&|s| entries(s, &|e| e[0])),
        ),
        (
            "attacking_channel",
            "entries_right_share",
            ratio(&|s| entries(s, &|e| e[2])),
        ),
        (
            "attacking_channel",
            "entries_centre_share",
            ratio(&|s| entries(s, &|e| e[1])),
        ),
        (
            "attacking_channel",
            "entries_wide_share",
            ratio(&|s| entries(s, &|e| e[0] + e[2])),
        ),
        (
            "attacking_channel",
            "entries_centre_left_share",
            ratio(&|s| entries(s, &|e| e[0] + e[1])),
        ),
        (
            "attacking_channel",
            "entries_centre_right_share",
            ratio(&|s| entries(s, &|e| e[1] + e[2])),
        ),
        (
            "final_third",
            "outside_box_shot_share",
            ratio(&|s| (s.outside_shots, s.shots)),
        ),
        (
            "final_third",
            "shot_distance_m",
            ratio(&|s| (s.shot_m, s.shots)),
        ),
        (
            "dribbling",
            "takeons_per_90",
            ratio(&|s| (s.takeons, s.team_matches)),
        ),
        (
            "crossing",
            "crosses_per_final_third_possession",
            ratio(&|s| (s.crosses, s.final_third_possessions)),
        ),
        (
            "cross_type",
            "high_cross_share",
            ratio(&|s| (s.high_crosses, s.crosses)),
        ),
        (
            "cross_type",
            "low_cross_share",
            ratio(&|s| (s.crosses - s.high_crosses, s.crosses)),
        ),
        (
            "cross_type",
            "cross_origin_m",
            ratio(&|s| (s.cross_origin_m, s.crosses)),
        ),
        (
            "on_winning_the_ball",
            "counter_attacks_per_match",
            ratio(&|s| (s.fast_breaks, s.team_matches)),
        ),
    ];
    let mut rows = Vec::new();
    for (primitive, stat, values) in stats {
        rows.push(spread(primitive, stat, competitions, matches, &values)?);
    }
    // The out-of-possession rows: each needs the columns and events it reads, so a file
    // without them gives none of its rows.
    let defending: [(&str, &str, Vec<f64>); 6] = [
        (
            "line_of_engagement",
            "def_action_height_m",
            ratio(&|s| (s.def_all_m, s.def_all)),
        ),
        (
            "line_height",
            "defender_action_height_m",
            ratio(&|s| (s.def_df_m, s.def_df)),
        ),
        (
            "force_the_opponent",
            "opp_entries_wide_share",
            ratio(&|s| {
                (
                    s.opp_entries[0] + s.opp_entries[2],
                    s.opp_entries.iter().sum::<f64>(),
                )
            }),
        ),
        (
            "tackling",
            "fouls_per_match",
            ratio(&|s| (s.fouls, s.card_matches)),
        ),
        (
            "tackling",
            "yellows_per_match",
            ratio(&|s| (s.yellows, s.card_matches)),
        ),
        (
            "offside_trap",
            "offsides_caught_per_match",
            ratio(&|s| (s.offsides_caught, s.opp_matches)),
        ),
    ];
    for (primitive, stat, values) in defending {
        if values.len() >= 5 {
            rows.push(spread(primitive, stat, competitions, matches, &values)?);
        }
    }
    // The goal-kick lengths of the fit leagues predate the 2019 law that lets a goal kick be
    // played inside the area, so they are checks beside the StatsBomb rows, not targets.
    let mut kick_checks: Vec<Check> = Check::of(
        "goal_kick_m",
        WYSCOUT,
        competitions,
        &ratio(&|s| (s.goal_kick_m, s.goal_kicks)),
    )
    .into_iter()
    .chain(Check::of(
        "opp_goal_kick_m",
        WYSCOUT,
        competitions,
        &ratio(&|s| (s.opp_goal_kick_m, s.opp_goal_kicks)),
    ))
    .collect();
    let (kick_rows, sb_kick_checks) = statsbomb_goal_kicks(dir)?;
    rows.extend(kick_rows);
    kick_checks.extend(sb_kick_checks);
    rows.extend(skillcorner_rows(dir)?);
    rows.extend(skillcorner_trigger_rows(dir)?);
    rows.extend(dfl_width_row(dir)?);
    let (tracked, tracked_checks) = tracking_rows(dir)?;
    rows.extend(tracked);
    let mut checks: Vec<Check> = statsbomb_check(dir)?.into_iter().collect();
    checks.extend(tracked_checks);
    checks.extend(kick_checks);
    // The counter-attack tag against the fast-break definition, and the Wyscout
    // regain-to-shot time beside the StatsBomb row.
    let fast = ratio(&|s| (s.fast_breaks, s.team_matches));
    let tagged = ratio(&|s| (s.tagged_counters, s.team_matches));
    if let Some(mut c) = Check::of(
        "tagged_counter_attacks_per_match",
        WYSCOUT,
        competitions,
        &tagged,
    ) {
        c.rank_correlation = Some(round6(spearman(&tagged, &fast)));
        c.against = Some("on_winning_the_ball counter_attacks_per_match".into());
        checks.push(c);
    }
    checks.extend(Check::of(
        "regain_to_shot_s",
        WYSCOUT,
        competitions,
        &ratio(&|s| (s.regain_shot_s, s.regain_shots)),
    ));
    let (sb_rows, sb_checks) = statsbomb_transitions(dir)?;
    rows.extend(sb_rows);
    checks.extend(sb_checks);
    checks.extend(skillcorner_counter_press(dir)?);
    Ok(Table {
        schema_version: SIGNATURES_VERSION,
        held_out: engine::data::primitives::HELD_OUT
            .iter()
            .map(|(s, c)| format!("{s}:{c}"))
            .collect(),
        rows,
        checks,
        substitutions: substitutions::build(dir, competitions)?,
    })
}

pub fn run(content_dir: Option<&Path>, opts: &SignaturesOpts) -> anyhow::Result<i32> {
    let dir = ContentDir::resolve(content_dir)?;
    let folder = crate::report::real::resolve(opts.real_data.as_deref(), dir.root());
    let Some(real) = folder.path.as_ref() else {
        bail!("no real-data folder: give --real-data DIR");
    };
    let competitions: Vec<String> = match &opts.competitions {
        Some(list) => list
            .split(',')
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .collect(),
        None => FIT_LEAGUES.iter().map(|c| c.to_string()).collect(),
    };
    let table = build(real, &competitions)?;
    let mut text = serde_json::to_string_pretty(&table)?;
    text.push('\n');
    let out: PathBuf = opts
        .out
        .clone()
        .unwrap_or_else(|| dir.root().join("signatures.json"));
    std::fs::write(&out, text).with_context(|| format!("cannot write {}", out.display()))?;
    let summary = serde_json::json!({
        "kind": "signatures",
        "out": out.display().to_string(),
        "rows": table.rows.len(),
        "teams": table.rows.first().map_or(0, |r| r.teams),
        "matches": table.rows.first().map_or(0, |r| r.matches),
    });
    println!("{summary}");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quoted_cell_keeps_its_comma() {
        assert_eq!(cells("a,\"b,c\",d\r"), ["a", "b,c", "d"]);
        assert_eq!(cells("\"x\"\"y\",,z"), ["x\"y", "", "z"]);
    }

    /// Three possessions of one half: team 1 recycles once and plays forward, then loses the
    /// ball to a completed pass of team 2 (a turnover); team 2 shoots, and the ball goes dead;
    /// team 1 starts again, team 2 touches the ball, and team 1 acts again within the short
    /// regain window, so its possession goes on. The half ends it.
    #[test]
    fn possessions_merge_a_short_regain_and_count_turnovers_and_recycling() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("sm-signatures-unit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("wy2_matches.csv"), "comp,match\nEngland,m1\n").unwrap();
        std::fs::write(
            dir.join("wy2_team_matches.csv"),
            "match,team,opp_build_vs,def_act\nm1,1,10,5\nm1,2,20,5\n",
        )
        .unwrap();
        let rows = [
            "1,0.0,1,Pass,Simple pass,30,50,25,50,1801",
            "2,2.0,1,Pass,Simple pass,25,50,45,50,1801",
            "3,4.0,2,Pass,Simple pass,60,50,70,50,1801",
            "4,6.0,2,Shot,Shot,80,50,100,50,",
            "5,7.0,2,Interruption,Ball out of the field,100,50,,,",
            "6,20.0,1,Pass,Simple pass,50,50,52,50,1801",
            "7,21.0,2,Others on the ball,Touch,48,50,48,50,",
            "8,22.0,1,Pass,Simple pass,52,50,53,50,1801",
        ];
        let mut events =
            String::from("comp,match,id,period,sec,team,player,role,ev,sub,x0,y0,x1,y1,tags\n");
        for r in rows {
            let c: Vec<&str> = r.splitn(4, ',').collect();
            events.push_str(&format!(
                "England,m1,{},1H,{},{},7,MD,{}\n",
                c[0], c[1], c[2], c[3]
            ));
        }
        let file = File::create(dir.join("wy2_events.csv.gz")).unwrap();
        let mut gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        gz.write_all(events.as_bytes()).unwrap();
        gz.finish().unwrap();

        let (seasons, matches) = wyscout_seasons(&dir, &["England".to_string()]).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(matches, 1);
        let one = seasons[&1];
        assert_eq!(one.possessions, 2);
        assert_eq!((one.passes, one.open_passes), (4, 4));
        assert_eq!(
            one.recycle, 3,
            "one before the forward pass, two in the second"
        );
        assert_eq!((one.turnovers, one.shot_possessions), (1, 0));
        let two = seasons[&2];
        assert_eq!(two.possessions, 1);
        assert_eq!((two.passes, two.shot_possessions, two.turnovers), (1, 1, 0));
    }

    /// Ten teams 1 to 10: the edges at 20, 40, 60, 80 percent by linear interpolation, and
    /// each quintile's median of the two teams inside it.
    #[test]
    fn the_quintiles_of_ten_teams() {
        let v: Vec<f64> = (1..=10).map(f64::from).collect();
        let row = spread("p", "ppda", &[], 1, &v).unwrap();
        assert_eq!(row.quintile_edges, [2.8, 4.6, 6.4, 8.2]);
        assert_eq!(row.medians, [1.5, 3.5, 5.5, 7.5, 9.5]);
        assert_eq!(row.quintile_ranges[0], [1.0, 2.8]);
        assert_eq!(row.quintile_ranges[4], [8.2, 10.0]);
        assert!(spread("p", "ppda", &[], 1, &v[..4]).is_err());
    }

    /// StatsBomb goal kicks of six teams in three matches: A, C and E kick short (16 yd, 14 m),
    /// B, D and F long (60 yd, 52.5 m). A keeps the ball 12 s in its possession, C loses it to
    /// D at 3 s, E shoots at 4 s: two of the three short kicks are kept. The rows hold each
    /// team's and its opponents' mean length; the checks the opponents' short and kept shares.
    #[test]
    fn the_goal_kick_rows_read_the_post_2019_lengths_and_keeping() {
        let dir = folder("goal-kicks");
        let mut text = String::from("mid,period,ts,type,team,poss,sub,x,y,ex,ey,comp,season\n");
        let mut add = |mid: &str, ts: &str, ty: &str, team: &str, poss: u32, sub: &str, ex: f64| {
            text.push_str(&format!(
                "{mid},1,00:00:{ts},{ty},{team},{poss},{sub},6,40,{ex},40,UEFA Euro,2024\n"
            ));
        };
        for (mid, short, long, keep) in [("1", "A", "B", 0), ("2", "C", "D", 1), ("3", "E", "F", 2)]
        {
            add(mid, "00.000", "Pass", short, 1, "Goal Kick", 22.0);
            match keep {
                0 => add(mid, "12.000", "Pass", short, 1, "", 30.0),
                1 => add(mid, "03.000", "Pass", long, 2, "", 30.0),
                _ => add(mid, "04.000", "Shot", short, 1, "", 30.0),
            }
            add(mid, "20.000", "Pass", long, 3, "Goal Kick", 66.0);
            add(mid, "25.000", "Pass", short, 4, "", 30.0);
        }
        gzip(&dir.join("sb2_events.csv.gz"), &text);
        let (rows, checks) = statsbomb_goal_kicks(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(rows.len(), 2);
        for (row, primitive) in rows.iter().zip(["build_up", "stop_short_goal_kicks"]) {
            assert_eq!(row.primitive, primitive);
            assert_eq!(row.source, STATSBOMB);
            assert!(row.national_teams);
            assert_eq!((row.teams, row.matches), (6, 3));
            assert_eq!((row.min, row.max), (14.0, 52.5), "{row:?}");
        }
        let check = |stat: &str| checks.iter().find(|c| c.stat == stat).unwrap();
        let short = check("opp_short_goal_kick_share");
        assert_eq!((short.min, short.median, short.max), (0.0, 0.5, 1.0));
        let kept = check("opp_short_goal_kick_kept_share");
        assert_eq!((kept.teams, kept.min, kept.median), (3, 0.0, 1.0));
    }

    /// Writes `text` gzipped to `path`.
    fn gzip(path: &Path, text: &str) {
        use std::io::Write;
        let file = File::create(path).unwrap();
        let mut gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        gz.write_all(text.as_bytes()).unwrap();
        gz.finish().unwrap();
    }

    /// A folder for one test's files.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sm-signatures-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Team 2 wins the ball 10.5 m inside its own half (a tagged counter-attack), reaches the
    /// final third 3 s later and shoots 6 s after the regain: a fast break with a regain shot.
    /// After a dead ball it wins the ball again 10.5 m into the other half: a regain, but no
    /// fast break. Team 1 starts both of its possessions after a dead ball: no regain.
    #[test]
    fn a_fast_break_is_a_regain_from_the_start_zone_that_reaches_the_final_third_in_time() {
        let dir = folder("fast-break");
        std::fs::write(dir.join("wy2_matches.csv"), "comp,match\nEngland,m1\n").unwrap();
        std::fs::write(
            dir.join("wy2_team_matches.csv"),
            "match,team,opp_build_vs,def_act\nm1,1,10,5\nm1,2,20,5\n",
        )
        .unwrap();
        let rows = [
            "1,0.0,1,Pass,Simple pass,30,50,45,50,1801",
            "2,2.0,2,Pass,Simple pass,40,50,55,50,1801 1901",
            "3,5.0,2,Pass,Simple pass,70,50,80,50,1801",
            "4,8.0,2,Shot,Shot,85,50,100,50,",
            "5,9.0,2,Interruption,Ball out of the field,100,50,,,",
            "6,20.0,1,Pass,Simple pass,50,50,52,50,1801",
            "7,21.0,2,Pass,Simple pass,60,50,62,50,1801",
            "8,40.0,2,Pass,Simple pass,80,50,90,50,1801",
        ];
        let mut events =
            String::from("comp,match,id,period,sec,team,player,role,ev,sub,x0,y0,x1,y1,tags\n");
        for r in rows {
            let c: Vec<&str> = r.splitn(4, ',').collect();
            events.push_str(&format!(
                "England,m1,{},1H,{},{},7,MD,{}\n",
                c[0], c[1], c[2], c[3]
            ));
        }
        gzip(&dir.join("wy2_events.csv.gz"), &events);
        let (seasons, _) = wyscout_seasons(&dir, &["England".to_string()]).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let two = seasons[&2];
        assert_eq!((two.regains, two.fast_breaks), (2.0, 1.0));
        assert_eq!((two.regain_shots, two.regain_shot_s), (1.0, 6.0));
        assert_eq!(two.tagged_counters, 1.0);
        assert_eq!(seasons[&1].regains, 0.0);
    }

    /// StatsBomb: A loses the ball at 4 s, presses twice (one flagged as a counterpress) and
    /// wins it back at 8 s, then shoots 4 s later; B, having lost it at 8 s, never presses
    /// before the goal kick. A held-out tournament stops the run.
    #[test]
    fn the_counter_press_counts_follow_the_shared_possession_rule() {
        let dir = folder("counter-press");
        let head = "mid,period,ts,type,team,sub,out,cp,comp,season\n";
        let rows = [
            "00:00:00.000,Pass,A,Kick Off,,False",
            "00:00:02.000,Pass,A,,,False",
            "00:00:04.000,Ball Receipt*,B,,,False",
            "00:00:04.500,Pass,B,,,False",
            "00:00:05.000,Pressure,A,,,True",
            "00:00:07.000,Pressure,A,,,False",
            "00:00:08.000,Ball Recovery,A,,,False",
            "00:00:08.500,Pass,A,,,False",
            "00:00:12.000,Shot,A,,,False",
            "00:00:13.000,Pass,B,Goal Kick,,False",
        ];
        let mut text = String::from(head);
        for r in rows {
            let c: Vec<&str> = r.splitn(2, ',').collect();
            text.push_str(&format!("1,1,{},{},1. Bundesliga,2023/2024\n", c[0], c[1]));
        }
        let path = dir.join("sb2_events.csv.gz");
        gzip(&path, &text);
        let (keys, teams) = statsbomb_teams(&path).unwrap();
        let of = |name: &str| &teams[keys.iter().position(|(_, t)| t == name).unwrap()];
        let a = of("A");
        assert_eq!(
            (a.losses, a.pressures, a.regains_in_window),
            (1.0, 2.0, 1.0)
        );
        assert_eq!((a.last_pressures, a.last_pressure_s), (1.0, 3.0));
        assert_eq!((a.regain_shots, a.regain_shot_s), (1.0, 4.0));
        assert_eq!((a.flagged, a.flagged_in_window), (1.0, 1.0));
        let b = of("B");
        assert_eq!((b.losses, b.pressures, b.last_pressures), (1.0, 0.0, 0.0));
        text.push_str("2,1,00:00:01.000,Pass,C,,,False,UEFA Euro,2016\n");
        gzip(&path, &text);
        let err = statsbomb_teams(&path).unwrap_err().to_string();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(err.contains("held-out set: UEFA Euro 2016"), "{err}");
    }
}
