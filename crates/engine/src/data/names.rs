//! Syllable tables for fictional club and player names, and a kit colour table.
//! Nothing here names a real club or a real person.

use crate::rng::EngineRng;

const TOWN_STARTS: [&str; 24] = [
    "Ash", "Bel", "Cal", "Dun", "Eld", "Far", "Gil", "Hal", "Ives", "Kel", "Lan", "Mar", "Nor",
    "Oak", "Pen", "Quen", "Ros", "Sal", "Tor", "Ul", "Vel", "Wen", "Yar", "Zel",
];
const TOWN_ENDS: [&str; 12] = [
    "ford", "bury", "wick", "ton", "mere", "haven", "dale", "field", "moor", "port", "stead",
    "worth",
];
const CLUB_SUFFIXES: [&str; 8] = [
    "United",
    "Town",
    "Athletic",
    "Rovers",
    "City",
    "Wanderers",
    "Albion",
    "Rangers",
];
const FIRST_STARTS: [&str; 20] = [
    "Al", "Ba", "Ce", "Da", "El", "Fa", "Go", "Ha", "Ida", "Jo", "Ka", "Le", "Ma", "Ni", "Or",
    "Pe", "Ra", "Sa", "Te", "Vi",
];
const FIRST_ENDS: [&str; 12] = [
    "ric", "mon", "lan", "vin", "den", "nis", "ro", "sef", "mil", "ton", "dor", "ren",
];
const LAST_STARTS: [&str; 20] = [
    "Ab", "Bro", "Car", "Dal", "Er", "Fen", "Gar", "Hol", "Iv", "Jen", "Kir", "Lom", "Mor", "Nar",
    "Ost", "Pal", "Ren", "Sto", "Tav", "Wal",
];
const LAST_ENDS: [&str; 12] = [
    "sen", "ley", "dine", "ker", "rick", "man", "ton", "ez", "ova", "berg", "ini", "wood",
];

/// Twelve kit colours as `#RRGGBB`.
pub const KIT_COLOURS: [&str; 12] = [
    "#c8102e", "#003087", "#ffffff", "#000000", "#ffd100", "#008751", "#6a0dad", "#ff6a13",
    "#87ceeb", "#800000", "#1e90ff", "#f5f5dc",
];

fn pick<'a>(rng: &mut EngineRng, table: &[&'a str]) -> &'a str {
    table[rng.range_usize(table.len())]
}

/// A club name and its short form, for example ("Ashford United", "ASH").
pub fn club_name(rng: &mut EngineRng) -> (String, String) {
    let start = pick(rng, &TOWN_STARTS);
    let town = format!("{start}{}", pick(rng, &TOWN_ENDS));
    let short: String = town.chars().take(3).collect::<String>().to_uppercase();
    (format!("{town} {}", pick(rng, &CLUB_SUFFIXES)), short)
}

/// A player name, for example "Alric Brosen".
pub fn player_name(rng: &mut EngineRng) -> String {
    format!(
        "{}{} {}{}",
        pick(rng, &FIRST_STARTS),
        pick(rng, &FIRST_ENDS),
        pick(rng, &LAST_STARTS),
        pick(rng, &LAST_ENDS)
    )
}

/// Two distinct kit colours.
pub fn kit(rng: &mut EngineRng) -> (String, String) {
    let primary = rng.range_usize(KIT_COLOURS.len());
    let secondary = (primary + 1 + rng.range_usize(KIT_COLOURS.len() - 1)) % KIT_COLOURS.len();
    (KIT_COLOURS[primary].into(), KIT_COLOURS[secondary].into())
}
