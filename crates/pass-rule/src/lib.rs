//! The realism pass rule.
//!
//! A band passes when the whole confidence interval of the engine statistic minus the real
//! value sits inside the band's own margin. This crate holds the statistics behind that rule
//! and nothing else: it plays no match and reads no engine state, so both the command line
//! and the engine's slow tests can judge a band with it. Randomness comes in as a closure
//! that returns uniform draws, so the caller keeps its own seeded stream.

pub mod agreement;
pub mod bounds;
pub mod change;
pub mod count;
pub mod floor;
pub mod interval;
pub mod limits;
pub mod rule;
pub mod shift;
pub mod special;
