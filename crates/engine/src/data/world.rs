//! The world database (`content/world.json`): every country and league tier the open sources
//! cover, each club with a generated facsimile name and an Elo strength, and the continental,
//! intercontinental and domestic cup competitions as structure only. No match reads it;
//! `engine-cli world-database build` writes it from local snapshots, never by hand.
//!
//! Nothing in the file names a real club, league, competition or country: a country is its
//! FIFA code, a league or cup is named from that code, and the real names stay in `gate/`,
//! which never ships. Every collection is an object keyed by an opaque id, so a key-level
//! diff of the file gives one stable key per field.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::EngineError;
use crate::data::{Loaded, load_json_bytes};

/// The world database's file name in the content folder.
pub const WORLD_FILE: &str = "world.json";
/// Schema version this build reads.
pub const WORLD_VERSION: u32 = 1;

/// The whole world database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct WorldDatabase {
    #[garde(skip)]
    pub schema_version: u32,
    /// The credited sources, each with its licence.
    #[garde(length(min = 1), dive)]
    pub sources: Vec<Source>,
    /// Countries by FIFA code.
    #[garde(dive)]
    pub countries: BTreeMap<String, Country>,
    /// Leagues by id (`l-` and 10 hex digits).
    #[garde(dive)]
    pub leagues: BTreeMap<String, League>,
    /// Clubs by id (`c-` and 10 hex digits).
    #[garde(dive)]
    pub clubs: BTreeMap<String, Club>,
    /// Competitions by id (`k-` and 10 hex digits).
    #[garde(dive)]
    pub competitions: BTreeMap<String, Competition>,
}

/// One credited source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// A short key, for example `clubelo`.
    #[garde(length(min = 1, max = 64))]
    pub key: String,
    /// The credit line.
    #[garde(length(min = 1, max = 200))]
    pub title: String,
    #[garde(length(min = 1, max = 100))]
    pub licence: String,
    #[garde(length(min = 1, max = 200))]
    pub url: String,
    /// The snapshot date or commit the build read.
    #[garde(length(min = 1, max = 200))]
    pub snapshot: String,
    /// Page revisions read (Wikipedia); empty for the other sources.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(skip)]
    pub revisions: Vec<u64>,
}

/// A geographic region (from the confederation; not an organisation's name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Region {
    Europe,
    Asia,
    Africa,
    SouthAmerica,
    NorthCentralAmerica,
    Oceania,
}

impl Region {
    /// The region as the file writes it.
    pub fn code(self) -> &'static str {
        match self {
            Region::Europe => "europe",
            Region::Asia => "asia",
            Region::Africa => "africa",
            Region::SouthAmerica => "south-america",
            Region::NorthCentralAmerica => "north-central-america",
            Region::Oceania => "oceania",
        }
    }
}

/// One country, keyed by its FIFA code; it carries no display name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Country {
    /// The ISO 3166 alpha-3 code, when the sources give one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub iso3: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub region: Option<Region>,
    /// The UEFA association ranking entry, for the 55 UEFA associations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(dive)]
    pub uefa: Option<UefaEntry>,
}

/// A UEFA association's 2025 ranking entry and its 2026/27 places.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UefaEntry {
    #[garde(range(min = 1, max = 60))]
    pub rank: u8,
    #[garde(range(min = 0.0, max = 200.0))]
    pub coefficient: f64,
    /// The real club count of the top division in 2026/27.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub top_division_clubs: Option<u16>,
    #[garde(dive)]
    pub places: Vec<Place>,
}

/// Places of one association in one competition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Place {
    /// The competition's id.
    #[garde(skip)]
    pub competition: String,
    /// The base places (extra places for a title holder or a performance spot are not
    /// counted).
    #[garde(range(max = 10))]
    pub clubs: u8,
}

/// One league.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct League {
    /// The country's FIFA code.
    #[garde(skip)]
    pub country: String,
    /// The tier, 1 at the top; absent when no source states it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub tier: Option<u8>,
    /// The group of a parallel regional league ("A", "B", ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub group: Option<String>,
    /// The facsimile name, built from the country code.
    #[garde(length(min = 1, max = 64))]
    pub name: String,
    /// The real club count, when a source states it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub club_count: Option<u16>,
    /// The source of the club count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub club_count_source: Option<String>,
}

/// Where a club's strength comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StrengthSource {
    /// The club's own ClubElo rating.
    Clubelo,
    /// The median rating of the rated clubs of its league.
    LeagueMedian,
    /// The nearest rated tier above, less the world's median drop per tier.
    TierStep,
    /// The 10th percentile of all rated clubs.
    Floor,
}

/// One club.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Club {
    /// The facsimile name.
    #[garde(length(min = 1, max = 64))]
    pub name: String,
    #[garde(length(min = 1, max = 8))]
    pub short_name: String,
    /// The country's FIFA code.
    #[garde(skip)]
    pub country: String,
    /// The league's id.
    #[garde(skip)]
    pub league: String,
    /// The Elo strength.
    #[garde(range(min = 1, max = 3000))]
    pub elo: u16,
    #[garde(skip)]
    pub strength_source: StrengthSource,
}

/// What a competition is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    Continental,
    Intercontinental,
    DomesticCup,
}

/// One competition, as structure only: no fixture, result, venue or prose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Competition {
    /// The facsimile name.
    #[garde(length(min = 1, max = 64))]
    pub name: String,
    #[garde(skip)]
    pub scope: Scope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub region: Option<Region>,
    /// The country's FIFA code, for a domestic cup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub country: Option<String>,
    /// 1 for the first competition of its region or country, and so on.
    #[garde(range(min = 1, max = 20))]
    pub level: u8,
    /// The season the structure describes ("2026-27" or "2026").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub season: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub entrants: Option<u16>,
    #[garde(length(min = 1), dive)]
    pub stages: Vec<Stage>,
}

/// The kind of a competition stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StageKind {
    Qualifying,
    League,
    Group,
    Knockout,
    Final,
}

/// One stage of a competition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    #[garde(skip)]
    pub kind: StageKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub teams: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub legs: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub matches_per_team: Option<u8>,
    /// Clubs that enter the competition at this stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub teams_entering: Option<u16>,
    /// Groups (or regions) the stage is split into.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub groups: Option<u8>,
}

/// Reads and checks a world database: the version, the types, the bounds, the id forms, and
/// that every reference resolves.
pub fn load_world(bytes: &[u8], shown: &str) -> Result<Loaded<WorldDatabase>, EngineError> {
    let loaded: Loaded<WorldDatabase> = load_json_bytes("world", bytes, shown, WORLD_VERSION, &())?;
    check(&loaded.value).map_err(|(field, reason)| EngineError::Data {
        kind: "world",
        path: shown.to_string(),
        field,
        reason,
    })?;
    Ok(loaded)
}

/// `prefix` and 10 lowercase hex digits.
fn is_id(id: &str, prefix: char) -> bool {
    let mut chars = id.chars();
    chars.next() == Some(prefix)
        && chars.next() == Some('-')
        && id.len() == 12
        && id[2..]
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// Three capital letters.
fn is_code(code: &str) -> bool {
    code.len() == 3 && code.chars().all(|c| c.is_ascii_uppercase())
}

/// The first fault of `w` beyond its types and bounds: (field, reason).
fn check(w: &WorldDatabase) -> Result<(), (String, String)> {
    let fault = |field: String, reason: &str| Err((field, reason.to_string()));
    for (code, c) in &w.countries {
        if !is_code(code) {
            return fault(
                format!("countries.{code}"),
                "a country key is 3 capital letters",
            );
        }
        if c.iso3.as_deref().is_some_and(|i| !is_code(i)) {
            return fault(format!("countries.{code}.iso3"), "3 capital letters");
        }
        for (i, p) in c.uefa.iter().flat_map(|u| u.places.iter()).enumerate() {
            if !w.competitions.contains_key(&p.competition) {
                return fault(
                    format!("countries.{code}.uefa.places[{i}].competition"),
                    "names no competition of the file",
                );
            }
        }
    }
    for (id, l) in &w.leagues {
        if !is_id(id, 'l') {
            return fault(
                format!("leagues.{id}"),
                "a league id is l- and 10 hex digits",
            );
        }
        if !w.countries.contains_key(&l.country) {
            return fault(
                format!("leagues.{id}.country"),
                "names no country of the file",
            );
        }
        if l.tier.is_some_and(|t| !(1..=20).contains(&t)) {
            return fault(format!("leagues.{id}.tier"), "a tier is 1 to 20");
        }
        if l.club_count.is_some() != l.club_count_source.is_some() {
            return fault(
                format!("leagues.{id}.club_count_source"),
                "a club count carries its source",
            );
        }
    }
    for (id, c) in &w.clubs {
        if !is_id(id, 'c') {
            return fault(format!("clubs.{id}"), "a club id is c- and 10 hex digits");
        }
        if !w.countries.contains_key(&c.country) {
            return fault(
                format!("clubs.{id}.country"),
                "names no country of the file",
            );
        }
        if !w.leagues.contains_key(&c.league) {
            return fault(format!("clubs.{id}.league"), "names no league of the file");
        }
        if c.name.trim().is_empty() || c.short_name.trim().is_empty() {
            return fault(format!("clubs.{id}.name"), "a name is not blank");
        }
    }
    for (id, k) in &w.competitions {
        if !is_id(id, 'k') {
            return fault(
                format!("competitions.{id}"),
                "a competition id is k- and 10 hex digits",
            );
        }
        if k.country
            .as_ref()
            .is_some_and(|c| !w.countries.contains_key(c))
        {
            return fault(
                format!("competitions.{id}.country"),
                "names no country of the file",
            );
        }
        if (k.scope == Scope::DomesticCup) != k.country.is_some() {
            return fault(
                format!("competitions.{id}.country"),
                "a domestic cup, and only a domestic cup, names its country",
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"{
  "schema_version": 1,
  "sources": [{"key": "x", "title": "X", "licence": "CC0", "url": "https://x", "snapshot": "2026-10-11"}],
  "countries": {"TST": {"iso3": "TST", "region": "europe"}},
  "leagues": {"l-0123456789": {"country": "TST", "tier": 1, "name": "TST tier 1", "club_count": 2, "club_count_source": "wikidata"}},
  "clubs": {"c-0123456789": {"name": "Ashmere Town", "short_name": "ASH", "country": "TST", "league": "l-0123456789", "elo": 1500, "strength_source": "clubelo"}},
  "competitions": {"k-0123456789": {"name": "TST cup 1", "scope": "domestic-cup", "country": "TST", "level": 1, "stages": [{"kind": "knockout", "legs": 1}]}}
}"#;

    fn refused(text: &str) -> String {
        load_world(text.as_bytes(), "world.json")
            .expect_err("refused")
            .to_string()
    }

    #[test]
    fn a_minimal_file_loads_and_writes_its_version_first() {
        let w = load_world(MINIMAL.as_bytes(), "world.json").unwrap().value;
        assert_eq!(
            w.clubs["c-0123456789"].strength_source,
            StrengthSource::Clubelo
        );
        let text = serde_json::to_string(&w).unwrap();
        assert!(text.starts_with("{\"schema_version\":1,"), "{text}");
    }

    #[test]
    fn a_dangling_league_is_refused_by_name() {
        let e = refused(&MINIMAL.replace(
            "\"league\": \"l-0123456789\"",
            "\"league\": \"l-ffffffffff\"",
        ));
        assert!(e.contains("clubs.c-0123456789.league"), "{e}");
    }

    #[test]
    fn a_malformed_id_is_refused() {
        let e = refused(&MINIMAL.replace("\"c-0123456789\"", "\"c-XYZ\""));
        assert!(e.contains("clubs.c-XYZ"), "{e}");
    }

    #[test]
    fn a_country_with_a_name_is_refused() {
        let e = refused(&MINIMAL.replace(
            "{\"iso3\": \"TST\",",
            "{\"name\": \"Testland\", \"iso3\": \"TST\",",
        ));
        assert!(e.contains("name"), "{e}");
    }

    #[test]
    fn a_negative_elo_is_refused() {
        let e = refused(&MINIMAL.replace("\"elo\": 1500", "\"elo\": -28"));
        assert!(e.contains("-28"), "{e}");
    }

    #[test]
    fn a_competition_without_stages_is_refused() {
        let e = refused(&MINIMAL.replace(
            "\"stages\": [{\"kind\": \"knockout\", \"legs\": 1}]",
            "\"stages\": []",
        ));
        assert!(e.contains("stages"), "{e}");
    }
}
