//! Every match writes one statistics record and one event stream file that validate against
//! the record schemas in `schemas/observability/` as written, with no transformation. The
//! schemas are checked by an independent JSON Schema validator, so a drift between the
//! engine's records and the schema files fails here.

mod common;

use common::{RecordSchemas, bin, record, temp};

#[test]
fn a_simulated_match_writes_records_that_validate_against_the_schemas() {
    let data = temp("engine-cli-schemas", "simulate");
    let out = bin(&data)
        .args(["simulate", "--seed", "7", "--minutes", "5", "--no-snapshot"])
        .arg("--ticks-out")
        .arg(data.join("m.ticks"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let schemas = RecordSchemas::load();
    let printed = record(&String::from_utf8_lossy(&out.stdout));
    let id = printed["match.id"].as_str().unwrap().to_string();
    let folder = data.join("matches").join(&id);

    let stats = record(&std::fs::read_to_string(folder.join("stats.json")).unwrap());
    schemas.stats(&stats).unwrap();
    assert_eq!(stats, printed);

    let events = std::fs::read_to_string(folder.join("events.jsonl")).unwrap();
    let rows: Vec<_> = events.lines().map(record).collect();
    assert!(rows.len() >= 4, "{} events", rows.len());
    for row in &rows {
        schemas.event(row).unwrap_or_else(|e| panic!("{e}\n{row}"));
        assert_eq!(row["match.id"], printed["match.id"]);
        // Every event but a verdict on a queued change carries its commentary line.
        if row["event.type"] != "tactics-change" {
            assert!(row["commentary"].is_string(), "{row}");
        }
    }
    let kinds: Vec<&str> = rows
        .iter()
        .map(|r| r["event.type"].as_str().unwrap())
        .collect();
    assert_eq!(kinds.first(), Some(&"kick-off"));
    assert_eq!(kinds.last(), Some(&"full-time"));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn the_schemas_refuse_a_record_without_its_match_or_with_another_version() {
    let schemas = RecordSchemas::load();
    let data = temp("engine-cli-schemas", "refuse");
    let out = bin(&data)
        .args(["simulate", "--seed", "8", "--minutes", "1", "--no-snapshot"])
        .arg("--ticks-out")
        .arg(data.join("m.ticks"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let good = record(&String::from_utf8_lossy(&out.stdout));
    schemas.stats(&good).unwrap();

    let mut no_match = good.clone();
    no_match.as_object_mut().unwrap().remove("match.id");
    assert!(schemas.stats(&no_match).is_err());

    let mut version_9 = good.clone();
    version_9["schema.version"] = "9".into();
    assert!(schemas.stats(&version_9).is_err());

    let mut wrong_kind = good;
    wrong_kind["record.kind"] = "match-event".into();
    assert!(schemas.stats(&wrong_kind).is_err());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_benchmark_report_validates_against_the_run_report_schema() {
    let data = temp("engine-cli-schemas", "bench");
    let out = bin(&data)
        .args(["bench", "--seed", "42", "--matches", "1", "--minutes", "1"])
        .output()
        .unwrap();
    let report = record(&String::from_utf8_lossy(&out.stdout));
    RecordSchemas::load().report(&report).unwrap();
    assert_eq!(report["operation"], "benchmark");
    let _ = std::fs::remove_dir_all(&data);
}
