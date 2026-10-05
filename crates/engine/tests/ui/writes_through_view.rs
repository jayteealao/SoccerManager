//! A module that writes match state through the view does not compile.

use engine::modules::{MatchView, OffsideModule, Proposal};

struct Writer;

impl OffsideModule for Writer {
    fn on_kick(&self, view: &MatchView<'_>, passer: usize, _team: usize) -> Proposal {
        view.player(passer).pos.x = 0.0;
        Proposal::SetOffside(0)
    }

    fn is_offence(&self, _view: &MatchView<'_>, _set: u32, _toucher: usize) -> bool {
        false
    }
}

fn main() {
    let _ = Writer;
}
