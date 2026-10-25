//! Reading the local snapshots. Every file the builder reads goes through [`Snapshot`], which
//! refuses a path outside its root and records each path it opens with its SHA-256, so the
//! coverage record lists every input and a test can prove that nothing else was read. No
//! network request is made: the snapshots were fetched before the build.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, bail};
use sha2::{Digest, Sha256};

use crate::real_targets::csv::{decode, split};

/// A read-only view of one folder that logs every file it opens.
pub struct Snapshot {
    root: PathBuf,
    /// The label each logged path starts with (`data`, `gate`).
    label: &'static str,
    opened: RefCell<BTreeMap<String, String>>,
}

impl Snapshot {
    pub fn new(root: &Path, label: &'static str) -> Self {
        Self {
            root: root.to_path_buf(),
            label,
            opened: RefCell::default(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `rel` under the root; a path that is absolute or climbs out is refused.
    fn path(&self, rel: &str) -> anyhow::Result<PathBuf> {
        let p = Path::new(rel);
        if p.components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
        {
            bail!("{rel} is outside the {} folder", self.label);
        }
        Ok(self.root.join(p))
    }

    /// Whether `rel` exists (nothing is read).
    pub fn exists(&self, rel: &str) -> bool {
        self.path(rel).is_ok_and(|p| p.is_file())
    }

    /// The bytes of `rel`, logged with their SHA-256.
    pub fn read(&self, rel: &str) -> anyhow::Result<Vec<u8>> {
        let path = self.path(rel)?;
        let bytes = std::fs::read(&path)
            .with_context(|| format!("cannot read {} ({})", rel, path.display()))?;
        self.opened.borrow_mut().insert(
            format!("{}:{}", self.label, rel.replace('\\', "/")),
            hex(&Sha256::digest(&bytes)),
        );
        Ok(bytes)
    }

    /// The text of `rel` (UTF-8, else Latin-1), without a byte-order mark.
    pub fn text(&self, rel: &str) -> anyhow::Result<String> {
        let text = decode(&self.read(rel)?);
        Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string())
    }

    /// `rel` as JSON.
    pub fn json(&self, rel: &str) -> anyhow::Result<serde_json::Value> {
        serde_json::from_str(&self.text(rel)?).with_context(|| format!("{rel} is not JSON"))
    }

    /// `rel` as CSV; a row whose field count differs from the header's is refused with its
    /// line (an unterminated quote shows as such a row).
    pub fn csv(&self, rel: &str) -> anyhow::Result<Csv> {
        Csv::parse(&self.text(rel)?).with_context(|| format!("in {rel}"))
    }

    /// The files under the folder `rel`, recursively, as paths relative to the root, sorted;
    /// nothing is opened.
    pub fn files(&self, rel: &str) -> anyhow::Result<Vec<String>> {
        fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
            for e in std::fs::read_dir(dir)? {
                let p = e?.path();
                if p.is_dir() {
                    walk(&p, root, out)?;
                } else if let Ok(r) = p.strip_prefix(root) {
                    out.push(r.to_string_lossy().replace('\\', "/"));
                }
            }
            Ok(())
        }
        let dir = self.path(rel)?;
        let mut out = Vec::new();
        if dir.is_dir() {
            walk(&dir, &self.root, &mut out)
                .with_context(|| format!("cannot list {}", dir.display()))?;
        }
        out.sort();
        Ok(out)
    }

    /// Every path opened so far, with its SHA-256.
    pub fn opened(&self) -> BTreeMap<String, String> {
        self.opened.borrow().clone()
    }
}

/// Lowercase hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A CSV file: its header and its rows, each the header's width.
pub struct Csv {
    columns: HashMap<String, usize>,
    pub rows: Vec<Vec<String>>,
}

impl Csv {
    /// Parses CSV text; blank lines are skipped.
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let mut lines = text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty());
        let Some((_, header)) = lines.next() else {
            bail!("no header line");
        };
        let header = split(header);
        let width = header.len();
        let columns = header
            .into_iter()
            .enumerate()
            .map(|(i, name)| (name.trim().to_string(), i))
            .collect();
        let mut rows = Vec::new();
        for (i, line) in lines {
            let row = split(line);
            if row.len() != width {
                bail!(
                    "line {}: {} fields where the header has {width} (an unterminated quote?)",
                    i + 1,
                    row.len()
                );
            }
            rows.push(row);
        }
        Ok(Self { columns, rows })
    }

    /// Whether the header names `column`.
    pub fn has(&self, column: &str) -> bool {
        self.columns.contains_key(column)
    }

    /// The trimmed field `column` of `row`; empty when the column is absent.
    pub fn get<'a>(&self, row: &'a [String], column: &str) -> &'a str {
        self.columns
            .get(column)
            .and_then(|&i| row.get(i))
            .map_or("", |s| s.trim())
    }

    /// Refuses the file unless its header names every column of `need`.
    pub fn require(&self, file: &str, need: &[&str]) -> anyhow::Result<()> {
        for c in need {
            if !self.has(c) {
                bail!("{file} has no column {c}");
            }
        }
        Ok(())
    }
}

/// One entry of an openfootball `*.leagues.txt` file: its tier and its names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfLeague {
    pub tier: u8,
    pub names: Vec<String>,
}

/// The text of `line` before a `#` comment.
fn uncommented(line: &str) -> &str {
    line.split('#').next().unwrap_or("")
}

/// An openfootball leagues file: the country heading (`= Austria =`) and each tiered entry.
/// An entry line is `<code> [<seasons>] <name> [=> <new name>][, cup]`; an indented `| a | b`
/// line adds aliases to the entry above it. The tier is the leading number of the code.
pub fn parse_of_leagues(text: &str) -> (Option<String>, Vec<OfLeague>) {
    let mut country = None;
    let mut out: Vec<OfLeague> = Vec::new();
    let mut last_tiered = false;
    for raw in text.lines() {
        let line = uncommented(raw);
        if line.trim().is_empty() {
            continue;
        }
        if let Some(h) = line.trim().strip_prefix('=') {
            if country.is_none() {
                country = Some(
                    h.trim_matches(|c: char| c == '=' || c.is_whitespace())
                        .into(),
                );
            }
            continue;
        }
        if raw.starts_with(char::is_whitespace) {
            if last_tiered && let Some(e) = out.last_mut() {
                e.names.extend(
                    line.split('|')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(String::from),
                );
            }
            continue;
        }
        let mut words = line.split_whitespace();
        let code = words.next().unwrap_or("");
        let digits: String = code.chars().take_while(char::is_ascii_digit).collect();
        let tier = digits.parse::<u8>().ok().filter(|t| (1..=20).contains(t));
        last_tiered = tier.is_some();
        let Some(tier) = tier else {
            continue;
        };
        let mut rest: Vec<&str> = words.collect();
        if rest.first().is_some_and(|w| {
            w.chars()
                .all(|c| c.is_ascii_digit() || c == '/' || c == '-')
        }) {
            rest.remove(0);
        }
        let rest = rest.join(" ");
        let names = rest
            .split("=>")
            .map(|n| n.split(',').next().unwrap_or("").trim().to_string())
            .filter(|n| !n.is_empty())
            .collect();
        out.push(OfLeague { tier, names });
    }
    (country, out)
}

/// The names of an openfootball clubs file: each club line's name (before its first comma)
/// and every `|` alias.
pub fn parse_of_clubs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = uncommented(raw);
        let t = line.trim();
        if t.is_empty() || t.starts_with('=') {
            continue;
        }
        if raw.starts_with(char::is_whitespace) || t.starts_with('|') {
            out.extend(
                t.split('|')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(String::from),
            );
        } else if let Some(name) = t.split(',').next().map(str::trim)
            && !name.is_empty()
        {
            out.push(name.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_doubled_quote_and_a_quoted_comma_read_through_the_splitter() {
        let c = Csv::parse("a,b\n\"x, y\",\"say \"\"hi\"\"\"\n").unwrap();
        assert_eq!(c.get(&c.rows[0], "a"), "x, y");
        assert_eq!(c.get(&c.rows[0], "b"), "say \"hi\"");
    }

    #[test]
    fn a_short_row_and_an_unterminated_quote_fail_with_the_line() {
        let e = Csv::parse("a,b,c\n1,2,3\n1,2\n").err().unwrap().to_string();
        assert!(e.contains("line 3"), "{e}");
        let e = Csv::parse("a,b\n\"open,1\n").err().unwrap().to_string();
        assert!(e.contains("line 2"), "{e}");
    }

    #[test]
    fn a_path_outside_the_root_is_refused() {
        let s = Snapshot::new(Path::new("."), "data");
        assert!(s.read("../x.csv").is_err());
        assert!(s.opened().is_empty());
    }

    #[test]
    fn an_openfootball_leagues_file_gives_tiers_names_and_aliases() {
        let text = "= Testland =   # note\n\n1  Top League   # yearly\n   | tl.1 | Top\n\
                    2  2018/19-  Old Two => New Two\n3.o  Third East\ncup  Test Cup, cup\n   | TC\n";
        let (country, l) = parse_of_leagues(text);
        assert_eq!(country.as_deref(), Some("Testland"));
        assert_eq!(l.len(), 3);
        assert_eq!(l[0].names, ["Top League", "tl.1", "Top"]);
        assert_eq!(l[1].names, ["Old Two", "New Two"]);
        assert_eq!((l[2].tier, l[2].names[0].as_str()), (3, "Third East"));
    }

    #[test]
    fn an_openfootball_clubs_file_gives_names_and_aliases() {
        let text = "= Testland\n\nAlpha FC,  Town   # c\n  | Alpha | AFC\nBeta,  Village\n";
        assert_eq!(parse_of_clubs(text), ["Alpha FC", "Alpha", "AFC", "Beta"]);
    }
}
