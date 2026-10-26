//! The setting rules of `content/sensitivity.json`: each ties one content setting to a band
//! and the way the band moves as the setting rises. A "side effect" note on a surprise writes a
//! draft rule; it counts (plays, and sets an expected move) only once the developer confirms
//! its id in the intent file.
//!
//! calibrate owns these rules: it checks them when it reads the file and writes them by
//! replacing only the text of the `setting_rules` array, so every other byte of the file stays
//! as it was; a write that would change any other key is refused.

use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::intent::{Direction, Intent, NoteKind};
use super::surprises::{Expected, SurpriseKind, Surprises};
use super::touched::{Reason, Touched};
use crate::report::bands::Registry;

/// The rules file's name inside the content folder.
pub const FILE: &str = "sensitivity.json";

/// Whether a setting rule counts yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleStatus {
    Draft,
    Confirmed,
}

/// One setting rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingRule {
    pub id: String,
    /// The setting, `<file>#<pointer>`.
    pub setting: String,
    pub band: String,
    /// The way the band moves as the setting rises (for a choice setting, from its first value
    /// to its second).
    pub direction: Direction,
    /// A number setting's three levels, rising.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub levels: Option<Vec<f64>>,
    /// A choice setting's two values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<Value>>,
    pub status: RuleStatus,
    pub note: String,
    pub from_run: String,
    pub since: String,
}

impl SettingRule {
    /// The rule's level values: its levels, or its choices.
    pub fn level_values(&self) -> Vec<Value> {
        match (&self.levels, &self.values) {
            (Some(l), _) => l.iter().map(|v| Value::from(*v)).collect(),
            (None, Some(v)) => v.clone(),
            (None, None) => Vec::new(),
        }
    }
}

/// The id of the rule of `setting`, `band` and `direction`: `sr-` and the first 8 hex
/// characters of their SHA-256, so the same rule is never written twice.
pub fn rule_id(setting: &str, band: &str, direction: Direction) -> String {
    let mut h = Sha256::new();
    h.update(setting.as_bytes());
    h.update(b"\0");
    h.update(band.as_bytes());
    h.update(b"\0");
    h.update(direction.code().as_bytes());
    format!("sr-{}", &format!("{:x}", h.finalize())[..8])
}

/// Refuses a setting rule list with a repeated id or one not as [`rule_id`] writes it, a
/// setting not named `<file>#/<pointer>`, a band the band file does not hold, levels that are
/// not three and rising, a choice that is not two values, or a blank note; the error names
/// the rule.
pub fn validate(rules: &[SettingRule], registry: &Registry) -> anyhow::Result<()> {
    for (i, r) in rules.iter().enumerate() {
        let at = |what: &str| format!("{FILE}: setting_rules[{i}] ({}): {what}", r.id);
        if rules[..i].iter().any(|o| o.id == r.id) {
            anyhow::bail!(at("the id is used twice"));
        }
        let hex = r.id.strip_prefix("sr-").unwrap_or("");
        if hex.len() != 8
            || !hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            anyhow::bail!(at(
                "an id is sr- and eight hex digits, as calibrate writes it"
            ));
        }
        if !r.setting.contains("#/") {
            anyhow::bail!(at("name the setting as <file>#/<pointer>"));
        }
        if registry.band(&r.band).is_none() {
            anyhow::bail!(at(&format!("{} is not a band of the band file", r.band)));
        }
        match (&r.levels, &r.values) {
            (Some(l), None) => {
                if l.len() != 3
                    || !l.windows(2).all(|w| w[0] < w[1])
                    || l.iter().any(|v| !v.is_finite())
                {
                    anyhow::bail!(at("levels are three numbers, rising"));
                }
            }
            (None, Some(v)) if v.len() == 2 && v[0] != v[1] => {}
            _ => anyhow::bail!(at("give three rising levels, or two values of a choice")),
        }
        if r.note.trim().is_empty() {
            anyhow::bail!(at("the note is blank"));
        }
    }
    Ok(())
}

/// The setting rules of the content folder `content_dir`, checked; none when the file has no
/// `setting_rules`.
pub fn read(content_dir: &Path, registry: &Registry) -> anyhow::Result<Vec<SettingRule>> {
    let path = content_dir.join(FILE);
    let bytes = std::fs::read(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let doc: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("{} is not JSON", path.display()))?;
    let rules: Vec<SettingRule> = match doc.get("setting_rules") {
        None => Vec::new(),
        Some(v) => serde_json::from_value(v.clone())
            .with_context(|| format!("{FILE}: setting_rules is refused"))?,
    };
    validate(&rules, registry)?;
    Ok(rules)
}

/// The confirmed rules, as (id, setting).
pub fn confirmed(rules: &[SettingRule]) -> Vec<(String, String)> {
    rules
        .iter()
        .filter(|r| r.status == RuleStatus::Confirmed)
        .map(|r| (r.id.clone(), r.setting.clone()))
        .collect()
}

/// The moves the confirmed rules `selected` (by id) expect, the way the change moved each
/// setting: the rule's direction when the setting rose, the other way when it fell.
pub fn expected(rules: &[SettingRule], selected: &[String], touched: &Touched) -> Vec<Expected> {
    let flip = |d: Direction| match d {
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
    };
    rules
        .iter()
        .filter(|r| r.status == RuleStatus::Confirmed && selected.contains(&r.id))
        .filter_map(|r| {
            let c = touched.changed.iter().find(|c| c.key == r.setting)?;
            let rose = match (
                c.old.as_ref().and_then(Value::as_f64),
                c.new.as_ref().and_then(Value::as_f64),
            ) {
                (Some(a), Some(b)) => b > a,
                _ => {
                    // A choice: the rule's direction runs from its first value to its second.
                    let values = r.values.as_ref()?;
                    c.new.as_ref() == values.get(1)
                }
            };
            Some(Expected {
                band: r.band.clone(),
                direction: if rose { r.direction } else { flip(r.direction) },
                rule: r.id.clone(),
            })
        })
        .collect()
}

/// What a judgement did with a rule: written, already there, confirmed, or skipped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub band: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
    /// `written`, `already there`, `confirmed`, `already confirmed` or `skipped`.
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

fn skipped(band: &str, setting: Option<&str>, why: &str) -> DraftInfo {
    DraftInfo {
        id: None,
        band: band.to_string(),
        setting: setting.map(str::to_string),
        action: "skipped".to_string(),
        reason: Some(why.to_string()),
    }
}

/// The draft rules the side-effect notes on `surprises` ask for, beside the rules already in
/// `existing`. A note on a change of several settings that names no setting stops here, naming
/// the note and the settings, before anything is written.
pub fn drafts(
    surprises: &Surprises,
    touched: &Touched,
    registry: &Registry,
    existing: &[SettingRule],
    run_id: &str,
    date: &str,
) -> anyhow::Result<(Vec<SettingRule>, Vec<DraftInfo>)> {
    let mut new_rules: Vec<SettingRule> = Vec::new();
    let mut info = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for s in &surprises.rows {
        let Some(note) = s.note.as_ref().filter(|n| n.kind == NoteKind::SideEffect) else {
            continue;
        };
        if seen.contains(&s.band.as_str()) {
            continue;
        }
        seen.push(&s.band);
        let setting = note.setting.as_deref();
        if s.kind == SurpriseKind::Missed {
            info.push(skipped(
                &s.band,
                setting,
                "a missed move is no side effect of a setting",
            ));
            continue;
        }
        if touched.reason != Reason::Content {
            let why = match touched.reason {
                Reason::Code => "the change is engine code; a draft names a content setting",
                Reason::NoBase => "no revision to compare with, so no setting is known",
                _ => "the change touched no setting",
            };
            info.push(skipped(&s.band, setting, why));
            continue;
        }
        if s.distribution || registry.band(&s.band).is_some_and(|b| b.is_distribution()) {
            info.push(skipped(
                &s.band,
                setting,
                "a distribution band moves by distance, with no up or down",
            ));
            continue;
        }
        let key = match setting {
            Some(k) => k.to_string(),
            None if touched.changed.len() == 1 => touched.changed[0].key.clone(),
            None => anyhow::bail!(
                "the side-effect note on {} names no setting, and the change touched {}: {}; \
                 add \"setting\" to the note",
                s.band,
                touched.changed.len(),
                touched
                    .changed
                    .iter()
                    .map(|c| c.key.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        let Some(c) = touched.changed.iter().find(|c| c.key == key) else {
            info.push(skipped(
                &s.band,
                Some(&key),
                "the change did not touch this setting",
            ));
            continue;
        };
        let (Some(old), Some(new)) = (&c.old, &c.new) else {
            info.push(skipped(
                &s.band,
                Some(&key),
                "the setting was added or removed",
            ));
            continue;
        };
        let Some(moved) = s.moved else {
            info.push(skipped(
                &s.band,
                Some(&key),
                "the band's move has no direction",
            ));
            continue;
        };
        let flip = |d: Direction| match d {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        };
        let (direction, levels, values) = match (old.as_f64(), new.as_f64()) {
            (Some(a), Some(b)) => {
                let (lo, hi) = if a < b { (a, b) } else { (b, a) };
                let dir = if b > a { moved } else { flip(moved) };
                (dir, Some(vec![lo, (lo + hi) / 2.0, hi]), None)
            }
            _ => (moved, None, Some(vec![old.clone(), new.clone()])),
        };
        let id = rule_id(&key, &s.band, direction);
        if existing.iter().chain(&new_rules).any(|r| r.id == id) {
            info.push(DraftInfo {
                id: Some(id),
                band: s.band.clone(),
                setting: Some(key),
                action: "already there".to_string(),
                reason: None,
            });
            continue;
        }
        info.push(DraftInfo {
            id: Some(id.clone()),
            band: s.band.clone(),
            setting: Some(key.clone()),
            action: "written".to_string(),
            reason: None,
        });
        new_rules.push(SettingRule {
            id,
            setting: key,
            band: s.band.clone(),
            direction,
            levels,
            values,
            status: RuleStatus::Draft,
            note: note.note.clone(),
            from_run: run_id.to_string(),
            since: date.to_string(),
        });
    }
    Ok((new_rules, info))
}

/// `rules` with each id of `confirm` confirmed; what was done with each id.
pub fn confirm(rules: &mut [SettingRule], confirm: &[String]) -> Vec<DraftInfo> {
    confirm
        .iter()
        .map(|id| match rules.iter_mut().find(|r| &r.id == id) {
            Some(r) => {
                let action = if r.status == RuleStatus::Confirmed {
                    "already confirmed"
                } else {
                    r.status = RuleStatus::Confirmed;
                    "confirmed"
                };
                DraftInfo {
                    id: Some(id.clone()),
                    band: r.band.clone(),
                    setting: Some(r.setting.clone()),
                    action: action.to_string(),
                    reason: None,
                }
            }
            None => DraftInfo {
                id: Some(id.clone()),
                band: String::new(),
                setting: None,
                action: "skipped".to_string(),
                reason: Some("no setting rule has this id".to_string()),
            },
        })
        .collect()
}

/// Writes the draft rules of `intent`'s side-effect notes and its confirmations into the
/// content folder's rules file. Returns the rules now in the file and what was done.
#[allow(clippy::too_many_arguments)] // sdlc-debt: one call site in calibrate; a struct of the eight inputs would only rename them
pub fn write_drafts(
    content_dir: &Path,
    registry: &Registry,
    surprises: &Surprises,
    touched: &Touched,
    intent: &Intent,
    run_id: &str,
    date: &str,
    existing: Vec<SettingRule>,
) -> anyhow::Result<(Vec<SettingRule>, Vec<DraftInfo>)> {
    let (new_rules, mut info) = drafts(surprises, touched, registry, &existing, run_id, date)?;
    let mut rules = existing;
    rules.extend(new_rules);
    info.extend(confirm(&mut rules, &intent.confirm));
    let changed = info
        .iter()
        .any(|d| d.action == "written" || d.action == "confirmed");
    if changed {
        validate(&rules, registry)?;
        write_rules(&content_dir.join(FILE), &rules)?;
    }
    Ok((rules, info))
}

/// The byte range of the top-level `"setting_rules"` array in `text`, its brackets included.
fn array_span(text: &str) -> Option<(usize, usize)> {
    let b = text.as_bytes();
    let (mut depth, mut i) = (0usize, 0usize);
    let mut want_value = false;
    let mut start = None;
    while i < b.len() {
        match b[i] {
            b'"' => {
                let from = i;
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                if depth == 1 && &text[from..=i.min(b.len() - 1)] == "\"setting_rules\"" {
                    let rest = text[i + 1..].trim_start();
                    if rest.starts_with(':') {
                        want_value = true;
                    }
                }
            }
            b'{' | b'[' => {
                if want_value && depth == 1 && b[i] == b'[' && start.is_none() {
                    start = Some((i, depth));
                    want_value = false;
                }
                depth += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                if let Some((s, d)) = start
                    && b[i] == b']'
                    && depth == d
                {
                    return Some((s, i + 1));
                }
            }
            c if want_value && !c.is_ascii_whitespace() && c != b':' => want_value = false,
            _ => {}
        }
        i += 1;
    }
    None
}

/// The array text of `rules`, indented as the shipped file's top-level keys.
fn array_text(rules: &[SettingRule]) -> anyhow::Result<String> {
    if rules.is_empty() {
        return Ok("[]".to_string());
    }
    let mut out = String::from("[\n");
    for (i, r) in rules.iter().enumerate() {
        let one = serde_json::to_string_pretty(r)?;
        for (k, line) in one.lines().enumerate() {
            if k > 0 {
                out.push('\n');
            }
            out.push_str("    ");
            out.push_str(line);
        }
        out.push_str(if i + 1 < rules.len() { ",\n" } else { "\n" });
    }
    out.push_str("  ]");
    Ok(out)
}

/// Replaces the `setting_rules` array of the rules file at `path` with `rules`, or adds it as
/// the last key; refuses the write when any other key would change.
pub fn write_rules(path: &Path, rules: &[SettingRule]) -> anyhow::Result<()> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let before: Value = serde_json::from_str(&text)?;
    let array = array_text(rules)?;
    let out = match array_span(&text) {
        Some((s, e)) => format!("{}{array}{}", &text[..s], &text[e..]),
        None => {
            let end = text
                .rfind('}')
                .with_context(|| format!("{} is not a JSON object", path.display()))?;
            let head = text[..end].trim_end();
            let sep = if head.ends_with('{') { "" } else { "," };
            format!("{head}{sep}\n  \"setting_rules\": {array}\n}}\n")
        }
    };
    let after: Value = serde_json::from_str(&out)
        .with_context(|| format!("the new {} would not be JSON", path.display()))?;
    let others = |v: &Value| {
        let mut v = v.clone();
        if let Some(o) = v.as_object_mut() {
            o.remove("setting_rules");
        }
        v
    };
    if others(&before) != others(&after) || after["setting_rules"] != serde_json::to_value(rules)? {
        anyhow::bail!(
            "the setting rules of {} were not written: another key would change",
            path.display()
        );
    }
    std::fs::write(path, out).with_context(|| format!("cannot write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibrate::intent::Note;
    use crate::calibrate::surprises::Surprise;
    use crate::calibrate::touched::Changed;
    use serde_json::json;

    fn registry() -> Registry {
        Registry::load(&engine::ContentDir::at(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        ))
        .unwrap()
    }

    fn content() -> tempfile::TempDir {
        let tmp = tempfile::TempDir::new().unwrap();
        let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        std::fs::copy(shipped.join(FILE), tmp.path().join(FILE)).unwrap();
        tmp
    }

    fn surprise(
        band: &str,
        kind: SurpriseKind,
        moved: Direction,
        setting: Option<&str>,
    ) -> Surprise {
        Surprise {
            band: band.into(),
            suite: "equal".into(),
            pairing: None,
            kind,
            old_word: "worse".into(),
            moved: Some(moved),
            intended: None,
            size: None,
            expected_by: None,
            change: Some(0.3),
            distribution: false,
            note: Some(Note {
                band: band.into(),
                kind: NoteKind::SideEffect,
                note: "keepers save less".into(),
                setting: setting.map(str::to_string),
            }),
        }
    }

    fn touched(changed: Vec<(&str, Value, Value)>) -> Touched {
        let mut t = Touched::every(Reason::Content, Some("abc".into()), None, Vec::new(), &[]);
        t.all = false;
        t.settings = changed.iter().map(|c| c.0.to_string()).collect();
        t.changed = changed
            .into_iter()
            .map(|(k, o, n)| Changed {
                key: k.into(),
                old: Some(o),
                new: Some(n),
            })
            .collect();
        t
    }

    fn surprises(rows: Vec<Surprise>) -> Surprises {
        Surprises {
            rows,
            ..Surprises::default()
        }
    }

    const SAVE: &str = "tuning.json#/engine/shots/save_high";

    #[test]
    fn the_splice_changes_only_the_setting_rules() {
        let dir = content();
        let path = dir.path().join(FILE);
        let before = std::fs::read_to_string(&path).unwrap();
        let t = touched(vec![(SAVE, json!(0.891), json!(0.0))]);
        let s = surprises(vec![surprise(
            "goals_per_match",
            SurpriseKind::Unintended,
            Direction::Up,
            None,
        )]);
        let (rules, info) = write_drafts(
            dir.path(),
            &registry(),
            &s,
            &t,
            &Intent::empty(),
            "calib-1",
            "2026-10-23",
            Vec::new(),
        )
        .unwrap();
        assert_eq!(info[0].action, "written");
        assert_eq!(rules.len(), 1);
        let r = &rules[0];
        assert_eq!(
            r.direction,
            Direction::Down,
            "the band rose as the setting fell"
        );
        assert_eq!(r.levels.as_deref(), Some(&[0.0, 0.4455, 0.891][..]));
        assert_eq!(r.status, RuleStatus::Draft);
        assert_eq!(r.id, rule_id(SAVE, "goals_per_match", Direction::Down));
        let after = std::fs::read_to_string(&path).unwrap();
        let (s0, e0) = array_span(&before).unwrap();
        let (s1, e1) = array_span(&after).unwrap();
        assert_eq!(&before[..s0], &after[..s1]);
        assert_eq!(&before[e0..], &after[e1..]);
        assert_eq!(read(dir.path(), &registry()).unwrap(), rules);
        // A second write of the same draft adds nothing.
        let (again, info) = write_drafts(
            dir.path(),
            &registry(),
            &s,
            &t,
            &Intent::empty(),
            "calib-1",
            "2026-10-23",
            rules.clone(),
        )
        .unwrap();
        assert_eq!(again, rules);
        assert_eq!(info[0].action, "already there");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), after);
    }

    #[test]
    fn confirmation_flips_the_status_only() {
        let dir = content();
        let t = touched(vec![(SAVE, json!(0.891), json!(0.0))]);
        let s = surprises(vec![surprise(
            "goals_per_match",
            SurpriseKind::Unintended,
            Direction::Up,
            None,
        )]);
        let (rules, _) = write_drafts(
            dir.path(),
            &registry(),
            &s,
            &t,
            &Intent::empty(),
            "calib-1",
            "2026-10-23",
            Vec::new(),
        )
        .unwrap();
        let id = rules[0].id.clone();
        let mut i = Intent::empty();
        i.confirm = vec![id.clone(), "sr-ffffffff".into()];
        let (after, info) = write_drafts(
            dir.path(),
            &registry(),
            &surprises(Vec::new()),
            &t,
            &i,
            "calib-2",
            "2026-10-24",
            rules.clone(),
        )
        .unwrap();
        assert_eq!(after[0].status, RuleStatus::Confirmed);
        assert_eq!(
            SettingRule {
                status: RuleStatus::Draft,
                ..after[0].clone()
            },
            rules[0]
        );
        assert_eq!(info[0].action, "confirmed");
        assert_eq!(info[1].action, "skipped");
        assert_eq!(confirmed(&after), [(id.clone(), SAVE.to_string())]);
        let exp = expected(&after, std::slice::from_ref(&id), &t);
        assert_eq!(
            exp[0].direction,
            Direction::Up,
            "the setting fell, so the band rises"
        );
        assert!(
            expected(&rules, &[id], &t).is_empty(),
            "a draft sets no expected move"
        );
    }

    #[test]
    fn a_choice_setting_gets_its_two_values() {
        let t = touched(vec![(
            "slots.json#/slots/engine.shot/module",
            json!("shot"),
            json!("shot2"),
        )]);
        let s = surprises(vec![surprise(
            "goals_per_match",
            SurpriseKind::Unintended,
            Direction::Up,
            None,
        )]);
        let (rules, _) = drafts(&s, &t, &registry(), &[], "r", "d").unwrap();
        assert_eq!(rules[0].values, Some(vec![json!("shot"), json!("shot2")]));
        assert_eq!(rules[0].levels, None);
        assert_eq!(rules[0].direction, Direction::Up);
    }

    #[test]
    fn a_note_on_a_change_of_two_settings_without_a_setting_stops_naming_both() {
        let t = touched(vec![
            (SAVE, json!(0.891), json!(0.0)),
            ("tuning.json#/engine/foul_base", json!(0.1), json!(0.2)),
        ]);
        let s = surprises(vec![surprise(
            "goals_per_match",
            SurpriseKind::Unintended,
            Direction::Up,
            None,
        )]);
        let e = format!(
            "{:#}",
            drafts(&s, &t, &registry(), &[], "r", "d").unwrap_err()
        );
        assert!(e.contains(SAVE) && e.contains("foul_base"), "{e}");
        let s = surprises(vec![surprise(
            "goals_per_match",
            SurpriseKind::Unintended,
            Direction::Up,
            Some(SAVE),
        )]);
        let (rules, _) = drafts(&s, &t, &registry(), &[], "r", "d").unwrap();
        assert_eq!(rules[0].setting, SAVE);
    }

    #[test]
    fn a_code_change_or_a_distribution_writes_no_draft_and_says_why() {
        let s = surprises(vec![surprise(
            "goals_per_match",
            SurpriseKind::Unintended,
            Direction::Up,
            None,
        )]);
        let code = Touched::every(Reason::Code, None, None, vec!["crates/x.rs".into()], &[]);
        let (rules, info) = drafts(&s, &code, &registry(), &[], "r", "d").unwrap();
        assert!(rules.is_empty());
        assert!(info[0].reason.as_deref().unwrap().contains("engine code"));
        let mut dist = surprise(
            "goals_per_match",
            SurpriseKind::Unintended,
            Direction::Up,
            None,
        );
        dist.distribution = true;
        let t = touched(vec![(SAVE, json!(0.891), json!(0.0))]);
        let (rules, info) = drafts(&surprises(vec![dist]), &t, &registry(), &[], "r", "d").unwrap();
        assert!(rules.is_empty());
        assert!(info[0].reason.as_deref().unwrap().contains("distribution"));
    }

    #[test]
    fn a_rule_list_is_checked_when_read() {
        let r = SettingRule {
            id: "sr-00000001".into(),
            setting: SAVE.into(),
            band: "goals_per_match".into(),
            direction: Direction::Up,
            levels: Some(vec![0.0, 0.5, 1.0]),
            values: None,
            status: RuleStatus::Draft,
            note: "n".into(),
            from_run: "r".into(),
            since: "d".into(),
        };
        validate(std::slice::from_ref(&r), &registry()).unwrap();
        let bad = |edit: &dyn Fn(&mut SettingRule)| {
            let mut x = r.clone();
            edit(&mut x);
            validate(&[x], &registry()).is_err()
        };
        assert!(bad(&|x| x.levels = Some(vec![1.0, 0.5, 0.0])));
        assert!(bad(&|x| x.band = "nope".into()));
        assert!(bad(&|x| x.setting = "foul".into()));
        assert!(bad(&|x| x.note = " ".into()));
        assert!(bad(&|x| x.id = "sr-/../x".into()));
        assert!(bad(&|x| x.id = "sr-a:b".into()));
        assert!(bad(&|x| x.id = "sr-0000000G".into()));
        assert!(validate(&[r.clone(), r], &registry()).is_err());
    }

    #[test]
    fn a_file_without_the_array_gains_it_as_its_last_key() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(FILE);
        std::fs::write(&path, "{\n  \"a\": {\"setting_rules\": [1]}\n}\n").unwrap();
        write_rules(&path, &[]).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["a"]["setting_rules"], json!([1]));
        assert_eq!(v["setting_rules"], json!([]));
    }
}
