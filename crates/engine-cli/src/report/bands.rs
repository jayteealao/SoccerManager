//! The accepted realism bands, read from `realism-bands.json` in the content folder. The
//! bands are acceptance criteria, never tuning values: the calibration harness reads them
//! and nothing writes them.

use engine::data::{ContentDir, load_json};
use garde::Validate;
use serde::{Deserialize, Serialize};

/// The file name inside the content folder.
pub const BANDS_FILE: &str = "realism-bands.json";
/// The layout version this build reads.
pub const BANDS_VERSION: u32 = 1;

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

/// The stronger-team criterion: a club with every attribute times `attribute_boost` wins
/// more than `min_win_rate` of its matches.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StrongerTeam {
    #[garde(range(min = 1.0, max = 2.0))]
    pub attribute_boost: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub min_win_rate: f64,
}

/// The realism bands file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Bands {
    #[garde(skip)]
    pub schema_version: u32,
    /// Matches per suite the bands are judged over.
    #[garde(range(min = 1, max = 100_000))]
    pub sample_size: u32,
    #[garde(dive)]
    pub goals_per_match: Band,
    #[garde(dive)]
    pub shots_per_team: Band,
    #[garde(dive)]
    pub possession_pct: Band,
    #[garde(dive)]
    pub stronger_team: StrongerTeam,
    /// The wall-time budget of one suite of `sample_size` matches, in minutes.
    #[garde(range(min = 0.1, max = 10_000.0))]
    pub wall_minutes_per_sample: f64,
}

impl Bands {
    /// Loads the bands from the content folder. The layout version is checked first, and a
    /// refusal names the file and the field.
    pub fn load(dir: &ContentDir) -> anyhow::Result<Self> {
        let path = dir.path(BANDS_FILE);
        Ok(load_json::<Bands>("realism bands", &path, BANDS_FILE, BANDS_VERSION, &())?.value)
    }

    /// The wall-time budget of a suite of `matches` matches, in milliseconds.
    pub fn wall_budget_ms(&self, matches: u32) -> u64 {
        let per_match = self.wall_minutes_per_sample * 60_000.0 / f64::from(self.sample_size);
        // At most 100 000 matches of at most 10 000 minutes: far inside u64.
        (per_match * f64::from(matches)).round() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn shipped() -> ContentDir {
        ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
    }

    #[test]
    fn the_shipped_bands_are_the_accepted_criteria() {
        let b = Bands::load(&shipped()).unwrap();
        assert_eq!(b.sample_size, 1000);
        assert_eq!(b.goals_per_match, Band { lo: 2.4, hi: 3.2 });
        assert_eq!(b.shots_per_team, Band { lo: 8.0, hi: 16.0 });
        assert_eq!(b.possession_pct, Band { lo: 35.0, hi: 65.0 });
        assert_eq!(b.stronger_team.attribute_boost, 1.15);
        assert_eq!(b.stronger_team.min_win_rate, 0.5);
        assert_eq!(b.wall_budget_ms(1000), 1_800_000);
    }

    #[test]
    fn another_version_is_refused_naming_the_file() {
        let dir = std::env::temp_dir().join(format!("bands-v9-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(BANDS_FILE), "{\"schema_version\": 9}").unwrap();
        let err = Bands::load(&ContentDir::at(&dir)).unwrap_err().to_string();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(err.contains(BANDS_FILE), "{err}");
        assert!(err.contains('9'), "{err}");
    }

    #[test]
    fn an_inverted_band_is_refused_naming_the_field() {
        let dir = std::env::temp_dir().join(format!("bands-inv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let text = std::fs::read_to_string(shipped().path(BANDS_FILE))
            .unwrap()
            .replace("\"lo\": 2.4, \"hi\": 3.2", "\"lo\": 3.2, \"hi\": 2.4");
        std::fs::write(dir.join(BANDS_FILE), text).unwrap();
        let err = Bands::load(&ContentDir::at(&dir)).unwrap_err().to_string();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(err.contains("goals_per_match"), "{err}");
    }
}
