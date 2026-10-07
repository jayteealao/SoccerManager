//! The stage tables of the attribute file and the blend that turns a player's ratings into
//! his stage values.

use garde::Validate;
use serde::{Deserialize, Serialize};

use super::params::ContractTuning;
use super::{STAGE_COUNT, STAGES, StageValues, curve};
use crate::data::attributes::AttributeSchema;
use crate::player::Attributes;

/// One attribute in a stage, with its weight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Weighted {
    #[garde(length(min = 2, max = 32))]
    pub attribute: String,
    #[garde(range(min = 0.0, max = 10.0))]
    pub weight: f64,
}

/// One stage of an action: the main attribute, up to three supports with smaller weights,
/// and where the weights come from: `design`, or `fitted:<reference>` for weights fitted from
/// real data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StageTable {
    #[garde(dive)]
    pub main: Weighted,
    #[serde(default)]
    #[garde(length(max = 3), dive)]
    pub supports: Vec<Weighted>,
    #[garde(custom(check_source))]
    pub source: String,
}

fn check_source(source: &str, _ctx: &()) -> garde::Result {
    if source == "design"
        || source
            .strip_prefix("fitted:")
            .is_some_and(|r| !r.is_empty())
    {
        Ok(())
    } else {
        Err(garde::Error::new(format!(
            "{source}; allowed design or fitted:<reference>"
        )))
    }
}

/// The skill gate of one gated skill: the technique rating from which a player tries it, the
/// agility rating from which his body pulls it off, and the log-odds his execution loses
/// below that.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct GateDef {
    #[garde(range(min = 1.0, max = 20.0))]
    pub technique_try: f64,
    #[garde(range(min = 1.0, max = 20.0))]
    pub agility_pull_off: f64,
    #[garde(range(min = 0.0, max = 5.0))]
    pub penalty_k: f64,
}

/// The gated skills that exist in play: the chip (a lofted pass over 25 m) and the take-on
/// (a dribble with an opponent close).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct GateDefs {
    #[garde(dive)]
    pub chip: GateDef,
    #[garde(dive)]
    pub take_on: GateDef,
}

/// The weights of every stage, resolved to attribute indices once per schema.
#[derive(Debug, Clone)]
pub struct Blend {
    /// Per stage: up to four `(attribute index, weight)`, and the total weight.
    parts: [[(usize, f64); 4]; STAGE_COUNT],
    counts: [usize; STAGE_COUNT],
    totals: [f64; STAGE_COUNT],
}

impl Blend {
    /// The blend of a validated schema.
    pub fn of(schema: &AttributeSchema) -> Self {
        let mut parts = [[(0usize, 0.0f64); 4]; STAGE_COUNT];
        let mut counts = [0usize; STAGE_COUNT];
        let mut totals = [0.0; STAGE_COUNT];
        for (i, &(action, stage)) in STAGES.iter().enumerate() {
            let table = &schema.actions[&action][&stage];
            let all: Vec<&Weighted> = std::iter::once(&table.main)
                .chain(table.supports.iter())
                .collect();
            for (slot, w) in parts[i].iter_mut().zip(&all) {
                let index = schema
                    .index(&w.attribute)
                    .expect("a validated schema names only its own attributes");
                *slot = (index, w.weight);
                totals[i] += w.weight;
            }
            counts[i] = all.len();
        }
        Self {
            parts,
            counts,
            totals,
        }
    }

    /// The stage values of a player with ratings `a`: per stage, the weighted mean of the
    /// curve value of each attribute in it, and that value's skill share with the action's
    /// strength. The sum is divided by the total weight once, at the end: scaling by the
    /// curve's 8 is exact, so a player rated 10 throughout has stage values of exactly 8.
    pub fn values(&self, a: &Attributes, c: &ContractTuning) -> StageValues {
        let mut on_curve = [f64::NAN; crate::data::attributes::MAX_ATTRIBUTES];
        let mut out = StageValues::flat(0.0, 0.5);
        for (i, &(action, _)) in STAGES.iter().enumerate() {
            let mut v = 0.0;
            for &(index, w) in &self.parts[i][..self.counts[i]] {
                if on_curve[index].is_nan() {
                    on_curve[index] = curve::f(a.get(index).decimal(), &c.curve);
                }
                v += w * on_curve[index];
            }
            let v = v / self.totals[i];
            out.f[i] = v;
            out.share[i] = curve::share(c.actions.of(action).k(), v, &c.curve);
        }
        out
    }
}

/// The stage values of a player with ratings `a` under `schema` and the contract block `c`.
pub fn stage_values(a: &Attributes, schema: &AttributeSchema, c: &ContractTuning) -> StageValues {
    Blend::of(schema).values(a, c)
}
