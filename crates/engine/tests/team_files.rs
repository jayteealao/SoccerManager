//! AC-b: a team file with a player attribute of 120 is refused with a message naming the
//! file, the player, and the attribute; a valid team file loads.

mod common;

use engine::EngineError;

#[test]
fn an_attribute_of_120_is_refused_naming_file_player_and_attribute() {
    let content = common::content();
    let dir = common::content_dir();
    let path = common::fixture_path("team-bad-attribute.json");
    let err = content.load_team(&dir, &path).unwrap_err();
    let text = err.to_string();
    assert!(
        matches!(err, EngineError::Data { kind: "team", .. }),
        "{text}"
    );
    assert_eq!(
        text,
        "content refused: team team-bad-attribute.json: players: \
         player p-club-00000001-00-03: attribute pace is 120; allowed 1 to 100"
    );
}

#[test]
fn a_valid_team_file_loads() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    assert_eq!(a.players.len(), 22);
    assert_eq!(b.players.len(), 22);
    assert_ne!(a.club.id, b.club.id);
}
