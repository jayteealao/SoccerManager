//! Effective ability: a player's ratings moved by his states, within caps.
//!
//! Each modifier gives a delta in rating points per attribute group (technical, mental,
//! physical, goalkeeping). A family (body, mind, familiarity, surroundings) acts only on the
//! groups it owns; its deltas are summed and clamped to the family's cap. The four families
//! are soft-combined per group ([`crate::modules::modifier::soft_combine`]), the result is
//! clamped to the total cap and rounded to a tenth, and each effective rating is the base
//! rating plus its group's delta, kept inside 1.0 to 20.0.
//!
//! On the curve a delta `d` is the factor `e^(d / width)`: `F(r + d) = F(r) · e^(d / width)`,
//! so a delta in rating points is what a factor on the curve value was before, now bounded in
//! the units the caps use.

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::attributes::{AttributeSchema, Group};
use crate::modules::modifier::{FAMILY_COUNT, Family, soft_combine};
use crate::player::Attributes;
use crate::rating::{MAX_TENTHS, MIN_TENTHS, Rating};

/// The number of attribute groups, in [`Group`] order.
pub const GROUP_COUNT: usize = 4;

/// The groups in index order.
pub const GROUPS: [Group; GROUP_COUNT] = [
    Group::Technical,
    Group::Mental,
    Group::Physical,
    Group::Goalkeeping,
];

/// The states block of the contract (`engine.contract.states`): the caps and which groups
/// each family acts on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StatesTuning {
    #[garde(dive)]
    pub caps: Caps,
    #[garde(dive)]
    pub family_groups: FamilyGroups,
}

/// Each cap as `[lowest, highest]` in rating points, the lowest at most 0 and the highest at
/// least 0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Caps {
    #[garde(custom(check_cap))]
    pub body: [f64; 2],
    #[garde(custom(check_cap))]
    pub mind: [f64; 2],
    #[garde(custom(check_cap))]
    pub familiarity: [f64; 2],
    #[garde(custom(check_cap))]
    pub surroundings: [f64; 2],
    #[garde(custom(check_cap))]
    pub total: [f64; 2],
}

impl Caps {
    /// The cap of `family`.
    pub fn of(&self, family: Family) -> [f64; 2] {
        match family {
            Family::Body => self.body,
            Family::Mind => self.mind,
            Family::Familiarity => self.familiarity,
            Family::Surroundings => self.surroundings,
        }
    }
}

/// The largest change a cap may allow, in rating points. A capped delta is stored in tenths
/// in an `i8`, which holds -12.8 to 12.7 points, so a wider cap could not be kept.
const CAP_LIMIT: f64 = 12.7;

fn check_cap(cap: &[f64; 2], _ctx: &()) -> garde::Result {
    let [lo, hi] = *cap;
    if !(-CAP_LIMIT..=0.0).contains(&lo) || !(0.0..=CAP_LIMIT).contains(&hi) {
        return Err(garde::Error::new(format!(
            "[{lo}, {hi}]; the lowest must be -12.7 to 0 and the highest 0 to 12.7"
        )));
    }
    Ok(())
}

/// The attribute groups each family acts on. A group a family does not list ignores that
/// family's deltas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FamilyGroups {
    #[garde(custom(check_groups))]
    pub body: Vec<Group>,
    #[garde(custom(check_groups))]
    pub mind: Vec<Group>,
    #[garde(custom(check_groups))]
    pub familiarity: Vec<Group>,
    #[garde(custom(check_groups))]
    pub surroundings: Vec<Group>,
}

impl FamilyGroups {
    /// `true` when `family` acts on `group`.
    pub fn owns(&self, family: Family, group: Group) -> bool {
        let list = match family {
            Family::Body => &self.body,
            Family::Mind => &self.mind,
            Family::Familiarity => &self.familiarity,
            Family::Surroundings => &self.surroundings,
        };
        list.contains(&group)
    }
}

fn check_groups(groups: &[Group], _ctx: &()) -> garde::Result {
    for (i, g) in groups.iter().enumerate() {
        if groups[..i].contains(g) {
            return Err(garde::Error::new(format!("{g:?} appears twice")));
        }
    }
    Ok(())
}

impl Default for StatesTuning {
    /// The shipped caps and family groups, as the player contract page sets them out.
    fn default() -> Self {
        use Group::*;
        Self {
            caps: Caps {
                body: [-2.0, 0.5],
                mind: [-1.0, 1.0],
                familiarity: [-1.25, 0.25],
                surroundings: [-0.75, 0.5],
                total: [-5.0, 2.25],
            },
            family_groups: FamilyGroups {
                body: vec![Technical, Physical, Goalkeeping],
                mind: vec![Mental],
                familiarity: vec![Technical, Mental],
                surroundings: vec![Technical, Physical],
            },
        }
    }
}

/// The families in [`Family`] index order.
const FAMILIES: [Family; FAMILY_COUNT] = [
    Family::Body,
    Family::Mind,
    Family::Familiarity,
    Family::Surroundings,
];

/// The capped delta of each group, in tenths of a rating point, from each family's summed
/// deltas in rating points: every group a family does not own drops its delta, each family is
/// clamped to its cap, the families are soft-combined per group, and the result is clamped to
/// the total cap and rounded to a tenth.
pub fn capped(
    per_family: [[f64; GROUP_COUNT]; FAMILY_COUNT],
    t: &StatesTuning,
) -> [i8; GROUP_COUNT] {
    let [lo, hi] = t.caps.total;
    std::array::from_fn(|g| {
        let deltas: [f64; FAMILY_COUNT] = std::array::from_fn(|f| {
            let family = FAMILIES[f];
            if !t.family_groups.owns(family, GROUPS[g]) {
                return 0.0;
            }
            let [flo, fhi] = t.caps.of(family);
            per_family[f][g].clamp(flo, fhi)
        });
        let d = soft_combine(deltas).clamp(lo, hi);
        // Inside the total cap, which the cap check keeps within ±12.7, so it fits an `i8`
        // in tenths.
        (d * 10.0).round() as i8
    })
}

/// The effective ratings of `base`: each visible rating plus the delta of its group, in
/// tenths, kept inside 1.0 to 20.0. A hidden value keeps its base: no state moves it.
pub fn effective(
    base: &Attributes,
    schema: &AttributeSchema,
    deltas: [i8; GROUP_COUNT],
) -> Attributes {
    let mut out = *base;
    for (slot, def) in out.values.iter_mut().zip(&schema.attributes) {
        let d = i16::from(deltas[def.group as usize]);
        if d != 0 && !def.hidden {
            *slot = moved(*slot, d);
        }
    }
    out
}

/// `r` moved by `d` tenths, kept inside 1.0 to 20.0.
fn moved(r: Rating, d: i16) -> Rating {
    let t = (i16::from(r.tenths()) + d).clamp(i16::from(MIN_TENTHS), i16::from(MAX_TENTHS));
    // Inside 10..=200 after the clamp.
    Rating::from_tenths(t as u8)
}

/// The ratings play reads for a player with state `deltas` and form offset `form`, both in
/// tenths: every rating a stage table reads takes its group's delta plus the form offset, that
/// sum kept inside the total cap, then the rating inside 1.0 to 20.0. Pace, which only the
/// speed map reads, takes its group's delta alone, and a hidden value keeps its base. With
/// `form` 0 it equals [`effective`].
pub fn effective_with_form(
    base: &Attributes,
    schema: &AttributeSchema,
    deltas: [i8; GROUP_COUNT],
    form: i8,
    t: &StatesTuning,
) -> Attributes {
    if form == 0 {
        return effective(base, schema, deltas);
    }
    let [lo, hi] = t.caps.total;
    // Inside ±19 points by the cap checks, so inside ±190 tenths.
    let (lo, hi) = ((lo * 10.0).round() as i16, (hi * 10.0).round() as i16);
    let pace = schema.index("pace");
    let mut out = *base;
    for (i, (slot, def)) in out.values.iter_mut().zip(&schema.attributes).enumerate() {
        if def.hidden {
            continue;
        }
        let state = i16::from(deltas[def.group as usize]);
        let d = if Some(i) == pace {
            state
        } else {
            (state + i16::from(form)).clamp(lo, hi)
        };
        if d != 0 {
            *slot = moved(*slot, d);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;

    fn at(f: Family, groups: [f64; GROUP_COUNT]) -> [[f64; GROUP_COUNT]; FAMILY_COUNT] {
        let mut out = [[0.0; GROUP_COUNT]; FAMILY_COUNT];
        out[f as usize] = groups;
        out
    }

    #[test]
    fn each_family_at_its_extreme_hits_its_own_cap_and_moves_only_its_groups() {
        let t = StatesTuning::default();
        for family in FAMILIES {
            for push in [-50.0, 50.0] {
                let d = capped(at(family, [push; GROUP_COUNT]), &t);
                let [lo, hi] = t.caps.of(family);
                for (g, group) in GROUPS.iter().enumerate() {
                    let expect = if t.family_groups.owns(family, *group) {
                        ((if push < 0.0 { lo } else { hi }) * 10.0).round() as i8
                    } else {
                        0
                    };
                    assert_eq!(d[g], expect, "{family:?} {group:?} {push}");
                }
            }
        }
    }

    #[test]
    fn a_cap_wider_than_a_tenths_delta_can_hold_is_refused() {
        let mut t = StatesTuning::default();
        t.caps.total = [-15.0, 2.25];
        assert!(t.validate().is_err(), "a lowest of -15 must be refused");
        t.caps.total = [-12.7, 12.7];
        assert!(t.validate().is_ok());
        // At the widest cap the stored delta is the cap itself, not a saturated value.
        let d = capped([[-50.0; GROUP_COUNT]; FAMILY_COUNT], &{
            let mut w = t.clone();
            w.caps.body = [-12.7, 0.5];
            w.caps.mind = [-12.7, 1.0];
            w.caps.familiarity = [-12.7, 0.25];
            w.caps.surroundings = [-12.7, 0.5];
            w
        });
        assert!(d.contains(&-127), "{d:?}");
    }

    #[test]
    fn a_planted_delta_past_its_cap_is_clamped_to_the_cap() {
        let t = StatesTuning::default();
        for family in FAMILIES {
            let d = capped(at(family, [-9.0; GROUP_COUNT]), &t);
            let lo = (t.caps.of(family)[0] * 10.0).round() as i8;
            assert!(d.iter().all(|&x| x == 0 || x == lo), "{family:?} {d:?}");
            assert!(d.iter().all(|&x| x >= lo));
        }
    }

    #[test]
    fn all_families_at_their_lower_caps_stay_within_the_total() {
        let t = StatesTuning::default();
        let d = capped([[-9.0; GROUP_COUNT]; FAMILY_COUNT], &t);
        for &x in &d {
            assert!(x >= -50, "{d:?}");
        }
        let up = capped([[9.0; GROUP_COUNT]; FAMILY_COUNT], &t);
        for &x in &up {
            assert!(x <= 23, "{up:?}");
        }
        // With a tighter total the clamp binds.
        let mut tight = t.clone();
        tight.caps.total = [-1.5, 0.5];
        let d = capped([[-9.0; GROUP_COUNT]; FAMILY_COUNT], &tight);
        assert!(d.iter().all(|&x| x >= -15), "{d:?}");
        assert!(d.contains(&-15), "{d:?}");
    }

    #[test]
    fn deltas_round_to_a_tenth() {
        let t = StatesTuning::default();
        let d = capped(at(Family::Body, [-0.04, 0.0, -0.36, -0.15]), &t);
        assert_eq!(d, [0, 0, -4, -2]);
    }

    #[test]
    fn effective_ratings_stay_inside_one_to_twenty_and_zero_deltas_give_the_base() {
        let content = shipped_content();
        let schema = &content.attributes;
        let mut base = Attributes {
            values: [Rating::from_tenths(100); crate::data::attributes::MAX_ATTRIBUTES],
            len: schema.len() as u8,
        };
        let pace = schema.index("pace").unwrap();
        let passing = schema.index("passing").unwrap();
        base.values[pace] = Rating::from_tenths(15);
        base.values[passing] = Rating::from_tenths(198);
        let same = effective(&base, schema, [0; GROUP_COUNT]);
        assert_eq!(same.values, base.values);
        let low = effective(&base, schema, [0, 0, -20, 0]);
        assert_eq!(low.get(pace).tenths(), 10, "1.5 − 2.0 reads 1.0");
        let high = effective(&base, schema, [10, 0, 0, 0]);
        assert_eq!(high.get(passing).tenths(), 200, "19.8 + 1.0 reads 20.0");
        assert_eq!(high.get(pace).tenths(), 15, "another group is untouched");
    }

    #[test]
    fn hidden_values_keep_their_base_and_form_moves_every_stage_rating_but_pace() {
        let content = shipped_content();
        let schema = &content.attributes;
        let t = StatesTuning::default();
        let base = Attributes {
            values: [Rating::from_tenths(100); crate::data::attributes::MAX_ATTRIBUTES],
            len: schema.len() as u8,
        };
        let idx = |n: &str| schema.index(n).unwrap();
        let tired = effective(&base, schema, [0, 0, -20, 0]);
        assert_eq!(tired.get(idx("injury_proneness")).tenths(), 100);
        assert_eq!(tired.get(idx("strength")).tenths(), 80);
        let same = effective_with_form(&base, schema, [0, -10, 0, 0], 0, &t);
        assert_eq!(same.values, effective(&base, schema, [0, -10, 0, 0]).values);
        let off = effective_with_form(&base, schema, [0, -10, 0, 0], -7, &t);
        assert_eq!(off.get(idx("passing")).tenths(), 93);
        assert_eq!(
            off.get(idx("decisions")).tenths(),
            83,
            "mental: state and form"
        );
        assert_eq!(off.get(idx("pace")).tenths(), 100, "pace takes no form");
        assert_eq!(off.get(idx("consistency")).tenths(), 100);
        assert_eq!(off.get(idx("injury_proneness")).tenths(), 100);
        // The sum stays inside the total cap.
        let floor = effective_with_form(&base, schema, [0, -50, 0, 0], -18, &t);
        assert_eq!(floor.get(idx("decisions")).tenths(), 50, "10.0 − 5.0");
    }
}
