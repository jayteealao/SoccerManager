//! The real substitution rows of the signature table: how many substitutions a team makes in
//! a match and at which minute, per competition and pooled per era of the substitution law.
//!
//! - The three-substitution era: the Wyscout 2017/18 matches of the fit leagues
//!   (`wy2_subs.csv`, one line per substitution with its match, team and minute, and the
//!   competition of each match from `wy2_matches.csv`). The held-out sets are never read: a
//!   line of a match outside the asked leagues is skipped before its minute is parsed, and a
//!   run that asks for a held-out league stops.
//! - The five-substitution era: the StatsBomb open data of Euro 2020, the 2022 World Cup and
//!   Euro 2024 (`sb2_events.csv.gz`, its `Substitution` events in periods 1 and 2, since the
//!   engine's equal suite plays no extra time), with the share of substitutions StatsBomb marks
//!   as forced by an injury. These are national teams. A held-out tournament stops the run.
//!
//! A minute is the clock minute as the source gives it: Wyscout's whole minute, StatsBomb's
//! minute and second (its second half starts at minute 45). Each team of each match counts,
//! with no substitution as a count of 0.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, bail};
use serde::Serialize;

use super::{
    STATSBOMB, STATSBOMB_HELD_OUT, WYSCOUT, cells, column, lines, quantile, refuse_held_out, round6,
};

/// The StatsBomb tournaments of the five-substitution era, as `competition season`.
pub const FIVE_SUB_TOURNAMENTS: [&str; 3] =
    ["UEFA Euro 2020", "FIFA World Cup 2022", "UEFA Euro 2024"];

/// One competition's substitutions.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Competition {
    pub source: String,
    pub competition: String,
    /// The substitution limit of its law: 3 or 5.
    pub limit: u32,
    pub matches: usize,
    pub substitutions: usize,
    /// Substitutions a team makes in a match.
    pub per_team: f64,
    /// Team matches by the number of substitutions made, from 0.
    pub counts: Vec<usize>,
    /// The minute at 25, 50 and 75 percent of the substitutions.
    pub minute_quartiles: [f64; 3],
    /// The share of substitutions the source marks as forced by an injury; Wyscout marks none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injury_share: Option<f64>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub national_teams: bool,
}

/// One era pooled over its competitions, with the range of the competitions' values: the
/// band a run on that era's rule pack is checked against.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Era {
    pub limit: u32,
    pub source: String,
    pub competitions: Vec<String>,
    pub matches: usize,
    pub substitutions: usize,
    pub per_team: f64,
    /// The lowest and highest competition's substitutions a team makes in a match.
    pub per_team_range: [f64; 2],
    /// The pooled minute quartiles, and each quartile's lowest and highest competition.
    pub minute_quartiles: [f64; 3],
    pub minute_quartile_ranges: [[f64; 2]; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injury_share: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injury_share_range: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub national_teams: bool,
}

/// The substitution part of the table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Substitutions {
    pub competitions: Vec<Competition>,
    pub eras: Vec<Era>,
}

/// What a competition's lines add up to while they are read.
#[derive(Default)]
struct Tally {
    matches: BTreeSet<String>,
    per_side: BTreeMap<(String, String), usize>,
    minutes: Vec<f64>,
    injuries: usize,
}

/// One era's competitions and every substitution minute of the era.
pub type EraRead = (Vec<Competition>, Vec<f64>);

impl Tally {
    /// The competition's row; its minutes join `all`.
    fn competition(
        self,
        source: &str,
        competition: &str,
        limit: u32,
        injuries: bool,
        all: &mut Vec<f64>,
    ) -> anyhow::Result<Competition> {
        let matches = self.matches.len();
        if matches == 0 || self.minutes.is_empty() {
            bail!("{source} {competition}: no substitution read");
        }
        let sides = 2 * matches;
        let most = self.per_side.values().copied().max().unwrap_or(0);
        let mut counts = vec![0usize; most + 1];
        for n in self.per_side.values() {
            counts[*n] += 1;
        }
        counts[0] += sides - self.per_side.len();
        let mut minutes = self.minutes;
        all.extend(&minutes);
        minutes.sort_by(f64::total_cmp);
        let n = minutes.len();
        Ok(Competition {
            source: source.into(),
            competition: competition.into(),
            limit,
            matches,
            substitutions: n,
            per_team: round6(n as f64 / sides as f64),
            counts,
            minute_quartiles: [0.25, 0.5, 0.75].map(|p| round6(quantile(&minutes, p))),
            injury_share: injuries.then(|| round6(self.injuries as f64 / n as f64)),
            national_teams: source == STATSBOMB,
        })
    }
}

/// The Wyscout competitions of `competitions`, one row each, from `wy2_subs.csv` and
/// `wy2_matches.csv` in `dir`; none when `wy2_subs.csv` is missing.
pub fn wyscout(dir: &Path, competitions: &[String]) -> anyhow::Result<EraRead> {
    for c in competitions {
        refuse_held_out(c)?;
    }
    let path = dir.join("wy2_subs.csv");
    if !path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }
    let comps = super::match_competitions(dir)?;
    let wanted: BTreeSet<&str> = competitions.iter().map(String::as_str).collect();
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for (m, comp) in &comps {
        if wanted.contains(comp.as_str()) {
            tallies
                .entry(comp.clone())
                .or_default()
                .matches
                .insert(m.clone());
        }
    }
    let file = "wy2_subs.csv";
    let mut it = lines(&path)?;
    let header = cells(&it.next().context("an empty wy2_subs.csv")??);
    let (c_match, c_team, c_minute) = (
        column(&header, "match", file)?,
        column(&header, "team", file)?,
        column(&header, "minute", file)?,
    );
    for line in it {
        let c = cells(&line?);
        let Some(comp) = comps.get(&c[c_match]) else {
            bail!("match {} is not in wy2_matches.csv", c[c_match]);
        };
        // Another competition's line, a held-out set's among them, is skipped unread.
        if !wanted.contains(comp.as_str()) {
            continue;
        }
        refuse_held_out(comp)?;
        let minute: f64 = c[c_minute]
            .parse()
            .with_context(|| format!("wy2_subs.csv: minute {}", c[c_minute]))?;
        let t = tallies.entry(comp.clone()).or_default();
        *t.per_side
            .entry((c[c_match].clone(), c[c_team].clone()))
            .or_default() += 1;
        t.minutes.push(minute);
    }
    let mut all = Vec::new();
    let rows = competitions
        .iter()
        .filter_map(|c| tallies.remove(c).map(|t| (c, t)))
        .map(|(c, t)| t.competition(WYSCOUT, c, 3, false, &mut all))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok((rows, all))
}

/// The StatsBomb tournaments of [`FIVE_SUB_TOURNAMENTS`], one row each, from
/// `sb2_events.csv.gz` in `dir`; none when the file is missing. A held-out tournament stops
/// the run.
pub fn statsbomb(dir: &Path) -> anyhow::Result<EraRead> {
    let path = dir.join("sb2_events.csv.gz");
    if !path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }
    let file = "sb2_events.csv.gz";
    let mut it = lines(&path)?;
    let header = cells(&it.next().context("an empty sb2_events.csv.gz")??);
    let col = |n: &str| column(&header, n, file);
    let (c_mid, c_period, c_minute, c_second, c_type, c_team, c_out, c_comp, c_season) = (
        col("mid")?,
        col("period")?,
        col("minute")?,
        col("second")?,
        col("type")?,
        col("team")?,
        col("out")?,
        col("comp")?,
        col("season")?,
    );
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for line in it {
        let c = cells(&line?);
        if c.len() <= c_season.max(c_out) {
            continue;
        }
        let comp = format!("{} {}", c[c_comp], c[c_season]);
        if STATSBOMB_HELD_OUT.contains(&comp.as_str()) {
            bail!("held-out set: {comp} is never read");
        }
        if !FIVE_SUB_TOURNAMENTS.contains(&comp.as_str())
            || !matches!(c[c_period].as_str(), "1" | "2")
        {
            continue;
        }
        let t = tallies.entry(comp).or_default();
        t.matches.insert(c[c_mid].clone());
        if c[c_type] != "Substitution" {
            continue;
        }
        let minute: f64 = c[c_minute].parse().context("a StatsBomb minute")?;
        let second: f64 = c[c_second].parse().unwrap_or(0.0);
        *t.per_side
            .entry((c[c_mid].clone(), c[c_team].clone()))
            .or_default() += 1;
        t.minutes.push(minute + second / 60.0);
        if c[c_out] == "Injury" {
            t.injuries += 1;
        }
    }
    let mut all = Vec::new();
    let rows = FIVE_SUB_TOURNAMENTS
        .iter()
        .filter_map(|c| tallies.remove(*c).map(|t| (*c, t)))
        .map(|(c, t)| t.competition(STATSBOMB, c, 5, true, &mut all))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok((rows, all))
}

/// `competitions` of one era pooled: the per-team rate and the injury share over every
/// substitution, the quartiles over every minute (from `minutes`), and the range of the
/// competitions' own values.
fn era(competitions: &[Competition], minutes: &[f64]) -> Option<Era> {
    let first = competitions.first()?;
    let matches: usize = competitions.iter().map(|c| c.matches).sum();
    let substitutions: usize = competitions.iter().map(|c| c.substitutions).sum();
    let range = |f: &dyn Fn(&Competition) -> f64| {
        let v: Vec<f64> = competitions.iter().map(f).collect();
        [
            v.iter().copied().fold(f64::INFINITY, f64::min),
            v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ]
    };
    let mut sorted = minutes.to_vec();
    sorted.sort_by(f64::total_cmp);
    let injuries = first.injury_share.is_some().then(|| {
        competitions
            .iter()
            .map(|c| c.injury_share.unwrap_or(0.0) * c.substitutions as f64)
            .sum::<f64>()
    });
    Some(Era {
        limit: first.limit,
        source: first.source.clone(),
        competitions: competitions.iter().map(|c| c.competition.clone()).collect(),
        matches,
        substitutions,
        per_team: round6(substitutions as f64 / (2 * matches) as f64),
        per_team_range: range(&|c| c.per_team),
        minute_quartiles: [0.25, 0.5, 0.75].map(|p| round6(quantile(&sorted, p))),
        minute_quartile_ranges: [0, 1, 2].map(|q| range(&|c| c.minute_quartiles[q])),
        injury_share: injuries.map(|i| round6(i / substitutions as f64)),
        injury_share_range: first
            .injury_share
            .is_some()
            .then(|| range(&|c| c.injury_share.unwrap_or(0.0))),
        national_teams: first.national_teams,
    })
}

/// The substitution part of the table from the files of `dir`: the Wyscout competitions of
/// `competitions` and the StatsBomb tournaments, each era pooled. `None` when neither source's
/// file is there.
pub fn build(dir: &Path, competitions: &[String]) -> anyhow::Result<Option<Substitutions>> {
    let (wy, wy_minutes) = wyscout(dir, competitions)?;
    let (sb, sb_minutes) = statsbomb(dir)?;
    if wy.is_empty() && sb.is_empty() {
        return Ok(None);
    }
    let mut eras = Vec::new();
    eras.extend(era(&wy, &wy_minutes));
    eras.extend(era(&sb, &sb_minutes));
    let mut all = wy;
    all.extend(sb);
    Ok(Some(Substitutions {
        competitions: all,
        eras,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sm-subs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn gzip(path: &Path, text: &str) {
        use std::io::Write;
        let file = std::fs::File::create(path).unwrap();
        let mut gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        gz.write_all(text.as_bytes()).unwrap();
        gz.finish().unwrap();
    }

    /// Two English matches: team 1 makes three substitutions in the first and team 2 one; in
    /// the second only team 3 makes two. A German line is skipped unread (its minute is not a
    /// number), and asking for Germany stops the run.
    #[test]
    fn the_wyscout_rows_count_every_team_match_and_skip_the_held_out_lines() {
        let dir = folder("wy");
        std::fs::write(
            dir.join("wy2_matches.csv"),
            "comp,match\nEngland,m1\nEngland,m2\nGermany,g1\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("wy2_subs.csv"),
            "match,team,minute,pin,pout\nm1,1,60,1,2\nm1,1,70,3,4\nm1,1,80,5,6\nm1,2,46,7,8\n\
             g1,9,x,1,1\nm2,3,65,1,2\nm2,3,90,3,4\n",
        )
        .unwrap();
        let (rows, minutes) = wyscout(&dir, &["England".to_string()]).unwrap();
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!((r.matches, r.substitutions, r.limit), (2, 6, 3));
        assert_eq!(r.per_team, 1.5);
        // Teams 1, 2, 3 and the second match's other side: 3, 1, 2 and 0.
        assert_eq!(r.counts, vec![1, 1, 1, 1]);
        assert_eq!(minutes.len(), 6);
        assert_eq!(r.injury_share, None);
        let err = wyscout(&dir, &["Germany".to_string()])
            .unwrap_err()
            .to_string();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(err.contains("held-out set: Germany"), "{err}");
    }

    /// One Euro 2024 match: A makes two substitutions (one for an injury) and B none; one in
    /// extra time is not counted, nor a Bundesliga one. A held-out tournament stops the run.
    #[test]
    fn the_statsbomb_rows_read_periods_one_and_two_of_the_tournaments() {
        let dir = folder("sb");
        let head = "mid,period,minute,second,type,team,out,comp,season\n";
        let mut text = String::from(head);
        for r in [
            "1,1,0,0,Pass,A,,UEFA Euro,2024",
            "1,2,60,30,Substitution,A,Tactical,UEFA Euro,2024",
            "1,2,75,0,Substitution,A,Injury,UEFA Euro,2024",
            "1,3,95,0,Substitution,B,Tactical,UEFA Euro,2024",
            "2,2,70,0,Substitution,C,Tactical,1. Bundesliga,2023/2024",
        ] {
            text.push_str(r);
            text.push('\n');
        }
        gzip(&dir.join("sb2_events.csv.gz"), &text);
        let (rows, minutes) = statsbomb(&dir).unwrap();
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.competition, "UEFA Euro 2024");
        assert_eq!((r.matches, r.substitutions, r.limit), (1, 2, 5));
        assert_eq!(r.counts, vec![1, 0, 1]);
        assert_eq!(r.injury_share, Some(0.5));
        assert!(r.national_teams);
        assert_eq!(minutes, vec![60.5, 75.0]);
        let one = era(&rows, &minutes).unwrap();
        assert_eq!(one.per_team_range, [1.0, 1.0]);
        text.push_str("3,1,10,0,Pass,D,,UEFA Euro,2016\n");
        gzip(&dir.join("sb2_events.csv.gz"), &text);
        let err = statsbomb(&dir).unwrap_err().to_string();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(err.contains("held-out set: UEFA Euro 2016"), "{err}");
    }
}
