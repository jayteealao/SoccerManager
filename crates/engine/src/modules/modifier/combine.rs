//! The soft combine across modifier families.

use super::FAMILY_COUNT;

/// The weight of the j-th factor in one direction: the first counts in full, and each
/// further one counts for half as much as the one before. Exact powers of two, so no
/// integer power whose rounding could differ between machines.
const WEIGHTS: [f64; FAMILY_COUNT] = [1.0, 0.5, 0.25, 0.125];

/// Combines one factor per family into one factor. Factors of exactly 1.0 are skipped. The
/// rest split into those below 1.0 and those above; each group is sorted by distance from
/// 1.0, largest first. The first factor of a direction counts as it is, and the j-th later
/// one as `f^WEIGHTS[j]`, so each further effect in the same direction counts for less than
/// the one before.
///
/// One factor alone passes through bit for bit: `1.0 * f == f` in IEEE 754.
pub fn soft_combine(factors: [f64; FAMILY_COUNT]) -> f64 {
    let (mut below, mut above) = ([0.0; FAMILY_COUNT], [0.0; FAMILY_COUNT]);
    let (mut nb, mut na) = (0, 0);
    for f in factors {
        if f < 1.0 {
            below[nb] = f;
            nb += 1;
        } else if f > 1.0 {
            above[na] = f;
            na += 1;
        }
    }
    // One factor or none: nothing to sort or weigh, and `1.0 * f == f`, so the factor alone
    // is the combine.
    match (nb, na) {
        (0, 0) => return 1.0,
        (1, 0) => return below[0],
        (0, 1) => return above[0],
        _ => {}
    }
    let (below, above) = (&mut below[..nb], &mut above[..na]);
    below.sort_unstable_by(|a, b| a.total_cmp(b));
    above.sort_unstable_by(|a, b| b.total_cmp(a));
    let mut acc = 1.0;
    for group in [&*below, &*above] {
        for (j, &f) in group.iter().enumerate() {
            acc *= if j == 0 { f } else { libm::pow(f, WEIGHTS[j]) };
        }
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;
    use crate::fatigue::multiplier;

    /// `m` in each family position, the other three neutral.
    fn alone(m: f64) -> [[f64; FAMILY_COUNT]; FAMILY_COUNT] {
        std::array::from_fn(|k| {
            let mut f = [1.0; FAMILY_COUNT];
            f[k] = m;
            f
        })
    }

    #[test]
    fn one_factor_alone_passes_through_bit_for_bit() {
        let fatigue = shipped_content().tuning.fatigue;
        let sweep = (0..=10_000).map(|k| 1.5 * f64::from(k) / 10_000.0);
        let curve_points = fatigue.curve.iter().map(|p| multiplier(p[0], &fatigue));
        let curve_sweep = (0..=1_000).map(|k| multiplier(f64::from(k) / 1_000.0, &fatigue));
        for m in sweep.chain(curve_points).chain(curve_sweep) {
            for factors in alone(m) {
                assert_eq!(soft_combine(factors).to_bits(), m.to_bits(), "{factors:?}");
            }
        }
    }

    #[test]
    fn all_neutral_is_exactly_one() {
        assert_eq!(
            soft_combine([1.0; FAMILY_COUNT]).to_bits(),
            1.0f64.to_bits()
        );
    }

    #[test]
    fn a_second_factor_in_one_direction_counts_for_less() {
        for (a, b) in [(0.8, 0.9), (1.2, 1.1)] {
            let combined = soft_combine([a, b, 1.0, 1.0]);
            let larger: f64 = f64::max((a - 1.0_f64).abs(), (b - 1.0_f64).abs());
            let product = (a * b - 1.0_f64).abs();
            let deviation = (combined - 1.0).abs();
            assert!(deviation > larger, "{a} {b}: {combined}");
            assert!(deviation < product, "{a} {b}: {combined}");
        }
    }

    #[test]
    fn opposite_directions_both_count() {
        let combined = soft_combine([0.8, 1.0, 1.1, 1.0]);
        assert!(combined > 0.8 && combined < 1.0, "{combined}");
        assert_eq!(combined.to_bits(), (0.8f64 * 1.1).to_bits());
    }

    #[test]
    fn the_order_of_the_families_does_not_matter() {
        let factors = [0.7, 1.3, 0.9, 1.05];
        let expected = soft_combine(factors).to_bits();
        let mut seen = 0;
        for order in 0..256_usize {
            let idx: [usize; FAMILY_COUNT] = std::array::from_fn(|k| (order >> (2 * k)) & 3);
            if (0..FAMILY_COUNT).any(|k| !idx.contains(&k)) {
                continue;
            }
            let perm = idx.map(|k| factors[k]);
            assert_eq!(soft_combine(perm).to_bits(), expected, "{perm:?}");
            seen += 1;
        }
        assert_eq!(seen, 24, "every order of the four families");
    }
}
