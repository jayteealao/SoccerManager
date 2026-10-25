//! A streaming reader for the tracking files' XML: the files are hundreds of megabytes with
//! one element per line and double-quoted attributes, so the reader takes one line at a time
//! and hands each tag on it to a callback, borrowed from the line, never holding the file.
//!
//! It knows start tags with their attributes, empty tags (`<a/>`), end tags, the XML
//! declaration and comments (both skipped), and the five named entities and numeric
//! character references in attribute values. A tag that does not close on its line, an
//! attribute without a quoted value and an unknown entity are errors that name the line.

use std::borrow::Cow;
use std::io::BufRead;

/// What a tag is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    /// `<name ...>`
    Start,
    /// `<name .../>`
    Empty,
    /// `</name>`
    End,
}

/// One tag, borrowed from its line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag<'a> {
    pub kind: TagKind,
    pub name: &'a str,
    pub attrs: Vec<(&'a str, Cow<'a, str>)>,
    /// The line of the file, from 1.
    pub line: usize,
}

impl Tag<'_> {
    /// The value of the attribute `key`.
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_ref())
    }

    /// `true` for a start or empty tag named `name`.
    pub fn opens(&self, name: &str) -> bool {
        self.kind != TagKind::End && self.name == name
    }
}

/// Reads every tag of `input` in order and hands it to `each`; the first error of the file
/// or of `each` ends the read.
pub fn read_tags<R: BufRead>(
    mut input: R,
    mut each: impl FnMut(&Tag<'_>) -> Result<(), String>,
) -> Result<(), String> {
    let mut line = String::new();
    let mut number = 0usize;
    loop {
        line.clear();
        let n = input
            .read_line(&mut line)
            .map_err(|e| format!("line {}: cannot read: {e}", number + 1))?;
        if n == 0 {
            return Ok(());
        }
        number += 1;
        parse_line(&line, number, &mut each)?;
    }
}

/// The tags of one line.
fn parse_line(
    text: &str,
    line: usize,
    each: &mut impl FnMut(&Tag<'_>) -> Result<(), String>,
) -> Result<(), String> {
    let err = |what: &str| format!("line {line}: {what}");
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        rest = &rest[open + 1..];
        if let Some(after) = rest.strip_prefix("!--") {
            let close = after
                .find("-->")
                .ok_or_else(|| err("a comment does not close on its line"))?;
            rest = &after[close + 3..];
            continue;
        }
        if rest.starts_with('?') {
            let close = rest
                .find("?>")
                .ok_or_else(|| err("the declaration does not close on its line"))?;
            rest = &rest[close + 2..];
            continue;
        }
        let close = tag_end(rest).ok_or_else(|| err("a tag does not close on its line"))?;
        let body = &rest[..close];
        rest = &rest[close + 1..];
        let tag = parse_tag(body, line).map_err(|w| err(&w))?;
        each(&tag)?;
    }
    Ok(())
}

/// The place of the `>` that ends a tag, outside quoted values.
fn tag_end(s: &str) -> Option<usize> {
    let mut quoted = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '>' if !quoted => return Some(i),
            _ => {}
        }
    }
    None
}

/// One tag's body: the text between `<` and `>`.
fn parse_tag(body: &str, line: usize) -> Result<Tag<'_>, String> {
    if let Some(name) = body.strip_prefix('/') {
        let name = name.trim();
        if name.is_empty() {
            return Err("an end tag without a name".into());
        }
        return Ok(Tag {
            kind: TagKind::End,
            name,
            attrs: Vec::new(),
            line,
        });
    }
    let (body, kind) = match body.strip_suffix('/') {
        Some(b) => (b, TagKind::Empty),
        None => (body, TagKind::Start),
    };
    let name_end = body
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(body.len());
    let name = &body[..name_end];
    if name.is_empty() {
        return Err("a tag without a name".into());
    }
    let mut attrs = Vec::new();
    let mut rest = body[name_end..].trim_start();
    while !rest.is_empty() {
        let eq = rest
            .find('=')
            .ok_or_else(|| format!("attribute {} has no value", rest.trim()))?;
        let key = rest[..eq].trim();
        if key.is_empty() || key.contains(char::is_whitespace) {
            return Err(format!("attribute {key} has no value"));
        }
        let after = rest[eq + 1..].trim_start();
        let value = after
            .strip_prefix('"')
            .ok_or_else(|| format!("attribute {key} has no double-quoted value"))?;
        let end = value
            .find('"')
            .ok_or_else(|| format!("attribute {key} has no closing quote"))?;
        attrs.push((key, decode(&value[..end])?));
        rest = value[end + 1..].trim_start();
    }
    Ok(Tag {
        kind,
        name,
        attrs,
        line,
    })
}

/// An attribute value with its entities replaced; borrowed when it has none.
fn decode(raw: &str) -> Result<Cow<'_, str>, String> {
    if !raw.contains('&') {
        return Ok(Cow::Borrowed(raw));
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        let semi = after
            .find(';')
            .ok_or_else(|| format!("an entity without its ; in {raw}"))?;
        let name = &after[..semi];
        let ch = match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = if let Some(hex) = name.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16).ok()
                } else if let Some(dec) = name.strip_prefix('#') {
                    dec.parse::<u32>().ok()
                } else {
                    None
                };
                code.and_then(char::from_u32)
                    .ok_or_else(|| format!("unknown entity &{name}; in {raw}"))?
            }
        };
        out.push(ch);
        rest = &after[semi + 1..];
    }
    out.push_str(rest);
    Ok(Cow::Owned(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every tag of `text` as (kind, name, attributes, line).
    fn tags(text: &str) -> Result<Vec<(TagKind, String, Vec<(String, String)>, usize)>, String> {
        let mut out = Vec::new();
        read_tags(text.as_bytes(), |t| {
            out.push((
                t.kind,
                t.name.to_string(),
                t.attrs
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), v.to_string()))
                    .collect(),
                t.line,
            ));
            Ok(())
        })?;
        Ok(out)
    }

    #[test]
    fn a_frame_line_gives_one_empty_tag_with_its_attributes() {
        let text = "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n\
                    <Frame N=\"10000\" T=\"2023-05-27T13:30:12.600+00:00\" X=\"19.95\" \
                    Y=\"-34.45\" M=\"1\"/>\n";
        let got = tags(text).unwrap();
        assert_eq!(got.len(), 1);
        let (kind, name, attrs, line) = &got[0];
        assert_eq!((*kind, name.as_str(), *line), (TagKind::Empty, "Frame", 2));
        assert_eq!(attrs[0], ("N".into(), "10000".into()));
        assert_eq!(attrs[3], ("Y".into(), "-34.45".into()));
        assert_eq!(attrs.len(), 5);
    }

    #[test]
    fn a_nested_event_opens_and_closes_in_order() {
        let text = "<Event EventId=\"7\" EventTime=\"2023-05-27T15:30:15.059+02:00\">\n \
                    <Play Player=\"P1\" Height=\"high\">\n  <Pass/>\n </Play>\n</Event>\n";
        let got: Vec<(TagKind, String)> = tags(text)
            .unwrap()
            .into_iter()
            .map(|(k, n, _, _)| (k, n))
            .collect();
        assert_eq!(
            got,
            vec![
                (TagKind::Start, "Event".into()),
                (TagKind::Start, "Play".into()),
                (TagKind::Empty, "Pass".into()),
                (TagKind::End, "Play".into()),
                (TagKind::End, "Event".into()),
            ]
        );
        let mut seen = None;
        read_tags(text.as_bytes(), |t| {
            if t.opens("Play") {
                seen = t.attr("Height").map(str::to_string);
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(seen.as_deref(), Some("high"));
    }

    #[test]
    fn entities_in_a_value_are_decoded() {
        let text = "<Team TeamName=\"A &amp; B &lt;II&gt; &quot;x&quot; &apos;y&apos; &#246;&#xE9;\" \
                    Plain=\"no entity\"/>\n<!-- a comment -->\n";
        let got = tags(text).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].2[0].1, "A & B <II> \"x\" 'y' öé");
        assert_eq!(got[0].2[1].1, "no entity");
        assert!(
            tags("<a b=\"&nope;\"/>")
                .unwrap_err()
                .contains("unknown entity")
        );
    }

    #[test]
    fn a_broken_quote_is_refused_naming_its_line() {
        let text = "<Frame N=\"1\"/>\n<Frame N=\"2 X=\"3\"/>\n";
        let err = tags(text).unwrap_err();
        assert!(err.starts_with("line 2:"), "{err}");
        let err = tags("<a>\n<b c=\"1/>\n").unwrap_err();
        assert!(err.starts_with("line 2:"), "{err}");
        let err = tags("<a b=1/>").unwrap_err();
        assert!(err.contains("double-quoted"), "{err}");
    }

    #[test]
    fn the_callback_error_ends_the_read() {
        let mut n = 0;
        let err = read_tags("<a/><b/>\n<c/>\n".as_bytes(), |_| {
            n += 1;
            if n == 2 { Err("stop".into()) } else { Ok(()) }
        })
        .unwrap_err();
        assert_eq!((err.as_str(), n), ("stop", 2));
    }
}
