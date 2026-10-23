//! Run kinds: change, milestone and release.
//!
//! The band file holds one table per kind: the seeds the kind may use, the factor that widens
//! a rare band's margin in that kind, the leagues per run, and the fixed count of paired
//! matches per suite unit (per formation pairing, per red-card arm) that a change run of the
//! kind plays. The count changes only by an edit of the band file, with its line in the
//! CHANGELOG. `computed` records how each count was set: the real-test count, the old-engine
//! count and the band that drove the larger. It never holds a real value or a real error,
//! because the content folder ships with the game.
//!
//! The seed sets do not overlap, so a seed names its kind; the release seeds are never seen
//! by tuning, and a tuning command refuses them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::Suite;
use super::bands::{BandDef, EngineUnit, ExposureUnit, Measure, Registry, Target};

/// The power the old-engine count sizes for.
pub const OLD_POWER: f64 = 0.8;

/// A run's kind.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, clap::ValueEnum,
)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A change of the engine or its content: one fixed count, rare bands on a wider margin.
    Change,
    /// A milestone: the normal margins on the milestone seeds.
    Milestone,
    /// A release: the normal margins on the release seeds, which tuning never sees.
    Release,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Change, Kind::Milestone, Kind::Release];

    pub fn code(self) -> &'static str {
        match self {
            Kind::Change => "change",
            Kind::Milestone => "milestone",
            Kind::Release => "release",
        }
    }
}

/// The seeds a kind may use, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedSet {
    pub from: u64,
    pub to: u64,
}

impl SeedSet {
    pub fn contains(&self, seed: u64) -> bool {
        (self.from..=self.to).contains(&seed)
    }
}

/// The factor that multiplies each rare band's own margin in a kind, with why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RareMargin {
    pub factor: f64,
    pub reason: String,
}

/// One test's count as recorded: the matches, the band that needs them and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Need {
    pub matches: u32,
    pub band: String,
    pub why: String,
}

/// How one suite's count was set: per suite unit, with the units a suite pools (the
/// formation pairings, the red-card arms), the real-test count and the old-engine count in
/// pooled matches, and the test that set the larger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Computed {
    pub units: u32,
    pub real: Need,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old: Option<Need>,
    pub count: u32,
    /// `real` or `old`.
    pub driver: String,
}

/// One kind's table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindDef {
    pub seeds: SeedSet,
    pub rare_margin: RareMargin,
    /// Matches per suite unit a change run of this kind plays, by suite code.
    pub count: BTreeMap<String, u32>,
    /// Generated leagues per run the floor is set on.
    pub leagues: u32,
    /// How each suite's count was set, by suite code; absent while the counts are the
    /// earlier design count.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub computed: BTreeMap<String, Computed>,
    /// `refused` on the release kind: no tuning command may use its seeds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tuning: Option<String>,
}

impl KindDef {
    /// Matches per suite unit of `suite`.
    pub fn count_for(&self, suite: Suite) -> u32 {
        self.count.get(suite.code()).copied().unwrap_or(0)
    }

    /// The units `suite` pools, as the count was computed with; 1 when not recorded.
    pub fn units_for(&self, suite: &str) -> u32 {
        self.computed.get(suite).map_or(1, |c| c.units.max(1))
    }

    /// The matches a band's floor is checked at: the fewest pooled matches over the suites
    /// that judge it (its count times the units each suite pools). A band no suite plays is
    /// judged by its slow test at the equal suite's count.
    pub fn floor_matches(&self, b: &BandDef) -> u32 {
        b.suites
            .iter()
            .map(|s| {
                self.count
                    .get(s)
                    .copied()
                    .unwrap_or(0)
                    .saturating_mul(self.units_for(s))
            })
            .min()
            .unwrap_or_else(|| self.count.get("equal").copied().unwrap_or(0))
    }

    /// A band's margin in this kind: a rare band's own margin times the kind's factor, every
    /// other band's own margin. A rare band cannot pass before its minimum exposure, and at
    /// or beyond it its own margin already holds the floor, so the factor never takes a
    /// margin under its floor.
    pub fn margin_for(&self, b: &BandDef) -> Option<f64> {
        let rule = b.rule.as_ref()?;
        Some(if rule.rare {
            rule.margin.value * self.rare_margin.factor
        } else {
            rule.margin.value
        })
    }
}

/// The three kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindTable {
    pub change: KindDef,
    pub milestone: KindDef,
    pub release: KindDef,
}

impl KindTable {
    pub fn get(&self, kind: Kind) -> &KindDef {
        match kind {
            Kind::Change => &self.change,
            Kind::Milestone => &self.milestone,
            Kind::Release => &self.release,
        }
    }

    /// The kind whose seed set holds `seed`.
    pub fn kind_of(&self, seed: u64) -> Option<Kind> {
        Kind::ALL
            .into_iter()
            .find(|&k| self.get(k).seeds.contains(seed))
    }

    /// A run of `kind` may use `seed` only from the kind's own set.
    pub fn check_seed(&self, kind: Kind, seed: u64) -> Result<(), String> {
        let set = self.get(kind).seeds;
        if set.contains(seed) {
            return Ok(());
        }
        let owner = self.kind_of(seed).map_or_else(
            || "no kind".to_string(),
            |k| format!("the {} kind", k.code()),
        );
        Err(format!(
            "--seed {seed} is not a seed of the {} kind ({} to {}); it belongs to {owner}. \
             Pick a seed of the kind's set, or another --kind",
            kind.code(),
            set.from,
            set.to
        ))
    }

    /// A tuning command refuses every seed of the release set before any match plays. The
    /// tuning commands are built with the measures feature only.
    #[cfg_attr(not(feature = "measures"), allow(dead_code))]
    pub fn refuse_release_seed(&self, seed: u64) -> Result<(), String> {
        let set = self.release.seeds;
        if set.contains(seed) {
            Err(format!(
                "seed {seed} is a release seed ({} to {}); tuning never sees the release \
                 seeds. Pick a seed outside that set",
                set.from, set.to
            ))
        } else {
            Ok(())
        }
    }

    /// The table's rules, every refusal collected, each naming its field.
    pub fn check(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for k in Kind::ALL {
            let def = self.get(k);
            let at = |field: &str| format!("kinds.{}.{field}", k.code());
            if def.seeds.from > def.seeds.to {
                out.push((at("seeds"), "from is above to".into()));
            }
            if !(def.rare_margin.factor.is_finite() && def.rare_margin.factor >= 1.0) {
                out.push((
                    at("rare_margin.factor"),
                    format!(
                        "{} is under 1; a kind may widen a rare margin, never narrow it",
                        def.rare_margin.factor
                    ),
                ));
            }
            if def.rare_margin.reason.trim().is_empty() {
                out.push((at("rare_margin.reason"), "needs a reason".into()));
            }
            if def.leagues == 0 {
                out.push((at("leagues"), "must be above 0".into()));
            }
            for s in [
                Suite::Equal,
                Suite::Strength,
                Suite::Formations,
                Suite::RedCard,
            ] {
                match def.count.get(s.code()) {
                    None => out.push((at(&format!("count.{}", s.code())), "is required".into())),
                    Some(0) => {
                        out.push((at(&format!("count.{}", s.code())), "must be above 0".into()))
                    }
                    Some(_) => {}
                }
            }
            for name in def.count.keys() {
                if Suite::parse(name).is_none() {
                    out.push((
                        at(&format!("count.{name}")),
                        "is not a suite (equal, strength, formations, red-card)".into(),
                    ));
                }
            }
            for (name, c) in &def.computed {
                if def.count.get(name) != Some(&c.count) {
                    out.push((
                        at(&format!("computed.{name}.count")),
                        format!(
                            "{} is not the count {} the kind plays; edit both together",
                            c.count,
                            def.count.get(name).copied().unwrap_or(0)
                        ),
                    ));
                }
            }
        }
        if self.release.tuning.as_deref() != Some("refused") {
            out.push((
                "kinds.release.tuning".into(),
                "must be \"refused\": tuning never sees the release seeds".into(),
            ));
        }
        for (a, b) in [
            (Kind::Change, Kind::Milestone),
            (Kind::Change, Kind::Release),
            (Kind::Milestone, Kind::Release),
        ] {
            let (x, y) = (self.get(a).seeds, self.get(b).seeds);
            if x.from <= y.to && y.from <= x.to {
                out.push((
                    format!("kinds.{}.seeds", b.code()),
                    format!("overlaps the {} seeds; each seed names one kind", a.code()),
                ));
            }
        }
        out
    }
}

/// One band's agreement between paired matches.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BandAgreement {
    pub r: f64,
    pub lo: f64,
    pub hi: f64,
    pub pairs: u32,
}

/// One measure of the agreement between paired matches, as recorded in the band file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgreementRecord {
    /// `before the split`, `after the split` or `head`.
    pub measure: String,
    /// The revision the two runs played.
    pub revision: String,
    /// How it was built.
    pub build: String,
    /// The two runs, by run id.
    pub runs: Vec<String>,
    /// The behaviour change the second run made.
    pub change: String,
    /// The median of the bands' correlations, with its 90 percent interval.
    pub median: f64,
    pub lo: f64,
    pub hi: f64,
    /// The pairs both runs played.
    pub pairs: u32,
    /// Each band's correlation, by band name; the head measure's lower bounds size the
    /// old-engine count.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bands: BTreeMap<String, BandAgreement>,
}

/// The measure the counts are computed from.
pub const HEAD_MEASURE: &str = "head";

/// A band whose floor no count meets, in a kind and suite.
#[derive(Debug, Clone, PartialEq)]
pub struct Refused {
    pub kind: Kind,
    pub suite: String,
    pub band: String,
    pub which: &'static str,
    pub margin: f64,
    pub needed: f64,
}

/// The floor inputs of each gating band `suite` judges, at `def`'s margins, with `real_se`
/// each band's real error (the reference file's for a target, else the error read here; 0
/// when unknown). A distribution band's floor is its noise floor, which no count moves, and
/// the load checks it; it takes no part.
fn floor_inputs(
    registry: &Registry,
    def: &KindDef,
    suite: Suite,
    real_se: &dyn Fn(&BandDef) -> Option<f64>,
) -> Vec<pass_rule::count::FloorInput> {
    let mut out = Vec::new();
    for b in registry
        .bands
        .iter()
        .filter(|b| b.gates && b.judged_in(suite) && !b.is_distribution())
    {
        let (Some(rule), Some(margin)) = (&b.rule, def.margin_for(b)) else {
            continue;
        };
        if matches!(b.measure, Measure::Scenario { .. }) {
            continue;
        }
        let spread = &rule.engine_spread;
        let rate = spread.per_match_sd.powi(2);
        let min_exposure = match rule.min_exposure.unit {
            ExposureUnit::Matches => rule.min_exposure.value,
            ExposureUnit::Events => rule.min_exposure.value / rate.max(1e-12),
        };
        let sigma_r = real_se(b).unwrap_or(0.0);
        let one = |which, margin, real_error| pass_rule::count::FloorInput {
            band: b.band.clone(),
            which,
            margin,
            per_match_sd: spread.per_match_sd,
            between_league_sd: spread.between_league_sd,
            per_league: rule.resampling.engine == EngineUnit::League,
            real_error,
            min_exposure: if rule.rare { min_exposure } else { 0.0 },
            rare: rule.rare,
        };
        match rule.target {
            Target::Real => out.push(one("margin", margin, sigma_r)),
            Target::Design { cap, .. } => {
                out.push(one("margin", margin, 0.0));
                if let Some(cap) = cap {
                    out.push(one("cap", cap, sigma_r));
                }
            }
        }
    }
    out
}

/// The old-engine inputs of each gating scalar band `suite` judges with a head agreement.
fn old_inputs(
    registry: &Registry,
    suite: Suite,
    head: Option<&AgreementRecord>,
) -> Vec<pass_rule::count::OldInput> {
    let Some(head) = head else {
        return Vec::new();
    };
    registry
        .bands
        .iter()
        .filter(|b| b.gates && b.judged_in(suite) && !b.is_distribution())
        .filter_map(|b| {
            let rule = b.rule.as_ref()?;
            let a = head.bands.get(&b.band)?;
            Some(pass_rule::count::OldInput {
                band: b.band.clone(),
                per_match_sd: rule.engine_spread.per_match_sd,
                rho_lo: a.lo,
                noticeable: rule.margin.noticeable_error,
            })
        })
        .collect()
}

/// The units each suite pools: the formation pairings and the red-card arms.
pub fn units(pairings: usize) -> BTreeMap<Suite, u32> {
    BTreeMap::from([
        (Suite::Equal, 1),
        (Suite::Strength, 1),
        (
            Suite::Formations,
            u32::try_from(pairings.max(1)).unwrap_or(u32::MAX),
        ),
        (
            Suite::RedCard,
            u32::try_from(super::RED_CARD_ARMS.len()).unwrap_or(u32::MAX),
        ),
    ])
}

/// Every kind's count, computed per suite: the larger of the real-test count (every gating
/// band of the suite at its floor with the kind's margins and leagues, every rare band at its
/// minimum exposure) and the old-engine count at the head agreement's lower bounds, in
/// pooled matches, then per suite unit. Every band whose floor no count meets is refused.
pub fn compute(
    registry: &Registry,
    units: &BTreeMap<Suite, u32>,
    real_se: &dyn Fn(&BandDef) -> Option<f64>,
) -> Result<BTreeMap<Kind, BTreeMap<String, Computed>>, Vec<Refused>> {
    let Some(table) = &registry.kinds else {
        return Ok(BTreeMap::new());
    };
    let Some(settings) = registry.pass_rule else {
        return Ok(BTreeMap::new());
    };
    let head = registry
        .pair_agreement
        .iter()
        .find(|a| a.measure == HEAD_MEASURE);
    let factor =
        pass_rule::floor::floor_factor(settings.alpha, settings.sound_pass, registry.gating());
    let mut out = BTreeMap::new();
    let mut refused = Vec::new();
    for kind in Kind::ALL {
        let def = table.get(kind);
        let mut suites = BTreeMap::new();
        for (&suite, &n_units) in units {
            let inputs = floor_inputs(registry, def, suite, real_se);
            let real = match pass_rule::count::real_count(&inputs, factor, f64::from(def.leagues)) {
                Ok(n) => n,
                Err(list) => {
                    refused.extend(list.into_iter().map(|u| Refused {
                        kind,
                        suite: suite.code().to_string(),
                        band: u.band,
                        which: u.which,
                        margin: u.margin,
                        needed: u.needed,
                    }));
                    continue;
                }
            };
            let olds = old_inputs(registry, suite, head);
            let old = (!olds.is_empty())
                .then(|| pass_rule::count::old_count_over(&olds, settings.alpha, OLD_POWER));
            let pooled = old
                .as_ref()
                .map_or(real.matches, |o| o.matches.max(real.matches));
            let driver = if old.as_ref().is_some_and(|o| o.matches > real.matches) {
                "old"
            } else {
                "real"
            };
            let count = pooled.div_ceil(n_units.max(1)).max(1);
            let need = |n: pass_rule::count::Needed| Need {
                matches: n.matches,
                band: n.band,
                why: n.why,
            };
            suites.insert(
                suite.code().to_string(),
                Computed {
                    units: n_units,
                    real: need(real),
                    old: old.map(need),
                    count,
                    driver: driver.to_string(),
                },
            );
        }
        out.insert(kind, suites);
    }
    if refused.is_empty() {
        Ok(out)
    } else {
        Err(refused)
    }
}

/// The computation as the console prints it, one line per kind and suite.
pub fn render(computed: &BTreeMap<Kind, BTreeMap<String, Computed>>) -> String {
    let mut out = String::new();
    for (kind, suites) in computed {
        for (suite, c) in suites {
            let old = c.old.as_ref().map_or_else(
                || "no head agreement".to_string(),
                |o| format!("{} ({}: {})", o.matches, o.band, o.why),
            );
            out.push_str(&format!(
                "{} {suite}: count {} per unit over {} unit(s); real test {} ({}: {}); \
                 old-engine test {old}; set by the {} test\n",
                kind.code(),
                c.count,
                c.units,
                c.real.matches,
                c.real.band,
                c.real.why,
                c.driver
            ));
        }
    }
    out
}

/// The CHANGELOG line a kind's counts need: `fixed count <kind>: equal N, strength N,
/// formations N, red-card N`.
pub fn changelog_line(kind: Kind, def: &KindDef) -> String {
    let parts: Vec<String> = [
        Suite::Equal,
        Suite::Strength,
        Suite::Formations,
        Suite::RedCard,
    ]
    .iter()
    .map(|s| format!("{} {}", s.code(), def.count_for(*s)))
    .collect();
    format!("fixed count {}: {}", kind.code(), parts.join(", "))
}

/// The headline's words for a kind's counts: `kind change: fixed count equal N, strength N,
/// formations N per pairing, red-card N per arm`.
pub fn headline_count(kind: Kind, def: &KindDef) -> String {
    format!(
        "kind {}: fixed count equal {}, strength {}, formations {} per pairing, red-card {} per \
         arm",
        kind.code(),
        def.count_for(Suite::Equal),
        def.count_for(Suite::Strength),
        def.count_for(Suite::Formations),
        def.count_for(Suite::RedCard)
    )
}

/// Every kind's count line that `changelog` lacks; empty when each kind's counts have their
/// line, so a count changed without its CHANGELOG entry is named.
#[cfg(test)]
pub fn missing_changelog_lines(table: &KindTable, changelog: &str) -> Vec<String> {
    Kind::ALL
        .into_iter()
        .map(|k| changelog_line(k, table.get(k)))
        .filter(|line| !changelog.contains(line.as_str()))
        .collect()
}

/// The kinds table at the earlier design count (1000 matches over 30 leagues), for tests
/// that build a band file by hand.
#[cfg(test)]
pub fn design_kinds() -> serde_json::Value {
    let count =
        serde_json::json!({"equal": 1000, "strength": 1000, "formations": 1000, "red-card": 1000});
    serde_json::json!({
        "change": {"seeds": {"from": 0, "to": 999_999},
                   "rare_margin": {"factor": 2.0, "reason": "change runs catch only large errors"},
                   "count": count, "leagues": 30},
        "milestone": {"seeds": {"from": 1_000_000, "to": 1_999_999},
                      "rare_margin": {"factor": 1.0, "reason": "the normal margin"},
                      "count": count, "leagues": 30},
        "release": {"seeds": {"from": 2_000_000, "to": 2_999_999},
                    "rare_margin": {"factor": 1.0, "reason": "the normal margin"},
                    "count": count, "leagues": 30, "tuning": "refused"}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(from: u64, to: u64, factor: f64) -> KindDef {
        KindDef {
            seeds: SeedSet { from, to },
            rare_margin: RareMargin {
                factor,
                reason: "a reason".into(),
            },
            count: ["equal", "strength", "formations", "red-card"]
                .into_iter()
                .map(|s| (s.to_string(), 1000))
                .collect(),
            leagues: 30,
            computed: BTreeMap::new(),
            tuning: None,
        }
    }

    fn table() -> KindTable {
        let mut release = def(2_000_000, 2_999_999, 1.0);
        release.tuning = Some("refused".into());
        KindTable {
            change: def(0, 999_999, 2.0),
            milestone: def(1_000_000, 1_999_999, 1.0),
            release,
        }
    }

    #[test]
    fn a_sound_table_holds_its_rules() {
        assert_eq!(table().check(), Vec::new());
    }

    #[test]
    fn overlapping_seed_sets_are_refused() {
        let mut t = table();
        t.milestone.seeds.from = 999_000;
        let refusals = t.check();
        assert!(
            refusals
                .iter()
                .any(|(f, r)| f == "kinds.milestone.seeds" && r.contains("overlaps the change")),
            "{refusals:?}"
        );
    }

    #[test]
    fn a_factor_below_one_and_a_missing_count_are_refused() {
        let mut t = table();
        t.change.rare_margin.factor = 0.5;
        t.release.count.remove("red-card");
        t.release.tuning = None;
        let fields: Vec<String> = t.check().into_iter().map(|(f, _)| f).collect();
        for want in [
            "kinds.change.rare_margin.factor",
            "kinds.release.count.red-card",
            "kinds.release.tuning",
        ] {
            assert!(fields.iter().any(|f| f == want), "{want}: {fields:?}");
        }
    }

    #[test]
    fn a_computed_block_holds_no_real_figure() {
        let raw = serde_json::json!({
            "units": 1,
            "real": {"matches": 10, "band": "b", "why": "w"},
            "count": 10,
            "driver": "real",
            "real_error": 0.1
        });
        let err = serde_json::from_value::<Computed>(raw)
            .unwrap_err()
            .to_string();
        assert!(err.contains("real_error"), "{err}");
    }

    #[test]
    fn a_seed_names_its_kind_and_tuning_refuses_the_release_seeds() {
        let t = table();
        assert_eq!(t.kind_of(9), Some(Kind::Change));
        assert_eq!(t.kind_of(1_500_000), Some(Kind::Milestone));
        assert_eq!(t.kind_of(2_000_000), Some(Kind::Release));
        assert!(t.check_seed(Kind::Change, 42).is_ok());
        let why = t.check_seed(Kind::Milestone, 9).unwrap_err();
        assert!(why.contains("belongs to the change kind"), "{why}");
        assert!(t.refuse_release_seed(2_000_000).is_err());
        assert!(t.refuse_release_seed(9).is_ok());
    }

    #[test]
    fn the_fast_model_fit_seeds_are_not_release_seeds() {
        let t = table();
        for seed in [crate::fast_model::FIT_SEED, crate::fast_model::CHECK_SEED] {
            assert!(t.refuse_release_seed(seed).is_ok(), "seed {seed}");
        }
    }

    #[test]
    fn the_changelog_line_names_every_count() {
        let t = table();
        let line = changelog_line(Kind::Change, &t.change);
        assert_eq!(
            line,
            "fixed count change: equal 1000, strength 1000, formations 1000, red-card 1000"
        );
        let text = format!(
            "{}\n{}\n{}\n",
            line,
            changelog_line(Kind::Milestone, &t.milestone),
            changelog_line(Kind::Release, &t.release)
        );
        assert!(missing_changelog_lines(&t, &text).is_empty());
        let mut edited = t.clone();
        edited.change.count.insert("equal".into(), 1200);
        let missing = missing_changelog_lines(&edited, &text);
        assert_eq!(missing.len(), 1);
        assert!(missing[0].contains("equal 1200"), "{missing:?}");
    }
}
