//! The tactics model: a formation, a mentality, six team instructions, and a role and duty
//! per slot, each an index into the tactics file. A [`TeamPlan`] turns a team's tactics into
//! the numbers play reads (block depth, width, pressing, option offsets). It is computed once
//! each time the tactics change, never per tick.

pub mod change;
pub mod hooks;
pub mod planned;
pub mod verdict;

use crate::data::primitives::{OWN, SETTINGS_MAX, input_field, settings};
use crate::data::tactics::{
    DIRECTNESS, LINE_HEIGHT, MENU_MAX, Offset, PRESSING, RestDefence, StandOffset, StandOption,
    StandOptionSizes, TEMPO, TIME_WASTING, TacticsSchema, WIDTH,
};
use crate::data::team::Position;
use crate::team::PLAYERS_PER_TEAM;
use crate::tuning::Tuning;

/// A role and a duty, as indices into the tactics file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RoleDuty {
    pub role: u8,
    pub duty: u8,
}

/// One team's tactics, as indices into the tactics file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tactics {
    pub formation: u8,
    pub mentality: u8,
    /// Levels in the order of [`crate::data::tactics::INSTRUCTIONS`].
    pub instructions: [u8; 6],
    /// Per formation slot.
    pub roles: [RoleDuty; PLAYERS_PER_TEAM],
    /// The level or choice of each primitive that owns its levels or choices, in the order of
    /// [`crate::data::primitives::settings`]; the bytes past the last such primitive are 0.
    pub settings: [u8; SETTINGS_MAX],
}

/// The default level or choice of every primitive that owns its levels or choices, in the
/// order of [`crate::data::primitives::settings`].
pub fn default_settings(schema: &TacticsSchema) -> [u8; SETTINGS_MAX] {
    let mut out = [0u8; SETTINGS_MAX];
    for (o, p) in out.iter_mut().zip(settings(schema)) {
        // A checked schema names every default among at most five levels or a few choices.
        *o = p.default_index(schema).unwrap_or(0) as u8;
    }
    out
}

/// `true` when every setting of `settings` indexes a level or a choice of its primitive and
/// every byte past the last primitive is 0.
pub fn settings_in_range(settings: &[u8; SETTINGS_MAX], schema: &TacticsSchema) -> bool {
    let owners: Vec<_> = crate::data::primitives::settings(schema).collect();
    settings
        .iter()
        .enumerate()
        .all(|(k, &s)| match owners.get(k) {
            Some(p) => usize::from(s) < p.level_names(schema).len(),
            None => s == 0,
        })
}

impl Tactics {
    /// The tactics file's defaults for the AI manager: its formation, mentality, and duty,
    /// every instruction and every setting at its default, and each slot's first suitable
    /// role.
    pub fn defaults(schema: &TacticsSchema) -> Self {
        // A checked schema resolves every AI name and holds at most 64 roles and 16
        // formations, so the indices fit a byte.
        let formation = schema.formation_index(&schema.ai.formation).unwrap_or(0);
        let duty = schema.duty_index(&schema.ai.duty).unwrap_or(0) as u8;
        let mut roles = [RoleDuty::default(); PLAYERS_PER_TEAM];
        for (slot, rd) in roles.iter_mut().enumerate() {
            let position = schema.formations[formation].slots[slot].position;
            *rd = RoleDuty {
                role: schema.default_role(position) as u8,
                duty,
            };
        }
        Self {
            formation: formation as u8,
            mentality: schema.mentality_index(&schema.ai.mentality).unwrap_or(0) as u8,
            instructions: schema.instructions.defaults(),
            roles,
            settings: default_settings(schema),
        }
    }

    /// Switches to `formation`. A slot whose role does not suit its new position takes the
    /// position's first suitable role and keeps its duty.
    pub fn set_formation(&mut self, formation: u8, schema: &TacticsSchema) {
        self.formation = formation;
        let slots = &schema.formations[usize::from(formation)].slots;
        for (rd, slot) in self.roles.iter_mut().zip(slots) {
            let suits = schema.roles[usize::from(rd.role)]
                .positions
                .contains(&slot.position);
            if !suits {
                rd.role = schema.default_role(slot.position) as u8;
            }
        }
    }
}

/// One side of an instruction, a duty or a mentality a trait of play can go against. An
/// instruction is held by the order of its level against the instruction's default (below
/// or above it), and a mentality by its place in the list, never by a level's name, so a
/// tactics file may name its levels freely. A duty is held by its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Instruction {
    PressingLow,
    WidthNarrow,
    WidthWide,
    TempoSlower,
    TempoFaster,
    LineDeep,
    LineHigh,
    DirectnessShort,
    DirectnessDirect,
    DutyDefend,
    DutyAttack,
    MentalityDefensive,
    MentalityCautious,
}

impl Instruction {
    /// Every instruction, in declaration order.
    pub const ALL: [Instruction; 13] = [
        Instruction::PressingLow,
        Instruction::WidthNarrow,
        Instruction::WidthWide,
        Instruction::TempoSlower,
        Instruction::TempoFaster,
        Instruction::LineDeep,
        Instruction::LineHigh,
        Instruction::DirectnessShort,
        Instruction::DirectnessDirect,
        Instruction::DutyDefend,
        Instruction::DutyAttack,
        Instruction::MentalityDefensive,
        Instruction::MentalityCautious,
    ];

    /// The instruction as the event contract spells it: `<instruction>:<level>`,
    /// `duty:<duty>` or `mentality:<mentality>`.
    pub fn code(self) -> &'static str {
        match self {
            Instruction::PressingLow => "pressing:low",
            Instruction::WidthNarrow => "width:narrow",
            Instruction::WidthWide => "width:wide",
            Instruction::TempoSlower => "tempo:slower",
            Instruction::TempoFaster => "tempo:faster",
            Instruction::LineDeep => "line_height:deep",
            Instruction::LineHigh => "line_height:high",
            Instruction::DirectnessShort => "passing_directness:short",
            Instruction::DirectnessDirect => "passing_directness:direct",
            Instruction::DutyDefend => "duty:defend",
            Instruction::DutyAttack => "duty:attack",
            Instruction::MentalityDefensive => "mentality:defensive",
            Instruction::MentalityCautious => "mentality:cautious",
        }
    }

    /// The instruction with code `code`.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|i| i.code() == code)
    }

    /// The instruction's bit in [`TeamPlan::held`].
    #[inline]
    pub fn bit(self) -> u16 {
        1 << (self as u16)
    }

    /// `true` when `tactics` holds this instruction for the player in `slot`: an
    /// instruction's level below or above its default (pressing low, narrow or wide, tempo
    /// more direct (faster) or more patient (slower) than its default, a deep or high line,
    /// short or direct passing), the lowest mentality (defensive) or the one just below the
    /// AI's default (cautious), or the slot's duty by its name.
    pub fn held(self, tactics: &Tactics, slot: usize, schema: &TacticsSchema) -> bool {
        let defaults = schema.instructions.defaults();
        let below = |index: usize| tactics.instructions[index] < defaults[index];
        let above = |index: usize| tactics.instructions[index] > defaults[index];
        let mentality = usize::from(tactics.mentality);
        let default_mentality = schema.mentality_index(&schema.ai.mentality).unwrap_or(0);
        match self {
            Instruction::PressingLow => below(PRESSING),
            Instruction::WidthNarrow => below(WIDTH),
            Instruction::WidthWide => above(WIDTH),
            Instruction::TempoSlower => above(TEMPO),
            Instruction::TempoFaster => below(TEMPO),
            Instruction::LineDeep => below(LINE_HEIGHT),
            Instruction::LineHigh => above(LINE_HEIGHT),
            Instruction::DirectnessShort => below(DIRECTNESS),
            Instruction::DirectnessDirect => above(DIRECTNESS),
            Instruction::DutyDefend => schema
                .duty_index("defend")
                .is_some_and(|d| d == usize::from(tactics.roles[slot].duty)),
            Instruction::DutyAttack => schema
                .duty_index("attack")
                .is_some_and(|d| d == usize::from(tactics.roles[slot].duty)),
            Instruction::MentalityDefensive => mentality == 0 && default_mentality > 0,
            Instruction::MentalityCautious => {
                default_mentality > 1 && mentality + 1 == default_mentality
            }
        }
    }
}

impl Tactics {
    /// `true` when these tactics hold `instruction` for the player in `slot`.
    pub fn holds(&self, instruction: Instruction, slot: usize, schema: &TacticsSchema) -> bool {
        instruction.held(self, slot, schema)
    }

    /// The bits of every [`Instruction`] these tactics hold for the player in `slot`.
    pub fn held(&self, slot: usize, schema: &TacticsSchema) -> u16 {
        Instruction::ALL
            .iter()
            .filter(|i| self.holds(**i, slot, schema))
            .fold(0, |bits, i| bits | i.bit())
    }
}

/// A change to a team's tactics. Every field is optional; a role change names the player by
/// squad index (the player's place in the team file).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TacticsPatch {
    pub formation: Option<u8>,
    pub mentality: Option<u8>,
    pub instructions: [Option<u8>; 6],
    pub roles: Vec<(usize, RoleDuty)>,
}

impl TacticsPatch {
    /// A patch that sets only the mentality.
    pub fn mentality(m: u8) -> Self {
        Self {
            mentality: Some(m),
            ..Self::default()
        }
    }

    /// A patch that sets only instruction `index` to `level`.
    pub fn instruction(index: usize, level: u8) -> Self {
        let mut p = Self::default();
        p.instructions[index] = Some(level);
        p
    }

    /// `tactics` with this patch applied for a team whose slots hold `lineup` (squad indices
    /// in slot order): the formation first, then the mentality, the instructions, and each
    /// named player's role. A role for a player not in `lineup` is skipped; the caller checks
    /// that first.
    pub fn applied_to(
        &self,
        mut tactics: Tactics,
        lineup: &[usize; PLAYERS_PER_TEAM],
        schema: &TacticsSchema,
    ) -> Tactics {
        if let Some(f) = self.formation {
            tactics.set_formation(f, schema);
        }
        if let Some(m) = self.mentality {
            tactics.mentality = m;
        }
        for (level, set) in tactics.instructions.iter_mut().zip(self.instructions) {
            if let Some(l) = set {
                *level = l;
            }
        }
        for (squad, rd) in &self.roles {
            if let Some(slot) = lineup.iter().position(|s| s == squad) {
                tactics.roles[slot] = *rd;
            }
        }
        tactics
    }

    /// `true` when every index lies inside the tactics file.
    pub fn in_range(&self, schema: &TacticsSchema) -> bool {
        let formation = self
            .formation
            .is_none_or(|f| usize::from(f) < schema.formations.len());
        let mentality = self
            .mentality
            .is_none_or(|m| usize::from(m) < schema.mentalities.len());
        let instructions =
            self.instructions.iter().enumerate().all(|(i, l)| {
                l.is_none_or(|l| usize::from(l) < schema.instructions.level_count(i))
            });
        let roles = self.roles.iter().all(|(_, rd)| {
            usize::from(rd.role) < schema.roles.len() && usize::from(rd.duty) < schema.duties.len()
        });
        formation && mentality && instructions && roles
    }
}

/// A role's stand options for one phase, in preference order; `None` ends the list.
pub type Menu = [Option<StandOption>; MENU_MAX];

/// `list` as a [`Menu`]. A checked file lists at most [`MENU_MAX`] options.
fn menu(list: &[StandOption]) -> Menu {
    let mut out = [None; MENU_MAX];
    for (o, &option) in out.iter_mut().zip(list) {
        *o = Some(option);
    }
    out
}

/// What one slot's position, role and duty add to play.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SlotPlan {
    /// The position the formation gives the slot; `None` in the neutral plan.
    pub position: Option<Position>,
    /// Metres the anchor moves up the pitch.
    pub depth: f64,
    pub shoot: f64,
    pub dribble: f64,
    /// Added to a forward pass in proportion to its progress.
    pub progress: f64,
    /// Metres the anchor moves while the team has the ball, and while it does not: the
    /// position's stand plus the role's offset.
    pub in_possession: Offset,
    pub out_of_possession: Offset,
    /// Added to a teammate's pass to this slot in proportion to the pass length.
    pub long_ball_target: f64,
    /// The role's stand options while the team has the ball, and while it does not.
    pub menu_in: Menu,
    pub menu_out: Menu,
}

/// The numbers play reads from a team's tactics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeamPlan {
    /// Metres the back line moves up out of possession: the line height.
    pub block_depth: f64,
    /// Metres the out-of-possession anchors in front of the back line move up the pitch: the
    /// line of engagement's shape.
    pub engage_depth_m: f64,
    /// The line of engagement's press start: a player presses the carrier only while the ball
    /// lies within this share of the pitch length of the own goal line; 1 never limits.
    pub engage_line: f64,
    /// Defensive width: a scale on every out-of-possession anchor's distance from the centre
    /// line; 1 moves nothing.
    pub defensive_width: f64,
    /// Force the opponent: the presser's aim moves toward the near touchline above 0 and the
    /// centre below 0, beside the weaker-foot show; 0 moves nothing.
    pub force: f64,
    /// Tackling: added to the tackle decision's score; 0 is the neutral form.
    pub tackle_willingness: f64,
    /// Tackling: a shift of the log-odds of a tackle's foul chance; 0 moves nothing.
    pub tackle_risk: f64,
    /// The offside trap: an unpressed opposing carrier facing forward starts a short window in
    /// which the back line steps up together.
    pub offside_trap: bool,
    /// Stop short goal kicks: at an opponent goal kick the front players stand on the keeper's
    /// short options outside the area.
    pub screen_short: bool,
    /// The press triggers: each opens the press-trigger window, in which the press reach
    /// grows.
    pub trigger_back_pass: bool,
    pub trigger_poor_touch: bool,
    pub trigger_facing_back: bool,
    pub trigger_wide: bool,
    /// A scale on every slot's distance from the centre line while the side has the ball.
    pub width: f64,
    /// Width's second input: added to a pass whose target lies in a wide channel of the
    /// middle or final third.
    pub wide_target: f64,
    /// Opponents who press this team's opponents' carrier, and from how far.
    pub press_count: usize,
    pub press_distance: f64,
    /// Pressing's third input: the willingness a presser (a player on a press job) adds to
    /// his tackle decision; 0 at the default level.
    pub press_willingness: f64,
    /// The widest gap between two neighbours in the back line, from the tuning.
    pub back_line_gap: f64,
    /// The back line's own share of the line's organisation beside the keeper's, from the
    /// tuning's hooks.
    pub organise_back_line: f64,
    /// The patience of the circulation: added to a short pass that does not go forward.
    pub patience: f64,
    /// The weight of a pass's gain toward goal, per 45 m gained; a back pass gains none.
    pub directness: f64,
    /// A factor on the team's restart delay while it leads.
    pub time_wasting: f64,
    /// On winning the ball above its default level: while the side's regain window is open,
    /// pressing and support jobs rise one urgency grade ([`crate::pace`]). No tempo or pressing
    /// level lifts running pace.
    pub pace_lift: bool,
    /// How much a lost ball's danger counts against each option's gain: the mentality's
    /// appetite for risk.
    pub loss_weight: f64,
    /// The fewest outfield players who stay goal-side of the ball while the team has it.
    pub rest_defence: usize,
    /// The sizes of the stand options; `None` takes no option.
    pub sizes: Option<StandOptionSizes>,
    /// The rest-defence guard; `None` holds nobody back.
    pub rest: Option<RestDefence>,
    pub slots: [SlotPlan; PLAYERS_PER_TEAM],
    /// The bits of every [`Instruction`] the tactics hold for each slot's player: the team's
    /// levels and mentality, and the slot's duty. A trait of play reads them.
    pub held: [u16; PLAYERS_PER_TEAM],
    /// Per slot: a deep line plays the keeper's rush off his line down, a high line his
    /// staying on it; the keeper's chase range reads them.
    pub keeper_rush_off: [bool; PLAYERS_PER_TEAM],
    pub keeper_stay_off: [bool; PLAYERS_PER_TEAM],
    /// The anchor depth, in metres, of the attack duty and of the defend duty, for an
    /// override that plays a player at another duty's depth; 0 when the file has no such duty.
    pub attack_depth: f64,
    pub defend_depth: f64,
    /// Build-up: added, in proportion to the pass length, to a long pass that starts in the
    /// side's own third, the keeper's included; the keeper's goal-kick pick reads it too.
    pub build_up: f64,
    /// The attacking channel: added to a pass whose target lies in the left, the centre or
    /// the right channel of the middle or final third, left as the side's left-back stands.
    pub channel_left: f64,
    pub channel_centre: f64,
    pub channel_right: f64,
    /// Final third: added to a shot from outside the penalty area.
    pub long_shot: f64,
    /// Final third: metres added to the range within which a shot is considered.
    pub shot_reach_m: f64,
    /// Dribbling: added to the dribble score.
    pub dribble: f64,
    /// Crossing frequency: added to a cross.
    pub crossing: f64,
    /// Cross type: added to a cross from far out (an early cross).
    pub early_cross: f64,
    /// Cross type: a cross's trajectory; below 0 driven low, above 0 floated, 0 the length
    /// rule of every pass.
    pub cross_loft: f64,
    /// Creative freedom: a factor on the spread of the choice noise around the plan's
    /// preferences.
    pub freedom: f64,
    /// Forward runs: how far from the defensive line, in metres, a forward may start a timed
    /// run: in behind above 0, coming short below 0; none at 0.
    pub run_start_m: f64,
    /// Full-back runs: -1 the underlap, 1 the overlap goes first in a full-back's menu and
    /// holds wherever the team-mate ahead stands; 0 keeps the role's menu.
    pub fullback_run: f64,
    /// Full-back runs: a factor on the reach of a full-back's overlap and underlap.
    pub fullback_reach: f64,
    /// On winning the ball: while the side's regain window is open, added per 45 m of a
    /// pass's gain toward goal, taken from the hold, and, above 0, the metres per unit from
    /// the defensive line within which an attack-line player may start a run in behind
    /// ([`crate::decision::COUNTER_RUN_M`]); 0 moves nothing.
    pub counter: f64,
    /// On losing the ball: while the side's counter-press window is open, its press reach
    /// takes the factor 1 plus this (0 at -1: nobody presses).
    pub counter_press: f64,
    /// Counter-press length: the seconds the counter-press window stays open after an
    /// open-play loss; 0 opens none.
    pub counter_press_s: f64,
}

impl TeamPlan {
    /// Sets the plan input `input` that a primitive's own step or choice holds; an input the
    /// plan does not hold is ignored (a checked tactics file names none).
    fn set_input(&mut self, input: &str, v: f64) {
        let field = match input {
            "build_up" => &mut self.build_up,
            "channel_left" => &mut self.channel_left,
            "channel_centre" => &mut self.channel_centre,
            "channel_right" => &mut self.channel_right,
            "long_shot" => &mut self.long_shot,
            "shot_reach_m" => &mut self.shot_reach_m,
            "dribble" => &mut self.dribble,
            "crossing" => &mut self.crossing,
            "early_cross" => &mut self.early_cross,
            "cross_loft" => &mut self.cross_loft,
            "freedom" => &mut self.freedom,
            "run_start_m" => &mut self.run_start_m,
            "fullback_run" => &mut self.fullback_run,
            "fullback_reach" => &mut self.fullback_reach,
            "counter" => &mut self.counter,
            "pace_lift" => {
                self.pace_lift = v > 0.0;
                return;
            }
            "counter_press" => &mut self.counter_press,
            "counter_press_s" => &mut self.counter_press_s,
            "engage_depth_m" => &mut self.engage_depth_m,
            "engage_line" => &mut self.engage_line,
            "defensive_width" => &mut self.defensive_width,
            "force" => &mut self.force,
            "tackle_willingness" => &mut self.tackle_willingness,
            "tackle_risk" => &mut self.tackle_risk,
            "offside_trap"
            | "screen_short"
            | "trigger_back_pass"
            | "trigger_poor_touch"
            | "trigger_facing_back"
            | "trigger_wide" => {
                let on = v > 0.0;
                match input {
                    "offside_trap" => self.offside_trap = on,
                    "screen_short" => self.screen_short = on,
                    "trigger_back_pass" => self.trigger_back_pass = on,
                    "trigger_poor_touch" => self.trigger_poor_touch = on,
                    "trigger_facing_back" => self.trigger_facing_back = on,
                    _ => self.trigger_wide = on,
                }
                return;
            }
            _ => return,
        };
        *field = v;
    }

    /// The plan of `tactics` under the tactics file and the tuning.
    pub fn from(tactics: &Tactics, schema: &TacticsSchema, t: &Tuning) -> Self {
        let level = |index: usize| usize::from(tactics.instructions[index]);
        let ins = &schema.instructions;
        let held: [u16; PLAYERS_PER_TEAM] = std::array::from_fn(|slot| tactics.held(slot, schema));
        let press = &ins.pressing.levels[level(PRESSING)];
        let mentality = &schema.mentalities[usize::from(tactics.mentality)];
        let formation = &schema.formations[usize::from(tactics.formation)];
        // The position's stand from its slot, then the role's own offset.
        let add = |a: StandOffset, b: Offset| Offset {
            x: a.x + b.x,
            y: a.y + b.y,
        };
        let duty_depth = |name: &str| {
            schema
                .duty_index(name)
                .map_or(0.0, |d| schema.duties[d].depth)
        };
        let mut slots = [SlotPlan::default(); PLAYERS_PER_TEAM];
        for ((plan, rd), slot) in slots.iter_mut().zip(&tactics.roles).zip(&formation.slots) {
            let role = &schema.roles[usize::from(rd.role)];
            let duty = &schema.duties[usize::from(rd.duty)];
            let actions = &role.preferred_actions;
            let stand = schema.stands.of(slot.position);
            *plan = SlotPlan {
                position: Some(slot.position),
                depth: duty.depth,
                shoot: actions.shoot * duty.scale,
                dribble: actions.dribble * duty.scale + duty.risk,
                progress: actions.progress * duty.scale + duty.risk,
                in_possession: add(stand.in_possession, role.in_possession),
                out_of_possession: add(stand.out_of_possession, role.out_of_possession),
                long_ball_target: role.teammates.long_ball_target,
                menu_in: menu(&role.options.in_possession),
                menu_out: menu(&role.options.out_of_possession),
            };
        }
        let mut plan = Self {
            block_depth: ins.line_height.levels[level(LINE_HEIGHT)].value,
            width: ins.width.levels[level(WIDTH)].value,
            wide_target: ins.width.levels[level(WIDTH)].wide_target,
            press_count: usize::from(press.press_count),
            press_distance: t.press_distance * press.press_distance_scale,
            press_willingness: press.press_willingness,
            back_line_gap: t.back_line_gap,
            organise_back_line: t.contract.hooks().organise_back_line,
            patience: ins.tempo.levels[level(TEMPO)].value,
            directness: ins.passing_directness.levels[level(DIRECTNESS)].value,
            time_wasting: ins.time_wasting.levels[level(TIME_WASTING)].value,
            loss_weight: mentality.loss_weight,
            rest_defence: usize::from(mentality.rest_defence),
            sizes: schema.stand_options,
            rest: schema.rest_defence,
            slots,
            held,
            keeper_rush_off: held.map(|bits| bits & Instruction::LineDeep.bit() != 0),
            keeper_stay_off: held.map(|bits| bits & Instruction::LineHigh.bit() != 0),
            attack_depth: duty_depth("attack"),
            defend_depth: duty_depth("defend"),
            ..Self::neutral(t)
        };
        // Each primitive that owns its levels or choices sets its own inputs from the level
        // or choice the tactics store.
        for (p, &at) in settings(schema).zip(&tactics.settings) {
            for ih in &p.inputs {
                let own = input_field(&ih.input).is_some_and(|(block, ..)| block == OWN);
                if let Some(v) = p.own_value(&ih.input, usize::from(at)).filter(|_| own) {
                    plan.set_input(&ih.input, v);
                }
            }
        }
        plan
    }

    /// The plan of the tactics file's defaults: the neutral plan the geometry tests use. Every
    /// input a primitive's own steps or choices hold is at its neutral value, where it moves
    /// nothing.
    pub fn neutral(t: &Tuning) -> Self {
        Self {
            block_depth: 0.0,
            engage_depth_m: 0.0,
            engage_line: 1.0,
            defensive_width: 1.0,
            force: 0.0,
            tackle_willingness: 0.0,
            tackle_risk: 0.0,
            offside_trap: false,
            screen_short: false,
            trigger_back_pass: false,
            trigger_poor_touch: false,
            trigger_facing_back: false,
            trigger_wide: false,
            width: 1.0,
            wide_target: 0.0,
            press_count: 2,
            press_distance: t.press_distance,
            press_willingness: 0.0,
            back_line_gap: t.back_line_gap,
            organise_back_line: t.contract.hooks().organise_back_line,
            patience: 0.0,
            directness: 0.0,
            time_wasting: 1.0,
            pace_lift: false,
            loss_weight: 0.0,
            rest_defence: 0,
            sizes: None,
            rest: None,
            slots: [SlotPlan::default(); PLAYERS_PER_TEAM],
            held: [0; PLAYERS_PER_TEAM],
            keeper_rush_off: [false; PLAYERS_PER_TEAM],
            keeper_stay_off: [false; PLAYERS_PER_TEAM],
            attack_depth: 0.0,
            defend_depth: 0.0,
            build_up: 0.0,
            channel_left: 0.0,
            channel_centre: 0.0,
            channel_right: 0.0,
            long_shot: 0.0,
            shot_reach_m: 0.0,
            dribble: 0.0,
            crossing: 0.0,
            early_cross: 0.0,
            cross_loft: 0.0,
            freedom: 1.0,
            run_start_m: 0.0,
            fullback_run: 0.0,
            fullback_reach: 1.0,
            counter: 0.0,
            counter_press: 0.0,
            counter_press_s: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;
    use crate::team::FORMATION_442;

    #[test]
    fn balanced_four_four_two_with_support_duties_is_the_neutral_plan() {
        let content = shipped_content();
        let schema = &content.tactics;
        let t = &content.tuning.engine;
        let tactics = Tactics::defaults(schema);
        assert_eq!(schema.slots(usize::from(tactics.formation)), FORMATION_442);
        let plan = TeamPlan::from(&tactics, schema, t);
        assert_eq!(plan.block_depth, 0.0);
        assert_eq!(plan.width, 1.0);
        assert_eq!(plan.press_count, 2);
        assert_eq!(plan.press_distance, t.press_distance);
        assert_eq!(plan.time_wasting, 1.0);
        assert!(!plan.pace_lift);
        assert!(plan.slots.iter().all(|s| s.depth == 0.0));
    }

    #[test]
    fn a_high_line_moves_the_block_up_attacking_seeks_risk_and_high_pressing_sends_three() {
        let content = shipped_content();
        let schema = &content.tactics;
        let t = &content.tuning.engine;
        let mut tactics = Tactics::defaults(schema);
        tactics.mentality = schema.mentality_index("attacking").unwrap() as u8;
        tactics.instructions[PRESSING] = schema.instructions.defaults()[PRESSING] + 1;
        let plan = TeamPlan::from(&tactics, schema, t);
        assert_eq!(
            plan.block_depth, 0.0,
            "the mentality does not move the block"
        );
        tactics.instructions[LINE_HEIGHT] = schema.instructions.defaults()[LINE_HEIGHT] + 1;
        let plan = TeamPlan::from(&tactics, schema, t);
        assert!(plan.block_depth > 0.0);
        let balanced = TeamPlan::from(&Tactics::defaults(schema), schema, t);
        assert!(plan.loss_weight < balanced.loss_weight);
        assert_eq!(plan.press_count, 3);
        // The level's reach is its fitted scale on the tuning distance: the extra presser, not
        // a longer reach, makes high pressing press more.
        let high = &schema.instructions.pressing.levels[tactics.instructions[PRESSING] as usize];
        assert_eq!(
            plan.press_distance,
            t.press_distance * high.press_distance_scale
        );
        assert!(!plan.pace_lift, "pressing lifts no running pace");
    }

    #[test]
    fn a_plan_holds_each_named_level_duty_and_mentality_for_its_slot() {
        let content = shipped_content();
        let schema = &content.tactics;
        let t = &content.tuning.engine;
        let mut tactics = Tactics::defaults(schema);
        let plan = TeamPlan::from(&tactics, schema, t);
        assert!(
            plan.held.iter().all(|&bits| bits == 0),
            "the AI default holds none"
        );
        tactics.instructions[DIRECTNESS] = 0;
        tactics.mentality = schema.mentality_index("cautious").unwrap() as u8;
        tactics.roles[6].duty = schema.duty_index("defend").unwrap() as u8;
        let plan = TeamPlan::from(&tactics, schema, t);
        for slot in 0..PLAYERS_PER_TEAM {
            let bits = plan.held[slot];
            assert!(bits & Instruction::DirectnessShort.bit() != 0);
            assert!(bits & Instruction::MentalityCautious.bit() != 0);
            assert!(bits & Instruction::MentalityDefensive.bit() == 0);
            assert_eq!(bits & Instruction::DutyDefend.bit() != 0, slot == 6);
        }
        assert!(plan.attack_depth > plan.defend_depth);
        for i in Instruction::ALL {
            assert_eq!(Instruction::from_code(i.code()), Some(i));
        }
    }

    #[test]
    fn a_new_formation_resets_only_the_roles_that_no_longer_suit() {
        let content = shipped_content();
        let schema = &content.tactics;
        let mut tactics = Tactics::defaults(schema);
        let before = tactics.roles;
        tactics.set_formation(schema.formation_index("4-3-3").unwrap() as u8, schema);
        // The back four and the goalkeeper keep their roles.
        assert_eq!(&tactics.roles[..5], &before[..5]);
        // Slot 10 was a striker and stays one; slot 8 turns from a right winger's slot into
        // a left winger's, which the winger role also suits.
        let slot6 = schema.formations[1].slots[6].position;
        assert!(
            schema.roles[usize::from(tactics.roles[6].role)]
                .positions
                .contains(&slot6)
        );
    }
}
