//! The bisect report: both builds, both stream schemes, the first tick whose state differs,
//! each named part of the state that differs with both values, and both debug trace
//! excerpts for that tick. Text by default, one JSON object with `--json`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use engine::gate::FieldKind;
use serde_json::{Value, json};

use super::runs::Field;

/// One build as the report names it.
#[derive(Debug, Clone)]
pub struct Build {
    /// `a` or `b`.
    pub side: &'static str,
    /// The ref or the path it was given as.
    pub given: String,
    /// `built`, `reused`, or `ready binary`.
    pub source: &'static str,
    /// The engine identity the build reported in its state digest file.
    pub engine: Value,
    pub scheme: u64,
}

impl Build {
    fn line(&self) -> String {
        let e = &self.engine;
        format!(
            "{}: {} = commit {}{}, executable sha256 {}, {}",
            self.side,
            self.given,
            e["commit"].as_str().unwrap_or("unknown"),
            if e["dirty"].as_bool() == Some(true) {
                " (dirty)"
            } else {
                ""
            },
            e["executable_sha256"].as_str().unwrap_or("unknown"),
            self.source
        )
    }

    fn json(&self) -> Value {
        json!({
            "given": self.given,
            "source": self.source,
            "scheme": self.scheme,
            "engine": self.engine,
        })
    }
}

/// A named part of the state whose values differ.
#[derive(Debug, Clone, PartialEq)]
pub struct Differing {
    pub name: String,
    pub kind: Option<FieldKind>,
    /// Both values, decoded by kind; `null` where one build lacks the part.
    pub values: [Value; 2],
}

/// Every part whose bytes differ between the two lists, in the order of `a`, then the parts
/// only `b` has.
pub fn differing(a: &[Field], b: &[Field]) -> Vec<Differing> {
    let find = |list: &[Field], name: &str| list.iter().find(|f| f.name == name).cloned();
    let mut out = Vec::new();
    for fa in a {
        match find(b, &fa.name) {
            Some(fb) if fb.bytes == fa.bytes => {}
            Some(fb) if fa.kind == FieldKind::Streams && fb.kind == FieldKind::Streams => {
                out.push(Differing {
                    name: fa.name.clone(),
                    kind: Some(FieldKind::Streams),
                    values: streams_diff(&fa.bytes, &fb.bytes),
                });
            }
            Some(fb) => out.push(Differing {
                name: fa.name.clone(),
                kind: Some(fa.kind),
                values: [decode(fa.kind, &fa.bytes), decode(fb.kind, &fb.bytes)],
            }),
            None => out.push(Differing {
                name: fa.name.clone(),
                kind: Some(fa.kind),
                values: [decode(fa.kind, &fa.bytes), Value::Null],
            }),
        }
    }
    for fb in b.iter().filter(|fb| find(a, &fb.name).is_none()) {
        out.push(Differing {
            name: fb.name.clone(),
            kind: Some(fb.kind),
            values: [Value::Null, decode(fb.kind, &fb.bytes)],
        });
    }
    out
}

/// A part's value: floats as numbers (a non-finite one as its text), anything else as hex.
fn decode(kind: FieldKind, bytes: &[u8]) -> Value {
    match kind {
        FieldKind::Floats if bytes.len().is_multiple_of(8) => Value::Array(
            bytes
                .chunks_exact(8)
                .map(|c| {
                    let v = f64::from_le_bytes(c.try_into().expect("8 bytes"));
                    serde_json::Number::from_f64(v)
                        .map_or_else(|| json!(format!("{v:?}")), Value::Number)
                })
                .collect(),
        ),
        _ => json!(stream::record::hex(bytes)),
    }
}

/// The stream state bytes: the scheme, then each stream id and its word position.
fn streams(bytes: &[u8]) -> Option<(u8, BTreeMap<u64, u128>)> {
    let scheme = *bytes.first()?;
    let count = u32::from_le_bytes(bytes.get(1..5)?.try_into().ok()?) as usize;
    let mut all = BTreeMap::new();
    for i in 0..count {
        let at = 5 + i * 24;
        let id = u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?);
        let pos = u128::from_le_bytes(bytes.get(at + 8..at + 24)?.try_into().ok()?);
        all.insert(id, pos);
    }
    (bytes.len() == 5 + count * 24).then_some((scheme, all))
}

/// Both sides of a stream state that differs: the scheme when it differs, and each stream
/// id whose word position differs or that one side lacks.
fn streams_diff(a: &[u8], b: &[u8]) -> [Value; 2] {
    let (Some((sa, ma)), Some((sb, mb))) = (streams(a), streams(b)) else {
        return [json!(stream::record::hex(a)), json!(stream::record::hex(b))];
    };
    let mut va = serde_json::Map::new();
    let mut vb = serde_json::Map::new();
    if sa != sb {
        va.insert("scheme".into(), json!(sa));
        vb.insert("scheme".into(), json!(sb));
    }
    let ids: std::collections::BTreeSet<u64> = ma.keys().chain(mb.keys()).copied().collect();
    for id in ids {
        let (pa, pb) = (ma.get(&id), mb.get(&id));
        if pa != pb {
            let key = format!("stream {id:#018x}");
            va.insert(
                key.clone(),
                pa.map_or(Value::Null, |p| json!(p.to_string())),
            );
            vb.insert(key, pb.map_or(Value::Null, |p| json!(p.to_string())));
        }
    }
    [Value::Object(va), Value::Object(vb)]
}

/// What bisect found.
#[derive(Debug, Clone)]
pub enum Found {
    /// Every tick and the state after full time are the same.
    Same { ticks: usize },
    /// The first tick whose state differs.
    Differs {
        tick: u32,
        fields: Vec<Differing>,
        /// Both tick counts, when the states agree on every tick both played.
        length: Option<[usize; 2]>,
        /// `true` when only the state after full time differs.
        full_time: bool,
        traces: [Vec<Value>; 2],
    },
}

/// The report of a complete comparison.
pub struct Report {
    pub fixture: String,
    pub builds: [Build; 2],
    pub found: Found,
}

impl Report {
    /// The note for two stream schemes, if they differ.
    fn scheme_note(&self) -> Option<String> {
        let [a, b] = &self.builds;
        (a.scheme != b.scheme).then(|| {
            format!(
                "the stream scheme differs ({} and {}): stream positions differ from the first draw",
                a.scheme, b.scheme
            )
        })
    }

    pub fn text(&self) -> String {
        let mut s = String::new();
        match &self.found {
            Found::Same { ticks } => {
                let _ = writeln!(
                    s,
                    "bisect: no difference: the state is the same after each of {ticks} ticks and after full time"
                );
            }
            Found::Differs { tick, .. } => {
                let _ = writeln!(s, "bisect: differs at tick {tick}");
            }
        }
        let _ = writeln!(s, "replay file: {}", self.fixture);
        for build in &self.builds {
            let _ = writeln!(s, "{}", build.line());
        }
        let _ = writeln!(
            s,
            "stream scheme: {} (a), {} (b)",
            self.builds[0].scheme, self.builds[1].scheme
        );
        if let Some(note) = self.scheme_note() {
            let _ = writeln!(s, "note: {note}");
        }
        let Found::Differs {
            tick,
            fields,
            length,
            full_time,
            traces,
        } = &self.found
        else {
            return s;
        };
        let _ = writeln!(s, "first differing tick: {tick}");
        if let Some([a, b]) = length {
            let _ = writeln!(
                s,
                "differing fields:\n  match length\n    a: {a} ticks\n    b: {b} ticks"
            );
        } else if *full_time {
            let _ = writeln!(
                s,
                "differing fields:\n  full time: every tick is the same; the state after full time differs"
            );
        } else {
            let _ = writeln!(s, "differing fields:");
            for f in fields {
                let _ = writeln!(
                    s,
                    "  {}\n    a: {}\n    b: {}",
                    f.name, f.values[0], f.values[1]
                );
            }
        }
        for (side, records) in ["a", "b"].iter().zip(traces) {
            let _ = writeln!(
                s,
                "debug trace of {side} at tick {tick}: {} records",
                records.len()
            );
            for r in records {
                let _ = writeln!(s, "  {r}");
            }
        }
        s
    }

    pub fn json(&self) -> Value {
        let mut out = json!({
            "fixture": self.fixture,
            "a": self.builds[0].json(),
            "b": self.builds[1].json(),
        });
        if let Some(note) = self.scheme_note() {
            out["scheme_note"] = json!(note);
        }
        match &self.found {
            Found::Same { ticks } => {
                out["verdict"] = json!("no difference");
                out["ticks"] = json!(ticks);
            }
            Found::Differs {
                tick,
                fields,
                length,
                full_time,
                traces,
            } => {
                out["verdict"] = json!("differs");
                out["tick"] = json!(tick);
                let mut list: Vec<Value> = fields
                    .iter()
                    .map(|f| {
                        json!({
                            "name": f.name,
                            "kind": f.kind.map(FieldKind::as_str),
                            "a": f.values[0],
                            "b": f.values[1],
                        })
                    })
                    .collect();
                if let Some([a, b]) = length {
                    list = vec![json!({ "name": "match length", "a": a, "b": b })];
                } else if *full_time {
                    list = vec![json!({ "name": "full time" })];
                }
                out["fields"] = Value::Array(list);
                out["trace"] = json!({ "a": traces[0], "b": traces[1] });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str, kind: FieldKind, bytes: Vec<u8>) -> Field {
        Field {
            name: name.into(),
            kind,
            bytes,
        }
    }

    fn floats(v: &[f64]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn stream_bytes(scheme: u8, streams: &[(u64, u128)]) -> Vec<u8> {
        let mut out = vec![scheme];
        out.extend((streams.len() as u32).to_le_bytes());
        for (id, pos) in streams {
            out.extend(id.to_le_bytes());
            out.extend(pos.to_le_bytes());
        }
        out
    }

    #[test]
    fn only_the_differing_fields_are_listed_with_decoded_values() {
        let a = [
            field("tick", FieldKind::Bytes, vec![1, 0, 0, 0]),
            field("ball.vel", FieldKind::Floats, floats(&[1.0, 2.0, 0.0])),
            field(
                "streams",
                FieldKind::Streams,
                stream_bytes(1, &[(7, 10), (9, 20)]),
            ),
        ];
        let b = [
            field("tick", FieldKind::Bytes, vec![1, 0, 0, 0]),
            field("ball.vel", FieldKind::Floats, floats(&[1.5, 2.0, 0.0])),
            field(
                "streams",
                FieldKind::Streams,
                stream_bytes(1, &[(7, 10), (9, 21), (11, 1)]),
            ),
            field("extra", FieldKind::Bytes, vec![0xab]),
        ];
        let d = differing(&a, &b);
        let names: Vec<&str> = d.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["ball.vel", "streams", "extra"]);
        assert_eq!(
            d[0].values,
            [json!([1.0, 2.0, 0.0]), json!([1.5, 2.0, 0.0])]
        );
        assert_eq!(
            d[1].values,
            [
                json!({ "stream 0x0000000000000009": "20", "stream 0x000000000000000b": null }),
                json!({ "stream 0x0000000000000009": "21", "stream 0x000000000000000b": "1" }),
            ]
        );
        assert_eq!(d[2].values, [Value::Null, json!("ab")]);
    }

    #[test]
    fn two_schemes_are_named_in_the_report() {
        let build = |side, scheme| Build {
            side,
            given: "HEAD".into(),
            source: "built",
            engine: json!({ "commit": "abc", "dirty": false, "executable_sha256": "00" }),
            scheme,
        };
        let report = Report {
            fixture: "m.smfx".into(),
            builds: [build("a", 1), build("b", 2)],
            found: Found::Same { ticks: 3 },
        };
        let text = report.text();
        assert!(
            text.contains("the stream scheme differs (1 and 2)"),
            "{text}"
        );
        assert!(
            text.contains("a: HEAD = commit abc, executable sha256 00, built"),
            "{text}"
        );
        assert_eq!(report.json()["verdict"], "no difference");
    }
}
