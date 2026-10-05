//! The formats of the files a re-simulation writes for bisect: the per-tick state digest
//! file, the state fields file of one tick, and the tick key of the debug trace. Each
//! format's writer and reader are here, so `resimulate` and `bisect` share one definition
//! and a change to one side is caught by the round-trip tests below.
//!
//! The state digest file is a JSON header line, then `<tick> <digest>` for every tick, then
//! `finish <digest>` for the state after full time, then `end <tick lines> <full time>`.
//! The state fields file is one JSON object: the tick, its named fields (name, kind, hex
//! bytes), the scheme, and the engine identity.

use std::io::Write;

use engine::gate::{FieldKind, STATE_DIGEST_FORMAT};
use serde::Serialize;

/// The word that starts the digest file's last-but-one line.
const FINISH_WORD: &str = "finish";
/// The word that starts the digest file's last line.
const END_WORD: &str = "end";

/// Writes the header line of a state digest file.
pub fn write_digest_header(
    w: &mut impl Write,
    inventory: impl Serialize,
    gate_schema: impl Serialize,
    scheme: u8,
    engine: &impl Serialize,
) -> std::io::Result<()> {
    let header = serde_json::json!({
        "state_digests": STATE_DIGEST_FORMAT,
        "inventory": inventory,
        "gate_schema": gate_schema,
        "scheme": scheme,
        "engine": engine,
    });
    writeln!(w, "{header}")
}

/// Writes the digest line of one tick.
pub fn write_tick_digest(w: &mut impl Write, tick: u32, digest: &str) -> std::io::Result<()> {
    writeln!(w, "{tick} {digest}")
}

/// Writes the digest line of the state after full time.
pub fn write_finish_digest(w: &mut impl Write, digest: &str) -> std::io::Result<()> {
    writeln!(w, "{FINISH_WORD} {digest}")
}

/// Writes the closing line, with the count of tick lines and the full-time flag.
pub fn write_digest_end(
    w: &mut impl Write,
    tick_lines: u64,
    full_time: bool,
) -> std::io::Result<()> {
    writeln!(w, "{END_WORD} {tick_lines} {full_time}")
}

/// Checks that two builds hash the same state inventory, so their digests compare.
pub fn same_inventory(a: &Digests, b: &Digests) -> Result<(), String> {
    for name in ["state_digests", "inventory"] {
        if a.header[name] != b.header[name] {
            return Err(format!(
                "the two builds hash different state inventories ({name} {} and {})",
                a.header[name], b.header[name]
            ));
        }
    }
    Ok(())
}

/// The text of a state fields file for `tick`: each field as `(name, kind, bytes)`.
pub fn fields_file<'a>(
    tick: u32,
    fields: impl IntoIterator<Item = (&'a str, FieldKind, &'a [u8])>,
    scheme: u8,
    engine: &impl Serialize,
) -> String {
    let fields: Vec<serde_json::Value> = fields
        .into_iter()
        .map(|(name, kind, bytes)| {
            serde_json::json!({
                "name": name,
                "kind": kind.as_str(),
                "hex": stream::record::hex(bytes),
            })
        })
        .collect();
    let mut found = serde_json::json!({ "tick": tick, "fields": fields });
    found["scheme"] = serde_json::json!(scheme);
    found["engine"] = serde_json::json!(engine);
    format!(
        "{found}
"
    )
}

/// A whole state digest file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digests {
    pub header: serde_json::Value,
    /// `(tick, digest)` for every tick, from tick 1 with no gap.
    pub ticks: Vec<(u32, String)>,
    /// The digest of the state after full time.
    pub finish: String,
    /// The `end` line's full-time flag.
    pub full_time: bool,
}

impl Digests {
    /// The stream scheme the build played.
    pub fn scheme(&self) -> u64 {
        self.header["scheme"].as_u64().unwrap_or(0)
    }
}

/// Reads a state digest file. A file with no `end` line, a gap in its ticks, or a line
/// count unlike the `end` line's is refused with the reason.
pub fn parse_digests(text: &str) -> Result<Digests, String> {
    let mut lines = text.lines();
    let header: serde_json::Value = lines
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .ok_or("ends early: the state digest file has no header line")?;
    if header["state_digests"].as_u64() != Some(u64::from(STATE_DIGEST_FORMAT)) {
        return Err(format!(
            "the state digest file is format {}, and this bisect reads format {STATE_DIGEST_FORMAT}",
            header["state_digests"]
        ));
    }
    let mut ticks: Vec<(u32, String)> = Vec::new();
    let mut finish = None;
    let mut end = None;
    for line in lines {
        if end.is_some() {
            return Err("the state digest file has lines after its end line".into());
        }
        let mut parts = line.split(' ');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(END_WORD), Some(count), Some(full_time)) => {
                let count: usize = count
                    .parse()
                    .map_err(|_| format!("a bad end line: {line}"))?;
                end = Some((count, full_time == "true"));
            }
            (Some(FINISH_WORD), Some(digest), None) => finish = Some(digest.to_string()),
            (Some(tick), Some(digest), None) if finish.is_none() => {
                let tick: u32 = tick
                    .parse()
                    .map_err(|_| format!("a bad state digest line: {line}"))?;
                let expected = ticks.last().map_or(1, |(t, _)| t + 1);
                if tick != expected {
                    return Err(format!(
                        "a gap in the state digests: tick {tick} follows tick {}",
                        expected - 1
                    ));
                }
                ticks.push((tick, digest.to_string()));
            }
            _ => return Err(format!("a bad state digest line: {line}")),
        }
    }
    let Some((count, full_time)) = end else {
        return Err(format!(
            "ends early: the state digest file stops after tick {} with no end line",
            ticks.last().map_or(0, |(t, _)| *t)
        ));
    };
    if count != ticks.len() {
        return Err(format!(
            "the state digest file has {} tick lines but its end line counts {count}",
            ticks.len()
        ));
    }
    let finish = finish.ok_or("ends early: the state digest file has no finish line")?;
    Ok(Digests {
        header,
        ticks,
        finish,
        full_time,
    })
}

/// One named part of a tick's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub kind: FieldKind,
    pub bytes: Vec<u8>,
}

/// The fields of a state fields file.
pub fn parse_fields(written: &serde_json::Value) -> Result<Vec<Field>, String> {
    let bad = || "the state fields file is not in the expected form".to_string();
    written["fields"]
        .as_array()
        .ok_or_else(bad)?
        .iter()
        .map(|f| {
            let name = f["name"].as_str().ok_or_else(bad)?.to_string();
            let kind = f["kind"]
                .as_str()
                .and_then(FieldKind::from_name)
                .ok_or_else(bad)?;
            let bytes = unhex(f["hex"].as_str().ok_or_else(bad)?).ok_or_else(bad)?;
            Ok(Field { name, kind, bytes })
        })
        .collect()
}

/// The bytes of lowercase or uppercase hex text.
fn unhex(text: &str) -> Option<Vec<u8>> {
    // `from_str_radix` alone would take a sign (`+f`), so every character is checked first.
    if !text.len().is_multiple_of(2) || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signed_pair_is_not_hex() {
        assert_eq!(unhex("+f"), None);
        assert_eq!(unhex("0a+f"), None);
        assert_eq!(unhex("0aff"), Some(vec![0x0a, 0xff]));
    }

    #[test]
    fn written_digests_read_back() {
        let mut w = Vec::new();
        write_digest_header(&mut w, 1, 1, 1, &serde_json::json!({})).unwrap();
        write_tick_digest(&mut w, 1, "aa").unwrap();
        write_tick_digest(&mut w, 2, "bb").unwrap();
        write_finish_digest(&mut w, "cc").unwrap();
        write_digest_end(&mut w, 2, true).unwrap();
        let d = parse_digests(std::str::from_utf8(&w).unwrap()).unwrap();
        assert_eq!(d.ticks, [(1, "aa".to_string()), (2, "bb".to_string())]);
        assert_eq!(d.finish, "cc");
        assert!(d.full_time);
    }

    #[test]
    fn written_fields_read_back() {
        let text = fields_file(
            7,
            [
                ("a", FieldKind::Floats, &[0u8, 255][..]),
                ("b", FieldKind::Bytes, &[][..]),
            ],
            1,
            &serde_json::json!({}),
        );
        let written: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(written["tick"], 7);
        let fields = parse_fields(&written).unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "a");
        assert_eq!(fields[0].kind, FieldKind::Floats);
        assert_eq!(fields[0].bytes, [0, 255]);
        assert!(fields[1].bytes.is_empty());
    }

    const HEADER: &str =
        r#"{"state_digests":1,"inventory":1,"gate_schema":1,"scheme":1,"engine":{}}"#;

    fn file(body: &str) -> String {
        format!("{HEADER}\n{body}")
    }

    #[test]
    fn a_whole_file_is_read() {
        let d = parse_digests(&file("1 aa\n2 bb\nfinish cc\nend 2 true\n")).unwrap();
        assert_eq!(d.ticks, [(1, "aa".to_string()), (2, "bb".to_string())]);
        assert_eq!(d.finish, "cc");
        assert!(d.full_time);
        assert_eq!(d.scheme(), 1);
    }

    #[test]
    fn a_missing_end_line_is_an_early_end() {
        let err = parse_digests(&file("1 aa\n2 bb\nfinish cc\n")).unwrap_err();
        assert!(err.starts_with("ends early"), "{err}");
        assert!(err.contains("after tick 2"), "{err}");
    }

    #[test]
    fn a_gap_or_a_late_start_is_refused() {
        let err = parse_digests(&file("1 aa\n3 bb\nfinish cc\nend 2 true\n")).unwrap_err();
        assert!(err.contains("gap"), "{err}");
        let err = parse_digests(&file("2 aa\nfinish cc\nend 1 true\n")).unwrap_err();
        assert!(err.contains("gap"), "{err}");
    }

    #[test]
    fn a_count_unlike_the_end_line_is_refused() {
        let err = parse_digests(&file("1 aa\n2 bb\nfinish cc\nend 3 true\n")).unwrap_err();
        assert!(err.contains("end line counts 3"), "{err}");
    }

    #[test]
    fn another_format_or_inventory_is_refused() {
        let other = "{\"state_digests\":2}\nend 0 true\n";
        assert!(parse_digests(other).unwrap_err().contains("format 2"));
        let a = parse_digests(&file("finish cc\nend 0 true\n")).unwrap();
        let mut b = a.clone();
        b.header["inventory"] = serde_json::json!(2);
        let err = same_inventory(&a, &b).unwrap_err();
        assert!(err.contains("different state inventories"), "{err}");
        assert_eq!(same_inventory(&a, &a.clone()), Ok(()));
    }

    #[test]
    fn hex_reads_back() {
        assert_eq!(unhex("00ff10"), Some(vec![0, 255, 16]));
        assert_eq!(unhex("0"), None);
        assert_eq!(unhex("zz"), None);
    }
}
