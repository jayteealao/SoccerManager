//! The blocked-shot cross-check: does football-data.co.uk's shot count (`HS`, `AS`) count
//! blocked shots? StatsBomb's open events of one competition give each team's shots with
//! and without the blocked ones; the result file's count is compared with both on every
//! match the two sources share. The builder builds shot targets only when the file's count
//! is nearer the total with blocked shots.
//!
//! The matches are joined by date and by one club present in every match of the
//! StatsBomb competition (Bundesliga 2023/24 holds Leverkusen's 34 matches), named as each
//! source names it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::csv::Table;

/// The competition the builder checks on: StatsBomb's matches file, the football-data file
/// of the same season, and the club in every match as each source names it.
#[derive(Debug, Clone)]
pub struct Spec {
    pub statsbomb_matches: &'static str,
    pub football_data: &'static str,
    pub statsbomb_club: &'static str,
    pub football_data_club: &'static str,
}

/// StatsBomb Bundesliga 2023/24 (competition 9, season 281) against `2324/D1.csv`.
pub const BUNDESLIGA_2324: Spec = Spec {
    statsbomb_matches: "sb/matches/9_281.json",
    football_data: "football-data/2324/D1.csv",
    statsbomb_club: "Bayer Leverkusen",
    football_data_club: "Leverkusen",
};

/// One team in one match: the file's count and StatsBomb's, with and without blocked
/// shots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub date: String,
    pub team: String,
    pub file: u32,
    pub statsbomb_all: u32,
    pub statsbomb_blocked: u32,
}

/// What the check found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub competition: String,
    pub matches: u32,
    /// Mean per team per match: the file, StatsBomb with blocked shots, without them.
    pub file_mean: f64,
    pub with_blocked_mean: f64,
    pub without_blocked_mean: f64,
    pub include_blocked: bool,
}

#[derive(Deserialize)]
struct SbMatch {
    match_id: u64,
    match_date: String,
    home_team: SbHome,
    away_team: SbAway,
}

#[derive(Deserialize)]
struct SbHome {
    home_team_name: String,
}

#[derive(Deserialize)]
struct SbAway {
    away_team_name: String,
}

#[derive(Deserialize)]
struct SbEvent {
    #[serde(rename = "type")]
    kind: SbName,
    period: u32,
    team: SbName,
    #[serde(default)]
    shot: Option<SbShot>,
}

#[derive(Deserialize)]
struct SbName {
    name: String,
}

#[derive(Deserialize)]
struct SbShot {
    outcome: SbName,
}

/// Runs the check of `spec` on the files of `data`; returns the lines, the summary and the
/// files read (relative to `data`).
pub fn run(data: &Path, spec: &Spec) -> anyhow::Result<(Vec<Line>, Summary, Vec<String>)> {
    let mut files = vec![
        spec.statsbomb_matches.to_string(),
        spec.football_data.to_string(),
    ];
    let text = std::fs::read_to_string(data.join(spec.statsbomb_matches))
        .with_context(|| format!("cannot read {}", spec.statsbomb_matches))?;
    let mut matches: Vec<SbMatch> = serde_json::from_str(&text)
        .with_context(|| format!("{} cannot be read", spec.statsbomb_matches))?;
    matches.sort_by(|a, b| (&a.match_date, a.match_id).cmp(&(&b.match_date, b.match_id)));
    let t = Table::read(&data.join(spec.football_data))?;
    // The file's rows of the club, by date (yyyy-mm-dd).
    let mut rows: BTreeMap<String, &Vec<String>> = BTreeMap::new();
    for row in &t.rows {
        if t.get(row, "HomeTeam") == spec.football_data_club
            || t.get(row, "AwayTeam") == spec.football_data_club
        {
            let d = t.get(row, "Date");
            let iso = match d.split('/').collect::<Vec<_>>()[..] {
                [dd, mm, yyyy] if yyyy.len() == 4 => format!("{yyyy}-{mm}-{dd}"),
                [dd, mm, yy] => format!("20{yy}-{mm}-{dd}"),
                _ => continue,
            };
            rows.insert(iso, row);
        }
    }
    let events_dir = Path::new(spec.statsbomb_matches)
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| Path::new("events").to_path_buf(), |p| p.join("events"));
    let mut lines = Vec::new();
    for m in &matches {
        let Some(row) = rows.get(&m.match_date) else {
            continue;
        };
        let home = &m.home_team.home_team_name;
        let away = &m.away_team.away_team_name;
        // The club's side must agree in both sources.
        let club_home = home == spec.statsbomb_club;
        if club_home != (t.get(row, "HomeTeam") == spec.football_data_club) {
            continue;
        }
        let rel = events_dir.join(format!("{}.json", m.match_id));
        let rel_text = rel.to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(data.join(&rel))
            .with_context(|| format!("cannot read {rel_text}"))?;
        let events: Vec<SbEvent> =
            serde_json::from_str(&text).with_context(|| format!("{rel_text} cannot be read"))?;
        files.push(rel_text);
        let mut count: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
        for e in &events {
            // Periods 1 to 4: play and extra time; 5 is a shoot-out.
            if e.kind.name == "Shot" && e.period <= 4 {
                let c = count.entry(e.team.name.as_str()).or_default();
                c.0 += 1;
                if e.shot.as_ref().is_some_and(|s| s.outcome.name == "Blocked") {
                    c.1 += 1;
                }
            }
        }
        for (team, column) in [(home, "HS"), (away, "AS")] {
            let Some(file) = t.num(row, column) else {
                continue;
            };
            let (all, blocked) = count.get(team.as_str()).copied().unwrap_or_default();
            lines.push(Line {
                date: m.match_date.clone(),
                team: team.clone(),
                file: file as u32,
                statsbomb_all: all,
                statsbomb_blocked: blocked,
            });
        }
    }
    let n = lines.len().max(1) as f64;
    let mean = |f: fn(&Line) -> u32| lines.iter().map(|l| f64::from(f(l))).sum::<f64>() / n;
    let file_mean = mean(|l| l.file);
    let with_blocked_mean = mean(|l| l.statsbomb_all);
    let without_blocked_mean = mean(|l| l.statsbomb_all - l.statsbomb_blocked);
    let summary = Summary {
        competition: format!("{} against {}", spec.statsbomb_matches, spec.football_data),
        matches: (lines.len() / 2) as u32,
        file_mean,
        with_blocked_mean,
        without_blocked_mean,
        include_blocked: !lines.is_empty()
            && (file_mean - with_blocked_mean).abs() < (file_mean - without_blocked_mean).abs(),
    };
    Ok((lines, summary, files))
}

/// The printed table: one line per team per match, then the means and the answer.
pub fn table(lines: &[Line], s: &Summary) -> String {
    let mut out = format!("shots check: {}\n", s.competition);
    let _ = writeln!(
        out,
        "{:<10} {:<26} {:>4} {:>9} {:>11}",
        "date", "team", "file", "statsbomb", "of which blocked"
    );
    for l in lines {
        let _ = writeln!(
            out,
            "{:<10} {:<26} {:>4} {:>9} {:>11}",
            l.date, l.team, l.file, l.statsbomb_all, l.statsbomb_blocked
        );
    }
    let _ = writeln!(
        out,
        "{} matches; mean per team: file {:.2}, statsbomb with blocked {:.2}, without {:.2}",
        s.matches, s.file_mean, s.with_blocked_mean, s.without_blocked_mean
    );
    let _ = writeln!(
        out,
        "answer: the file's shots {} blocked shots",
        if s.include_blocked {
            "count"
        } else {
            "do not count"
        }
    );
    out
}
