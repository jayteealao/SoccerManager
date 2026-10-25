//! The SoccerTrack v2 readers: the game-state file of a half and, when present, the
//! ball-action file of a match.
//!
//! A game-state file (`<match>_1st.json`, `<match>_2nd.json`, 2.7 to 2.9 GB) is read as a
//! stream, one annotation at a time: its `images` are skipped, and of its `annotations` only
//! the players and keepers are kept, each with its image, track id within the half, side
//! (`left` or `right` in that half) and pitch position (`bbox_pitch`, the bottom middle of
//! the box, metres from the centre spot). Every other annotation (the pitch lines, the
//! camera, the referees and any ball) is dropped by the reader and never reaches its output,
//! and the reader computes no speed: the vendor positions come in 1.05-metre steps, so a
//! speed from them means nothing. The objects are handed on one image at a time, in image
//! order.
//!
//! A ball-action file (`bas/<match>/<match>_12_class_events.json`) gives each action's
//! `gameTime` (`<half> - mm:ss`) and `label`; the reader counts the passes by choice
//! (`Pass` a ground pass, `High Pass`, `Cross`) per half and refuses a label outside the
//! dataset's twelve.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

use super::contrast::PassChoice;

/// A player or keeper in one image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Object {
    /// The image's number in the half.
    pub image: u64,
    pub track: u32,
    pub keeper: bool,
    /// 0 for the side named `left` in this half, 1 for `right`.
    pub side: usize,
    pub x: f32,
    pub y: f32,
}

#[derive(Deserialize)]
struct Attributes {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    team: Option<String>,
}

#[derive(Deserialize)]
struct PitchBox {
    x_bottom_middle: f64,
    y_bottom_middle: f64,
}

#[derive(Deserialize)]
struct Annotation {
    #[serde(default)]
    image_id: Option<String>,
    #[serde(default)]
    track_id: Option<u32>,
    #[serde(default)]
    supercategory: Option<String>,
    #[serde(default)]
    attributes: Option<Attributes>,
    #[serde(default)]
    bbox_pitch: Option<PitchBox>,
}

impl Annotation {
    /// The player or keeper this annotation is, or `None` for anything else.
    fn object(self) -> Option<Object> {
        if self.supercategory.as_deref() != Some("object") {
            return None;
        }
        let a = self.attributes?;
        let keeper = match a.role.as_deref()? {
            "player" => false,
            "goalkeeper" => true,
            _ => return None,
        };
        let side = match a.team.as_deref()? {
            "left" => 0,
            "right" => 1,
            _ => return None,
        };
        let b = self.bbox_pitch?;
        Some(Object {
            image: self.image_id?.parse().ok()?,
            track: self.track_id?,
            keeper,
            side,
            x: b.x_bottom_middle as f32,
            y: b.y_bottom_middle as f32,
        })
    }
}

/// Hands the objects of each image on, in image order.
struct Images<'a> {
    image: Option<u64>,
    objects: Vec<Object>,
    each: &'a mut dyn FnMut(u64, &[Object]) -> anyhow::Result<()>,
}

impl Images<'_> {
    fn push(&mut self, o: Object) -> Result<(), String> {
        match self.image {
            Some(i) if i == o.image => {}
            Some(i) if o.image < i => {
                return Err(format!("image {} follows image {i}", o.image));
            }
            _ => self.flush()?,
        }
        self.image = Some(o.image);
        self.objects.push(o);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        if let Some(i) = self.image
            && !self.objects.is_empty()
        {
            (self.each)(i, &self.objects).map_err(|e| e.to_string())?;
        }
        self.objects.clear();
        Ok(())
    }
}

struct AnnotationsSeed<'a, 'b>(&'a mut Images<'b>);

impl<'de> DeserializeSeed<'de> for AnnotationsSeed<'_, '_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for AnnotationsSeed<'_, '_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a list of annotations")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while let Some(a) = seq.next_element::<Annotation>()? {
            if let Some(o) = a.object() {
                self.0.push(o).map_err(de::Error::custom)?;
            }
        }
        Ok(())
    }
}

struct FileSeed<'a, 'b>(&'a mut Images<'b>);

impl<'de> DeserializeSeed<'de> for FileSeed<'_, '_> {
    type Value = bool;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<bool, D::Error> {
        d.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for FileSeed<'_, '_> {
    type Value = bool;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a SoccerTrack game-state file")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<bool, A::Error> {
        let mut seen = false;
        while let Some(key) = map.next_key::<String>()? {
            if key == "annotations" {
                map.next_value_seed(AnnotationsSeed(&mut *self.0))?;
                seen = true;
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(seen)
    }
}

/// Reads a game-state file, handing on the players and keepers of each image in order,
/// with the image's number counted from the file's first image.
pub fn read_half(
    input: impl Read,
    each: &mut dyn FnMut(u64, &[Object]) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut first: Option<u64> = None;
    let mut relative = |image: u64, objects: &[Object]| -> anyhow::Result<()> {
        let base = *first.get_or_insert(image);
        each(image - base, objects)
    };
    let mut images = Images {
        image: None,
        objects: Vec::new(),
        each: &mut relative,
    };
    let mut de =
        serde_json::Deserializer::from_reader(std::io::BufReader::with_capacity(1 << 20, input));
    let seen = FileSeed(&mut images)
        .deserialize(&mut de)
        .map_err(|e| anyhow::anyhow!("not a SoccerTrack game-state file: {e}"))?;
    anyhow::ensure!(seen, "not a SoccerTrack game-state file: no annotations");
    images.flush().map_err(anyhow::Error::msg)?;
    Ok(())
}

/// The game-state files of `dir` by match: `(match, [first half, second half])`, in match
/// order; a match needs both halves.
pub fn matches(dir: &Path) -> anyhow::Result<Vec<(String, [PathBuf; 2])>> {
    let mut out = Vec::new();
    let read = std::fs::read_dir(dir).with_context(|| format!("cannot read {}", dir.display()))?;
    let mut names: Vec<String> = read
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    for n in &names {
        let Some(id) = n.strip_suffix("_1st.json") else {
            continue;
        };
        let second = format!("{id}_2nd.json");
        anyhow::ensure!(
            names.contains(&second),
            "{} has the first half of {id} and not its second",
            dir.display()
        );
        out.push((id.to_string(), [dir.join(n), dir.join(second)]));
    }
    Ok(out)
}

/// The twelve labels of the ball-action files.
pub const LABELS: [&str; 12] = [
    "Pass",
    "Drive",
    "Header",
    "High Pass",
    "Out",
    "Cross",
    "Throw In",
    "Shot",
    "Ball Player Block",
    "Player Successful Tackle",
    "Free Kick",
    "Goal",
];

#[derive(Deserialize)]
struct ActionFile {
    annotations: Vec<Action>,
}

#[derive(Deserialize)]
struct Action {
    #[serde(rename = "gameTime")]
    game_time: String,
    label: String,
}

/// The pass choice of each half from a ball-action file's text; a label outside the twelve
/// is refused.
pub fn pass_choice(text: &str) -> anyhow::Result<Vec<PassChoice>> {
    let file: ActionFile =
        serde_json::from_str(text).context("not a SoccerTrack ball-action file")?;
    let mut halves = vec![PassChoice::default(); 2];
    for a in &file.annotations {
        anyhow::ensure!(
            LABELS.contains(&a.label.as_str()),
            "unknown ball-action label {:?}; the file's labels are {}",
            a.label,
            LABELS.join(", ")
        );
        let half: usize = a
            .game_time
            .split_once(" - ")
            .and_then(|(h, _)| h.trim().parse().ok())
            .with_context(|| format!("a ball action has the game time {:?}", a.game_time))?;
        anyhow::ensure!((1..=2).contains(&half), "a ball action is in half {half}");
        let c = &mut halves[half - 1];
        match a.label.as_str() {
            "Pass" => c.ground += 1,
            "High Pass" => c.high += 1,
            "Cross" => c.cross += 1,
            _ => {}
        }
    }
    Ok(halves)
}

/// The ball-action file of match `id` under `dir`, when present.
pub fn action_file(dir: &Path, id: &str) -> PathBuf {
    dir.join("bas")
        .join(id)
        .join(format!("{id}_12_class_events.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tracking/soccertrack")
            .join(name)
    }

    /// The reader hands on the players and keepers image by image; the planted ball, the
    /// referee and the pitch lines never reach its output.
    #[test]
    fn the_reader_keeps_players_and_keepers_and_nothing_else() {
        let file = std::fs::File::open(fixture("900001_1st.json")).unwrap();
        let mut images = Vec::new();
        read_half(file, &mut |i, objects| {
            images.push((i, objects.to_vec()));
            Ok(())
        })
        .unwrap();
        assert_eq!(images.len(), 3);
        assert_eq!(images[0].0, 0);
        assert_eq!(images[2].0, 2);
        for (_, objects) in &images {
            assert_eq!(objects.len(), 6, "{objects:?}");
            assert_eq!(objects.iter().filter(|o| o.keeper).count(), 2);
        }
        // The planted ball sits at (1.5, 2.5); no object stands there.
        assert!(
            images
                .iter()
                .flat_map(|(_, o)| o)
                .all(|o| (o.x, o.y) != (1.5, 2.5))
        );
    }

    #[test]
    fn a_file_without_annotations_is_refused() {
        let err = read_half(&b"{\"images\": []}"[..], &mut |_, _| Ok(())).unwrap_err();
        assert!(err.to_string().contains("no annotations"), "{err}");
        let err = read_half(&b"[1, 2]"[..], &mut |_, _| Ok(())).unwrap_err();
        assert!(err.to_string().contains("game-state"), "{err}");
    }

    /// Pass, High Pass and Cross count by half; Drive counts nowhere; an unknown label is
    /// refused.
    #[test]
    fn the_ball_actions_count_pass_choice_and_refuse_an_unknown_label() {
        let text =
            std::fs::read_to_string(fixture("bas/900001/900001_12_class_events.json")).unwrap();
        let c = pass_choice(&text).unwrap();
        assert_eq!(
            c[0],
            PassChoice {
                ground: 2,
                high: 1,
                cross: 1
            }
        );
        assert_eq!(
            c[1],
            PassChoice {
                ground: 1,
                high: 0,
                cross: 1
            }
        );
        let bad = r#"{"annotations": [{"gameTime": "1 - 00:01", "label": "Dribble"}]}"#;
        let err = pass_choice(bad).unwrap_err().to_string();
        assert!(
            err.contains("unknown ball-action label \"Dribble\""),
            "{err}"
        );
    }
}
