//! The pass-lane early exit in the carrier's options changes no draw, no score, and no
//! choice: every carrier decision of the 22 gate matches is scored by the code before the
//! exit and by the live code from the same stream position, and the two agree on every
//! team-mate, every score's bits, the chosen pass, and the stream position after the call.
//! The audited matches keep this machine's committed hashes. Controls prove that the audit
//! sees an extra draw, a one-ulp score change, and one more rejected team-mate.
//!
//! The test lives in this crate because the knockout fixture needs the sample script pack.

use std::path::Path;
use std::sync::OnceLock;

use engine::data::{TEAM_A_FILE, TEAM_B_FILE, TeamFile};
use engine::gate::audit::{Control, OptionsAudit, play_audited};
use engine::gate::golden;
use engine::gate::{self, Fixture, Inputs, PackInputs, Played};
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

/// Plays `fixture` with the audit on, with the sample pack under the gate clock as the gate
/// command loads it.
fn audited(fixture: &Fixture, control: Option<Control>) -> (Played, OptionsAudit) {
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
    play_audited(fixture, &inputs, control).unwrap()
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
fn the_early_exit_changes_no_draw_score_or_choice() {
    let fixtures = gate::fixtures();
    assert_eq!(fixtures.len(), 22);
    let results = parallel(fixtures.clone(), |f| audited(f, None));

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/golden.json");
    let file = golden::load(&path, &fixtures).expect("the committed golden file loads");
    let stored = match file.set_for(&golden::set_key()) {
        Ok(set) => Some(set),
        Err(err) => {
            println!("note: skipping the golden-file compare: {err}");
            None
        }
    };

    let mut total = OptionsAudit::default();
    for (fixture, (played, audit)) in fixtures.iter().zip(&results) {
        println!(
            "{}: calls {} breaks {} skipped {} mixed {} mismatches {}",
            fixture.id,
            audit.calls,
            audit.breaks,
            audit.skipped_opponents,
            audit.mixed_calls,
            audit.mismatches
        );
        assert_eq!(
            audit.mismatches, 0,
            "{}: {:?}",
            fixture.id, audit.first_mismatches
        );
        if let Some(set) = stored {
            let want = set.iter().find(|m| m.id == fixture.id).unwrap();
            assert_eq!(&played.hashes, want, "{}: the audited match", fixture.id);
        }
        total.calls += audit.calls;
        total.breaks += audit.breaks;
        total.skipped_opponents += audit.skipped_opponents;
        total.mixed_calls += audit.mixed_calls;
    }
    println!(
        "all: calls {} breaks {} skipped {} mixed {}",
        total.calls, total.breaks, total.skipped_opponents, total.mixed_calls
    );
    assert!(total.calls > 0, "the audit saw carrier decisions");
    assert!(total.breaks > 0, "the early exit fired");
    assert!(
        total.skipped_opponents > 0,
        "the early exit skipped opponents"
    );
    assert!(
        total.mixed_calls > 0,
        "a call had both a rejected and a scored team-mate"
    );
}

#[test]
fn the_audit_sees_each_control() {
    let controls = vec![Control::ExtraDraw, Control::OneUlp, Control::RejectMate];
    let results = parallel(controls.clone(), |&c| audited(&Fixture::seed(42), Some(c)));
    for (control, (_, audit)) in controls.iter().zip(&results) {
        println!(
            "{control:?}: calls {} mismatches {} first {:?}",
            audit.calls,
            audit.mismatches,
            audit.first_mismatches.first()
        );
        assert!(audit.mismatches > 0, "{control:?} is not seen");
    }
}
