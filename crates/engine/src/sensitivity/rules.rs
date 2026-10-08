//! The sensitivity rules file (`content/sensitivity.json`): one rule per attribute job and per
//! body job, the thresholds every rule is judged by, the size of the run, and the five-match
//! rule. The job table in `attributes.json` and the body jobs in `tuning.json` stay the source
//! of each statistic's label and direction: the rules file repeats the label so a reader sees
//! it, and the load refuses a label that differs, a job with no rule, and a rule with no job.

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::contract::body::BodyJobs;
use crate::data::attributes::{AttributeSchema, Direction};
use crate::data::team::Position;
use crate::error::EngineError;

/// The version this build reads.
pub const SENSITIVITY_VERSION: u32 = 1;

/// The rules file's path inside the content folder.
pub const SENSITIVITY_FILE: &str = "sensitivity.json";

/// The body jobs, with the range a level of each may take.
pub const BODY_JOBS: [(&str, std::ops::RangeInclusive<f64>); 3] = [
    ("height", 150.0..=215.0),
    ("age", 15.0..=45.0),
    ("nationality", 0.0..=100.0),
];

/// The statistic a rule measures. Each key names one counter set of the job probe or one
/// value the run reads off the match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Measure {
    PassCompletion,
    TakeOnCompletion,
    ReceiptLoss,
    BoxFinishing,
    CrossCompletion,
    HeadersWon,
    LongShotsOnTarget,
    TacklesWon,
    SkillAttempts,
    Blocks,
    PassProgress,
    XgPerShot,
    PressedCompletion,
    LooseBallShare,
    TackleAttempts,
    Fouls,
    Lapses,
    AnchorDistance,
    TopSpeed,
    CloseLooseBalls,
    FullTimeEnergy,
    StandingTacklesLost,
    FouledKept,
    AerialBallsReached,
    HalfTimeGain,
    Injuries,
    SavesHeld,
    FarSaves,
    HighClaims,
    NearSaves,
    KeeperKicks,
    KeeperThrows,
    CrossesClaimed,
    OffsidesWon,
    Sweeps,
    RatingSpread,
    LateEnergyLoss,
    MentalRating,
}

impl Measure {
    /// Every measure, in declaration order.
    pub const ALL: [Measure; 38] = [
        Measure::PassCompletion,
        Measure::TakeOnCompletion,
        Measure::ReceiptLoss,
        Measure::BoxFinishing,
        Measure::CrossCompletion,
        Measure::HeadersWon,
        Measure::LongShotsOnTarget,
        Measure::TacklesWon,
        Measure::SkillAttempts,
        Measure::Blocks,
        Measure::PassProgress,
        Measure::XgPerShot,
        Measure::PressedCompletion,
        Measure::LooseBallShare,
        Measure::TackleAttempts,
        Measure::Fouls,
        Measure::Lapses,
        Measure::AnchorDistance,
        Measure::TopSpeed,
        Measure::CloseLooseBalls,
        Measure::FullTimeEnergy,
        Measure::StandingTacklesLost,
        Measure::FouledKept,
        Measure::AerialBallsReached,
        Measure::HalfTimeGain,
        Measure::Injuries,
        Measure::SavesHeld,
        Measure::FarSaves,
        Measure::HighClaims,
        Measure::NearSaves,
        Measure::KeeperKicks,
        Measure::KeeperThrows,
        Measure::CrossesClaimed,
        Measure::OffsidesWon,
        Measure::Sweeps,
        Measure::RatingSpread,
        Measure::LateEnergyLoss,
        Measure::MentalRating,
    ];

    /// The measure's index in [`Measure::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Whether a rule's job belongs to an attribute or to a body field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Attribute,
    Body,
}

/// The size of a sensitivity run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct DesignSize {
    /// Matches of the balanced design, an even number.
    #[garde(range(min = 2, max = 100_000), custom(even))]
    pub matches: u32,
    /// Matches of each arm: every job low, and every job high.
    #[garde(range(min = 2, max = 100_000))]
    pub arm_matches: u32,
    #[garde(skip)]
    pub seed: u64,
    /// Bootstrap resamples per interval.
    #[garde(range(min = 99, max = 99_999))]
    pub resamples: u32,
}

fn even(n: &u32, _ctx: &()) -> garde::Result {
    if n.is_multiple_of(2) {
        Ok(())
    } else {
        Err(garde::Error::new(format!(
            "{n} is odd; each job is high in exactly half"
        )))
    }
}

/// The thresholds of every rule unless the rule overrides one.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Thresholds {
    /// The least relative move of the statistic from the low to the high level, in the job's
    /// direction.
    #[garde(range(min = 0.0, max = 10.0))]
    pub min_move: f64,
    /// The largest share of the outcome move of all jobs together that one job may carry.
    #[garde(range(min = 0.0, max = 1.0))]
    pub ceiling: f64,
}

/// One rule: the job, the statistic it moves, and the two levels it is measured at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RuleDef {
    #[garde(length(min = 2, max = 32))]
    pub job: String,
    #[garde(skip)]
    pub kind: JobKind,
    #[garde(skip)]
    pub measure: Measure,
    /// The job's statistic as its job table words it.
    #[garde(length(min = 3, max = 120))]
    pub statistic: String,
    /// The low and the high level: ratings for an attribute; centimetres, years or percent of
    /// adaptation for a body job.
    #[garde(skip)]
    pub levels: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(inner(range(min = 0.0, max = 10.0)))]
    pub min_move: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(inner(range(min = 0.0, max = 1.0)))]
    pub ceiling: Option<f64>,
    /// Why a threshold differs from the defaults; required with an override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub reason: Option<String>,
}

impl RuleDef {
    /// The rule's thresholds: its overrides over `defaults`.
    pub fn thresholds(&self, defaults: Thresholds) -> Thresholds {
        Thresholds {
            min_move: self.min_move.unwrap_or(defaults.min_move),
            ceiling: self.ceiling.unwrap_or(defaults.ceiling),
        }
    }
}

/// The five-match rule: a top and an average copy of one player per role, each over runs of
/// five matches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FiveMatch {
    #[garde(length(min = 1, max = 10))]
    pub roles: Vec<Position>,
    /// Paired runs per role.
    #[garde(range(min = 1, max = 10_000))]
    pub runs: u32,
    #[garde(range(min = 1, max = 50))]
    pub matches_per_run: u32,
    /// Every visible attribute of the top copy and of the average copy.
    #[garde(range(min = 1.0, max = 20.0))]
    pub top: f64,
    #[garde(range(min = 1.0, max = 20.0))]
    pub average: f64,
    /// The share of runs the top copy should win, and how far the interval may stray.
    #[garde(range(min = 0.0, max = 1.0))]
    pub target: f64,
    #[garde(range(min = 0.0, max = 0.5))]
    pub tolerance: f64,
}

/// The rules file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct SensitivityFile {
    #[garde(skip)]
    pub schema_version: u32,
    #[garde(dive)]
    pub design: DesignSize,
    #[garde(dive)]
    pub defaults: Thresholds,
    #[garde(length(min = 1, max = 100), dive)]
    pub rules: Vec<RuleDef>,
    /// Fields with no engine job, proven by their own derivation test.
    #[garde(skip)]
    pub derived: Vec<String>,
    #[garde(dive)]
    pub five_match: FiveMatch,
}

/// One job as its owner names it: the label and the direction of its statistic.
struct JobText<'a> {
    name: &'a str,
    kind: JobKind,
    statistic: &'a str,
    direction: Direction,
}

/// Every job the rules must cover: each attribute of `schema`, then the three body jobs.
fn jobs<'a>(schema: &'a AttributeSchema, body: &'a BodyJobs) -> Vec<JobText<'a>> {
    let mut out: Vec<JobText<'a>> = schema
        .attributes
        .iter()
        .map(|a| JobText {
            name: &a.name,
            kind: JobKind::Attribute,
            statistic: &a.job.statistic,
            direction: a.job.direction,
        })
        .collect();
    for (name, row) in [
        ("height", &body.jobs.height),
        ("age", &body.jobs.age),
        ("nationality", &body.jobs.nationality),
    ] {
        out.push(JobText {
            name,
            kind: JobKind::Body,
            statistic: &row.statistic,
            direction: row.direction,
        });
    }
    out
}

impl SensitivityFile {
    /// Loads and checks the rules file's bytes against the attribute schema and the body jobs.
    /// A refusal names the field: a job with no rule, a rule with no job or twice, a statistic
    /// that differs from the job's own, an override without a reason, a level outside its
    /// range, a low level not below the high one, or a derived field that is a job.
    pub fn load(
        bytes: &[u8],
        shown: &str,
        schema: &AttributeSchema,
        body: &BodyJobs,
    ) -> Result<Self, EngineError> {
        let file = crate::data::load_json_bytes::<Self>(
            "sensitivity",
            bytes,
            shown,
            SENSITIVITY_VERSION,
            &(),
        )?
        .value;
        file.check(schema, body).map_err(|(field, reason)| {
            tracing::error!(signal = "content.refused", kind = "sensitivity", path = shown, field = %field, reason = %reason);
            EngineError::Data {
                kind: "sensitivity",
                path: shown.to_string(),
                field,
                reason,
            }
        })?;
        Ok(file)
    }

    /// The checks against the job tables, as `(field, reason)`.
    pub fn check(&self, schema: &AttributeSchema, body: &BodyJobs) -> Result<(), (String, String)> {
        let all = jobs(schema, body);
        for (i, rule) in self.rules.iter().enumerate() {
            let field = |f: &str| format!("rules[{i}].{f}");
            let Some(job) = all.iter().find(|j| j.name == rule.job) else {
                return Err((
                    field("job"),
                    format!(
                        "{} is not a job; no attribute or body field has it",
                        rule.job
                    ),
                ));
            };
            if self.rules[..i].iter().any(|r| r.job == rule.job) {
                return Err((field("job"), format!("{} has a second rule", rule.job)));
            }
            if rule.kind != job.kind {
                return Err((
                    field("kind"),
                    format!("{} is a {:?} job", rule.job, job.kind).to_lowercase(),
                ));
            }
            if rule.statistic != job.statistic {
                return Err((
                    field("statistic"),
                    format!(
                        "{}: \"{}\" differs from the job's \"{}\"",
                        rule.job, rule.statistic, job.statistic
                    ),
                ));
            }
            if (rule.min_move.is_some() || rule.ceiling.is_some())
                && rule.reason.as_deref().is_none_or(|r| r.trim().is_empty())
            {
                return Err((
                    field("reason"),
                    format!("{}: an override needs a reason", rule.job),
                ));
            }
            let [lo, hi] = rule.levels;
            let range = match job.kind {
                JobKind::Attribute => 1.0..=20.0,
                JobKind::Body => BODY_JOBS
                    .iter()
                    .find(|(n, _)| *n == rule.job)
                    .map(|(_, r)| r.clone())
                    .expect("every body job has a range"),
            };
            if !range.contains(&lo) || !range.contains(&hi) {
                return Err((
                    field("levels"),
                    format!(
                        "{}: {lo} and {hi} must lie in {} to {}",
                        rule.job,
                        range.start(),
                        range.end()
                    ),
                ));
            }
            if job.kind == JobKind::Attribute
                && [lo, hi]
                    .iter()
                    .any(|v| ((v * 10.0).round() - v * 10.0).abs() > 1e-9)
            {
                return Err((
                    field("levels"),
                    format!("{}: a rating level must be on a tenth", rule.job),
                ));
            }
            if lo >= hi {
                return Err((
                    field("levels"),
                    format!(
                        "{}: the low level {lo} is not below the high {hi}",
                        rule.job
                    ),
                ));
            }
        }
        if let Some(job) = all
            .iter()
            .find(|j| !self.rules.iter().any(|r| r.job == j.name))
        {
            return Err(("rules".into(), format!("job {} has no rule", job.name)));
        }
        for (i, name) in self.derived.iter().enumerate() {
            if all.iter().any(|j| j.name == name) {
                return Err((
                    format!("derived[{i}]"),
                    format!("{name} has a job; give it a rule instead"),
                ));
            }
        }
        if self.five_match.average >= self.five_match.top {
            return Err((
                "five_match.average".into(),
                format!(
                    "{} is not below the top {}",
                    self.five_match.average, self.five_match.top
                ),
            ));
        }
        Ok(())
    }

    /// The direction of `rule`'s statistic as its job states it.
    pub fn direction(
        &self,
        rule: &RuleDef,
        schema: &AttributeSchema,
        body: &BodyJobs,
    ) -> Direction {
        jobs(schema, body)
            .into_iter()
            .find(|j| j.name == rule.job)
            .map_or(Direction::Up, |j| j.direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;

    fn shipped() -> (Vec<u8>, crate::data::Content) {
        let content = shipped_content();
        let path = crate::data::test_support::shipped_dir().path(SENSITIVITY_FILE);
        (std::fs::read(path).unwrap(), content)
    }

    /// The shipped file with `edit` applied to its JSON, loaded.
    fn edited(edit: impl FnOnce(&mut serde_json::Value)) -> Result<SensitivityFile, EngineError> {
        let (bytes, content) = shipped();
        let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        edit(&mut v);
        let bytes = serde_json::to_vec(&v).unwrap();
        SensitivityFile::load(
            &bytes,
            SENSITIVITY_FILE,
            &content.attributes,
            &content.tuning.engine.contract.body,
        )
    }

    fn refusal(r: Result<SensitivityFile, EngineError>) -> (String, String) {
        match r {
            Err(EngineError::Data { field, reason, .. }) => (field, reason),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    fn rule_of<'a>(v: &'a mut serde_json::Value, job: &str) -> &'a mut serde_json::Value {
        v["rules"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["job"] == job)
            .unwrap()
    }

    #[test]
    fn the_shipped_file_loads_with_one_rule_per_job() {
        let (bytes, content) = shipped();
        let file = SensitivityFile::load(
            &bytes,
            SENSITIVITY_FILE,
            &content.attributes,
            &content.tuning.engine.contract.body,
        )
        .unwrap();
        assert_eq!(file.rules.len(), content.attributes.len() + 3);
        assert_eq!(file.derived, vec!["build".to_string()]);
    }

    #[test]
    fn a_job_with_no_rule_is_refused_by_name() {
        let (field, reason) = refusal(edited(|v| {
            v["rules"]
                .as_array_mut()
                .unwrap()
                .retain(|r| r["job"] != "vision");
        }));
        assert_eq!(field, "rules");
        assert!(reason.contains("vision"), "{reason}");
    }

    #[test]
    fn a_rule_with_no_job_is_refused() {
        let (field, reason) = refusal(edited(|v| rule_of(v, "vision")["job"] = "flair".into()));
        assert!(field.ends_with(".job"), "{field}");
        assert!(reason.contains("flair"), "{reason}");
    }

    #[test]
    fn a_statistic_that_differs_from_the_job_is_refused() {
        let (field, reason) = refusal(edited(|v| {
            rule_of(v, "passing")["statistic"] = "passes per match".into()
        }));
        assert!(field.ends_with(".statistic"), "{field}");
        assert!(reason.contains("pass completion share"), "{reason}");
    }

    #[test]
    fn an_unknown_measure_is_refused() {
        let (_, reason) = refusal(edited(|v| {
            rule_of(v, "passing")["measure"] = "elegance".into()
        }));
        assert!(reason.contains("elegance"), "{reason}");
    }

    #[test]
    fn an_override_without_a_reason_is_refused() {
        let (field, _) = refusal(edited(|v| rule_of(v, "pace")["ceiling"] = 0.5.into()));
        assert!(field.ends_with(".reason"), "{field}");
        let file = edited(|v| {
            let r = rule_of(v, "pace");
            r["ceiling"] = 0.5.into();
            r["reason"] = "a test of the override".into();
        })
        .unwrap();
        let pace = file.rules.iter().find(|r| r.job == "pace").unwrap();
        assert_eq!(pace.thresholds(file.defaults).ceiling, 0.5);
    }

    #[test]
    fn levels_outside_their_range_are_refused() {
        let (field, _) = refusal(edited(|v| {
            rule_of(v, "passing")["levels"] = serde_json::json!([8.0, 21.0])
        }));
        assert!(field.ends_with(".levels"), "{field}");
        let (field, _) = refusal(edited(|v| {
            rule_of(v, "height")["levels"] = serde_json::json!([140.0, 190.0])
        }));
        assert!(field.ends_with(".levels"), "{field}");
        let (field, _) = refusal(edited(|v| {
            rule_of(v, "passing")["levels"] = serde_json::json!([8.05, 16.0])
        }));
        assert!(field.ends_with(".levels"), "{field}");
        let (field, _) = refusal(edited(|v| {
            rule_of(v, "age")["levels"] = serde_json::json!([34.0, 22.0])
        }));
        assert!(field.ends_with(".levels"), "{field}");
    }

    #[test]
    fn an_unknown_field_is_refused() {
        refusal(edited(|v| rule_of(v, "passing")["direction"] = "up".into()));
    }

    #[test]
    fn every_measure_has_a_rule_or_is_shared() {
        let (bytes, content) = shipped();
        let file = SensitivityFile::load(
            &bytes,
            SENSITIVITY_FILE,
            &content.attributes,
            &content.tuning.engine.contract.body,
        )
        .unwrap();
        for m in Measure::ALL {
            assert!(
                file.rules.iter().any(|r| r.measure == m),
                "{m:?} has no rule"
            );
        }
    }
}
