//! Team files: a version 1 file with an attribute of 120 is refused naming the file, the
//! player, and the attribute; a valid file loads. Version 2: a file in tenths with body
//! fields loads and reads back unchanged; a version 1 file converts to `2v` tenths on the
//! file path and the byte path alike; a version 2 value out of range or off the tenth grid,
//! or a missing body field, is refused naming the player and the field; a version 1 tactics
//! file converts to the shipped version 2 file exactly. Version 3: a file with position
//! ratings, a foot, traits and personality loads; bad person content is refused by player,
//! field and rule; every shipped version 1 file converts with values from related attributes
//! and plays.

mod common;

use std::collections::BTreeMap;

use engine::EngineError;
use engine::data::Position;
use engine::data::team::TeamFile;

/// A copy of a converted file that a version 3 file may hold: ratings under 1.0 lifted to
/// 1.0, and body fields on every player.
fn as_v3(file: &TeamFile) -> TeamFile {
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

/// The ratings of a version 2 player in tenths, by name, as the file writes them.
fn tenths_of(raw: &serde_json::Value) -> BTreeMap<String, u32> {
    raw["attributes"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, v)| {
            let tenths = (v.as_f64().unwrap() * 10.0).round() as u32;
            (name.clone(), tenths)
        })
        .collect()
}

/// The mean of `names` in `old`, rounded half up to a tenth and kept to 1.0 to 20.0,
/// recomputed here from the old numbers.
fn mean_tenths(old: &BTreeMap<String, u32>, names: &[&str]) -> u32 {
    let values: Vec<u32> = names.iter().map(|n| old[*n]).collect();
    let n = values.len() as u32;
    ((values.iter().sum::<u32>() + n / 2) / n).clamp(10, 200)
}

/// Checks one converted player against the version 2 ratings `old` he was converted from:
/// every old value kept (an outfield player's communication aside), each new performance value
/// the mean of its related values, an outfield player's communication the old mental mean,
/// each personality value 10.0, his listed position at his level and every other at 1.0, a
/// weaker foot of 50 percent, and no traits.
fn assert_converted(
    p: &engine::data::PlayerEntry,
    old: &BTreeMap<String, u32>,
    content: &engine::Content,
) {
    use engine::data::convert::{MENTAL_V3, V3_RELATED};
    let keeper = p.position == Position::GK;
    for (name, &tenths) in old {
        if name == "communication" && !keeper {
            continue;
        }
        assert_eq!(
            u32::from(p.attributes[name].tenths()),
            tenths,
            "{} {name}",
            p.id
        );
    }
    for (name, from) in V3_RELATED {
        assert_eq!(
            u32::from(p.attributes[name].tenths()),
            mean_tenths(old, from),
            "{} {name}",
            p.id
        );
    }
    let communication = u32::from(p.attributes["communication"].tenths());
    if keeper {
        assert_eq!(
            communication, old["communication"],
            "{}: a keeper keeps his",
            p.id
        );
    } else {
        assert_eq!(communication, mean_tenths(old, &MENTAL_V3), "{}", p.id);
    }
    for def in content
        .attributes
        .attributes
        .iter()
        .filter(|d| d.is_personality())
    {
        assert_eq!(
            p.attributes[&def.name].tenths(),
            100,
            "{} {}",
            p.id,
            def.name
        );
    }
    let level = engine::contract::level::overall_level(
        &engine::player::Attributes::from_entry(&p.attributes, &content.attributes),
        &content.attributes,
        p.position,
    );
    let positions = p
        .positions
        .as_ref()
        .expect("a converted player has positions");
    assert_eq!(positions.len(), 10, "{}", p.id);
    for (code, rating) in positions {
        let want = if *code == p.position {
            level.max(10)
        } else {
            10
        };
        assert_eq!(rating.tenths(), want, "{} {}", p.id, code.code());
    }
    let foot = p.foot.expect("a converted player has a foot");
    assert_eq!(foot.weaker, 50, "{}", p.id);
    assert_eq!(foot.strong, engine::data::convert::strong_side_of(&p.id));
    assert!(p.traits.is_empty(), "{}", p.id);
}

#[test]
fn a_version_2_file_converts_with_values_from_related_attributes() {
    let content = common::content();
    let bytes = fixture_bytes("teams/v2-good.json");
    let loaded = content.team_from_bytes(&bytes, "v2-good.json").unwrap();
    assert_eq!(loaded.converted_from, Some(2));
    let file = loaded.value;
    assert_eq!(file.schema_version, 3);
    let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for (p, raw) in file.players.iter().zip(doc["players"].as_array().unwrap()) {
        assert_converted(p, &tenths_of(raw), &content);
        assert_eq!(
            u64::from(p.height.unwrap()),
            raw["height"].as_u64().unwrap()
        );
        assert_eq!(u64::from(p.age.unwrap()), raw["age"].as_u64().unwrap());
        assert_eq!(p.nationality.as_deref(), raw["nationality"].as_str());
    }
    // Written again, the converted file is a version 3 file that reads back the same.
    let again = serde_json::to_vec(&file).unwrap();
    let reread = content.team_from_bytes(&again, "again.json").unwrap();
    assert_eq!(reread.converted_from, None);
    assert_eq!(reread.value, file);
}

/// A shipped version 1 player's ratings in the version 2 shape: `2v` tenths, at least 1.0,
/// injury resistance R as proneness 21.0 âˆ’ R, and consistency 10.0.
fn v1_as_v2(raw: &serde_json::Value) -> BTreeMap<String, u32> {
    let mut out: BTreeMap<String, u32> = raw["attributes"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, v)| {
            let tenths = (2 * v.as_u64().unwrap() as u32).max(10);
            if name == "injury_resistance" {
                ("injury_proneness".to_string(), 210 - tenths)
            } else {
                (name.clone(), tenths)
            }
        })
        .collect();
    out.insert("consistency".into(), 100);
    out
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
        assert_eq!(
            p.attributes["consistency"].tenths(),
            100,
            "{}: a version 1 player is of middling consistency",
            p.id
        );
        let old = v1_as_v2(raw);
        assert_converted(p, &old, &content);
        for (name, v) in raw["attributes"].as_object().unwrap() {
            if v.as_u64().unwrap() < 5 && name != "injury_resistance" {
                assert_eq!(p.attributes[name].tenths(), 10, "{} {name}", p.id);
                low += 1;
            }
        }
    }
    assert!(low > 0, "the shipped file holds values under 5");
}

#[test]
fn every_shipped_version_1_file_converts_with_values_from_related_attributes() {
    let content = common::content();
    let dir = common::content_dir();
    let mut names: Vec<_> = std::fs::read_dir(dir.path("teams"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    names.sort();
    assert_eq!(names.len(), 10, "the shipped team files");
    let mut keepers = 0;
    let mut left = 0;
    let mut players = 0;
    for path in &names {
        let bytes = std::fs::read(path).unwrap();
        let v1: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v1["schema_version"], 1, "{}", path.display());
        let loaded = content.load_team(&dir, path).unwrap();
        assert_eq!(loaded.converted_from, Some(1), "{}", path.display());
        for (p, raw) in loaded
            .value
            .players
            .iter()
            .zip(v1["players"].as_array().unwrap())
        {
            assert_converted(p, &v1_as_v2(raw), &content);
            let positions = p.positions.as_ref().unwrap();
            if p.position == Position::GK {
                keepers += 1;
                for (code, rating) in positions {
                    if *code != Position::GK {
                        assert_eq!(
                            rating.tenths(),
                            10,
                            "{}: a keeper's outfield positions",
                            p.id
                        );
                    }
                }
            }
            players += 1;
            if p.foot.unwrap().strong == engine::data::team::Side::Left {
                left += 1;
            }
        }
    }
    assert!(keepers >= 10, "every shipped squad has a keeper");
    assert!(
        left > 0 && left < players / 2,
        "the id fixes the side: some left, most right ({left} of {players})"
    );
}

/// The ten shipped files, converted, play five full matches (file 1 against 2, 3 against 4,
/// and so on) to full time.
#[test]
fn the_converted_shipped_files_play_five_full_matches() {
    let content = common::content();
    let dir = common::content_dir();
    let mut names: Vec<_> = std::fs::read_dir(dir.path("teams"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    names.sort();
    let teams: Vec<TeamFile> = names
        .iter()
        .map(|p| content.load_team(&dir, p).unwrap().value)
        .collect();
    let results = common::run_many(0..=4, |pair| {
        let (a, b) = (&teams[2 * pair as usize], &teams[2 * pair as usize + 1]);
        let config = engine::MatchConfig::new(pair + 1, 90, &content, [a, b]).unwrap();
        let mut sim = engine::Simulation::new(config).unwrap();
        while !sim.is_over() {
            sim.step();
        }
        sim.tick()
    });
    assert_eq!(results.len(), 5);
    assert!(results.iter().all(|&t| t > 0));
}

/// The version 3 fixture: one club as `engine-cli generate` writes it.
fn v3_good() -> serde_json::Value {
    serde_json::from_slice(&fixture_bytes("teams/v3-good.json")).unwrap()
}

#[test]
fn a_version_3_file_loads_with_every_person_field() {
    let content = common::content();
    let bytes = fixture_bytes("teams/v3-good.json");
    let loaded = content.team_from_bytes(&bytes, "v3-good.json").unwrap();
    assert_eq!(loaded.converted_from, None);
    let file = loaded.value;
    assert_eq!(file.schema_version, 3);
    let personality: Vec<&str> = content
        .attributes
        .attributes
        .iter()
        .filter(|d| d.is_personality())
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(personality.len(), 11);
    let mut traits = 0;
    for p in &file.players {
        let positions = p.positions.as_ref().expect("positions");
        assert_eq!(positions.len(), 10, "{}", p.id);
        let natural = positions[&p.position];
        assert!(positions.values().all(|v| *v <= natural), "{}", p.id);
        let foot = p.foot.expect("a foot");
        assert!((1..=100).contains(&foot.weaker), "{}", p.id);
        for name in &personality {
            let v = p.attributes[*name].tenths();
            assert!((10..=200).contains(&v), "{} {name}", p.id);
        }
        traits += p.traits.len();
    }
    assert!(traits > 0, "the generated club carries some traits");
    // Written again, the file reads back the same.
    let again = serde_json::to_vec(&file).unwrap();
    assert_eq!(
        content.team_from_bytes(&again, "again.json").unwrap().value,
        file
    );
}

/// The refusal of `edit` applied to the version 3 fixture.
fn refused_v3(edit: impl FnOnce(&mut serde_json::Value)) -> String {
    let content = common::content();
    let mut doc = v3_good();
    edit(&mut doc);
    let err = content
        .team_from_bytes(&serde_json::to_vec(&doc).unwrap(), "bad.json")
        .unwrap_err();
    assert!(
        matches!(err, EngineError::Data { kind: "team", .. }),
        "{err}"
    );
    err.to_string()
}

/// The index and id of the fixture's first outfield player.
fn outfield(doc: &serde_json::Value) -> (usize, String) {
    let i = doc["players"]
        .as_array()
        .unwrap()
        .iter()
        .position(|p| p["position"] != "GK")
        .unwrap();
    (i, doc["players"][i]["id"].as_str().unwrap().to_string())
}

#[test]
fn bad_person_content_is_refused_naming_player_field_and_rule() {
    let doc = v3_good();
    let (i, id) = outfield(&doc);
    let natural = doc["players"][i]["position"].as_str().unwrap().to_string();
    let other = if natural == "CB" { "LB" } else { "CB" };

    let text = refused_v3(|d| {
        let t = &mut d["players"][i]["traits"];
        *t = serde_json::json!({ "cuts_inside": "strong", "hugs_touchline": "moderate" });
    });
    assert!(
        text.ends_with(&format!(
            "players: player {id}: traits cuts_inside and hugs_touchline are opposites"
        )),
        "{text}"
    );

    let text = refused_v3(|d| {
        d["players"][i]["traits"] = serde_json::json!({ "spins_on_ball": "strong" });
    });
    assert!(
        text.ends_with(&format!(
            "player {id}: trait spins_on_ball is not in the trait catalogue"
        )),
        "{text}"
    );

    let text = refused_v3(|d| {
        let natural_value = d["players"][i]["positions"][&natural].as_f64().unwrap();
        let above = ((natural_value * 10.0).round() + 1.0) / 10.0;
        d["players"][i]["positions"][other] = serde_json::json!(above);
    });
    assert!(
        text.contains(&format!("player {id}: position rating {other} is "))
            && text.contains(&format!(", above his natural position {natural} at ")),
        "{text}"
    );

    let text = refused_v3(|d| {
        d["players"][i]["positions"]
            .as_object_mut()
            .unwrap()
            .remove("GK");
    });
    assert!(
        text.ends_with(&format!("player {id}: position rating GK is missing")),
        "{text}"
    );

    for weaker in [120, 0] {
        let text = refused_v3(|d| d["players"][i]["foot"]["weaker"] = weaker.into());
        assert!(
            text.ends_with(&format!(
                "player {id}: foot field weaker is {weaker}; allowed 1 to 100"
            )),
            "{text}"
        );
    }

    for value in [21.0, 0.5] {
        let text = refused_v3(|d| d["players"][i]["attributes"]["temperament"] = value.into());
        assert!(
            text.ends_with(&format!(
                "player {id}: attribute temperament is {value}; allowed 1.0 to 20.0"
            )),
            "{text}"
        );
    }

    let text = refused_v3(|d| d["players"][i]["attributes"]["leadership"] = 12.34.into());
    assert!(
        text.ends_with(&format!(
            "player {id}: attribute leadership is 12.34; not on a tenth"
        )),
        "{text}"
    );

    let text = refused_v3(|d| d["players"][i]["positions"]["ST"] = 20.5.into());
    assert!(
        text.ends_with(&format!(
            "player {id}: position rating ST is 20.5; allowed 1.0 to 20.0"
        )),
        "{text}"
    );
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
        .replacen("\"schema_version\": 2", "\"schema_version\": 4", 1);
    let err = content
        .team_from_bytes(bytes.as_bytes(), "v4.json")
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "content refused: team v4.json: schema_version 4; this build reads 3"
    );
}

/// A version 2 tactics file (three tempo levels, a block depth and flat offsets per
/// mentality, no primitives) converts to the shipped version 3 file: tempo takes the five
/// patience levels, each mentality drops its block depth and offsets and takes its loss weight
/// by its place, and the file the primitive entries with their fits.
#[test]
fn a_version_2_tactics_file_converts_to_the_shipped_version_3_file() {
    let path = common::content_dir().path(engine::data::TACTICS_FILE);
    let mut v2: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    v2["schema_version"] = 2.into();
    v2.as_object_mut().unwrap().remove("primitives").unwrap();
    v2["instructions"]["tempo"] = serde_json::json!({
        "default": "normal",
        "levels": [
            { "name": "slower", "value": -0.15 },
            { "name": "normal", "value": 0.0 },
            { "name": "faster", "value": 0.15 }
        ]
    });
    for (i, m) in v2["mentalities"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let m = m.as_object_mut().unwrap();
        m.remove("loss_weight").unwrap();
        let offset = 0.15 * (i as f64 - 2.0);
        m.insert("block_depth".into(), (offset * 4.0 / 0.15).into());
        m.insert("shoot".into(), offset.into());
        m.insert("progress".into(), offset.into());
        m.insert("hold".into(), (-offset * 2.0 / 3.0).into());
    }
    let converted =
        engine::data::load_tactics_bytes(&serde_json::to_vec(&v2).unwrap(), "tactics-v2.json")
            .unwrap();
    assert_eq!(converted.converted_from, Some(2));
    let shipped = common::content().tactics;
    assert_eq!(converted.value.instructions, shipped.instructions);
    assert_eq!(converted.value.mentalities, shipped.mentalities);
    assert_eq!(converted.value.primitives, shipped.primitives);
}

/// A version 3 tactics file (three width and three directness levels, only the tempo and
/// mentality primitives) converts to the shipped version 4 file: width and passing directness
/// take their five levels, and the file takes every primitive entry version 4 adds, after its
/// own.
#[test]
fn a_version_3_tactics_file_converts_to_the_shipped_version_4_file() {
    let path = common::content_dir().path(engine::data::TACTICS_FILE);
    let mut v3: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    v3["schema_version"] = 3.into();
    v3["primitives"]
        .as_array_mut()
        .unwrap()
        .retain(|p| p["name"] == "tempo" || p["name"] == "mentality");
    v3["instructions"]["width"] = serde_json::json!({
        "default": "normal",
        "levels": [
            { "name": "narrow", "value": 0.85 },
            { "name": "normal", "value": 1.0 },
            { "name": "wide", "value": 1.15 }
        ]
    });
    v3["instructions"]["passing_directness"] = serde_json::json!({
        "default": "mixed",
        "levels": [
            { "name": "short", "value": -0.2 },
            { "name": "mixed", "value": 0.0 },
            { "name": "direct", "value": 0.2 }
        ]
    });
    let converted =
        engine::data::load_tactics_bytes(&serde_json::to_vec(&v3).unwrap(), "tactics-v3.json")
            .unwrap();
    assert_eq!(converted.converted_from, Some(3));
    let shipped = common::content().tactics;
    assert_eq!(converted.value.instructions, shipped.instructions);
    assert_eq!(converted.value.mentalities, shipped.mentalities);
    assert_eq!(converted.value.primitives, shipped.primitives);
    assert_eq!(converted.value.roles, shipped.roles);
}

/// A version 4 tactics file (tempo levels named very direct to very patient) converts to the
/// shipped version 5 file: the tempo levels, the default and the tempo primitive's default and
/// fit records take the names very fast to very slow, with the same values and order; passing
/// directness keeps its names.
#[test]
fn a_version_4_tactics_file_converts_to_the_shipped_version_5_file() {
    let path = common::content_dir().path(engine::data::TACTICS_FILE);
    let mut v4: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    v4["schema_version"] = 4.into();
    let old = [
        "very_direct",
        "direct",
        "balanced",
        "patient",
        "very_patient",
    ];
    v4["instructions"]["tempo"]["default"] = "balanced".into();
    for (level, name) in v4["instructions"]["tempo"]["levels"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(old)
    {
        level["name"] = name.into();
    }
    let tempo = v4["primitives"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["name"] == "tempo")
        .unwrap();
    tempo["default"] = "balanced".into();
    for fit in tempo["fits"].as_array_mut().unwrap() {
        let renamed = match fit["level"].as_str().unwrap() {
            "very_fast" => "very_direct",
            "fast" => "direct",
            "slow" => "patient",
            "very_slow" => "very_patient",
            other => panic!("unexpected tempo fit level {other}"),
        };
        fit["level"] = renamed.into();
    }
    let converted =
        engine::data::load_tactics_bytes(&serde_json::to_vec(&v4).unwrap(), "tactics-v4.json")
            .unwrap();
    assert_eq!(converted.converted_from, Some(4));
    let shipped = common::content().tactics;
    assert_eq!(converted.value.instructions, shipped.instructions);
    assert_eq!(converted.value.mentalities, shipped.mentalities);
    assert_eq!(converted.value.primitives, shipped.primitives);
    assert_eq!(converted.value.roles, shipped.roles);
    let names: Vec<&str> = shipped
        .instructions
        .tempo
        .levels
        .iter()
        .map(|l| l.name.as_str())
        .collect();
    assert_eq!(names, ["very_fast", "fast", "normal", "slow", "very_slow"]);
}

/// A version 5 tactics file (three pressing and three line-height levels, none of the
/// out-of-possession primitives) converts to the shipped version 6 file: pressing and line
/// height take their outer levels around the three they had, with the same names, values and
/// defaults, and the file takes every primitive entry version 6 adds, after its own.
#[test]
fn a_version_5_tactics_file_converts_to_the_shipped_version_6_file() {
    let path = common::content_dir().path(engine::data::TACTICS_FILE);
    let mut v5: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    v5["schema_version"] = 5.into();
    let added = [
        "pressing",
        "line_height",
        "line_of_engagement",
        "defensive_width",
        "force_the_opponent",
        "tackling",
        "offside_trap",
        "stop_short_goal_kicks",
        "press_on_back_pass",
        "press_on_poor_touch",
        "press_on_facing_back",
        "press_on_wide",
    ];
    v5["primitives"]
        .as_array_mut()
        .unwrap()
        .retain(|p| !added.contains(&p["name"].as_str().unwrap()));
    for block in ["pressing", "line_height"] {
        let levels = v5["instructions"][block]["levels"].as_array_mut().unwrap();
        levels.remove(4);
        levels.remove(0);
    }
    let converted =
        engine::data::load_tactics_bytes(&serde_json::to_vec(&v5).unwrap(), "tactics-v5.json")
            .unwrap();
    assert_eq!(converted.converted_from, Some(5));
    let shipped = common::content().tactics;
    assert_eq!(converted.value.instructions, shipped.instructions);
    assert_eq!(converted.value.primitives, shipped.primitives);
    assert_eq!(converted.value.roles, shipped.roles);
    assert_eq!(
        shipped.instructions.level_names(0),
        ["very_low", "low", "mid", "high", "very_high"]
    );
    assert_eq!(
        shipped.instructions.level_names(3),
        ["very_deep", "deep", "normal", "high", "very_high"]
    );
    assert_eq!(shipped.instructions.defaults()[0], 2);
    assert_eq!(shipped.instructions.defaults()[3], 2);

    // A version 5 file whose own pressing and line-height levels hold other values (an older
    // replay's content) keeps them and loads; the fits of those levels, made at the shipped
    // values, are dropped, and the other fits stay.
    v5["instructions"]["pressing"]["levels"][0]["press_distance_scale"] = 0.8.into();
    v5["instructions"]["line_height"]["levels"][2]["value"] = 5.0.into();
    let converted =
        engine::data::load_tactics_bytes(&serde_json::to_vec(&v5).unwrap(), "tactics-v5.json")
            .unwrap();
    let fits = |name: &str| -> Vec<String> {
        converted
            .value
            .primitives
            .iter()
            .find(|p| p.name == name)
            .unwrap()
            .fits
            .iter()
            .map(|f| f.level.clone())
            .collect()
    };
    assert_eq!(fits("pressing"), ["very_low", "high", "very_high"]);
    assert_eq!(fits("line_height"), ["very_high"]);
    assert_eq!(
        converted.value.instructions.pressing.levels[1].press_distance_scale,
        0.8
    );
}

#[test]
fn a_version_1_tactics_file_converts_to_the_shipped_version_3_file() {
    let v1 = fixture_bytes("tactics-v1.json");
    let converted = engine::data::load_tactics_bytes(&v1, "tactics-v1.json").unwrap();
    assert_eq!(converted.converted_from, Some(1));
    // The stand table, the stand options and their sizes, each role's menu and the
    // rest-defence guard are optional parts of version 2 that a version 1 file has none of:
    // it converts with them neutral, so it plays as it did.
    let mut shipped = common::content().tactics;
    shipped.stands = Default::default();
    shipped.stand_options = None;
    shipped.rest_defence = None;
    for role in &mut shipped.roles {
        role.options = Default::default();
    }
    for mentality in &mut shipped.mentalities {
        mentality.rest_defence = 0;
    }
    // The out-of-position margin of the AI lineup is optional too: 0 when absent.
    shipped.ai.out_of_position_margin = 0.0;
    // So is a pressing level's presser willingness: the three levels the file holds take 0,
    // and the two outer levels version 6 adds take theirs.
    for level in &mut shipped.instructions.pressing.levels[1..4] {
        level.press_willingness = 0.0;
    }
    assert_eq!(converted.value, shipped);
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
    let a = as_v3(&a);
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
    let a = as_v3(&a);
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

/// A player's match condition reads back from the file; a file without the block writes no
/// block, so its team digest is what it was before the block existed.
#[test]
fn the_condition_block_reads_back_and_its_absence_writes_nothing() {
    use engine::data::team::Condition;
    let content = common::content();
    let file = content
        .team_from_bytes(
            &fixture_bytes("teams/condition-good.json"),
            "condition-good.json",
        )
        .unwrap()
        .value;
    assert_eq!(
        file.players[3].condition,
        Some(Condition {
            sharpness: Some(60),
            adaptation: Some(40),
            rest_days: Some(3),
            matches_at_club: Some(12),
            ..Condition::default()
        })
    );
    assert_eq!(
        file.players[4].condition,
        Some(Condition {
            rest_days: Some(7),
            ..Condition::default()
        })
    );
    assert_eq!(file.players[0].condition, None);
    let plain = content
        .team_from_bytes(&fixture_bytes("teams/v2-good.json"), "v2-good.json")
        .unwrap()
        .value;
    let plain_bytes = serde_json::to_vec(&plain).unwrap();
    assert!(!String::from_utf8_lossy(&plain_bytes).contains("condition"));
    let mut stripped = file.clone();
    for p in &mut stripped.players {
        p.condition = None;
    }
    assert_eq!(serde_json::to_vec(&stripped).unwrap(), plain_bytes);
    assert_ne!(serde_json::to_vec(&file).unwrap(), plain_bytes);
}

#[test]
fn a_condition_value_out_of_range_is_refused_naming_the_player_and_the_field() {
    let content = common::content();
    let name = "condition-bad-sharpness.json";
    let err = content
        .team_from_bytes(&fixture_bytes(&format!("teams/{name}")), name)
        .unwrap_err();
    assert!(
        matches!(err, EngineError::Data { kind: "team", .. }),
        "{err}"
    );
    assert_eq!(
        err.to_string(),
        format!(
            "content refused: team {name}: players: player p-club-00000001-00-04: \
             condition field sharpness is 101; allowed 0 to 100"
        )
    );
}

/// The season inputs (recent form in tenths, goal progress in percent) load in range and are
/// refused out of range, naming the player, the field and the range.
#[test]
fn the_season_inputs_load_in_range_and_are_refused_out_of_range() {
    let content = common::content();
    let with = |field: &str, v: u32| {
        let mut raw: serde_json::Value =
            serde_json::from_slice(&fixture_bytes("teams/v2-good.json")).unwrap();
        raw["players"][0]["condition"] = serde_json::json!({ field: v });
        content.team_from_bytes(&serde_json::to_vec(&raw).unwrap(), "planted.json")
    };
    for (field, v, range) in [
        ("recent_rating", 101, "10 to 100"),
        ("recent_rating", 9, "10 to 100"),
        ("goal_progress", 201, "0 to 200"),
    ] {
        let text = with(field, v).unwrap_err().to_string();
        assert!(
            text.contains(&format!(
                "player p-club-00000001-00-01: condition field {field} is {v}; allowed {range}"
            )),
            "{text}"
        );
    }
    for (field, v) in [
        ("recent_rating", 10),
        ("recent_rating", 100),
        ("goal_progress", 0),
        ("goal_progress", 200),
    ] {
        let file = with(field, v).unwrap().value;
        let c = file.players[0].condition.unwrap();
        let got = if field == "recent_rating" {
            c.recent_rating
        } else {
            c.goal_progress
        };
        assert_eq!(got, Some(u8::try_from(v).unwrap()), "{field}");
    }
}

/// A copy of a converted version 1 file says so with `converted_from: 1`, and its players
/// load without body fields; without the mark a missing body field is still refused, and any
/// other version it names is refused.
#[test]
fn a_converted_copy_loads_without_body_fields_and_only_from_version_1() {
    let content = common::content();
    let dir = common::content_dir();
    let shipped = content
        .load_team(&dir, &dir.path(engine::data::TEAM_A_FILE))
        .unwrap()
        .value;
    assert!(shipped.players.iter().all(|p| p.height.is_none()));
    let mut copy = shipped.clone();
    copy.schema_version = engine::data::TEAM_VERSION;
    let load =
        |file: &TeamFile| content.team_from_bytes(&serde_json::to_vec(file).unwrap(), "copy.json");
    let text = load(&copy).unwrap_err().to_string();
    assert!(text.contains("body field height is missing"), "{text}");
    copy.converted_from = Some(1);
    let loaded = load(&copy).unwrap().value;
    assert_eq!(loaded.players, shipped.players);
    copy.converted_from = Some(2);
    let text = load(&copy).unwrap_err().to_string();
    assert!(text.contains("converted_from is 2; allowed 1"), "{text}");
}

/// A version 2 file still naming the old injury resistance is refused naming the player and
/// the attribute: version 2 carries both hidden values and no resistance.
#[test]
fn a_version_2_file_naming_injury_resistance_is_refused() {
    let content = common::content();
    let bytes = fixture_bytes("teams/v2-good.json");
    let mut raw: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let first = &mut raw["players"][0]["attributes"];
    let proneness = first["injury_proneness"].clone();
    first.as_object_mut().unwrap().remove("injury_proneness");
    first["injury_resistance"] = proneness;
    let err = content
        .team_from_bytes(&serde_json::to_vec(&raw).unwrap(), "planted.json")
        .unwrap_err();
    let text = err.to_string();
    assert!(text.contains("player p-club-00000001-00-01"), "{text}");
    assert!(text.contains("injury_"), "{text}");
}
