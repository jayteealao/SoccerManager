//! The band registry, read from `realism-bands.json` in the content folder. The bands are
//! acceptance criteria, never tuning values: the calibration harness reads them and nothing
//! writes them.
//!
//! The file is a list of bands (version 3). Each band names its measure (a mean, a share,
//! or a pooled ratio of per-match fields from a closed catalog, or one of three built-in
//! measures), the suites that judge it, its range, and the smallest shift that matters. A
//! new band needs only a new entry. A version 2 file, one fixed field per band, still loads:
//! it migrates in memory to the same ranges, each smallest shift a quarter of its range.

use engine::EngineError;
use engine::data::{ContentDir, load_json_bytes, read_bytes};
use garde::Validate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{RED_CARD_FLOOR, RED_CARD_LIMIT, Suite};

/// The file name inside the content folder.
pub const BANDS_FILE: &str = "realism-bands.json";
/// The layout version this build writes and reads.
pub const REGISTRY_VERSION: u32 = 3;
/// The older layout this build migrates on load.
pub const LEGACY_VERSION: u32 = 2;

const KIND: &str = "realism bands";

/// A closed range, `lo` to `hi`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[garde(allow_unvalidated)]
pub struct Band {
    pub lo: f64,
    #[garde(custom(at_least(self.lo)))]
    pub hi: f64,
}

impl Band {
    pub fn contains(&self, value: f64) -> bool {
        (self.lo..=self.hi).contains(&value)
    }

    /// A floor band starts at 0, so only a value above `hi` is a miss.
    pub fn is_floor(&self) -> bool {
        self.lo == 0.0 && self.hi > 0.0
    }
}

fn at_least(lo: f64) -> impl FnOnce(&f64, &()) -> garde::Result {
    move |hi, _| {
        if *hi >= lo {
            Ok(())
        } else {
            Err(garde::Error::new(format!("hi {hi} is below lo {lo}")))
        }
    }
}

/// The strength suite's setting: the stronger club has every attribute times
/// `attribute_boost`. Its criterion is the band `stronger_team_win_rate`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StrongerTeam {
    #[garde(range(min = 1.0, max = 2.0))]
    pub attribute_boost: f64,
}

/// Over what a mean is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Per {
    /// The sum of the fields in each match, averaged over matches.
    Match,
    /// Each team's sum of the fields (`{side}` in a field name), averaged over teams.
    Team,
}

/// How a share compares a match's sum of fields with its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    #[serde(rename = ">=")]
    AtLeast,
    #[serde(rename = "==")]
    Equal,
    #[serde(rename = "<=")]
    AtMost,
}

impl Op {
    pub fn holds(self, sum: f64, value: f64) -> bool {
        match self {
            Op::AtLeast => sum >= value,
            Op::Equal => sum == value,
            Op::AtMost => sum <= value,
        }
    }
}

/// The measures that compare arms or sides of a suite, and so are not a sum of one match's
/// fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Builtin {
    /// The strength suite: the share of matches the boosted club wins. It passes above `lo`.
    StrongerTeamWinRate,
    /// The red-card suite, per arm: the reduced side's goals minus the full side's, per
    /// match. It passes at or below `hi`.
    ReducedMinusFull,
    /// The red-card suite, per arm: the full side's goals per match over the control's home
    /// goals per match. It passes at or below `hi`.
    FullOverControl,
}

/// A band's measure, over the per-match fields of the field catalog
/// ([`super::measures::FIELDS`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Measure {
    /// The mean of the per-match (or per-team) sum of `of`.
    Mean {
        per: Per,
        of: Vec<String>,
    },
    /// The share of matches whose sum of `of` stands `op` `value`.
    Share {
        of: Vec<String>,
        op: Op,
        value: f64,
    },
    /// The sum of `num` over the sum of `den`, pooled over the suite.
    Ratio {
        num: Vec<String>,
        den: Vec<String>,
    },
    Builtin {
        name: Builtin,
    },
}

/// One band of the registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BandDef {
    pub band: String,
    /// The suites that judge it: `equal`, `strength`, `formations` (per pairing), and
    /// `red-card` (per arm).
    pub suites: Vec<String>,
    pub measure: Measure,
    pub lo: f64,
    pub hi: f64,
    /// The smallest change of the measure that matters: a change run plays until it can
    /// see a change this size with 80 percent power, or reports the band not sure.
    pub smallest_shift: f64,
    /// Bands that belong together, such as the two possession shares.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

impl BandDef {
    pub fn range(&self) -> Band {
        Band {
            lo: self.lo,
            hi: self.hi,
        }
    }

    /// `true` when `suite` judges the band.
    pub fn judged_in(&self, suite: Suite) -> bool {
        self.suites.iter().any(|s| s == suite.code())
    }

    /// A short digest of everything that decides the band's verdict: its measure, suites,
    /// range and smallest shift.
    pub fn digest(&self) -> String {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        engine::data::hex12(&Sha256::digest(bytes))
    }
}

/// The band registry: the suite settings and the bands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    #[garde(skip)]
    pub schema_version: u32,
    /// Matches per suite the wall-time budget is set for.
    #[garde(range(min = 1, max = 100_000))]
    pub sample_size: u32,
    /// The wall-time budget of one suite of `sample_size` matches, in minutes.
    #[garde(range(min = 0.1, max = 10_000.0))]
    pub wall_minutes_per_sample: f64,
    #[garde(dive)]
    pub stronger_team: StrongerTeam,
    #[garde(skip)]
    pub bands: Vec<BandDef>,
    /// The layout version the file was migrated from; `None` for a version 3 file.
    #[serde(skip)]
    #[garde(skip)]
    pub migrated_from: Option<u32>,
}

impl Registry {
    /// Loads the registry from the content folder: a version 3 file as it is, a version 2
    /// file through the migration. Any other version is refused, as is a band with an
    /// unknown field, an unknown catalog field, no fields, an inverted range, a smallest
    /// shift that is not positive, an unknown suite, or a name used twice; the refusal names
    /// the file and the field.
    pub fn load(dir: &ContentDir) -> anyhow::Result<Self> {
        let bytes = read_bytes(&dir.path(BANDS_FILE), BANDS_FILE)?;
        Ok(Self::from_bytes(&bytes)?)
    }

    /// [`Self::load`] over bytes already in memory.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EngineError> {
        let version = serde_json::from_slice::<serde_json::Value>(bytes)
            .ok()
            .and_then(|v| v["schema_version"].as_u64());
        let registry = if version == Some(u64::from(LEGACY_VERSION)) {
            load_json_bytes::<BandsV2>(KIND, bytes, BANDS_FILE, LEGACY_VERSION, &())?
                .value
                .migrate()
        } else {
            load_json_bytes::<Registry>(KIND, bytes, BANDS_FILE, REGISTRY_VERSION, &())?.value
        };
        registry
            .check()
            .map_err(|(field, reason)| EngineError::Data {
                kind: KIND,
                path: BANDS_FILE.to_string(),
                field,
                reason,
            })?;
        Ok(registry)
    }

    /// The registry's own rules, beyond its layout. A refusal names the field.
    fn check(&self) -> Result<(), (String, String)> {
        let mut seen = std::collections::BTreeSet::new();
        for (i, b) in self.bands.iter().enumerate() {
            let at = |field: &str| format!("bands[{i}] ({}).{field}", b.band);
            if b.band.trim().is_empty() {
                return Err((format!("bands[{i}].band"), "is empty".into()));
            }
            if !seen.insert(b.band.as_str()) {
                return Err((at("band"), "names a band already in the file".into()));
            }
            if !(b.lo.is_finite() && b.hi.is_finite()) || b.hi < b.lo {
                return Err((at("hi"), format!("hi {} is below lo {}", b.hi, b.lo)));
            }
            if !(b.smallest_shift.is_finite() && b.smallest_shift > 0.0) {
                return Err((
                    at("smallest_shift"),
                    format!("{} is not above 0", b.smallest_shift),
                ));
            }
            if b.suites.is_empty() {
                return Err((at("suites"), "names no suite".into()));
            }
            for s in &b.suites {
                if Suite::parse(s).is_none() {
                    return Err((
                        at("suites"),
                        format!("{s} is not a suite (equal, strength, formations, red-card)"),
                    ));
                }
            }
            super::measures::check(&b.measure).map_err(|why| (at("measure"), why))?;
        }
        Ok(())
    }

    /// The band named `name`.
    pub fn band(&self, name: &str) -> Option<&BandDef> {
        self.bands.iter().find(|b| b.band == name)
    }

    /// The range of the band `name`, if the registry has it.
    pub fn range(&self, name: &str) -> Option<Band> {
        self.band(name).map(BandDef::range)
    }

    /// Every band name, in file order.
    pub fn names(&self) -> Vec<&str> {
        self.bands.iter().map(|b| b.band.as_str()).collect()
    }

    /// The suites that judge `band`, in the order of [`Suite`]; `None` for an unknown band.
    pub fn suites_of(&self, band: &str) -> Option<Vec<Suite>> {
        let def = self.band(band)?;
        let mut out: Vec<Suite> = def.suites.iter().filter_map(|s| Suite::parse(s)).collect();
        out.sort();
        out.dedup();
        Some(out)
    }

    /// SHA-256, as 12 hex characters, over every band in file order: a changed range, shift,
    /// measure or suite changes it.
    pub fn digest(&self) -> String {
        let bytes = serde_json::to_vec(&self.bands).unwrap_or_default();
        engine::data::hex12(&Sha256::digest(bytes))
    }

    /// The wall-time budget of a suite of `matches` matches, in milliseconds.
    pub fn wall_budget_ms(&self, matches: u32) -> u64 {
        let per_match = self.wall_minutes_per_sample * 60_000.0 / f64::from(self.sample_size);
        // At most 100 000 matches of at most 10 000 minutes: far inside u64.
        (per_match * f64::from(matches)).round() as u64
    }
}

/// The version 2 layout: one fixed field per band.
#[derive(Debug, Clone, PartialEq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
struct BandsV2 {
    #[garde(skip)]
    schema_version: u32,
    #[garde(range(min = 1, max = 100_000))]
    sample_size: u32,
    #[garde(dive)]
    goals_per_match: Band,
    #[garde(dive)]
    shots_per_team: Band,
    #[garde(dive)]
    possession_pct: Band,
    #[garde(dive)]
    stronger_team: StrongerTeamV2,
    #[garde(range(min = 0.1, max = 10_000.0))]
    wall_minutes_per_sample: f64,
    #[garde(dive)]
    ten_plus_goals_share: Band,
    #[garde(dive)]
    sending_off_share: Band,
    #[garde(dive)]
    yellow_cards_per_team: Band,
    #[garde(dive)]
    shots_on_target_share: Band,
    #[garde(dive)]
    goals_per_xg: Band,
    #[garde(dive)]
    passes_per_team: Band,
    #[garde(dive)]
    pass_accuracy_pct: Band,
    #[garde(dive)]
    corners_per_team: Band,
    #[garde(dive)]
    throw_ins_per_match: Band,
    #[garde(dive)]
    goal_kicks_per_match: Band,
    #[garde(dive)]
    goalless_share: Band,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
struct StrongerTeamV2 {
    #[garde(range(min = 1.0, max = 2.0))]
    attribute_boost: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    min_win_rate: f64,
}

fn fields(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}

const GOALS: [&str; 2] = ["goals.home", "goals.away"];

impl BandsV2 {
    /// The same bands as a registry: the same ranges, each smallest shift a quarter of its
    /// range, possession as its home and away rows, the stronger team and the red-card pair
    /// as built-in bands.
    fn migrate(self) -> Registry {
        let def = |band: &str, suites: &[&str], measure: Measure, range: Band| BandDef {
            band: band.to_string(),
            suites: fields(suites),
            measure,
            lo: range.lo,
            hi: range.hi,
            // Rounded to 9 places, so 0.8 / 4 reads 0.2 as the shipped file writes it.
            smallest_shift: engine::observe::round_to((range.hi - range.lo) / 4.0, 9),
            group: None,
        };
        let mean = |per: Per, of: &[&str]| Measure::Mean {
            per,
            of: fields(of),
        };
        let share = |op: Op, value: f64| Measure::Share {
            of: fields(&GOALS),
            op,
            value,
        };
        let equal = &["equal"][..];
        let goals = &["equal", "formations"][..];
        let possession = |side: &str| {
            let mut d = def(
                &format!("possession_{side}_pct"),
                equal,
                mean(Per::Match, &[&format!("possession_pct.{side}")]),
                self.possession_pct,
            );
            d.group = Some("possession_pct".into());
            d
        };
        let bands = vec![
            def(
                "goals_per_match",
                goals,
                mean(Per::Match, &GOALS),
                self.goals_per_match,
            ),
            def(
                "shots_per_team",
                equal,
                mean(Per::Team, &["shots.{side}"]),
                self.shots_per_team,
            ),
            possession("home"),
            possession("away"),
            def(
                "ten_plus_goals_share",
                goals,
                share(Op::AtLeast, 10.0),
                self.ten_plus_goals_share,
            ),
            def(
                "sending_off_share",
                equal,
                Measure::Share {
                    of: fields(&["red.home", "red.away"]),
                    op: Op::AtLeast,
                    value: 1.0,
                },
                self.sending_off_share,
            ),
            def(
                "yellow_cards_per_team",
                equal,
                mean(Per::Team, &["yellow.{side}"]),
                self.yellow_cards_per_team,
            ),
            def(
                "shots_on_target_share",
                equal,
                Measure::Ratio {
                    num: fields(&["shots_on_target.home", "shots_on_target.away"]),
                    den: fields(&["shots.home", "shots.away"]),
                },
                self.shots_on_target_share,
            ),
            def(
                "goals_per_xg",
                equal,
                Measure::Ratio {
                    num: fields(&GOALS),
                    den: fields(&["xg.home", "xg.away"]),
                },
                self.goals_per_xg,
            ),
            def(
                "passes_per_team",
                equal,
                mean(Per::Team, &["passes.{side}"]),
                self.passes_per_team,
            ),
            def(
                "pass_accuracy_pct",
                equal,
                mean(Per::Team, &["pass_accuracy_pct.{side}"]),
                self.pass_accuracy_pct,
            ),
            def(
                "corners_per_team",
                equal,
                mean(Per::Team, &["corners.{side}"]),
                self.corners_per_team,
            ),
            def(
                "throw_ins_per_match",
                equal,
                mean(Per::Match, &["throw_ins.home", "throw_ins.away"]),
                self.throw_ins_per_match,
            ),
            def(
                "goal_kicks_per_match",
                equal,
                mean(Per::Match, &["goal_kicks.home", "goal_kicks.away"]),
                self.goal_kicks_per_match,
            ),
            def(
                "goalless_share",
                goals,
                share(Op::Equal, 0.0),
                self.goalless_share,
            ),
            def(
                "stronger_team_win_rate",
                &["strength"],
                Measure::Builtin {
                    name: Builtin::StrongerTeamWinRate,
                },
                Band {
                    lo: self.stronger_team.min_win_rate,
                    hi: 1.0,
                },
            ),
            def(
                "reduced_minus_full",
                &["red-card"],
                Measure::Builtin {
                    name: Builtin::ReducedMinusFull,
                },
                Band {
                    lo: RED_CARD_FLOOR,
                    hi: 0.0,
                },
            ),
            def(
                "full_over_control",
                &["red-card"],
                Measure::Builtin {
                    name: Builtin::FullOverControl,
                },
                Band {
                    lo: 0.0,
                    hi: RED_CARD_LIMIT,
                },
            ),
        ];
        Registry {
            schema_version: REGISTRY_VERSION,
            sample_size: self.sample_size,
            wall_minutes_per_sample: self.wall_minutes_per_sample,
            stronger_team: StrongerTeam {
                attribute_boost: self.stronger_team.attribute_boost,
            },
            bands,
            migrated_from: Some(self.schema_version),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn shipped() -> ContentDir {
        ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
    }

    /// The version 2 file as it shipped before the registry, kept as the migration's
    /// fixture.
    const V2: &str = r#"{
  "schema_version": 2,
  "sample_size": 1000,
  "goals_per_match": { "lo": 2.4, "hi": 3.2 },
  "shots_per_team": { "lo": 8.0, "hi": 16.0 },
  "possession_pct": { "lo": 35.0, "hi": 65.0 },
  "stronger_team": { "attribute_boost": 1.15, "min_win_rate": 0.5 },
  "wall_minutes_per_sample": 30.0,
  "ten_plus_goals_share": { "lo": 0.0, "hi": 0.005 },
  "sending_off_share": { "lo": 0.08, "hi": 0.22 },
  "yellow_cards_per_team": { "lo": 1.2, "hi": 2.6 },
  "shots_on_target_share": { "lo": 0.3, "hi": 0.42 },
  "goals_per_xg": { "lo": 0.85, "hi": 1.15 },
  "passes_per_team": { "lo": 350.0, "hi": 550.0 },
  "pass_accuracy_pct": { "lo": 75.0, "hi": 88.0 },
  "corners_per_team": { "lo": 3.5, "hi": 6.5 },
  "throw_ins_per_match": { "lo": 35.0, "hi": 55.0 },
  "goal_kicks_per_match": { "lo": 12.0, "hi": 22.0 },
  "goalless_share": { "lo": 0.04, "hi": 0.12 }
}"#;

    /// The 14 bands of version 2 (possession as one), their ranges, and the confirmed
    /// smallest shifts: a quarter of each range.
    const CONFIRMED: [(&str, f64, f64, f64); 15] = [
        ("goals_per_match", 2.4, 3.2, 0.2),
        ("shots_per_team", 8.0, 16.0, 2.0),
        ("possession_home_pct", 35.0, 65.0, 7.5),
        ("possession_away_pct", 35.0, 65.0, 7.5),
        ("ten_plus_goals_share", 0.0, 0.005, 0.00125),
        ("sending_off_share", 0.08, 0.22, 0.035),
        ("yellow_cards_per_team", 1.2, 2.6, 0.35),
        ("shots_on_target_share", 0.3, 0.42, 0.03),
        ("goals_per_xg", 0.85, 1.15, 0.075),
        ("passes_per_team", 350.0, 550.0, 50.0),
        ("pass_accuracy_pct", 75.0, 88.0, 3.25),
        ("corners_per_team", 3.5, 6.5, 0.75),
        ("throw_ins_per_match", 35.0, 55.0, 5.0),
        ("goal_kicks_per_match", 12.0, 22.0, 2.5),
        ("goalless_share", 0.04, 0.12, 0.02),
    ];

    fn assert_confirmed(r: &Registry) {
        for (name, lo, hi, shift) in CONFIRMED {
            let b = r.band(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!((b.lo, b.hi), (lo, hi), "{name}");
            assert!((b.smallest_shift - shift).abs() < 1e-9, "{name}");
        }
        let stronger = r.band("stronger_team_win_rate").unwrap();
        assert_eq!((stronger.lo, stronger.hi), (0.5, 1.0));
        assert!((stronger.smallest_shift - 0.125).abs() < 1e-12);
        let reduced = r.band("reduced_minus_full").unwrap();
        assert_eq!(
            (reduced.lo, reduced.hi, reduced.smallest_shift),
            (-10.0, 0.0, 2.5)
        );
        let full = r.band("full_over_control").unwrap();
        assert_eq!((full.lo, full.hi), (0.0, 1.6));
        assert!((full.smallest_shift - 0.4).abs() < 1e-12);
        assert_eq!(r.sample_size, 1000);
        assert_eq!(r.stronger_team.attribute_boost, 1.15);
        assert_eq!(r.wall_budget_ms(1000), 1_800_000);
    }

    #[test]
    fn the_shipped_registry_holds_the_accepted_bands_and_the_confirmed_shifts() {
        let r = Registry::load(&shipped()).unwrap();
        assert_eq!(r.schema_version, REGISTRY_VERSION);
        assert_eq!(r.migrated_from, None);
        assert_confirmed(&r);
        assert_eq!(r.bands.len(), 18);
        assert_eq!(
            r.band("possession_home_pct").unwrap().group.as_deref(),
            Some("possession_pct")
        );
        assert!(r.range("ten_plus_goals_share").unwrap().is_floor());
        assert!(!r.range("goalless_share").unwrap().is_floor());
    }

    #[test]
    fn a_version_two_file_migrates_to_the_same_bands_as_the_shipped_registry() {
        let migrated = Registry::from_bytes(V2.as_bytes()).unwrap();
        assert_eq!(migrated.migrated_from, Some(2));
        assert_confirmed(&migrated);
        let shipped = Registry::load(&shipped()).unwrap();
        assert_eq!(
            migrated.bands, shipped.bands,
            "the shipped file is the migration"
        );
        assert_eq!(migrated.digest(), shipped.digest());
    }

    fn refusal(text: &str) -> String {
        Registry::from_bytes(text.as_bytes())
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn another_version_is_refused_naming_the_file_and_the_version() {
        let err = refusal(&V2.replace("\"schema_version\": 2", "\"schema_version\": 1"));
        assert!(err.contains(BANDS_FILE), "{err}");
        assert!(err.contains("schema_version 1"), "{err}");
        assert!(err.contains("reads 3"), "{err}");
        let err = refusal("{\"schema_version\": 9}");
        assert!(err.contains(BANDS_FILE) && err.contains('9'), "{err}");
    }

    #[test]
    fn a_version_two_file_without_a_band_or_with_an_unknown_field_is_refused() {
        let text: String = V2
            .lines()
            .filter(|line| !line.contains("goals_per_xg"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(refusal(&text).contains("goals_per_xg"));
        let err = refusal(&V2.replace("\"sample_size\"", "\"sample_sizes\""));
        assert!(err.contains("sample_sizes"), "{err}");
        let err = refusal(&V2.replace("\"lo\": 2.4, \"hi\": 3.2", "\"lo\": 3.2, \"hi\": 2.4"));
        assert!(err.contains("goals_per_match"), "{err}");
    }

    fn shipped_text() -> String {
        std::fs::read_to_string(shipped().path(BANDS_FILE)).unwrap()
    }

    /// The shipped file with the band `name` changed by `edit`.
    fn edited(name: &str, edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut doc: serde_json::Value = serde_json::from_str(&shipped_text()).unwrap();
        let band = doc["bands"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|b| b["band"] == name)
            .unwrap();
        edit(band);
        doc.to_string()
    }

    type Edit = Box<dyn FnOnce(&mut serde_json::Value)>;

    #[test]
    fn each_refusal_names_its_band_and_field() {
        let cases: [(&str, Edit, &str); 7] = [
            (
                "inverted range",
                Box::new(|b| b["lo"] = 99.0.into()),
                "goals_per_match).hi",
            ),
            (
                "zero shift",
                Box::new(|b| b["smallest_shift"] = 0.0.into()),
                "goals_per_match).smallest_shift",
            ),
            (
                "unknown catalog field",
                Box::new(|b| b["measure"]["of"][0] = "goals.middle".into()),
                "goals.middle",
            ),
            (
                "empty field list",
                Box::new(|b| b["measure"]["of"] = serde_json::json!([])),
                "goals_per_match).measure",
            ),
            (
                "unknown suite",
                Box::new(|b| b["suites"] = serde_json::json!(["cup"])),
                "cup",
            ),
            (
                "unknown key",
                Box::new(|b| b["colour"] = "red".into()),
                "colour",
            ),
            (
                "a side field in a per-match mean",
                Box::new(|b| b["measure"]["of"] = serde_json::json!(["goals.{side}"])),
                "{side}",
            ),
        ];
        for (what, edit, named) in cases {
            let err = refusal(&edited("goals_per_match", edit));
            assert!(err.contains(BANDS_FILE), "{what}: {err}");
            assert!(err.contains(named), "{what}: {err}");
        }
        let mut doc: serde_json::Value = serde_json::from_str(&shipped_text()).unwrap();
        let first = doc["bands"][0].clone();
        doc["bands"].as_array_mut().unwrap().push(first);
        let err = refusal(&doc.to_string());
        assert!(err.contains("already"), "{err}");
    }

    #[test]
    fn the_digest_follows_a_range_and_a_shift() {
        let base = Registry::from_bytes(shipped_text().as_bytes()).unwrap();
        let narrowed =
            Registry::from_bytes(edited("corners_per_team", |b| b["hi"] = 6.0.into()).as_bytes())
                .unwrap();
        assert_ne!(base.digest(), narrowed.digest());
        let a = base.band("corners_per_team").unwrap().digest();
        let b = narrowed.band("corners_per_team").unwrap().digest();
        assert_ne!(a, b);
        assert_eq!(
            base.band("goals_per_match").unwrap().digest(),
            narrowed.band("goals_per_match").unwrap().digest()
        );
    }
}
