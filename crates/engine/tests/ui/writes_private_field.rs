//! A module cannot reach the match behind the view to write it.

use engine::modules::{MatchView, OffsideModule, Proposal};

struct Reacher;

impl OffsideModule for Reacher {
    fn on_kick(&self, view: &MatchView<'_>, _passer: usize, _team: usize) -> Proposal {
        let _sim = view.sim;
        Proposal::SetOffside(0)
    }

    fn is_offence(&self, _view: &MatchView<'_>, _set: u32, _toucher: usize) -> bool {
        false
    }
}

fn main() {
    let _ = Reacher;
}
