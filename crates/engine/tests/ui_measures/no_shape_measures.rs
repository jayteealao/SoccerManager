// A normal build has no team-shape measure: the sink does not exist.
use engine::measures::shape::ShapeSink;

fn main() {
    let _ = ShapeSink::default();
}
