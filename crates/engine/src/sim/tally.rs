//! Per-player tallies: what each player did in the match, kept beside the team counts for the
//! match rating ([`crate::match_rating`]). A tally is keyed by side and squad index, not by
//! roster slot, because a substitute takes the slot of the player he replaces.

use crate::match_rating::{Outcome, rating};
use crate::math::DVec2;
use crate::sim::Simulation;

/// What one player did in a match.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerTally {
    /// `true` once he took the pitch: a starter, or a substitute who came on.
    pub played: bool,
    /// Ticks on the pitch, counted at each effective-value refresh.
    pub ticks_played: u32,
    /// Open-play passes he played, and those a team-mate controlled next.
    pub passes: u32,
    pub passes_completed: u32,
    pub shots: u32,
    pub shots_on_target: u32,
    /// Goals he scored for his side; an own goal counts for nobody.
    pub goals: u32,
    /// The expected goals of his shots.
    pub xg: f64,
    /// Tackles that won the ball.
    pub tackles_won: u32,
    /// Passes of the other side he controlled.
    pub interceptions: u32,
    /// Shots he blocked.
    pub blocks: u32,
    /// His clearances: a carrier's clearance and a cleared pass.
    pub clearances: u32,
    /// Shots he saved, held or parried.
    pub saves: u32,
    pub fouls: u32,
    /// Yellow cards, a second yellow included, and sendings-off.
    pub yellow: u32,
    pub red: u32,
    /// Goals his side conceded while he was on the pitch.
    pub conceded: u32,
    /// Goals he made: his completed pass reached the scorer.
    pub assists: u32,
    /// `true` when he was on the pitch at kick-off.
    pub started: bool,
    /// As his side's keeper: the post-shot expected goals of the shots on target he faced,
    /// the save line's chance that each goes in (`1 -` [`crate::shot::save_chance`] at the
    /// shot's quality), and the goals from recorded shots he conceded. Their difference is
    /// the goals he prevented.
    pub psxg_faced: f64,
    pub shot_goals_conceded: u32,
    /// The threat his actions added, in expected goals, read with the real threat of a place
    /// ([`crate::sim::place_threat`]): a spell on the ball that ends with a team-mate gains
    /// the threat where the team-mate takes it less the threat where his spell began; a spell
    /// that ends with the other side loses the threat where it began; a shot adds its expected
    /// goals less the threat where his spell began, and its goal the rest of a goal; a tackle
    /// won, an interception, a block or a cleared ball of the other side adds the threat the
    /// other side had at that place.
    pub value_added: f64,
}

/// Which counter a match event adds to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Count {
    Pass,
    PassCompleted,
    Shot,
    ShotOnTarget,
    Goal,
    TackleWon,
    Interception,
    Block,
    Clearance,
    Save,
    Foul,
    Yellow,
    Red,
    Assist,
}

impl PlayerTally {
    fn add(&mut self, count: Count) {
        let c = match count {
            Count::Pass => &mut self.passes,
            Count::PassCompleted => &mut self.passes_completed,
            Count::Shot => &mut self.shots,
            Count::ShotOnTarget => &mut self.shots_on_target,
            Count::Goal => &mut self.goals,
            Count::TackleWon => &mut self.tackles_won,
            Count::Interception => &mut self.interceptions,
            Count::Block => &mut self.blocks,
            Count::Clearance => &mut self.clearances,
            Count::Save => &mut self.saves,
            Count::Foul => &mut self.fouls,
            Count::Yellow => &mut self.yellow,
            Count::Red => &mut self.red,
            Count::Assist => &mut self.assists,
        };
        *c += 1;
    }
}

impl Simulation {
    /// Adds `count` to the tally of the player at roster index `i`.
    #[inline]
    pub(crate) fn tally(&mut self, i: usize, count: Count) {
        let p = &self.players[i];
        self.tallies[p.team][p.squad].add(count);
    }

    /// Adds `xg` to the expected goals of the player at roster index `i`.
    #[inline]
    pub(crate) fn tally_xg(&mut self, i: usize, xg: f64) {
        let p = &self.players[i];
        self.tallies[p.team][p.squad].xg += xg;
    }

    /// A shot on target at `team`'s goal, of save-line quality `quality`: its keeper faced
    /// its chance of going in.
    pub(crate) fn tally_faced(&mut self, team: usize, quality: f64) {
        let k = self.keeper(team);
        let line = crate::shot::save_chance(quality, &self.config.tuning);
        let p = &self.players[k];
        self.tallies[team][p.squad].psxg_faced += 1.0 - line;
    }

    /// A goal from a recorded shot against `team`: its keeper conceded it.
    pub(crate) fn tally_shot_goal_conceded(&mut self, team: usize) {
        let k = self.keeper(team);
        let p = &self.players[k];
        self.tallies[team][p.squad].shot_goals_conceded += 1;
    }

    /// A goal against `team`: every player of it on the pitch conceded one.
    pub(crate) fn tally_conceded(&mut self, team: usize) {
        for p in &self.players {
            if p.team == team && p.active() {
                self.tallies[team][p.squad].conceded += 1;
            }
        }
    }

    /// The real threat of the place `at` for `team`, in expected goals (0 with the threat
    /// off).
    fn threat_xg(&self, team: usize, at: DVec2) -> f64 {
        let t = &self.config.tuning.decision.threat;
        crate::sim::place_threat(at, self.teams[team].attack_x, t, self.config.pitch()) * t.unit
    }

    fn add_value(&mut self, i: usize, v: f64) {
        let p = &self.players[i];
        self.tallies[p.team][p.squad].value_added += v;
    }

    /// The player at roster index `i` takes the ball at `at`, in open play or as a restart's
    /// taker: the open spell on the ball closes. A spell of a team-mate gains the threat at
    /// `at` less the threat where it began; a spell of the other side loses the threat where it
    /// began. His own spell goes on from where it began.
    pub(crate) fn value_gain(&mut self, i: usize, at: DVec2) {
        let team = self.players[i].team;
        if let Some((k, from)) = self.value_from {
            if k == i {
                return;
            }
            let side = self.players[k].team;
            let v = if side == team {
                self.threat_xg(side, at) - self.threat_xg(side, from)
            } else {
                -self.threat_xg(side, from)
            };
            self.add_value(k, v);
        }
        self.value_from = Some((i, at));
    }

    /// The player at roster index `i` shoots from `at` with expected goals `xg`: the shot
    /// closes his spell (or the spell that reached him) and adds its expected goals less the
    /// threat where his spell began; nobody's spell is open while the shot runs.
    pub(crate) fn value_shot(&mut self, i: usize, at: DVec2, xg: f64) {
        self.value_gain(i, at);
        let from = self.value_from.map_or(at, |(_, from)| from);
        let team = self.players[i].team;
        let v = xg - self.threat_xg(team, from);
        self.add_value(i, v);
        self.value_from = None;
    }

    /// The shot of the player at roster index `i`, of expected goals `xg`, went in: the goal
    /// adds the rest of a goal.
    pub(crate) fn value_goal(&mut self, i: usize, xg: f64) {
        self.add_value(i, 1.0 - xg);
    }

    /// The player at roster index `i` won the ball, cut out or blocked a pass or a shot, or
    /// cleared a ball of the other side at `at`: he adds the threat the other side had there.
    pub(crate) fn value_won(&mut self, i: usize, at: DVec2) {
        let other = 1 - self.players[i].team;
        let v = self.threat_xg(other, at);
        self.add_value(i, v);
    }

    /// Marks the player at roster index `i` as having taken the pitch.
    pub(crate) fn tally_played(&mut self, i: usize) {
        let p = &self.players[i];
        self.tallies[p.team][p.squad].played = true;
    }

    /// Marks the player at roster index `i` as on the pitch at kick-off.
    pub(crate) fn tally_started(&mut self, i: usize) {
        let p = &self.players[i];
        self.tallies[p.team][p.squad].started = true;
    }

    /// `ticks` more on the pitch for every player of both sides who is on it.
    pub(crate) fn tally_ticks(&mut self, ticks: u32) {
        for p in &self.players {
            if p.active() {
                self.tallies[p.team][p.squad].ticks_played += ticks;
            }
        }
    }

    /// Every player's tally, by side and squad index.
    pub fn tallies(&self) -> &[Vec<PlayerTally>; 2] {
        &self.tallies
    }

    /// The match rating of every player who played, home first and in squad order: final
    /// once the match is over. A shoot-out decides a knockout match but rates as a draw.
    pub fn match_ratings(&self) -> Vec<PlayerRating> {
        let goals = self.summary.goals;
        let mut out = Vec::new();
        for (team, tallies) in self.tallies.iter().enumerate() {
            let outcome = match goals[team].cmp(&goals[1 - team]) {
                std::cmp::Ordering::Greater => Outcome::Win,
                std::cmp::Ordering::Equal => Outcome::Draw,
                std::cmp::Ordering::Less => Outcome::Loss,
            };
            for (squad, t) in tallies.iter().enumerate() {
                if !t.played {
                    continue;
                }
                let position = self.teams[team].squad[squad].position;
                out.push(PlayerRating {
                    team,
                    squad,
                    rating_tenths: rating(
                        t,
                        position,
                        outcome,
                        self.tick,
                        &self.config.match_rating,
                    ),
                });
            }
        }
        out
    }
}

/// One player's match rating: his side, his squad index, and the rating in tenths, 10 to
/// 100 (1.0 to 10.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerRating {
    pub team: usize,
    pub squad: usize,
    pub rating_tenths: u8,
}

impl PlayerRating {
    /// The rating with one decimal, 1.0 to 10.0.
    pub fn rating(&self) -> f64 {
        f64::from(self.rating_tenths) / 10.0
    }
}

#[cfg(test)]
mod tests {
    use crate::data::test_support::shipped_config;
    use crate::math::DVec2;
    use crate::sim::Simulation;

    /// A home and an away player with the threat of a place on, and where the home side's
    /// box and halfway are.
    fn scene() -> (Simulation, f64) {
        let sim = Simulation::new(shipped_config(7, 90).unwrap()).unwrap();
        let attack = sim.teams[0].attack_x;
        (sim, attack)
    }

    fn value(sim: &Simulation, i: usize) -> f64 {
        let p = &sim.players[i];
        sim.tallies[p.team][p.squad].value_added
    }

    #[test]
    fn a_pass_into_the_box_adds_threat_and_a_ball_lost_there_gives_it_up() {
        let (mut sim, a) = scene();
        let (passer, receiver, defender) = (6, 9, 13);
        let halfway = DVec2::new(5.0 * a, 10.0);
        let box_spot = DVec2::new(44.0 * a, 3.0);
        sim.value_gain(passer, halfway);
        sim.value_gain(receiver, box_spot);
        let gained = sim.threat_xg(0, box_spot) - sim.threat_xg(0, halfway);
        assert!(gained > 0.03, "a pass into the box gains {gained}");
        assert!((value(&sim, passer) - gained).abs() < 1e-12);
        // The receiver loses the ball where he took it: his side gives up its threat there.
        sim.value_gain(defender, box_spot);
        assert!((value(&sim, receiver) + sim.threat_xg(0, box_spot)).abs() < 1e-12);
        // A square pass at halfway adds almost nothing.
        let (mut sim, a) = scene();
        sim.value_gain(passer, DVec2::new(5.0 * a, 10.0));
        sim.value_gain(receiver, DVec2::new(5.0 * a, -10.0));
        assert!(value(&sim, passer).abs() < 0.002, "{}", value(&sim, passer));
    }

    #[test]
    fn a_ball_won_near_the_own_goal_earns_more_than_one_won_at_halfway() {
        let (mut sim, a) = scene();
        let defender = 13;
        // The home side attacks toward x = 40·a, so the away defender's own goal is there.
        sim.value_won(defender, DVec2::new(44.0 * a, 0.0));
        let near = value(&sim, defender);
        let (mut sim, _) = scene();
        sim.value_won(defender, DVec2::new(0.0, 0.0));
        let halfway = value(&sim, defender);
        assert!(
            near > 10.0 * halfway && halfway > 0.0,
            "{near} against {halfway}"
        );
    }

    #[test]
    fn a_shot_adds_its_expected_goals_and_its_goal_the_rest_of_a_goal() {
        let (mut sim, a) = scene();
        let (passer, shooter) = (6, 9);
        let from = DVec2::new(10.0 * a, 0.0);
        let spot = DVec2::new(40.0 * a, 2.0);
        sim.value_gain(passer, from);
        sim.value_shot(shooter, spot, 0.12);
        // The pass that reached him gains the threat at his spot; his shot adds its expected
        // goals less that threat, and nobody's spell runs on.
        let at = sim.threat_xg(0, spot);
        assert!((value(&sim, passer) - (at - sim.threat_xg(0, from))).abs() < 1e-12);
        assert!((value(&sim, shooter) - (0.12 - at)).abs() < 1e-12);
        assert!(sim.value_from.is_none());
        sim.value_goal(shooter, 0.12);
        assert!((value(&sim, shooter) - (1.0 - at)).abs() < 1e-12);
    }
}
