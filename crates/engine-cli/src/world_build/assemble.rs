//! Assembling the world from the snapshots: countries, leagues and tiers, clubs and their
//! home nations, the ClubElo links, strengths, competitions, the gap list, the coverage
//! record and the real names. Every real name stays in the gate outputs; the world database
//! holds codes, opaque ids and facsimile names only.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, bail};
use engine::data::world::{
    Club, Competition, Country, League, Place, Region, Scope, Source, Stage, StageKind,
    StrengthSource, UefaEntry, WORLD_VERSION, WorldDatabase,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::facsimile::{self, RealNames, fold};
use super::sources::{Snapshot, parse_of_clubs, parse_of_leagues};
use super::strength::{self, LeagueIn};

/// The builder's version, recorded in the coverage record.
pub const BUILDER_VERSION: u32 = 1;

/// One row of the gap list.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Gap {
    pub kind: &'static str,
    pub country: String,
    pub entity: &'static str,
    pub real_name: String,
    pub id: String,
    pub missing: &'static str,
    pub reason: String,
    pub sources_tried: &'static str,
}

/// Everything one build writes.
pub struct Built {
    pub world: WorldDatabase,
    pub gaps: Vec<Gap>,
    pub name_map: Value,
    /// Real names for the packaging scan, before the structural strings are taken out.
    pub scan_names: BTreeSet<String>,
    /// The coverage record, without its inputs and scan exceptions (the writer adds them).
    pub coverage: Value,
}

/// The hand-fill file: facts a person filled from a public page.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandFills {
    #[serde(default)]
    pub fills: Vec<Fill>,
    #[serde(default)]
    pub link_overrides: Vec<LinkOverride>,
}

/// One filled fact: `entity` is `league:<Wikidata id>`, `domestic:<code>:<tier>` or
/// `club:<Wikidata id>`; `field` is `tier`, `club_count` or `home_nation`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fill {
    pub entity: String,
    pub field: String,
    pub value: Value,
    /// Only `wikipedia` is accepted.
    pub source: String,
    pub page: String,
    pub revision: u64,
}

/// A ClubElo link refused after the spot-check.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkOverride {
    pub clubelo: String,
    pub wikidata: String,
    pub reason: String,
}

const TRIED_STRENGTH: &str = "clubelo snapshot; wikidata clubelo_match.csv";
const TRIED_TIER: &str = "wikidata level; openfootball leagues; domestic-structure.json; hand fills; wikidata level chain";
const TRIED_COUNT: &str = "domestic-structure.json; uefa-ranking-2026-27.csv; wikidata team count";
const TRIED_HOME: &str = "wikidata club; wikidata league; clubelo country; hand fills";
const TRIED_LINK: &str = "wikidata clubelo_match.csv; clubelo snapshot";
const TRIED_LEAGUE: &str = "wikidata current league (P118)";
const TRIED_COUNTRY: &str = "wikidata country (P17); associations.csv";

/// An opaque id: `prefix-` and the first 10 hex digits of SHA-256 over `key`.
pub fn opaque(prefix: char, key: &str) -> String {
    let d = Sha256::digest(key.as_bytes());
    format!("{prefix}-{}", super::sources::hex(&d[..5]))
}

fn region_of(confederation: &str) -> Option<Region> {
    Some(match confederation {
        "UEFA" => Region::Europe,
        "AFC" => Region::Asia,
        "CAF" => Region::Africa,
        "CONMEBOL" => Region::SouthAmerica,
        "CONCACAF" => Region::NorthCentralAmerica,
        "OFC" => Region::Oceania,
        _ => return None,
    })
}

fn is_code(s: &str) -> bool {
    s.len() == 3 && s.chars().all(|c| c.is_ascii_uppercase())
}

/// The leading whole number of a text ("8,506 (infobox)" is 8,506).
fn leading_number(s: &str) -> Option<u64> {
    let digits: String = s
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .filter(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

fn number(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str().and_then(leading_number))
}

/// A season as `2026-27` or `2026`, from the start of a text.
fn season(v: &Value) -> Option<String> {
    let s = v.as_str()?.trim();
    let b = s.as_bytes();
    let year = b.len() >= 4 && b[..4].iter().all(u8::is_ascii_digit);
    if !year {
        return None;
    }
    if b.len() >= 7 && b[4] == b'-' && b[5..7].iter().all(u8::is_ascii_digit) {
        Some(s[..7].to_string())
    } else {
        Some(s[..4].to_string())
    }
}

/// The legs of a cup from its text: 1 for a single match, 2 when every tie has two legs.
fn legs_of(v: &Value) -> Option<u8> {
    if let Some(n) = v.as_u64() {
        return u8::try_from(n).ok();
    }
    let t = v.as_str()?.to_lowercase();
    if t.starts_with("single") {
        Some(1)
    } else if t.starts_with("two") {
        Some(2)
    } else {
        None
    }
}

fn small<T: TryFrom<u64>>(v: Option<&Value>) -> Option<T> {
    v.and_then(number).and_then(|n| T::try_from(n).ok())
}

/// One stage from a source object.
fn stage(kind: StageKind, v: &Value) -> Stage {
    Stage {
        kind,
        teams: small(v.get("teams")),
        legs: v.get("legs").and_then(legs_of),
        matches_per_team: small(v.get("matches_per_team")),
        teams_entering: small(v.get("teams_entering")),
        groups: small(v.get("groups")).or_else(|| small(v.get("regions"))),
    }
}

/// The stages of a continental or intercontinental competition, in play order.
fn stages_of(c: &Value) -> Vec<Stage> {
    let mut out = Vec::new();
    let list = |key: &str| {
        c.get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    for q in list("qualifying_rounds") {
        out.push(stage(StageKind::Qualifying, &q));
    }
    for key in ["league_phase", "league_stage"] {
        if let Some(l) = c.get(key).filter(|v| v.is_object()) {
            out.push(stage(StageKind::League, l));
        }
    }
    if let Some(g) = c.get("group_stage").filter(|v| v.is_object()) {
        out.push(stage(StageKind::Group, g));
    }
    for k in list("knockout_rounds") {
        out.push(stage(StageKind::Knockout, &k));
    }
    if let Some(f) = c.get("final").filter(|v| v.is_object()) {
        let mut s = stage(StageKind::Final, f);
        s.teams = Some(2);
        out.push(s);
    }
    out
}

/// One association row.
struct Assoc {
    qid: String,
    label: String,
    code: String,
    iso3: String,
    ioc: String,
    confederation: String,
    clubs_with_league: u64,
}

/// One chosen country.
struct CountryIn {
    label: String,
    qid: String,
    country: Country,
}

/// One Wikidata club row, reduced.
struct ClubRow {
    qid: String,
    names: Vec<String>,
    /// Label, native label and official name: the names the packaging scan lists.
    scan: Vec<String>,
    country_qid: String,
    home_nation: String,
    league_qid: String,
}

impl ClubRow {
    /// The club's first name, or its Wikidata id in brackets when it has none.
    fn label(&self) -> String {
        self.names
            .first()
            .cloned()
            .unwrap_or_else(|| format!("({})", self.qid))
    }
}

/// A league being assembled.
struct LeagueBuild {
    key: String,
    qid: Option<String>,
    names: Vec<String>,
    country: String,
    tier: Option<u8>,
    tier_source: &'static str,
    counts: Vec<(u16, &'static str)>,
}

/// The newest ClubElo snapshot of the data folder: `clubelo/clubelo-YYYY-MM-DD.csv`.
fn newest_clubelo(data: &Snapshot) -> anyhow::Result<(String, String)> {
    let mut best = None;
    for f in data.files("clubelo")? {
        let name = f.rsplit('/').next().unwrap_or("");
        let Some(date) = name
            .strip_prefix("clubelo-")
            .and_then(|s| s.strip_suffix(".csv"))
        else {
            continue;
        };
        let b = date.as_bytes();
        let dated = b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
        if dated
            && best
                .as_ref()
                .is_none_or(|(d, _): &(String, String)| date > d.as_str())
        {
            best = Some((date.to_string(), f.clone()));
        }
    }
    best.context("no ClubElo snapshot clubelo/clubelo-YYYY-MM-DD.csv in the data folder")
}

/// The UK home nation of a text ("England (via league)" is "England").
fn home_label(s: &str) -> &str {
    s.split(" (").next().unwrap_or("").trim()
}

/// Builds the world from the snapshots in `data`, the real-name lists in `gate`, and the hand
/// fills.
pub fn build(data: &Snapshot, gate: &Snapshot, fills: &HandFills) -> anyhow::Result<Built> {
    let mut gaps: Vec<Gap> = Vec::new();
    let mut real = RealNames::default();

    // The real names of the gate lists (other than this builder's own) are names no facsimile
    // may take.
    for f in gate.files("real-names")? {
        if !f.ends_with(".json") || f == "real-names/world.json" {
            continue;
        }
        let mut all = Vec::new();
        strings(&gate.json(&f)?, &mut all);
        all.iter().for_each(|n| real.add(n));
    }

    // Hand fills: only facts from Wikipedia pages are accepted.
    let mut fill_pages: BTreeSet<(String, u64)> = BTreeSet::new();
    let mut fill: BTreeMap<(String, String), Value> = BTreeMap::new();
    for f in &fills.fills {
        if f.source != "wikipedia" {
            bail!(
                "hand fill {} {}: source {} is not wikipedia",
                f.entity,
                f.field,
                f.source
            );
        }
        fill_pages.insert((f.page.clone(), f.revision));
        fill.insert((f.entity.clone(), f.field.clone()), f.value.clone());
    }
    let refused_links: BTreeMap<(String, String), String> = fills
        .link_overrides
        .iter()
        .map(|o| ((o.clubelo.clone(), o.wikidata.clone()), o.reason.clone()))
        .collect();

    // Countries.
    let assoc_csv = data.csv("wikidata/associations.csv")?;
    assoc_csv.require(
        "associations.csv",
        &[
            "country_qid",
            "country",
            "fifa_code",
            "iso3",
            "ioc_code",
            "confederation",
        ],
    )?;
    let assocs: Vec<Assoc> = assoc_csv
        .rows
        .iter()
        .map(|r| Assoc {
            qid: assoc_csv.get(r, "country_qid").into(),
            label: assoc_csv.get(r, "country").into(),
            code: assoc_csv.get(r, "fifa_code").into(),
            iso3: assoc_csv.get(r, "iso3").into(),
            ioc: assoc_csv.get(r, "ioc_code").into(),
            confederation: assoc_csv.get(r, "confederation").into(),
            clubs_with_league: assoc_csv
                .get(r, "clubs_with_current_league")
                .parse()
                .unwrap_or(0),
        })
        .collect();
    for r in &assoc_csv.rows {
        real.add(assoc_csv.get(r, "country"));
        real.add(assoc_csv.get(r, "association"));
    }
    let uk_qids: BTreeSet<&str> = assocs
        .iter()
        .filter(|a| a.label == "United Kingdom")
        .map(|a| a.qid.as_str())
        .collect();
    let mut countries: BTreeMap<String, CountryIn> = BTreeMap::new();
    for code in assocs
        .iter()
        .filter(|a| is_code(&a.code))
        .map(|a| a.code.clone())
        .collect::<BTreeSet<_>>()
    {
        let rows: Vec<&Assoc> = assocs.iter().filter(|a| a.code == code).collect();
        // The country of a code is its row with the most clubs; a code shared with a former
        // state or with the United Kingdom (whose territories carry English codes in the
        // source) goes to the country with the clubs.
        let chosen = rows
            .iter()
            .filter(|a| !uk_qids.contains(a.qid.as_str()))
            .max_by_key(|a| a.clubs_with_league)
            .or_else(|| rows.first())
            .expect("a code has a row");
        let mine: Vec<&&Assoc> = rows.iter().filter(|a| a.qid == chosen.qid).collect();
        let region = if mine.iter().any(|a| a.confederation == "UEFA") {
            Some(Region::Europe)
        } else {
            let mut count: BTreeMap<&str, usize> = BTreeMap::new();
            for a in &mine {
                *count.entry(a.confederation.as_str()).or_default() += 1;
            }
            count
                .iter()
                .filter_map(|(c, n)| Some((region_of(c)?, *n)))
                .max_by_key(|(r, n)| (*n, std::cmp::Reverse(*r)))
                .map(|(r, _)| r)
        };
        if region.is_none() {
            gaps.push(Gap {
                kind: "missing-region",
                country: code.clone(),
                entity: "country",
                real_name: chosen.label.clone(),
                id: code.clone(),
                missing: "region",
                reason: "no confederation recorded for the association".into(),
                sources_tried: "associations.csv",
            });
        }
        countries.insert(
            code.clone(),
            CountryIn {
                label: chosen.label.clone(),
                qid: chosen.qid.clone(),
                country: Country {
                    iso3: is_code(&chosen.iso3).then(|| chosen.iso3.clone()),
                    region,
                    uefa: None,
                },
            },
        );
    }
    // A Wikidata country id to its code; the UK has none (its clubs take a home nation).
    let mut qid_code: BTreeMap<String, String> = BTreeMap::new();
    for a in &assocs {
        if is_code(&a.code) && !a.qid.is_empty() && !uk_qids.contains(a.qid.as_str()) {
            qid_code
                .entry(a.qid.clone())
                .or_insert_with(|| a.code.clone());
        }
    }
    let label_code: BTreeMap<String, String> = countries
        .iter()
        .map(|(code, c)| (c.label.clone(), code.clone()))
        .collect();
    let codes: BTreeSet<String> = countries.keys().cloned().collect();
    let any_code = |s: &str| -> Option<String> {
        if codes.contains(s) {
            return Some(s.to_string());
        }
        assocs
            .iter()
            .find(|a| is_code(&a.code) && (a.ioc == s || a.iso3 == s))
            .map(|a| a.code.clone())
    };

    // Competitions (continental and intercontinental) and the UEFA places.
    let comps_json = data.json("competitions/competitions.json")?;
    let mut competitions: BTreeMap<String, Competition> = BTreeMap::new();
    let mut comp_names: BTreeMap<String, String> = BTreeMap::new();
    let mut source_comp_id: BTreeMap<String, String> = BTreeMap::new();
    let mut skipped: Vec<Value> = Vec::new();
    let mut per_region: BTreeMap<String, u8> = BTreeMap::new();
    for c in comps_json["competitions"]
        .as_array()
        .context("competitions.json: no list")?
    {
        let sid = c["id"]
            .as_str()
            .context("competitions.json: a competition without id")?;
        let real_name = c["name"].as_str().unwrap_or("").to_string();
        real.add(&real_name);
        let stages = stages_of(c);
        if stages.is_empty() {
            skipped.push(json!({"source_id": sid, "reason": "no stage structure in the source"}));
            continue;
        }
        let conf = c["confederation"].as_str().unwrap_or("");
        let (scope, region) = if conf == "FIFA" {
            (Scope::Intercontinental, None)
        } else {
            (Scope::Continental, region_of(conf))
        };
        let group = region.map_or("world", Region::code).to_string();
        let level = per_region.entry(group.clone()).or_default();
        *level += 1;
        let name = if region.is_some() {
            format!("{group} cup {level}")
        } else {
            format!("world club cup {level}")
        };
        let id = opaque('k', &format!("competition:{sid}"));
        competitions.insert(
            id.clone(),
            Competition {
                name: facsimile::structural_name(name, &real)?,
                scope,
                region,
                country: None,
                level: *level,
                season: season(&c["season"]),
                entrants: small(c.get("entrants_total")),
                stages,
            },
        );
        comp_names.insert(id.clone(), real_name);
        source_comp_id.insert(sid.to_string(), id);
    }
    let uefa_csv = data.csv("competitions/uefa-ranking-2026-27.csv")?;
    uefa_csv.require(
        "uefa-ranking-2026-27.csv",
        &[
            "rank",
            "country_code",
            "coefficient",
            "top_division_clubs_2026_27",
        ],
    )?;
    let mut uefa_clubs: BTreeMap<String, u16> = BTreeMap::new();
    for r in &uefa_csv.rows {
        let raw = uefa_csv.get(r, "country_code");
        let code = any_code(raw)
            .with_context(|| format!("uefa-ranking-2026-27.csv: no country for {raw}"))?;
        let top = uefa_csv
            .get(r, "top_division_clubs_2026_27")
            .parse::<u16>()
            .ok();
        if let Some(n) = top {
            uefa_clubs.insert(code.clone(), n);
        }
        let mut places = Vec::new();
        for (col, sid) in [
            ("ucl_places_2026_27", "uefa-champions-league"),
            ("uel_places_2026_27", "uefa-europa-league"),
            ("uecl_places_2026_27", "uefa-conference-league"),
        ] {
            if let (Ok(n), Some(id)) = (uefa_csv.get(r, col).parse::<u8>(), source_comp_id.get(sid))
            {
                places.push(Place {
                    competition: id.clone(),
                    clubs: n,
                });
            }
        }
        let entry = UefaEntry {
            rank: uefa_csv.get(r, "rank").parse().context("uefa rank")?,
            coefficient: uefa_csv
                .get(r, "coefficient")
                .parse()
                .context("uefa coefficient")?,
            top_division_clubs: top,
            places,
        };
        if let Some(c) = countries.get_mut(&code) {
            c.country.uefa = Some(entry);
        }
    }

    // Domestic structure of the top 30: tiers and cups.
    let domestic = data.json("competitions/domestic-structure.json")?;
    let domestic = domestic["associations"]
        .as_array()
        .context("domestic-structure.json: no list")?
        .clone();

    // openfootball: league tiers by country, and every club and league name.
    let mut of_leagues: BTreeMap<String, Vec<super::sources::OfLeague>> = BTreeMap::new();
    let mut of_canonical: BTreeSet<String> = BTreeSet::new();
    let mut of_unmapped: Vec<String> = Vec::new();
    for f in data.files("openfootball/leagues")? {
        if !f.ends_with(".leagues.txt") {
            continue;
        }
        let (heading, list) = parse_of_leagues(&data.text(&f)?);
        for l in &list {
            l.names.iter().for_each(|n| real.add(n));
        }
        match heading.as_deref().and_then(|h| label_code.get(h)) {
            Some(code) => of_leagues.entry(code.clone()).or_default().extend(list),
            None => of_unmapped.push(f),
        }
    }
    for f in data.files("openfootball/clubs")? {
        if !f.ends_with(".clubs.txt") {
            continue;
        }
        let text = data.text(&f)?;
        for n in parse_of_clubs(&text) {
            real.add(&n);
        }
        for line in text.lines() {
            let t = line.split('#').next().unwrap_or("");
            if !t.starts_with(char::is_whitespace)
                && !t.trim().starts_with(['=', '|'])
                && let Some(n) = t.split(',').next().map(str::trim)
                && !n.is_empty()
            {
                of_canonical.insert(n.to_string());
            }
        }
    }
    let of_manifest = data.json("openfootball/manifest.json")?;

    // Wikidata leagues and clubs.
    let league_csv = data.csv("wikidata/leagues.csv")?;
    league_csv.require(
        "leagues.csv",
        &[
            "qid",
            "label_en",
            "native_label",
            "official_name",
            "country_qid",
            "home_nation",
            "level",
            "n_teams",
            "level_above_qid",
            "level_below_qid",
        ],
    )?;
    let league_rows: BTreeMap<String, &Vec<String>> = league_csv
        .rows
        .iter()
        .map(|r| (league_csv.get(r, "qid").to_string(), r))
        .collect();
    for r in &league_csv.rows {
        for col in ["label_en", "native_label", "official_name"] {
            real.add(league_csv.get(r, col));
        }
    }
    let club_csv = data.csv("wikidata/clubs.csv")?;
    club_csv.require(
        "clubs.csv",
        &[
            "qid",
            "label_en",
            "native_label",
            "official_name",
            "short_name",
            "aliases_en",
            "country_qid",
            "home_nation",
            "league_qid",
        ],
    )?;
    let clubs_all: BTreeMap<String, ClubRow> = club_csv
        .rows
        .iter()
        .map(|r| {
            let mut names: Vec<String> =
                ["label_en", "native_label", "official_name", "short_name"]
                    .iter()
                    .map(|c| club_csv.get(r, c).to_string())
                    .chain(
                        club_csv
                            .get(r, "aliases_en")
                            .split('|')
                            .map(|s| s.trim().to_string()),
                    )
                    .filter(|s| !s.is_empty())
                    .collect();
            names.dedup();
            let scan = ["label_en", "native_label", "official_name"]
                .iter()
                .map(|c| club_csv.get(r, c).to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let row = ClubRow {
                qid: club_csv.get(r, "qid").into(),
                names,
                scan,
                country_qid: club_csv.get(r, "country_qid").into(),
                home_nation: home_label(club_csv.get(r, "home_nation")).into(),
                league_qid: club_csv.get(r, "league_qid").into(),
            };
            (row.qid.clone(), row)
        })
        .collect();
    for c in clubs_all.values() {
        c.names.iter().for_each(|n| real.add(n));
    }
    // ClubElo and the match file, row by row.
    let (elo_date, elo_file) = newest_clubelo(data)?;
    let elo_csv = data.csv(&elo_file)?;
    elo_csv.require(
        &elo_file,
        &["Club", "Country", "Level", "Elo", "Provisional"],
    )?;
    let match_csv = data.csv("wikidata/clubelo_match.csv")?;
    match_csv.require(
        "clubelo_match.csv",
        &["clubelo_club", "clubelo_country", "wikidata_qid", "method"],
    )?;
    if elo_csv.rows.len() != match_csv.rows.len() {
        bail!(
            "clubelo_match.csv has {} rows; {elo_file} has {}",
            match_csv.rows.len(),
            elo_csv.rows.len()
        );
    }
    struct EloRow {
        club: String,
        country: String,
        level: Option<u8>,
        elo: f64,
        qid: Option<String>,
        review: Option<String>,
        refusal: Option<String>,
    }
    let mut elo_rows: Vec<EloRow> = Vec::new();
    for (i, (e, m)) in elo_csv.rows.iter().zip(&match_csv.rows).enumerate() {
        let club = elo_csv.get(e, "Club").to_string();
        let country = elo_csv.get(e, "Country").to_string();
        if (
            match_csv.get(m, "clubelo_club"),
            match_csv.get(m, "clubelo_country"),
        ) != (club.as_str(), country.as_str())
        {
            bail!(
                "clubelo_match.csv row {} is not the ClubElo row {club} ({country})",
                i + 2
            );
        }
        real.add(&club);
        let method = match_csv.get(m, "method");
        let qid = match_csv.get(m, "wikidata_qid");
        let elo: Option<f64> = elo_csv.get(e, "Elo").parse().ok();
        let mut row = EloRow {
            club: club.clone(),
            country,
            level: elo_csv.get(e, "Level").parse().ok(),
            elo: elo.unwrap_or(0.0),
            qid: None,
            review: None,
            refusal: None,
        };
        let refusal = if elo_csv.get(e, "Provisional") == "true" {
            Some("provisional ClubElo row".to_string())
        } else if elo.is_none_or(|x| x < 1.0) {
            Some(format!(
                "ClubElo rating out of range ({})",
                elo_csv.get(e, "Elo")
            ))
        } else {
            match method {
                "name/alias (exact or token-prefix)" | "ClubElo URL slug" => None,
                "exact name, no ClubElo country" | "exact name, other country" => {
                    row.review = Some(format!("linked by '{method}'"));
                    None
                }
                "unmatched" => Some("unmatched in Wikidata".into()),
                "reserve team" => Some("reserve or B team".into()),
                m if m.starts_with("ambiguous") => {
                    Some(format!("ambiguous: {}", m.trim_start_matches("ambiguous:")))
                }
                m => Some(format!("match method does not link ({m})")),
            }
        };
        if refusal.is_none() && club.contains('\u{fffd}') {
            row.review = Some("ClubElo name has a broken character".into());
        }
        row.refusal = refusal.or_else(|| {
            if qid.is_empty() {
                Some("no Wikidata id".into())
            } else if let Some(why) = refused_links.get(&(club.clone(), qid.to_string())) {
                Some(format!("link refused at the spot-check ({why})"))
            } else if !clubs_all.contains_key(qid) {
                Some(format!("not in the Wikidata club list ({qid})"))
            } else {
                None
            }
        });
        if row.refusal.is_none() {
            row.qid = Some(qid.to_string());
        }
        elo_rows.push(row);
    }
    let mut per_qid: BTreeMap<String, usize> = BTreeMap::new();
    for r in &elo_rows {
        if let Some(q) = &r.qid {
            *per_qid.entry(q.clone()).or_default() += 1;
        }
    }
    for r in &mut elo_rows {
        if r.qid.as_ref().is_some_and(|q| per_qid[q] > 1) {
            r.refusal = Some(format!(
                "two ClubElo rows on one club ({})",
                r.qid.take().unwrap()
            ));
        }
    }
    let linked: BTreeMap<String, usize> = elo_rows
        .iter()
        .enumerate()
        .filter_map(|(i, r)| Some((r.qid.clone()?, i)))
        .collect();

    // The country of a league row.
    // A league with two countries in Wikidata (a cross-border league) has none of its own
    // here; it takes the most common country of its clubs below.
    let league_country = |r: &[String]| -> Option<String> {
        let q = league_csv.get(r, "country_qid");
        if uk_qids.contains(q) {
            label_code
                .get(home_label(league_csv.get(r, "home_nation")))
                .cloned()
        } else {
            qid_code.get(q).cloned()
        }
    };
    // A club's own country: the first of its Wikidata countries with a code.
    let own_code = |q: &str| q.split('|').find_map(|x| qid_code.get(x.trim()).cloned());
    // The country of a club: its own, its UK home nation (from the club, its league, a hand
    // fill, then its ClubElo country), else its league's.
    let club_country = |c: &ClubRow, league: Option<&[String]>| -> Result<String, &'static str> {
        if uk_qids.contains(c.country_qid.as_str()) {
            let from_league = league
                .map(|r| home_label(league_csv.get(r, "home_nation")).to_string())
                .unwrap_or_default();
            let from_fill = fill
                .get(&(format!("club:{}", c.qid), "home_nation".to_string()))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let from_elo = linked
                .get(&c.qid)
                .map(|&i| elo_rows[i].country.clone())
                .filter(|c| ["ENG", "SCO", "WAL", "NIR"].contains(&c.as_str()));
            return [c.home_nation.clone(), from_league, from_fill]
                .iter()
                .find_map(|l| label_code.get(l).cloned())
                .or(from_elo)
                .ok_or("missing-home-nation");
        }
        own_code(&c.country_qid)
            .or_else(|| league.and_then(league_country))
            .ok_or("missing-country")
    };
    // The country of a league with none of its own: the most common country of its clubs
    // (the first code on a tie).
    let mut votes: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for c in clubs_all.values() {
        if let Some(r) = league_rows.get(&c.league_qid)
            && league_country(r).is_none()
            && let Ok(code) = club_country(c, None)
        {
            *votes
                .entry(c.league_qid.clone())
                .or_default()
                .entry(code)
                .or_default() += 1;
        }
    }
    let fallback: BTreeMap<String, String> = votes
        .into_iter()
        .filter_map(|(q, v)| {
            let best = v.values().max().copied()?;
            Some((q, v.into_iter().find(|(_, n)| *n == best)?.0))
        })
        .collect();
    let league_country_of = |q: &str, r: &[String]| -> Option<String> {
        league_country(r).or_else(|| fallback.get(q).cloned())
    };

    // Leagues that hold a club, and their clubs.
    let mut leagues: BTreeMap<String, LeagueBuild> = BTreeMap::new();
    let mut members: Vec<(String, String, String)> = Vec::new(); // (club qid, league key, country)
    let mut no_league: BTreeMap<String, usize> = BTreeMap::new();
    for c in clubs_all.values() {
        let league_row = league_rows.get(&c.league_qid).map(|r| r.as_slice());
        if c.league_qid.is_empty() || league_row.is_none() {
            if !linked.contains_key(&c.qid) {
                let country = club_country(c, None).unwrap_or_else(|_| "(none)".into());
                *no_league.entry(country).or_default() += 1;
            }
            continue;
        }
        let r = league_row.expect("checked");
        let league_country_code = league_country_of(&c.league_qid, r);
        let country = league_country_code
            .as_ref()
            .ok_or("missing-country")
            .and_then(|lc| match club_country(c, Some(r)) {
                Err("missing-country") => Ok(lc.clone()),
                other => other,
            });
        if country.is_err() && linked.contains_key(&c.qid) {
            continue; // placed by its ClubElo level below, or listed as unlinked
        }
        let Some(lc) = league_country_code else {
            gaps.push(Gap {
                kind: "missing-country",
                country: "(none)".into(),
                entity: "club",
                real_name: c.label(),
                id: String::new(),
                missing: "country",
                reason: format!(
                    "neither the club nor its league {} has a country with a code",
                    c.league_qid
                ),
                sources_tried: TRIED_COUNTRY,
            });
            continue;
        };
        let country = match country {
            Ok(code) => code,
            Err(kind) => {
                gaps.push(Gap {
                    kind,
                    country: lc.clone(),
                    entity: "club",
                    real_name: c.label(),
                    id: String::new(),
                    missing: if kind == "missing-country" {
                        "country"
                    } else {
                        "home_nation"
                    },
                    reason: "left out: no country for the club".into(),
                    sources_tried: if kind == "missing-country" {
                        TRIED_COUNTRY
                    } else {
                        TRIED_HOME
                    },
                });
                continue;
            }
        };
        let key = format!("league:{}", c.league_qid);
        leagues.entry(key.clone()).or_insert_with(|| LeagueBuild {
            key: key.clone(),
            qid: Some(c.league_qid.clone()),
            names: ["label_en", "native_label", "official_name"]
                .iter()
                .map(|col| league_csv.get(r, col).to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            country: lc,
            tier: None,
            tier_source: "",
            counts: Vec::new(),
        });
        members.push((c.qid.clone(), key, country));
    }

    // Tiers: Wikidata level, openfootball, domestic structure, hand fill.
    for l in leagues.values_mut() {
        let r: &[String] = league_rows[l.qid.as_ref().expect("a Wikidata league")];
        let level = league_csv
            .get(r, "level")
            .split('|')
            .filter_map(|s| s.trim().parse::<u8>().ok())
            .filter(|t| (1..=20).contains(t))
            .min();
        let folded: BTreeSet<String> = l.names.iter().map(|n| fold(n)).collect();
        let of = of_leagues.get(&l.country).and_then(|list| {
            list.iter()
                .find(|o| o.names.iter().any(|n| folded.contains(&fold(n))))
                .map(|o| o.tier)
        });
        let dom = domestic
            .iter()
            .find(|a| a["code"] == l.country.as_str())
            .and_then(|a| a["tiers"].as_array())
            .and_then(|ts| {
                ts.iter().find(|t| {
                    t["name"]
                        .as_str()
                        .is_some_and(|n| folded.contains(&fold(n)))
                })
            })
            .and_then(|t| small::<u8>(t.get("tier")));
        let hand = fill
            .get(&(l.key.clone(), "tier".to_string()))
            .and_then(Value::as_u64)
            .and_then(|t| u8::try_from(t).ok());
        (l.tier, l.tier_source) = match (level, of, dom, hand) {
            (Some(t), ..) => (Some(t), "wikidata"),
            (None, Some(t), ..) => (Some(t), "openfootball"),
            (None, None, Some(t), _) => (Some(t), "domestic-structure"),
            (None, None, None, Some(t)) => (Some(t), "hand-fill"),
            _ => (None, ""),
        };
        if let Some(n) = league_csv
            .get(r, "n_teams")
            .parse::<u16>()
            .ok()
            .filter(|n| *n > 0)
        {
            l.counts.push((n, "wikidata"));
        }
    }
    // The Wikidata level chain: a league still without a tier, one level below a league of
    // its country with a tier (or above one, below tier 1), takes that tier plus (or minus)
    // one. The chain reads the Wikidata level of every league of the snapshot.
    let level_of = |r: &[String]| -> Option<u8> {
        league_csv
            .get(r, "level")
            .split('|')
            .filter_map(|s| s.trim().parse::<u8>().ok())
            .filter(|t| (1..=20).contains(t))
            .min()
    };
    let mut known: BTreeMap<String, (u8, Option<String>)> = league_rows
        .iter()
        .filter_map(|(q, r)| Some((q.clone(), (level_of(r)?, league_country_of(q, r)))))
        .collect();
    for l in leagues.values() {
        if let (Some(q), Some(t)) = (&l.qid, l.tier) {
            known.insert(q.clone(), (t, Some(l.country.clone())));
        }
    }
    for _ in 0..20 {
        let mut moved = false;
        for l in leagues.values_mut() {
            let (Some(q), None) = (l.qid.clone(), l.tier) else {
                continue;
            };
            let r: &[String] = league_rows[&q];
            let near = |col: &str| -> Vec<(u8, Option<String>)> {
                league_csv
                    .get(r, col)
                    .split('|')
                    .filter_map(|x| known.get(x.trim()).cloned())
                    .filter(|(_, c)| c.as_ref().is_none_or(|c| *c == l.country))
                    .collect()
            };
            let tier = near("level_above_qid")
                .into_iter()
                .map(|(t, _)| t + 1)
                .chain(
                    near("level_below_qid")
                        .into_iter()
                        .map(|(t, _)| t.saturating_sub(1)),
                )
                .filter(|t| (1..=20).contains(t))
                .min();
            if let Some(t) = tier {
                (l.tier, l.tier_source) = (Some(t), "wikidata-chain");
                known.insert(q, (t, Some(l.country.clone())));
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }

    // Domestic tiers: matched by name, else by (country, tier) when one league holds it, else,
    // when no league holds the tier, a league of their own with no club yet.
    for a in &domestic {
        let code = a["code"].as_str().unwrap_or("").to_string();
        for t in a["tiers"].as_array().cloned().unwrap_or_default() {
            let Some(tier) = small::<u8>(t.get("tier")) else {
                continue;
            };
            let name = t["name"].as_str().unwrap_or("").to_string();
            real.add(&name);
            let by_name = leagues
                .values()
                .find(|l| l.country == code && l.names.iter().any(|n| fold(n) == fold(&name)))
                .map(|l| l.key.clone());
            let at_tier: Vec<String> = leagues
                .values()
                .filter(|l| l.country == code && l.tier == Some(tier))
                .map(|l| l.key.clone())
                .collect();
            let key = by_name.or_else(|| (at_tier.len() == 1).then(|| at_tier[0].clone()));
            if key.is_none() && !at_tier.is_empty() {
                continue; // several leagues hold the tier: listed for review below
            }
            let key = key.unwrap_or_else(|| {
                let key = format!("domestic:{code}:{tier}");
                leagues.insert(
                    key.clone(),
                    LeagueBuild {
                        key: key.clone(),
                        qid: None,
                        names: vec![name.clone()],
                        country: code.clone(),
                        tier: Some(tier),
                        tier_source: "domestic-structure",
                        counts: Vec::new(),
                    },
                );
                key
            });
            let l = leagues.get_mut(&key).expect("present");
            if l.tier.is_none() {
                (l.tier, l.tier_source) = (Some(tier), "domestic-structure");
            }
            if let Some(n) = small::<u16>(t.get("clubs")) {
                l.counts.insert(0, (n, "domestic-structure"));
            }
        }
    }
    // A tier the domestic structure names as one entry but the database holds in several
    // leagues is listed for review.
    for a in &domestic {
        let code = a["code"].as_str().unwrap_or("");
        for t in a["tiers"].as_array().cloned().unwrap_or_default() {
            let Some(tier) = small::<u8>(t.get("tier")) else {
                continue;
            };
            let n = leagues
                .values()
                .filter(|l| l.country == code && l.tier == Some(tier))
                .count();
            if n > 1 {
                gaps.push(Gap {
                    kind: "tier-review",
                    country: code.to_string(),
                    entity: "tier",
                    real_name: t["name"].as_str().unwrap_or("").to_string(),
                    id: format!("{code} tier {tier}"),
                    missing: "tier",
                    reason: format!(
                        "domestic-structure.json names one entry for tier {tier}; {n} leagues hold it"
                    ),
                    sources_tried: TRIED_TIER,
                });
            }
        }
    }

    // The UEFA file's top-division count joins tier 1 when one league holds it.
    for (code, n) in &uefa_clubs {
        let top: Vec<String> = leagues
            .values()
            .filter(|l| &l.country == code && l.tier == Some(1))
            .map(|l| l.key.clone())
            .collect();
        if let [key] = top.as_slice() {
            let l = leagues.get_mut(key).expect("present");
            let at = usize::from(
                l.counts
                    .first()
                    .is_some_and(|c| c.1 == "domestic-structure"),
            );
            l.counts.insert(at, (*n, "uefa-ranking"));
        }
    }
    for l in leagues.values_mut() {
        if let Some(v) = fill
            .get(&(l.key.clone(), "club_count".to_string()))
            .and_then(Value::as_u64)
            .and_then(|n| u16::try_from(n).ok())
        {
            l.counts.insert(0, (v, "hand-fill"));
        }
    }

    // ClubElo-linked clubs without a current league: placed by their ClubElo level when one
    // league of their country holds that tier.
    let in_db: BTreeSet<String> = members.iter().map(|m| m.0.clone()).collect();
    let mut unplaced: BTreeMap<usize, String> = BTreeMap::new();
    for (qid, &i) in &linked {
        if in_db.contains(qid) {
            continue;
        }
        let c = &clubs_all[qid];
        let row = &elo_rows[i];
        let country = club_country(c, None)
            .ok()
            .or_else(|| any_code(&row.country));
        let target = country.as_ref().and_then(|code| {
            let at: Vec<&LeagueBuild> = leagues
                .values()
                .filter(|l| &l.country == code && row.level.is_some() && l.tier == row.level)
                .collect();
            (at.len() == 1).then(|| at[0].key.clone())
        });
        match (country, target) {
            (Some(code), Some(key)) => members.push((qid.clone(), key, code)),
            _ => {
                unplaced.insert(
                    i,
                    format!(
                        "no current league and no single league at its ClubElo level ({})",
                        row.level.map_or("none".into(), |l| l.to_string())
                    ),
                );
            }
        }
    }

    // League ids, groups and names.
    let mut league_ids: BTreeMap<String, String> = BTreeMap::new();
    for (key, l) in &leagues {
        let id_key = match &l.qid {
            Some(q) => format!("league:{q}"),
            None => key.clone(),
        };
        league_ids.insert(key.clone(), opaque('l', &id_key));
    }
    let mut world_leagues: BTreeMap<String, League> = BTreeMap::new();
    let mut by_country: BTreeMap<String, Vec<&LeagueBuild>> = BTreeMap::new();
    for l in leagues.values() {
        by_country.entry(l.country.clone()).or_default().push(l);
    }
    for (code, mut list) in by_country {
        list.sort_by_key(|l| (l.tier.unwrap_or(u8::MAX), league_ids[&l.key].clone()));
        let mut untiered = 0;
        for l in &list {
            let same: Vec<&&LeagueBuild> = list
                .iter()
                .filter(|o| o.tier.is_some() && o.tier == l.tier)
                .collect();
            let group = (same.len() > 1).then(|| {
                let at = same.iter().position(|o| o.key == l.key).expect("in list");
                char::from(b'A' + u8::try_from(at % 26).expect("small")).to_string()
            });
            let name = match (l.tier, &group) {
                (Some(t), Some(g)) => format!("{code} tier {t} group {g}"),
                (Some(t), None) => format!("{code} tier {t}"),
                (None, _) => {
                    untiered += 1;
                    format!("{code} other league {untiered}")
                }
            };
            let first = l.counts.first().copied();
            for (n, src) in l.counts.iter().skip(1) {
                if first.is_some_and(|(f, _)| f != *n) {
                    let (f, fsrc) = first.expect("checked");
                    gaps.push(Gap {
                        kind: "club-count-conflict",
                        country: code.clone(),
                        entity: "league",
                        real_name: l.names.first().cloned().unwrap_or_default(),
                        id: league_ids[&l.key].clone(),
                        missing: "club_count",
                        reason: format!("{fsrc} gives {f}, {src} gives {n}; {fsrc} is used"),
                        sources_tried: TRIED_COUNT,
                    });
                }
            }
            if l.tier.is_none() {
                gaps.push(Gap {
                    kind: "missing-tier",
                    country: code.clone(),
                    entity: "league",
                    real_name: l.names.first().cloned().unwrap_or_default(),
                    id: league_ids[&l.key].clone(),
                    missing: "tier",
                    reason: "no source states the league's tier".into(),
                    sources_tried: TRIED_TIER,
                });
            }
            world_leagues.insert(
                league_ids[&l.key].clone(),
                League {
                    country: code.clone(),
                    tier: l.tier,
                    group,
                    name: facsimile::structural_name(name, &real)?,
                    club_count: first.map(|c| c.0),
                    club_count_source: first.map(|c| c.1.to_string()),
                },
            );
        }
    }

    // Strengths.
    let mut rated: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for (qid, key, _) in &members {
        if let Some(&i) = linked.get(qid) {
            rated.entry(key.clone()).or_default().push(elo_rows[i].elo);
        }
    }
    let league_in: BTreeMap<String, LeagueIn> = leagues
        .iter()
        .map(|(key, l)| {
            (
                key.clone(),
                LeagueIn {
                    country: l.country.clone(),
                    tier: l.tier,
                    rated: rated.get(key).cloned().unwrap_or_default(),
                },
            )
        })
        .collect();
    let derived = strength::derive(&league_in);

    // Clubs, named in id order so that a redraw is stable.
    let mut club_ids: Vec<(String, &(String, String, String))> = members
        .iter()
        .map(|m| (opaque('c', &format!("club:{}", m.0)), m))
        .collect();
    club_ids.sort();
    let mut used: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut world_clubs: BTreeMap<String, Club> = BTreeMap::new();
    let mut club_names: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut scan_names: BTreeSet<String> = BTreeSet::new();
    for (id, (qid, key, country)) in &club_ids {
        let c = &clubs_all[qid];
        let mine = used.entry(country.clone()).or_default();
        let (name, short) = facsimile::club_name(qid, mine, &real)?;
        mine.insert(name.clone());
        let league_id = league_ids[key].clone();
        let (elo, source) = match linked.get(qid) {
            Some(&i) => (elo_rows[i].elo, StrengthSource::Clubelo),
            None => derived.by_league[key],
        };
        let real_name = c.label();
        if source != StrengthSource::Clubelo {
            gaps.push(Gap {
                kind: "missing-strength",
                country: country.clone(),
                entity: "club",
                real_name: real_name.clone(),
                id: id.clone(),
                missing: "strength",
                reason: format!(
                    "no ClubElo rating; strength {} by {}",
                    strength::stored(elo),
                    match source {
                        StrengthSource::LeagueMedian => "league-median",
                        StrengthSource::TierStep => "tier-step",
                        _ => "floor",
                    }
                ),
                sources_tried: TRIED_STRENGTH,
            });
        }
        if let Some(&i) = linked.get(qid) {
            let r = &elo_rows[i];
            let tier = world_leagues[&league_id].tier;
            let mut review = r.review.clone();
            if let (Some(lv), Some(t)) = (r.level, tier)
                && lv != t
            {
                review = Some(format!("ClubElo level {lv}, league tier {t}"));
            }
            if let Some(reason) = review {
                gaps.push(Gap {
                    kind: "match-review",
                    country: country.clone(),
                    entity: "club",
                    real_name: format!("{} = {}", r.club, real_name),
                    id: id.clone(),
                    missing: "link check",
                    reason,
                    sources_tried: TRIED_LINK,
                });
            }
        }
        scan_names.extend(c.scan.iter().cloned());
        club_names.insert(id.clone(), c.names.clone());
        world_clubs.insert(
            id.clone(),
            Club {
                name,
                short_name: short,
                country: country.clone(),
                league: league_id,
                elo: strength::stored(elo),
                strength_source: source,
            },
        );
    }
    // ClubElo rows that link no club of the database.
    let mut by_reason: BTreeMap<String, usize> = BTreeMap::new();
    for (i, r) in elo_rows.iter().enumerate() {
        let reason = match (&r.refusal, unplaced.get(&i)) {
            (Some(reason), _) | (None, Some(reason)) => reason.clone(),
            (None, None) => continue,
        };
        let key = reason
            .split([':', '('])
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        *by_reason.entry(key).or_default() += 1;
        gaps.push(Gap {
            kind: "clubelo-unlinked",
            country: if r.country.is_empty() {
                "(none)".into()
            } else {
                r.country.clone()
            },
            entity: "club",
            real_name: r.club.clone(),
            id: String::new(),
            missing: "link",
            reason,
            sources_tried: TRIED_LINK,
        });
    }
    let linked_in_db = elo_rows.iter().filter(|r| r.refusal.is_none()).count() - unplaced.len();
    for (country, n) in &no_league {
        gaps.push(Gap {
            kind: "no-current-league",
            country: country.clone(),
            entity: "club",
            real_name: String::new(),
            id: String::new(),
            missing: "league",
            reason: format!("{n} clubs with no current league in the league list are left out"),
            sources_tried: TRIED_LEAGUE,
        });
    }

    // Domestic cups.
    for a in &domestic {
        let code = a["code"].as_str().unwrap_or("").to_string();
        let region = countries.get(&code).and_then(|c| c.country.region);
        for (i, cup) in a["cups"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            let level = u8::try_from(i + 1).expect("few cups");
            let real_name = cup["name"].as_str().unwrap_or("").to_string();
            real.add(&real_name);
            let entrants = small::<u16>(cup.get("entrants"));
            let id = opaque('k', &format!("cup:{code}:{level}"));
            competitions.insert(
                id.clone(),
                Competition {
                    name: facsimile::structural_name(format!("{code} cup {level}"), &real)?,
                    scope: Scope::DomesticCup,
                    region,
                    country: Some(code.clone()),
                    level,
                    season: season(&cup["season"]),
                    entrants,
                    stages: vec![Stage {
                        kind: StageKind::Knockout,
                        teams: entrants,
                        legs: legs_of(&cup["legs"]),
                        matches_per_team: None,
                        teams_entering: None,
                        groups: None,
                    }],
                },
            );
            comp_names.insert(id, real_name);
        }
    }

    // Sources and credits.
    let mut sources = Vec::new();
    let mut of_repos = Vec::new();
    for r in of_manifest["repos"].as_array().cloned().unwrap_or_default() {
        let repo = r["repo"].as_str().unwrap_or("").to_string();
        let commit = r["commit"].as_str().unwrap_or("").to_string();
        sources.push(Source {
            key: format!("openfootball:{}", repo.trim_start_matches("openfootball/")),
            title: format!("{repo} (openfootball)"),
            licence: r["license"].as_str().unwrap_or("CC0-1.0").to_string(),
            url: r["url"]
                .as_str()
                .unwrap_or("https://github.com/openfootball")
                .to_string(),
            snapshot: commit.clone(),
            revisions: Vec::new(),
        });
        of_repos.push(json!({"repo": repo, "commit": commit}));
    }
    sources.push(Source {
        key: "clubelo".into(),
        title: "ClubElo, by Lars Schiefler".into(),
        licence: "credited reuse; commercial use not addressed".into(),
        url: "https://clubelo.com".into(),
        snapshot: elo_date.clone(),
        revisions: Vec::new(),
    });
    let wikidata_date = if data.exists("wikidata/manifest.json") {
        data.json("wikidata/manifest.json")?["built_utc"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(10)
            .collect()
    } else {
        String::new()
    };
    sources.push(Source {
        key: "wikidata".into(),
        title: "Wikidata".into(),
        licence: "CC0 1.0".into(),
        url: "https://www.wikidata.org".into(),
        snapshot: if wikidata_date.is_empty() {
            "unknown".into()
        } else {
            wikidata_date
        },
        revisions: Vec::new(),
    });
    let comp_manifest = data.json("competitions/manifest.json")?;
    let mut pages: BTreeSet<(String, u64)> = fill_pages.clone();
    for p in comp_manifest["pages_used"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        if let (Some(page), Some(rev)) = (p["page"].as_str(), p["revid"].as_u64()) {
            pages.insert((page.to_string(), rev));
        }
    }
    let revisions: BTreeSet<u64> = pages.iter().map(|p| p.1).collect();
    sources.push(Source {
        key: "wikipedia".into(),
        title: "Wikipedia contributors (facts only)".into(),
        licence: "CC BY-SA 4.0".into(),
        url: "https://en.wikipedia.org".into(),
        snapshot: comp_manifest["fetch_window_utc"]["last"]
            .as_str()
            .unwrap_or("unknown")
            .chars()
            .take(10)
            .collect(),
        revisions: revisions.into_iter().collect(),
    });

    let world = WorldDatabase {
        schema_version: WORLD_VERSION,
        sources,
        countries: countries
            .iter()
            .map(|(k, c)| (k.clone(), c.country.clone()))
            .collect(),
        leagues: world_leagues,
        clubs: world_clubs,
        competitions,
    };

    // The name map: every real name compared against, by content id where there is one.
    let mut league_names = BTreeMap::new();
    for (key, l) in &leagues {
        league_names.insert(league_ids[key].clone(), l.names.clone());
    }
    let in_map: BTreeSet<&String> = club_names.values().flatten().collect();
    let mut other: BTreeSet<String> = BTreeSet::new();
    for c in clubs_all.values() {
        if !in_db.contains(&c.qid) {
            other.extend(c.names.iter().filter(|n| !in_map.contains(n)).cloned());
        }
    }
    other.extend(elo_rows.iter().map(|r| r.club.clone()));
    other.extend(of_canonical.iter().cloned());
    let name_map = json!({
        "clubs": club_names,
        "leagues": league_names,
        "competitions": comp_names,
        "countries": countries.iter().map(|(k, c)| (k.clone(), json!({"name": c.label, "wikidata": c.qid}))).collect::<BTreeMap<_, _>>(),
        "other": other,
    });

    // Scan names: labels, native labels and official names of the clubs in the database,
    // ClubElo names and openfootball canonical names, of 4 or more characters.
    scan_names.extend(elo_rows.iter().map(|r| r.club.clone()));
    scan_names.extend(of_canonical.iter().cloned());
    scan_names.retain(|n| n.trim().chars().count() >= 4);

    let tiers: BTreeSet<(String, u8)> = world
        .leagues
        .values()
        .filter_map(|l| Some((l.country.clone(), l.tier?)))
        .collect();
    let mut tier_sources: BTreeMap<&str, usize> = BTreeMap::new();
    for l in leagues.values() {
        *tier_sources
            .entry(if l.tier.is_some() {
                l.tier_source
            } else {
                "none"
            })
            .or_default() += 1;
    }
    let coverage = json!({
        "builder_version": BUILDER_VERSION,
        "countries": world.countries.keys().collect::<Vec<_>>(),
        "tiers": tiers.iter().map(|(c, t)| json!([c, t])).collect::<Vec<_>>(),
        "clubelo": {
            "file": elo_file,
            "rows": elo_rows.len(),
            "linked": linked_in_db,
            "on_gap_list": elo_rows.len() - linked_in_db,
            "by_reason": by_reason,
        },
        "strength": {
            "tier_drop": derived.tier_drop,
            "floor": derived.floor,
        },
        "wikipedia_pages": pages.iter().map(|(p, r)| json!({"page": p, "revision": r})).collect::<Vec<_>>(),
        "openfootball": of_repos,
        "openfootball_unmapped_league_files": of_unmapped,
        "competitions_skipped": skipped,
        "tiers_by_source": tier_sources,
        "real_names_compared": real.len(),
    });
    gaps.sort();
    Ok(Built {
        world,
        gaps,
        name_map,
        scan_names,
        coverage,
    })
}

/// Every string value of a JSON value.
pub fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_give_numbers_seasons_and_legs() {
        assert_eq!(leading_number("8,506 (main-article infobox)"), Some(8506));
        assert_eq!(leading_number("about 10"), None);
        assert_eq!(
            season(&json!("2026 (played 12 Aug 2026)")),
            Some("2026".into())
        );
        assert_eq!(
            season(&json!("2026-27 (4 Sep 2026-May 2027)")),
            Some("2026-27".into())
        );
        assert_eq!(season(&json!("next year")), None);
        assert_eq!(legs_of(&json!("single match")), Some(1));
        assert_eq!(legs_of(&json!("two legs")), Some(2));
        assert_eq!(legs_of(&json!(2)), Some(2));
        assert_eq!(legs_of(&Value::Null), None);
    }

    #[test]
    fn an_opaque_id_is_its_prefix_and_ten_hex_digits() {
        let id = opaque('c', "club:Q1");
        assert_eq!(id.len(), 12);
        assert!(id.starts_with("c-"));
        assert_eq!(id, opaque('c', "club:Q1"));
    }

    #[test]
    fn stages_come_in_play_order_without_their_prose() {
        let c = json!({
            "qualifying_rounds": [{"round": "First", "teams": 4, "legs": 2, "teams_entering": 4, "note": "x"}],
            "group_stage": {"groups": 2, "teams": 8, "matches_per_team": 3, "format": "single round-robin"},
            "knockout_rounds": [{"round": "Semi-finals", "teams": 4, "legs": 1}],
            "final": {"legs": 1, "venue": "somewhere"}
        });
        let s = stages_of(&c);
        let kinds: Vec<StageKind> = s.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            [
                StageKind::Qualifying,
                StageKind::Group,
                StageKind::Knockout,
                StageKind::Final
            ]
        );
        assert_eq!(s[1].groups, Some(2));
        assert_eq!(s[3].teams, Some(2));
    }
}
