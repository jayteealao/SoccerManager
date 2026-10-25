//! The realism measures of one match as the compact rows keep them: their sizes, their
//! names and the per-match records. The calibrate run writes them into its rows and the
//! report reads them back into the bands, so both depend on this module and neither on the
//! other for the row layout.

use crate::report::tracked_shape::{TRACKED_FIELDS, TrackedValues};

/// Shot distance bins: 1 m each from 0 m to 40 m, then 40 m and over.
pub const SHOT_BINS: usize = 41;
/// Pass cells: ground pass, long pass or chip, cross, each under 10 m, 10 to 25 m and over
/// 25 m, then keeper throw, keeper kick, and a pass without a plan.
pub const PASS_CELL_NAMES: [&str; 12] = [
    "ground_lt10",
    "ground_10_25",
    "ground_gt25",
    "chip_lt10",
    "chip_10_25",
    "chip_gt25",
    "cross_lt10",
    "cross_10_25",
    "cross_gt25",
    "keeper_throw",
    "keeper_kick",
    "unknown",
];
pub const PASS_CELLS: usize = PASS_CELL_NAMES.len();
/// The phases a goal can take.
pub const PHASES: [&str; 7] = [
    "open_play",
    "corner",
    "free_kick",
    "penalty",
    "throw_in",
    "kick_off",
    "goal_kick",
];
/// The thirds of the pitch, from a side's own goal line.
pub const THIRDS: [&str; 3] = ["own", "middle", "final"];
/// Chain length bins in passes: 0 to 14, then 15 and over.
pub const CHAIN_PASS_BINS: usize = 16;
/// Chain length bins in seconds: 2 s each up to 60 s, then 60 s and over.
pub const CHAIN_SECOND_BINS: usize = 31;
/// Physical record slots per side: eleven starters and five substitutes.
pub const PLAYER_SLOTS: usize = 16;
/// The speed zones of the real player file, in m/s: walk under 2, jog 2 to 4, run 4 to 5.5,
/// high intensity 5.5 to 7, sprint 7 and over.
pub const ZONES: [&str; 5] = ["walk", "jog", "run", "hi", "sprint"];
pub const SIDES: [&str; 2] = ["home", "away"];
/// The team-shape groups, by the position of a player's formation slot: centre-backs,
/// full-backs, central midfielders (holding, central and attacking), wingers, strikers.
pub const SHAPE_GROUPS: [&str; 5] = ["cb", "fb", "cm", "w", "st"];
/// The team-shape sums kept per group, in order (the engine's `measures::shape`).
pub const SHAPE_GROUP_FIELDS: [&str; 26] = [
    "depth_in_sum",
    "depth_in_n",
    "depth_out_sum",
    "depth_out_n",
    "width_near_in_sum",
    "width_near_in_n",
    "width_near_out_sum",
    "width_near_out_n",
    "width_far_in_sum",
    "width_far_in_n",
    "width_far_out_sum",
    "width_far_out_n",
    "pace_stand",
    "pace_walk",
    "pace_jog",
    "pace_run",
    "pace_hs",
    "pace_sprint",
    "speed_near_sum",
    "speed_near_n",
    "speed_far_sum",
    "speed_far_n",
    "sprints_fwd",
    "sprints_back",
    "sprints_lat",
    "live_ticks",
];
/// The team-shape sums kept per team, in order.
pub const SHAPE_TEAM_FIELDS: [&str; 27] = [
    "slide_x_in_sxy",
    "slide_x_in_sxx",
    "slide_x_out_sxy",
    "slide_x_out_sxx",
    "slide_y_in_sxy",
    "slide_y_in_sxx",
    "slide_y_out_sxy",
    "slide_y_out_sxx",
    "far_1s_30_45_pq",
    "far_1s_30_45_qq",
    "far_1s_gt45_pq",
    "far_1s_gt45_qq",
    "far_10s_30_45_pq",
    "far_10s_30_45_qq",
    "far_10s_gt45_pq",
    "far_10s_gt45_qq",
    "calm_ticks",
    "dead_ticks",
    "stops_throw_in",
    "stops_free_kick",
    "stops_goal_kick",
    "stops_corner",
    "stops_kick_off",
    "stops_penalty",
    "links_top2",
    "links_back",
    "links_total",
];
/// Team-shape sums per side: every group's, then the team's.
pub const SHAPE_FIELDS: usize =
    SHAPE_GROUPS.len() * SHAPE_GROUP_FIELDS.len() + SHAPE_TEAM_FIELDS.len();

/// The name of team-shape sum `i` of a side: `<group>.<field>`, then the team fields.
pub fn shape_name(i: usize) -> String {
    let per = SHAPE_GROUP_FIELDS.len();
    match SHAPE_GROUPS.get(i / per) {
        Some(g) => format!("{g}.{}", SHAPE_GROUP_FIELDS[i % per]),
        None => SHAPE_TEAM_FIELDS[i - SHAPE_GROUPS.len() * per].to_string(),
    }
}

/// The place of the team-shape sum `name` (`<group>.<field>` or a team field).
pub fn shape_place(name: &str) -> Option<usize> {
    let per = SHAPE_GROUP_FIELDS.len();
    match name.split_once('.') {
        Some((g, f)) => {
            let g = SHAPE_GROUPS.iter().position(|x| *x == g)?;
            Some(g * per + SHAPE_GROUP_FIELDS.iter().position(|x| *x == f)?)
        }
        None => SHAPE_TEAM_FIELDS
            .iter()
            .position(|x| *x == name)
            .map(|t| SHAPE_GROUPS.len() * per + t),
    }
}

// The engine's measures and the rows agree on every size.
#[cfg(feature = "measures")]
const _: () = assert!(
    SHOT_BINS == engine::measures::SHOT_BINS
        && PASS_CELLS == engine::measures::PASS_CELLS
        && PHASES.len() == engine::measures::PHASE_COUNT
        && CHAIN_PASS_BINS == engine::measures::CHAIN_PASS_BINS
        && CHAIN_SECOND_BINS == engine::measures::CHAIN_SECOND_BINS
        && SHAPE_FIELDS == engine::measures::shape::SHAPE_FIELDS
);

/// The realism measures of one match, as the rows keep them. Per-side arrays are home
/// first; the fields are those of the engine's measures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measures {
    /// `true` when a measures build played the match; every other field is 0 otherwise.
    pub present: bool,
    pub shot_bins: [[u32; SHOT_BINS]; 2],
    pub shot_distance_m: [f64; 2],
    pub pass_attempts: [[u32; PASS_CELLS]; 2],
    pub pass_completed: [[u32; PASS_CELLS]; 2],
    pub goals_by_phase: [[u32; PHASES.len()]; 2],
    pub third_ticks: [[u32; 3]; 2],
    pub chains: [u32; 2],
    pub chain_pass_bins: [[u32; CHAIN_PASS_BINS]; 2],
    pub chain_second_bins: [[u32; CHAIN_SECOND_BINS]; 2],
    pub chain_passes: [u32; 2],
    pub chain_ticks: [u64; 2],
}

impl Default for Measures {
    fn default() -> Self {
        Self {
            present: false,
            shot_bins: [[0; SHOT_BINS]; 2],
            shot_distance_m: [0.0; 2],
            pass_attempts: [[0; PASS_CELLS]; 2],
            pass_completed: [[0; PASS_CELLS]; 2],
            goals_by_phase: [[0; PHASES.len()]; 2],
            third_ticks: [[0; 3]; 2],
            chains: [0; 2],
            chain_pass_bins: [[0; CHAIN_PASS_BINS]; 2],
            chain_second_bins: [[0; CHAIN_SECOND_BINS]; 2],
            chain_passes: [0; 2],
            chain_ticks: [0; 2],
        }
    }
}

#[cfg(feature = "measures")]
impl From<&engine::measures::MatchMeasures> for Measures {
    fn from(m: &engine::measures::MatchMeasures) -> Self {
        Self {
            present: true,
            shot_bins: m.shot_bins,
            shot_distance_m: m.shot_distance_m,
            pass_attempts: m.pass_attempts,
            pass_completed: m.pass_completed,
            goals_by_phase: m.goals_by_phase,
            third_ticks: m.third_ticks,
            chains: m.chains,
            chain_pass_bins: m.chain_pass_bins,
            chain_second_bins: m.chain_second_bins,
            chain_passes: m.chain_passes,
            chain_ticks: m.chain_ticks,
        }
    }
}

/// One player's running in one match.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerRow {
    /// His place in his team file.
    pub squad: u8,
    /// Ticks on the pitch, the ball in play or not: his minutes played.
    pub ticks_on: u32,
    /// Metres covered: his speed times the tick, over every tick on the pitch.
    pub distance_m: f64,
    /// Ball-in-play ticks in each speed zone, in [`ZONES`] order.
    pub zone_ticks: [u32; 5],
    /// His highest speed, in m/s.
    pub peak_mps: f64,
}

/// A match's realism measures and each player's running, as a measures build records them.
/// A row holds one only when the match carries the measures, so a row of a build without the
/// feature keeps none in memory.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureBlock {
    /// The realism measures.
    pub measures: Measures,
    /// Each player's running, by side and slot in order of first appearance.
    pub physical: [[PlayerRow; PLAYER_SLOTS]; 2],
    /// Players of a side beyond [`PLAYER_SLOTS`], whose running no slot holds.
    pub physical_overflow: [u32; 2],
    /// The team-shape sums, per side, in [`shape_name`] order.
    pub shape: [[f64; SHAPE_FIELDS]; 2],
    /// The shared shape report's figures, per side, in
    /// [`crate::report::tracked_shape::field_names`] order.
    pub tracked: TrackedValues,
}

impl Default for MeasureBlock {
    fn default() -> Self {
        Self {
            measures: Measures::default(),
            physical: [[PlayerRow::default(); PLAYER_SLOTS]; 2],
            physical_overflow: [0; 2],
            shape: [[0.0; SHAPE_FIELDS]; 2],
            tracked: [[0.0; TRACKED_FIELDS]; 2],
        }
    }
}

#[cfg(all(test, feature = "measures"))]
mod tests {
    /// The rows name the team-shape sums as the engine keeps them.
    #[test]
    fn the_shape_names_are_the_engines() {
        let engine = engine::measures::shape::field_names();
        let rows: Vec<String> = (0..super::SHAPE_FIELDS).map(super::shape_name).collect();
        assert_eq!(rows, engine);
        for (i, name) in rows.iter().enumerate() {
            assert_eq!(super::shape_place(name), Some(i), "{name}");
        }
    }
}
