//! The stream-id table: one row per random draw in match play. Each row names the subsystem
//! and the kind of action of the draw, whether it is keyed by the player who acts or by a
//! named match key, and which scripted-draw queue a test scene may feed it from.
//!
//! A key is a row plus a player key. The keyed scheme derives one stream id per key:
//! `subsystem << 48 | action code << 32 | player key`, where a player key is
//! `team << 8 | squad index` and the match key is `0xFFFF`. The squad index, not the roster
//! index, names the player, so a substitute (who takes the roster index of the player he
//! replaces) gets his own key. No keyed id is 0, the id of the legacy shared stream.

use crate::player::Player;

/// The keyed scheme's id: every key reads its own stream from the match seed.
pub const KEYED_SCHEME: u8 = 1;

/// The largest squad a team file may hold (`data/team.rs`).
pub const SQUAD_MAX: usize = 40;

/// Player slots per action in the dense key index: two squads, then the match key.
pub(crate) const SLOTS_PER_ACTION: usize = 2 * SQUAD_MAX + 1;

/// Every possible key: 36 actions times 81 player slots.
pub const KEY_COUNT: usize = Action::ALL.len() * SLOTS_PER_ACTION;

/// The part of the engine a draw belongs to; the number is its stream-id code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subsystem {
    Decision = 1,
    Kick = 2,
    Ball = 3,
    Laws = 4,
    Shootout = 5,
    Fatigue = 6,
}

impl Subsystem {
    pub fn name(self) -> &'static str {
        match self {
            Subsystem::Decision => "decision",
            Subsystem::Kick => "kick",
            Subsystem::Ball => "ball",
            Subsystem::Laws => "laws",
            Subsystem::Shootout => "shootout",
            Subsystem::Fatigue => "fatigue",
        }
    }
}

/// Who a row's draws are keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerKind {
    /// The player who acts: the carrier, the kicker, the blocker, the keeper, the tackler,
    /// the offender, or the player who rolls for an injury.
    Actor,
    /// A named match key: the draw involves no one player.
    Match,
}

/// The scripted-draw queue a test scene may feed a row from, before the stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// The referee's queue: tackles, cards, added time, tosses, blocks, saves, clearances.
    Referee,
    /// The injury queue.
    Injury,
    /// Never scripted.
    Unscripted,
}

/// The kind of action of one draw. The discriminant indexes [`TABLE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    ShotScore,
    PassScore,
    DribbleScore,
    HoldScore,
    ClearScore,
    PassAim,
    ClearWide,
    ClearAim,
    ShotSide,
    ShotAim,
    ShotSpread,
    ShotLoft,
    Block,
    BlockDeflect,
    Save,
    SaveHold,
    ParrySide,
    ParryAngle,
    ParryLoft,
    CrossClear,
    CrossWide,
    CrossAngle,
    CrossLoft,
    KeeperCatch,
    Tackle,
    FoulCard,
    ExtraTimeAdded,
    AddedTime,
    ExtraKickOff,
    ShootoutFirstTeam,
    ShootoutEnd,
    KeeperDive,
    ShootoutSave,
    ShootoutSaveHold,
    InjuryMinute,
    InjuryTackle,
}

impl Action {
    /// Every action in table order.
    pub const ALL: [Action; 36] = [
        Action::ShotScore,
        Action::PassScore,
        Action::DribbleScore,
        Action::HoldScore,
        Action::ClearScore,
        Action::PassAim,
        Action::ClearWide,
        Action::ClearAim,
        Action::ShotSide,
        Action::ShotAim,
        Action::ShotSpread,
        Action::ShotLoft,
        Action::Block,
        Action::BlockDeflect,
        Action::Save,
        Action::SaveHold,
        Action::ParrySide,
        Action::ParryAngle,
        Action::ParryLoft,
        Action::CrossClear,
        Action::CrossWide,
        Action::CrossAngle,
        Action::CrossLoft,
        Action::KeeperCatch,
        Action::Tackle,
        Action::FoulCard,
        Action::ExtraTimeAdded,
        Action::AddedTime,
        Action::ExtraKickOff,
        Action::ShootoutFirstTeam,
        Action::ShootoutEnd,
        Action::KeeperDive,
        Action::ShootoutSave,
        Action::ShootoutSaveHold,
        Action::InjuryMinute,
        Action::InjuryTackle,
    ];

    /// The action's table row.
    pub fn row(self) -> &'static Row {
        &TABLE[self as usize]
    }
}

/// One table row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub action: Action,
    pub subsystem: Subsystem,
    /// The action's code inside its subsystem.
    pub code: u8,
    pub name: &'static str,
    pub kind: PlayerKind,
    pub class: Class,
}

const fn row(
    action: Action,
    subsystem: Subsystem,
    code: u8,
    name: &'static str,
    kind: PlayerKind,
    class: Class,
) -> Row {
    Row {
        action,
        subsystem,
        code,
        name,
        kind,
        class,
    }
}

use Class::{Injury, Referee, Unscripted};
use PlayerKind::{Actor, Match};
use Subsystem::{Ball, Decision, Fatigue, Kick, Laws, Shootout};

/// The stream-id table, one row per draw call in match play, in [`Action`] order.
pub const TABLE: [Row; 36] = [
    row(
        Action::ShotScore,
        Decision,
        1,
        "shot_score",
        Actor,
        Unscripted,
    ),
    row(
        Action::PassScore,
        Decision,
        2,
        "pass_score",
        Actor,
        Unscripted,
    ),
    row(
        Action::DribbleScore,
        Decision,
        3,
        "dribble_score",
        Actor,
        Unscripted,
    ),
    row(
        Action::HoldScore,
        Decision,
        4,
        "hold_score",
        Actor,
        Unscripted,
    ),
    row(
        Action::ClearScore,
        Decision,
        5,
        "clear_score",
        Actor,
        Unscripted,
    ),
    row(Action::PassAim, Kick, 1, "pass_aim", Actor, Unscripted),
    row(Action::ClearWide, Kick, 2, "clear_wide", Actor, Referee),
    row(Action::ClearAim, Kick, 3, "clear_aim", Actor, Unscripted),
    row(Action::ShotSide, Kick, 4, "shot_side", Actor, Unscripted),
    row(Action::ShotAim, Kick, 5, "shot_aim", Actor, Unscripted),
    row(
        Action::ShotSpread,
        Kick,
        6,
        "shot_spread",
        Actor,
        Unscripted,
    ),
    row(Action::ShotLoft, Kick, 7, "shot_loft", Actor, Unscripted),
    row(Action::Block, Ball, 1, "block", Actor, Referee),
    row(
        Action::BlockDeflect,
        Ball,
        2,
        "block_deflect",
        Actor,
        Unscripted,
    ),
    row(Action::Save, Ball, 3, "save", Actor, Referee),
    row(Action::SaveHold, Ball, 4, "save_hold", Actor, Referee),
    row(Action::ParrySide, Ball, 5, "parry_side", Actor, Unscripted),
    row(
        Action::ParryAngle,
        Ball,
        6,
        "parry_angle",
        Actor,
        Unscripted,
    ),
    row(Action::ParryLoft, Ball, 7, "parry_loft", Actor, Unscripted),
    row(Action::CrossClear, Ball, 8, "cross_clear", Actor, Referee),
    row(Action::CrossWide, Ball, 9, "cross_wide", Actor, Referee),
    row(
        Action::CrossAngle,
        Ball,
        10,
        "cross_angle",
        Actor,
        Unscripted,
    ),
    row(Action::CrossLoft, Ball, 11, "cross_loft", Actor, Unscripted),
    row(
        Action::KeeperCatch,
        Ball,
        12,
        "keeper_catch",
        Actor,
        Unscripted,
    ),
    row(Action::Tackle, Laws, 1, "tackle", Actor, Referee),
    row(Action::FoulCard, Laws, 2, "foul_card", Actor, Referee),
    row(
        Action::ExtraTimeAdded,
        Laws,
        3,
        "extra_time_added",
        Match,
        Referee,
    ),
    row(Action::AddedTime, Laws, 4, "added_time", Match, Referee),
    row(
        Action::ExtraKickOff,
        Laws,
        5,
        "extra_kick_off",
        Match,
        Referee,
    ),
    row(
        Action::ShootoutFirstTeam,
        Shootout,
        1,
        "first_team",
        Match,
        Referee,
    ),
    row(Action::ShootoutEnd, Shootout, 2, "end", Match, Referee),
    row(
        Action::KeeperDive,
        Shootout,
        3,
        "keeper_dive",
        Actor,
        Unscripted,
    ),
    row(Action::ShootoutSave, Shootout, 4, "save", Actor, Referee),
    row(
        Action::ShootoutSaveHold,
        Shootout,
        5,
        "save_hold",
        Actor,
        Referee,
    ),
    row(
        Action::InjuryMinute,
        Fatigue,
        1,
        "injury_minute",
        Actor,
        Injury,
    ),
    row(
        Action::InjuryTackle,
        Fatigue,
        2,
        "injury_tackle",
        Actor,
        Injury,
    ),
];

/// Who a key names: `team << 8 | squad index`, or [`PlayerKey::MATCH`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerKey(pub u16);

impl PlayerKey {
    /// The named match key, for a draw that involves no one player.
    pub const MATCH: PlayerKey = PlayerKey(0xFFFF);

    /// The key of `p`: his team and his squad index. A squad index past 255 is clamped to
    /// 255, which the table check refuses.
    pub fn of(p: &Player) -> Self {
        Self::of_squad(p.team, p.squad)
    }

    /// The key of squad player `squad` of `team`.
    pub fn of_squad(team: usize, squad: usize) -> Self {
        // Both values are clamped into one byte first, so the casts cannot truncate.
        PlayerKey(((team.min(0xFF) as u16) << 8) | squad.min(0xFF) as u16)
    }

    pub fn team(self) -> usize {
        usize::from(self.0 >> 8)
    }

    pub fn squad(self) -> usize {
        usize::from(self.0 & 0xFF)
    }

    /// The key's slot among an action's 81, or `None` for a key outside both squads.
    fn slot(self) -> Option<usize> {
        if self == Self::MATCH {
            return Some(2 * SQUAD_MAX);
        }
        (self.team() < 2 && self.squad() < SQUAD_MAX)
            .then(|| self.team() * SQUAD_MAX + self.squad())
    }
}

impl std::fmt::Display for PlayerKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Self::MATCH {
            write!(f, "match")
        } else {
            write!(f, "team {} squad {}", self.team(), self.squad())
        }
    }
}

/// One stream key: a table row and who it is drawn for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub action: Action,
    pub player: PlayerKey,
}

impl Key {
    /// A draw for player `p`, who acts.
    pub fn player(action: Action, p: &Player) -> Self {
        Self {
            action,
            player: PlayerKey::of(p),
        }
    }

    /// A draw on the named match key.
    pub fn of_match(action: Action) -> Self {
        Self {
            action,
            player: PlayerKey::MATCH,
        }
    }

    /// `true` when the table holds this key: the player kind matches the row, and a player
    /// key names a team of 0 or 1 and a squad index below 40.
    pub fn registered(self) -> bool {
        match self.action.row().kind {
            PlayerKind::Match => self.player == PlayerKey::MATCH,
            PlayerKind::Actor => self.player != PlayerKey::MATCH && self.player.slot().is_some(),
        }
    }

    /// The key's place in the dense index of all [`KEY_COUNT`] keys, or `None` when the table
    /// does not hold it.
    pub(crate) fn index(self) -> Option<usize> {
        if !self.registered() {
            return None;
        }
        Some(self.action as usize * SLOTS_PER_ACTION + self.player.slot()?)
    }

    /// The keyed scheme's stream id of this key.
    pub fn stream_id(self) -> u64 {
        let row = self.action.row();
        (row.subsystem as u64) << SUBSYSTEM_SHIFT
            | u64::from(row.code) << ACTION_SHIFT
            | u64::from(self.player.0)
    }

    /// The key a keyed stream id derives from, or `None` when no table key derives it.
    pub fn from_stream_id(id: u64) -> Option<Self> {
        let subsystem = id >> SUBSYSTEM_SHIFT;
        let code = (id >> ACTION_SHIFT) & 0xFFFF;
        let player = id & 0xFFFF_FFFF;
        let row = TABLE
            .iter()
            .find(|r| r.subsystem as u64 == subsystem && u64::from(r.code) == code)?;
        let key = Key {
            action: row.action,
            player: PlayerKey(u16::try_from(player).ok()?),
        };
        key.registered().then_some(key)
    }

    /// `subsystem.action player`, as a refusal names it.
    pub fn name(self) -> String {
        let row = self.action.row();
        format!("{}.{} {}", row.subsystem.name(), row.name, self.player)
    }
}

const SUBSYSTEM_SHIFT: u32 = 48;
const ACTION_SHIFT: u32 = 32;

/// The committed digest of each scheme's table, key derivation, and draw conversion. A test
/// fails when [`digest`] gives another value: the table, the derivation, or the conversion
/// changed, and the change needs a new scheme id and a new line here.
pub const SCHEME_DIGESTS: [(u8, &str); 1] = [(
    1,
    "91652878ed8a8c15d99f0a6fe956e03e55b9d248e1176b67679ffd602504a384",
)];

/// The SHA-256, as lowercase hex, of what fixes `scheme`: the scheme id, every row (the
/// subsystem, the action code, the name, the player kind, and the queue class), the key
/// derivation (the two shifts, the team shift, and the match key), and known-answer draws:
/// the first three draws' bits from four sample keys of a seed-42 registry in that scheme.
#[cfg(any(test, feature = "scenario"))]
pub fn digest(scheme: super::Scheme) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update([scheme.id()]);
    for r in &TABLE {
        h.update([r.subsystem as u8, r.code]);
        h.update(r.name.as_bytes());
        h.update([0, r.kind as u8, r.class as u8]);
    }
    h.update(SUBSYSTEM_SHIFT.to_le_bytes());
    h.update(ACTION_SHIFT.to_le_bytes());
    h.update(8u32.to_le_bytes());
    h.update(PlayerKey::MATCH.0.to_le_bytes());
    let mut streams = super::Streams::keyed(42);
    for key in [
        Key {
            action: Action::ShotScore,
            player: PlayerKey::of_squad(0, 3),
        },
        Key {
            action: Action::Tackle,
            player: PlayerKey::of_squad(1, 7),
        },
        Key::of_match(Action::AddedTime),
        Key {
            action: Action::InjuryMinute,
            player: PlayerKey::of_squad(0, 3),
        },
    ] {
        for _ in 0..3 {
            h.update(streams.draw(key).to_bits().to_le_bytes());
        }
    }
    crate::gate::hex(&h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key of the table, in index order.
    fn every_key() -> Vec<Key> {
        let mut keys = Vec::new();
        for action in Action::ALL {
            match action.row().kind {
                PlayerKind::Match => keys.push(Key::of_match(action)),
                PlayerKind::Actor => {
                    for team in 0..2 {
                        for squad in 0..SQUAD_MAX {
                            keys.push(Key {
                                action,
                                player: PlayerKey::of_squad(team, squad),
                            });
                        }
                    }
                }
            }
        }
        keys
    }

    #[test]
    fn every_stream_id_is_unique_non_zero_and_derives_its_key_back() {
        let keys = every_key();
        let mut ids: Vec<u64> = keys.iter().map(|k| k.stream_id()).collect();
        assert!(ids.iter().all(|&id| id != 0));
        for key in &keys {
            assert_eq!(Key::from_stream_id(key.stream_id()), Some(*key));
            assert!(key.index().unwrap() < KEY_COUNT);
        }
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), keys.len());
        let mut indices: Vec<usize> = keys.iter().map(|k| k.index().unwrap()).collect();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices.len(), keys.len());
    }

    #[test]
    fn each_row_sits_at_its_action_and_the_classes_match_todays_split() {
        for action in Action::ALL {
            assert_eq!(TABLE[action as usize].action, action);
        }
        let count = |c: Class| TABLE.iter().filter(|r| r.class == c).count();
        assert_eq!(count(Class::Referee), 15);
        assert_eq!(count(Class::Injury), 2);
        assert_eq!(count(Class::Unscripted), 19);
        let mut codes: Vec<(u8, u8)> = TABLE.iter().map(|r| (r.subsystem as u8, r.code)).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), TABLE.len(), "one code per row");
    }

    #[test]
    fn a_key_outside_the_table_is_not_registered_and_derives_nothing() {
        let wrong_kind = Key {
            action: Action::AddedTime,
            player: PlayerKey::of_squad(0, 3),
        };
        assert!(!wrong_kind.registered());
        assert!(!Key::of_match(Action::Tackle).registered());
        let squad_40 = Key {
            action: Action::Tackle,
            player: PlayerKey::of_squad(0, 40),
        };
        assert!(!squad_40.registered());
        assert_eq!(Key::from_stream_id(squad_40.stream_id()), None);
        assert_eq!(Key::from_stream_id(0), None);
        assert_eq!(Key::from_stream_id(7 << 48 | 1 << 32), None);
        assert_eq!(
            wrong_kind.name(),
            "laws.added_time team 0 squad 3",
            "a refusal names the key"
        );
    }
}
