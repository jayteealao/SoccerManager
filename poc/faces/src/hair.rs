//! Hair: the scalp region (hairline, recession) and, in `cards`, the
//! hair-card, coil-shell and twisted-tube geometry.

use crate::age::{smoothstep, AgeState};
use crate::genome::Genome;
use glam::Vec3;

/// Centre used to measure angles around the skull (base-mesh coordinates).
pub const SKULL_AXIS_Z: f32 = 0.5;

/// Hair density on the scalp at a base-mesh point, 0..1, after the genome's
/// hairline and the age's recession.
pub fn scalp_density(b: Vec3, g: &Genome, age: &AgeState) -> f32 {
    let az = b.x.atan2(b.z - SKULL_AXIS_Z).abs(); // 0 front, PI back
    let rec = age.recession;
    let hl = g.hairline.height * 0.06;
    // Hairline height by angle: front, temple, sideburn, over ear, behind ear, nape.
    let table: [(f32, f32); 7] = [
        (0.00, 7.96 + hl + 0.34 * rec),
        (0.55, 7.88 + hl + 0.5 * rec),
        (0.95, 7.42 + 0.10 * rec),
        (1.30, 7.72),
        (1.75, 7.58),
        (2.40, 6.95),
        (std::f32::consts::PI, 6.82),
    ];
    let mut line = table[table.len() - 1].1;
    for w in table.windows(2) {
        if az >= w[0].0 && az <= w[1].0 {
            let t = (az - w[0].0) / (w[1].0 - w[0].0);
            line = w[0].1 + (w[1].1 - w[0].1) * t;
            break;
        }
    }
    let mut d = smoothstep(line - 0.02, line + 0.08, b.y);
    // Keep hair off the ears.
    let ear = smoothstep(0.62, 0.72, b.x.abs()) * (1.0 - smoothstep(7.62, 7.75, b.y)) * smoothstep(-0.1, 0.1, b.z);
    d *= 1.0 - ear;
    // Crown thinning for strong recession.
    let crown = Vec3::new(0.0, 8.35, 0.15);
    let r = (b - crown).length();
    let thin = smoothstep(0.55, 1.0, rec) * (1.0 - smoothstep(0.15 + 0.35 * rec, 0.35 + 0.4 * rec, r));
    d * (1.0 - thin)
}
