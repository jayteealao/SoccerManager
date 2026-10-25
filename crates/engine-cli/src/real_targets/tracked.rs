//! The tracked-match targets: each tracked band's real value from the DFL tracking file
//! (`gate/tracking/dfl-matches.json`), measured by the shared shape report.
//!
//! The intake reads only a tracking file whose source is on a closed list (DFL); a
//! SoccerTrack contrast, or any other file, is refused: SoccerTrack data sets no target.

use std::path::Path;

use anyhow::Context;

use crate::tracking::measure::TrackingFile;

/// The sources whose tracked matches may set a target.
pub const PROVIDERS: [&str; 1] = ["DFL"];

/// Reads the tracking file at `path` for the targets, refusing a source off the list.
pub fn read_tracking(path: &Path) -> anyhow::Result<TrackingFile> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read the tracking file {}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("{} is not JSON", path.display()))?;
    let source = value["source"].as_str().unwrap_or("");
    anyhow::ensure!(
        source != crate::tracking::contrast::SOURCE,
        "{}: SoccerTrack data sets no target",
        path.display()
    );
    anyhow::ensure!(
        PROVIDERS.contains(&source),
        "{}: the source {source:?} sets no target; the tracked targets come from {}",
        path.display(),
        PROVIDERS.join(", ")
    );
    TrackingFile::read(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A contrast file fed to the intake is refused, as is a tracking file of a source off
    /// the list.
    #[test]
    fn the_intake_refuses_soccertrack_and_unlisted_sources() {
        let dir = tempfile::TempDir::new().unwrap();
        let contrast = dir.path().join("contrast.json");
        std::fs::write(
            &contrast,
            serde_json::json!({ "schema": 1, "source": "SoccerTrack", "amateur": {} }).to_string(),
        )
        .unwrap();
        let err = read_tracking(&contrast).unwrap_err().to_string();
        assert!(err.contains("SoccerTrack data sets no target"), "{err}");
        let other = dir.path().join("other.json");
        std::fs::write(
            &other,
            serde_json::json!({ "schema": 1, "source": "Metrica", "matches": [] }).to_string(),
        )
        .unwrap();
        let err = read_tracking(&other).unwrap_err().to_string();
        assert!(err.contains("sets no target"), "{err}");
    }
}
