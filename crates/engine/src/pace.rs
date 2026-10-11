//! The pace selector: every movement target carries a job, the job an urgency grade, and the
//! grade one of six pace levels by the player's work rate, his determination and
//! professionalism, his energy, the game state and the team's instructions. Steering arrives
//! at the chosen level's speed.
//!
//! - A job is URGENT (may sprint), NORMAL (jog or run) or CALM (stand or walk).
//! - Raised tempo or pressing, or trailing late, lifts pressing and support jobs one step;
//!   leading late with time wasting drops NORMAL jobs one step while the ball is dead. In open
//!   play both teams keep their grades. An URGENT job never drops.
//! - The levels are stand, walk, jog, run, high speed and sprint; each level's speed is its
//!   upper limit, and a sprint is the player's own top speed.
//! - Work rate sets how often an URGENT job gets a sprint (one draw when the job starts), never
//!   the speed of a level.
//! - A tired player saves energy by his determination and professionalism: a high one keeps
//!   his sprints and moves a step slower at every other level, down to a walk while he is
//!   away from his target; a low one skips the less urgent sprints first and keeps his other
//!   levels.

use crate::tuning::PaceTuning;

/// What a player's target is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Job {
    /// The carrier's own move with the ball.
    Carry = 0,
    /// A chosen presser of the carrier.
    Press = 1,
    /// A recovery run while goal-side cover is missing.
    Recover = 2,
    /// The chaser who reaches a loose ball first, or a keeper who comes for it.
    LooseBall = 3,
    /// A timed run in behind (none is made yet).
    TimedRun = 4,
    /// The other team's chaser of a loose ball.
    Chase = 5,
    /// A player whose stand option offers a pass.
    Offer = 6,
    /// A return to a stand that is far away.
    Return = 7,
    /// A player in shape near the ball, following the slide.
    Follow = 8,
    /// A player in shape far from the ball, or in a concentration lapse.
    Hold = 9,
    /// A player with no job at a dead ball (none is set yet).
    Wait = 10,
}

impl Job {
    /// Every job in code order.
    pub const ALL: [Job; 11] = [
        Job::Carry,
        Job::Press,
        Job::Recover,
        Job::LooseBall,
        Job::TimedRun,
        Job::Chase,
        Job::Offer,
        Job::Return,
        Job::Follow,
        Job::Hold,
        Job::Wait,
    ];

    /// The job's byte in the snapshot and the gate.
    pub fn code(self) -> u8 {
        self as u8
    }

    /// The job of a snapshot or gate byte.
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.get(usize::from(code)).copied()
    }
}

/// How urgent a job is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Grade {
    Calm = 0,
    Normal = 1,
    Urgent = 2,
}

impl Grade {
    const ALL: [Grade; 3] = [Grade::Calm, Grade::Normal, Grade::Urgent];

    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.get(usize::from(code)).copied()
    }

    fn up(self) -> Self {
        match self {
            Grade::Calm => Grade::Normal,
            _ => Grade::Urgent,
        }
    }

    fn down(self) -> Self {
        match self {
            Grade::Urgent => Grade::Normal,
            _ => Grade::Calm,
        }
    }
}

/// The six pace levels, slowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Level {
    Stand = 0,
    Walk = 1,
    Jog = 2,
    Run = 3,
    HighSpeed = 4,
    Sprint = 5,
}

impl Level {
    const ALL: [Level; 6] = [
        Level::Stand,
        Level::Walk,
        Level::Jog,
        Level::Run,
        Level::HighSpeed,
        Level::Sprint,
    ];

    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.get(usize::from(code)).copied()
    }

    /// One level slower; standing stays standing.
    fn down(self) -> Self {
        Self::ALL[usize::from(self.code().saturating_sub(1))]
    }
}

/// A player's pace this tick: his job, its grade, the level and the speed steering arrives
/// at, the tick his job started, and whether the job's sprint draw succeeded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pace {
    pub job: Job,
    pub grade: Grade,
    pub level: Level,
    /// The speed steering arrives at and never exceeds, in metres per second. The largest
    /// finite value in the off state: steering then reads the player's top speed, as before
    /// the selector. (Finite, so the snapshot and the gate hash it as any other speed.)
    pub speed: f64,
    /// The tick the job started.
    pub since: u32,
    /// The job's sprint draw: `true` lets an URGENT job sprint.
    pub sprint: bool,
}

impl Pace {
    /// The off state: no limit below the player's own top speed. A player starts a match, comes
    /// on, and waits at a dead ball in it.
    pub const OFF: Pace = Pace {
        job: Job::Wait,
        grade: Grade::Calm,
        level: Level::Sprint,
        speed: f64::MAX,
        since: 0,
        sprint: false,
    };
}

/// The grade of a job before the game state and the instructions move it.
pub fn base_grade(job: Job) -> Grade {
    match job {
        Job::Carry | Job::Press | Job::Recover | Job::LooseBall | Job::TimedRun => Grade::Urgent,
        Job::Chase | Job::Offer | Job::Return | Job::Follow => Grade::Normal,
        Job::Hold | Job::Wait => Grade::Calm,
    }
}

/// The game state and the instructions of the player's team, as the grades read them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TeamPace {
    /// Tempo or pressing raised above the normal level.
    pub lift: bool,
    /// Trailing in the late minutes.
    pub trailing_late: bool,
    /// Leading in the late minutes with time wasting on, while the ball is dead. In open play
    /// the team keeps its grades, with or without the ball.
    pub wasting_late: bool,
}

/// Whether an URGENT job keeps its grade. `Keep` is the only value play uses; `Off` is the
/// fault the guarded check must catch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Floor {
    Keep,
    #[cfg(test)]
    Off,
}

impl Floor {
    fn keeps(self) -> bool {
        match self {
            Floor::Keep => true,
            #[cfg(test)]
            Floor::Off => false,
        }
    }
}

/// The grade of `job` under the team's game state: pressing jobs (`Press`, `Chase`) and the
/// support job (`Offer`) rise one step under a lift or trailing late; NORMAL jobs drop one step
/// leading late with time wasting. Under [`Floor::Keep`] an URGENT job never drops.
pub fn adjust(grade: Grade, job: Job, ctx: TeamPace, floor: Floor) -> Grade {
    let base = base_grade(job);
    let mut g = grade;
    if (ctx.lift || ctx.trailing_late) && matches!(job, Job::Press | Job::Chase | Job::Offer) {
        g = g.up();
    }
    if ctx.wasting_late && (base == Grade::Normal || !floor.keeps()) {
        g = g.down();
    }
    if floor.keeps() && base == Grade::Urgent {
        g = Grade::Urgent;
    }
    g
}

/// The indices of every URGENT job graded below URGENT: the guarded check of the floor.
pub fn floor_violations(graded: &[(Job, Grade)]) -> Vec<usize> {
    graded
        .iter()
        .enumerate()
        .filter(|(_, (job, grade))| base_grade(*job) == Grade::Urgent && *grade < Grade::Urgent)
        .map(|(i, _)| i)
        .collect()
}

/// The chance that an URGENT job gets a sprint, from the player's work-rate share (0 to 1).
pub fn sprint_chance(work_share: f64, t: &PaceTuning) -> f64 {
    (t.sprint_mid + t.sprint_slope * (work_share - 0.5)).clamp(0.0, 1.0)
}

/// How much a player saves energy, 0 (fresh) to 1 (empty).
pub fn saving(energy: f64, t: &PaceTuning) -> f64 {
    if energy < t.tired_from && t.tired_from > 0.0 {
        (t.tired_from - energy) / t.tired_from
    } else {
        0.0
    }
}

/// The skip threshold of an URGENT job's sprint: lower for a less urgent job, so its sprint
/// goes first. A job that is not URGENT by its base grade has the press threshold.
fn skip_at(job: Job, t: &PaceTuning) -> f64 {
    let s = &t.skip_at;
    match job {
        Job::Recover => s.recovery,
        Job::LooseBall => s.loose_ball,
        Job::Carry => s.carry,
        Job::TimedRun => s.timed_run,
        _ => s.press,
    }
}

/// The level of a player with a job of `grade` `to_target_m` metres from his target, with the
/// job's sprint draw `sprint`, saving `saving` energy with tired mix `mix` (the mean share of
/// his determination and professionalism).
pub fn level(
    grade: Grade,
    job: Job,
    to_target_m: f64,
    sprint: bool,
    saving: f64,
    mix: f64,
    t: &PaceTuning,
) -> Level {
    if to_target_m < t.stand_m {
        return Level::Stand;
    }
    let base = match grade {
        Grade::Calm => Level::Walk,
        Grade::Normal if to_target_m > t.run_from_m => Level::Run,
        Grade::Normal => Level::Jog,
        Grade::Urgent if sprint => Level::Sprint,
        Grade::Urgent => Level::HighSpeed,
    };
    if base == Level::Sprint {
        if saving * (1.0 - mix) >= skip_at(job, t) && saving > 0.0 {
            Level::HighSpeed
        } else {
            Level::Sprint
        }
    } else if saving * mix >= t.step_down_at && saving > 0.0 {
        // A step slower, but a player away from his target still moves: a walk stays a walk.
        base.down().max(Level::Walk)
    } else {
        base
    }
}

/// The speed of `level` for a player of top speed `top`: the level's upper limit, a sprint
/// his top speed, never above it.
pub fn speed(level: Level, top: f64, t: &PaceTuning) -> f64 {
    let limit = match level {
        Level::Sprint => top,
        other => t.limits_ms[usize::from(other.code())],
    };
    limit.min(top)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped values with a jog band: a NORMAL job jogs within 15 m of its target and
    /// runs beyond (the shipped file runs every NORMAL job), so the level tests see both.
    fn tuning() -> PaceTuning {
        PaceTuning {
            run_from_m: 15.0,
            ..PaceTuning::shipped()
        }
    }

    const LATE_WASTING: TeamPace = TeamPace {
        lift: false,
        trailing_late: false,
        wasting_late: true,
    };

    #[test]
    fn every_job_carries_the_decided_grade() {
        use Grade::*;
        let table = [
            (Job::Carry, Urgent),
            (Job::Press, Urgent),
            (Job::Recover, Urgent),
            (Job::LooseBall, Urgent),
            (Job::TimedRun, Urgent),
            (Job::Chase, Normal),
            (Job::Offer, Normal),
            (Job::Return, Normal),
            (Job::Follow, Normal),
            (Job::Hold, Calm),
            (Job::Wait, Calm),
        ];
        assert_eq!(table.len(), Job::ALL.len());
        for (job, grade) in table {
            assert_eq!(base_grade(job), grade, "{job:?}");
        }
    }

    #[test]
    fn a_lift_or_trailing_late_raises_pressing_and_support_jobs_one_step() {
        for ctx in [
            TeamPace {
                lift: true,
                ..TeamPace::default()
            },
            TeamPace {
                trailing_late: true,
                ..TeamPace::default()
            },
        ] {
            for job in Job::ALL {
                let g = adjust(base_grade(job), job, ctx, Floor::Keep);
                let expected = match job {
                    Job::Chase | Job::Offer => Grade::Urgent,
                    _ => base_grade(job),
                };
                assert_eq!(g, expected, "{job:?} under {ctx:?}");
            }
        }
        // No lift, level and early: every grade stays.
        for job in Job::ALL {
            assert_eq!(
                adjust(base_grade(job), job, TeamPace::default(), Floor::Keep),
                base_grade(job)
            );
        }
    }

    #[test]
    fn leading_late_with_time_wasting_drops_normal_jobs_and_keeps_presses_and_recoveries() {
        for job in Job::ALL {
            let g = adjust(base_grade(job), job, LATE_WASTING, Floor::Keep);
            let expected = match base_grade(job) {
                Grade::Normal => Grade::Calm,
                other => other,
            };
            assert_eq!(g, expected, "{job:?}");
        }
        assert_eq!(
            adjust(Grade::Urgent, Job::Press, LATE_WASTING, Floor::Keep),
            Grade::Urgent
        );
        assert_eq!(
            adjust(Grade::Urgent, Job::Recover, LATE_WASTING, Floor::Keep),
            Grade::Urgent
        );
    }

    /// The guarded check: under the floor no context drops an URGENT job; the fault that
    /// forces the drop is caught.
    #[test]
    fn the_floor_keeps_every_urgent_job_and_a_forced_drop_is_caught() {
        let contexts = [false, true].into_iter().flat_map(|lift| {
            [false, true].into_iter().flat_map(move |trailing_late| {
                [false, true].into_iter().map(move |wasting_late| TeamPace {
                    lift,
                    trailing_late,
                    wasting_late,
                })
            })
        });
        for ctx in contexts {
            let graded: Vec<_> = Job::ALL
                .iter()
                .map(|&job| (job, adjust(base_grade(job), job, ctx, Floor::Keep)))
                .collect();
            assert!(floor_violations(&graded).is_empty(), "{ctx:?}");
        }
        let forced: Vec<_> = [Job::Press, Job::Recover]
            .iter()
            .map(|&job| (job, adjust(base_grade(job), job, LATE_WASTING, Floor::Off)))
            .collect();
        assert_eq!(floor_violations(&forced), vec![0, 1], "{forced:?}");
    }

    #[test]
    fn each_grade_gets_its_level_and_speed_and_a_sprint_never_passes_top_speed() {
        let t = tuning();
        let lv = |g, d, s| level(g, Job::Press, d, s, 0.0, 0.5, &t);
        assert_eq!(lv(Grade::Urgent, 0.5, true), Level::Stand);
        assert_eq!(lv(Grade::Calm, 30.0, false), Level::Walk);
        assert_eq!(lv(Grade::Normal, 10.0, false), Level::Jog);
        assert_eq!(lv(Grade::Normal, 20.0, false), Level::Run);
        assert_eq!(lv(Grade::Urgent, 20.0, false), Level::HighSpeed);
        assert_eq!(lv(Grade::Urgent, 20.0, true), Level::Sprint);
        let speeds: Vec<_> = Level::ALL.iter().map(|&l| speed(l, 9.0, &t)).collect();
        assert_eq!(speeds, vec![0.2, 2.0, 4.0, 5.5, 7.0, 9.0]);
        assert_eq!(speed(Level::Sprint, 6.5, &t), 6.5);
        assert_eq!(speed(Level::HighSpeed, 6.5, &t), 6.5);
    }

    #[test]
    fn work_rate_sets_the_sprint_chance_not_a_speed() {
        let t = tuning();
        assert_eq!(sprint_chance(0.5, &t), 0.5);
        assert!(sprint_chance(0.8, &t) > sprint_chance(0.3, &t));
        assert_eq!(sprint_chance(1.0, &t), 1.0);
        assert_eq!(sprint_chance(0.0, &t), 0.0);
    }

    /// The tired mix: at energy 0.1 a determined, professional player keeps his sprint on a
    /// press and goes a step slower on a return; a low one gives up the sprint and keeps his
    /// run. At a smaller saving the low one still sprints on a recovery while skipping the
    /// carry's sprint. Fresh, both move alike.
    #[test]
    fn the_tired_mix_follows_determination_and_professionalism() {
        let t = tuning();
        let (high, low) = (0.9, 0.25);
        let s = saving(0.1, &t);
        assert!((s - 0.8).abs() < 1e-12);
        let press = |mix, s| level(Grade::Urgent, Job::Press, 20.0, true, s, mix, &t);
        let ret = |mix, s| level(Grade::Normal, Job::Return, 20.0, false, s, mix, &t);
        assert_eq!(press(high, s), Level::Sprint);
        assert_eq!(ret(high, s), Level::Jog);
        assert_eq!(press(low, s), Level::HighSpeed);
        assert_eq!(ret(low, s), Level::Run);
        // A smaller saving: the least urgent sprint goes first.
        let small = saving(0.4, &t);
        let urgent = |job, mix| level(Grade::Urgent, job, 20.0, true, small, mix, &t);
        assert_eq!(urgent(Job::Carry, low), Level::HighSpeed);
        assert_eq!(urgent(Job::Recover, low), Level::Sprint);
        // Control: untired, the two mixes give the same levels.
        for job in Job::ALL {
            for grade in [Grade::Calm, Grade::Normal, Grade::Urgent] {
                for sprint in [false, true] {
                    assert_eq!(
                        level(grade, job, 20.0, sprint, saving(0.9, &t), high, &t),
                        level(grade, job, 20.0, sprint, saving(0.9, &t), low, &t),
                    );
                }
            }
        }
    }

    /// A tired player who steps down still walks to a target beyond the stand distance: the
    /// step down never leaves him standing while his place moves away.
    #[test]
    fn a_tired_step_down_still_walks_to_a_target_away_from_him() {
        let t = tuning();
        let tired = saving(0.1, &t);
        assert!(tired * 1.0 >= t.step_down_at, "the step down applies");
        assert_eq!(
            level(Grade::Calm, Job::Hold, 6.0, false, tired, 1.0, &t),
            Level::Walk
        );
        assert_eq!(
            level(Grade::Normal, Job::Return, 6.0, false, tired, 1.0, &t),
            Level::Walk
        );
        assert_eq!(
            level(Grade::Calm, Job::Hold, 0.5, false, tired, 1.0, &t),
            Level::Stand
        );
    }

    #[test]
    fn every_code_reads_back() {
        for job in Job::ALL {
            assert_eq!(Job::from_code(job.code()), Some(job));
        }
        for g in Grade::ALL {
            assert_eq!(Grade::from_code(g.code()), Some(g));
        }
        for l in Level::ALL {
            assert_eq!(Level::from_code(l.code()), Some(l));
        }
        assert_eq!(Job::from_code(11), None);
        assert_eq!(Level::from_code(6), None);
    }
}
