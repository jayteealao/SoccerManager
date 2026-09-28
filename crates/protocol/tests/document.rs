//! AC-f: every message the implementation names appears in the protocol reference document
//! with every one of its fields.

use std::path::PathBuf;

use protocol::{Encoding, MESSAGES};

fn document() -> String {
    let path: PathBuf =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/reference/protocol.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

#[test]
fn every_message_appears_in_the_document_with_its_fields() {
    let text = document();
    for message in MESSAGES {
        let heading = format!("### {}\n", message.name);
        assert!(
            text.contains(&heading),
            "the document has no section for the {} message",
            message.name
        );
        for field in message.fields {
            assert!(
                text.contains(&format!("| `{field}` |")),
                "the {} section does not describe the field {field}",
                message.name
            );
        }
    }
}

#[test]
fn the_document_names_the_framing_of_every_message() {
    let text = document();
    assert!(text.contains("## Framing"));
    assert!(text.contains("## Backpressure"));
    assert!(text.contains("## Replay files"));
    let binary: Vec<&str> = MESSAGES
        .iter()
        .filter(|m| m.encoding == Encoding::Binary)
        .map(|m| m.name)
        .collect();
    assert_eq!(binary, vec!["tick"], "only the tick frame is binary");
    assert!(text.contains("Binary, not JSON"));
}
