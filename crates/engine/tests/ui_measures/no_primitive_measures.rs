// A normal build has no primitive measure: the possession counters and the block height do
// not exist.
use engine::measures::MatchMeasures;

fn main() {
    let m = MatchMeasures::default();
    let _ = m.turnovers;
}
