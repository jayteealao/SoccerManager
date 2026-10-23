//! The committed world database: it loads, covers every country and tier of its coverage
//! record, holds a facsimile name and a strength for every club and no real name, and its gap
//! list and ClubElo accounting are complete. The real names it is checked against sit in
//! `gate/`, which never ships.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use engine::data::world::{Scope, StrengthSource, WORLD_FILE, WorldDatabase, load_world};
use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn json(rel: &str) -> Value {
    serde_json::from_str(&read(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn world() -> WorldDatabase {
    let bytes = std::fs::read(repo().join("content").join(WORLD_FILE)).unwrap();
    load_world(&bytes, "content/world.json").unwrap().value
}

/// Every string value of a JSON value.
fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

/// A name for comparison: lowercase letters and digits, one club-form word dropped from
/// either end.
fn fold(name: &str) -> String {
    const FORMS: [&str; 7] = ["fc", "afc", "cf", "sc", "ac", "fk", "sk"];
    let mut words: Vec<String> = name
        .to_lowercase()
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect();
    if words.len() > 1 && FORMS.contains(&words[0].as_str()) {
        words.remove(0);
    }
    if words.len() > 1 && FORMS.contains(&words[words.len() - 1].as_str()) {
        words.pop();
    }
    words.concat()
}

/// The gap list's rows as named fields.
fn gap_rows() -> Vec<BTreeMap<String, String>> {
    let text = read("gate/world/gaps.csv");
    let mut lines = text.lines();
    let header: Vec<String> = lines.next().unwrap().split(',').map(String::from).collect();
    lines
        .map(|l| {
            let mut fields = Vec::new();
            let (mut cur, mut quoted) = (String::new(), false);
            let mut chars = l.chars().peekable();
            while let Some(c) = chars.next() {
                match c {
                    '"' if quoted && chars.peek() == Some(&'"') => {
                        cur.push('"');
                        chars.next();
                    }
                    '"' => quoted = !quoted,
                    ',' if !quoted => fields.push(std::mem::take(&mut cur)),
                    c => cur.push(c),
                }
            }
            fields.push(cur);
            assert_eq!(fields.len(), header.len(), "gaps.csv row {l}");
            header.iter().cloned().zip(fields).collect()
        })
        .collect()
}

#[test]
fn every_country_and_tier_of_the_coverage_record_is_present() {
    let w = world();
    let coverage = json("gate/world/coverage.json");
    for c in coverage["countries"].as_array().unwrap() {
        assert!(w.countries.contains_key(c.as_str().unwrap()), "{c}");
    }
    let tiers: BTreeSet<(String, u8)> = w
        .leagues
        .values()
        .filter_map(|l| Some((l.country.clone(), l.tier?)))
        .collect();
    for t in coverage["tiers"].as_array().unwrap() {
        let key = (
            t[0].as_str().unwrap().to_string(),
            t[1].as_u64().unwrap() as u8,
        );
        assert!(tiers.contains(&key), "{key:?}");
    }
    assert!(w.countries.len() > 200, "{} countries", w.countries.len());
    assert!(w.clubs.len() > 10_000, "{} clubs", w.clubs.len());
}

#[test]
fn every_club_has_a_facsimile_name_and_a_strength() {
    let w = world();
    let map = json("gate/world/name-map.json");
    let mut real = BTreeSet::new();
    let mut all = Vec::new();
    for key in ["clubs", "leagues", "competitions", "other"] {
        strings(&map[key], &mut all);
    }
    real.extend(all.iter().map(|n| fold(n)).filter(|n| !n.is_empty()));
    for (id, c) in &w.clubs {
        assert!(
            !c.name.trim().is_empty() && !c.short_name.trim().is_empty(),
            "{id}"
        );
        assert!(c.elo > 0, "{id}");
        assert!(
            !real.contains(&fold(&c.name)),
            "{id}: {} folds to a real name",
            c.name
        );
        let own = map["clubs"][id.as_str()]
            .as_array()
            .expect("a real-name entry");
        for n in own {
            assert_ne!(n.as_str().unwrap(), c.name, "{id}");
        }
    }
    for l in w.leagues.values() {
        assert!(!real.contains(&fold(&l.name)), "{}", l.name);
    }
    for k in w.competitions.values() {
        assert!(!real.contains(&fold(&k.name)), "{}", k.name);
    }
}

#[test]
fn no_string_of_the_world_file_equals_a_listed_real_name() {
    let mut names = BTreeSet::new();
    for e in std::fs::read_dir(repo().join("gate/real-names")).unwrap() {
        let path = e.unwrap().path();
        if path.extension().is_some_and(|x| x == "json") {
            let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let mut all = Vec::new();
            strings(&v, &mut all);
            names.extend(all.iter().map(|s| s.trim().to_lowercase()));
        }
    }
    let list = json("gate/real-names/world.json");
    assert!(list.as_array().unwrap().len() > 10_000);
    let mut all = Vec::new();
    strings(&json("content/world.json"), &mut all);
    for s in all {
        assert!(
            !names.contains(&s.trim().to_lowercase()),
            "{s:?} is a real name"
        );
    }
}

#[test]
fn the_readme_section_names_no_real_club() {
    let readme = read("content/README.md");
    let start = readme
        .find("\n## World database")
        .expect("a World database section");
    let rest = &readme[start + 1..];
    let section = rest[3..].find("\n## ").map_or(rest, |e| &rest[..e + 3]);
    let words: BTreeSet<String> = section
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    let text = section.to_lowercase();
    for n in json("gate/real-names/world.json").as_array().unwrap() {
        let n = n.as_str().unwrap().to_lowercase();
        let whole = if n.contains(' ') {
            text.contains(&n)
        } else {
            words.contains(&n)
        };
        assert!(!whole, "the README names {n}");
    }
}

#[test]
fn the_top_30_uefa_associations_have_their_club_count() {
    let w = world();
    let mut ranked: Vec<(u8, &String, Option<u16>)> = w
        .countries
        .iter()
        .filter_map(|(k, c)| c.uefa.as_ref().map(|u| (u.rank, k, u.top_division_clubs)))
        .collect();
    ranked.sort();
    assert_eq!(ranked.len(), 55);
    for (rank, code, clubs) in ranked.iter().take(30) {
        assert!(clubs.is_some_and(|n| n > 0), "{rank} {code}");
        let top = w
            .leagues
            .values()
            .filter(|l| &l.country == *code && l.tier == Some(1))
            .count();
        assert!(top >= 1, "{code} has no tier 1 league");
    }
}

#[test]
fn every_derived_strength_and_every_unlinked_clubelo_row_is_on_the_gap_list() {
    let w = world();
    let rows = gap_rows();
    for r in &rows {
        for f in ["kind", "entity", "missing", "reason", "sources_tried"] {
            assert!(!r[f].is_empty(), "{f} empty: {r:?}");
        }
    }
    let derived: BTreeSet<&str> = rows
        .iter()
        .filter(|r| r["kind"] == "missing-strength")
        .map(|r| r["id"].as_str())
        .collect();
    for (id, c) in &w.clubs {
        if c.strength_source != StrengthSource::Clubelo {
            assert!(
                derived.contains(id.as_str()),
                "{id} has no missing-strength row"
            );
        }
    }
    let coverage = json("gate/world/coverage.json");
    let e = &coverage["clubelo"];
    let unlinked = rows
        .iter()
        .filter(|r| r["kind"] == "clubelo-unlinked")
        .count() as u64;
    assert_eq!(e["on_gap_list"].as_u64().unwrap(), unlinked);
    assert_eq!(
        e["linked"].as_u64().unwrap() + unlinked,
        e["rows"].as_u64().unwrap(),
        "the ClubElo accounting closes"
    );
    let linked = w
        .clubs
        .values()
        .filter(|c| c.strength_source == StrengthSource::Clubelo)
        .count() as u64;
    assert_eq!(linked, e["linked"].as_u64().unwrap());
}

#[test]
fn competitions_are_structure_and_the_sources_are_credited() {
    let w = world();
    assert!(w.competitions.values().all(|k| !k.stages.is_empty()));
    for scope in [
        Scope::Continental,
        Scope::Intercontinental,
        Scope::DomesticCup,
    ] {
        assert!(
            w.competitions.values().any(|k| k.scope == scope),
            "{scope:?}"
        );
    }
    let keys: BTreeMap<&str, &str> = w
        .sources
        .iter()
        .map(|s| (s.key.as_str(), s.licence.as_str()))
        .collect();
    assert!(keys.keys().any(|k| k.starts_with("openfootball:")));
    assert_eq!(keys["wikidata"], "CC0 1.0");
    assert_eq!(keys["wikipedia"], "CC BY-SA 4.0");
    assert!(keys.contains_key("clubelo"));
    assert!(w.sources.iter().all(|s| !s.licence.is_empty()));
}

#[test]
fn the_world_file_fits_its_size_cap() {
    let len = std::fs::metadata(repo().join("content").join(WORLD_FILE))
        .unwrap()
        .len();
    assert!(len <= 8 * 1024 * 1024, "{len} bytes");
}
