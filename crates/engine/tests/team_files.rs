//! Team files: a version 1 file with an attribute of 120 is refused naming the file, the
//! player, and the attribute; a valid file loads. Version 2: a file in tenths with body
//! fields loads and reads back unchanged; a version 1 file converts to `2v` tenths on the
//! file path and the byte path alike; a version 2 value out of range or off the tenth grid,
//! or a missing body field, is refused naming the player and the field; a version 1 tactics
//! file converts to the shipped version 2 file exactly.

mod common;

use engine::data::team::TeamFile;
use engine::{EngineError, Rating};

/// A copy of a converted file that a version 2 file may hold: ratings under 1.0 lifted to
/// 1.0, and body fields on every player.
fn as_v2(file: &TeamFile) -> TeamFile {
    let mut out = file.clone();
    engine::data::generator::lift_to_floor(&mut out);
    for p in &mut out.players {
        p.height = Some(180);
        p.age = Some(25);
        p.nationality = Some("ENG".into());
    }
    out
}

fn fixture_bytes(name: &str) -> Vec<u8> {
    std::fs::read(common::fixture_path(name)).unwrap()
}

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

#[test]
fn a_version_2_file_loads_and_reads_back_every_value_unchanged() {
    let content = common::content();
    let bytes = fixture_bytes("teams/v2-good.json");
    let loaded = content.team_from_bytes(&bytes, "v2-good.json").unwrap();
    assert_eq!(loaded.converted_from, None);
    let file = loaded.value;
    let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for (p, raw) in file.players.iter().zip(doc["players"].as_array().unwrap()) {
        for (name, rating) in &p.attributes {
            let written = raw["attributes"][name].as_f64().unwrap();
            assert_eq!(rating.decimal(), written, "{} {name}", p.id);
            assert_eq!(Rating::from_decimal(written).unwrap(), *rating);
        }
        assert_eq!(
            u64::from(p.height.unwrap()),
            raw["height"].as_u64().unwrap()
        );
        assert_eq!(u64::from(p.age.unwrap()), raw["age"].as_u64().unwrap());
        assert_eq!(p.nationality.as_deref(), raw["nationality"].as_str());
    }
    // Written again, the file reads back the same.
    let again = serde_json::to_vec(&file).unwrap();
    let reread = content.team_from_bytes(&again, "again.json").unwrap().value;
    assert_eq!(reread, file);
}

#[test]
fn a_version_1_file_converts_to_twice_its_values_on_both_paths() {
    let content = common::content();
    let dir = common::content_dir();
    let path = dir.path(engine::data::TEAM_A_FILE);
    let bytes = std::fs::read(&path).unwrap();
    let v1: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v1["schema_version"], 1);
    let from_file = content.load_team(&dir, &path).unwrap();
    let from_bytes = content.team_from_bytes(&bytes, "default-a.json").unwrap();
    assert_eq!(from_file.converted_from, Some(1));
    assert_eq!(from_bytes.converted_from, Some(1));
    assert_eq!(from_file.value, from_bytes.value);
    assert_eq!(from_file.digest, from_bytes.digest);
    let mut low = 0;
    for (p, raw) in from_file
        .value
        .players
        .iter()
        .zip(v1["players"].as_array().unwrap())
    {
        assert_eq!(
            (p.height, p.age, p.nationality.as_deref()),
            (None, None, None)
        );
        for (name, rating) in &p.attributes {
            let v = raw["attributes"][name].as_u64().unwrap();
            assert_eq!(u64::from(rating.tenths()), 2 * v, "{} {name}", p.id);
            if v < 5 {
                // 1 to 4 become 0.2 to 0.8: the temporary floor of converted files.
                assert!((2..=8).contains(&rating.tenths()));
                low += 1;
            }
        }
    }
    assert!(low > 0, "the shipped file holds values under 5");
}

#[test]
fn a_bad_version_2_value_or_a_missing_body_field_is_refused_by_player_and_field() {
    let content = common::content();
    for (name, reason) in [
        (
            "v2-out-of-range.json",
            "player p-club-00000001-00-04: attribute pace is 20.5; allowed 1.0 to 20.0",
        ),
        (
            "v2-off-grid.json",
            "player p-club-00000001-00-04: attribute pace is 12.34; not on a tenth",
        ),
        (
            "v2-missing-height.json",
            "player p-club-00000001-00-04: body field height is missing",
        ),
    ] {
        let err = content
            .team_from_bytes(&fixture_bytes(&format!("teams/{name}")), name)
            .unwrap_err();
        assert!(
            matches!(err, EngineError::Data { kind: "team", .. }),
            "{err}"
        );
        assert_eq!(
            err.to_string(),
            format!("content refused: team {name}: players: {reason}")
        );
    }
}

#[test]
fn a_team_file_of_another_version_is_refused_naming_both() {
    let content = common::content();
    let bytes = String::from_utf8(fixture_bytes("teams/v2-good.json"))
        .unwrap()
        .replacen("\"schema_version\": 2", "\"schema_version\": 3", 1);
    let err = content
        .team_from_bytes(bytes.as_bytes(), "v3.json")
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "content refused: team v3.json: schema_version 3; this build reads 2"
    );
}

#[test]
fn a_version_1_tactics_file_converts_to_the_shipped_version_2_file() {
    let v1 = fixture_bytes("tactics-v1.json");
    let converted = engine::data::load_tactics_bytes(&v1, "tactics-v1.json").unwrap();
    assert_eq!(converted.converted_from, Some(1));
    assert_eq!(converted.value, common::content().tactics);
}

/// A team file with `ground` set in its club, as bytes.
fn with_ground(file: &TeamFile, ground: Option<(f64, f64)>) -> Vec<u8> {
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
    let a = as_v2(&a);
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
    let digest = |home: &TeamFile| {
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
    let a = as_v2(&a);
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
