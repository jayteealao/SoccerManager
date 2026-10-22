//! The band file read by name, for the engine's slow tests.
//!
//! A slow test plays its own scenario and judges the sample with the same pass rule as the
//! calibrate report: the band's margin and target come from `content/realism-bands.json`,
//! never from a number in the test. The real side of a band resolves in this order: the
//! band's design target; else the reference-file target its `target` key names (read through
//! a narrow view of `gate/real-targets.json`: `value`, `se`, `n` and `between_league_sd`);
//! else the band's real value in the calibrate report named by `SM_REAL_REPORT`; else the
//! band is not sure, with that reason. Not sure is never a pass.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use crate::bounds::clopper_pearson;
use crate::interval::{Interval, RareKind, Real, Unit, interval, rare_interval, real_bounds};
use crate::rule::{Word, gate_exposure, judge, judge_design};
use crate::special::norm_quantile;

/// The variable that names a calibrate report whose real values the slow tests may read.
pub const REAL_REPORT: &str = "SM_REAL_REPORT";

/// The bootstrap resamples of a slow test's interval.
const REPLICATES: usize = 2000;

/// The narrow view of one reference-file target: the four figures the pass rule reads. A
/// distribution target has no single value.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct RefTarget {
    #[serde(default)]
    pub value: Option<f64>,
    pub se: f64,
    pub n: u64,
    #[serde(default)]
    pub between_league_sd: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RefFile {
    targets: BTreeMap<String, RefTarget>,
}

/// Reads the reference file's targets through the narrow view; other fields are ignored.
pub fn reference_targets(path: &Path) -> Result<BTreeMap<String, RefTarget>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file: RefFile =
        serde_json::from_str(&text).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    Ok(file.targets)
}

#[derive(Debug, Deserialize)]
struct TargetView {
    #[serde(default)]
    design: Option<f64>,
    #[serde(default)]
    cap: Option<f64>,
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MarginView {
    value: f64,
}

#[derive(Debug, Deserialize)]
struct ExposureView {
    value: f64,
}

#[derive(Debug, Deserialize)]
struct MeasureView {
    kind: String,
}

#[derive(Debug, Deserialize)]
struct BandView {
    band: String,
    target: TargetView,
    margin: MarginView,
    measure: MeasureView,
    #[serde(default)]
    rare: bool,
    min_exposure: ExposureView,
}

#[derive(Debug, Deserialize)]
struct RuleView {
    alpha: f64,
}

#[derive(Debug, Deserialize)]
struct BandFile {
    schema_version: u32,
    pass_rule: RuleView,
    bands: Vec<BandView>,
}

#[derive(Debug, Deserialize)]
struct ReportFile {
    #[serde(rename = "calib.real", default)]
    real: Option<ReportReal>,
}

#[derive(Debug, Deserialize)]
struct ReportReal {
    bands: Vec<ReportBand>,
}

#[derive(Debug, Deserialize)]
struct ReportBand {
    band: String,
    #[serde(default)]
    value: Option<f64>,
    #[serde(default)]
    se: Option<f64>,
}

/// One band as a slow test reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    pub band: String,
    pub design: Option<f64>,
    pub cap: Option<f64>,
    pub target_key: Option<String>,
    pub margin: f64,
    pub rare: bool,
    pub share: bool,
    pub min_exposure: f64,
}

/// The band file and the real values a slow test can reach.
#[derive(Debug, Clone)]
pub struct Limits {
    alpha: f64,
    bands: BTreeMap<String, Limit>,
    reference: BTreeMap<String, RefTarget>,
    /// The real values of the report named by [`REAL_REPORT`], or why there are none.
    report: Result<BTreeMap<String, (f64, f64)>, String>,
}

/// A slow test's judgement of one band.
#[derive(Debug, Clone, PartialEq)]
pub struct Judged {
    pub band: String,
    pub word: Word,
    /// The engine value of the sample.
    pub value: f64,
    /// The interval of engine minus target (or real), when one was formed.
    pub interval: Option<Interval>,
    pub margin: f64,
    /// What the engine was judged against: `design 0.775`, `reference tier.1.x`, or
    /// `report <path>`.
    pub against: String,
    pub exposure: f64,
    /// Why the word is not sure, when it is.
    pub reason: Option<String>,
}

impl fmt::Display for Judged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} (value {:.4}",
            self.band,
            self.word.as_str(),
            self.value
        )?;
        if let Some(iv) = &self.interval {
            write!(
                f,
                ", interval [{:.4}, {:.4}] against margin {}",
                iv.lo, iv.hi, self.margin
            )?;
        } else {
            write!(f, ", no interval, margin {}", self.margin)?;
        }
        write!(f, ", against {}, exposure {}", self.against, self.exposure)?;
        if let Some(reason) = &self.reason {
            write!(f, "; {reason}")?;
        }
        write!(f, ")")
    }
}

impl Limits {
    /// Reads `realism-bands.json` from `content_dir`, `real-targets.json` from `gate_dir`
    /// (an absent file gives no reference targets) and, when [`REAL_REPORT`] is set, that
    /// calibrate report.
    pub fn load(content_dir: &Path, gate_dir: &Path) -> Result<Limits, String> {
        let report = match std::env::var(REAL_REPORT).ok().filter(|v| !v.is_empty()) {
            Some(path) => report_values(Path::new(&path)),
            None => Err(format!("{REAL_REPORT} is not set")),
        };
        Self::load_with(content_dir, gate_dir, report)
    }

    /// As [`Limits::load`], with the report's real values given.
    pub fn load_with(
        content_dir: &Path,
        gate_dir: &Path,
        report: Result<BTreeMap<String, (f64, f64)>, String>,
    ) -> Result<Limits, String> {
        let path = content_dir.join("realism-bands.json");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let file: BandFile = serde_json::from_str(&text)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        if file.schema_version != 6 {
            return Err(format!(
                "{} is schema_version {}; the slow tests read 6",
                path.display(),
                file.schema_version
            ));
        }
        let reference_path = gate_dir.join("real-targets.json");
        let reference = if reference_path.exists() {
            reference_targets(&reference_path)?
        } else {
            BTreeMap::new()
        };
        let bands = file
            .bands
            .into_iter()
            .map(|b| {
                let limit = Limit {
                    band: b.band.clone(),
                    design: b.target.design,
                    cap: b.target.cap,
                    target_key: b.target.target,
                    margin: b.margin.value,
                    rare: b.rare,
                    share: b.measure.kind == "share",
                    min_exposure: b.min_exposure.value,
                };
                (b.band, limit)
            })
            .collect();
        Ok(Limits {
            alpha: file.pass_rule.alpha,
            bands,
            reference,
            report,
        })
    }

    /// The band named `band`.
    pub fn limit(&self, band: &str) -> Option<&Limit> {
        self.bands.get(band)
    }

    /// The real side of `limit` other than a design target, by the resolution order.
    fn real(&self, limit: &Limit) -> Result<(f64, f64, String), String> {
        if let Some(key) = &limit.target_key {
            return match self.reference.get(key) {
                Some(RefTarget {
                    value: Some(value),
                    se,
                    ..
                }) => Ok((*value, *se, format!("reference {key}"))),
                Some(_) => Err(format!("the reference target {key} has no single value")),
                None => Err(format!("the reference file has no target {key}")),
            };
        }
        match &self.report {
            Ok(values) => match values.get(&limit.band) {
                Some(&(value, se)) => Ok((value, se, format!("the report of {REAL_REPORT}"))),
                None => Err(format!("the report of {REAL_REPORT} has no real value")),
            },
            Err(why) => Err(format!("no real value: {why}")),
        }
    }

    /// Judges `band` on a slow test's sample: one unit per match (or per penalty, scene or
    /// pair of matches), resampled whole. A rare band takes the exact bounds of its events
    /// (`sum num`) over its exposure (`sum den`); a sample of yes-or-no units against a
    /// fixed target takes the exact Clopper-Pearson bounds, so a sample with no misses still
    /// has an interval; any other sample takes the bootstrap interval. A band the file does
    /// not hold, or whose real side cannot be read, is not sure.
    pub fn judge_sample(&self, band: &str, sample: &[Unit]) -> Judged {
        let total = |f: fn(&Unit) -> f64| sample.iter().map(f).sum::<f64>();
        let scaled = total(|u| u.scale_num) / total(|u| u.scale_den);
        let value = total(|u| u.num) / total(|u| u.den) * scaled;
        let mut judged = Judged {
            band: band.to_string(),
            word: Word::NotSure,
            value,
            interval: None,
            margin: f64::NAN,
            against: "nothing".to_string(),
            exposure: sample.len() as f64,
            reason: None,
        };
        let Some(limit) = self.limit(band) else {
            judged.reason = Some(format!("the band file has no band {band}"));
            return judged;
        };
        judged.margin = limit.margin;
        let real = match limit.design {
            Some(design) => Ok((design, 0.0, format!("design {design}"))),
            None => self.real(limit),
        };
        let (target, target_se, against) = match real {
            Ok(r) => r,
            Err(why) => {
                judged.reason = Some(why);
                return judged;
            }
        };
        judged.against = against;
        let binary = sample
            .iter()
            .all(|u| (u.num == 0.0 || u.num == 1.0) && u.den == 1.0 && u.scale_den == u.scale_num);
        let formed = if limit.rare {
            let events = total(|u| u.num).round().max(0.0) as u64;
            let exposure = total(|u| u.den);
            judged.exposure = exposure;
            let kind = if limit.share {
                RareKind::Share
            } else {
                RareKind::Rate
            };
            let bounds = real_bounds(target, target_se, self.alpha);
            Ok(rare_interval(kind, events, exposure, bounds, self.alpha))
        } else if binary && target_se == 0.0 {
            let k = total(|u| u.num).round() as u64;
            let n = sample.len() as u64;
            let (lo, hi) = clopper_pearson(k, n, 1.0 - self.alpha);
            Ok(Interval {
                point: value - target,
                lo: lo - target,
                hi: hi - target,
                se: f64::NAN,
                dropped: 0,
            })
        } else {
            let real = if target_se == 0.0 {
                Real::Fixed(target)
            } else {
                Real::Summary {
                    value: target,
                    se: target_se,
                }
            };
            let mut draw = stream(seed_of(band));
            interval(sample, &real, self.alpha, REPLICATES, &mut draw)
        };
        let iv = match formed {
            Ok(iv) => iv,
            Err(why) => {
                judged.reason = Some(why);
                return judged;
            }
        };
        judged.interval = Some(iv);
        let word = match (limit.design, limit.cap) {
            (Some(_), Some(cap)) => match self.real(limit) {
                Ok((r, r_se, _)) => {
                    // Engine minus real: the interval to the design target moved by the
                    // target's distance from the real value, widened by the real error.
                    let z = norm_quantile(1.0 - self.alpha);
                    let to_real = Interval {
                        point: iv.point + target - r,
                        lo: iv.lo + target - r - z * r_se,
                        hi: iv.hi + target - r + z * r_se,
                        se: iv.se,
                        dropped: 0,
                    };
                    judge_design(&iv, limit.margin, Some((&to_real, cap)))
                }
                Err(why) => {
                    judged.reason = Some(format!("the cap has no real side: {why}"));
                    let word = judge(&iv, limit.margin);
                    if word == Word::Pass {
                        Word::NotSure
                    } else {
                        word
                    }
                }
            },
            _ => judge(&iv, limit.margin),
        };
        let (word, why) = gate_exposure(word, judged.exposure, limit.min_exposure);
        judged.word = word;
        if why.is_some() {
            judged.reason = why;
        }
        judged
    }
}

/// The real values of a calibrate report: `calib.real.bands[]` with a value.
pub fn report_values(path: &Path) -> Result<BTreeMap<String, (f64, f64)>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let report: ReportFile =
        serde_json::from_str(&text).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let real = report
        .real
        .ok_or_else(|| format!("{} has no calib.real section", path.display()))?;
    Ok(real
        .bands
        .into_iter()
        .filter_map(|b| Some((b.band, (b.value?, b.se.unwrap_or(0.0)))))
        .collect())
}

/// A seed from the band's name (FNV-1a), so a band's interval is the same on every run.
fn seed_of(band: &str) -> u64 {
    band.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A small seeded uniform stream (SplitMix64).
fn stream(seed: u64) -> impl FnMut() -> f64 {
    let mut s = seed;
    move || {
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn band(name: &str, target: &str, margin: f64, kind: &str, rare: bool) -> String {
        format!(
            r#"{{"band": "{name}", "target": {target}, "margin": {{"value": {margin}}},
                "measure": {{"kind": "{kind}"}}, "rare": {rare},
                "min_exposure": {{"unit": "matches", "value": 100}}}}"#
        )
    }

    fn folder(bands: &[String], reference: Option<&str>) -> TempDir {
        let dir = TempDir::new().unwrap();
        let file = format!(
            r#"{{"schema_version": 6, "pass_rule": {{"alpha": 0.05}}, "bands": [{}]}}"#,
            bands.join(",")
        );
        std::fs::write(dir.path().join("realism-bands.json"), file).unwrap();
        if let Some(r) = reference {
            std::fs::write(dir.path().join("real-targets.json"), r).unwrap();
        }
        dir
    }

    fn units(values: &[f64]) -> Vec<Unit> {
        values.iter().map(|&v| Unit::ratio(v, 1.0)).collect()
    }

    fn limits(dir: &TempDir, report: Result<BTreeMap<String, (f64, f64)>, String>) -> Limits {
        Limits::load_with(dir.path(), dir.path(), report).unwrap()
    }

    fn no_report() -> Result<BTreeMap<String, (f64, f64)>, String> {
        Err("not set".to_string())
    }

    #[test]
    fn a_design_band_passes_inside_its_margin_and_fails_far_outside() {
        let dir = folder(
            &[band("p", r#"{"design": 0.8}"#, 0.1, "scenario", false)],
            None,
        );
        let l = limits(&dir, no_report());
        let mut sample = units(&vec![1.0; 400]);
        sample.extend(units(&vec![0.0; 100]));
        let j = l.judge_sample("p", &sample);
        assert_eq!(j.word, Word::Pass, "{j}");
        assert!((j.value - 0.8).abs() < 1e-12);
        let far = units(&vec![0.0; 500]);
        assert_eq!(l.judge_sample("p", &far).word, Word::Fail);
    }

    #[test]
    fn a_sample_with_no_misses_still_has_an_interval() {
        let dir = folder(
            &[band("p", r#"{"design": 1.0}"#, 0.1, "scenario", false)],
            None,
        );
        let j = limits(&dir, no_report()).judge_sample("p", &units(&vec![1.0; 100]));
        assert!(j.interval.is_some(), "{j}");
        assert_eq!(j.word, Word::Pass, "{j}");
    }

    #[test]
    fn a_continuous_sample_takes_the_bootstrap_interval() {
        let dir = folder(
            &[band("g", r#"{"design": 2.0}"#, 2.0, "scenario", false)],
            None,
        );
        let values: Vec<f64> = (0..200).map(|i| f64::from(i % 5)).collect();
        let l = limits(&dir, no_report());
        let j = l.judge_sample("g", &units(&values));
        assert_eq!(j.word, Word::Pass, "{j}");
        // The same band and sample give the same interval.
        assert_eq!(l.judge_sample("g", &units(&values)).interval, j.interval);
    }

    #[test]
    fn a_real_band_reads_the_reference_file_then_the_report_then_is_not_sure() {
        let reference =
            r#"{"targets": {"tier.1.x": {"value": 2.0, "se": 0.05, "n": 900, "other": 1}}}"#;
        let dir = folder(
            &[
                band("keyed", r#"{"target": "tier.1.x"}"#, 1.0, "mean", false),
                band("plain", r#"{"real": {}}"#, 1.0, "mean", false),
            ],
            Some(reference),
        );
        let values: Vec<f64> = (0..200).map(|i| f64::from(i % 5)).collect();
        let l = limits(&dir, no_report());
        let keyed = l.judge_sample("keyed", &units(&values));
        assert_eq!(keyed.word, Word::Pass, "{keyed}");
        assert_eq!(keyed.against, "reference tier.1.x");
        let plain = l.judge_sample("plain", &units(&values));
        assert_eq!(plain.word, Word::NotSure);
        assert!(
            plain.reason.as_deref().unwrap().contains("no real value"),
            "{plain}"
        );
        let report = BTreeMap::from([("plain".to_string(), (2.0, 0.05))]);
        let l = limits(&dir, Ok(report));
        let plain = l.judge_sample("plain", &units(&values));
        assert_eq!(plain.word, Word::Pass, "{plain}");
    }

    #[test]
    fn an_unknown_band_and_a_small_sample_are_not_sure() {
        let dir = folder(
            &[band("p", r#"{"design": 0.8}"#, 0.5, "scenario", false)],
            None,
        );
        let l = limits(&dir, no_report());
        let j = l.judge_sample("missing", &units(&[1.0, 0.0]));
        assert_eq!(j.word, Word::NotSure);
        let mut few = units(&vec![1.0; 40]);
        few.extend(units(&vec![0.0; 10]));
        let j = l.judge_sample("p", &few);
        assert_eq!(j.word, Word::NotSure, "{j}");
        assert!(j.reason.as_deref().unwrap().contains("under the minimum"));
    }

    #[test]
    fn a_rare_band_takes_exact_bounds_of_its_events() {
        let dir = folder(
            &[band("r", r#"{"design": 0.05}"#, 0.05, "scenario", true)],
            None,
        );
        let l = limits(&dir, no_report());
        // 8 events over 200 matches: Garwood bounds about 0.017 to 0.079.
        let mut sample = units(&vec![0.0; 192]);
        sample.extend(units(&vec![1.0; 8]));
        let j = l.judge_sample("r", &sample);
        assert_eq!(j.word, Word::Pass, "{j}");
        assert_eq!(j.exposure, 200.0);
        // Zero events still give an upper bound above zero.
        let j = l.judge_sample("r", &units(&vec![0.0; 200]));
        assert!(j.interval.unwrap().hi > -0.05);
    }

    #[test]
    fn the_reference_view_reads_four_figures_and_ignores_the_rest() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("real-targets.json");
        std::fs::write(
            &path,
            r#"{"schema_version": 1, "targets": {"a": {"value": 1.5, "se": 0.1, "n": 7,
                "between_league_sd": 0.2, "label": "x"}}}"#,
        )
        .unwrap();
        let t = reference_targets(&path).unwrap();
        assert_eq!(
            t["a"],
            RefTarget {
                value: Some(1.5),
                se: 0.1,
                n: 7,
                between_league_sd: Some(0.2)
            }
        );
    }

    #[test]
    fn an_earlier_band_file_is_refused() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("realism-bands.json"),
            r#"{"schema_version": 5, "pass_rule": {"alpha": 0.05}, "bands": []}"#,
        )
        .unwrap();
        let err = Limits::load_with(dir.path(), dir.path(), no_report()).unwrap_err();
        assert!(err.contains("schema_version 5"), "{err}");
    }

    #[test]
    fn a_report_gives_its_real_values() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("report.json");
        std::fs::write(
            &path,
            r#"{"calib.real": {"bands": [{"band": "a", "value": 2.5, "se": 0.1},
                {"band": "b", "status": "not_here"}]}}"#,
        )
        .unwrap();
        let values = report_values(&path).unwrap();
        assert_eq!(values.get("a"), Some(&(2.5, 0.1)));
        assert!(!values.contains_key("b"));
    }
}
