//! AC-b: the same seed and build on the same machine produce byte-identical tick files.

mod common;

use engine::{FileSink, Simulation, ticks_for_minutes};

fn write(name: &str) -> Vec<u8> {
    let path = common::temp_path(name);
    let config = common::full_match();
    let ticks = ticks_for_minutes(90);
    let mut sim = Simulation::new(config.clone()).unwrap();
    let mut sink = FileSink::create(&path, config.seed, config.tuning.dt, ticks).unwrap();
    sim.run(ticks, &mut sink).unwrap();
    sink.finish().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    bytes
}

#[test]
fn two_runs_are_byte_identical() {
    let a = write("det-a");
    let b = write("det-b");
    assert_eq!(a.len(), b.len());
    assert!(a == b, "the two tick files differ");
}
