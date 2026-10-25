//! The tracked-match targets: each tracked band's real value from the DFL tracking file
//! (`gate/tracking/dfl-matches.json`), measured by the shared shape report.
//!
//! A tracked band is one of the band file whose real side is the tracked matches: a
//! `trk.dfl.<band>` target, or the earlier tracked-match files it replaces (the movement
//! study's `shape3_real.json` and the player file `idsse_players.csv`). Its target is keyed
//! `trk.dfl.<band>`: the band's own measure applied to each tracked match's figures, one
//! value per match; the target's value is their mean, its error the jackknife error of the
//! mean over the matches, and the per-match values go with it so a band may resample the
//! matches. A match whose denominator is 0 gives no value. A band field is read by its own
//! name: `trk.<figure>.<side>` as the shared report names it, and an earlier name
//! (`shape.<sum>`, the running fields, `ball_in_play_min`) by the same figure.
//!
//! The intake reads only a tracking file whose source is on a closed list (DFL); a
//! SoccerTrack contrast, or any other file, is refused: SoccerTrack data sets no target.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Context;
use serde_json::Value;

use super::{EngineCount, Label, Layer, Target};
use crate::report::tracked_shape::FRAMES_PER_SECOND;
use crate::tracking::measure::{MatchFigures, TrackingFile};

/// The sources whose tracked matches may set a target.
pub const PROVIDERS: [&str; 1] = ["DFL"];
/// The real-source files the tracked targets replace.
pub const EARLIER_FILES: [&str; 2] = ["shape3_real.json", "idsse_players.csv"];
/// The prefix of a tracked target's key.
pub const KEY_PREFIX: &str = "trk.dfl.";

/// Reads the tracking file at `path` for the targets, refusing a source off the list.
pub fn read_tracking(path: &Path) -> anyhow::Result<TrackingFile> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read the tracking file {}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("{} is not JSON", path.display()))?;
    let source = value["source"].as_str().unwrap_or("");
    anyhow::ensure!(
        source != crate::tracking::contrast::SOURCE,
        "{}: SoccerTrack data sets no target",
        path.display()
    );
    anyhow::ensure!(
        PROVIDERS.contains(&source),
        "{}: the source {source:?} sets no target; the tracked targets come from {}",
        path.display(),
        PROVIDERS.join(", ")
    );
    TrackingFile::read(path)
}

/// The shared report's figure a band field names, without its side: `trk.<figure>` as it
/// is, and an earlier name by the same figure.
pub fn figure_of(base: &str) -> Option<String> {
    if let Some(f) = base.strip_prefix("trk.") {
        return Some(f.to_string());
    }
    if let Some(sum) = base.strip_prefix("shape.") {
        return Some(match sum.split_once('.') {
            Some((g, "live_ticks")) => format!("{g}.live_frames"),
            Some(_) => sum.to_string(),
            None => match sum {
                "calm_ticks" => "team.calm_frames".into(),
                "dead_ticks" => "team.dead_frames".into(),
                other => format!("team.{other}"),
            },
        });
    }
    Some(match base {
        "distance_m" => "run.distance_m".into(),
        "minutes_on" => "run.minutes_on".into(),
        "peak_mps" => "run.peak_sum".into(),
        "players_used" => "run.players_used".into(),
        "ball_in_play_min" => "team.live_min".into(),
        _ => format!("run.zone_{}", base.strip_prefix("zone_ticks.")?),
    })
}

/// The value of the band field `name` (`<base>.home`, `<base>.away`, or a match field, read
/// on the home side) in one match's figures.
fn field(m: &MatchFigures, name: &str) -> Option<f64> {
    let (base, side) = match name.rsplit_once('.') {
        Some((b, "home")) => (b, 0),
        Some((b, "away")) => (b, 1),
        _ => (name, 0),
    };
    let figure = figure_of(base)?;
    let per_minute = FRAMES_PER_SECOND * 60.0;
    if let Some(of) = figure.strip_suffix(".live_min") {
        return Some(m.figures.get(&format!("{of}.live_frames"))?[side] / per_minute);
    }
    if figure == "run.minutes_on" {
        return Some(m.figures.get("run.frames_on")?[side] / per_minute);
    }
    Some(m.figures.get(&figure)?[side])
}

fn names(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn sum(m: &MatchFigures, fields: &[String]) -> Option<f64> {
    fields.iter().map(|f| field(m, f)).sum()
}

/// A band measure's value in one match: a ratio of the summed fields, or a mean per match
/// (the fields summed) or per team (the sides' mean). `None` for another kind, a field the
/// figures lack, or a ratio whose denominator is 0.
pub fn match_value(measure: &Value, m: &MatchFigures) -> Option<f64> {
    match measure["kind"].as_str()? {
        "ratio" => {
            let den = sum(m, &names(&measure["den"]))?;
            if den == 0.0 {
                return None;
            }
            Some(sum(m, &names(&measure["num"]))? / den)
        }
        "mean" => {
            let of = names(&measure["of"]);
            match measure["per"].as_str()? {
                "match" => sum(m, &of),
                "team" => {
                    let side = |s: &str| -> Vec<String> {
                        of.iter().map(|f| f.replace("{side}", s)).collect()
                    };
                    Some((sum(m, &side("home"))? + sum(m, &side("away"))?) / 2.0)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// The mean of `values` and its jackknife error.
pub fn mean_and_jackknife(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    let total: f64 = values.iter().sum();
    let mean = total / n;
    if values.len() < 2 {
        return (mean, 0.0);
    }
    let left_out: Vec<f64> = values.iter().map(|v| (total - v) / (n - 1.0)).collect();
    let centre = left_out.iter().sum::<f64>() / n;
    let ss: f64 = left_out.iter().map(|v| (v - centre).powi(2)).sum();
    (mean, ((n - 1.0) / n * ss).sqrt())
}

/// Whether a band's real side is the tracked matches.
pub fn is_tracked(band: &Value) -> bool {
    let source = &band["real"]["source"];
    source["target"]
        .as_str()
        .is_some_and(|t| t.starts_with(KEY_PREFIX))
        || source["file"]
            .as_str()
            .is_some_and(|f| EARLIER_FILES.contains(&f))
}

/// The tracked targets of every tracked band of the band file `registry`.
pub fn targets(
    file: &TrackingFile,
    registry: &Value,
    out: &mut BTreeMap<String, Target>,
    absent: &mut BTreeMap<String, String>,
) {
    let bands = registry["bands"].as_array().cloned().unwrap_or_default();
    for band in bands.iter().filter(|b| is_tracked(b)) {
        let Some(name) = band["band"].as_str() else {
            continue;
        };
        let key = format!("{KEY_PREFIX}{name}");
        let values: Vec<f64> = file
            .matches
            .iter()
            .filter_map(|m| match_value(&band["measure"], m))
            .collect();
        if values.is_empty() {
            absent.insert(
                key,
                "no tracked match gives the band's measure a value".into(),
            );
            continue;
        }
        let (value, se) = mean_and_jackknife(&values);
        out.insert(
            key,
            Target {
                value: Some(value),
                shares: None,
                bins: None,
                width: None,
                se,
                n: values.len() as u64,
                seasons: vec!["2223".into()],
                between_season_sd: None,
                between_league_sd: None,
                layer: Layer::Pooled,
                leagues: Some(vec!["D1".into(), "D2".into()]),
                provider: file.source.clone(),
                source_label: format!(
                    "{} tracked matches, measured at 25 frames a second by the shared shape \
                     report",
                    file.source
                ),
                event_definition: format!("the band {name}'s measure in each tracked match"),
                denominator: "tracked matches".into(),
                unit_of_observation: "match".into(),
                label: Label::Calibration,
                engine_count: EngineCount {
                    text: format!("the band {name} on the engine's matches"),
                    band: Some(name.to_string()),
                    measure: Some(band["measure"].to_string()),
                },
                context: None,
                extrapolated: None,
                formula: None,
                per_match: Some(values),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn figures(id: &str, pairs: &[(&str, [f64; 2])]) -> MatchFigures {
        MatchFigures {
            match_id: id.into(),
            frames: 0,
            live_minutes: Vec::new(),
            clock_offset: None,
            checks: crate::tracking::measure::Checks {
                records: 0,
                rotated: false,
                off_pitch_spells: 0,
                ball_missing: 0,
                events_placed: 0,
                events_unplaced: 0,
            },
            figures: pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect(),
            halves: Vec::new(),
        }
    }

    fn ratio(num: &str, den: &str) -> Value {
        serde_json::json!({
            "kind": "ratio",
            "num": [format!("{num}.home"), format!("{num}.away")],
            "den": [format!("{den}.home"), format!("{den}.away")]
        })
    }

    /// The target of three fixture matches is the mean of their values and the
    /// leave-one-out error, with the values; a band read by the earlier names and one read
    /// by the shared report's names get the same target; another band gets none.
    #[test]
    fn a_tracked_target_is_the_mean_and_jackknife_error_of_the_matches() {
        let sums = [[10.0, 20.0], [20.0, 20.0], [30.0, 60.0]];
        let file = TrackingFile {
            schema: 1,
            source: "DFL".into(),
            credit: String::new(),
            frame_rate: 25,
            matches: sums
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    figures(
                        &format!("m{i}"),
                        &[("cb.depth_in_sum", *s), ("cb.depth_in_n", [1.0, 1.0])],
                    )
                })
                .collect(),
        };
        let registry = serde_json::json!({ "bands": [
            { "band": "cb_depth_in",
              "measure": ratio("shape.cb.depth_in_sum", "shape.cb.depth_in_n"),
              "real": { "source": { "file": "shape3_real.json", "key": "x" } } },
            { "band": "cb_depth_new",
              "measure": ratio("trk.cb.depth_in_sum", "trk.cb.depth_in_n"),
              "real": { "source": { "target": "trk.dfl.cb_depth_new" } } },
            { "band": "goals", "measure": ratio("goals", "shots"),
              "real": { "source": { "file": "other.json", "key": "x" } } }
        ]});
        let (mut out, mut absent) = (BTreeMap::new(), BTreeMap::new());
        targets(&file, &registry, &mut out, &mut absent);
        assert_eq!(out.len(), 2, "{out:?}");
        let t = &out["trk.dfl.cb_depth_in"];
        // Per match 15, 20 and 45; the leave-one-out error of a mean is sd / sqrt(n).
        assert_eq!(t.per_match.as_deref(), Some(&[15.0, 20.0, 45.0][..]));
        let mean = 80.0 / 3.0;
        assert!((t.value.unwrap() - mean).abs() < 1e-12);
        let var = [15.0f64, 20.0, 45.0]
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>()
            / 2.0;
        assert!((t.se - (var / 3.0).sqrt()).abs() < 1e-12, "{}", t.se);
        assert_eq!(t.n, 3);
        assert_eq!(t.provider, "DFL");
        assert_eq!(out["trk.dfl.cb_depth_new"].value, t.value);
        assert!(absent.is_empty());
    }

    #[test]
    fn the_earlier_names_read_the_shared_report_s_figures() {
        assert_eq!(figure_of("shape.cb.live_ticks").unwrap(), "cb.live_frames");
        assert_eq!(figure_of("shape.cb.live_min").unwrap(), "cb.live_min");
        assert_eq!(
            figure_of("shape.slide_x_in_sxy").unwrap(),
            "team.slide_x_in_sxy"
        );
        assert_eq!(figure_of("shape.calm_ticks").unwrap(), "team.calm_frames");
        assert_eq!(figure_of("zone_ticks.hi").unwrap(), "run.zone_hi");
        assert_eq!(figure_of("ball_in_play_min").unwrap(), "team.live_min");
        assert_eq!(figure_of("trk.free.n").unwrap(), "free.n");
        assert_eq!(figure_of("goals"), None);
        let m = figures(
            "a",
            &[
                ("run.frames_on", [3000.0, 1500.0]),
                ("cm.live_frames", [1500.0, 0.0]),
            ],
        );
        assert_eq!(field(&m, "minutes_on.home"), Some(2.0));
        assert_eq!(field(&m, "shape.cm.live_min.home"), Some(1.0));
        let per_team =
            serde_json::json!({"kind": "mean", "per": "team", "of": ["minutes_on.{side}"]});
        assert_eq!(match_value(&per_team, &m), Some(1.5));
        let zero = figures(
            "z",
            &[
                ("cb.depth_in_sum", [1.0, 1.0]),
                ("cb.depth_in_n", [0.0, 0.0]),
            ],
        );
        assert_eq!(
            match_value(&ratio("trk.cb.depth_in_sum", "trk.cb.depth_in_n"), &zero),
            None
        );
    }

    /// A contrast file fed to the intake is refused, as is a tracking file of a source off
    /// the list.
    #[test]
    fn the_intake_refuses_soccertrack_and_unlisted_sources() {
        let dir = tempfile::TempDir::new().unwrap();
        let contrast = dir.path().join("contrast.json");
        std::fs::write(
            &contrast,
            serde_json::json!({ "schema": 1, "source": "SoccerTrack", "amateur": {} }).to_string(),
        )
        .unwrap();
        let err = read_tracking(&contrast).unwrap_err().to_string();
        assert!(err.contains("SoccerTrack data sets no target"), "{err}");
        let other = dir.path().join("other.json");
        std::fs::write(
            &other,
            serde_json::json!({ "schema": 1, "source": "Metrica", "matches": [] }).to_string(),
        )
        .unwrap();
        let err = read_tracking(&other).unwrap_err().to_string();
        assert!(err.contains("sets no target"), "{err}");
    }
}
