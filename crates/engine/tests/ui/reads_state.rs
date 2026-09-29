//! A module that only reads the view compiles.

use engine::modules::{MatchView, OffsideModule, Proposal};

struct Reader;

impl OffsideModule for Reader {
    fn on_kick(&self, view: &MatchView<'_>, passer: usize, team: usize) -> Proposal {
        let ahead = view
            .players()
            .iter()
            .enumerate()
            .filter(|(i, p)| {
                *i != passer && p.team == team && p.pos.x * view.attack_x(team) > view.ball_x()
            })
            .fold(0u32, |set, (i, _)| set | (1 << i));
        let _ = (view.tick(), view.tuning(), view.player(passer).yellow);
        Proposal::SetOffside(ahead)
    }

    fn is_offence(&self, _view: &MatchView<'_>, set: u32, toucher: usize) -> bool {
        set & (1 << toucher) != 0
    }
}

fn main() {
    let _ = Reader;
}
