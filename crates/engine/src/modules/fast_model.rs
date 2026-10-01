//! The fast-model slot (`engine.fast-model`): a results model fitted from full-engine
//! results. Given the two teams as they kick off, it returns a final score and the goal
//! events of a 90-minute match without playing a tick.
//!
//! Nothing in the game uses it yet. The module sits in a private field of
//! [`ResolvedModules`], and [`resolve`] is the one way to reach it; a source-scan test fails
//! the build when anything outside the fit and check commands calls it, so no runtime path
//! can fall back to it in place of the full engine.
//!
//! The model: the goals of each side are negative binomial with mean
//! `exp(base + home * [home side] + slope * d + curve * d^2)`, where `d` is the side's
//! strength minus the other side's, over 10, and one shared dispersion; the joint table over
//! 0 to 15 goals a side carries the Dixon-Coles factor on 0-0, 1-0, 0-1 and 1-1 and is
//! normalised. A goal's minute is drawn from 90 shares fitted from the full engine's goal
//! minutes. The model plays regulation time only.

use serde::{Deserialize, Serialize};

use super::ResolvedModules;
use super::card::ModuleCard;
use crate::error::EngineError;
use crate::rng::EngineRng;
use crate::sim::{EngineEvent, EngineEventKind, MatchConfig};
use crate::ticks_for_minutes;

/// Goals a side can score in the score table: 0 to 15.
pub const MAX_GOALS: usize = 16;
/// The minutes of regulation time a goal can fall in.
pub const MINUTES: usize = 90;
/// The fit file's layout version.
pub const FIT_VERSION: u32 = 1;
/// The fit file inside the content folder. `Content::load` never reads it, so it moves no
/// content hash.
pub const FIT_FILE: &str = "fast-model.json";
/// The name of the fitted model, as the slot file and the fit file name it.
pub const FITTED_SCORES: &str = "fitted-scores";

/// The six fitted parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FastParams {
    /// Log goals of a side at equal strength, away.
    pub base: f64,
    /// Added to the log goals of the home side.
    pub home: f64,
    /// Log goals per unit of strength difference over 10.
    pub slope: f64,
    /// Log goals per squared unit of strength difference over 10.
    pub curve: f64,
    /// The negative binomial dispersion `k`: the variance is `mean + mean^2 / k`.
    pub dispersion: f64,
    /// The Dixon-Coles low-score factor.
    pub rho: f64,
}

/// What the model plays from: its parameters and the share of goals in each minute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FastFit {
    pub params: FastParams,
    /// One share per minute of regulation time, 0 to 89; they sum to 1.
    pub minute_shares: Vec<f64>,
}

impl FastFit {
    /// Refuses a fit the model cannot play: a dispersion that is not positive, a factor that
    /// is not finite, or minute shares that are not 90 non-negative values summing to 1.
    pub fn check(&self) -> Result<(), EngineError> {
        let p = &self.params;
        let finite = [p.base, p.home, p.slope, p.curve, p.dispersion, p.rho]
            .iter()
            .all(|v| v.is_finite());
        if !finite || p.dispersion <= 0.0 {
            return Err(EngineError::InvalidConfig(
                "the fast-model parameters must be finite with a positive dispersion".into(),
            ));
        }
        let sum: f64 = self.minute_shares.iter().sum();
        if self.minute_shares.len() != MINUTES
            || self
                .minute_shares
                .iter()
                .any(|s| !s.is_finite() || *s < 0.0)
            || (sum - 1.0).abs() > 1e-6
        {
            return Err(EngineError::InvalidConfig(format!(
                "the fast-model minute shares must be {MINUTES} values summing to 1"
            )));
        }
        Ok(())
    }
}

/// The two teams as they kick off: each side's strength, home first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KickOff {
    /// The mean of every attribute value of the eleven players a side starts with.
    pub strength: [f64; 2],
}

/// The kick-off of the match `config` describes: the starting elevens the pre-match setup
/// picked, so the fast model and the full engine start from the same teams.
pub fn kick_off(config: &MatchConfig) -> KickOff {
    let strength = [0, 1].map(|team| {
        let (sum, count) = config
            .players
            .iter()
            .filter(|p| p.team == team)
            .flat_map(|p| p.attributes.iter())
            .fold((0u64, 0u64), |(s, n), v| (s + u64::from(v), n + 1));
        if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        }
    });
    KickOff { strength }
}

/// A match the fast model played.
#[derive(Debug, Clone, PartialEq)]
pub struct FastMatch {
    /// The final score, home first.
    pub scores: [u32; 2],
    /// Kick-off, the goals in minute order, half-time, the second-half kick-off, full time.
    pub events: Vec<EngineEvent>,
}

/// A results model in the fast-model slot.
pub trait FastModel: Send + Sync + 'static {
    /// Plays a match from `kick_off` with `fit`, drawing from `seed`.
    fn play(&self, fit: &FastFit, kick_off: &KickOff, seed: u64) -> Result<FastMatch, EngineError>;
}

/// The module of the fast-model slot, for the fit and check commands only. Calling it from
/// any other place fails the build's source-scan test.
pub fn resolve(modules: &ResolvedModules) -> &'static dyn FastModel {
    modules.fast_model
}

/// The mean goals of each side, home first.
pub fn means(params: &FastParams, kick_off: &KickOff) -> [f64; 2] {
    let d = (kick_off.strength[0] - kick_off.strength[1]) / 10.0;
    let side =
        |d: f64, home: f64| libm::exp(params.base + home + params.slope * d + params.curve * d * d);
    [side(d, params.home), side(-d, 0.0)]
}

/// The negative binomial probabilities of 0 to 15 goals with mean `mean` and dispersion `k`.
pub fn goal_pmf(mean: f64, k: f64) -> [f64; MAX_GOALS] {
    let mut out = [0.0; MAX_GOALS];
    let q = mean / (k + mean);
    out[0] = libm::pow(k / (k + mean), k);
    for y in 1..MAX_GOALS {
        let prev = y as f64 - 1.0;
        out[y] = out[y - 1] * (prev + k) / (prev + 1.0) * q;
    }
    out
}

/// The Dixon-Coles factor of the score `home`-`away` with means `lambda` and `mu`.
pub fn tau(home: usize, away: usize, lambda: f64, mu: f64, rho: f64) -> f64 {
    let t = match (home, away) {
        (0, 0) => 1.0 - lambda * mu * rho,
        (0, 1) => 1.0 + lambda * rho,
        (1, 0) => 1.0 + mu * rho,
        (1, 1) => 1.0 - rho,
        _ => 1.0,
    };
    t.max(0.0)
}

/// The normalised score table: row = home goals, column = away goals.
pub fn score_table(params: &FastParams, kick_off: &KickOff) -> [[f64; MAX_GOALS]; MAX_GOALS] {
    let [lambda, mu] = means(params, kick_off);
    let home = goal_pmf(lambda, params.dispersion);
    let away = goal_pmf(mu, params.dispersion);
    let mut table = [[0.0; MAX_GOALS]; MAX_GOALS];
    let mut sum = 0.0;
    for (h, row) in table.iter_mut().enumerate() {
        for (a, cell) in row.iter_mut().enumerate() {
            *cell = home[h] * away[a] * tau(h, a, lambda, mu, params.rho);
            sum += *cell;
        }
    }
    if sum > 0.0 {
        for row in &mut table {
            for cell in row.iter_mut() {
                *cell /= sum;
            }
        }
    }
    table
}

/// The fitted model, `fitted-scores@1`.
pub struct FittedScoresV1;

impl FastModel for FittedScoresV1 {
    fn play(&self, fit: &FastFit, kick_off: &KickOff, seed: u64) -> Result<FastMatch, EngineError> {
        fit.check()?;
        let table = score_table(&fit.params, kick_off);
        let mut rng = EngineRng::from_seed(seed);
        let u = rng.next_f64();
        let mut acc = 0.0;
        let mut scores = [(MAX_GOALS - 1) as u32; 2];
        'find: for (h, row) in table.iter().enumerate() {
            for (a, cell) in row.iter().enumerate() {
                acc += cell;
                if u < acc {
                    scores = [h as u32, a as u32];
                    break 'find;
                }
            }
        }
        // Each goal takes a minute from the shares; goals in the same minute keep draw order.
        let mut goals: Vec<(u32, usize)> = Vec::new();
        for (team, &n) in scores.iter().enumerate() {
            for _ in 0..n {
                goals.push((minute(&fit.minute_shares, rng.next_f64()), team));
            }
        }
        goals.sort_by_key(|&(m, _)| m);
        Ok(FastMatch {
            scores,
            events: events(&goals),
        })
    }
}

/// The minute whose share holds `u` on the cumulative scale.
fn minute(shares: &[f64], u: f64) -> u32 {
    let mut acc = 0.0;
    for (m, s) in shares.iter().enumerate() {
        acc += s;
        if u < acc {
            return m as u32;
        }
    }
    (MINUTES - 1) as u32
}

/// The event stream of a match whose goals are `goals` (minute, team) in minute order.
fn events(goals: &[(u32, usize)]) -> Vec<EngineEvent> {
    let per_minute = ticks_for_minutes(1);
    let half = ticks_for_minutes(45);
    let mut out = vec![event(EngineEventKind::KickOff, 0, Some(0), [0, 0], 0)];
    let mut scores = [0u32; 2];
    let mut half_time_done = false;
    let half_time = |out: &mut Vec<EngineEvent>, scores: [u32; 2]| {
        out.push(event(EngineEventKind::HalfTime, half, None, scores, 45));
        out.push(event(EngineEventKind::KickOff, half, Some(1), scores, 45));
    };
    for &(m, team) in goals {
        if m >= 45 && !half_time_done {
            half_time(&mut out, scores);
            half_time_done = true;
        }
        scores[team] += 1;
        out.push(event(
            EngineEventKind::Goal,
            m * per_minute + per_minute / 2,
            Some(team),
            scores,
            m,
        ));
    }
    if !half_time_done {
        half_time(&mut out, scores);
    }
    out.push(event(
        EngineEventKind::FullTime,
        ticks_for_minutes(90),
        None,
        scores,
        90,
    ));
    out
}

fn event(
    kind: EngineEventKind,
    tick: u32,
    team: Option<usize>,
    scores: [u32; 2],
    minute: u32,
) -> EngineEvent {
    EngineEvent {
        tick,
        kind,
        team,
        scores,
        minute,
        minute_added: None,
        player: None,
        secondary: None,
        card: None,
        advantage: None,
        added_time_s: None,
        spot: None,
        detail: None,
        period: None,
        shootout_round: None,
        shootout_scored: None,
        shootout_scores: None,
        decided_by: None,
    }
}

/// The off version: the slot is off, so there is no model to play.
pub struct FastModelOff;

impl FastModel for FastModelOff {
    fn play(&self, _: &FastFit, _: &KickOff, _: u64) -> Result<FastMatch, EngineError> {
        Err(EngineError::InvalidConfig(
            "the fast-model slot is off".into(),
        ))
    }
}

pub const FITTED_SCORES_V1_CARD: ModuleCard = ModuleCard {
    purpose: "A results model fitted from full-engine results: a final score and the goal \
              events of a 90-minute match from the two teams at kick-off, with no ticks.",
    inputs: "Each side's strength at kick-off (the mean attribute of its starting eleven) and \
             the fit file content/fast-model.json.",
    outputs: "The final score and the kick-off, goal, half-time and full-time events.",
    tuning: &["none"],
    calibration: "none: fitted to equal the full engine on its own check, not to a band",
    keys: &[],
};

pub const FAST_MODEL_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "Switches the slot off: the fast model refuses to play.",
    inputs: "None.",
    outputs: "A refusal that names the slot.",
    tuning: &["none"],
    calibration: "none: the slot is off",
    keys: &[],
};

#[cfg(test)]
mod tests {
    use super::*;

    fn fit() -> FastFit {
        FastFit {
            params: FastParams {
                base: -0.3,
                home: 0.05,
                slope: 1.2,
                curve: 0.1,
                dispersion: 6.0,
                rho: -0.08,
            },
            minute_shares: vec![1.0 / MINUTES as f64; MINUTES],
        }
    }

    #[test]
    fn the_negative_binomial_sums_to_one_and_has_its_mean() {
        let pmf = goal_pmf(1.4, 5.0);
        let sum: f64 = pmf.iter().sum();
        let mean: f64 = pmf.iter().enumerate().map(|(y, p)| y as f64 * p).sum();
        assert!((sum - 1.0).abs() < 1e-6, "{sum}");
        assert!((mean - 1.4).abs() < 1e-4, "{mean}");
    }

    #[test]
    fn the_table_is_normalised_and_the_factor_lifts_the_draws() {
        let ko = KickOff {
            strength: [50.0, 50.0],
        };
        let table = score_table(&fit().params, &ko);
        let sum: f64 = table.iter().flatten().sum();
        assert!((sum - 1.0).abs() < 1e-12);
        let mut plain = fit();
        plain.params.rho = 0.0;
        let flat = score_table(&plain.params, &ko);
        assert!(table[0][0] > flat[0][0]);
        assert!(table[1][1] > flat[1][1]);
        assert!(table[1][0] < flat[1][0]);
    }

    #[test]
    fn a_stronger_side_scores_more() {
        let [h, a] = means(
            &fit().params,
            &KickOff {
                strength: [58.0, 50.0],
            },
        );
        assert!(h > 2.0 * a, "{h} {a}");
    }

    #[test]
    fn a_seed_plays_the_same_match_and_the_goals_match_the_score() {
        let ko = KickOff {
            strength: [55.0, 50.0],
        };
        let one = FittedScoresV1.play(&fit(), &ko, 9).unwrap();
        assert_eq!(one, FittedScoresV1.play(&fit(), &ko, 9).unwrap());
        for seed in 0..500 {
            let m = FittedScoresV1.play(&fit(), &ko, seed).unwrap();
            let goals = m
                .events
                .iter()
                .filter(|e| e.kind == EngineEventKind::Goal)
                .count() as u32;
            assert_eq!(goals, m.scores[0] + m.scores[1]);
            assert_eq!(m.events.last().unwrap().scores, m.scores);
            assert!(m.events.windows(2).all(|w| w[0].tick <= w[1].tick));
        }
    }

    #[test]
    fn the_off_version_refuses_and_a_bad_fit_is_refused() {
        let ko = KickOff {
            strength: [50.0, 50.0],
        };
        let err = FastModelOff.play(&fit(), &ko, 1).unwrap_err().to_string();
        assert!(err.contains("the fast-model slot is off"), "{err}");
        let mut bad = fit();
        bad.minute_shares.pop();
        assert!(FittedScoresV1.play(&bad, &ko, 1).is_err());
        let mut bad = fit();
        bad.params.dispersion = 0.0;
        assert!(FittedScoresV1.play(&bad, &ko, 1).is_err());
    }
}
