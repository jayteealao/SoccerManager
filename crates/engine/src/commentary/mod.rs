//! The commentator (named mechanism): one per match, outside the engine loop. It reads each
//! engine event after the tick that produced it and writes one English line, chosen from the
//! commentary file by the match situation.
//!
//! Selection: every template set of the event's kind whose conditions all hold and whose
//! placeholders all have values is a candidate. Candidates are tried from the most specific
//! (the most conditions) to the least. Inside a set the search starts at a rotation index
//! derived from the seed, the kind, and how many events of that kind came before, and takes
//! the first line not used for that kind in the last ten minutes of play. When every
//! candidate line was used in that window, the least recently used line is taken. The
//! commentator draws nothing from the engine's random-number generator, so the same seed
//! gives the same match and the same lines.

pub mod context;
pub mod render;
pub mod templates;

use context::{History, Length, REPEAT_WINDOW_TICKS, Situation};
use render::Values;
pub use templates::{COMMENTARY_VERSION, Commentary, CommentaryFile, commented};

use crate::sim::{EngineEvent, EngineEventKind, EventDetail, Simulation};

/// The names a match's lines use: each club's name, each club's squad in file order, and the
/// player in each roster slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchNames {
    pub clubs: [String; 2],
    pub squads: [Vec<String>; 2],
    pub roster: Vec<String>,
}

impl MatchNames {
    /// The names as the match stands now.
    pub fn of(sim: &Simulation) -> Self {
        let teams = sim.teams();
        Self {
            clubs: [teams[0].name.clone(), teams[1].name.clone()],
            squads: [teams[0].player_names.clone(), teams[1].player_names.clone()],
            roster: sim.player_names(),
        }
    }
}

/// Writes the line for each event of one match.
pub struct Commentator<'a> {
    commentary: &'a Commentary,
    names: MatchNames,
    length: Length,
    seed: u64,
    history: History,
    /// The tick each line of each set was last used on.
    last_used: Vec<Vec<Option<u32>>>,
    fallbacks: u32,
}

impl<'a> Commentator<'a> {
    /// A commentator for `sim`, read before its first event.
    pub fn for_match(commentary: &'a Commentary, sim: &Simulation) -> Self {
        let names = MatchNames::of(sim);
        let length = Length {
            minutes: sim.minutes(),
            halves: u32::from(sim.config().rules.halves),
        };
        Self::new(commentary, names, length, sim.seed())
    }

    /// A commentator from its parts, for scripted events.
    pub fn new(commentary: &'a Commentary, names: MatchNames, length: Length, seed: u64) -> Self {
        let players = names.roster.len();
        Self {
            last_used: commentary
                .sets
                .iter()
                .map(|s| vec![None; s.lines.len()])
                .collect(),
            commentary,
            names,
            length,
            seed,
            history: History::new(players),
            fallbacks: 0,
        }
    }

    /// Events no template set could fill, each written with the built-in line instead.
    pub fn fallbacks(&self) -> u32 {
        self.fallbacks
    }

    /// The line for `event`, or `None` for a verdict on a queued change and for a shoot-out
    /// kick, which get no line (the penalty templates announce an award, which a shoot-out
    /// kick is not). Call it once per event, in event order.
    pub fn line(&mut self, event: &EngineEvent) -> Option<String> {
        if !commented(event.kind) || event.shootout_round.is_some() {
            return None;
        }
        let values = self.values(event);
        let situation = Situation {
            event,
            length: self.length,
            history: &self.history,
        };
        let mut candidates: Vec<usize> = self
            .commentary
            .sets
            .iter()
            .enumerate()
            .filter(|(_, set)| {
                set.kind == event.kind
                    && situation.holds(&set.when)
                    && set.lines.iter().all(|l| render::fill(l, &values).is_some())
            })
            .map(|(i, _)| i)
            .collect();
        // A stable sort keeps file order among sets of equal specificity.
        candidates.sort_by_key(|&i| std::cmp::Reverse(self.commentary.sets[i].specificity));
        let chosen = self.choose(&candidates, event);
        let line = chosen.and_then(|(s, l)| {
            self.last_used[s][l] = Some(event.tick);
            render::fill(&self.commentary.sets[s].lines[l], &values)
        });
        let line = line.unwrap_or_else(|| self.fallback(event, &values));
        self.history.record(event);
        self.rename(event);
        Some(line)
    }

    /// Earlier events of `kind` in the whole match.
    fn kind_count(&self, kind: EngineEventKind) -> u64 {
        self.history
            .events
            .iter()
            .filter(|(_, k)| *k == kind)
            .count() as u64
    }

    /// The set and line to use: the first line not used in the repeat window, searching the
    /// candidates in order, or else the least recently used candidate line.
    fn choose(&self, candidates: &[usize], event: &EngineEvent) -> Option<(usize, usize)> {
        let start = rotation(self.seed, event.kind).wrapping_add(self.kind_count(event.kind));
        let mut oldest: Option<(u32, usize, usize)> = None;
        for &s in candidates {
            let n = self.commentary.sets[s].lines.len();
            for k in 0..n {
                // The remainder is below `n`, so it fits `usize`.
                let l = ((start % n as u64) as usize + k) % n;
                match self.last_used[s][l] {
                    Some(t) if t + REPEAT_WINDOW_TICKS > event.tick => {
                        if oldest.is_none_or(|(o, _, _)| t < o) {
                            oldest = Some((t, s, l));
                        }
                    }
                    _ => return Some((s, l)),
                }
            }
        }
        oldest.map(|(_, s, l)| (s, l))
    }

    /// The built-in line for an event no set could fill. It names the club, and it emits
    /// `commentary.fallback` so a gap in the file shows up in the logs.
    fn fallback(&mut self, event: &EngineEvent, values: &Values) -> String {
        self.fallbacks += 1;
        tracing::warn!(
            signal = "commentary.fallback",
            tick = event.tick,
            kind = event.kind.code(),
            reason = "no template set could be filled"
        );
        let who = values.get("team").map_or_else(
            || format!("{} v {}", self.names.clubs[0], self.names.clubs[1]),
            str::to_string,
        );
        format!("{}: {who}.", event.kind.code())
    }

    /// The placeholder values for `event`.
    fn values(&self, event: &EngineEvent) -> Values {
        let mut v = Values::default();
        let [home, away] = &self.names.clubs;
        v.set("home", home.as_str());
        v.set("away", away.as_str());
        v.set("home_score", event.scores[0].to_string());
        v.set("away_score", event.scores[1].to_string());
        v.set("score", format!("{}-{}", event.scores[0], event.scores[1]));
        match event.minute_added {
            Some(added) => {
                v.set("minute", event.minute.to_string());
                v.set("added_minutes", added.to_string());
            }
            // The clock counts from 0; the first minute of play reads 1.
            None => v.set("minute", (event.minute + 1).to_string()),
        }
        if let Some(team) = event.team {
            v.set("team", self.names.clubs[team].as_str());
            v.set("opponent", self.names.clubs[1 - team].as_str());
        }
        let roster = |i: Option<usize>| i.and_then(|i| self.names.roster.get(i));
        match (event.detail, event.team) {
            (Some(EventDetail::Substitution { off, on }), Some(team)) => {
                if let Some(name) = self.names.squads[team].get(off) {
                    v.set("player", name.as_str());
                }
                if let Some(name) = self.names.squads[team].get(on) {
                    v.set("other_player", name.as_str());
                }
            }
            _ => {
                if let Some(name) = roster(event.player) {
                    v.set("player", name.as_str());
                }
                if let Some(name) = roster(event.secondary) {
                    v.set("other_player", name.as_str());
                }
            }
        }
        v
    }

    /// A substitution puts the new player's name in the roster slot.
    fn rename(&mut self, event: &EngineEvent) {
        if let (Some(EventDetail::Substitution { on, .. }), Some(team), Some(slot)) =
            (event.detail, event.team, event.player)
            && let (Some(name), Some(entry)) = (
                self.names.squads[team].get(on).cloned(),
                self.names.roster.get_mut(slot),
            )
        {
            *entry = name;
        }
    }
}

/// The rotation start for `kind` in a match with `seed`: a SplitMix64 mix, so two seeds open
/// on different lines.
fn rotation(seed: u64, kind: EngineEventKind) -> u64 {
    let index = EngineEventKind::ALL
        .iter()
        .position(|k| *k == kind)
        .unwrap_or(0) as u64;
    let mut z = seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::templates::{Set, When};
    use super::*;
    use crate::sim::EngineEvent;

    fn names() -> MatchNames {
        let squad = |c: &str| (0..18).map(|i| format!("{c}{i}")).collect::<Vec<_>>();
        let mut roster = squad("A")[..11].to_vec();
        roster.extend_from_slice(&squad("B")[..11]);
        MatchNames {
            clubs: ["Alpha".into(), "Beta".into()],
            squads: [squad("A"), squad("B")],
            roster,
        }
    }

    fn set(kind: EngineEventKind, when: When, lines: &[&str]) -> Set {
        Set {
            kind,
            specificity: when.specificity(),
            when,
            lines: lines.iter().map(|l| l.to_string()).collect(),
        }
    }

    fn corner(minute: u32) -> EngineEvent {
        EngineEvent {
            tick: minute * 3_000 + 1,
            kind: EngineEventKind::Corner,
            team: Some(0),
            scores: [0, 0],
            minute,
            minute_added: None,
            player: Some(7),
            secondary: None,
            card: None,
            advantage: None,
            added_time_s: None,
            spot: None,
            detail: None,
            period: None,
            shootout_round: None,
            shootout_scored: None,
            shootout_scores: None,
            decided_by: None,
        }
    }

    const LENGTH: Length = Length {
        minutes: 90,
        halves: 2,
    };

    #[test]
    fn the_most_specific_set_wins() {
        let commentary = Commentary {
            language: "en".into(),
            sets: vec![
                set(EngineEventKind::Corner, When::default(), &["plain {team}"]),
                set(
                    EngineEventKind::Corner,
                    When {
                        minute: Some(templates::MinuteBand::Early),
                        ..When::default()
                    },
                    &["early {player}"],
                ),
            ],
        };
        let mut c = Commentator::new(&commentary, names(), LENGTH, 42);
        assert_eq!(c.line(&corner(3)).unwrap(), "early A7");
        assert_eq!(c.line(&corner(40)).unwrap(), "plain Alpha");
        assert_eq!(c.fallbacks(), 0);
    }

    #[test]
    fn no_line_repeats_inside_the_window_while_an_unused_one_exists() {
        let commentary = Commentary {
            language: "en".into(),
            sets: vec![set(
                EngineEventKind::Corner,
                When::default(),
                &["one", "two", "three"],
            )],
        };
        let mut c = Commentator::new(&commentary, names(), LENGTH, 7);
        let lines: Vec<String> = (20..25).map(|m| c.line(&corner(m)).unwrap()).collect();
        let mut first = lines[..3].to_vec();
        first.sort();
        first.dedup();
        assert_eq!(first.len(), 3, "{lines:?}");
    }

    #[test]
    fn the_same_seed_gives_the_same_lines() {
        let commentary = Commentary {
            language: "en".into(),
            sets: vec![set(
                EngineEventKind::Corner,
                When::default(),
                &["one", "two", "three", "four", "five"],
            )],
        };
        let run = |seed| {
            let mut c = Commentator::new(&commentary, names(), LENGTH, seed);
            (1..30)
                .map(|m| c.line(&corner(m * 3)).unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(43));
    }

    #[test]
    fn an_unfillable_event_gets_the_built_in_line() {
        let commentary = Commentary {
            language: "en".into(),
            sets: vec![set(
                EngineEventKind::Corner,
                When::default(),
                &["{other_player}"],
            )],
        };
        let mut c = Commentator::new(&commentary, names(), LENGTH, 1);
        assert_eq!(c.line(&corner(5)).unwrap(), "corner: Alpha.");
        assert_eq!(c.fallbacks(), 1);
    }

    #[test]
    fn a_verdict_gets_no_line_and_a_substitute_takes_the_slot_name() {
        let commentary = Commentary {
            language: "en".into(),
            sets: vec![
                set(
                    EngineEventKind::Substitution,
                    When::default(),
                    &["{other_player} on for {player}"],
                ),
                set(EngineEventKind::Corner, When::default(), &["{player}"]),
            ],
        };
        let mut c = Commentator::new(&commentary, names(), LENGTH, 1);
        let mut e = corner(60);
        e.kind = EngineEventKind::ChangeApplied;
        assert_eq!(c.line(&e), None);
        e.kind = EngineEventKind::Substitution;
        e.detail = Some(EventDetail::Substitution { off: 7, on: 15 });
        assert_eq!(c.line(&e).unwrap(), "A15 on for A7");
        assert_eq!(c.line(&corner(61)).unwrap(), "A15");
    }
}
