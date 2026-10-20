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

/// One stage of an action: the main attribute, up to three shown supports with smaller
/// weights, up to four small weights lighter than every shown support (they act in play but no
/// screen lists them), and where the weights come from: `design`, or `fitted:<reference>` for weights fitted from
/// real data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StageTable {
    #[garde(dive)]
    pub main: Weighted,
    #[serde(default)]
    #[garde(length(max = MAX_SUPPORTS), dive)]
    pub supports: Vec<Weighted>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(length(max = MAX_MINOR), dive)]
    pub minor: Vec<Weighted>,
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

/// The most shown supports a stage has.
pub const MAX_SUPPORTS: usize = 3;
/// The most small weights a stage has.
pub const MAX_MINOR: usize = 4;
/// The most parts a stage blends: the main attribute, the shown supports, the small weights.
const BLEND_PARTS: usize = 1 + MAX_SUPPORTS + MAX_MINOR;

/// The weights of every stage, resolved to attribute indices once per schema.
#[derive(Debug, Clone)]
pub struct Blend {
    /// Per stage: up to [`BLEND_PARTS`] `(attribute index, weight)`, and the total weight.
    parts: [[(usize, f64); BLEND_PARTS]; STAGE_COUNT],
    counts: [usize; STAGE_COUNT],
    totals: [f64; STAGE_COUNT],
    /// Per stage: the consistency degree of its action, how far a form offset moves it.
    degrees: [f64; STAGE_COUNT],
}

impl Blend {
    /// The blend of a validated schema.
    pub fn of(schema: &AttributeSchema) -> Self {
        let mut parts = [[(0usize, 0.0f64); BLEND_PARTS]; STAGE_COUNT];
        let mut counts = [0usize; STAGE_COUNT];
        let mut totals = [0.0; STAGE_COUNT];
        let mut degrees = [1.0; STAGE_COUNT];
        for (i, &(action, stage)) in STAGES.iter().enumerate() {
            // A validated schema gives every action a degree; one built in code without them
            // reads the uniform 1.0.
            degrees[i] = schema
                .consistency_degrees
                .get(&action)
                .copied()
                .unwrap_or(1.0);
            let table = &schema.actions[&action][&stage];
            let all: Vec<&Weighted> = std::iter::once(&table.main)
                .chain(table.supports.iter())
                .chain(table.minor.iter())
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
            degrees,
        }
    }

    /// The consistency degree of stage `i`.
    pub fn degree(&self, i: usize) -> f64 {
        self.degrees[i]
    }

    /// The stage values of a player with ratings `a`: per stage, the weighted mean of the
    /// curve value of each attribute in it, and that value's skill share with the action's
    /// strength. The sum is divided by the total weight once, at the end: scaling by the
    /// curve's 8 is exact, so a player rated 10 throughout has stage values of exactly 8.
    pub fn values(&self, a: &Attributes, c: &ContractTuning) -> StageValues {
        self.values_with_form(a, c, 0)
    }

    /// [`Blend::values`] with the form offset `form`, in tenths of a rating point, applied
    /// after the blend: each stage's value is multiplied by `e^(d · o / width)`, where `o` is
    /// the offset in rating points and `d` the degree of the stage's action, and its share is
    /// taken from the moved value. Since `F(r + o) = F(r) · e^(o / width)`, degree 1.0 moves a
    /// stage as the offset on its ratings did; a higher degree widens the swing, a lower one
    /// narrows it, and degree 0 leaves the stage still.
    pub fn values_with_form(&self, a: &Attributes, c: &ContractTuning, form: i8) -> StageValues {
        self.values_floored(a, c, form, None)
    }

    /// [`Blend::values_with_form`] with the shared floor: given `(base, drop)`, each stage's
    /// value after the form factor is kept at or above its base value less `drop` rating
    /// points ([`super::states::floored`]), before its share is taken.
    pub fn values_floored(
        &self,
        a: &Attributes,
        c: &ContractTuning,
        form: i8,
        floor: Option<(&StageValues, f64)>,
    ) -> StageValues {
        let o = f64::from(form) / 10.0;
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
            let mut v = v / self.totals[i];
            if form != 0 {
                v *= super::consistency::stage_factor(self.degrees[i], o, c.curve.width);
            }
            if let Some((base, drop)) = floor {
                v = super::states::floored(v, base.f[i], drop, c.curve.width);
            }
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
