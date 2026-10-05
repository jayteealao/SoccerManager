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

/// A team file with `ground` set in its club, as bytes.
fn with_ground(file: &engine::data::team::TeamFile, ground: Option<(f64, f64)>) -> Vec<u8> {
    let mut doc = serde_json::to_value(file).unwrap();
    if let Some((length, width)) = ground {
        doc["club"]["ground"] = serde_json::json!({ "length": length, "width": width });
    }
    serde_json::to_vec_pretty(&doc).unwrap()
}

#[test]
fn a_file_with_no_ground_and_one_with_105_by_68_hash_the_same_and_100_by_64_differs() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    assert!(
        a.club.ground.is_default(),
        "the sample teams play on 105 by 68"
    );
    let load = |ground| {
        content
            .team_from_bytes(&with_ground(&a, ground), "home.json")
            .unwrap()
            .value
    };
    let none = load(None);
    let default = load(Some((105.0, 68.0)));
    let smaller = load(Some((100.0, 64.0)));
    assert_eq!(
        serde_json::to_vec(&none).unwrap(),
        serde_json::to_vec(&a).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&default).unwrap(),
        serde_json::to_vec(&none).unwrap()
    );
    let digest = |home: &engine::data::team::TeamFile| {
        let config = engine::MatchConfig::new(1, 90, &content, [home, &b]).unwrap();
        (
            config.team_digests[0],
            config.content_hash.clone(),
            *config.pitch(),
        )
    };
    let (d_none, h_none, p_none) = digest(&none);
    let (d_default, h_default, _) = digest(&default);
    let (d_smaller, h_smaller, p_smaller) = digest(&smaller);
    assert_eq!((d_none, &h_none), (d_default, &h_default));
    assert_ne!(d_none, d_smaller);
    assert_ne!(h_none, h_smaller);
    assert_eq!(p_none, engine::pitch::Pitch::DEFAULT);
    assert_eq!((p_smaller.length(), p_smaller.width()), (100.0, 64.0));
}

#[test]
fn a_ground_outside_the_laws_is_refused_naming_the_club_the_value_and_the_limit() {
    let content = common::content();
    let [a, _] = common::default_teams(&content);
    let club = a.club.name.clone();
    for (length, width, reason) in [
        (
            89.0,
            68.0,
            "the ground is 89 m long; the Laws allow 90 to 120 m",
        ),
        (
            121.0,
            68.0,
            "the ground is 121 m long; the Laws allow 90 to 120 m",
        ),
        (
            105.0,
            44.0,
            "the ground is 44 m wide; the Laws allow 45 to 90 m",
        ),
        (
            105.0,
            91.0,
            "the ground is 91 m wide; the Laws allow 45 to 90 m",
        ),
    ] {
        let err = content
            .team_from_bytes(&with_ground(&a, Some((length, width))), "home.json")
            .unwrap_err();
        assert!(
            matches!(err, EngineError::Data { kind: "team", .. }),
            "{err}"
        );
        assert_eq!(
            err.to_string(),
            format!("content refused: team home.json: club.ground: {club}: {reason}")
        );
    }
    for (length, width) in [(90.0, 68.0), (120.0, 68.0), (105.0, 45.0), (105.0, 90.0)] {
        let file = content
            .team_from_bytes(&with_ground(&a, Some((length, width))), "home.json")
            .unwrap()
            .value;
        assert_eq!(file.club.pitch().unwrap().length(), length);
    }
}
