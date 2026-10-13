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
//! The output is a pure function of the input files: keys in a fixed order, values rounded to
//! 6 decimals, and no time stamp, so two runs write the same bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use engine::ContentDir;
use engine::data::primitives::held_out;
use engine::decision::{FORWARD_M, REGAIN_S};
use serde::Serialize;

use crate::cli::SignaturesOpts;

/// The source name of the Wyscout club data.
pub const WYSCOUT: &str = "wyscout_2017_18";
/// The source name of the StatsBomb open data.
pub const STATSBOMB: &str = "statsbomb_open";
/// The four leagues the fit reads by default.
pub const FIT_LEAGUES: [&str; 4] = ["England", "France", "Italy", "Spain"];
/// Wyscout positions are percentages of a 105 m pitch along its length.
const PITCH_M: f64 = 105.0;

/// The table's format version. Version 2 adds the in-possession rows and the thin-sample rows
/// of the SkillCorner runs.
pub const SIGNATURES_VERSION: u32 = 2;
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
/// The SkillCorner positions of the forwards (the attack line) and of the full-backs.
pub const SC_FORWARDS: [&str; 5] = ["CF", "LF", "RF", "LW", "RW"];
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
}

/// The table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Table {
    pub schema_version: u32,
    /// The held-out sets the table never read, as `source:competition`.
    pub held_out: Vec<String>,
    pub rows: Vec<Row>,
    pub checks: Vec<Check>,
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

/// The value at share `p` of the sorted values `v`, linear between the two nearest ranks.
fn quantile(v: &[f64], p: f64) -> f64 {
    let at = p * (v.len() - 1) as f64;
    let i = at.floor() as usize;
    let j = (i + 1).min(v.len() - 1);
    v[i] + (v[j] - v[i]) * (at - i as f64)
}

fn round6(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
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
    let edges: [f64; 4] = std::array::from_fn(|i| quantile(&v, (i + 1) as f64 / 5.0));
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
            quantile(&groups[q], 0.5)
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
}

impl Possession {
    fn new(team: u64) -> Self {
        Self {
            team,
            passes: 0,
            open_passes: 0,
            recycle: 0,
            forward: false,
            shot: false,
            final_third: false,
        }
    }

    /// An on-ball action: a shot, a restart kick, or an open-play pass gaining `gain` metres;
    /// `final_third` when it starts in the final third.
    fn act(&mut self, shot: bool, pass: bool, restart: bool, gain: f64, final_third: bool) {
        self.final_third |= final_third;
        if shot {
            self.shot = true;
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
fn count_event(s: &mut Season, ev: &str, sub: &str, role: &str, at: [Option<f64>; 4], tags: &str) {
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
        s.team_matches += 1.0;
    }

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
        match open.as_mut() {
            None => {
                let mut p = Possession::new(team);
                p.act(shot, pass, restart, gain, final_third);
                open = Some(p);
            }
            Some(p) if p.team == team => {
                // The team with the ball acts again: a short regain.
                spell = None;
                p.act(shot, pass, restart, gain, final_third);
            }
            Some(_) => {
                let s = spell.get_or_insert(Spell {
                    start: sec,
                    poss: Possession::new(team),
                });
                s.poss.act(shot, pass, restart, gain, final_third);
                let completed = pass && c[c_tags].split([' ', ';']).any(|t| t == "1801");
                if shot || completed {
                    take_over(&mut open, &mut spell, &mut seasons);
                }
            }
        }
    }
    close(open.take(), End::Other, &mut seasons);
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
    let mut values: Vec<f64> = teams
        .values()
        .filter(|(_, p)| !p.is_empty())
        .map(|(n, p)| *n as f64 / p.len() as f64)
        .collect();
    if values.is_empty() {
        return Ok(None);
    }
    values.sort_by(f64::total_cmp);
    Ok(Some(Check {
        stat: "passes_per_possession".into(),
        source: STATSBOMB.into(),
        competitions: competitions.into_iter().collect(),
        teams: values.len(),
        median: round6(quantile(&values, 0.5)),
        min: round6(values[0]),
        max: round6(values[values.len() - 1]),
    }))
}

/// One SkillCorner team's runs: its forwards' off-ball runs and the runs in behind among them,
/// its full-backs' overlaps and underlaps, and its matches.
#[derive(Debug, Clone, Default)]
struct TeamRuns {
    forward_runs: f64,
    behind: f64,
    overlaps: f64,
    underlaps: f64,
    matches: BTreeSet<String>,
}

/// The thin-sample rows of the SkillCorner open data in `dir` (`sc_dynamic_events.csv.gz`):
/// per team, the share of its forwards' off-ball runs that go in behind, and its full-backs'
/// overlaps, underlaps and both per match. None without the file.
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
        if SC_FORWARDS.contains(&pos) {
            t.forward_runs += 1.0;
            if sub == "behind" {
                t.behind += 1.0;
            }
        }
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
    let behind: Vec<f64> = teams
        .values()
        .filter(|t| t.forward_runs > 0.0)
        .map(|t| t.behind / t.forward_runs)
        .collect();
    let n = matches.len();
    Ok(vec![
        thin(
            "forward_runs",
            "runs_in_behind_share",
            SKILLCORNER,
            &competitions,
            n,
            &behind,
        )?,
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
            "goal_kick_m",
            ratio(&|s| (s.goal_kick_m, s.goal_kicks)),
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
    ];
    let mut rows = Vec::new();
    for (primitive, stat, values) in stats {
        rows.push(spread(primitive, stat, competitions, matches, &values)?);
    }
    rows.extend(skillcorner_rows(dir)?);
    let checks = statsbomb_check(dir)?.into_iter().collect();
    Ok(Table {
        schema_version: SIGNATURES_VERSION,
        held_out: engine::data::primitives::HELD_OUT
            .iter()
            .map(|(s, c)| format!("{s}:{c}"))
            .collect(),
        rows,
        checks,
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
}
