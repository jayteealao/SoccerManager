//! The rule pack slot (`game.rules`). Version 1 loads the content folder's rule file with the
//! same checks, refusals, and digest as every other content file. The off version reads no
//! custom rule pack: it loads the standard Laws built into the program, which are the bytes
//! of the shipped `content/rules/default.json`, so a match with the slot off still plays
//! under complete Laws.

use crate::data::{Loaded, RULES_FILE, RULES_VERSION, RulePack, load_json_bytes};
use crate::error::EngineError;
use crate::modules::{ModuleCard, RulesModule};

/// The standard Laws built into the program: the shipped rule file, byte for byte.
pub const STANDARD_LAWS: &[u8] = include_bytes!("../../../../content/rules/default.json");

/// How the off version names the built-in Laws in a refusal or a load signal.
pub const STANDARD_LAWS_SHOWN: &str = "built-in laws";

/// Version 1 of the rules slot: the content folder's rule file.
pub struct RulePackV1;

impl RulesModule for RulePackV1 {
    fn load(&self, written: &[u8]) -> Result<Loaded<RulePack>, EngineError> {
        load_json_bytes::<RulePack>("rules", written, RULES_FILE, RULES_VERSION, &())
    }
}

pub const RULE_PACK_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Loads the rule pack a match plays under from the content folder's rule file.",
    inputs: "The bytes of the content folder's rule file.",
    outputs: "The rule pack and its digest, or the refusal that names the file and the field.",
    tuning: &["none"],
    calibration: "none: rule pack loader, no realism band",
    keys: &[],
};

/// The rules slot's off version: the standard Laws built into the program.
pub struct RulePackOff;

impl RulesModule for RulePackOff {
    fn load(&self, _: &[u8]) -> Result<Loaded<RulePack>, EngineError> {
        load_json_bytes::<RulePack>(
            "rules",
            STANDARD_LAWS,
            STANDARD_LAWS_SHOWN,
            RULES_VERSION,
            &(),
        )
    }
}

pub const RULE_PACK_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "Loads the standard Laws built into the program; the content folder's rule file is not read.",
    inputs: "None: the content folder's rule file is ignored.",
    outputs: "The built-in rule pack and its digest.",
    tuning: &["none"],
    calibration: "none: rule pack loader, no realism band",
    keys: &[],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::read_bytes;
    use crate::data::test_support::shipped_dir;

    fn shipped_rules() -> Vec<u8> {
        read_bytes(&shipped_dir().path(RULES_FILE), RULES_FILE).unwrap()
    }

    #[test]
    fn rule_pack_v1_loads_the_written_file_as_the_direct_call_does() {
        let bytes = shipped_rules();
        let direct =
            load_json_bytes::<RulePack>("rules", &bytes, RULES_FILE, RULES_VERSION, &()).unwrap();
        let through = RulePackV1.load(&bytes).unwrap();
        assert_eq!(through.value, direct.value);
        assert_eq!(through.digest, direct.digest);
    }

    #[test]
    fn the_off_version_ignores_the_written_file_and_loads_the_built_in_laws() {
        let shipped = shipped_rules();
        assert_eq!(
            STANDARD_LAWS,
            shipped.as_slice(),
            "the built-in Laws are the shipped rule file"
        );
        let built_in = RulePackV1.load(STANDARD_LAWS).unwrap();
        for written in [&b"not json"[..], &shipped[..], &[][..]] {
            let off = RulePackOff.load(written).unwrap();
            assert_eq!(off.value, built_in.value);
            assert_eq!(off.digest, built_in.digest);
        }
    }
}
