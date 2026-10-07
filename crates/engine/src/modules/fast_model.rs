//! The fast-model slot (`engine.fast-model`): a results model fitted from full-engine
//! results. Given the two teams as they kick off, it returns a final score and the goal
//! events of a 90-minute match without playing a tick.
//!
//! Nothing in the game uses it yet. The module sits in a private field of
//! [`ResolvedModules`], and [`resolve`] is the one way to reach it; a source-scan test fails
//! the build when anything outside the fit and check commands calls it, so no runtime path
//! can fall back to it in place of the full engine.
//!
//! The model: each side's goals have the mean
//! `exp(base + home * [home side] + attack * a + curve * a^2 + defence * e)`, where `a` is
//! the side's attack and `e` the other side's defence, each as (level − 10) / 2. A level is
//! read through the attribute contract, as play reads the players: each starter's mean
//! curve value over the stages of his role (the keeper's stages for the keeper in slot 0,
//! every other stage for an outfielder), the mean of those over the group, and the rating
//! whose curve value that is.
//! Both sides share one match factor, a gamma with shape `dispersion` and mean 1 that
//! multiplies both means, so the two scores rise and fall together as the full engine's do
//! (a bivariate negative binomial: each side alone is negative binomial with that
//! dispersion). The joint table over 0 to 15 goals a side then carries the Dixon-Coles
//! factor on 0-0, 1-0, 0-1 and 1-1 and a weight on every draw, and is normalised. A goal's
//! minute is drawn from 90 shares fitted from the full engine's goal minutes, and its scorer
//! from the side's six most advanced outfield players by finishing. The model plays
//! regulation time only.
//!
//! After the score, [`super::fast_events`] plays every other event kind the full engine
//! emits in a regulation match, each naming the player the full engine would name: both
//! sides' starters and benches come with the kick-off.

use serde::{Deserialize, Serialize};

use super::ResolvedModules;
use super::card::ModuleCard;
use super::fast_events::{self, EventFit};
use crate::contract::curve::f_inv;
use crate::contract::{ActionKind, STAGES, Stage, StageValues};
use crate::data::{Position, StoppageKind};
use crate::error::EngineError;
use crate::player::Player;
use crate::rng::EngineRng;
use crate::sim::{EngineEvent, MatchConfig};
use crate::team::PLAYERS_PER_TEAM;
use crate::validate::StreamRules;

/// Goals a side can score in the score table: 0 to 15.
pub const MAX_GOALS: usize = 16;
/// The minutes of regulation time a goal can fall in.
pub const MINUTES: usize = 90;
/// The fit file's layout version.
pub const FIT_VERSION: u32 = 2;
/// The fit file inside the content folder. `Content::load` never reads it, so it moves no
/// content hash.
pub const FIT_FILE: &str = "fast-model.json";
/// The name of the fitted model, as the slot file and the fit file name it.
pub const FITTED_SCORES: &str = "fitted-scores";

/// The level, on the 1 to 20 scale, a side's attack and defence are measured from.
pub const REFERENCE: f64 = 10.0;
/// Rating points per unit of a covariate: `(level − 10) / 2`.
pub const STEP: f64 = 2.0;

/// The actions only a keeper plays: a keeper's level reads their stages, an outfielder's
/// every other stage.
pub const KEEPER_ACTIONS: [ActionKind; 8] = [
    ActionKind::Hold,
    ActionKind::Save,
    ActionKind::Claim,
    ActionKind::OneOnOne,
    ActionKind::KeeperKick,
    ActionKind::KeeperThrow,
    ActionKind::Organise,
    ActionKind::Rush,
];

/// A player's mean curve value over the stages of his role: the keeper's stages for a
/// keeper, every other stage for an outfielder. The injury stage is left out: it reads
/// injury proneness, which rises as a player gets worse, so it is no part of his level.
pub fn role_curve_value(stages: &StageValues, keeper: bool) -> f64 {
    let (sum, count) = STAGES
        .iter()
        .zip(stages.f.iter())
        .filter(|((action, _), _)| {
            *action != ActionKind::Injury && KEEPER_ACTIONS.contains(action) == keeper
        })
        .fold((0.0, 0u32), |(s, n), (_, v)| (s + v, n + 1));
    sum / f64::from(count.max(1))
}

/// The seven fitted parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FastParams {
    /// Log goals of an away side whose attack and the other side's defence are at the
    /// reference.
    pub base: f64,
    /// Added to the log goals of the home side.
    pub home: f64,
    /// Log goals per unit of the side's attack, (level − 10) / 2.
    pub attack: f64,
    /// Log goals per squared unit of the side's attack.
    pub curve: f64,
    /// Log goals per unit of the other side's defence, (level − 10) / 2.
    pub defence: f64,
    /// The shape `k` of the shared match factor: each side's variance is
    /// `mean + mean^2 / k`.
    pub dispersion: f64,
    /// The Dixon-Coles low-score factor.
    pub rho: f64,
    /// Every draw's probability is multiplied by `1 + draw` before the table is normalised.
    pub draw: f64,
}

/// What the model plays from: its parameters, the share of goals in each minute, and the
/// rates of every other event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FastFit {
    pub params: FastParams,
    /// One share per minute of regulation time, 0 to 89; they sum to 1.
    pub minute_shares: Vec<f64>,
    pub events: EventFit,
}

impl FastFit {
    /// Refuses a fit the model cannot play: a dispersion that is not positive, a factor that
    /// is not finite, minute shares that are not 90 non-negative values summing to 1, or an
    /// event fit the model cannot play.
    pub fn check(&self) -> Result<(), EngineError> {
        let p = &self.params;
        let finite = [
            p.base,
            p.home,
            p.attack,
            p.curve,
            p.defence,
            p.dispersion,
            p.rho,
            p.draw,
        ]
        .iter()
        .all(|v| v.is_finite());
        if !finite || p.dispersion <= 0.0 || p.draw <= -1.0 {
            return Err(EngineError::InvalidConfig(
                "the fast-model parameters must be finite, with a positive dispersion and a \
                 draw weight above -1"
                    .into(),
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
        self.events.check()
    }
}

/// A player the fast model can name: the squad index and the weights its picks use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FastPlayer {
    pub squad: usize,
    pub keeper: bool,
    /// The weight of committing a foul: the shares of his tackle choose and execute stages
    /// (aggression, tackling), multiplied.
    pub foul: f64,
    /// The weight of scoring a goal and of taking a penalty: the share of his shot execute
    /// stage (finishing).
    pub attack: f64,
}

/// One side as it kicks off.
#[derive(Debug, Clone, PartialEq)]
pub struct Side {
    /// The player in each roster slot; slot 0 keeps goal.
    pub starters: [FastPlayer; PLAYERS_PER_TEAM],
    /// `true` for the slots of the six most advanced starters by their formation slot.
    pub advanced: [bool; PLAYERS_PER_TEAM],
    /// The substitutes in bench order.
    pub bench: Vec<FastPlayer>,
}

impl Side {
    /// A side of generated players: squad 0 to 10 in their slots with slots 5 to 10
    /// advanced, and squad 11 to 17 on the bench with a keeper first; every weight 1.
    pub fn generated() -> Self {
        let player = |squad: usize, keeper: bool| FastPlayer {
            squad,
            keeper,
            foul: 1.0,
            attack: 1.0,
        };
        Self {
            starters: std::array::from_fn(|s| player(s, s == 0)),
            advanced: std::array::from_fn(|s| s >= PLAYERS_PER_TEAM - ATTACKERS),
            bench: (11..18).map(|s| player(s, s == 11)).collect(),
        }
    }

    /// The squad index in each roster slot.
    pub fn lineup(&self) -> [usize; PLAYERS_PER_TEAM] {
        self.starters.map(|p| p.squad)
    }
}

/// The two teams as they kick off, home first.
#[derive(Debug, Clone, PartialEq)]
pub struct KickOff {
    /// The level of the eleven players a side starts with: the rating whose curve value is
    /// the mean of their role curve values ([`role_curve_value`]).
    pub strength: [f64; 2],
    /// The level of a side's six most advanced starters.
    pub attack: [f64; 2],
    /// The level of a side's other five starters, the keeper included.
    pub defence: [f64; 2],
    /// Each side's starters and bench.
    pub sides: [Side; 2],
}

/// Starters in a side's attack: the six most advanced by their formation slot.
pub const ATTACKERS: usize = 6;

impl KickOff {
    /// Two generated sides whose attack, defence and strength are all `strength`.
    pub fn even(strength: [f64; 2]) -> Self {
        Self {
            strength,
            attack: strength,
            defence: strength,
            sides: [Side::generated(), Side::generated()],
        }
    }
}

/// The event-stream rules a fast match keeps: the fit's substitution limits and added time,
/// and the line-ups of `kick_off`.
pub fn stream_rules(fit: &FastFit, kick_off: &KickOff) -> StreamRules {
    let rules = &fit.events.rules;
    StreamRules {
        substitutions: u32::from(rules.substitutions.limit),
        windows: u32::from(rules.substitutions.windows),
        half_time_exempt: rules.substitutions.exempt(StoppageKind::HalfTime),
        added_s: Some((rules.added_time.min_s, rules.added_time.max_s)),
        lineups: [kick_off.sides[0].lineup(), kick_off.sides[1].lineup()],
    }
}

/// The kick-off of the match `config` describes: the starting elevens the pre-match setup
/// picked, so the fast model and the full engine start from the same teams.
pub fn kick_off(config: &MatchConfig) -> KickOff {
    let curve = &config.tuning.contract.curve;
    // A group's level: the rating whose curve value is the mean of its role curve values.
    let level = |values: &mut dyn Iterator<Item = f64>| {
        let (sum, count) = values.fold((0.0, 0u32), |(s, n), v| (s + v, n + 1));
        if count == 0 {
            0.0
        } else {
            f_inv(sum / f64::from(count), curve)
        }
    };
    let mut out = KickOff::even([0.0; 2]);
    for team in 0..2 {
        let side = &config.teams[team];
        let player = |squad: usize, keeper: bool| {
            let s = &side.squad[squad].stages;
            FastPlayer {
                squad,
                keeper,
                foul: s.share(Stage::TACKLE_CHOOSE) * s.share(Stage::TACKLE_EXECUTE),
                attack: s.share(Stage::SHOT_EXECUTE),
            }
        };
        let mut starters: Vec<_> = config.players.iter().filter(|p| p.team == team).collect();
        let value = |p: &&Player| role_curve_value(&side.squad[p.squad].stages, p.slot == 0);
        out.strength[team] = level(&mut starters.iter().map(value));
        // Most advanced first: the slot furthest from the own goal line, then slot order.
        let depth = |slot: usize| config.teams[team].base_formation[slot].0;
        starters.sort_by(|a, b| {
            depth(b.slot)
                .total_cmp(&depth(a.slot))
                .then(a.slot.cmp(&b.slot))
        });
        let (front, back) = starters.split_at(ATTACKERS.min(starters.len()));
        out.attack[team] = level(&mut front.iter().map(value));
        out.defence[team] = level(&mut back.iter().map(value));
        let mut advanced = [false; PLAYERS_PER_TEAM];
        for p in front {
            advanced[p.slot] = true;
        }
        out.sides[team] = Side {
            starters: std::array::from_fn(|slot| player(side.lineup[slot], slot == 0)),
            advanced,
            bench: side
                .bench
                .iter()
                .map(|&s| player(s, side.squad[s].position == Position::GK))
                .collect(),
        };
    }
    out
}

/// A match the fast model played.
#[derive(Debug, Clone, PartialEq)]
pub struct FastMatch {
    /// The final score, home first.
    pub scores: [u32; 2],
    /// Every event of the match in tick order, from the kick-off to full time.
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

/// A side's covariates for its goals: intercept, home, its attack, its attack squared, and
/// the other side's defence, each measured as (level − 10) / 2.
pub fn covariates(kick_off: &KickOff, side: usize) -> [f64; 5] {
    let a = (kick_off.attack[side] - REFERENCE) / STEP;
    let e = (kick_off.defence[1 - side] - REFERENCE) / STEP;
    [1.0, if side == 0 { 1.0 } else { 0.0 }, a, a * a, e]
}

/// The mean goals of each side, home first.
pub fn means(params: &FastParams, kick_off: &KickOff) -> [f64; 2] {
    let beta = [
        params.base,
        params.home,
        params.attack,
        params.curve,
        params.defence,
    ];
    [0, 1].map(|side| {
        let x = covariates(kick_off, side);
        libm::exp(beta.iter().zip(&x).map(|(b, x)| b * x).sum())
    })
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

/// The bivariate negative binomial probabilities of `h`-`a` for `h`, `a` from 0 to 15: two
/// Poisson scores with means `lambda` and `mu` times one gamma factor of shape `k`.
pub fn shared_pmf(lambda: f64, mu: f64, k: f64) -> [[f64; MAX_GOALS]; MAX_GOALS] {
    let s = k + lambda + mu;
    let (ph, pa) = (lambda / s, mu / s);
    let mut table = [[0.0; MAX_GOALS]; MAX_GOALS];
    table[0][0] = libm::pow(k / s, k);
    for h in 0..MAX_GOALS {
        if h > 0 {
            let n = (h - 1) as f64;
            table[h][0] = table[h - 1][0] * (k + n) / (n + 1.0) * ph;
        }
        for a in 1..MAX_GOALS {
            let n = (h + a - 1) as f64;
            table[h][a] = table[h][a - 1] * (k + n) / a as f64 * pa;
        }
    }
    table
}

/// The normalised score table: row = home goals, column = away goals.
pub fn score_table(params: &FastParams, kick_off: &KickOff) -> [[f64; MAX_GOALS]; MAX_GOALS] {
    let [lambda, mu] = means(params, kick_off);
    let mut table = shared_pmf(lambda, mu, params.dispersion);
    let mut sum = 0.0;
    for (h, row) in table.iter_mut().enumerate() {
        for (a, cell) in row.iter_mut().enumerate() {
            *cell *= tau(h, a, lambda, mu, params.rho);
            if h == a {
                *cell *= 1.0 + params.draw;
            }
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
        let events = fast_events::play(&fit.events, kick_off, &goals, &mut rng);
        Ok(FastMatch { scores, events })
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
    purpose: "A results model fitted from full-engine results: a final score and every event \
              of a 90-minute match from the two teams at kick-off, without playing a tick.",
    inputs: "Each side's levels at kick-off (its starting eleven, its six most advanced \
             starters and the other five, each read through the attribute contract's stage \
             values on the 1 to 20 scale), its starters and bench with their foul and finishing weights from the \
             attribute contract's tackle and shot stages, and the fit file \
             content/fast-model.json.",
    outputs: "The final score and the events the full engine emits in a regulation match: \
              kick-offs, goals, fouls, offsides, cards, restarts, injuries, substitutions with \
              the manager's decision and its verdict, half-time and full time with added time.",
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
    use crate::modules::fast_events::FitRules;
    use crate::sim::EngineEventKind;

    fn fit() -> FastFit {
        FastFit {
            params: FastParams {
                base: -0.3,
                home: 0.05,
                attack: 1.2,
                curve: 0.1,
                defence: -0.2,
                dispersion: 6.0,
                rho: -0.08,
                draw: 0.3,
            },
            minute_shares: vec![1.0 / MINUTES as f64; MINUTES],
            events: EventFit::plain(FitRules::standard()),
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

    /// Each side of the shared table is the negative binomial with the same dispersion, and
    /// the two scores are positively correlated.
    #[test]
    fn the_shared_table_has_negative_binomial_sides_and_moves_them_together() {
        let table = shared_pmf(1.2, 0.8, 5.0);
        let home = goal_pmf(1.2, 5.0);
        for (h, row) in table.iter().enumerate() {
            assert!((row.iter().sum::<f64>() - home[h]).abs() < 1e-4, "{h}");
        }
        let (mut eh, mut ea, mut eha) = (0.0, 0.0, 0.0);
        for (h, row) in table.iter().enumerate() {
            for (a, p) in row.iter().enumerate() {
                eh += h as f64 * p;
                ea += a as f64 * p;
                eha += (h * a) as f64 * p;
            }
        }
        assert!(eha - eh * ea > 0.1, "covariance {}", eha - eh * ea);
    }

    #[test]
    fn the_table_is_normalised_and_the_factor_lifts_the_draws() {
        let ko = KickOff::even([10.0, 10.0]);
        let table = score_table(&fit().params, &ko);
        let sum: f64 = table.iter().flatten().sum();
        assert!((sum - 1.0).abs() < 1e-12);
        let mut plain = fit();
        plain.params.rho = 0.0;
        plain.params.draw = 0.0;
        let flat = score_table(&plain.params, &ko);
        assert!(table[0][0] > flat[0][0]);
        assert!(table[1][1] > flat[1][1]);
        assert!(table[1][0] < flat[1][0]);
    }

    #[test]
    fn a_stronger_side_scores_more() {
        let [h, a] = means(&fit().params, &KickOff::even([11.6, 10.0]));
        assert!(h > 2.0 * a, "{h} {a}");
    }

    #[test]
    fn a_seed_plays_the_same_match_and_the_goals_match_the_score() {
        let ko = KickOff::even([11.0, 10.0]);
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
        let ko = KickOff::even([10.0, 10.0]);
        let err = FastModelOff.play(&fit(), &ko, 1).unwrap_err().to_string();
        assert!(err.contains("the fast-model slot is off"), "{err}");
        let mut bad = fit();
        bad.minute_shares.pop();
        assert!(FittedScoresV1.play(&bad, &ko, 1).is_err());
        let mut bad = fit();
        bad.params.dispersion = 0.0;
        assert!(FittedScoresV1.play(&bad, &ko, 1).is_err());
        let mut bad = fit();
        bad.events.yellow.coefficients[2] = f64::NAN;
        assert!(FittedScoresV1.play(&bad, &ko, 1).is_err());
        let mut bad = fit();
        bad.events.substitutions.weights[0] = 1.0;
        assert!(FittedScoresV1.play(&bad, &ko, 1).is_err());
        let mut bad = fit();
        bad.events.fouls.minute_shares.pop();
        assert!(FittedScoresV1.play(&bad, &ko, 1).is_err());
    }

    /// The events never change the score: a seed plays the same score and goal minutes as
    /// the score draws alone give, and the events come after them on the generator.
    #[test]
    fn the_events_follow_the_score_on_the_generator() {
        let ko = KickOff::even([11.0, 10.0]);
        for seed in 0..200 {
            let m = FittedScoresV1.play(&fit(), &ko, seed).unwrap();
            let table = score_table(&fit().params, &ko);
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
            assert_eq!(m.scores, scores, "seed {seed}");
        }
    }
}
