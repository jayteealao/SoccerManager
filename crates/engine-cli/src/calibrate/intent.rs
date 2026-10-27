//! The intent of a change run: what the developer says the change should move, stated before
//! any match plays. It names bands and the way each should move (`up` or `down`, the band's
//! value), optionally by how much; later it also carries one note per surprise and the ids of
//! the draft setting rules the developer confirms.
//!
//! The intent comes from `--intent <FILE>`, from the run folder's own `intent.json` when the
//! run is resumed or judged again, or from a prompt on standard error when standard input is a
//! terminal. A change run without any of them stops before any match plays. The bands and
//! directions of a played run never change; only its notes and confirmations do.

use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::report::bands::Registry;

/// The intent file's name inside a run folder.
pub const INTENT_FILE: &str = "intent.json";
/// The version of the intent file this build reads and writes.
pub const INTENT_VERSION: u32 = 1;

/// The stop of a change run that has no intent and no terminal to ask at.
pub const MISSING: &str = "a change run needs its intent: give --intent <FILE> or run it at a \
                           terminal; no match was played";

/// Which way a band's value should move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
}

impl Direction {
    pub fn code(self) -> &'static str {
        match self {
            Direction::Up => "up",
            Direction::Down => "down",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "up" => Some(Direction::Up),
            "down" => Some(Direction::Down),
            _ => None,
        }
    }
}

/// One band the change should move.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntentBand {
    pub band: String,
    pub direction: Direction,
    /// How far the developer expects it to move, in the band's unit; only reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
}

/// What a note says a surprise is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteKind {
    #[serde(rename = "side effect")]
    SideEffect,
    #[serde(rename = "bug")]
    Bug,
}

impl NoteKind {
    pub fn code(self) -> &'static str {
        match self {
            NoteKind::SideEffect => "side effect",
            NoteKind::Bug => "bug",
        }
    }
}

/// The developer's one line on a surprise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub band: String,
    pub kind: NoteKind,
    pub note: String,
    /// The content setting a side effect comes from, `<file>#/<pointer>`; needed when the
    /// change touched more than one setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
}

/// The intent file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    pub schema_version: u32,
    /// One line on what the change does.
    #[serde(default)]
    pub change: String,
    /// The bands the change should move; none means it should move no band.
    #[serde(default)]
    pub bands: Vec<IntentBand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
    /// The ids of draft setting rules the developer confirms.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub confirm: Vec<String>,
}

impl Intent {
    /// An intent that names no band: the change should move nothing.
    pub fn empty() -> Self {
        Self {
            schema_version: INTENT_VERSION,
            change: String::new(),
            bands: Vec::new(),
            notes: Vec::new(),
            confirm: Vec::new(),
        }
    }

    /// The intended direction of `band`, when the intent names it.
    pub fn direction_of(&self, band: &str) -> Option<&IntentBand> {
        self.bands.iter().find(|b| b.band == band)
    }

    /// The note on `band`, when there is one.
    pub fn note_on(&self, band: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.band == band)
    }

    /// Refuses an intent that names a band the band file does not hold, a band twice, a
    /// blank note, or a blank confirmation, naming the field.
    pub fn validate(&self, registry: &Registry) -> anyhow::Result<()> {
        if self.schema_version != INTENT_VERSION {
            anyhow::bail!(
                "intent: schema_version {} is not {INTENT_VERSION}, the version this build reads",
                self.schema_version
            );
        }
        let known = |field: String, band: &str| -> anyhow::Result<()> {
            if registry.band(band).is_none() {
                anyhow::bail!(
                    "intent: {field}: {band} is not a band of the band file{}",
                    closest_hint(registry, band)
                );
            }
            Ok(())
        };
        for (i, b) in self.bands.iter().enumerate() {
            known(format!("bands[{i}].band"), &b.band)?;
            if self.bands[..i].iter().any(|o| o.band == b.band) {
                anyhow::bail!("intent: bands[{i}].band: {} is named twice", b.band);
            }
            if b.size.is_some_and(|s| !s.is_finite()) {
                anyhow::bail!("intent: bands[{i}].size: not a number");
            }
        }
        for (i, n) in self.notes.iter().enumerate() {
            known(format!("notes[{i}].band"), &n.band)?;
            if self.notes[..i].iter().any(|o| o.band == n.band) {
                anyhow::bail!("intent: notes[{i}].band: {} has a second note", n.band);
            }
            if n.note.trim().is_empty() {
                anyhow::bail!("intent: notes[{i}].note: the note on {} is blank", n.band);
            }
            if n.setting.as_deref().is_some_and(|s| !s.contains("#/")) {
                anyhow::bail!(
                    "intent: notes[{i}].setting: name it as <file>#/<pointer>, such as \
                     tuning.json#/engine/foul_base"
                );
            }
        }
        for (i, id) in self.confirm.iter().enumerate() {
            if id.trim().is_empty() {
                anyhow::bail!("intent: confirm[{i}]: blank");
            }
        }
        Ok(())
    }

    /// SHA-256, as hex, over the bands and their directions: what a played run keeps fixed.
    pub fn bands_digest(&self) -> String {
        let mut bands: Vec<(&str, &str)> = self
            .bands
            .iter()
            .map(|b| (b.band.as_str(), b.direction.code()))
            .collect();
        bands.sort_unstable();
        let mut h = Sha256::new();
        for (band, dir) in bands {
            h.update(band.as_bytes());
            h.update(b"\0");
            h.update(dir.as_bytes());
            h.update(b"\n");
        }
        format!("{:x}", h.finalize())
    }
}

/// Reads and checks the intent file at `path`.
pub fn load(path: &Path, registry: &Registry) -> anyhow::Result<Intent> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("cannot read the intent file {}", path.display()))?;
    let intent: Intent = serde_json::from_slice(&bytes)
        .with_context(|| format!("the intent file {} is refused", path.display()))?;
    intent
        .validate(registry)
        .with_context(|| format!("the intent file {} is refused", path.display()))?;
    Ok(intent)
}

/// The intent stored in `run_dir`, when it holds one.
pub fn stored(run_dir: &Path, registry: &Registry) -> anyhow::Result<Option<Intent>> {
    let path = run_dir.join(INTENT_FILE);
    if !path.is_file() {
        return Ok(None);
    }
    load(&path, registry).map(Some)
}

/// `stored`, the intent of a played run, with the notes and confirmations of `given`. The
/// bands and directions of a played run cannot change.
pub fn merge_notes(stored: &Intent, given: &Intent) -> anyhow::Result<Intent> {
    if stored.bands_digest() != given.bands_digest() {
        let list = |i: &Intent| {
            let mut v: Vec<String> = i
                .bands
                .iter()
                .map(|b| format!("{} {}", b.band, b.direction.code()))
                .collect();
            v.sort();
            if v.is_empty() {
                "no band".to_string()
            } else {
                v.join(", ")
            }
        };
        anyhow::bail!(
            "the intent of a played run cannot change; only notes and confirmations: the run \
             holds {}, and --intent names {}",
            list(stored),
            list(given)
        );
    }
    Ok(Intent {
        notes: given.notes.clone(),
        confirm: given.confirm.clone(),
        ..stored.clone()
    })
}

/// Where the run's intent came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// `--intent <FILE>`.
    File,
    /// The run folder's own `intent.json`.
    Folder,
    /// The prompt at a terminal.
    Prompt,
}

impl Source {
    pub fn code(self) -> &'static str {
        match self {
            Source::File => "file",
            Source::Folder => "run folder",
            Source::Prompt => "prompt",
        }
    }
}

/// The run's intent: `given` (`--intent`), merged onto the run folder's `stored` intent when
/// there is one; else `stored`; else the prompt when standard input is a `terminal`; else the
/// run stops with nothing played.
pub fn resolve(
    given: Option<&Path>,
    stored: Option<Intent>,
    terminal: bool,
    registry: &Registry,
    prompt: impl FnOnce() -> anyhow::Result<Intent>,
) -> anyhow::Result<(Intent, Source)> {
    match (given, stored) {
        (Some(path), Some(stored)) => {
            Ok((merge_notes(&stored, &load(path, registry)?)?, Source::File))
        }
        (Some(path), None) => Ok((load(path, registry)?, Source::File)),
        (None, Some(stored)) => Ok((stored, Source::Folder)),
        (None, None) if terminal => Ok((prompt()?, Source::Prompt)),
        (None, None) => anyhow::bail!("{MISSING}"),
    }
}

/// Asks for the intent on `writer`, reading the answers from `reader`: one line on what the
/// change does, then one band per line, `<band> up|down [size]`, until an empty line. A line
/// that names no band of the band file or no direction is asked again. The answers are shown
/// before anything else happens.
pub fn prompt(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    registry: &Registry,
) -> anyhow::Result<Intent> {
    let mut line = String::new();
    let mut read = |line: &mut String| -> anyhow::Result<bool> {
        line.clear();
        Ok(reader.read_line(line).context("cannot read the answer")? > 0)
    };
    writeln!(writer, "What does this change do? (one line)")?;
    let change = if read(&mut line)? {
        line.trim().to_string()
    } else {
        String::new()
    };
    writeln!(
        writer,
        "Which bands should it move? One per line: <band> up|down [size]; an empty line ends"
    )?;
    let mut bands: Vec<IntentBand> = Vec::new();
    while read(&mut line)? {
        let text = line.trim();
        if text.is_empty() {
            break;
        }
        match parse_band(text, registry) {
            Ok(b) if bands.iter().any(|o| o.band == b.band) => {
                writeln!(writer, "{} is named already; give another band", b.band)?;
            }
            Ok(b) => bands.push(b),
            Err(why) => writeln!(writer, "{why}; give it again")?,
        }
    }
    let intent = Intent {
        change,
        bands,
        ..Intent::empty()
    };
    writeln!(writer, "{}", summary(&intent))?;
    Ok(intent)
}

/// One answer line, `<band> up|down [size]`.
fn parse_band(text: &str, registry: &Registry) -> Result<IntentBand, String> {
    let mut parts = text.split_whitespace();
    let band = parts.next().unwrap_or_default();
    if registry.band(band).is_none() {
        return Err(format!("{band} is not a band of the band file"));
    }
    let Some(direction) = parts.next().and_then(Direction::parse) else {
        return Err(format!("{band}: say up or down"));
    };
    let size = match parts.next() {
        None => None,
        Some(s) => Some(
            s.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("{band}: the size {s} is not a number"))?,
        ),
    };
    if parts.next().is_some() {
        return Err(format!(
            "{band}: name a band, up or down, and at most a size"
        ));
    }
    Ok(IntentBand {
        band: band.to_string(),
        direction,
        size,
    })
}

/// One line that shows the intent.
pub fn summary(intent: &Intent) -> String {
    let bands = if intent.bands.is_empty() {
        "no band should move".to_string()
    } else {
        intent
            .bands
            .iter()
            .map(|b| match b.size {
                Some(s) => format!("{} {} by {s}", b.band, b.direction.code()),
                None => format!("{} {}", b.band, b.direction.code()),
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    if intent.change.is_empty() {
        format!("intent: {bands}")
    } else {
        format!("intent: {}: {bands}", intent.change)
    }
}

/// Writes `intent` into `run_dir` as `intent.json` and returns the file's SHA-256, as hex.
pub fn write_into(run_dir: &Path, intent: &Intent) -> anyhow::Result<String> {
    let mut text = serde_json::to_string_pretty(intent)?;
    text.push('\n');
    let path = run_dir.join(INTENT_FILE);
    std::fs::write(&path, &text).with_context(|| format!("cannot write {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(text.as_bytes())))
}

/// "; did you mean NAME?" for the band name closest to `band` by edit distance, when one
/// is within three edits; empty otherwise.
fn closest_hint(registry: &Registry, band: &str) -> String {
    registry
        .names()
        .into_iter()
        .map(|name| (edit_distance(band, name), name))
        .filter(|(d, _)| *d <= 3)
        .min()
        .map_or_else(String::new, |(_, name)| format!("; did you mean {name}?"))
}

/// The Levenshtein distance between `a` and `b`, by characters.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diag = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if ca == cb {
                diag
            } else {
                1 + diag.min(above).min(row[j])
            };
            diag = above;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        Registry::load(&engine::ContentDir::at(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        ))
        .unwrap()
    }

    fn ask(input: &str) -> (Intent, String) {
        let mut out = Vec::new();
        let intent = prompt(
            &mut std::io::Cursor::new(input.as_bytes()),
            &mut out,
            &registry(),
        )
        .unwrap();
        (intent, String::from_utf8(out).unwrap())
    }

    #[test]
    fn two_bands_and_a_blank_line_give_that_intent() {
        let (intent, shown) =
            ask("harder pressing\nfouls_per_team up 0.5\ngoals_per_match down\n\nignored up\n");
        assert_eq!(intent.change, "harder pressing");
        assert_eq!(
            intent.bands,
            vec![
                IntentBand {
                    band: "fouls_per_team".into(),
                    direction: Direction::Up,
                    size: Some(0.5)
                },
                IntentBand {
                    band: "goals_per_match".into(),
                    direction: Direction::Down,
                    size: None
                },
            ]
        );
        assert!(
            shown.starts_with("What does this change do? (one line)\n"),
            "{shown}"
        );
        assert!(shown.contains("an empty line ends"), "{shown}");
        assert!(
            shown.ends_with(
                "intent: harder pressing: fouls_per_team up by 0.5, goals_per_match down\n"
            ),
            "{shown}"
        );
    }

    #[test]
    fn a_mistyped_band_names_the_closest_band() {
        let reg = registry();
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(
            closest_hint(&reg, "goals_per_mach"),
            "; did you mean goals_per_match?"
        );
        assert_eq!(closest_hint(&reg, "nothing_like_any_band_name"), "");
    }

    #[test]
    fn a_bad_line_is_asked_again() {
        let (intent, shown) = ask(
            "x\nnot_a_band up\nfouls_per_team sideways\nfouls_per_team up\nfouls_per_team down\n\n",
        );
        assert_eq!(intent.bands.len(), 1);
        assert_eq!(intent.bands[0].direction, Direction::Up);
        assert!(
            shown.contains("not_a_band is not a band of the band file; give it again"),
            "{shown}"
        );
        assert!(shown.contains("fouls_per_team: say up or down"), "{shown}");
        assert!(shown.contains("fouls_per_team is named already"), "{shown}");
    }

    #[test]
    fn the_end_of_input_ends_the_prompt_with_no_band() {
        let (intent, shown) = ask("");
        assert!(intent.bands.is_empty());
        assert!(shown.ends_with("intent: no band should move\n"), "{shown}");
    }

    #[test]
    fn a_distribution_band_takes_up_or_down() {
        let r = registry();
        let dist = r
            .names()
            .into_iter()
            .find(|n| r.band(n).unwrap().is_distribution())
            .unwrap()
            .to_string();
        let (intent, _) = ask(&format!("x\n{dist} down\n\n"));
        assert_eq!(intent.bands[0].band, dist);
    }

    fn intent(json: serde_json::Value) -> anyhow::Result<Intent> {
        let i: Intent = serde_json::from_value(json)?;
        i.validate(&registry())?;
        Ok(i)
    }

    #[test]
    fn refusals_name_the_field() {
        let e = |j: serde_json::Value| format!("{:#}", intent(j).unwrap_err());
        let base = |extra: serde_json::Value| {
            let mut v = serde_json::json!({"schema_version": 1, "bands": []});
            for (k, x) in extra.as_object().unwrap() {
                v[k] = x.clone();
            }
            v
        };
        assert!(
            e(base(
                serde_json::json!({"bands": [{"band": "nope", "direction": "up"}]})
            ))
            .contains("bands[0].band: nope is not a band")
        );
        let twice = serde_json::json!({"bands": [
            {"band": "fouls_per_team", "direction": "up"},
            {"band": "fouls_per_team", "direction": "down"}]});
        assert!(e(base(twice)).contains("bands[1].band: fouls_per_team is named twice"));
        let blank = serde_json::json!({"notes": [
            {"band": "fouls_per_team", "kind": "bug", "note": "  "}]});
        assert!(e(base(blank)).contains("notes[0].note"));
        let kind = serde_json::json!({"notes": [
            {"band": "fouls_per_team", "kind": "typo", "note": "x"}]});
        assert!(intent(base(kind)).is_err());
        assert!(e(serde_json::json!({"schema_version": 2})).contains("schema_version 2"));
        assert!(intent(base(serde_json::json!({"extra": 1}))).is_err());
        let setting = serde_json::json!({"notes": [
            {"band": "fouls_per_team", "kind": "side effect", "note": "x", "setting": "foul"}]});
        assert!(e(base(setting)).contains("notes[0].setting"));
    }

    #[test]
    fn an_intent_with_no_band_is_valid() {
        let i = intent(serde_json::json!({"schema_version": 1})).unwrap();
        assert!(i.bands.is_empty());
        assert_eq!(i.bands_digest(), Intent::empty().bands_digest());
    }

    #[test]
    fn notes_and_confirmations_merge_and_a_changed_direction_is_refused() {
        let stored = intent(serde_json::json!({"schema_version": 1, "change": "c",
            "bands": [{"band": "fouls_per_team", "direction": "up"}]}))
        .unwrap();
        let notes = intent(serde_json::json!({"schema_version": 1,
            "bands": [{"band": "fouls_per_team", "direction": "up", "size": 1.0}],
            "notes": [{"band": "goals_per_match", "kind": "bug", "note": "a bug"}],
            "confirm": ["sr-1234abcd"]}))
        .unwrap();
        let merged = merge_notes(&stored, &notes).unwrap();
        assert_eq!(merged.change, "c");
        assert_eq!(merged.bands, stored.bands);
        assert_eq!(merged.notes, notes.notes);
        assert_eq!(merged.confirm, vec!["sr-1234abcd".to_string()]);
        let turned = intent(serde_json::json!({"schema_version": 1,
            "bands": [{"band": "fouls_per_team", "direction": "down"}]}))
        .unwrap();
        let e = format!("{:#}", merge_notes(&stored, &turned).unwrap_err());
        assert!(
            e.contains("cannot change; only notes and confirmations"),
            "{e}"
        );
        assert!(
            e.contains("fouls_per_team up") && e.contains("fouls_per_team down"),
            "{e}"
        );
    }

    #[test]
    fn without_a_file_a_folder_or_a_terminal_the_run_stops() {
        let r = registry();
        let e = resolve(None, None, false, &r, || panic!("no prompt")).unwrap_err();
        assert_eq!(e.to_string(), MISSING);
        let (i, s) = resolve(None, None, true, &r, || Ok(Intent::empty())).unwrap();
        assert_eq!((i, s), (Intent::empty(), Source::Prompt));
        let (_, s) = resolve(None, Some(Intent::empty()), true, &r, || {
            panic!("no prompt")
        })
        .unwrap();
        assert_eq!(s, Source::Folder);
    }

    #[test]
    fn the_written_file_reads_back() {
        let dir = tempfile::TempDir::new().unwrap();
        let i = Intent {
            change: "c".into(),
            bands: vec![IntentBand {
                band: "fouls_per_team".into(),
                direction: Direction::Up,
                size: None,
            }],
            ..Intent::empty()
        };
        let sha = write_into(dir.path(), &i).unwrap();
        assert_eq!(sha.len(), 64);
        let back = stored(dir.path(), &registry()).unwrap().unwrap();
        assert_eq!(back, i);
        assert!(
            stored(&dir.path().join("none"), &registry())
                .unwrap()
                .is_none()
        );
    }
}
