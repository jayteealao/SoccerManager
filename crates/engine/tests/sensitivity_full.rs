//! Every sensitivity rule and the five-match rule at the size `content/sensitivity.json` sets:
//! about 6,400 matches, a few minutes on all cores of a release build. Run it with
//!
//! ```text
//! cargo test --release -p engine --test sensitivity_full -- --ignored --nocapture
//! ```
//!
//! It prints one line per rule (job, statistic, low, high, move and its interval, share and
//! its interval, word) and the five-match share per role and pooled, writes the same as JSON to
//! `target/sensitivity/rules.json`, and passes only when every rule and the five-match rule
//! pass.

mod common;

use engine::sensitivity::words::Word;
use engine::sensitivity::{self, Size};

#[test]
#[ignore = "the full run plays about 6,400 matches; run it in release by hand"]
fn every_sensitivity_rule_and_the_five_match_rule_pass() {
    let content = common::content();
    let dir = common::content_dir();
    let clubs = sensitivity::default_clubs(&content, &dir).unwrap();
    let file = sensitivity::load_rules(&content, &dir).unwrap();
    let started = std::time::Instant::now();
    let (played, rules) = sensitivity::run_rules(&content, &clubs, &file, None, Size::of(&file));
    let five = sensitivity::five_match::run(
        &content,
        &clubs,
        &file.five_match,
        file.design.seed,
        file.design.resamples,
    );
    let elapsed = started.elapsed();

    println!(
        "{:<17} {:<52} {:>9} {:>9} {:>26} {:>26} word",
        "job", "statistic", "low", "high", "move [95%]", "share [95%]"
    );
    for r in &rules {
        let share = r.share.map_or("-".to_string(), |s| {
            format!("{:+.3} [{:+.3}, {:+.3}]", s.est, s.lo, s.hi)
        });
        println!(
            "{:<17} {:<52} {:>9.4} {:>9.4} {:>26} {:>26} {}",
            r.job,
            r.statistic,
            r.low,
            r.high,
            format!("{:+.3} [{:+.3}, {:+.3}]", r.mv.est, r.mv.lo, r.mv.hi),
            share,
            r.word.name()
        );
    }
    let count = |w: Word| rules.iter().filter(|r| r.word == w).count();
    println!(
        "rules: {} pass, {} fail, {} not sure, of {}",
        count(Word::Pass),
        count(Word::Fail),
        count(Word::NotSure),
        rules.len()
    );
    for role in &five.roles {
        println!(
            "five-match {:?}: {:.3} of {} runs",
            role.role, role.share, role.runs
        );
    }
    println!(
        "five-match pooled: {:.3} [{:.3}, {:.3}], target {:.3} +/- {:.3}: {}",
        five.pooled.est,
        five.pooled.lo,
        five.pooled.hi,
        five.target,
        five.tolerance,
        five.word.name()
    );
    let matches = played.design.len()
        + played.arms.as_ref().map_or(0, |(a, b)| a.len() + b.len())
        + five.roles.iter().map(|r| r.runs).sum::<usize>()
            * 2
            * file.five_match.matches_per_run as usize;
    println!("{matches} matches in {elapsed:.1?}");

    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/sensitivity");
    std::fs::create_dir_all(&out).unwrap();
    let report = serde_json::json!({
        "matches": matches,
        "seconds": elapsed.as_secs_f64(),
        "rules": rules,
        "five_match": five,
    });
    std::fs::write(
        out.join("rules.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();

    let failing: Vec<String> = rules
        .iter()
        .filter(|r| r.word != Word::Pass)
        .map(|r| format!("{} {}", r.job, r.word.name()))
        .collect();
    assert!(
        failing.is_empty(),
        "rules that do not pass: {}",
        failing.join(", ")
    );
    assert_eq!(five.word, Word::Pass, "five-match {:?}", five.pooled);
}
