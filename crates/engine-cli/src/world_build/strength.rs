//! The strength rule. A club with a ClubElo link takes its ClubElo rating. Any other club
//! takes, in order: the median rating of the rated clubs of its league, when at least 3 are
//! rated (`league-median`); the median of the nearest rated tier above it in its country, less
//! the world's median drop per tier for each tier between (`tier-step`); the 10th percentile
//! of all rated clubs (`floor`).
//!
//! A tier's median is the median of its rated leagues' medians. The world's drop per tier is
//! the median, over every country and every pair of adjacent rated tiers, of the upper tier's
//! median less the lower one's.

use std::collections::BTreeMap;

use engine::data::world::StrengthSource;

/// Rated clubs a league needs for its median to count.
pub const MIN_RATED: usize = 3;

/// One league as the rule sees it.
#[derive(Debug, Clone)]
pub struct LeagueIn {
    pub country: String,
    pub tier: Option<u8>,
    /// The ratings of its clubs with a ClubElo link.
    pub rated: Vec<f64>,
}

/// The median of `v` (the mean of the two middle values for an even count); `None` when
/// empty.
pub fn median(v: &[f64]) -> Option<f64> {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(s[n / 2]),
        _ => Some((s[n / 2 - 1] + s[n / 2]) / 2.0),
    }
}

/// The `p` percentile of `v` by the lower nearest rank: the value at index
/// floor((n - 1) * p / 100) of the sorted values.
pub fn percentile(v: &[f64], p: usize) -> Option<f64> {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    (!s.is_empty()).then(|| s[(s.len() - 1) * p / 100])
}

/// The derived strengths of the leagues: for each league id, the rating and source that an
/// unrated club of it takes.
#[derive(Debug, Clone)]
pub struct Derived {
    pub by_league: BTreeMap<String, (f64, StrengthSource)>,
    /// The world's median drop per tier, when any country has two adjacent rated tiers.
    pub tier_drop: Option<f64>,
    pub floor: f64,
}

/// Applies the rule to `leagues` (by id).
pub fn derive(leagues: &BTreeMap<String, LeagueIn>) -> Derived {
    let all: Vec<f64> = leagues.values().flat_map(|l| l.rated.clone()).collect();
    let floor = percentile(&all, 10).unwrap_or(1000.0);
    let league_median: BTreeMap<&String, f64> = leagues
        .iter()
        .filter(|(_, l)| l.rated.len() >= MIN_RATED)
        .filter_map(|(id, l)| Some((id, median(&l.rated)?)))
        .collect();
    let mut tiers: BTreeMap<(&str, u8), Vec<f64>> = BTreeMap::new();
    for (id, m) in &league_median {
        let l = &leagues[*id];
        if let Some(t) = l.tier {
            tiers.entry((l.country.as_str(), t)).or_default().push(*m);
        }
    }
    let tier_median: BTreeMap<(&str, u8), f64> = tiers
        .into_iter()
        .filter_map(|(k, v)| Some((k, median(&v)?)))
        .collect();
    let drops: Vec<f64> = tier_median
        .iter()
        .filter_map(|(&(c, t), m)| Some(m - tier_median.get(&(c, t + 1))?))
        .collect();
    let tier_drop = median(&drops);
    let mut by_league = BTreeMap::new();
    for (id, l) in leagues {
        let derived = if let Some(m) = league_median.get(id) {
            (*m, StrengthSource::LeagueMedian)
        } else if let (Some(t), Some(drop)) = (l.tier, tier_drop)
            && let Some((above, m)) = (1..t)
                .rev()
                .find_map(|u| Some((u, *tier_median.get(&(l.country.as_str(), u))?)))
        {
            (m - drop * f64::from(t - above), StrengthSource::TierStep)
        } else {
            (floor, StrengthSource::Floor)
        };
        by_league.insert(id.clone(), derived);
    }
    Derived {
        by_league,
        tier_drop,
        floor,
    }
}

/// A rating as the file stores it: rounded, at least 1, at most 3,000.
pub fn stored(elo: f64) -> u16 {
    elo.round().clamp(1.0, 3000.0) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn league(country: &str, tier: Option<u8>, rated: &[f64]) -> LeagueIn {
        LeagueIn {
            country: country.into(),
            tier,
            rated: rated.to_vec(),
        }
    }

    #[test]
    fn median_and_percentile_follow_their_definitions() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), Some(2.5));
        assert_eq!(median(&[]), None);
        let v: Vec<f64> = (1..=21).map(f64::from).collect();
        assert_eq!(percentile(&v, 10), Some(3.0));
        assert_eq!(percentile(&[7.0], 10), Some(7.0));
    }

    #[test]
    fn each_step_of_the_rule_applies_in_order() {
        let leagues = BTreeMap::from([
            // A: tiers 1 and 2 rated, drop 300; tier 3 has 2 rated clubs (too few), tier 4
            // none; an untiered league.
            (
                "a1".into(),
                league("AAA", Some(1), &[1800.0, 1700.0, 1600.0]),
            ),
            (
                "a2".into(),
                league("AAA", Some(2), &[1400.0, 1400.0, 1400.0]),
            ),
            ("a3".into(), league("AAA", Some(3), &[1300.0, 1100.0])),
            ("a4".into(), league("AAA", Some(4), &[])),
            ("ax".into(), league("AAA", None, &[])),
            // B: tiers 1 and 2 rated, drop 100.
            (
                "b1".into(),
                league("BBB", Some(1), &[1500.0, 1500.0, 1500.0]),
            ),
            (
                "b2".into(),
                league("BBB", Some(2), &[1400.0, 1400.0, 1400.0]),
            ),
            // C: tier 1 rated, tier 2 not; no tier above C's tier 1.
            (
                "c1".into(),
                league("CCC", Some(1), &[1600.0, 1600.0, 1600.0]),
            ),
            ("c2".into(), league("CCC", Some(2), &[])),
            ("d1".into(), league("DDD", Some(1), &[])),
        ]);
        let d = derive(&leagues);
        assert_eq!(d.tier_drop, Some(200.0));
        assert_eq!(d.by_league["a1"], (1700.0, StrengthSource::LeagueMedian));
        assert_eq!(d.by_league["a3"], (1200.0, StrengthSource::TierStep));
        assert_eq!(d.by_league["a4"], (1000.0, StrengthSource::TierStep));
        assert_eq!(d.by_league["c2"], (1400.0, StrengthSource::TierStep));
        let all = [
            1800.0, 1700.0, 1600.0, 1400.0, 1400.0, 1400.0, 1300.0, 1100.0, 1500.0, 1500.0, 1500.0,
            1400.0, 1400.0, 1400.0, 1600.0, 1600.0, 1600.0,
        ];
        let floor = percentile(&all, 10).unwrap();
        assert_eq!(d.floor, floor);
        assert_eq!(d.by_league["ax"], (floor, StrengthSource::Floor));
        assert_eq!(d.by_league["d1"], (floor, StrengthSource::Floor));
    }

    #[test]
    fn without_two_adjacent_rated_tiers_there_is_no_tier_step() {
        let leagues = BTreeMap::from([
            (
                "a1".into(),
                league("AAA", Some(1), &[1500.0, 1500.0, 1500.0]),
            ),
            ("a2".into(), league("AAA", Some(2), &[])),
        ]);
        let d = derive(&leagues);
        assert_eq!(d.tier_drop, None);
        assert_eq!(d.by_league["a2"].1, StrengthSource::Floor);
    }

    #[test]
    fn a_stored_rating_is_rounded_and_bounded() {
        assert_eq!(stored(1499.6), 1500);
        assert_eq!(stored(-28.0), 1);
    }
}
