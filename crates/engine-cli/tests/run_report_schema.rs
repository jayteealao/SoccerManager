//! The run report schema keeps its required keys for every report version after the first:
//! a calibrate report of the versions the engine writes today without its registry and its
//! stages is refused for them by name, as a version 2 one always was; a version 1 report
//! never needed them.

mod common;

use common::RecordSchemas;
use serde_json::json;

#[test]
fn a_calibrate_report_of_any_later_version_needs_its_registry_and_stages() {
    let schemas = RecordSchemas::load();
    let refusal = |version: &str| {
        let report = json!({
            "record.kind": "run-report",
            "schema.version": version,
            "owner.id": "0123456789abcdef0123456789abcdef",
            "service": "engine-cli",
            "version": "0.0.0",
            "build.hash": "test",
            "env": "test",
            "operation": "calibrate",
            "run.id": "run",
            "seed": 9,
            "content.hash": "test",
            "outcome": "success",
            "machine.hash": "0123456789ab",
        });
        schemas.report(&report).err().unwrap_or_default()
    };
    assert!(!refusal("1").contains("calib.registry"), "{}", refusal("1"));
    for version in ["2", "7", "8", "9", "10"] {
        let text = refusal(version);
        assert!(
            text.contains("calib.registry") && text.contains("calib.stages"),
            "version {version}: {text}"
        );
    }
}
