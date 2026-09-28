//! A stand-in engine build for the bisect tests: a standalone program (std only, not a cargo
//! target) that answers `resimulate --help` and writes a state digest file, a state fields
//! file, a debug trace, and a verdict line in the re-simulate format. The test compiles it
//! with rustc and copies it under behaviour names; the behaviour comes from its file stem:
//!
//! - `twin*`: 3,000 ticks, the same state as every other twin;
//! - `differs-at-2000*`: as a twin until tick 1,999; from tick 2,000 the ball's speed and
//!   one stream differ;
//! - `crash*`: panics when it re-simulates;
//! - `nocommand*`: has no re-simulate command;
//! - `nooption*`: re-simulates but has none of the four options;
//! - `short*`: the digest file stops with no end line;
//! - `gap*`: the digest file skips tick 1,500;
//! - `hang*`: never finishes a re-simulation;
//! - `marker*`: writes `<its own path>.started` when it starts, then acts as a twin.

use std::path::PathBuf;

const TICKS: u32 = 3_000;
const OPTIONS: &str = "      --state-digests <FILE>\n      --state-fields <FILE>\n      --at-tick <TICK>\n      --debug-trace <FILE>\n";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn floats(v: &[f64]) -> String {
    hex(&v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>())
}

fn streams(extra: u128) -> String {
    let mut out = vec![1u8];
    out.extend(2u32.to_le_bytes());
    for (id, pos) in [(7u64, 100u128), (9u64, 200u128 + extra)] {
        out.extend(id.to_le_bytes());
        out.extend(pos.to_le_bytes());
    }
    hex(&out)
}

fn main() {
    let exe = std::env::current_exe().expect("the program knows its path");
    let stem = exe
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("twin")
        .to_string();
    let is = |name: &str| stem.starts_with(name);
    if is("marker") {
        let mut marker = exe.clone().into_os_string();
        marker.push(".started");
        std::fs::write(PathBuf::from(marker), "started").expect("the marker is written");
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("resimulate") || is("nocommand") {
        eprintln!("error: unrecognized subcommand 'resimulate'");
        std::process::exit(2);
    }
    if args.iter().any(|a| a == "--help") {
        print!("Play a recorded match again from its replay file alone.\n\nUsage: engine-cli resimulate [OPTIONS] --fixture <FILE>\n\nOptions:\n      --fixture <FILE>\n      --compare\n");
        if !is("nooption") {
            print!("{OPTIONS}");
        }
        return;
    }
    if is("nooption") {
        eprintln!("error: unexpected argument '--state-digests' found");
        std::process::exit(2);
    }
    if is("crash") {
        panic!("a prepared crash");
    }
    if is("hang") {
        std::thread::sleep(std::time::Duration::from_secs(600));
    }
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let differs_from = if is("differs-at-2000") { 2_000 } else { u32::MAX };
    let engine = format!(
        "{{\"commit\":\"{stem}\",\"dirty\":false,\"executable_sha256\":\"{stem}\",\"scheme\":1}}"
    );
    if let Some(path) = value("--state-digests") {
        let mut text = format!(
            "{{\"state_digests\":1,\"inventory\":1,\"gate_schema\":1,\"scheme\":1,\"engine\":{engine}}}\n"
        );
        let mut lines = 0;
        for tick in 1..=TICKS {
            if is("gap") && tick == 1_500 {
                continue;
            }
            let salt = if tick >= differs_from { 1u64 << 40 } else { 0 };
            text.push_str(&format!("{tick} {:064x}\n", u64::from(tick) + salt));
            lines += 1;
        }
        text.push_str(&format!("finish {:064x}\n", if differs_from == u32::MAX { 1 } else { 2 }));
        if !is("short") {
            text.push_str(&format!("end {lines} true\n"));
        }
        std::fs::write(path, text).expect("the digest file is written");
    }
    let tick: u32 = value("--at-tick").map_or(TICKS, |t| t.parse().expect("a tick"));
    let changed = tick >= differs_from;
    if let Some(path) = value("--state-fields") {
        let vel = if changed { 1.25 + 1e-9 } else { 1.25 };
        let text = format!(
            "{{\"tick\":{tick},\"scheme\":1,\"engine\":{engine},\"fields\":[\
             {{\"name\":\"tick\",\"kind\":\"bytes\",\"hex\":\"{}\"}},\
             {{\"name\":\"ball.pos\",\"kind\":\"floats\",\"hex\":\"{}\"}},\
             {{\"name\":\"ball.vel\",\"kind\":\"floats\",\"hex\":\"{}\"}},\
             {{\"name\":\"streams\",\"kind\":\"streams\",\"hex\":\"{}\"}},\
             {{\"name\":\"events\",\"kind\":\"bytes\",\"hex\":\"00000000\"}}]}}\n",
            hex(&tick.to_le_bytes()),
            floats(&[10.0, 20.0, 0.0]),
            floats(&[vel, -0.5, 0.0]),
            streams(u128::from(changed)),
        );
        std::fs::write(path, text).expect("the fields file is written");
    }
    if let Some(path) = value("--debug-trace") {
        let mut text = String::from("{\"seed\":42,\"scheme\":1,\"trace_version\":1}\n");
        for t in [tick.saturating_sub(1), tick] {
            let draw = if t >= differs_from { 0.75 } else { 0.25 };
            text.push_str(&format!(
                "{{\"t\":{t},\"k\":\"draw\",\"stream\":\"0x0000000000000009\",\"value\":{draw}}}\n"
            ));
        }
        std::fs::write(path, text).expect("the trace file is written");
    }
    println!("{{\"mode\":\"compare\",\"verdict\":\"identical\"}}");
}
