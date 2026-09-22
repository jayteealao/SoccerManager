//! Cards and sendings-off (IFAB Laws 3 and 12). A second caution sends a player off, as a
//! straight red card does. A sent-off player stands at a fixed parking spot beside the pitch
//! until full time, because the tick record holds 22 positions and no status. The team's
//! formation line closes the gap, and a team with fewer players than the rule pack allows
//! cannot continue, so the match is abandoned.

use crate::math::DVec2;
use crate::pitch;
use crate::player::{Player, Status};
use crate::rules::fouls::Card;
use crate::team::Team;

/// Records `card` against `player`. Returns `true` when the card sends the player off.
pub fn book(player: &mut Player, card: Card) -> bool {
    if card != Card::Red {
        player.yellow = player.yellow.saturating_add(1);
    }
    card.sends_off()
}

/// Takes player `i` off the pitch to its parking spot and closes the gap in its line.
pub fn send_off(players: &mut [Player], teams: &mut [Team; 2], i: usize) {
    let p = &mut players[i];
    p.status = Status::SentOff;
    p.pos = pitch::parking_spot(p.team, p.slot);
    p.vel = DVec2::ZERO;
    p.target = p.pos;
    teams[p.team].reshape(p.slot);
}

/// Players of `team` on the pitch.
pub fn on_pitch_count(players: &[Player], team: usize) -> usize {
    players
        .iter()
        .filter(|p| p.team == team && p.active())
        .count()
}

/// The first team with fewer than `min_players` on the pitch, or `None`.
pub fn abandoned(players: &[Player], min_players: u8) -> Option<usize> {
    (0..2).find(|&team| on_pitch_count(players, team) < usize::from(min_players))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;

    #[test]
    fn a_second_yellow_sends_off_and_a_first_does_not() {
        let config = shipped_config(1, 90).unwrap();
        let mut p = config.players[3];
        assert!(!book(&mut p, Card::Yellow));
        assert_eq!(p.yellow, 1);
        assert!(book(&mut p, Card::SecondYellow));
        assert_eq!(p.yellow, 2);
    }

    #[test]
    fn a_send_off_parks_the_player_and_five_abandon_the_match() {
        let config = shipped_config(1, 90).unwrap();
        let mut players = config.players.clone();
        let mut teams = config.teams.clone();
        send_off(&mut players, &mut teams, 2);
        assert_eq!(on_pitch_count(&players, 0), 10);
        assert!(!pitch::contains(players[2].pos));
        assert!(!players[2].active());
        assert_eq!(abandoned(&players, 7), None);
        for i in [3, 4, 5, 6] {
            send_off(&mut players, &mut teams, i);
        }
        assert_eq!(on_pitch_count(&players, 0), 6);
        assert_eq!(abandoned(&players, 7), Some(0));
    }
}
