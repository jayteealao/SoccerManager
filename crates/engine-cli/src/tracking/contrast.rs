//! The amateur-against-elite contrast: team shape and pass choice of the SoccerTrack v2
//! matches (amateur, game-state files) beside the DFL matches (elite, the tracking file).
//!
//! The shape figures are those of the shared report that need no ball, possession or speed:
//! each side's length (deepest to highest outfield player), width and spread (the outfield
//! players' mean distance from their centroid), averaged over every frame of a half and over
//! both sides, then over the matches with each match weighted alike. Pass choice is the
//! share of ground passes, high passes and crosses among the three, per half; SoccerTrack
//! gives it only from its ball-action files, and without them the contrast says so.
//!
//! The contrast holds nothing else: no target, no value for a band and no speed. SoccerTrack
//! data sets no target, and the reference file's intake refuses a contrast file.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::Group;
use super::measure::TrackingFile;
use super::soccertrack;
use crate::report::tracked_shape::{Frame, Holder, Mode, TrackedShape, TrackedValues, place};

/// The contrast file's schema.
pub const SCHEMA: u32 = 1;
/// The contrast file's source, which the reference file's intake refuses.
pub const SOURCE: &str = "SoccerTrack";
/// What the contrast is and is not.
pub const NOTE: &str = "a contrast of team shape and pass choice only; SoccerTrack data sets no \
                        target";
/// Said when no SoccerTrack match has a ball-action file.
pub const NO_PASS_CHOICE: &str = "pass choice: no ball-action files";

/// A half's team shape, the mean over its frames and both sides, in metres.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreeShape {
    pub length_m: f64,
    pub width_m: f64,
    pub spread_m: f64,
    pub frames: u32,
}

impl FreeShape {
    /// The shape of a report's figures: each side's sums pooled.
    pub fn of(v: &TrackedValues) -> Self {
        let at = |n: &str| place(n).expect("a free figure");
        let sum = |n: &str| v[0][at(n)] + v[1][at(n)];
        let n = sum("free.n");
        let mean = |s: &str| if n > 0.0 { sum(s) / n } else { 0.0 };
        Self {
            length_m: mean("free.length_sum"),
            width_m: mean("free.width_sum"),
            spread_m: mean("free.spread_sum"),
            frames: v[0][at("team.frames")] as u32,
        }
    }
}

/// Passes by choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassChoice {
    pub ground: u32,
    pub high: u32,
    pub cross: u32,
}

impl PassChoice {
    fn add(&mut self, o: &PassChoice) {
        self.ground += o.ground;
        self.high += o.high;
        self.cross += o.cross;
    }

    /// The shares of ground passes, high passes and crosses; `None` without a pass.
    pub fn shares(&self) -> Option<[f64; 3]> {
        let n = f64::from(self.ground + self.high + self.cross);
        (n > 0.0).then(|| {
            [
                f64::from(self.ground) / n,
                f64::from(self.high) / n,
                f64::from(self.cross) / n,
            ]
        })
    }
}

/// One half of a match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContrastHalf {
    pub shape: FreeShape,
    pub pass_choice: Option<PassChoice>,
}

/// One match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContrastMatch {
    pub match_id: String,
    pub halves: Vec<ContrastHalf>,
}

/// The mean of one half over the matches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HalfMean {
    pub length_m: f64,
    pub width_m: f64,
    pub spread_m: f64,
    /// Ground pass, high pass and cross shares over the matches' passes pooled.
    pub pass_shares: Option<[f64; 3]>,
    pub matches: u32,
}

/// One level of play.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Level {
    pub dataset: String,
    pub matches: Vec<ContrastMatch>,
    /// Per half.
    pub mean: Vec<HalfMean>,
}

impl Level {
    fn new(dataset: &str, matches: Vec<ContrastMatch>) -> Self {
        let halves = matches.iter().map(|m| m.halves.len()).max().unwrap_or(0);
        let mean = (0..halves)
            .map(|h| {
                let of: Vec<&ContrastHalf> =
                    matches.iter().filter_map(|m| m.halves.get(h)).collect();
                let n = of.len() as f64;
                let avg =
                    |f: fn(&FreeShape) -> f64| of.iter().map(|x| f(&x.shape)).sum::<f64>() / n;
                let mut passes = PassChoice::default();
                let mut any = false;
                for x in &of {
                    if let Some(p) = &x.pass_choice {
                        passes.add(p);
                        any = true;
                    }
                }
                HalfMean {
                    length_m: avg(|s| s.length_m),
                    width_m: avg(|s| s.width_m),
                    spread_m: avg(|s| s.spread_m),
                    pass_shares: if any { passes.shares() } else { None },
                    matches: of.len() as u32,
                }
            })
            .collect();
        Self {
            dataset: dataset.into(),
            matches,
            mean,
        }
    }
}

/// The contrast file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contrast {
    pub schema: u32,
    pub source: String,
    pub note: String,
    pub amateur: Level,
    pub elite: Level,
    /// Why a level has no pass choice, when it has none.
    pub absent: Vec<String>,
}

/// The shape of one SoccerTrack half: each image a frame, the side named left in the half
/// team 0, up to eleven players a side in track order with the keeper apart.
pub fn half_shape(path: &Path) -> anyhow::Result<FreeShape> {
    let file =
        std::fs::File::open(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut report = TrackedShape::new(Mode::BallFree, 105.0);
    let mut players = [[0.0f32; 2]; 22];
    read_shape(file, &mut report, &mut players)
        .with_context(|| format!("cannot read {}", path.display()))?;
    Ok(FreeShape::of(&report.finish()))
}

fn read_shape(
    input: impl std::io::Read,
    report: &mut TrackedShape,
    players: &mut [[f32; 2]; 22],
) -> anyhow::Result<()> {
    soccertrack::read_half(input, &mut |image, objects| {
        let mut on = [None; 22];
        for side in 0..2 {
            let mut ours: Vec<&soccertrack::Object> =
                objects.iter().filter(|o| o.side == side).collect();
            ours.sort_by_key(|o| (!o.keeper, o.track));
            for (k, o) in ours.iter().take(11).enumerate() {
                let i = side * 11 + k;
                players[i] = [o.x, o.y];
                on[i] = Some(Holder {
                    person: o.track,
                    group: if o.keeper { Group::Keeper } else { Group::Cm },
                });
            }
        }
        let tick = u32::try_from(2 * image).context("too many images in a half")?;
        report
            .frame(&Frame {
                tick,
                half: 0,
                ball: None,
                players: *players,
                on,
                live: None,
                possession: None,
                restarts: Vec::new(),
                attack_x: [1.0, -1.0],
            })
            .map_err(anyhow::Error::msg)
    })
}

/// The amateur level: every SoccerTrack match of `dir`.
pub fn amateur(dir: &Path) -> anyhow::Result<(Level, bool)> {
    let mut out = Vec::new();
    let mut any_actions = false;
    for (id, halves) in soccertrack::matches(dir)? {
        let actions = soccertrack::action_file(dir, &id);
        let choice = if actions.exists() {
            any_actions = true;
            let text = std::fs::read_to_string(&actions)
                .with_context(|| format!("cannot read {}", actions.display()))?;
            Some(soccertrack::pass_choice(&text).with_context(|| actions.display().to_string())?)
        } else {
            None
        };
        let mut hs = Vec::new();
        for (h, path) in halves.iter().enumerate() {
            hs.push(ContrastHalf {
                shape: half_shape(path)?,
                pass_choice: choice.as_ref().map(|c| c[h]),
            });
        }
        out.push(ContrastMatch {
            match_id: id,
            halves: hs,
        });
    }
    anyhow::ensure!(!out.is_empty(), "no SoccerTrack match in {}", dir.display());
    Ok((
        Level::new("SoccerTrack v2 (amateur, CC BY 4.0)", out),
        any_actions,
    ))
}

/// The elite level: the matches of a tracking file.
pub fn elite(file: &TrackingFile) -> Level {
    let matches = file
        .matches
        .iter()
        .map(|m| ContrastMatch {
            match_id: m.match_id.clone(),
            halves: m
                .halves
                .iter()
                .map(|h| ContrastHalf {
                    shape: h.shape,
                    pass_choice: Some(h.passes),
                })
                .collect(),
        })
        .collect();
    Level::new(&format!("{} tracked matches (elite)", file.source), matches)
}

/// The contrast of the SoccerTrack matches of `soccertrack` with the tracking file.
pub fn contrast(soccertrack: &Path, tracking: &TrackingFile) -> anyhow::Result<Contrast> {
    let (amateur, actions) = amateur(soccertrack)?;
    Ok(Contrast {
        schema: SCHEMA,
        source: SOURCE.into(),
        note: NOTE.into(),
        amateur,
        elite: elite(tracking),
        absent: if actions {
            Vec::new()
        } else {
            vec![format!("amateur {NO_PASS_CHOICE}")]
        },
    })
}

impl Contrast {
    /// Writes the contrast to `path`; never under the checkout's `gate/` folder, where the
    /// files that set targets live.
    pub fn write(&self, path: &Path, gate: &Path) -> anyhow::Result<()> {
        let abs = |p: &Path| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
        anyhow::ensure!(
            !abs(path).starts_with(abs(gate)),
            "the contrast never writes under {}: SoccerTrack data sets no target",
            gate.display()
        );
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("cannot create {}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(self)? + "\n";
        std::fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))
    }

    /// The table the command prints.
    pub fn table(&self) -> String {
        let mut out = String::from(
            "level    half  matches  length m  width m  spread m  ground  high  cross\n",
        );
        for (name, level) in [("amateur", &self.amateur), ("elite", &self.elite)] {
            for (h, m) in level.mean.iter().enumerate() {
                let shares = m
                    .pass_shares
                    .map_or("     -     -      -".to_string(), |s| {
                        format!("{:>6.3} {:>5.3} {:>6.3}", s[0], s[1], s[2])
                    });
                out.push_str(&format!(
                    "{name:<8} {:>4}  {:>7}  {:>8.1}  {:>7.1}  {:>8.1}  {shares}\n",
                    h + 1,
                    m.matches,
                    m.length_m,
                    m.width_m,
                    m.spread_m
                ));
            }
        }
        for a in &self.absent {
            out.push_str(a);
            out.push('\n');
        }
        out
    }
}

/// The contrast's default place: `.scratch/tracking-contrast/contrast.json` of the main
/// checkout, beside the raw data.
pub fn default_out(data: &Path) -> PathBuf {
    data.parent()
        .unwrap_or(data)
        .join("tracking-contrast/contrast.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracking")
    }

    /// The fixture half's shape: left has outfield players at x -10 and 10 (y 0 and 20),
    /// right at x -5 and 15 (y -10 and 10): length 20, width 20, spread about 14.1.
    #[test]
    fn a_half_s_shape_pools_both_sides_over_every_image() {
        let s = half_shape(&fixtures().join("soccertrack/900001_1st.json")).unwrap();
        assert_eq!(s.frames, 3);
        assert!((s.length_m - 20.0).abs() < 1e-6, "{s:?}");
        assert!((s.width_m - 20.0).abs() < 1e-6, "{s:?}");
        assert!((s.spread_m - 200f64.sqrt()).abs() < 1e-6, "{s:?}");
    }

    #[test]
    fn the_shares_are_of_the_three_choices() {
        let p = PassChoice {
            ground: 6,
            high: 3,
            cross: 1,
        };
        assert_eq!(p.shares(), Some([0.6, 0.3, 0.1]));
        assert_eq!(PassChoice::default().shares(), None);
    }

    /// The contrast of the fixtures compares shape and pass choice per half and nothing
    /// else, and the reference file's intake refuses it.
    #[test]
    fn the_contrast_holds_shape_and_pass_choice_and_sets_no_target() {
        let data = fixtures().join("dfl");
        let m = crate::tracking::dfl::read(
            &crate::tracking::dfl::find(&data, "DFL-MAT-FIX001").unwrap(),
        )
        .unwrap();
        let conv = crate::tracking::convert::to_ticks(&m).unwrap();
        let figures = crate::tracking::measure::figures_of(
            &conv.records,
            &conv.side,
            &crate::tracking::measure::dfl_pass_choice(&m),
        )
        .unwrap();
        let tracking = TrackingFile {
            schema: crate::tracking::measure::TRACKING_SCHEMA,
            source: "DFL".into(),
            credit: String::new(),
            frame_rate: 25,
            matches: vec![figures],
        };
        let c = contrast(&fixtures().join("soccertrack"), &tracking).unwrap();
        assert_eq!(c.amateur.matches.len(), 1);
        assert_eq!(c.amateur.mean.len(), 2);
        assert_eq!(c.elite.mean.len(), 2);
        assert!(c.absent.is_empty());
        let s = c.amateur.mean[0].pass_shares.unwrap();
        assert_eq!(s, [0.5, 0.25, 0.25]);
        // The sides swap at half time; pooled over both, the shape is the same.
        assert_eq!(
            c.amateur.matches[0].halves[0].shape,
            c.amateur.matches[0].halves[1].shape
        );
        let json = serde_json::to_value(&c).unwrap();
        fn keys(v: &serde_json::Value, out: &mut Vec<String>) {
            match v {
                serde_json::Value::Object(m) => {
                    for (k, x) in m {
                        out.push(k.clone());
                        keys(x, out);
                    }
                }
                serde_json::Value::Array(a) => a.iter().for_each(|x| keys(x, out)),
                _ => {}
            }
        }
        let mut all = Vec::new();
        keys(&json, &mut all);
        for k in &all {
            for bad in ["speed", "ball", "target"] {
                assert!(!k.contains(bad), "the contrast has a key {k}");
            }
        }
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("contrast.json");
        c.write(&path, &dir.path().join("gate")).unwrap();
        let err = crate::real_targets::tracked::read_tracking(&path)
            .unwrap_err()
            .to_string();
        assert!(err.contains("SoccerTrack data sets no target"), "{err}");
        // Without ball-action files the amateur level says pass choice is absent.
        let bare = dir.path().join("bare");
        std::fs::create_dir_all(&bare).unwrap();
        for h in ["900001_1st.json", "900001_2nd.json"] {
            std::fs::copy(fixtures().join("soccertrack").join(h), bare.join(h)).unwrap();
        }
        let c = contrast(&bare, &tracking).unwrap();
        assert_eq!(c.absent, [format!("amateur {NO_PASS_CHOICE}")]);
        assert_eq!(c.amateur.mean[0].pass_shares, None);
        assert!(c.table().contains(NO_PASS_CHOICE));
    }

    /// The contrast never writes under `gate/`.
    #[test]
    fn the_contrast_never_writes_under_gate() {
        let dir = tempfile::TempDir::new().unwrap();
        let gate = dir.path().join("gate");
        let c = Contrast {
            schema: SCHEMA,
            source: SOURCE.into(),
            note: NOTE.into(),
            amateur: Level::new("a", Vec::new()),
            elite: Level::new("e", Vec::new()),
            absent: Vec::new(),
        };
        let err = c
            .write(&gate.join("tracking/contrast.json"), &gate)
            .unwrap_err()
            .to_string();
        assert!(err.contains("never writes under"), "{err}");
        assert!(!gate.exists());
        c.write(&dir.path().join("out/contrast.json"), &gate)
            .unwrap();
    }
}
