//! A player's overall level: one figure on the 1 to 20 scale, in tenths, for the page's level
//! column and its "plays between" range. It reads the visible ratings only, so a hidden value
//! never moves it, and it is read for display; play never uses it.

use crate::data::attributes::{AttributeSchema, Group};
use crate::data::team::Position;
use crate::player::Attributes;

/// The overall level of `ratings` for a player at `position`, in tenths of the 1 to 20 scale:
/// the mean of the visible attributes of the groups his position plays, rounded to a tenth.
/// A keeper plays the goalkeeping, mental and physical groups; an outfield player the
/// technical, mental and physical groups. A hidden value is never part of it. 0 when no
/// attribute counts.
pub fn overall_level(ratings: &Attributes, schema: &AttributeSchema, position: Position) -> u8 {
    let keeper = position == Position::GK;
    let (sum, count) = schema
        .attributes
        .iter()
        .enumerate()
        .filter(|(_, def)| !def.hidden)
        .filter(|(_, def)| match def.group {
            Group::Technical => !keeper,
            Group::Goalkeeping => keeper,
            Group::Mental | Group::Physical => true,
        })
        .fold((0u32, 0u32), |(s, n), (i, _)| {
            (s + u32::from(ratings.get(i).tenths()), n + 1)
        });
    if count == 0 {
        return 0;
    }
    u8::try_from((sum + count / 2) / count).unwrap_or(u8::MAX)
}

/// The mean of the visible ratings of `group` in `ratings`, on the 1 to 20 scale: the
/// sensitivity rules read the mental group to see adaptation act. 0 when the group is empty.
pub fn group_mean(ratings: &Attributes, schema: &AttributeSchema, group: Group) -> f64 {
    let (sum, count) = schema
        .attributes
        .iter()
        .enumerate()
        .filter(|(_, def)| !def.hidden && def.group == group)
        .fold((0.0, 0u32), |(s, n), (i, _)| {
            (s + ratings.get(i).decimal(), n + 1)
        });
    if count == 0 {
        0.0
    } else {
        sum / f64::from(count)
    }
}
