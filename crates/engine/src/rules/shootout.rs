//! The penalty shoot-out (IFAB Law 10) as pure functions: who may kick, who keeps goal, the
//! kicking order, which team kicks next, and when the shoot-out is decided. The referee in
//! `rules` plays each kick on the pitch; this module only answers the law's questions.
//!
//! Only players on the pitch at the end of extra time take part. A team with more players
//! drops players until the numbers are equal, the weakest kickers first; the goalkeeper
//! stays. Each team kicks in turn, the team that won the toss first, `kicks` times each;
//! the shoot-out ends early once one team cannot catch up, and then continues in pairs of
//! kicks (sudden death). Once every eligible player of a team has kicked, its order starts
//! again.

/// Rounds after which a shoot-out that has not finished is stopped as a defect. With a
/// goalkeeper always present and noise on every shot, a level round is far from certain, so
/// 100 rounds never happen in play.
pub const SAFETY_ROUNDS: u32 = 100;

/// One player the shoot-out may use: a roster index and the two skills the law's choices
/// read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    /// Roster index.
    pub index: usize,
    /// `true` for the player keeping the team's goal when the shoot-out starts.
    pub goalkeeper: bool,
    /// Kicking skill: finishing plus composure.
    pub kicking: f64,
    /// Goalkeeping skill: reflexes plus one-on-ones.
    pub keeping: f64,
}

/// The candidates of both teams with the numbers made equal: the larger side drops outfield
/// players, lowest kicking skill first (the later roster index on a tie).
pub fn equalise(mut teams: [Vec<Candidate>; 2]) -> [Vec<Candidate>; 2] {
    let n = teams[0].len().min(teams[1].len());
    for side in &mut teams {
        while side.len() > n {
            let drop = side
                .iter()
                .enumerate()
                .filter(|(_, c)| !c.goalkeeper)
                .min_by(|(_, a), (_, b)| {
                    a.kicking.total_cmp(&b.kicking).then(b.index.cmp(&a.index))
                })
                .map_or(side.len() - 1, |(k, _)| k);
            side.remove(drop);
        }
    }
    teams
}

/// The player who keeps goal: the goalkeeper when eligible, otherwise the eligible player
/// with the highest goalkeeping skill (the earlier roster index on a tie). `None` for an
/// empty side.
pub fn keeper(eligible: &[Candidate]) -> Option<usize> {
    if let Some(gk) = eligible.iter().find(|c| c.goalkeeper) {
        return Some(gk.index);
    }
    eligible
        .iter()
        .max_by(|a, b| a.keeping.total_cmp(&b.keeping).then(b.index.cmp(&a.index)))
        .map(|c| c.index)
}

/// The kicking order: every eligible player but `keeper` by kicking skill, highest first
/// (the earlier roster index on a tie), then `keeper`.
pub fn order(eligible: &[Candidate], keeper: Option<usize>) -> Vec<usize> {
    let mut outfield: Vec<Candidate> = eligible
        .iter()
        .copied()
        .filter(|c| Some(c.index) != keeper)
        .collect();
    outfield.sort_by(|a, b| b.kicking.total_cmp(&a.kicking).then(a.index.cmp(&b.index)));
    let mut order: Vec<usize> = outfield.iter().map(|c| c.index).collect();
    order.extend(keeper);
    order
}

/// The team that kicks next: `first` when both teams have taken as many kicks, otherwise the
/// other team (A B A B).
pub fn next_team(first: usize, taken: [u32; 2]) -> usize {
    if taken[first] == taken[1 - first] {
        first
    } else {
        1 - first
    }
}

/// `true` once the shoot-out has a winner: within the first `kicks` kicks each, when one team
/// cannot catch up with the kicks it has left; after them, when both teams have taken as many
/// kicks and the scores differ.
pub fn decided(scores: [u32; 2], taken: [u32; 2], kicks: u32) -> bool {
    if taken[0] < kicks || taken[1] < kicks {
        let left = [0, 1].map(|t| kicks.saturating_sub(taken[t]));
        scores[0] + left[0] < scores[1] || scores[1] + left[1] < scores[0]
    } else {
        taken[0] == taken[1] && scores[0] != scores[1]
    }
}

/// The team that won a decided shoot-out.
pub fn winner(scores: [u32; 2]) -> Option<usize> {
    match scores[0].cmp(&scores[1]) {
        std::cmp::Ordering::Greater => Some(0),
        std::cmp::Ordering::Less => Some(1),
        std::cmp::Ordering::Equal => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(first: usize, kicking: &[f64]) -> Vec<Candidate> {
        kicking
            .iter()
            .enumerate()
            .map(|(k, &skill)| Candidate {
                index: first + k,
                goalkeeper: k == 0,
                kicking: skill,
                keeping: if k == 0 { 150.0 } else { 40.0 + k as f64 },
            })
            .collect()
    }

    #[test]
    fn the_larger_side_drops_its_weakest_kickers_and_keeps_its_goalkeeper() {
        // The home goalkeeper is the weakest kicker of all and still stays.
        let home = side(0, &[10.0, 120.0, 90.0, 150.0, 60.0]);
        let away = side(11, &[80.0, 100.0, 110.0]);
        let [home, away] = equalise([home, away]);
        assert_eq!(home.len(), 3);
        assert_eq!(away.len(), 3);
        let kept: Vec<usize> = home.iter().map(|c| c.index).collect();
        assert_eq!(kept, vec![0, 1, 3]);
    }

    #[test]
    fn the_goalkeeper_keeps_goal_otherwise_the_best_keeper_does() {
        let home = side(0, &[10.0, 120.0, 90.0]);
        assert_eq!(keeper(&home), Some(0));
        let without: Vec<Candidate> = home.into_iter().filter(|c| !c.goalkeeper).collect();
        // Index 2 has keeping 42 against 41 for index 1.
        assert_eq!(keeper(&without), Some(2));
        assert_eq!(keeper(&[]), None);
    }

    #[test]
    fn the_order_is_the_best_kickers_first_and_the_goalkeeper_last() {
        let home = side(0, &[200.0, 120.0, 90.0, 150.0, 120.0]);
        assert_eq!(order(&home, Some(0)), vec![3, 1, 4, 2, 0]);
    }

    #[test]
    fn the_teams_alternate_from_the_toss_winner() {
        let mut taken = [0, 0];
        let mut kickers = Vec::new();
        for _ in 0..6 {
            let team = next_team(1, taken);
            kickers.push(team);
            taken[team] += 1;
        }
        assert_eq!(kickers, vec![1, 0, 1, 0, 1, 0]);
    }

    #[test]
    fn three_to_nothing_after_six_kicks_ends_it_early() {
        assert!(decided([3, 0], [3, 3], 5));
        assert!(!decided([3, 1], [3, 3], 5));
        // After five kicks against four the trailing side still has one kick left.
        assert!(!decided([4, 3], [5, 4], 5));
        assert!(decided([4, 2], [5, 4], 5));
        assert!(!decided([0, 0], [0, 0], 5));
    }

    #[test]
    fn sudden_death_decides_a_round_the_teams_split() {
        assert!(!decided([5, 5], [5, 5], 5));
        assert!(!decided([6, 5], [6, 5], 5), "the second team still kicks");
        assert!(decided([6, 5], [6, 6], 5));
        assert!(!decided([6, 6], [6, 6], 5));
        assert_eq!(winner([6, 5]), Some(0));
        assert_eq!(winner([5, 5]), None);
    }
}
