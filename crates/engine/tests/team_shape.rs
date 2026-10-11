//! Team shape in seeded matches: a duty or a mentality moves the shape the way it says, and
//! in build-up from the back the centre-backs split, a midfielder drops and the full-backs
//! push on. The scripted fixtures of each mechanism are the engine's own tests
//! (`sim/shape_tests.rs`).

mod common;

use engine::math::DVec2;
use engine::{MatchConfig, Simulation, Tactics};

/// Home slots of the 4-4-2: the left-back, the centre-backs, the right-back and the central
/// midfielders.
const LEFT_BACK: usize = 1;
const CENTRE_BACKS: [usize; 2] = [2, 3];
const FULL_BACKS: [usize; 2] = [1, 4];
const MIDFIELDERS: [usize; 2] = [6, 7];

/// A seed-42 match of `minutes` with no injuries, the home side's tactics set by `edit`
/// from the defaults.
fn seeded(
    minutes: u32,
    edit: impl FnOnce(&mut Tactics, &engine::data::tactics::TacticsSchema),
) -> MatchConfig {
    seeded_on(common::content(), minutes, edit)
}

/// [`seeded`] on `content`.
fn seeded_on(
    content: engine::Content,
    minutes: u32,
    edit: impl FnOnce(&mut Tactics, &engine::data::tactics::TacticsSchema),
) -> MatchConfig {
    let [a, b] = common::default_teams(&content);
    let mut config = MatchConfig::new(common::SEED, minutes, &content, [&a, &b]).unwrap();
    config.tuning.injury_per_minute = 0.0;
    config.tuning.injury_per_tackle = 0.0;
    let mut tactics = Tactics::defaults(&content.tactics);
    edit(&mut tactics, &content.tactics);
    config.with_tactics(0, tactics)
}

/// Plays `config` and calls `each` on every live tick with the match.
fn live_ticks(config: MatchConfig, mut each: impl FnMut(&Simulation)) {
    let mut sim = Simulation::new(config).unwrap();
    while !sim.is_over() {
        sim.step();
        if sim.dead_ball().is_none() && !sim.is_over() {
            each(&sim);
        }
    }
}

/// The home team's attack direction now.
fn home_attack(sim: &Simulation) -> f64 {
    sim.team_timeline().last().unwrap().1[0].attack_x
}

/// Mean depth up the pitch, in the home team's attack direction, of the home `slots` over
/// the live ticks of `config`.
fn mean_depth(config: MatchConfig, slots: &[usize]) -> f64 {
    let (mut sum, mut n) = (0.0, 0.0);
    live_ticks(config, |sim| {
        let ax = home_attack(sim);
        for &s in slots {
            sum += sim.players()[s].pos.x * ax;
            n += 1.0;
        }
    });
    sum / n
}

/// Shape AC-3: over the same 20 seeded minutes, an attacking duty takes the left-back at
/// least 2 m further up on average than a defending one, and an attacking mentality takes the
/// home outfield at least 2 m further up than a balanced one. 2 m is a quarter of the 8 m
/// the content moves a slot between those duties or mentalities.
#[test]
fn a_duty_and_a_mentality_move_the_shape_the_way_they_say() {
    const SMALLEST_SHIFT_M: f64 = 2.0;
    let duty = |name: &str| {
        seeded(20, |t, schema| {
            t.roles[LEFT_BACK].duty = schema.duty_index(name).unwrap() as u8;
        })
    };
    let defend = mean_depth(duty("defend"), &[LEFT_BACK]);
    let attack = mean_depth(duty("attack"), &[LEFT_BACK]);
    assert!(
        attack - defend >= SMALLEST_SHIFT_M,
        "left-back: defend {defend:.2} m, attack {attack:.2} m"
    );
    let outfield: Vec<usize> = (1..11).collect();
    let mentality = |name: &str| {
        seeded(20, |t, schema| {
            t.mentality = schema.mentality_index(name).unwrap() as u8;
        })
    };
    let balanced = mean_depth(mentality("balanced"), &outfield);
    let attacking = mean_depth(mentality("attacking"), &outfield);
    assert!(
        attacking - balanced >= SMALLEST_SHIFT_M,
        "home outfield: balanced {balanced:.2} m, attacking {attacking:.2} m"
    );
}

/// The charter's build-up from the back: over a seeded 45 minutes, on the ticks the home
/// keeper or a home back-line player holds the ball in the home half, the centre-back without
/// the ball aims at least 2 m wider than the centre-backs aim out of possession, a central
/// midfielder's target lies at least 4 m deeper than his stand on at least 30 percent of
/// those ticks, and the full-backs stand at least 5 m further up than the centre-backs. The
/// split reads the targets, the spots the stand options set: a build-up spell lasts under a
/// second on most ticks, too short for the centre-backs to reach them. The full-backs push on
/// by their stands, so the match plays with the team stands on.
#[test]
fn in_build_up_the_centre_backs_split_a_midfielder_drops_and_the_full_backs_push_on() {
    let config = seeded_on(common::content_with_stands(), 45, |_, _| {});
    let tuning = config.tuning.clone();
    let mut build_up = 0.0;
    let mut out_of_possession = 0.0;
    let (mut cb_wide_in, mut cb_wide_out) = (0.0, 0.0);
    let mut dropped = 0.0;
    let (mut fb_depth, mut cb_depth) = (0.0, 0.0);
    live_ticks(config, |sim| {
        let team = &sim.team_timeline().last().unwrap().1[0];
        let ax = team.attack_x;
        let players = sim.players();
        let record = sim.record();
        let ball = DVec2::new(f64::from(record.ball[0]), f64::from(record.ball[1]));
        let back_line = team.back_line();
        // The mean distance from the centre line the centre-backs other than the carrier aim
        // at.
        let aim_across = |carrier: Option<usize>| {
            let aims: Vec<f64> = CENTRE_BACKS
                .iter()
                .filter(|&&s| Some(s) != carrier)
                .map(|&s| players[s].target.y.abs())
                .collect();
            aims.iter().sum::<f64>() / aims.len() as f64
        };
        let mean_up = |slots: &[usize]| {
            slots.iter().map(|&s| players[s].pos.x * ax).sum::<f64>() / slots.len() as f64
        };
        match (sim.possession_phase(), sim.carrier()) {
            (Some(0), Some(c))
                if c < 11
                    && (c == sim.keeper(0) || back_line.contains(&c))
                    && ball.x * ax < 0.0 =>
            {
                build_up += 1.0;
                cb_wide_in += aim_across(Some(c));
                fb_depth += mean_up(&FULL_BACKS);
                cb_depth += mean_up(&CENTRE_BACKS);
                let drops = MIDFIELDERS.iter().any(|&s| {
                    let stand = team.anchor(s, ball, true, &tuning);
                    (stand.x - players[s].target.x) * ax >= 4.0
                });
                if drops {
                    dropped += 1.0;
                }
            }
            (Some(1), _) => {
                out_of_possession += 1.0;
                cb_wide_out += aim_across(None);
            }
            _ => {}
        }
    });
    assert!(build_up >= 200.0, "only {build_up} build-up ticks");
    let split = cb_wide_in / build_up - cb_wide_out / out_of_possession;
    assert!(split >= 2.0, "the centre-backs split by {split:.2} m");
    let share = dropped / build_up;
    assert!(
        share >= 0.30,
        "a midfielder drops on {:.0} percent",
        share * 100.0
    );
    let push = (fb_depth - cb_depth) / build_up;
    assert!(
        push >= 5.0,
        "the full-backs stand {push:.2} m ahead of the centre-backs"
    );
}
