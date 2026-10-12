//! The staff advice on the named styles (`content/styles.json`) before a match: how well a
//! squad suits each style, with an error drawn from the advice's own seeded generator, and the
//! AI manager's pick of the best. The advice runs before a match exists and reads no match
//! stream; nothing keeps an estimate, and play never reads one.

use sha2::{Digest, Sha256};

use crate::ai::{Setup, pre_match_for};
use crate::data::attributes::{AttributeSchema, Group};
use crate::data::styles::Styles;
use crate::data::tactics::TacticsSchema;
use crate::rng::EngineRng;
use crate::tactics::Tactics;
use crate::team::Team;

/// The advice's estimate of one style for a squad: how well the lineup the AI would field for
/// it suits its key jobs, with the advice's error added, and the formation of that lineup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StyleEstimate {
    pub fit: f64,
    pub formation: u8,
}

/// The seed of the advice's error for `side` (0 home) of the match with `seed`: SHA-256 over
/// the match seed, the side, the club id, and the label `style-advice`. The advice reads no
/// match stream, so play draws the same numbers with or without it.
fn advice_seed(seed: u64, side: usize, club_id: &str) -> u64 {
    let mut h = Sha256::new();
    h.update(seed.to_le_bytes());
    h.update([side as u8]);
    h.update(club_id.as_bytes());
    h.update(b"style-advice");
    let d = h.finalize();
    u64::from_le_bytes(core::array::from_fn(|i| d[i]))
}

/// The staff advice on each style for `team` (`side` of the match with `seed`), in the styles
/// file's order. For each style: the lineup [`crate::ai::pre_match_for`] picks in each of its formations,
/// the formation whose lineup has the highest summed rating at its slots (the first in the
/// list on a tie), and over that lineup's players at the style's fit positions the weighted
/// mean of the style's fit attributes less the same players' mean over every visible outfield
/// attribute, so a squad's level does not decide the pick; plus a draw of the advice's error.
/// Nothing keeps an estimate: play never reads one.
pub fn style_estimates(
    team: &Team,
    styles: &Styles,
    schema: &TacticsSchema,
    attrs: &AttributeSchema,
    seed: u64,
    side: usize,
) -> Vec<StyleEstimate> {
    let visible: Vec<usize> = attrs
        .attributes
        .iter()
        .enumerate()
        .filter(|(_, d)| {
            !d.hidden && matches!(d.group, Group::Technical | Group::Mental | Group::Physical)
        })
        .map(|(i, _)| i)
        .collect();
    let mut rng = EngineRng::from_seed(advice_seed(seed, side, &team.club_id));
    styles
        .styles
        .iter()
        .map(|style| {
            let mut best: Option<(f64, u8, Setup)> = None;
            for &f in &style.formations {
                let setup = pre_match_for(team, style.tactics(f, schema), schema, attrs);
                let slots = &schema.formations[usize::from(f)].slots;
                let total: f64 = setup
                    .lineup
                    .iter()
                    .zip(slots)
                    .map(|(&s, slot)| team.squad[s].positions[slot.position as usize].decimal())
                    .sum();
                if best.as_ref().is_none_or(|(t, _, _)| total > *t) {
                    best = Some((total, f, setup));
                }
            }
            // A checked style lists at least one formation.
            let Some((_, formation, setup)) = best else {
                return StyleEstimate {
                    fit: f64::NEG_INFINITY,
                    formation: 0,
                };
            };
            let slots = &schema.formations[usize::from(formation)].slots;
            let weight: f64 = style.fit.iter().map(|(_, w)| w).sum();
            let gaps: Vec<f64> = setup
                .lineup
                .iter()
                .zip(slots)
                .filter(|(_, slot)| style.fit_positions.contains(&slot.position))
                .map(|(&s, _)| {
                    let a = &team.squad[s].attributes;
                    let fit = style
                        .fit
                        .iter()
                        .map(|&(k, w)| w * a.get(k).decimal())
                        .sum::<f64>()
                        / weight;
                    let level = visible.iter().map(|&k| a.get(k).decimal()).sum::<f64>()
                        / visible.len().max(1) as f64;
                    fit - level
                })
                .collect();
            let fit = if gaps.is_empty() {
                f64::NEG_INFINITY
            } else {
                gaps.iter().sum::<f64>() / gaps.len() as f64
            };
            StyleEstimate {
                fit: fit + rng.bell(0.0, styles.advice_noise),
                formation,
            }
        })
        .collect()
}

/// The AI manager's pre-match tactics for `team` from the staff advice: the style with the
/// best estimate ([`style_estimates`]; the first on a tie) in the formation the advice found
/// for it. `None` when no style is given.
pub fn advise(
    team: &Team,
    styles: &Styles,
    schema: &TacticsSchema,
    attrs: &AttributeSchema,
    seed: u64,
    side: usize,
) -> Option<Tactics> {
    let estimates = style_estimates(team, styles, schema, attrs, seed, side);
    let (k, best) = estimates.iter().enumerate().fold(
        None::<(usize, StyleEstimate)>,
        |acc, (k, e)| match acc {
            Some((_, b)) if b.fit >= e.fit => acc,
            _ => Some((k, *e)),
        },
    )?;
    Some(styles.styles[k].tactics(best.formation, schema))
}
