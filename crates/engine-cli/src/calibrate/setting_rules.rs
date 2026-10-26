//! The confirmed setting rules of a change run. A rule ties one content setting to a band and
//! the way the band moves as the setting rises; a rule whose setting the change touched (or
//! every confirmed rule, on a change to engine code) plays its levels: for each level a copy of
//! the run's content with that one setting set, on the first fixtures of the run's equal suite,
//! into stored rows under `rules/<id>/l<k>/` of the run folder. The band's value at each level
//! comes from those rows through the same measures the verdict reads; the fixtures are the same
//! at every level, so a paired bootstrap over them gives the change from the lowest level to the
//! highest and the two steps between them. The rule passes when that change lies wholly in its
//! direction and no step surely turns back, fails when it lies wholly against it or a step
//! surely turns back, and is not sure otherwise. Setting rules play in any build; a band that
//! reads the measures has no value in a build without them and reads not sure.

use std::path::{Path, PathBuf};

use anyhow::Context;
use engine::rng::EngineRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::drafts::{RuleStatus, SettingRule};
use super::intent::Direction;
use crate::report::RuleInterval;

/// Resamples of a setting rule's paired bootstrap.
pub const RESAMPLES: usize = 999;
/// Mixed into the run seed for a setting rule's resampling stream.
const STREAM_LABEL: u64 = 0x7365_7474_696e_6773;

/// Matches each level of a setting rule plays: the rules file's `design.setting_matches`,
/// 200 when it does not say.
pub fn setting_matches(content_dir: &Path) -> u32 {
    std::fs::read(content_dir.join(super::drafts::FILE))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["design"]["setting_matches"].as_u64())
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(200)
        .max(2)
}

/// The folder of level `k` of rule `id` inside the run folder.
pub fn level_dir(run_dir: &Path, id: &str, k: usize) -> PathBuf {
    run_dir.join("rules").join(id).join(format!("l{k}"))
}

/// One setting rule's row of the rules stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingRuleRow {
    pub id: String,
    pub setting: String,
    pub band: String,
    pub direction: Direction,
    /// The setting's value at each level.
    pub levels: Vec<Value>,
    /// The band's value at each level; none at a level with no value.
    pub values: Vec<Option<f64>>,
    /// Matches each level played.
    pub matches: u32,
    /// The band's change from the lowest level to the highest, in the rule's direction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<RuleInterval>,
    /// The two steps between three levels, in the rule's direction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<RuleInterval>>,
    /// `pass`, `fail`, `not_sure` or `not_played`.
    pub word: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The note of a rule confirmed on a run judged again: its levels were never played here.
pub const CONFIRMED_LATE: &str =
    "confirmed after this run was played; plays in the next change run";

impl SettingRuleRow {
    /// The row of a rule that did not play in this run, with `note`.
    pub fn not_played(rule: &SettingRule, note: &str) -> Self {
        SettingRuleRow {
            id: rule.id.clone(),
            setting: rule.setting.clone(),
            band: rule.band.clone(),
            direction: rule.direction,
            levels: rule.level_values(),
            values: Vec::new(),
            matches: 0,
            change: None,
            steps: None,
            word: "not_played".to_string(),
            note: Some(note.to_string()),
        }
    }
}

/// `true` for a rule that plays in this run: confirmed and selected.
pub fn plays(rule: &SettingRule, selected: &[String]) -> bool {
    rule.status == RuleStatus::Confirmed && selected.contains(&rule.id)
}

/// One part of a setting's pointer: an object key, or an array item by its key.
fn segments(pointer: &str) -> Vec<(bool, String)> {
    let mut out = Vec::new();
    let mut rest = pointer;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix('/') {
            let end = r.find(['/', '[']).unwrap_or(r.len());
            out.push((false, r[..end].replace("~1", "/").replace("~0", "~")));
            rest = &r[end..];
        } else if let Some(r) = rest.strip_prefix('[') {
            let end = r.find(']').unwrap_or(r.len());
            out.push((true, r[..end].to_string()));
            rest = r.get(end + 1..).unwrap_or("");
        } else {
            break;
        }
    }
    out
}

/// Sets the setting `pointer` (with keyed array items, as the touched settings name them) of
/// `doc` to `value`.
pub fn set_key(doc: &mut Value, pointer: &str, value: Value) -> anyhow::Result<()> {
    // Each keyed item becomes its index, so the plain JSON pointer reaches the setting.
    let mut plain = String::new();
    let mut at: &Value = doc;
    for (item, key) in segments(pointer) {
        let (next, part) = if item {
            let (i, x) = at
                .as_array()
                .and_then(|a| {
                    a.iter().enumerate().find(|(_, x)| {
                        ["name", "job", "id"]
                            .iter()
                            .any(|k| x.get(*k).and_then(Value::as_str) == Some(key.as_str()))
                    })
                })
                .with_context(|| format!("{pointer}: no item [{key}]"))?;
            (x, i.to_string())
        } else {
            let x = match at {
                Value::Array(a) => key.parse::<usize>().ok().and_then(|i| a.get(i)),
                Value::Object(o) => o.get(&key),
                _ => None,
            }
            .with_context(|| format!("{pointer}: no key {key}"))?;
            (x, key.replace('~', "~0").replace('/', "~1"))
        };
        plain.push('/');
        plain.push_str(&part);
        at = next;
    }
    *doc.pointer_mut(&plain)
        .with_context(|| format!("{pointer}: not found"))? = value;
    Ok(())
}

/// Copies the content folder `from` to `to` with the setting `setting` (`<file>#<pointer>`)
/// set to `value`.
pub fn patched_content(from: &Path, to: &Path, setting: &str, value: &Value) -> anyhow::Result<()> {
    fn copy(from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(to)?;
        for e in std::fs::read_dir(from)? {
            let e = e?;
            let t = to.join(e.file_name());
            if e.file_type()?.is_dir() {
                copy(&e.path(), &t)?;
            } else {
                std::fs::copy(e.path(), t)?;
            }
        }
        Ok(())
    }
    let (file, pointer) = setting
        .split_once('#')
        .with_context(|| format!("{setting}: name it as <file>#<pointer>"))?;
    let inside = !file.is_empty()
        && Path::new(file)
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)));
    anyhow::ensure!(
        inside,
        "{setting}: the file must be a path inside the content folder"
    );
    let _ = std::fs::remove_dir_all(to);
    copy(from, to).with_context(|| format!("cannot copy the content to {}", to.display()))?;
    let path = to.join(file);
    let mut doc: Value = serde_json::from_slice(
        &std::fs::read(&path).with_context(|| format!("cannot read {}", path.display()))?,
    )
    .with_context(|| format!("{} is not JSON", path.display()))?;
    set_key(&mut doc, pointer, value.clone())?;
    std::fs::write(&path, serde_json::to_string_pretty(&doc)?)
        .with_context(|| format!("cannot write {}", path.display()))
}

/// The value of per-match sums `t` under `weights`: `(Σ w a / Σ w b) · (Σ w d / Σ w c)`, as
/// the verdict reads a band; `None` when a denominator is 0.
fn weighted(t: &[Vec<f64>; 4], weights: &[f64]) -> Option<f64> {
    let mut s = [0.0f64; 4];
    for (i, &w) in weights.iter().enumerate() {
        if w == 0.0 {
            continue;
        }
        for (k, sum) in s.iter_mut().enumerate() {
            *sum += w * t[k][i];
        }
    }
    let [a, b, c, d] = s;
    (b > 0.0 && c > 0.0).then(|| a / b * (d / c))
}

/// A rule's judgement: the band's value per level, the change from the first level to the
/// last, the steps between levels, and the word.
pub type Judged = (
    Vec<Option<f64>>,
    Option<RuleInterval>,
    Option<Vec<RuleInterval>>,
    &'static str,
);

/// The judgement of a rule from the per-match sums of its band at each level (the same
/// fixtures, in the same order, at every level): the band's value per level, the change from
/// the first level to the last and the steps between them (each signed by `direction`, the
/// way the band should move), and the word.
pub fn judge(per_level: &[[Vec<f64>; 4]], direction: Direction, seed: u64) -> Judged {
    let n = per_level.first().map_or(0, |t| t[0].len());
    let ones = vec![1.0; n];
    let values: Vec<Option<f64>> = per_level.iter().map(|t| weighted(t, &ones)).collect();
    if n < 2 || per_level.len() < 2 || values.iter().any(Option::is_none) {
        return (values, None, None, "not_sure");
    }
    let sign = match direction {
        Direction::Up => 1.0,
        Direction::Down => -1.0,
    };
    let last = per_level.len() - 1;
    let moves = |v: &[f64]| -> Vec<f64> {
        let mut out = vec![sign * (v[last] - v[0])];
        if per_level.len() > 2 {
            out.extend(v.windows(2).map(|w| sign * (w[1] - w[0])));
        }
        out
    };
    let point: Vec<f64> = values.iter().map(|v| v.unwrap_or(0.0)).collect();
    let point = moves(&point);
    let mut rng = EngineRng::from_seed(seed ^ STREAM_LABEL);
    let mut draws: Vec<Vec<f64>> = vec![Vec::with_capacity(RESAMPLES); point.len()];
    for _ in 0..RESAMPLES {
        let mut w = vec![0.0; n];
        for _ in 0..n {
            w[rng.range_usize(n)] += 1.0;
        }
        let v: Option<Vec<f64>> = per_level.iter().map(|t| weighted(t, &w)).collect();
        let Some(v) = v else { continue };
        for (k, m) in moves(&v).into_iter().enumerate() {
            draws[k].push(m);
        }
    }
    let intervals: Vec<RuleInterval> = draws
        .into_iter()
        .zip(&point)
        .map(|(mut d, &est)| {
            d.sort_by(f64::total_cmp);
            RuleInterval {
                est,
                lo: pass_rule::interval::quantile_rank(&d, 0.025),
                hi: pass_rule::interval::quantile_rank(&d, 0.975),
            }
        })
        .collect();
    let change = intervals[0];
    let steps = intervals[1..].to_vec();
    let reversed = steps.iter().any(|s| s.hi < 0.0);
    let word = if change.hi < 0.0 || reversed {
        "fail"
    } else if change.lo > 0.0 {
        "pass"
    } else {
        "not_sure"
    };
    (
        values,
        Some(change),
        (!steps.is_empty()).then_some(steps),
        word,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Per-match sums of a mean band at each level: `n` matches around `means[k]`, the same
    /// fixtures at every level (a shared noise per match plus a small one per level).
    fn levels(means: &[f64], n: usize, noise: f64) -> Vec<[Vec<f64>; 4]> {
        let mut rng = EngineRng::from_seed(5);
        let shared: Vec<f64> = (0..n).map(|_| rng.range_f64(-noise, noise)).collect();
        means
            .iter()
            .map(|m| {
                let a: Vec<f64> = shared
                    .iter()
                    .map(|s| m + s + rng.range_f64(-noise / 4.0, noise / 4.0))
                    .collect();
                [a, vec![1.0; n], vec![1.0; n], vec![1.0; n]]
            })
            .collect()
    }

    #[test]
    fn a_rise_passes_a_reversed_step_fails_and_flat_is_not_sure() {
        let (v, change, steps, word) = judge(&levels(&[2.0, 2.5, 3.0], 200, 1.0), Direction::Up, 1);
        assert_eq!(word, "pass", "{change:?} {steps:?}");
        assert!((v[1].unwrap() - 2.5).abs() < 0.2);
        assert_eq!(steps.unwrap().len(), 2);
        let (_, _, steps, word) = judge(&levels(&[2.0, 3.5, 3.0], 200, 1.0), Direction::Up, 1);
        assert!(steps.unwrap()[1].hi < 0.0);
        assert_eq!(word, "fail");
        let (_, change, _, word) = judge(&levels(&[2.0, 2.0, 2.0], 200, 1.0), Direction::Up, 1);
        assert!(change.unwrap().lo < 0.0 && change.unwrap().hi > 0.0);
        assert_eq!(word, "not_sure");
        let (_, _, _, word) = judge(&levels(&[3.0, 2.5, 2.0], 200, 1.0), Direction::Down, 1);
        assert_eq!(word, "pass");
        let (_, _, _, word) = judge(&levels(&[3.0, 2.0], 200, 1.0), Direction::Up, 1);
        assert_eq!(word, "fail");
    }

    #[test]
    fn a_level_with_no_value_is_not_sure() {
        let mut l = levels(&[2.0, 2.5, 3.0], 20, 1.0);
        l[1][1] = vec![0.0; 20];
        let (v, change, _, word) = judge(&l, Direction::Up, 1);
        assert_eq!(v[1], None);
        assert!(change.is_none());
        assert_eq!(word, "not_sure");
    }

    #[test]
    fn a_setting_is_set_by_its_key() {
        let mut doc = json!({"engine": {"shots": {"save_high": 0.891}},
            "attributes": [{"name": "passing", "group": "technical"}], "list": [1, 2]});
        set_key(&mut doc, "/engine/shots/save_high", json!(0.0)).unwrap();
        set_key(&mut doc, "/attributes[passing]/group", json!("mental")).unwrap();
        set_key(&mut doc, "/list/1", json!(5)).unwrap();
        assert_eq!(doc["engine"]["shots"]["save_high"], json!(0.0));
        assert_eq!(doc["attributes"][0]["group"], json!("mental"));
        assert_eq!(doc["list"], json!([1, 5]));
        assert!(set_key(&mut doc, "/attributes[nope]/group", json!(1)).is_err());
    }

    #[test]
    fn a_patched_copy_changes_one_setting() {
        let tmp = tempfile::TempDir::new().unwrap();
        let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let to = tmp.path().join("c");
        patched_content(
            &shipped,
            &to,
            "tuning.json#/engine/shots/save_high",
            &json!(0.5),
        )
        .unwrap();
        let v: Value =
            serde_json::from_slice(&std::fs::read(to.join("tuning.json")).unwrap()).unwrap();
        assert_eq!(v["engine"]["shots"]["save_high"], json!(0.5));
        let changed = super::super::touched::diff_dirs(&shipped, &to).unwrap();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].key, "tuning.json#/engine/shots/save_high");
    }

    #[test]
    fn a_setting_whose_file_leaves_the_content_folder_is_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        let from = tmp.path().join("from");
        std::fs::create_dir_all(&from).unwrap();
        let to = tmp.path().join("c");
        for setting in ["../x.json#/a", "#/a", "/x.json#/a"] {
            assert!(
                patched_content(&from, &to, setting, &json!(1)).is_err(),
                "{setting}"
            );
        }
        assert!(!to.exists());
    }
}
