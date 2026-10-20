//! The restart wait table (`content/restart-waits.json`): how long each kind of stoppage
//! waits beyond its visible steps. A stoppage lasts its visible steps (the taker walks to the
//! ball, team-mates set up, a wall forms, players walk back) plus a drawn extra, the unseen
//! part such as fetching the ball; the ball waits where it left play during the extra.
//!
//! Each kind holds 21 quantiles of the extra in seconds (0, 5, ..., 100 percent), and a draw
//! reads them linearly between two quantiles. A committed command writes the file from real
//! restart waits minus the engine's own measured visible steps (`engine-cli restart-waits`);
//! no ball-in-play figure is an input. The file also holds the hard limit on the visible
//! steps, the factor a team leading late takes longer by, and the quick-restart rule.
//!
//! The file is optional: content without it plays the fixed delays of the tuning file
//! (`restart_delay_s`), with no fetch and no quick restart, as before the table existed.

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::EngineError;
use crate::data::rules::StoppageKind;
use crate::data::{Loaded, load_json_bytes};

/// Schema version this build reads.
pub const RESTART_WAITS_VERSION: u32 = 1;
/// The quantiles each kind holds: 0, 5, ..., 100 percent.
pub const QUANTILES: usize = 21;
/// The longest extra a quantile may hold, in seconds.
pub const MAX_EXTRA_S: f64 = 600.0;

/// A stoppage kind as the wait table names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitKind {
    ThrowIn,
    GoalKick,
    Corner,
    FreeKick,
    Penalty,
    KickOff,
    DropBall,
}

impl WaitKind {
    /// Every kind, in the file's order.
    pub const ALL: [WaitKind; 7] = [
        WaitKind::ThrowIn,
        WaitKind::GoalKick,
        WaitKind::Corner,
        WaitKind::FreeKick,
        WaitKind::Penalty,
        WaitKind::KickOff,
        WaitKind::DropBall,
    ];

    /// The kind of a restart: a dropped ball after an injury is `DropBall`; a goal and
    /// half-time restart with a kick-off.
    pub fn of(kind: StoppageKind) -> Self {
        match kind {
            StoppageKind::ThrowIn => WaitKind::ThrowIn,
            StoppageKind::GoalKick => WaitKind::GoalKick,
            StoppageKind::Corner => WaitKind::Corner,
            StoppageKind::FreeKick => WaitKind::FreeKick,
            StoppageKind::Penalty => WaitKind::Penalty,
            StoppageKind::Injury => WaitKind::DropBall,
            StoppageKind::KickOff | StoppageKind::Goal | StoppageKind::HalfTime => {
                WaitKind::KickOff
            }
        }
    }

    /// The kind as the file writes it.
    pub fn code(self) -> &'static str {
        match self {
            WaitKind::ThrowIn => "throw_in",
            WaitKind::GoalKick => "goal_kick",
            WaitKind::Corner => "corner",
            WaitKind::FreeKick => "free_kick",
            WaitKind::Penalty => "penalty",
            WaitKind::KickOff => "kick_off",
            WaitKind::DropBall => "drop_ball",
        }
    }
}

/// One kind's wait: the extra's quantiles and the hard limit on the visible steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct KindWait {
    /// The extra beyond the visible steps at 0, 5, ..., 100 percent, in seconds: finite, at
    /// least 0, at most 600, never falling.
    #[garde(custom(check_quantiles))]
    pub extra_s: [f64; QUANTILES],
    /// The longest the visible steps may take, in seconds: a restart whose players are not
    /// set by then is taken anyway and logged (`rules.dead_ball_stalled`).
    #[garde(range(min = 10.0, max = 300.0))]
    pub steps_limit_s: f64,
}

/// The wait of every kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Kinds {
    #[garde(dive)]
    pub throw_in: KindWait,
    #[garde(dive)]
    pub goal_kick: KindWait,
    #[garde(dive)]
    pub corner: KindWait,
    #[garde(dive)]
    pub free_kick: KindWait,
    #[garde(dive)]
    pub penalty: KindWait,
    #[garde(dive)]
    pub kick_off: KindWait,
    #[garde(dive)]
    pub drop_ball: KindWait,
}

impl Kinds {
    /// The wait of `kind`.
    pub fn get(&self, kind: WaitKind) -> &KindWait {
        match kind {
            WaitKind::ThrowIn => &self.throw_in,
            WaitKind::GoalKick => &self.goal_kick,
            WaitKind::Corner => &self.corner,
            WaitKind::FreeKick => &self.free_kick,
            WaitKind::Penalty => &self.penalty,
            WaitKind::KickOff => &self.kick_off,
            WaitKind::DropBall => &self.drop_ball,
        }
    }

    /// The wait of `kind`, to write.
    pub fn get_mut(&mut self, kind: WaitKind) -> &mut KindWait {
        match kind {
            WaitKind::ThrowIn => &mut self.throw_in,
            WaitKind::GoalKick => &mut self.goal_kick,
            WaitKind::Corner => &mut self.corner,
            WaitKind::FreeKick => &mut self.free_kick,
            WaitKind::Penalty => &mut self.penalty,
            WaitKind::KickOff => &mut self.kick_off,
            WaitKind::DropBall => &mut self.drop_ball,
        }
    }
}

/// The quick-restart rule: at a kind in `kinds`, the taker restarts at once (no drawn extra)
/// when a team-mate is free and the opponents are out of shape, with the chance
/// `base + trailing · [his team trails] + skill_slope · (his decisions and flair share − 0.5)`.
/// A team with time wasting on never restarts quickly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Quick {
    /// The kinds a taker may restart quickly.
    #[garde(length(max = 7), custom(check_kinds))]
    pub kinds: Vec<WaitKind>,
    #[garde(range(min = 0.0, max = 1.0))]
    pub base: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub trailing: f64,
    #[garde(range(min = 0.0, max = 2.0))]
    pub skill_slope: f64,
    /// A team-mate counts as free within this distance of the spot, in metres...
    #[garde(range(min = 0.0, max = 60.0))]
    pub free_mate_m: f64,
    /// ...with no opponent within this distance of him.
    #[garde(range(min = 0.0, max = 30.0))]
    pub free_space_m: f64,
}

impl Quick {
    /// The chance of a quick restart for a taker whose decisions and flair share is `share`
    /// (0 to 1), when his team trails or not, before the free team-mate and the opponents'
    /// shape are read.
    pub fn chance(&self, trailing: bool, share: f64) -> f64 {
        let lift = if trailing { self.trailing } else { 0.0 };
        (self.base + lift + self.skill_slope * (share - 0.5)).clamp(0.0, 1.0)
    }
}

/// The restart wait table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RestartWaits {
    #[garde(skip)]
    pub schema_version: u32,
    /// Where the numbers came from: the real-data file, the rows per kind, the seeds and the
    /// engine version of the command that wrote the file.
    #[garde(length(max = 64))]
    pub source: Vec<String>,
    #[garde(dive)]
    pub kinds: Kinds,
    /// A team leading in the late window (the last minutes of regulation, the pace tuning's
    /// `late_minutes`, or the closing minutes of an extra-time period) takes this many times
    /// the drawn extra.
    #[garde(range(min = 1.0, max = 5.0))]
    pub late_lead_factor: f64,
    #[garde(dive, custom(check_quick))]
    pub quick: Quick,
}

impl RestartWaits {
    /// The extra of a restart of `kind` for the uniform draw `u` in `[0, 1)`, in seconds:
    /// linear between the two quantiles around `u`.
    pub fn draw(&self, kind: StoppageKind, u: f64) -> f64 {
        quantile(&self.kinds.get(WaitKind::of(kind)).extra_s, u)
    }

    /// The hard limit on the visible steps of a restart of `kind`, in seconds.
    pub fn steps_limit_s(&self, kind: StoppageKind) -> f64 {
        self.kinds.get(WaitKind::of(kind)).steps_limit_s
    }

    /// `true` when a taker may restart a stoppage of `kind` quickly.
    pub fn quick_kind(&self, kind: StoppageKind) -> bool {
        self.quick.kinds.contains(&WaitKind::of(kind))
    }

    /// A table with every extra 0 and no quick restart, the other values as `like` holds them:
    /// the table the wait-table command plays with to measure the visible steps alone.
    pub fn zero(like: &RestartWaits) -> RestartWaits {
        let mut zero = like.clone();
        for kind in WaitKind::ALL {
            zero.kinds.get_mut(kind).extra_s = [0.0; QUANTILES];
        }
        zero.quick.kinds.clear();
        zero
    }

    /// The starting values of everything the wait-table command does not measure: a steps
    /// limit of 90 s for every kind, the late-lead factor 1.7, and quick restarts at throw-ins,
    /// free kicks and goal kicks.
    pub fn defaults() -> RestartWaits {
        let kind = KindWait {
            extra_s: [0.0; QUANTILES],
            steps_limit_s: 90.0,
        };
        RestartWaits {
            schema_version: RESTART_WAITS_VERSION,
            source: Vec::new(),
            kinds: Kinds {
                throw_in: kind.clone(),
                goal_kick: kind.clone(),
                corner: kind.clone(),
                free_kick: kind.clone(),
                penalty: kind.clone(),
                kick_off: kind.clone(),
                drop_ball: kind,
            },
            late_lead_factor: 1.7,
            quick: Quick {
                kinds: vec![WaitKind::ThrowIn, WaitKind::FreeKick, WaitKind::GoalKick],
                base: 0.10,
                trailing: 0.15,
                skill_slope: 0.20,
                free_mate_m: 20.0,
                free_space_m: 5.0,
            },
        }
    }
}

/// The value of the 21 quantiles `q` at the uniform draw `u`: linear between the two quantiles
/// around it. A draw outside `[0, 1]` reads the nearer end.
pub fn quantile(q: &[f64; QUANTILES], u: f64) -> f64 {
    let at = u.clamp(0.0, 1.0) * (QUANTILES - 1) as f64;
    // `at` lies in 0 to 20, so the cast is exact.
    let i = (at.floor() as usize).min(QUANTILES - 2);
    let frac = at - i as f64;
    q[i] + (q[i + 1] - q[i]) * frac
}

fn check_quantiles(q: &[f64; QUANTILES], _ctx: &()) -> garde::Result {
    for (i, v) in q.iter().enumerate() {
        if !v.is_finite() || *v < 0.0 || *v > MAX_EXTRA_S {
            return Err(garde::Error::new(format!(
                "quantile {i} is {v}; allowed 0 to {MAX_EXTRA_S} s"
            )));
        }
        if i > 0 && *v < q[i - 1] {
            return Err(garde::Error::new(format!(
                "quantile {i} ({v}) falls below quantile {} ({})",
                i - 1,
                q[i - 1]
            )));
        }
    }
    Ok(())
}

fn check_kinds(kinds: &[WaitKind], _ctx: &()) -> garde::Result {
    for (i, k) in kinds.iter().enumerate() {
        if kinds[..i].contains(k) {
            return Err(garde::Error::new(format!("{} appears twice", k.code())));
        }
    }
    Ok(())
}

/// The chance stays inside 0 to 1 at both ends of the skill share, trailing or not.
fn check_quick(q: &Quick, _ctx: &()) -> garde::Result {
    let low = q.base - q.skill_slope / 2.0;
    let high = q.base + q.trailing + q.skill_slope / 2.0;
    if low < 0.0 || high > 1.0 {
        return Err(garde::Error::new(format!(
            "the quick-restart chance runs from {low} to {high}; it must stay inside 0 to 1"
        )));
    }
    Ok(())
}

/// Loads and checks the wait table's bytes. A refusal names the field and the rule.
pub fn load_restart_waits_bytes(
    bytes: &[u8],
    shown: &str,
) -> Result<Loaded<RestartWaits>, EngineError> {
    load_json_bytes::<RestartWaits>("restart_waits", bytes, shown, RESTART_WAITS_VERSION, &())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> RestartWaits {
        let mut t = RestartWaits::defaults();
        t.kinds.throw_in.extra_s = std::array::from_fn(|i| i as f64);
        t
    }

    fn refusal(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut v = serde_json::to_value(table()).unwrap();
        edit(&mut v);
        load_restart_waits_bytes(&serde_json::to_vec(&v).unwrap(), "restart-waits.json")
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn a_draw_reads_linearly_between_two_quantiles() {
        let t = table();
        assert_eq!(t.draw(StoppageKind::ThrowIn, 0.0), 0.0);
        assert_eq!(t.draw(StoppageKind::ThrowIn, 1.0), 20.0);
        assert!((t.draw(StoppageKind::ThrowIn, 0.5) - 10.0).abs() < 1e-12);
        assert!((t.draw(StoppageKind::ThrowIn, 0.525) - 10.5).abs() < 1e-9);
        assert_eq!(t.draw(StoppageKind::Corner, 0.7), 0.0);
    }

    #[test]
    fn the_table_round_trips_and_loads() {
        let bytes = serde_json::to_vec(&table()).unwrap();
        let loaded = load_restart_waits_bytes(&bytes, "restart-waits.json").unwrap();
        assert_eq!(loaded.value, table());
    }

    #[test]
    fn a_falling_quantile_is_refused() {
        let text = refusal(|v| v["kinds"]["throw_in"]["extra_s"][3] = 0.5.into());
        assert!(text.contains("quantile 3"), "{text}");
    }

    #[test]
    fn a_negative_or_huge_quantile_is_refused() {
        let text = refusal(|v| v["kinds"]["corner"]["extra_s"][0] = (-1.0).into());
        assert!(text.contains("quantile 0"), "{text}");
        let text = refusal(|v| v["kinds"]["corner"]["extra_s"][20] = 601.0.into());
        assert!(text.contains("quantile 20"), "{text}");
    }

    #[test]
    fn a_steps_limit_or_factor_out_of_range_is_refused() {
        let text = refusal(|v| v["kinds"]["penalty"]["steps_limit_s"] = 5.0.into());
        assert!(text.contains("steps_limit_s"), "{text}");
        let text = refusal(|v| v["late_lead_factor"] = 0.9.into());
        assert!(text.contains("late_lead_factor"), "{text}");
    }

    #[test]
    fn a_quick_chance_outside_zero_to_one_is_refused() {
        let text = refusal(|v| v["quick"]["base"] = 0.05.into());
        assert!(text.contains("quick-restart chance"), "{text}");
        let text = refusal(|v| v["quick"]["trailing"] = 0.9.into());
        assert!(text.contains("quick-restart chance"), "{text}");
    }

    #[test]
    fn an_unknown_key_is_refused() {
        let text = refusal(|v| v["ball_in_play"] = 1.into());
        assert!(text.contains("unknown field"), "{text}");
    }

    #[test]
    fn the_zero_table_keeps_the_limits_and_drops_every_extra_and_quick_restart() {
        let zero = RestartWaits::zero(&table());
        assert_eq!(zero.draw(StoppageKind::ThrowIn, 0.99), 0.0);
        assert!(zero.quick.kinds.is_empty());
        assert_eq!(zero.kinds.throw_in.steps_limit_s, 90.0);
        assert_eq!(zero.late_lead_factor, 1.7);
    }

    #[test]
    fn the_chance_rises_when_trailing_and_with_the_skill_share() {
        let q = RestartWaits::defaults().quick;
        assert!(q.chance(true, 0.5) > q.chance(false, 0.5));
        assert!(q.chance(false, 0.9) > q.chance(false, 0.1));
        assert!((q.chance(false, 0.5) - 0.10).abs() < 1e-12);
    }
}
