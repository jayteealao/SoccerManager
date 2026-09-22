//! Offside (IFAB Law 11). A player is in an offside position when a team-mate plays the ball
//! and the player is in the opponents' half, nearer to the opponents' goal line than both the
//! ball and the second-last opponent. Level is onside. The set of players in an offside
//! position is computed once, when the team-mate plays the ball, and an offence is called
//! only when one of them is the next player to touch the ball. A throw-in, a goal kick, and a
//! corner create no offside, so the caller computes no set for them.

use crate::player::Player;

/// Roster indices in an offside position, one bit per player.
pub type OffsideSet = u32;

/// The team-mates of `passer` in an offside position when `passer` plays the ball at
/// `ball_x`. `attack_x` is the passer's attack direction.
pub fn offside_set(passer: usize, ball_x: f64, players: &[Player], attack_x: f64) -> OffsideSet {
    let team = players[passer].team;
    // The two opponents nearest their own goal line, measured along the attack direction.
    let mut last = f64::NEG_INFINITY;
    let mut second = f64::NEG_INFINITY;
    for p in players {
        if p.team == team || !p.active() {
            continue;
        }
        let depth = p.pos.x * attack_x;
        if depth > last {
            second = last;
            last = depth;
        } else if depth > second {
            second = depth;
        }
    }
    let ball = ball_x * attack_x;
    let mut set = 0;
    for (i, p) in players.iter().enumerate() {
        if i == passer || p.team != team || !p.active() {
            continue;
        }
        let depth = p.pos.x * attack_x;
        if depth > 0.0 && depth > ball && depth > second {
            set |= 1 << i;
        }
    }
    set
}

/// `true` when `toucher` was in the offside position set when the ball was played.
pub fn is_offence(set: OffsideSet, toucher: usize) -> bool {
    set & (1 << toucher) != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::DVec2;
    use crate::player::test_support::flat_player;
    use crate::tuning::Tuning;

    /// Home attacks +x. Home 0 passes from `passer_x`; home 1 is the attacker; away 2 and 3
    /// are the last two defenders.
    fn scene(passer_x: f64, attacker_x: f64, defenders: [f64; 2]) -> Vec<Player> {
        let t = Tuning::default();
        let mut players = Vec::new();
        for (i, (team, x)) in [
            (0, passer_x),
            (0, attacker_x),
            (1, defenders[0]),
            (1, defenders[1]),
        ]
        .into_iter()
        .enumerate()
        {
            let mut p = flat_player(i, 50, &t);
            p.team = team;
            p.pos = DVec2::new(x, 0.0);
            players.push(p);
        }
        players
    }

    #[test]
    fn beyond_the_second_last_defender_is_offside() {
        let players = scene(10.0, 40.0, [35.0, 50.0]);
        let set = offside_set(0, 10.0, &players, 1.0);
        assert!(is_offence(set, 1));
        assert!(!is_offence(set, 0));
    }

    #[test]
    fn level_with_the_second_last_defender_is_onside() {
        let players = scene(10.0, 35.0, [35.0, 50.0]);
        assert_eq!(offside_set(0, 10.0, &players, 1.0), 0);
    }

    #[test]
    fn the_own_half_is_onside() {
        let players = scene(-30.0, -1.0, [-20.0, -5.0]);
        assert_eq!(offside_set(0, -30.0, &players, 1.0), 0);
    }

    #[test]
    fn behind_the_ball_is_onside() {
        let players = scene(45.0, 40.0, [30.0, 50.0]);
        assert_eq!(offside_set(0, 45.0, &players, 1.0), 0);
    }

    #[test]
    fn the_away_team_attacks_the_other_way() {
        let mut players = scene(-10.0, -40.0, [-35.0, -50.0]);
        for p in &mut players {
            p.team = 1 - p.team;
        }
        assert!(is_offence(offside_set(0, -10.0, &players, -1.0), 1));
    }
}
