//! The gate clock: under `Backstop::Never` no script hook is stopped on time, even when
//! every call runs "long", so a busy machine cannot change a gate match. The control proves
//! the forcing works: the same forced clock under the 2 ms wall limit aborts hooks.

use std::path::Path;
use std::sync::OnceLock;

use engine::data::{TEAM_A_FILE, TEAM_B_FILE, TeamFile};
use engine::gate::{self, Fixture, Inputs, PackInputs, Played};
use engine::{Content, ContentDir};
use script::sandbox::CALL_BACKSTOP;
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

/// The knockout fixture with the sample pack under `backstop`.
fn knockout(backstop: Backstop) -> Played {
    let (dir, content, [a, b]) = loaded();
    let pack = LoadedPack::load_with(&dir.path("scripts/sample"), backstop).unwrap();
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
    gate::play_fixture(&Fixture::knockout(gate::KNOCKOUT_SEED), &inputs).unwrap()
}

#[test]
fn no_hook_is_stopped_on_time_under_the_gate_clock() {
    // The three knockout matches are independent; they play side by side.
    let (unforced, control, forced) = std::thread::scope(|scope| {
        let unforced = scope.spawn(|| knockout(Backstop::Never));
        // Control: the forced clock under the 2 ms limit.
        let control = scope.spawn(|| {
            knockout(Backstop::Skewed {
                limit: Some(CALL_BACKSTOP),
            })
        });
        // The gate clock with the forced clock.
        let forced = scope.spawn(|| knockout(Backstop::Skewed { limit: None }));
        (
            unforced.join().unwrap(),
            control.join().unwrap(),
            forced.join().unwrap(),
        )
    });
    assert!(unforced.facts.script_calls > 0, "the pack's hooks run");
    assert_eq!(unforced.facts.script_aborts, 0);
    assert!(
        control.facts.script_aborts > 0,
        "the forced clock must abort a hook under the wall limit"
    );
    assert_eq!(forced.facts.script_aborts, 0, "no hook is aborted");
    assert_eq!(forced.facts.script_calls, unforced.facts.script_calls);
    assert_eq!(forced.hashes, unforced.hashes);
}
