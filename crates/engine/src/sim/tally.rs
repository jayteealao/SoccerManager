//! Per-player tallies: what each player did in the match, kept beside the team counts for the
//! match rating ([`crate::match_rating`]). A tally is keyed by side and squad index, not by
//! roster slot, because a substitute takes the slot of the player he replaces.

use crate::match_rating::{Outcome, rating};
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

    /// A goal against `team`: every player of it on the pitch conceded one.
    pub(crate) fn tally_conceded(&mut self, team: usize) {
        for p in &self.players {
            if p.team == team && p.active() {
                self.tallies[team][p.squad].conceded += 1;
            }
        }
    }

    /// Marks the player at roster index `i` as having taken the pitch.
    pub(crate) fn tally_played(&mut self, i: usize) {
        let p = &self.players[i];
        self.tallies[p.team][p.squad].played = true;
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
