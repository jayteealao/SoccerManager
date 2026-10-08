//! The soft combine across modifier families.

use super::FAMILY_COUNT;

/// The weight of the j-th delta in one direction: the first counts in full, and each further
/// one counts for half as much as the one before. Exact powers of two.
const WEIGHTS: [f64; FAMILY_COUNT] = [1.0, 0.5, 0.25, 0.125];

/// Combines one delta per family into one delta. Deltas of exactly 0 are skipped. The rest
/// split into those below 0 and those above; each group is sorted by size, largest first.
/// The first delta of a direction counts as it is, and the j-th later one times
/// `WEIGHTS[j]`, so each further effect in the same direction counts for less than the one
/// before. On the curve a delta is a factor `e^(d / width)`, so this is the combine a factor
/// `f^w` was.
///
/// One delta alone passes through bit for bit.
pub fn soft_combine(deltas: [f64; FAMILY_COUNT]) -> f64 {
    let (mut below, mut above) = ([0.0; FAMILY_COUNT], [0.0; FAMILY_COUNT]);
    let (mut nb, mut na) = (0, 0);
    for d in deltas {
        if d < 0.0 {
            below[nb] = d;
            nb += 1;
        } else if d > 0.0 {
            above[na] = d;
            na += 1;
        }
    }
    // One delta or none: nothing to sort or weigh, so the delta alone is the combine.
    match (nb, na) {
        (0, 0) => return 0.0,
        (1, 0) => return below[0],
        (0, 1) => return above[0],
        _ => {}
    }
    let (below, above) = (&mut below[..nb], &mut above[..na]);
    below.sort_unstable_by(|a, b| a.total_cmp(b));
    above.sort_unstable_by(|a, b| b.total_cmp(a));
    let mut acc = 0.0;
    for group in [&*below, &*above] {
        for (j, &d) in group.iter().enumerate() {
            acc += WEIGHTS[j] * d;
        }
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `d` in each family position, the other three neutral.
    fn alone(d: f64) -> [[f64; FAMILY_COUNT]; FAMILY_COUNT] {
        std::array::from_fn(|k| {
            let mut f = [0.0; FAMILY_COUNT];
            f[k] = d;
            f
        })
    }

    #[test]
    fn one_delta_alone_passes_through_bit_for_bit() {
        let sweep = (0..=10_000).map(|k| -5.0 + 10.0 * f64::from(k) / 10_000.0);
        for d in sweep {
            for deltas in alone(d) {
                assert_eq!(soft_combine(deltas).to_bits(), d.to_bits(), "{deltas:?}");
            }
        }
    }

    #[test]
    fn all_neutral_is_exactly_zero() {
        assert_eq!(
            soft_combine([0.0; FAMILY_COUNT]).to_bits(),
            0.0f64.to_bits()
        );
    }

    #[test]
    fn a_second_delta_in_one_direction_counts_for_less() {
        for (a, b) in [(-0.8, -0.5), (1.2, 0.4)] {
            let combined: f64 = soft_combine([a, b, 0.0, 0.0]);
            let larger = f64::max(f64::abs(a), f64::abs(b));
            assert!(combined.abs() > larger, "{a} {b}: {combined}");
            assert!(combined.abs() < (a + b).abs(), "{a} {b}: {combined}");
        }
    }

    #[test]
    fn opposite_directions_both_count() {
        let combined = soft_combine([-0.8, 0.0, 0.3, 0.0]);
        assert_eq!(combined.to_bits(), (-0.8f64 + 0.3).to_bits());
    }

    #[test]
    fn the_order_of_the_families_does_not_matter() {
        let deltas = [-0.7, 1.3, -0.9, 0.05];
        let expected = soft_combine(deltas).to_bits();
        let mut seen = 0;
        for order in 0..256_usize {
            let idx: [usize; FAMILY_COUNT] = std::array::from_fn(|k| (order >> (2 * k)) & 3);
            if (0..FAMILY_COUNT).any(|k| !idx.contains(&k)) {
                continue;
            }
            let perm = idx.map(|k| deltas[k]);
            assert_eq!(soft_combine(perm).to_bits(), expected, "{perm:?}");
            seen += 1;
        }
        assert_eq!(seen, 24, "every order of the four families");
    }
}
