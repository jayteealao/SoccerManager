//! The match situation an event happened in: pure functions over the event, what the
//! commentator remembers of earlier events, and the match length.

use super::templates::{Form, MinuteBand, Repeat, ScoreState, When};
use crate::rules::fouls::Card;
use crate::sim::{EngineEvent, EngineEventKind, EventDetail};
use crate::team::PLAYERS_PER_TEAM;

/// Ticks in one minute of play.
const TICKS_PER_MINUTE: u32 = crate::TICKS_PER_SECOND * 60;
/// The repeat window: ten minutes of play.
pub const REPEAT_WINDOW_TICKS: u32 = 10 * TICKS_PER_MINUTE;
/// A lead of this many goals is a rout.
pub const ROUT_MARGIN: u32 = 3;

/// What the commentator remembers of earlier events.
#[derive(Debug, Clone, Default)]
pub struct History {
    /// The tick and kind of every earlier commented event.
    pub events: Vec<(u32, EngineEventKind)>,
    /// The tick and scoring club of every goal so far.
    pub goals: Vec<(u32, usize)>,
    /// `true` for a roster slot whose player holds a yellow card.
    pub booked: Vec<bool>,
}

impl History {
    pub fn new(players: usize) -> Self {
        Self {
            booked: vec![false; players],
            ..Self::default()
        }
    }

    /// Records `event` after its line was chosen.
    pub fn record(&mut self, event: &EngineEvent) {
        self.events.push((event.tick, event.kind));
        match event.kind {
            EngineEventKind::Goal => {
                if let Some(team) = event.team {
                    self.goals.push((event.tick, team));
                }
            }
            EngineEventKind::Card => {
                if let (Some(p), Some(Card::Yellow)) = (event.player, event.card)
                    && let Some(b) = self.booked.get_mut(p)
                {
                    *b = true;
                }
            }
            EngineEventKind::Substitution => {
                // The substitute in the slot holds no card.
                if let Some(b) = event.player.and_then(|p| self.booked.get_mut(p)) {
                    *b = false;
                }
            }
            _ => {}
        }
    }
}

/// The match length the bands are fractions of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Length {
    /// Regulation minutes.
    pub minutes: u32,
    pub halves: u32,
}

impl Length {
    fn half_minutes(self) -> u32 {
        (self.minutes / self.halves.max(1)).max(1)
    }

    /// The half an event falls in, from 0. In added time the clock shows the end of the half.
    pub fn half_of(self, minute: u32, added: Option<u32>) -> u32 {
        let half = self.half_minutes();
        let h = if added.is_some() {
            (minute / half).saturating_sub(1)
        } else {
            minute / half
        };
        h.min(self.halves.saturating_sub(1))
    }

    /// Ticks in one sixth of the match, the form window.
    fn sixth_ticks(self) -> u32 {
        self.minutes * TICKS_PER_MINUTE / 6
    }
}

/// The situation one event happened in.
#[derive(Debug, Clone, Copy)]
pub struct Situation<'a> {
    pub event: &'a EngineEvent,
    pub length: Length,
    pub history: &'a History,
}

impl Situation<'_> {
    /// The club whose side the score is read from: the event's club, or the home club at
    /// half-time and full time.
    fn side(&self) -> usize {
        self.event.team.unwrap_or(0)
    }

    pub fn minute_holds(&self, band: MinuteBand) -> bool {
        let e = self.event;
        let m = self.length.minutes;
        match band {
            MinuteBand::Early => e.minute_added.is_none() && e.minute * 6 < m,
            MinuteBand::Late => {
                let last = self.length.half_of(e.minute, e.minute_added) + 1 >= self.length.halves;
                if e.minute_added.is_some() {
                    last
                } else {
                    e.minute * 9 >= 8 * m
                }
            }
            MinuteBand::AddedTime => e.minute_added.is_some(),
            MinuteBand::FirstHalf => {
                self.length.halves >= 2 && self.length.half_of(e.minute, e.minute_added) == 0
            }
            MinuteBand::SecondHalf => {
                self.length.halves >= 2 && self.length.half_of(e.minute, e.minute_added) == 1
            }
        }
    }

    pub fn score_holds(&self, state: ScoreState) -> bool {
        let side = self.side();
        let mine = self.event.scores[side];
        let theirs = self.event.scores[1 - side];
        let goal = self.event.kind == EngineEventKind::Goal && self.event.team.is_some();
        let before = mine.saturating_sub(1);
        match state {
            ScoreState::Level => mine == theirs,
            ScoreState::Leading => mine > theirs,
            ScoreState::Trailing => mine < theirs,
            ScoreState::Rout => mine >= theirs + ROUT_MARGIN,
            ScoreState::Opener => goal && mine == 1 && theirs == 0,
            ScoreState::Equaliser => goal && mine == theirs,
            ScoreState::GoAhead => goal && before == theirs,
            ScoreState::ExtendsLead => goal && before > theirs,
            ScoreState::Consolation => goal && mine < theirs,
        }
    }

    pub fn form_holds(&self, form: Form) -> bool {
        let side = self.side();
        let window = self.length.sixth_ticks();
        // Goals in the window, this goal included.
        let recent = |team: usize| {
            let earlier = self
                .history
                .goals
                .iter()
                .filter(|&&(tick, t)| t == team && tick + window > self.event.tick)
                .count();
            let this = self.event.kind == EngineEventKind::Goal && self.event.team == Some(team);
            earlier + usize::from(this)
        };
        match form {
            Form::Hot => self.event.team.is_some() && recent(side) >= 2,
            Form::Cold => self.event.team.is_some() && recent(1 - side) >= 2,
            Form::Booked => self
                .event
                .player
                .and_then(|p| self.history.booked.get(p))
                .copied()
                .unwrap_or(false),
        }
    }

    /// Earlier events of the same kind in the last ten minutes of play.
    pub fn repeats(&self) -> usize {
        self.history
            .events
            .iter()
            .filter(|&&(tick, kind)| {
                kind == self.event.kind && tick + REPEAT_WINDOW_TICKS > self.event.tick
            })
            .count()
    }

    pub fn repeat_holds(&self, repeat: Repeat) -> bool {
        let n = self.repeats();
        match repeat {
            Repeat::First => n == 0,
            Repeat::Again => n >= 1,
            Repeat::Streak => n >= 2,
        }
    }

    /// `true` when the goal's last kicker plays for the other club.
    pub fn own_goal(&self) -> bool {
        match (self.event.kind, self.event.team, self.event.player) {
            (EngineEventKind::Goal, Some(team), Some(p)) => p / PLAYERS_PER_TEAM != team,
            _ => false,
        }
    }

    /// `true` when every condition of `when` holds for this event.
    pub fn holds(&self, when: &When) -> bool {
        let e = self.event;
        when.minute.is_none_or(|b| self.minute_holds(b))
            && when.score.is_none_or(|s| self.score_holds(s))
            && when.form.is_none_or(|f| self.form_holds(f))
            && when.repeat.is_none_or(|r| self.repeat_holds(r))
            && when
                .card
                .is_none_or(|c| e.card.is_some_and(|card| c.matches(card)))
            && when.advantage.is_none_or(|a| e.advantage == Some(a))
            && when
                .own_goal
                .is_none_or(|o| e.kind == EngineEventKind::Goal && self.own_goal() == o)
            && when.decision.is_none_or(|d| match e.detail {
                Some(EventDetail::Ai { code }) => d.matches(code),
                _ => false,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NINETY: Length = Length {
        minutes: 90,
        halves: 2,
    };

    fn event(
        kind: EngineEventKind,
        team: Option<usize>,
        minute: u32,
        scores: [u32; 2],
    ) -> EngineEvent {
        EngineEvent {
            tick: minute * TICKS_PER_MINUTE + 1,
            kind,
            team,
            scores,
            minute,
            minute_added: None,
            player: Some(3),
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

    fn at<'a>(e: &'a EngineEvent, h: &'a History) -> Situation<'a> {
        Situation {
            event: e,
            length: NINETY,
            history: h,
        }
    }

    #[test]
    fn minute_bands() {
        let h = History::new(22);
        let e = event(EngineEventKind::ThrowIn, Some(0), 14, [0, 0]);
        assert!(at(&e, &h).minute_holds(MinuteBand::Early));
        assert!(at(&e, &h).minute_holds(MinuteBand::FirstHalf));
        let e = event(EngineEventKind::ThrowIn, Some(0), 15, [0, 0]);
        assert!(!at(&e, &h).minute_holds(MinuteBand::Early));
        let e = event(EngineEventKind::ThrowIn, Some(0), 88, [0, 0]);
        assert!(
            at(&e, &h).minute_holds(MinuteBand::Late),
            "minute 88 of 90 is late"
        );
        assert!(at(&e, &h).minute_holds(MinuteBand::SecondHalf));
        assert!(!at(&e, &h).minute_holds(MinuteBand::AddedTime));
        let e = event(EngineEventKind::ThrowIn, Some(0), 79, [0, 0]);
        assert!(!at(&e, &h).minute_holds(MinuteBand::Late));
        let mut e = event(EngineEventKind::ThrowIn, Some(0), 45, [0, 0]);
        e.minute_added = Some(2);
        assert!(at(&e, &h).minute_holds(MinuteBand::AddedTime));
        assert!(at(&e, &h).minute_holds(MinuteBand::FirstHalf));
        assert!(
            !at(&e, &h).minute_holds(MinuteBand::Late),
            "first-half added time is not late"
        );
        e.minute = 90;
        assert!(at(&e, &h).minute_holds(MinuteBand::Late));
        assert!(at(&e, &h).minute_holds(MinuteBand::SecondHalf));
    }

    #[test]
    fn score_states() {
        let h = History::new(22);
        let e = event(EngineEventKind::Goal, Some(1), 88, [1, 1]);
        let s = at(&e, &h);
        assert!(s.score_holds(ScoreState::Equaliser));
        assert!(s.score_holds(ScoreState::Level));
        assert!(!s.score_holds(ScoreState::GoAhead));
        let e = event(EngineEventKind::Goal, Some(0), 20, [4, 0]);
        let s = at(&e, &h);
        assert!(s.score_holds(ScoreState::Rout));
        assert!(s.score_holds(ScoreState::ExtendsLead));
        assert!(s.score_holds(ScoreState::Leading));
        assert!(!s.score_holds(ScoreState::Opener));
        let e = event(EngineEventKind::Goal, Some(0), 5, [1, 0]);
        assert!(at(&e, &h).score_holds(ScoreState::Opener));
        assert!(at(&e, &h).score_holds(ScoreState::GoAhead));
        let e = event(EngineEventKind::Goal, Some(1), 70, [3, 1]);
        assert!(at(&e, &h).score_holds(ScoreState::Consolation));
        assert!(at(&e, &h).score_holds(ScoreState::Trailing));
        let e = event(EngineEventKind::Corner, Some(1), 70, [3, 1]);
        assert!(
            !at(&e, &h).score_holds(ScoreState::Consolation),
            "goal states hold on goals only"
        );
        let e = event(EngineEventKind::FullTime, None, 90, [0, 3]);
        assert!(
            at(&e, &h).score_holds(ScoreState::Trailing),
            "home side at full time"
        );
    }

    #[test]
    fn form_and_bookings() {
        let mut h = History::new(22);
        h.record(&event(EngineEventKind::Goal, Some(0), 70, [1, 0]));
        let e = event(EngineEventKind::Goal, Some(0), 80, [2, 0]);
        assert!(at(&e, &h).form_holds(Form::Hot));
        let e = event(EngineEventKind::Corner, Some(1), 81, [2, 0]);
        assert!(!at(&e, &h).form_holds(Form::Cold), "one goal in the window");
        h.record(&event(EngineEventKind::Goal, Some(0), 80, [2, 0]));
        assert!(at(&e, &h).form_holds(Form::Cold));
        let late = event(EngineEventKind::Corner, Some(1), 96, [2, 0]);
        assert!(
            !at(&late, &h).form_holds(Form::Cold),
            "the window is one sixth"
        );
        let foul = event(EngineEventKind::Foul, Some(0), 30, [0, 0]);
        assert!(!at(&foul, &h).form_holds(Form::Booked));
        let mut card = event(EngineEventKind::Card, Some(0), 20, [0, 0]);
        card.card = Some(Card::Yellow);
        h.record(&card);
        assert!(at(&foul, &h).form_holds(Form::Booked));
        h.record(&event(EngineEventKind::Substitution, Some(0), 25, [0, 0]));
        assert!(
            !at(&foul, &h).form_holds(Form::Booked),
            "a substitute holds no card"
        );
    }

    #[test]
    fn repeats() {
        let mut h = History::new(22);
        let e = event(EngineEventKind::Corner, Some(0), 30, [0, 0]);
        assert!(at(&e, &h).repeat_holds(Repeat::First));
        h.record(&event(EngineEventKind::Corner, Some(0), 18, [0, 0]));
        assert!(
            at(&e, &h).repeat_holds(Repeat::First),
            "outside ten minutes"
        );
        h.record(&event(EngineEventKind::Corner, Some(0), 25, [0, 0]));
        assert!(at(&e, &h).repeat_holds(Repeat::Again));
        assert!(!at(&e, &h).repeat_holds(Repeat::Streak));
        h.record(&event(EngineEventKind::Corner, Some(1), 28, [0, 0]));
        assert!(at(&e, &h).repeat_holds(Repeat::Streak));
        h.record(&event(EngineEventKind::ThrowIn, Some(1), 29, [0, 0]));
        assert_eq!(at(&e, &h).repeats(), 2);
    }

    #[test]
    fn own_goals() {
        let h = History::new(22);
        let mut e = event(EngineEventKind::Goal, Some(0), 30, [1, 0]);
        e.player = Some(14);
        assert!(at(&e, &h).own_goal());
        e.player = Some(9);
        assert!(!at(&e, &h).own_goal());
    }
}
