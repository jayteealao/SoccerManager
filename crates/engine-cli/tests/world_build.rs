//! The world database builder on invented snapshots: what it writes, what it leaves out, the
//! gap list, the facsimile names, the pre-write scan, determinism, and that it reads only
//! the local snapshot, hand-fill and gate list files.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture() -> PathBuf {
    manifest().join("tests/fixtures/world")
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dest = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &dest);
        } else {
            std::fs::copy(e.path(), dest).unwrap();
        }
    }
}

/// One build of `data` into a fresh folder: the folder (content/ and gate/) and the output.
/// The child is waited on before the folder can drop.
fn build_from(data: &Path, extra_list: Option<&str>) -> (TempDir, Output) {
    let tmp = TempDir::new().unwrap();
    copy_dir(&fixture().join("gate"), &tmp.path().join("gate"));
    if let Some(text) = extra_list {
        std::fs::write(tmp.path().join("gate/real-names/bad.json"), text).unwrap();
    }
    std::fs::create_dir_all(tmp.path().join("content")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_LOG", "warn")
        .env("SM_CONTENT_DIR", manifest().join("../../content"))
        .args(["world-database", "build", "--data"])
        .arg(data)
        .arg("--out")
        .arg(tmp.path().join("content/world.json"))
        .arg("--gate")
        .arg(tmp.path().join("gate"))
        .arg("--hand-fills")
        .arg(fixture().join("hand-fills.json"))
        .output()
        .unwrap();
    (tmp, out)
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
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

/// The gap list as rows of named fields.
fn gaps(path: &Path) -> Vec<std::collections::BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    let header: Vec<String> = lines.next().unwrap().split(',').map(String::from).collect();
    lines
        .map(|l| {
            // A field holding a comma is quoted; split outside quotes.
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
            assert_eq!(fields.len(), header.len(), "{l}");
            header.iter().cloned().zip(fields).collect()
        })
        .collect()
}

/// The content id of the club whose first real name is `name`.
fn club_id(name_map: &Value, name: &str) -> String {
    name_map["clubs"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, names)| names[0] == name)
        .unwrap_or_else(|| panic!("no club {name}"))
        .0
        .clone()
}

#[test]
fn the_fixture_world_holds_every_country_tier_club_and_competition() {
    let (tmp, out) = build_from(&fixture().join("data"), None);
    assert!(out.status.success(), "{}", text(&out));
    let world = json(&tmp.path().join("content/world.json"));
    let gate = tmp.path().join("gate");
    let coverage = json(&gate.join("world/coverage.json"));
    let names = json(&gate.join("world/name-map.json"));

    // Every country and (country, tier) the sources cover.
    let countries: BTreeSet<&str> = world["countries"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(countries, BTreeSet::from(["FRL", "NTL", "OLD", "TST"]));
    let tiers: BTreeSet<(String, u64)> = world["leagues"]
        .as_object()
        .unwrap()
        .values()
        .filter_map(|l| Some((l["country"].as_str()?.to_string(), l["tier"].as_u64()?)))
        .collect();
    for (c, t) in [
        ("TST", 1),
        ("TST", 2),
        ("TST", 3),
        ("TST", 4),
        ("NTL", 1),
        ("FRL", 1),
    ] {
        assert!(tiers.contains(&(c.to_string(), t)), "{c} tier {t} missing");
    }
    for t in coverage["tiers"].as_array().unwrap() {
        assert!(tiers.contains(&(t[0].as_str().unwrap().into(), t[1].as_u64().unwrap())));
    }
    // No country carries a name; the UEFA entries hold the ranking and the places.
    for c in world["countries"].as_object().unwrap().values() {
        assert!(c.get("name").is_none());
    }
    assert_eq!(world["countries"]["TST"]["uefa"]["top_division_clubs"], 5);
    assert_eq!(
        world["countries"]["FRL"]["uefa"]["rank"], 3,
        "IOC code FRX maps to FRL"
    );
    assert_eq!(world["countries"]["TST"]["region"], "europe");
    assert_eq!(world["countries"]["FRL"]["region"], "asia");

    // Leagues: tiers from each source, groups for a shared tier, a tier no league holds.
    let leagues = world["leagues"].as_object().unwrap();
    let league = |real: &str| {
        let id = names["leagues"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(_, n)| n[0] == real)
            .unwrap()
            .0;
        &leagues[id]
    };
    assert_eq!(league("Testland Second Circuit")["tier"], 2, "openfootball");
    assert_eq!(
        league("Testland Third Circuit East")["tier"],
        3,
        "level chain"
    );
    assert_eq!(league("Testland Third Circuit East")["group"], "A");
    assert_eq!(league("Testland Third Circuit West")["group"], "B");
    assert_eq!(league("Testland Fourth Circuit")["club_count"], 16);
    assert_eq!(league("Testland Premier Circuit")["name"], "TST tier 1");
    assert!(league("Testland Sunday Circuit").get("tier").is_none());
    assert_eq!(
        league("Farland Major Circuit, \"Top\"")["tier"],
        1,
        "the lower of 1|2"
    );

    // Clubs: a facsimile name clear of every real name, and a strength.
    let mut real = BTreeSet::new();
    let mut all = Vec::new();
    strings(&names, &mut all);
    strings(
        &json(&fixture().join("gate/real-names/other.json")),
        &mut all,
    );
    real.extend(all.iter().map(|s| s.trim().to_lowercase()));
    let clubs = world["clubs"].as_object().unwrap();
    assert_eq!(clubs.len(), 17);
    for (id, c) in clubs {
        let name = c["name"].as_str().unwrap();
        let short = c["short_name"].as_str().unwrap();
        assert!(
            !real.contains(&name.to_lowercase()),
            "{id}: {name} is a real name"
        );
        assert!(
            !real.contains(&short.to_lowercase()),
            "{id}: {short} is a real name"
        );
        assert!(c["elo"].as_u64().unwrap() > 0);
        for own in names["clubs"][id].as_array().unwrap() {
            assert_ne!(own.as_str().unwrap(), name);
        }
    }
    // The planted names forced redraws.
    assert!(clubs.values().all(|c| c["name"] != "Salworth Town"));
    assert!(clubs.values().all(|c| c["short_name"] != "ULM"));
    // A club of the United Kingdom takes its home nation.
    let nation = |real: &str| clubs[&club_id(&names, real)]["country"].clone();
    assert_eq!(nation("Borage Harriers"), "NTL");
    assert_eq!(nation("Chervil Lions"), "NTL", "from its league");
    assert_eq!(nation("Dill Saints"), "NTL", "from the hand fill");

    // Strengths: every linked club has its ClubElo rating; the others follow the rule.
    let club = |real: &str| clubs[&club_id(&names, real)].clone();
    for (real, elo) in [
        ("Brisket Athletic", 1700),
        ("Mallow Rovers F.C.", 1650),
        ("Quince Town", 1600),
        ("Tamarind City", 1450),
        ("Sumac Athletic", 1420),
        ("Anise Reserves", 1350),
        ("Lovage Comets", 1250),
    ] {
        assert_eq!(club(real)["elo"], elo, "{real}");
        assert_eq!(club(real)["strength_source"], "clubelo", "{real}");
    }
    assert_eq!(club("Sorrel United")["strength_source"], "league-median");
    assert_eq!(club("Fennel Wanderers")["strength_source"], "league-median");
    assert_eq!(club("Saffron Dale FC")["strength_source"], "tier-step");
    assert_eq!(club("Caraway Stead")["strength_source"], "floor");
    // Placed by its ClubElo level: no current league, ClubElo level 2.
    assert_eq!(
        club("Sumac Athletic")["league"],
        club("Tamarind City")["league"]
    );

    // The gap list: every unlinked row with its reason, every field filled.
    let rows = gaps(&gate.join("world/gaps.csv"));
    for r in &rows {
        for f in ["kind", "entity", "missing", "sources_tried", "reason"] {
            assert!(!r[f].is_empty(), "{f} empty in {r:?}");
        }
    }
    let unlinked: Vec<(&str, &str)> = rows
        .iter()
        .filter(|r| r["kind"] == "clubelo-unlinked")
        .map(|r| (r["real_name"].as_str(), r["reason"].as_str()))
        .collect();
    for (club, why) in [
        ("Ghost Club", "unmatched"),
        ("Twin Club", "ambiguous"),
        ("Brisket Reserves", "reserve"),
        ("Newcomer", "provisional"),
        ("Mallow Duplicate", "two ClubElo rows"),
        ("Sorrel", "two ClubElo rows"),
        ("Fennel", "refused at the spot-check"),
    ] {
        assert!(
            unlinked.iter().any(|(c, r)| *c == club && r.contains(why)),
            "{club}: {unlinked:?}"
        );
    }
    let kinds: BTreeSet<&str> = rows.iter().map(|r| r["kind"].as_str()).collect();
    for k in [
        "club-count-conflict",
        "match-review",
        "missing-home-nation",
        "missing-region",
        "missing-strength",
        "missing-tier",
        "no-current-league",
        "tier-review",
    ] {
        assert!(kinds.contains(k), "no {k} row");
    }
    let review: Vec<&str> = rows
        .iter()
        .filter(|r| r["kind"] == "match-review")
        .map(|r| r["reason"].as_str())
        .collect();
    assert!(review.iter().any(|r| r.contains("other country")));
    assert!(review.iter().any(|r| r.contains("no ClubElo country")));
    assert!(review.iter().any(|r| r.contains("broken character")));
    // Every derived strength has its gap row, by content id.
    for (id, c) in clubs {
        if c["strength_source"] != "clubelo" {
            assert!(
                rows.iter()
                    .any(|r| r["kind"] == "missing-strength" && r["id"] == *id),
                "{id}"
            );
        }
    }
    // The ClubElo accounting closes.
    let e = &coverage["clubelo"];
    assert_eq!(
        e["file"], "clubelo/clubelo-2026-01-01.csv",
        "the newest dated file"
    );
    assert_eq!(
        e["linked"].as_u64().unwrap() + e["on_gap_list"].as_u64().unwrap(),
        e["rows"].as_u64().unwrap()
    );
    assert_eq!(e["on_gap_list"], unlinked.len());

    // Competitions as structure only; no prose or venue reaches the content file.
    let comps = world["competitions"].as_object().unwrap();
    assert_eq!(comps.len(), 4);
    assert!(
        comps
            .values()
            .all(|c| !c["stages"].as_array().unwrap().is_empty())
    );
    let content = std::fs::read_to_string(tmp.path().join("content/world.json")).unwrap();
    for prose in [
        "Mount Venue",
        "champions only",
        "seeded",
        "single table",
        "Trophy",
    ] {
        assert!(!content.contains(prose), "{prose} reached the content file");
    }
    assert_eq!(
        coverage["competitions_skipped"][0]["source_id"],
        "world-club-trophy-2029"
    );
    let places = world["countries"]["TST"]["uefa"]["places"]
        .as_array()
        .unwrap();
    assert!(
        places
            .iter()
            .all(|p| comps.contains_key(p["competition"].as_str().unwrap()))
    );
    // Credits.
    let keys: BTreeSet<&str> = world["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["key"].as_str().unwrap())
        .collect();
    for k in ["openfootball:testland", "clubelo", "wikidata", "wikipedia"] {
        assert!(keys.contains(k), "no source {k}");
    }
    let wiki = world["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"] == "wikipedia")
        .unwrap();
    assert_eq!(wiki["revisions"], serde_json::json!([111, 222, 444]));

    // The packaging list holds real club names and no structural string; the other list is
    // untouched.
    let list = json(&gate.join("real-names/world.json"));
    let list: BTreeSet<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for n in [
        "Brisket Athletic",
        "Brisket Athletic Football Club",
        "Pepper Corn FC",
        "Ghost Club",
    ] {
        assert!(list.contains(n), "{n} not listed");
    }
    for n in ["Brisket A.", "The \"Briskets\"", "BRK", "BRI"] {
        assert!(!list.contains(n), "{n}: aliases and short names stay out");
    }
    assert!(
        list.contains("Brisket"),
        "a ClubElo name of 4 or more characters is listed"
    );
    let mut world_strings = Vec::new();
    strings(&world, &mut world_strings);
    for s in &world_strings {
        assert!(!list.contains(s.as_str()), "{s} listed");
    }
    assert_eq!(
        std::fs::read(gate.join("real-names/other.json")).unwrap(),
        std::fs::read(fixture().join("gate/real-names/other.json")).unwrap()
    );

    // Only the snapshot, the hand fills and the gate lists were read.
    for i in coverage["inputs"].as_array().unwrap() {
        let p = i["path"].as_str().unwrap();
        assert!(
            p.starts_with("data:")
                || p == "hand-fills:hand-fills.json"
                || (p.starts_with("gate:real-names/") && p != "gate:real-names/world.json"),
            "{p}"
        );
    }
    drop(tmp);
}

#[test]
fn two_builds_write_the_same_bytes() {
    let (a, oa) = build_from(&fixture().join("data"), None);
    let (b, ob) = build_from(&fixture().join("data"), None);
    assert!(oa.status.success() && ob.status.success());
    for f in [
        "content/world.json",
        "gate/real-names/world.json",
        "gate/world/name-map.json",
        "gate/world/gaps.csv",
        "gate/world/coverage.json",
    ] {
        assert_eq!(
            std::fs::read(a.path().join(f)).unwrap(),
            std::fs::read(b.path().join(f)).unwrap(),
            "{f}"
        );
    }
}

#[test]
fn a_list_holding_a_string_of_the_world_file_fails_the_build_and_writes_nothing() {
    let (tmp, out) = build_from(&fixture().join("data"), Some(r#"["europe"]"#));
    assert!(!out.status.success());
    let t = text(&out);
    assert!(
        t.contains("\"europe\"") && t.contains("real-names/bad.json"),
        "{t}"
    );
    assert!(!tmp.path().join("content/world.json").exists());
    assert!(!tmp.path().join("gate/world").exists());
}

#[test]
fn a_missing_snapshot_file_fails_naming_it() {
    let tmp = TempDir::new().unwrap();
    copy_dir(&fixture().join("data"), tmp.path());
    std::fs::remove_file(tmp.path().join("wikidata/leagues.csv")).unwrap();
    let (out_dir, out) = build_from(tmp.path(), None);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("wikidata/leagues.csv"),
        "{}",
        text(&out)
    );
    drop(out_dir);
}

#[test]
fn a_malformed_row_fails_naming_its_file_and_line() {
    let tmp = TempDir::new().unwrap();
    copy_dir(&fixture().join("data"), tmp.path());
    let path = tmp.path().join("wikidata/clubs.csv");
    let mut text_in = std::fs::read_to_string(&path).unwrap();
    text_in.push_str("Q99,\"Open Quote,,,,,Q100,,Q500\n");
    std::fs::write(&path, text_in).unwrap();
    let (out_dir, out) = build_from(tmp.path(), None);
    assert!(!out.status.success());
    let t = text(&out);
    assert!(t.contains("clubs.csv") && t.contains("line 21"), "{t}");
    drop(out_dir);
}

#[test]
fn the_builder_source_names_no_network_or_process_call() {
    let dir = manifest().join("src/world_build");
    for e in std::fs::read_dir(&dir).unwrap() {
        let path = e.unwrap().path();
        let src = std::fs::read_to_string(&path).unwrap();
        for word in [
            "std::net",
            "TcpStream",
            "UdpSocket",
            "process::Command",
            "http",
        ] {
            let hits: Vec<&str> = src
                .lines()
                .filter(|l| l.contains(word))
                // The credits name the sources' web addresses as data.
                .filter(|l| !(word == "http" && l.contains("\"https://")))
                .collect();
            assert!(hits.is_empty(), "{}: {word}: {hits:?}", path.display());
        }
    }
}
