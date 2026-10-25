//! The smallest margin a band may hold.
//!
//! A sound engine should pass the whole run, every gating band at once, with probability at
//! least `sound_pass` (0.97). With `K` gating bands each must pass with probability
//! `p = sound_pass^(1/K)`. A band passes when its interval of half-width about
//! `z(1 - alpha) * sigma` sits inside the margin around an estimate that itself wanders by
//! `z((1 + p) / 2) * sigma` at that probability, so the margin must be at least the sum of the
//! two times the band's combined error. A narrower margin would fail sound engines more often
//! than the target allows, so the band file refuses it and names the margin the band needs.

use crate::bounds::garwood;
use crate::special::norm_quantile;

/// The factor that multiplies a band's combined error to give its smallest margin.
pub fn floor_factor(alpha: f64, sound_pass: f64, gating: usize) -> f64 {
    let k = gating.max(1) as f64;
    let p = sound_pass.powf(1.0 / k);
    norm_quantile(1.0 - alpha) + norm_quantile((1.0 + p) / 2.0)
}

/// The engine's error at a run of `matches` matches over `leagues` leagues, from the band's
/// measured spreads: `sqrt(between_league_sd^2 / leagues + per_match_sd^2 / matches)`.
pub fn engine_error(per_match_sd: f64, between_league_sd: f64, matches: f64, leagues: f64) -> f64 {
    (between_league_sd.powi(2) / leagues.max(1.0) + per_match_sd.powi(2) / matches.max(1.0)).sqrt()
}

/// The smallest margin of a band whose engine error is `sigma_e` and whose real error is
/// `sigma_r`.
pub fn needed_margin(
    sigma_e: f64,
    sigma_r: f64,
    alpha: f64,
    sound_pass: f64,
    gating: usize,
) -> f64 {
    floor_factor(alpha, sound_pass, gating) * (sigma_e.powi(2) + sigma_r.powi(2)).sqrt()
}

/// The smallest margin of a rare band: the half-width of the engine's exact Garwood interval
/// at the band's minimum exposure, at the expected `rate` per match, each side at
/// `1 - alpha / 2`, plus the real side's half-width at the same level. The interval of the
/// difference is the engine interval minus the real one, so its half-width is the sum.
pub fn needed_rare_margin(rate: f64, min_exposure: f64, sigma_r: f64, alpha: f64) -> f64 {
    let confidence = 1.0 - alpha / 2.0;
    let k = (rate * min_exposure).round().max(0.0) as u64;
    let (lo, hi) = garwood(k, min_exposure, confidence);
    (hi - lo) / 2.0 + norm_quantile(confidence) * sigma_r
}

/// Whether a margin is at or above the floor. A margin within a part in a million of the
/// floor counts as at it, so a margin written to the floor's printed digits loads.
pub fn holds(margin: f64, needed: f64) -> bool {
    margin >= needed * (1.0 - 1e-6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_hundred_fifty_four_gating_bands_need_five_point_three_seven_errors() {
        let f = floor_factor(0.05, 0.97, 154);
        assert!((f - 5.37).abs() < 0.01, "factor {f}");
    }

    #[test]
    fn a_margin_under_the_floor_is_refused_and_one_at_it_holds() {
        let needed = needed_margin(0.03, 0.04, 0.05, 0.97, 154);
        assert!((needed - 0.05 * floor_factor(0.05, 0.97, 154)).abs() < 1e-12);
        assert!(!holds(needed * 0.99, needed));
        assert!(holds(needed, needed));
        assert!(holds(needed * 1.5, needed));
    }

    #[test]
    fn the_engine_error_shrinks_with_leagues_and_matches() {
        let e = engine_error(2.0, 0.3, 1000.0, 30.0);
        assert!((e - (0.09 / 30.0 + 4.0 / 1000.0_f64).sqrt()).abs() < 1e-12);
        assert!(engine_error(2.0, 0.3, 4000.0, 120.0) < e);
    }

    #[test]
    fn a_rare_band_needs_at_least_the_exact_half_width() {
        // 0.02 per match over 1000 matches: 20 events; Garwood 97.5 percent each side.
        let m = needed_rare_margin(0.02, 1000.0, 0.0, 0.05);
        let (lo, hi) = garwood(20, 1000.0, 0.975);
        assert!((m - (hi - lo) / 2.0).abs() < 1e-12);
        assert!(needed_rare_margin(0.02, 1000.0, 0.001, 0.05) > m);
        assert!(needed_rare_margin(0.02, 4000.0, 0.0, 0.05) < m);
    }
}
