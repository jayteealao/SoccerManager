//! Facsimile names. A club's name comes from the game's own club-name generator, seeded per
//! club from its source id, and is drawn again while it equals a real name: its folded form
//! against every folded real name, its short name against every real name of any length, and
//! its name against the names already used in its country. League and cup names are built
//! from the country code and are checked against the same real names.

use std::collections::{BTreeSet, HashSet};

use anyhow::bail;
use engine::data::names;
use engine::rng::EngineRng;
use sha2::{Digest, Sha256};

/// Draws before a club's name is given up on.
pub const MAX_DRAWS: usize = 64;

/// Club-form words dropped from either end of a name before names are compared.
const CLUB_FORMS: [&str; 7] = ["fc", "afc", "cf", "sc", "ac", "fk", "sk"];

/// `c` without its diacritic, for the Latin-1 and Latin Extended-A letters.
fn plain(c: char) -> &'static str {
    match c {
        'à'..='å' | 'ā' | 'ă' | 'ą' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'ď' | 'đ' | 'ð' => "d",
        'è'..='ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        'ì'..='ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'ĵ' => "j",
        'ķ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' => "n",
        'ò'..='ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'œ' => "oe",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' => "s",
        'ß' => "ss",
        'ţ' | 'ť' | 'ŧ' => "t",
        'ù'..='ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        'þ' => "th",
        _ => "",
    }
}

/// A name for comparison: lowercase, diacritics folded, one club-form word dropped from the
/// front and one from the end, letters and digits only.
pub fn fold(name: &str) -> String {
    let mut words: Vec<String> = name
        .to_lowercase()
        .split_whitespace()
        .map(|w| {
            w.chars()
                .map(|c| match plain(c) {
                    "" if c.is_alphanumeric() => c.to_string(),
                    p => p.to_string(),
                })
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect();
    if words.len() > 1 && CLUB_FORMS.contains(&words[0].as_str()) {
        words.remove(0);
    }
    if words.len() > 1 && CLUB_FORMS.contains(&words[words.len() - 1].as_str()) {
        words.pop();
    }
    words.concat()
}

/// The real names a facsimile must not equal.
#[derive(Debug, Default)]
pub struct RealNames {
    /// Every real name, trimmed and lowercased.
    lower: HashSet<String>,
    /// Every real name, folded.
    folded: HashSet<String>,
}

impl RealNames {
    pub fn add(&mut self, name: &str) {
        let t = name.trim();
        if t.is_empty() {
            return;
        }
        self.lower.insert(t.to_lowercase());
        let f = fold(t);
        if !f.is_empty() {
            self.folded.insert(f);
        }
    }

    /// Whether `name` equals a real name, folded, or trimmed and lowercased.
    pub fn clashes(&self, name: &str) -> bool {
        self.lower.contains(&name.trim().to_lowercase()) || self.folded.contains(&fold(name))
    }

    /// Whether `short` equals a real name of any length, trimmed and lowercased.
    pub fn short_clashes(&self, short: &str) -> bool {
        self.lower.contains(&short.trim().to_lowercase())
    }

    pub fn len(&self) -> usize {
        self.lower.len()
    }
}

/// The 64-bit seed of an entity: the first 8 bytes of SHA-256 over `tag:id`.
pub fn seed(tag: &str, id: &str) -> u64 {
    let d = Sha256::digest(format!("{tag}:{id}").as_bytes());
    u64::from_le_bytes(d[..8].try_into().expect("8 bytes"))
}

/// The facsimile name and short name of the club `id`, drawn again while it clashes with a
/// real name or with a name in `used` (the names of its country so far).
pub fn club_name(
    id: &str,
    used: &BTreeSet<String>,
    real: &RealNames,
) -> anyhow::Result<(String, String)> {
    let mut rng = EngineRng::from_seed(seed("world-club", id));
    for _ in 0..MAX_DRAWS {
        let (name, short) = names::club_name(&mut rng);
        if !real.clashes(&name) && !real.short_clashes(&short) && !used.contains(&name) {
            return Ok((name, short));
        }
    }
    bail!("club {id}: no facsimile name clear of the real names in {MAX_DRAWS} draws")
}

/// A league or cup name built from a country code (or region), refused when it equals a
/// real name.
pub fn structural_name(name: String, real: &RealNames) -> anyhow::Result<String> {
    if real.clashes(&name) {
        bail!("the facsimile name {name:?} equals a real name");
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folding_drops_case_marks_and_club_forms() {
        assert_eq!(fold("Ashford United F.C."), "ashfordunited");
        assert_eq!(fold("FC Köln"), "koln");
        assert_eq!(fold("  Ålesund  "), "alesund");
        assert_eq!(fold("FC"), "fc");
    }

    #[test]
    fn the_same_id_gives_the_same_name() {
        let real = RealNames::default();
        let used = BTreeSet::new();
        assert_eq!(
            club_name("Q1", &used, &real).unwrap(),
            club_name("Q1", &used, &real).unwrap()
        );
        assert_ne!(
            club_name("Q1", &used, &real).unwrap(),
            club_name("Q2", &used, &real).unwrap()
        );
    }

    #[test]
    fn a_planted_real_name_forces_a_redraw() {
        let first = club_name("Q1", &BTreeSet::new(), &RealNames::default()).unwrap();
        let mut real = RealNames::default();
        real.add(&format!("{} F.C.", first.0));
        let again = club_name("Q1", &BTreeSet::new(), &real).unwrap();
        assert_ne!(again.0, first.0);
        assert!(!real.clashes(&again.0));
    }

    #[test]
    fn a_planted_real_short_name_forces_a_redraw() {
        let first = club_name("Q1", &BTreeSet::new(), &RealNames::default()).unwrap();
        let mut real = RealNames::default();
        real.add(&first.1.to_lowercase());
        let again = club_name("Q1", &BTreeSet::new(), &real).unwrap();
        assert_ne!(again.1, first.1);
    }

    #[test]
    fn a_used_name_forces_a_redraw() {
        let first = club_name("Q1", &BTreeSet::new(), &RealNames::default()).unwrap();
        let used = BTreeSet::from([first.0.clone()]);
        assert_ne!(
            club_name("Q1", &used, &RealNames::default()).unwrap().0,
            first.0
        );
    }

    #[test]
    fn sixty_four_clashes_fail_naming_the_club() {
        let mut rng = EngineRng::from_seed(seed("world-club", "Q1"));
        let mut real = RealNames::default();
        for _ in 0..MAX_DRAWS {
            real.add(&names::club_name(&mut rng).0);
        }
        let e = club_name("Q1", &BTreeSet::new(), &real)
            .unwrap_err()
            .to_string();
        assert!(e.contains("Q1"), "{e}");
    }

    #[test]
    fn a_structural_name_equal_to_a_real_name_is_refused() {
        let mut real = RealNames::default();
        real.add("TST tier 1");
        assert!(structural_name("TST tier 1".into(), &real).is_err());
        assert!(structural_name("TST tier 2".into(), &real).is_ok());
    }
}
