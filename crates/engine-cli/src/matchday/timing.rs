//! `engine-cli matchday-timing`: a hidden test seam that times a whole matchday.
//!
//! It limits the process to a number of logical cores, starts the matchday as `serve` does,
//! plays the player's match flat out on the main thread for the most contention, and checks
//! every ground event against its reveal moment: the moment the player's clock reaches the
//! event's tick when the match plays at the given speed from kick-off. It writes the record
//! and exits 0 when every event was computed before its moment, 3 otherwise.

use anyhow::Context;
use engine::{ContentDir, Manager, MatchConfig, Simulation};
use protocol::ServerMessage;

use super::{FINISH_WAIT, Matchday, Options, Round, cores};
use crate::cli::TimingOpts;

/// The exit code when an event was computed after its reveal moment.
pub const EXIT_LATE: i32 = 3;

pub fn run(content_dir: Option<&std::path::Path>, opts: &TimingOpts) -> anyhow::Result<i32> {
    if let Some(n) = opts.cores {
        cores::limit_to(n).map_err(anyhow::Error::msg)?;
    }
    let cores = cores::usable();
    let loaded = crate::content::load(content_dir, None, None, None)?;
    let dir = ContentDir::resolve(content_dir)?;
    let [team_a, team_b] = &loaded.teams;
    let mut player = MatchConfig::new(opts.seed, opts.minutes, &loaded.content, [team_a, team_b])?
        .with_manager(0, Manager::Human);
    loaded.fold(&mut player);
    let round = Round::for_match(
        &dir,
        &loaded.content,
        [&player.teams[0].club_id, &player.teams[1].club_id],
        opts.seed,
    );
    let data = std::env::temp_dir().join(format!("engine-cli-timing-{}", std::process::id()));
    let mut sim = Simulation::new(player.clone())?;
    loaded.attach(&mut sim);
    let matchday = Matchday::start(
        &round,
        &loaded,
        &player,
        Options {
            data_dir: data.clone(),
            match_id: format!("timing-{}", opts.seed),
            threads: opts.threads,
            fault: None,
            not_late_through: 0,
        },
    );
    // The player's match, flat out beside the pool.
    let cap = player.max_ticks();
    while !sim.is_over() && (sim.tick() < cap || sim.in_shootout()) {
        sim.step();
        let _ = sim.take_events();
        let _ = matchday.due(sim.tick(), false);
    }
    sim.finish();
    let played_ms = matchday.started().elapsed().as_millis();
    let rest = matchday.finish(sim.tick(), false, FINISH_WAIT);
    let dt_ms = player.tuning.dt * 1000.0;
    let speed = opts.speed;
    let mut late = Vec::new();
    let events: Vec<serde_json::Value> = matchday
        .computed()
        .iter()
        .map(|(event, at)| {
            let computed_at_ms = at.as_secs_f64() * 1000.0;
            let reveal_at_ms = f64::from(event.tick) * dt_ms / speed;
            if computed_at_ms > reveal_at_ms {
                late.push((event.clone(), computed_at_ms, reveal_at_ms));
            }
            serde_json::json!({
                "fixture": event.fixture,
                "tick": event.tick,
                "kind": event.kind,
                "computed_at_ms": round_ms(computed_at_ms),
                "reveal_at_ms": round_ms(reveal_at_ms),
            })
        })
        .collect();
    let unavailable = rest.iter().any(|m| {
        matches!(m, ServerMessage::GroundEvent(e) if e.kind == protocol::GroundKind::Unavailable)
    });
    let fixtures: Vec<serde_json::Value> = matchday
        .outcomes()
        .iter()
        .zip(&round.fixtures)
        .map(|(outcome, f)| {
            let (failure, ticks, duration_ms) = outcome.unwrap_or((Some("did-not-finish"), 0, 0));
            serde_json::json!({
                "fixture": f.index,
                "seed": f.seed,
                "ticks": ticks,
                "duration_ms": duration_ms,
                "outcome": failure.unwrap_or("success"),
            })
        })
        .collect();
    let record = serde_json::json!({
        "seed": opts.seed,
        "minutes": opts.minutes,
        "cores": cores,
        "threads": matchday.threads(),
        "speed": speed,
        "player_match_ms": played_ms,
        "fixtures": fixtures,
        "events": events,
        "late_events": late.len(),
    });
    std::fs::write(&opts.out, format!("{record:#}\n"))
        .with_context(|| format!("cannot write {}", opts.out.display()))?;
    let _ = std::fs::remove_dir_all(&data);
    println!(
        "matchday timing: {} fixtures on {} threads ({} cores), {} events, {} late at {speed}x",
        round.fixtures.len(),
        matchday.threads(),
        cores,
        events.len(),
        late.len()
    );
    if unavailable {
        eprintln!("error: a background match has no result");
        return Ok(EXIT_LATE);
    }
    if let Some((event, computed, reveal)) = late.first() {
        eprintln!(
            "error: fixture {} {:?} at tick {} was computed at {:.0} ms, after its reveal moment at {:.0} ms",
            event.fixture, event.kind, event.tick, computed, reveal
        );
        return Ok(EXIT_LATE);
    }
    Ok(0)
}

fn round_ms(ms: f64) -> f64 {
    (ms * 10.0).round() / 10.0
}
