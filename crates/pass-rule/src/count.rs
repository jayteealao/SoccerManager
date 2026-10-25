//! The fixed count of a change run.
//!
//! A change run plays one fixed number of paired matches per suite unit. Two tests each need
//! a number, and the count is the larger:
//!
//! - **The real test.** Every gating band must hold its 97 percent floor at the run's size:
//!   `factor * sqrt(sigma_e(N)^2 + sigma_r^2) <= margin`, with the engine error
//!   `sigma_e(N)^2 = between_league_sd^2 / G + per_match_sd^2 / N` (both terms over `N` when
//!   the band resamples matches, not leagues), and every rare band must reach its minimum
//!   exposure. The smallest such `N` is the real-test count.
//! - **The old-engine test.** A band's paired change must be told from its noticeable error
//!   `e` at 80 percent power: `N = 2 (1 - rho) per_match_sd^2 (z(1 - alpha) + z(power))^2 / e^2`,
//!   with `rho` the lower bound of the measured agreement between paired matches.
//!
//! A band whose floor no `N` meets (its league or real error alone exceeds its margin) is
//! refused by name; the count never hides it.

use crate::special::norm_quantile;

/// One floor check of a band at the run's size: a margin (or a design band's cap) and the
/// errors behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct FloorInput {
    pub band: String,
    /// "margin" or "cap".
    pub which: &'static str,
    pub margin: f64,
    pub per_match_sd: f64,
    pub between_league_sd: f64,
    /// True when the band resamples whole leagues, so the between-league term is over the
    /// run's leagues; false when it resamples matches, so both terms are over `N`.
    pub per_league: bool,
    pub real_error: f64,
    /// The band's minimum exposure in matches (0 when it has none to reach).
    pub min_exposure: f64,
    /// A rare band holds its floor by its exact bound at the kind's margin; only its
    /// minimum exposure bounds the count.
    pub rare: bool,
}

/// The count one test needs and the band that drives it.
#[derive(Debug, Clone, PartialEq)]
pub struct Needed {
    pub matches: u32,
    pub band: String,
    pub why: String,
}

/// A band whose floor no count meets.
#[derive(Debug, Clone, PartialEq)]
pub struct Unreachable {
    pub band: String,
    pub which: &'static str,
    pub margin: f64,
    /// The margin the band would need at an endless run.
    pub needed: f64,
}

/// The smallest count at which one check holds, or `None` when none does.
pub fn smallest_for(input: &FloorInput, factor: f64, leagues: f64) -> Option<f64> {
    if input.rare {
        return Some(input.min_exposure.max(1.0));
    }
    let league_term = if input.per_league {
        input.between_league_sd.powi(2) / leagues.max(1.0)
    } else {
        0.0
    };
    let room = (input.margin / factor).powi(2) - input.real_error.powi(2) - league_term;
    let over_n = input.per_match_sd.powi(2)
        + if input.per_league {
            0.0
        } else {
            input.between_league_sd.powi(2)
        };
    let floor_n = if over_n <= 0.0 {
        (room >= 0.0).then_some(1.0)?
    } else if room <= 0.0 {
        return None;
    } else {
        // A part in a million of slack, as the floor's own check allows.
        (over_n / room * (1.0 - 1e-6)).ceil()
    };
    Some(floor_n.max(input.min_exposure).max(1.0))
}

/// The real-test count: the smallest `N` at which every check holds its floor at `factor`
/// errors, with `leagues` leagues per run. Every band whose floor no count meets is refused.
pub fn real_count(
    inputs: &[FloorInput],
    factor: f64,
    leagues: f64,
) -> Result<Needed, Vec<Unreachable>> {
    let mut best: Option<(f64, &FloorInput)> = None;
    let mut refused = Vec::new();
    for input in inputs {
        match smallest_for(input, factor, leagues) {
            Some(n) => {
                if best.is_none_or(|(b, _)| n > b) {
                    best = Some((n, input));
                }
            }
            None => {
                let league_term = if input.per_league {
                    input.between_league_sd.powi(2) / leagues.max(1.0)
                } else {
                    0.0
                };
                refused.push(Unreachable {
                    band: input.band.clone(),
                    which: input.which,
                    margin: input.margin,
                    needed: factor * (input.real_error.powi(2) + league_term).sqrt(),
                });
            }
        }
    }
    if !refused.is_empty() {
        return Err(refused);
    }
    let (n, input) = best.map_or((1.0, None), |(n, i)| (n, Some(i)));
    Ok(Needed {
        matches: n.min(f64::from(u32::MAX)) as u32,
        band: input.map_or_else(String::new, |i| i.band.clone()),
        why: match input {
            Some(i) if i.rare || n <= i.min_exposure => "its minimum exposure".into(),
            Some(i) => format!("its {} floor", i.which),
            None => "no gating band".into(),
        },
    })
}

/// The count at which one band's paired change is told from its noticeable error at
/// `power`, with paired-match agreement `rho_lo`.
pub fn old_count(per_match_sd: f64, rho_lo: f64, noticeable: f64, alpha: f64, power: f64) -> u32 {
    if noticeable <= 0.0 || per_match_sd <= 0.0 {
        return 1;
    }
    let z = norm_quantile(1.0 - alpha) + norm_quantile(power);
    let rho = rho_lo.clamp(-1.0, 1.0);
    let n = 2.0 * (1.0 - rho) * per_match_sd.powi(2) * z.powi(2) / noticeable.powi(2);
    n.ceil().clamp(1.0, f64::from(u32::MAX)) as u32
}

/// One band's inputs to the old-engine count.
#[derive(Debug, Clone, PartialEq)]
pub struct OldInput {
    pub band: String,
    pub per_match_sd: f64,
    pub rho_lo: f64,
    pub noticeable: f64,
}

/// The old-engine count over every band: the largest band's count.
pub fn old_count_over(inputs: &[OldInput], alpha: f64, power: f64) -> Needed {
    inputs
        .iter()
        .map(|i| {
            (
                old_count(i.per_match_sd, i.rho_lo, i.noticeable, alpha, power),
                i,
            )
        })
        .max_by_key(|(n, _)| *n)
        .map_or_else(
            || Needed {
                matches: 1,
                band: String::new(),
                why: "no band with a measured agreement".into(),
            },
            |(n, i)| Needed {
                matches: n,
                band: i.band.clone(),
                why: format!(
                    "told from its noticeable error {} at agreement {:.4}",
                    i.noticeable, i.rho_lo
                ),
            },
        )
}

/// Which test sets the fixed count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Driver {
    Real,
    Old,
}

/// The fixed count: the larger of the two, with the test and band that set it.
pub fn fixed_count(real: &Needed, old: &Needed) -> (u32, Driver) {
    if old.matches > real.matches {
        (old.matches, Driver::Old)
    } else {
        (real.matches, Driver::Real)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::floor::{engine_error, floor_factor, holds, needed_margin};

    fn band(margin: f64, sd: f64, between: f64, real: f64) -> FloorInput {
        FloorInput {
            band: "b".into(),
            which: "margin",
            margin,
            per_match_sd: sd,
            between_league_sd: between,
            per_league: true,
            real_error: real,
            min_exposure: 0.0,
            rare: false,
        }
    }

    #[test]
    fn a_band_at_its_floor_at_one_thousand_matches_needs_one_thousand() {
        let f = floor_factor(0.05, 0.97, 154);
        let sigma_e = engine_error(2.4, 0.3, 1000.0, 30.0);
        let m = needed_margin(sigma_e, 0.05, 0.05, 0.97, 154);
        let n = real_count(&[band(m, 2.4, 0.3, 0.05)], f, 30.0).unwrap();
        assert_eq!(n.matches, 1000);
        // And the floor holds there, and not one match below (to the floor's own slack).
        assert!(holds(m, needed_margin(sigma_e, 0.05, 0.05, 0.97, 154)));
        let below = engine_error(2.4, 0.3, 990.0, 30.0);
        assert!(!holds(m, needed_margin(below, 0.05, 0.05, 0.97, 154)));
    }

    #[test]
    fn the_old_engine_count_is_the_closed_form() {
        // 2 * 0.5 * (1.6449 + 0.8416)^2 / 0.01 = 618.3, so 619.
        assert_eq!(old_count(1.0, 0.5, 0.1, 0.05, 0.8), 619);
        assert!(old_count(1.0, 0.9, 0.1, 0.05, 0.8) < 619);
    }

    #[test]
    fn a_floor_that_no_count_meets_is_refused_by_name() {
        let f = floor_factor(0.05, 0.97, 154);
        let mut b = band(0.1, 2.0, 0.0, 0.05);
        b.band = "too_narrow".into();
        let err = real_count(&[band(10.0, 2.0, 0.0, 0.0), b], f, 30.0).unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].band, "too_narrow");
        assert!((err[0].needed - f * 0.05).abs() < 1e-12);
    }

    #[test]
    fn a_rare_band_needs_its_minimum_exposure() {
        let f = floor_factor(0.05, 0.97, 154);
        let mut rare = band(0.01, 0.1, 0.0, 0.0);
        rare.rare = true;
        rare.min_exposure = 5000.0;
        rare.band = "rare".into();
        let n = real_count(&[band(10.0, 2.0, 0.0, 0.0), rare], f, 30.0).unwrap();
        assert_eq!((n.matches, n.band.as_str()), (5000, "rare"));
    }

    #[test]
    fn a_band_that_resamples_matches_puts_both_spreads_over_the_count() {
        let f = floor_factor(0.05, 0.97, 154);
        let mut b = band(1.0, 2.0, 1.0, 0.0);
        b.per_league = false;
        let n = smallest_for(&b, f, 30.0).unwrap();
        assert_eq!(n, (5.0 / (1.0 / f).powi(2) * (1.0 - 1e-6)).ceil());
    }

    #[test]
    fn the_fixed_count_is_the_larger() {
        let r = Needed {
            matches: 800,
            band: "a".into(),
            why: String::new(),
        };
        let o = Needed {
            matches: 900,
            band: "b".into(),
            why: String::new(),
        };
        assert_eq!(fixed_count(&r, &o), (900, Driver::Old));
        assert_eq!(fixed_count(&o, &r), (900, Driver::Real));
    }
}
