//! Placeholder scanning and filling for commentary lines. A placeholder is a name in braces,
//! such as `{player}`. Rust's formatting macros need a literal format string, so a line read
//! from a file is filled by this single left-to-right scan instead.

/// Every placeholder a line may use.
pub const PLACEHOLDERS: [&str; 11] = [
    "player",
    "other_player",
    "team",
    "opponent",
    "home",
    "away",
    "home_score",
    "away_score",
    "score",
    "minute",
    "added_minutes",
];

/// The index of `name` in [`PLACEHOLDERS`].
pub fn index_of(name: &str) -> Option<usize> {
    PLACEHOLDERS.iter().position(|p| *p == name)
}

/// The value of each placeholder for one event; `None` where the event has no value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Values([Option<String>; PLACEHOLDERS.len()]);

impl Values {
    /// Sets `name` to `value`. `name` must be one of [`PLACEHOLDERS`].
    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        if let Some(i) = index_of(name) {
            self.0[i] = Some(value.into());
        }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        index_of(name).and_then(|i| self.0[i].as_deref())
    }
}

/// One piece of a scanned line.
enum Piece<'a> {
    Text(&'a str),
    Name(&'a str),
}

/// Splits `line` into text and placeholder names, or says where a brace is stray.
fn scan(line: &str) -> Result<Vec<Piece<'_>>, String> {
    let mut pieces = Vec::new();
    let mut rest = line;
    let mut at = 0;
    while !rest.is_empty() {
        let Some(o) = rest.find(['{', '}']) else {
            pieces.push(Piece::Text(rest));
            break;
        };
        if rest.as_bytes()[o] == b'}' {
            return Err(format!("stray '}}' at character {}", at + o));
        }
        pieces.push(Piece::Text(&rest[..o]));
        let after = &rest[o + 1..];
        let end = after
            .find(['{', '}'])
            .filter(|&e| after.as_bytes()[e] == b'}')
            .ok_or_else(|| format!("stray '{{' at character {}", at + o))?;
        pieces.push(Piece::Name(&after[..end]));
        let used = o + 1 + end + 1;
        at += used;
        rest = &rest[used..];
    }
    Ok(pieces)
}

/// The placeholder names `line` uses, in order, or the position of a stray brace.
pub fn placeholders(line: &str) -> Result<Vec<&str>, String> {
    Ok(scan(line)?
        .into_iter()
        .filter_map(|p| match p {
            Piece::Name(n) => Some(n),
            Piece::Text(_) => None,
        })
        .collect())
}

/// `line` with every placeholder replaced by its value. `None` when a placeholder has no
/// value or a brace is stray, so no line with an unfilled placeholder can leave the
/// commentator.
pub fn fill(line: &str, values: &Values) -> Option<String> {
    let mut out = String::with_capacity(line.len() + 16);
    for piece in scan(line).ok()? {
        match piece {
            Piece::Text(t) => out.push_str(t),
            Piece::Name(n) => out.push_str(values.get(n)?),
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> Values {
        let mut v = Values::default();
        for name in PLACEHOLDERS {
            v.set(name, format!("<{name}>"));
        }
        v
    }

    #[test]
    fn every_placeholder_fills() {
        let v = values();
        for name in PLACEHOLDERS {
            let line = format!("a {{{name}}} b");
            assert_eq!(fill(&line, &v).unwrap(), format!("a <{name}> b"));
        }
    }

    #[test]
    fn a_missing_value_gives_no_line() {
        let mut v = Values::default();
        v.set("team", "Rovers");
        assert_eq!(fill("{team} win it", &v).unwrap(), "Rovers win it");
        assert_eq!(fill("{player} for {team}", &v), None);
    }

    #[test]
    fn a_line_with_no_placeholder_is_unchanged() {
        assert_eq!(fill("Play on.", &Values::default()).unwrap(), "Play on.");
        assert!(placeholders("Play on.").unwrap().is_empty());
    }

    #[test]
    fn a_stray_brace_is_reported() {
        assert_eq!(
            placeholders("oops } here").unwrap_err(),
            "stray '}' at character 5"
        );
        assert_eq!(
            placeholders("{team} and {player").unwrap_err(),
            "stray '{' at character 11"
        );
        assert_eq!(
            placeholders("{te{am}").unwrap_err(),
            "stray '{' at character 0"
        );
        assert_eq!(fill("{team", &values()), None);
    }

    #[test]
    fn placeholders_come_back_in_order() {
        assert_eq!(
            placeholders("{player} fouls {other_player} ({team})").unwrap(),
            ["player", "other_player", "team"]
        );
    }
}
