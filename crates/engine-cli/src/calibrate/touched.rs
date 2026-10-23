//! The settings a change touched, and the sensitivity rules they select.
//!
//! Each JSON file of the content folder is flattened to its leaf keys, `<file>#<pointer>`
//! (`tuning.json#/engine/foul_base`), with each array of objects keyed by its items' `name`,
//! `job` or `id` (`attributes.json#/attributes[passing]/group`), so an inserted item changes
//! only its own keys. A key added, removed or changed against the base revision's content is a
//! changed setting; a file that is not JSON is one setting. The setting rules of the rules
//! file (`sensitivity.json#/setting_rules`) are its bookkeeping and never a changed setting.
//!
//! A change to engine code (a path under `crates/`, or the Cargo or toolchain files) selects
//! every rule, and so does a run with no revision to compare with. Otherwise each changed
//! setting selects the rules the table in [`rules_of`] names; a setting no rule names (a
//! module in `slots.json`, a band of the band file, a club) selects none and is listed as
//! unruled.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The five-match rule's id among the rules.
pub const FIVE_MATCH: &str = "five-match";

/// The paths of a checkout whose change is a change to engine code.
pub fn is_code(path: &str) -> bool {
    path.starts_with("crates/")
        || matches!(path, "Cargo.toml" | "Cargo.lock" | "rust-toolchain.toml")
}

/// Every leaf key of the JSON `value` of `file`, with its value.
pub fn flatten(file: &str, value: &Value) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    walk(&format!("{file}#"), value, &mut out);
    out
}

/// The key of an array item that names it: its `name`, `job` or `id`.
fn item_key(v: &Value) -> Option<&str> {
    ["name", "job", "id"]
        .iter()
        .find_map(|k| v.get(*k).and_then(Value::as_str))
}

fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn walk(prefix: &str, value: &Value, out: &mut BTreeMap<String, Value>) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (k, v) in map {
                walk(&format!("{prefix}/{}", escape(k)), v, out);
            }
        }
        Value::Array(items) if !items.is_empty() => {
            let keys: Vec<Option<&str>> = items.iter().map(item_key).collect();
            let unique: BTreeSet<&str> = keys.iter().flatten().copied().collect();
            let keyed = keys.iter().all(Option::is_some) && unique.len() == items.len();
            for (i, v) in items.iter().enumerate() {
                let segment = match keys[i] {
                    Some(k) if keyed => format!("[{k}]"),
                    _ => format!("/{i}"),
                };
                walk(&format!("{prefix}{segment}"), v, out);
            }
        }
        leaf => {
            out.insert(prefix.to_string(), leaf.clone());
        }
    }
}

/// `true` for a key of the rules file's own bookkeeping: its setting rules.
fn bookkeeping(key: &str) -> bool {
    key.starts_with("sensitivity.json#/setting_rules")
}

/// One changed setting: its key, and its value before and after (none for a key added or
/// removed, or a file that is not JSON).
#[derive(Debug, Clone, PartialEq)]
pub struct Changed {
    pub key: String,
    pub old: Option<Value>,
    pub new: Option<Value>,
}

/// Every file of `dir`, by its path inside it with `/` separators.
fn files(dir: &Path) -> anyhow::Result<BTreeMap<String, std::path::PathBuf>> {
    fn go(
        root: &Path,
        dir: &Path,
        out: &mut BTreeMap<String, std::path::PathBuf>,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                go(root, &path, out)?;
            } else {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                let rel = rel.to_string_lossy().replace('\\', "/");
                if rel != ".complete" {
                    out.insert(rel, path);
                }
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    go(dir, dir, &mut out).with_context(|| format!("cannot read {}", dir.display()))?;
    Ok(out)
}

/// A file's settings: its leaf keys when it is JSON, else its text (line ends made alike) as
/// one setting.
fn settings_of(name: &str, path: Option<&Path>) -> anyhow::Result<BTreeMap<String, Value>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let bytes = std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    if name.ends_with(".json")
        && let Ok(v) = serde_json::from_slice::<Value>(&bytes)
    {
        return Ok(flatten(name, &v));
    }
    let text = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
    Ok(BTreeMap::from([(name.to_string(), Value::String(text))]))
}

/// The settings that differ between the content folders `old` and `new`.
pub fn diff_dirs(old: &Path, new: &Path) -> anyhow::Result<Vec<Changed>> {
    let (a, b) = (files(old)?, files(new)?);
    let names: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    let mut out = Vec::new();
    for name in names {
        let before = settings_of(name, a.get(name).map(|p| p.as_path()))?;
        let after = settings_of(name, b.get(name).map(|p| p.as_path()))?;
        let keys: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
        for key in keys {
            let (o, n) = (before.get(key), after.get(key));
            if o == n || bookkeeping(key) {
                continue;
            }
            let file_only = !key.contains('#');
            out.push(Changed {
                key: key.clone(),
                old: o.filter(|_| !file_only).cloned(),
                new: n.filter(|_| !file_only).cloned(),
            });
        }
    }
    Ok(out)
}

/// What the rules table reads from the content: every rule's job, and the attributes each
/// stage table names, by action and stage.
#[derive(Debug, Clone, Default)]
pub struct RuleIndex {
    pub jobs: Vec<String>,
    pub stages: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
}

impl RuleIndex {
    /// The index of the content folders `dirs`: the rules of the first one that has a rules
    /// file, and the stage tables of all of them together.
    pub fn of(dirs: &[&Path]) -> RuleIndex {
        let read = |dir: &Path, file: &str| -> Option<Value> {
            serde_json::from_slice(&std::fs::read(dir.join(file)).ok()?).ok()
        };
        let mut index = RuleIndex::default();
        for dir in dirs {
            if index.jobs.is_empty()
                && let Some(rules) = read(dir, "sensitivity.json")
            {
                index.jobs = rules["rules"]
                    .as_array()
                    .map(|r| {
                        r.iter()
                            .filter_map(|r| r["job"].as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
            }
            let Some(attrs) = read(dir, "attributes.json") else {
                continue;
            };
            let Some(actions) = attrs["actions"].as_object() else {
                continue;
            };
            for (action, stages) in actions {
                for (stage, table) in stages.as_object().into_iter().flatten() {
                    let named = index
                        .stages
                        .entry(action.clone())
                        .or_default()
                        .entry(stage.clone())
                        .or_default();
                    let main = std::iter::once(&table["main"]);
                    let supports = table["supports"].as_array().into_iter().flatten();
                    for part in main.chain(supports) {
                        if let Some(a) = part["attribute"].as_str() {
                            named.insert(a.to_string());
                        }
                    }
                }
            }
        }
        index
    }

    fn has(&self, job: &str) -> bool {
        self.jobs.iter().any(|j| j == job)
    }

    /// The rules of one action's stage, or of every stage of it when `stage` is none: each
    /// attribute the table names that has a rule, and the stage rule `<action>.<stage>`.
    fn stage_rules(&self, action: &str, stage: Option<&str>) -> Vec<String> {
        let mut out = BTreeSet::new();
        for (s, named) in self.stages.get(action).into_iter().flatten() {
            if stage.is_some_and(|x| x != s) {
                continue;
            }
            out.extend(named.iter().filter(|a| self.has(a)).cloned());
            let rule = format!("{action}.{s}");
            if self.has(&rule) {
                out.insert(rule);
            }
        }
        out.into_iter().collect()
    }

    /// Every job rule: each rule of the rules file except the five-match rule.
    fn every_job(&self) -> Vec<String> {
        self.jobs.clone()
    }
}

/// One part of a key's pointer: an object key, or an array item by its key or index.
#[derive(Debug, Clone, PartialEq)]
enum Seg {
    Key(String),
    Item(String),
}

/// The file and the parts of a key.
fn parse_key(key: &str) -> (&str, Vec<Seg>) {
    let Some((file, pointer)) = key.split_once('#') else {
        return (key, Vec::new());
    };
    let mut out = Vec::new();
    let mut rest = pointer;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix('/') {
            let end = r.find(['/', '[']).unwrap_or(r.len());
            out.push(Seg::Key(r[..end].replace("~1", "/").replace("~0", "~")));
            rest = &r[end..];
        } else if let Some(r) = rest.strip_prefix('[') {
            let end = r.find(']').unwrap_or(r.len());
            out.push(Seg::Item(r[..end].to_string()));
            rest = r.get(end + 1..).unwrap_or("");
        } else {
            break;
        }
    }
    (file, out)
}

/// The rules that a changed setting `key` selects, by the table below; none for a setting no
/// rule names.
///
/// - `attributes.json#/attributes[X]/...`: rule `X`;
/// - `attributes.json#/actions/<a>/<s>/...` and `tuning.json#/engine/contract/actions/<a>/...`:
///   the rules of every attribute the stage tables name, and the stage rules `<a>.<s>`;
/// - `tuning.json#/engine/contract/curve/...` and `tuning.json#/match_rating/...`: the
///   five-match rule;
/// - `tuning.json#/engine/contract/body/jobs/<j>/...`: body rule `j`; the body blocks
///   `height`, `age` and `adaptation`: the rules `height`, `age` and `nationality`;
/// - `sensitivity.json#/rules[X]/...`: rule `X`; `#/design/...` and `#/defaults/...`: every
///   job rule; `#/five_match/...`: the five-match rule.
pub fn rules_of(key: &str, index: &RuleIndex) -> Vec<String> {
    use Seg::{Item, Key};
    let (file, segs) = parse_key(key);
    let k = |i: usize| match segs.get(i) {
        Some(Key(s)) => Some(s.as_str()),
        _ => None,
    };
    let item = |i: usize| match segs.get(i) {
        Some(Item(s)) => Some(s.as_str()),
        _ => None,
    };
    let only_if = |job: &str| {
        if index.has(job) {
            vec![job.to_string()]
        } else {
            Vec::new()
        }
    };
    match file {
        "attributes.json" => match (k(0), item(1), k(1)) {
            (Some("attributes"), Some(x), _) => only_if(x),
            (Some("actions"), _, Some(a)) => index.stage_rules(a, k(2)),
            _ => Vec::new(),
        },
        "tuning.json" => match (k(0), k(1), k(2), k(3)) {
            (Some("engine"), Some("contract"), Some("actions"), Some(a)) => {
                index.stage_rules(a, None)
            }
            (Some("engine"), Some("contract"), Some("curve"), _) | (Some("match_rating"), ..) => {
                vec![FIVE_MATCH.to_string()]
            }
            (Some("engine"), Some("contract"), Some("body"), Some(part)) => match part {
                "jobs" => k(4).map(only_if).unwrap_or_default(),
                "height" => only_if("height"),
                "age" => only_if("age"),
                "adaptation" => only_if("nationality"),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        },
        "sensitivity.json" => match (k(0), item(1)) {
            (Some("rules"), Some(x)) => only_if(x),
            (Some("design" | "defaults"), _) => index.every_job(),
            (Some("five_match"), _) => vec![FIVE_MATCH.to_string()],
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// `true` when `key` is the setting `setting` or a key under it.
pub fn under(key: &str, setting: &str) -> bool {
    key == setting
        || key
            .strip_prefix(setting)
            .is_some_and(|rest| rest.starts_with('/') || rest.starts_with('['))
}

/// Why the rules were selected as they were.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reason {
    /// Engine code changed: every rule.
    Code,
    /// Content settings changed: their rules.
    Content,
    /// No revision to compare with: every rule.
    NoBase,
    /// Nothing changed against the revision: no rule.
    None,
}

impl Reason {
    pub fn code(self) -> &'static str {
        match self {
            Reason::Code => "code",
            Reason::Content => "content",
            Reason::NoBase => "no-base",
            Reason::None => "none",
        }
    }
}

/// `calib.rules.touched`: what the change touched and the rules it selected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Touched {
    pub reason: Reason,
    /// The commit the content was compared with; none without a revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub against: Option<String>,
    /// Why there is nothing to compare with, for a `no-base` run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// The changed paths of engine code.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub code: Vec<String>,
    /// The changed settings.
    pub settings: Vec<String>,
    /// The changed settings that no rule names.
    pub unruled: Vec<String>,
    /// The selected rules: job rules by job, and `five-match`; every rule when `all`.
    pub rules: Vec<String>,
    /// The selected confirmed setting rules, by id.
    pub setting_rules: Vec<String>,
    /// Every rule is selected.
    pub all: bool,
    /// Each changed setting's value before and after, for the draft rules.
    #[serde(skip)]
    pub changed: Vec<Changed>,
}

impl Touched {
    /// Every rule, for `reason` (`code` or `no-base`).
    pub fn every(
        reason: Reason,
        against: Option<String>,
        why: Option<String>,
        code: Vec<String>,
        confirmed: &[(String, String)],
    ) -> Touched {
        Touched {
            reason,
            against,
            why,
            code,
            settings: Vec::new(),
            unruled: Vec::new(),
            rules: Vec::new(),
            setting_rules: confirmed.iter().map(|(id, _)| id.clone()).collect(),
            all: true,
            changed: Vec::new(),
        }
    }

    /// Whether the job rule (or `five-match`) `job` is selected.
    pub fn selects(&self, job: &str) -> bool {
        self.all || self.rules.iter().any(|r| r == job)
    }

    /// The one line of the console: why the rules were selected.
    pub fn why_line(&self) -> String {
        match self.reason {
            Reason::Code => format!("engine code changed: {}", self.code.len()),
            Reason::NoBase => format!(
                "no revision to compare with{}",
                self.why
                    .as_deref()
                    .map_or(String::new(), |w| format!(": {w}"))
            ),
            Reason::Content => format!(
                "{} changed settings, {} with no rule",
                self.settings.len(),
                self.unruled.len()
            ),
            Reason::None => "no setting changed".to_string(),
        }
    }
}

/// The rules the `changed` settings select, against the content of `against`, with the
/// confirmed setting rules `confirmed` (id, setting).
pub fn select(
    changed: Vec<Changed>,
    against: String,
    index: &RuleIndex,
    confirmed: &[(String, String)],
) -> Touched {
    let mut rules = BTreeSet::new();
    let mut setting_rules = BTreeSet::new();
    let mut unruled = Vec::new();
    for c in &changed {
        let found = rules_of(&c.key, index);
        let by_setting: Vec<&String> = confirmed
            .iter()
            .filter(|(_, s)| under(&c.key, s))
            .map(|(id, _)| id)
            .collect();
        if found.is_empty() && by_setting.is_empty() {
            unruled.push(c.key.clone());
        }
        rules.extend(found);
        setting_rules.extend(by_setting.into_iter().cloned());
    }
    Touched {
        reason: if changed.is_empty() {
            Reason::None
        } else {
            Reason::Content
        },
        against: Some(against),
        why: None,
        code: Vec::new(),
        settings: changed.iter().map(|c| c.key.clone()).collect(),
        unruled,
        rules: rules.into_iter().collect(),
        setting_rules: setting_rules.into_iter().collect(),
        all: false,
        changed,
    }
}

/// What the change of the content folder `content_dir` touched against `rev`: a code change
/// or a run with no revision selects every rule, a content change its settings' rules.
pub fn find(
    content_dir: &Path,
    rev: Option<&str>,
    confirmed: &[(String, String)],
) -> anyhow::Result<Touched> {
    let Some(rev) = rev else {
        return Ok(Touched::every(
            Reason::NoBase,
            None,
            Some("a ready old engine with no revision named".to_string()),
            Vec::new(),
            confirmed,
        ));
    };
    if super::base::checkout_of(content_dir).is_none() {
        return Ok(Touched::every(
            Reason::NoBase,
            None,
            Some(format!(
                "the content folder {} is not a checkout's content folder",
                content_dir.display()
            )),
            Vec::new(),
            confirmed,
        ));
    }
    let base = super::base::content_at(content_dir, rev)?;
    let code: Vec<String> = super::base::changed_paths(&base.repo, &base.commit)?
        .into_iter()
        .filter(|p| is_code(p))
        .collect();
    if !code.is_empty() {
        return Ok(Touched::every(
            Reason::Code,
            Some(base.commit),
            None,
            code,
            confirmed,
        ));
    }
    let changed = diff_dirs(&base.content, content_dir)?;
    let index = RuleIndex::of(&[content_dir, &base.content]);
    Ok(select(changed, base.commit, &index, confirmed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn shipped() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
    }

    fn index() -> RuleIndex {
        RuleIndex::of(&[&shipped()])
    }

    /// Two copies of the shipped content, the second with `edit` applied to `file`.
    fn pair(file: &str, edit: impl FnOnce(&mut Value)) -> (tempfile::TempDir, Vec<Changed>) {
        let tmp = tempfile::TempDir::new().unwrap();
        for side in ["old", "new"] {
            copy(&shipped(), &tmp.path().join(side));
        }
        let path = tmp.path().join("new").join(file);
        let mut v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        edit(&mut v);
        std::fs::write(&path, serde_json::to_string_pretty(&v).unwrap()).unwrap();
        let changed = diff_dirs(&tmp.path().join("old"), &tmp.path().join("new")).unwrap();
        (tmp, changed)
    }

    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            let t = to.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copy(&e.path(), &t);
            } else {
                std::fs::copy(e.path(), t).unwrap();
            }
        }
    }

    fn keys(c: &[Changed]) -> Vec<&str> {
        c.iter().map(|c| c.key.as_str()).collect()
    }

    #[test]
    fn one_changed_tuning_value_is_one_setting() {
        let (_t, c) = pair("tuning.json", |v| v["engine"]["foul_base"] = json!(0.2));
        assert_eq!(keys(&c), ["tuning.json#/engine/foul_base"]);
        assert_eq!(c[0].old, Some(json!(0.1)));
        assert_eq!(c[0].new, Some(json!(0.2)));
        let t = select(c, "abc".into(), &index(), &[]);
        assert_eq!(t.reason, Reason::Content);
        assert!(t.rules.is_empty());
        assert_eq!(t.unruled, ["tuning.json#/engine/foul_base"]);
    }

    #[test]
    fn an_inserted_attribute_changes_only_its_own_keys() {
        let (_t, c) = pair("attributes.json", |v| {
            let list = v["attributes"].as_array_mut().unwrap();
            let mut copy = list[0].clone();
            copy["name"] = json!("new_one");
            list.insert(1, copy);
        });
        assert!(!c.is_empty());
        assert!(
            c.iter()
                .all(|c| c.key.starts_with("attributes.json#/attributes[new_one]/")),
            "{:?}",
            keys(&c)
        );
        assert!(c.iter().all(|c| c.old.is_none()));
    }

    #[test]
    fn a_stage_table_edit_maps_to_its_attributes_and_stage_rule() {
        let (_t, c) = pair("attributes.json", |v| {
            v["actions"]["pass"]["execute"]["main"]["weight"] = json!(0.9);
        });
        assert_eq!(
            keys(&c),
            ["attributes.json#/actions/pass/execute/main/weight"]
        );
        let t = select(c, "abc".into(), &index(), &[]);
        assert_eq!(t.rules, ["passing", "technique", "vision"]);
        assert!(t.unruled.is_empty());
        let press = rules_of(
            "attributes.json#/actions/press/choose/main/weight",
            &index(),
        );
        assert!(press.contains(&"press.choose".to_string()), "{press:?}");
        let whole = rules_of("tuning.json#/engine/contract/actions/pass/k", &index());
        assert!(whole.contains(&"decisions".to_string()), "{whole:?}");
        assert!(whole.contains(&"passing".to_string()), "{whole:?}");
    }

    #[test]
    fn a_module_change_is_a_setting_with_no_rule() {
        let (_t, c) = pair("slots.json", |v| {
            let slots = v["slots"].as_object_mut().unwrap();
            let (name, slot) = slots.iter_mut().next().unwrap();
            let _ = name;
            slot["module"] = json!("another-module");
        });
        assert_eq!(c.len(), 1, "{:?}", keys(&c));
        assert!(c[0].key.starts_with("slots.json#/slots/"), "{:?}", keys(&c));
        let t = select(c, "abc".into(), &index(), &[]);
        assert!(t.rules.is_empty());
        assert_eq!(t.unruled.len(), 1);
    }

    #[test]
    fn a_setting_rules_edit_is_no_changed_setting() {
        let (_t, c) = pair("sensitivity.json", |v| {
            v["setting_rules"] = json!([{"id": "sr-00000000", "status": "draft"}]);
        });
        assert!(c.is_empty(), "{:?}", keys(&c));
        let (_t, c) = pair("sensitivity.json", |v| v["design"]["seed"] = json!(7));
        let t = select(c, "abc".into(), &index(), &[]);
        assert_eq!(t.rules.len(), index().jobs.len());
    }

    #[test]
    fn a_confirmed_setting_rule_is_selected_by_its_setting() {
        let (_t, c) = pair("tuning.json", |v| {
            v["engine"]["shots"]["save_high"] = json!(0.5);
            v["engine"]["foul_base"] = json!(0.2);
        });
        let confirmed = [(
            "sr-1".to_string(),
            "tuning.json#/engine/shots/save_high".to_string(),
        )];
        let t = select(c, "abc".into(), &index(), &confirmed);
        assert_eq!(t.setting_rules, ["sr-1"]);
        assert_eq!(t.unruled, ["tuning.json#/engine/foul_base"]);
        assert!(under(
            "tuning.json#/engine/shots",
            "tuning.json#/engine/shots"
        ));
        assert!(under(
            "tuning.json#/engine/shots/save_high",
            "tuning.json#/engine/shots"
        ));
        assert!(!under(
            "tuning.json#/engine/shots_x",
            "tuning.json#/engine/shots"
        ));
    }

    #[test]
    fn every_table_prefix_resolves_in_the_shipped_content() {
        let content = shipped();
        let read = |f: &str| -> Value {
            serde_json::from_slice(&std::fs::read(content.join(f)).unwrap()).unwrap()
        };
        let attrs = read("attributes.json");
        let tuning = read("tuning.json");
        let rules = read("sensitivity.json");
        assert!(
            attrs["attributes"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
        );
        assert!(attrs["actions"]["pass"]["execute"].is_object());
        for p in [
            "/engine/contract/actions/pass",
            "/engine/contract/curve",
            "/engine/contract/body/jobs/height",
            "/engine/contract/body/height",
            "/engine/contract/body/age",
            "/engine/contract/body/adaptation",
            "/match_rating",
        ] {
            assert!(tuning.pointer(p).is_some(), "tuning.json {p}");
        }
        for p in ["/rules", "/design", "/defaults", "/five_match"] {
            assert!(rules.pointer(p).is_some(), "sensitivity.json {p}");
        }
        let index = index();
        for job in &index.jobs {
            let as_attribute =
                rules_of(&format!("attributes.json#/attributes[{job}]/group"), &index);
            let as_rule = rules_of(&format!("sensitivity.json#/rules[{job}]/levels/0"), &index);
            assert_eq!(as_rule, [job.clone()]);
            if !job.contains('.') && !["height", "age", "nationality"].contains(&job.as_str()) {
                assert_eq!(as_attribute, [job.clone()], "{job}");
            }
        }
        assert_eq!(
            rules_of(
                "tuning.json#/engine/contract/body/jobs/age/statistic",
                &index
            ),
            ["age"]
        );
        assert_eq!(
            rules_of("tuning.json#/engine/contract/curve/scale", &index),
            [FIVE_MATCH]
        );
    }

    fn git(dir: &Path, args: &[&str]) {
        let ok = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "core.autocrlf=false",
            ])
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(
            ok.status.success(),
            "{}",
            String::from_utf8_lossy(&ok.stderr)
        );
    }

    #[test]
    fn a_fixture_checkout_tells_code_from_content() {
        let repo = tempfile::TempDir::new().unwrap();
        let content = repo.path().join("content");
        copy(&shipped(), &content);
        git(repo.path(), &["init", "-q"]);
        git(repo.path(), &["add", "-A"]);
        git(repo.path(), &["commit", "-q", "-m", "one"]);
        // The same content: nothing changed.
        let t = find(&content, Some("HEAD"), &[]).unwrap();
        assert_eq!(t.reason, Reason::None, "{t:?}");
        // A content edit.
        let path = content.join("tuning.json");
        let mut v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        v["engine"]["foul_base"] = json!(0.3);
        std::fs::write(&path, serde_json::to_string_pretty(&v).unwrap()).unwrap();
        let t = find(&content, Some("HEAD"), &[]).unwrap();
        assert_eq!(t.reason, Reason::Content, "{t:?}");
        assert_eq!(t.settings, ["tuning.json#/engine/foul_base"]);
        // An untracked file under crates/ is a code change.
        std::fs::create_dir_all(repo.path().join("crates/x")).unwrap();
        std::fs::write(repo.path().join("crates/x/lib.rs"), "").unwrap();
        let t = find(&content, Some("HEAD"), &[]).unwrap();
        assert_eq!(t.reason, Reason::Code);
        assert_eq!(t.code, ["crates/x/lib.rs"]);
        assert!(t.all && t.selects("passing"));
        // No revision.
        let t = find(&content, None, &[]).unwrap();
        assert_eq!(t.reason, Reason::NoBase);
        assert!(t.all);
        // A content folder outside a checkout.
        let outside = tempfile::TempDir::new().unwrap();
        let t = find(outside.path(), Some("HEAD"), &[]).unwrap();
        assert_eq!(t.reason, Reason::NoBase);
    }
}
