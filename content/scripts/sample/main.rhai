// Sample script pack. Each hook is one function; see content/scripts/README.md for the
// context fields, the return values, and the sandbox limits.

// Decision hook: offsets added to the ball carrier's option scores. Return a map with any of
// pass, dribble, shoot, clear, and hold (each -10 to 10), or () to keep the engine's scores.
// The engine asks again for a new carrier, after a stoppage, and every refresh_ticks ticks.
fn decide(ctx) {
    let out = #{};
    // A team that is behind shoots more readily from inside 25 metres.
    if ctx.goal_diff < 0 && ctx.goal_distance < 25.0 {
        out.shoot = 0.4;
    }
    // Under close pressure, the carrier is less keen to dribble.
    if ctx.nearest_opponent < 2.5 {
        out.dribble = -0.3;
    }
    out
}

// Rule hook: the referee chose `native` ("none", "yellow", "second-yellow", or "red") for a
// foul. Return "none", "yellow", or "red", or () to keep the referee's card.
fn card(ctx, native) {
    // Early leniency: a first caution in the first 10 minutes becomes a word, not a card.
    if native == "yellow" && ctx.minute < 10 && ctx.yellows == 0 {
        return "none";
    }
}

// Commentary hook: `native` is the commentator's line. Return a new line of at most 280
// characters, or () to keep it.
fn line(ctx, native) {
    if ctx.kind == "goal" {
        let band = if ctx.minute < 15 {
            "Early"
        } else if ctx.minute >= 75 {
            "Late"
        } else {
            "Midway"
        };
        return band + " goal! " + native;
    }
}
