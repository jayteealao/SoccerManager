//! Module cards: what each module is, reads, writes, tunes, is calibrated against, and which
//! action keys it owns. The card check refuses an empty field; the ownership check refuses a
//! key with two owners or none.

use crate::streams::Action;

/// The action keys drawn by the parts that have moved onto the contract: every row of the
/// stream table.
pub const MOVED_KEYS: &[Action] = &[
    Action::Tackle,
    Action::FoulCard,
    Action::Save,
    Action::ShootoutSave,
    Action::InjuryMinute,
    Action::InjuryTackle,
    Action::AddedTime,
    Action::ExtraTimeAdded,
    Action::ExtraKickOff,
    Action::ShootoutFirstTeam,
    Action::ShootoutEnd,
    Action::KeeperDive,
    Action::ShootoutSaveHold,
    Action::BlockDeflect,
    Action::ParryAngle,
    Action::ParryLoft,
    Action::CrossAngle,
    Action::CrossLoft,
    Action::Block,
    Action::SaveHold,
    Action::ParrySide,
    Action::CrossClear,
    Action::CrossWide,
    Action::KeeperCatch,
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
    Action::Lapse,
    Action::Header,
    Action::FormMatch,
    Action::FormPeriod,
];

/// What one module is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleCard {
    /// What the module does, in one sentence.
    pub purpose: &'static str,
    /// What it reads from the view.
    pub inputs: &'static str,
    /// What it returns: results or proposed changes.
    pub outputs: &'static str,
    /// The tuning values it reads, or the one entry `"none"`.
    pub tuning: &'static [&'static str],
    /// A band name from `content/realism-bands.json`, or `none: <reason>`.
    pub calibration: &'static str,
    /// The action keys whose draws the loop takes for this module.
    pub keys: &'static [Action],
}

/// Why a card fails the check.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CardError {
    #[error("card field {field} is empty")]
    Empty { field: &'static str },
    #[error(
        "card calibration {value:?} names no band in realism-bands.json and is not `none: <reason>`"
    )]
    Calibration { value: String },
}

/// Checks that every field of `card` is filled. `band_names` are the band keys of
/// `content/realism-bands.json`.
pub fn check_card(card: &ModuleCard, band_names: &[&str]) -> Result<(), CardError> {
    for (field, value) in [
        ("purpose", card.purpose),
        ("inputs", card.inputs),
        ("outputs", card.outputs),
        ("calibration", card.calibration),
    ] {
        if value.trim().is_empty() {
            return Err(CardError::Empty { field });
        }
    }
    if card.tuning.is_empty() || card.tuning.iter().any(|t| t.trim().is_empty()) {
        return Err(CardError::Empty { field: "tuning" });
    }
    let calibration = card.calibration.trim();
    let reasoned = calibration
        .strip_prefix("none:")
        .is_some_and(|reason| !reason.trim().is_empty());
    if !reasoned && !band_names.contains(&calibration) {
        return Err(CardError::Calibration {
            value: card.calibration.to_string(),
        });
    }
    Ok(())
}

/// Why the key-ownership check fails.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OwnershipError {
    #[error("action key {key:?} is claimed by {owners:?}")]
    ClaimedTwice {
        key: Action,
        owners: Vec<&'static str>,
    },
    #[error("action key {key:?} has no owner")]
    Unclaimed { key: Action },
}

/// Checks that every key in `required` is owned by exactly one card in `cards` and that no
/// card claims a key twice across modules. `cards` pairs each module name with its card.
pub fn check_ownership(
    cards: &[(&'static str, &ModuleCard)],
    required: &[Action],
) -> Result<(), OwnershipError> {
    let owners_of = |key: Action| -> Vec<&'static str> {
        cards
            .iter()
            .filter(|(_, card)| card.keys.contains(&key))
            .map(|(name, _)| *name)
            .collect()
    };
    let mut claimed: Vec<Action> = cards
        .iter()
        .flat_map(|(_, card)| card.keys.iter().copied())
        .collect();
    claimed.extend_from_slice(required);
    for key in claimed {
        let owners = owners_of(key);
        match owners.len() {
            0 => return Err(OwnershipError::Unclaimed { key }),
            1 => {}
            _ => return Err(OwnershipError::ClaimedTwice { key, owners }),
        }
    }
    Ok(())
}
