//! Debug mode over the 22 gate matches: every draw is recorded with its fields and counted
//! against the registry; on every tick each draw and each event has one of its
//! coverage points, and the points reached plus the scene-only points are the whole
//! coverage list; and every traced match keeps the committed portable hashes.
//!
//! The test lives in this crate because the knockout fixture needs the sample script pack.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::OnceLock;

use engine::data::{TEAM_A_FILE, TEAM_B_FILE, TeamFile};
use engine::gate::golden;
use engine::gate::{self, Fixture, Inputs, PackInputs, Played};
use engine::trace::{self, Point};
use engine::{Content, ContentDir};
use script::{Backstop, LoadedPack};

fn loaded() -> &'static (ContentDir, Content, [TeamFile; 2]) {
    static LOADED: OnceLock<(ContentDir, Content, [TeamFile; 2])> = OnceLock::new();
    LOADED.get_or_init(|| {
        let dir = ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"));
        let content = Content::load(&dir).expect("the shipped content loads");
        let a = content
            .load_team(&dir, &dir.path(TEAM_A_FILE))
            .unwrap()
            .value;
        let b = content
            .load_team(&dir, &dir.path(TEAM_B_FILE))
            .unwrap()
            .value;
        (dir, content, [a, b])
    })
}

/// What one traced match showed.
struct Traced {
    played: Played,
    registry: u64,
    draws: u64,
    points: BTreeSet<Point>,
    /// The first failure of the draw or tick checks, if any.
    failure: Option<String>,
}

/// Plays `fixture` traced, with the sample pack as the gate command loads it, checking
/// each tick as it is handed over, so no match keeps its whole trace in memory.
fn traced(fixture: &Fixture) -> Traced {
    let (dir, content, [a, b]) = loaded();
    let pack = LoadedPack::load_with(
        &dir.path(&format!("scripts/{}", gate::KNOCKOUT_PACK)),
        Backstop::Never,
    )
    .unwrap();
    let hooks = || pack.plugins();
    let inputs = Inputs {
        content,
        teams: [a, b],
        pack: Some(PackInputs {
            id: &pack.pack.manifest.id,
            sha: *pack.sha(),
            hooks: &hooks,
        }),
    };
    let mut draws = trace::DrawCheck::default();
    let mut points = BTreeSet::new();
    let mut failure = None;
    let (played, registry) = gate::play_traced(fixture, &inputs, |records, events| {
        trace::note_points(&mut points, records);
        if failure.is_none()
            && let Err(e) = draws
                .feed(records)
                .and_then(|()| trace::check_tick(records, events))
        {
            failure = Some(e);
        }
    })
    .unwrap();
    if failure.is_none()
        && let Err(e) = draws.finish(registry)
    {
        failure = Some(e);
    }
    Traced {
        played,
        registry,
        draws: draws.count(),
        points,
        failure,
    }
}

/// Runs `jobs` on eight threads and returns the results in order.
fn parallel<T: Sync, R: Send>(jobs: Vec<T>, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let chunk = jobs.len().div_ceil(8).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .chunks(chunk)
            .map(|part| scope.spawn(|| part.iter().map(&f).collect::<Vec<R>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    })
}

#[test]
fn every_gate_match_traces_every_draw_and_point_and_keeps_its_hashes() {
    let fixtures = gate::fixtures();
    assert_eq!(fixtures.len(), 22);
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/golden.json");
    let file = golden::load(&path, &fixtures).expect("the committed golden file loads");
    let set = file
        .set_for(golden::PORTABLE)
        .expect("the committed golden file has the portable set");

    let results = parallel(fixtures.clone(), traced);
    let mut union = BTreeSet::new();
    for (fixture, t) in fixtures.iter().zip(&results) {
        println!(
            "{}: {} draws recorded, registry {}, {} points",
            fixture.id,
            t.draws,
            t.registry,
            t.points.len()
        );
        assert_eq!(t.failure, None, "{}", fixture.id);
        assert_eq!(t.draws, t.registry, "{}", fixture.id);
        let want = set.iter().find(|m| m.id == fixture.id).unwrap();
        assert_eq!(&t.played.hashes, want, "{}: the traced match", fixture.id);
        union.extend(t.points.iter().copied());
    }
    let reached: Vec<_> = union.iter().map(|p| p.name()).collect();
    println!("points reached ({}): {}", reached.len(), reached.join(", "));
    let unreached: Vec<_> = Point::ALL
        .iter()
        .filter(|p| !union.contains(p))
        .map(|p| p.name())
        .collect();
    println!("points not reached: {}", unreached.join(", "));
    trace::check_partition(&union).unwrap();
}
