//! A small CSV reader for the raw files: a header map and a line splitter with double-quote
//! handling. football-data.co.uk files have no quoted field; FotMob files quote every field.
//! A file that is not UTF-8 (one football-data.co.uk file is Latin-1) is read as Latin-1.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;

/// A CSV file: its header and its rows, each split into fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    columns: HashMap<String, usize>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    /// Reads `path`: a byte-order mark is dropped, blank lines are skipped.
    pub fn read(path: &Path) -> anyhow::Result<Self> {
        let bytes =
            std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
        Self::parse(&decode(&bytes)).with_context(|| format!("{} has no header", path.display()))
    }

    /// Parses CSV text; `None` when it has no header line.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());
        let header = split(lines.next()?);
        let columns = header
            .into_iter()
            .enumerate()
            .map(|(i, name)| (name.trim().to_string(), i))
            .collect();
        let rows = lines.map(split).collect();
        Some(Self { columns, rows })
    }

    /// The trimmed field `column` of `row`; empty when the column or the field is absent.
    pub fn get<'a>(&self, row: &'a [String], column: &str) -> &'a str {
        self.columns
            .get(column)
            .and_then(|&i| row.get(i))
            .map_or("", |s| s.trim())
    }

    /// The field `column` of `row` as a number; `None` when it is empty or not a number.
    pub fn num(&self, row: &[String], column: &str) -> Option<f64> {
        self.get(row, column).parse().ok()
    }
}

/// UTF-8 text, or the bytes as Latin-1 when they are not UTF-8.
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

/// Splits one line at commas outside double quotes; a doubled quote inside quotes is one
/// quote.
pub fn split(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut field)),
            '\r' if !quoted => {}
            c => field.push(c),
        }
    }
    out.push(field);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_football_data_line_splits_at_every_comma() {
        let t = Table::parse("\u{feff}Div,Date,HomeTeam,AwayTeam,FTHG,FTAG\r\nE0,16/08/2024,Man United,Fulham,1,0\r\n\r\n")
            .unwrap();
        assert_eq!(t.rows.len(), 1);
        assert_eq!(t.get(&t.rows[0], "HomeTeam"), "Man United");
        assert_eq!(t.num(&t.rows[0], "FTHG"), Some(1.0));
        assert_eq!(t.num(&t.rows[0], "HS"), None);
    }

    #[test]
    fn a_quoted_fotmob_line_keeps_its_commas_and_empty_fields() {
        assert_eq!(
            split(r#""Brighton, Hove","86.1","","","512.4""#),
            ["Brighton, Hove", "86.1", "", "", "512.4"]
        );
        assert_eq!(split(r#""say ""hi""",x"#), [r#"say "hi""#, "x"]);
    }

    #[test]
    fn latin_1_bytes_are_read_as_latin_1() {
        assert_eq!(decode(b"K\xf6ln"), "K\u{f6}ln");
        assert_eq!(decode("Köln".as_bytes()), "Köln");
    }
}
