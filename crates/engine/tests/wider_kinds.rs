//! The wider kinds of data (team numbers, other rated people, relationships and history,
//! fixed facts) have written data rules on the shared rating type and no stored field yet:
//! the contract page sets the rules, and no struct the content files load holds a field the
//! page lists as not stored.

use std::path::Path;

use engine::Rating;

/// The shared rating type is usable where the wider kinds will need it, at compile time too.
const REFEREE_STRICTNESS_EXAMPLE: Rating = Rating::from_tenths(155);

fn repo() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn contract_page() -> String {
    std::fs::read_to_string(repo().join("docs/explanation/player-contract.md")).unwrap()
}

/// The names the page lists under "Not stored yet", each in backticks.
fn not_stored(page: &str) -> Vec<String> {
    let section = page
        .split("### Not stored yet")
        .nth(1)
        .expect("the page has a Not stored yet section");
    section
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

#[test]
fn the_contract_page_sets_the_rules_of_the_four_wider_kinds_on_the_rating_type() {
    let page = contract_page();
    for heading in [
        "### Team numbers",
        "### Other rated people",
        "### Relationships and history",
        "### Fixed facts",
    ] {
        assert!(page.contains(heading), "the page has no {heading}");
    }
    assert!(
        page.contains("`Rating`"),
        "the page does not name the Rating type"
    );
    assert_eq!(REFEREE_STRICTNESS_EXAMPLE.whole(), 16);
}

/// The field names declared in `file`: every `name:` that starts a line, after `pub`.
fn fields(file: &str) -> Vec<String> {
    file.lines()
        .filter_map(|line| {
            let t = line.trim_start();
            let t = t
                .strip_prefix("pub(crate) ")
                .or_else(|| t.strip_prefix("pub "))
                .unwrap_or(t);
            let (name, _) = t.split_once(':')?;
            (!name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
            .then(|| name.to_string())
        })
        .collect()
}

#[test]
fn no_field_of_a_wider_kind_is_stored() {
    let names = not_stored(&contract_page());
    assert!(names.len() >= 9, "the page lists only {names:?}");
    let src = repo().join("crates/engine/src/data");
    let mut found = Vec::new();
    for file in ["team.rs", "tactics.rs", "tuning.rs", "attributes.rs"] {
        let text = std::fs::read_to_string(src.join(file)).unwrap();
        for field in fields(&text) {
            if let Some(name) = names.iter().find(|n| field.contains(n.as_str())) {
                found.push(format!("{file}: {field} ({name})"));
            }
        }
    }
    assert!(found.is_empty(), "wider-kind fields stored: {found:?}");
}

#[test]
fn the_field_scan_finds_a_planted_field() {
    let planted =
        "pub struct Pair {\n    pub liking: Rating,\n    pub(crate) understanding: Rating,\n}";
    assert_eq!(fields(planted), vec!["liking", "understanding"]);
}
