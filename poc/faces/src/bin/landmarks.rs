//! Renders the base head with landmark markers, to check the picks.
use regen_faces_poc::{face::FaceBuilder, img::*, measure::Landmarks, render::*, age::AgeState, genome::Genome, pools::nation};
use glam::Vec3;
fn main() {
    let r = Renderer::new();
    let mut fb = FaceBuilder::new().unwrap();
    let lm = Landmarks::find(&mut fb.base);
    let g = Genome::generate(6, nation("NHV").unwrap(), 2004);
    let age = AgeState::new(&g, 24.0);
    let w = fb.base.weights_for(&g, &age);
    let head = fb.base.build(&w);
    let (mut draws, _) = fb.draws(&g, &age, &head);
    let mut vs = Vec::new();
    let mut is = Vec::new();
    for gi in lm.all() {
        let c = head.positions[head.remap[&gi] as usize];
        let base = vs.len() as u32;
        for d in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z] {
            vs.push(Vertex { pos: (c + d * 0.04).to_array(), nrm: d.to_array(), ..Default::default() });
        }
        for t in [[0, 2, 4], [2, 1, 4], [1, 3, 4], [3, 0, 4], [2, 0, 5], [1, 2, 5], [3, 1, 5], [0, 3, 5]] {
            is.extend(t.iter().map(|k| base + k));
        }
    }
    draws.push(Draw { vertices: vs, indices: is, params: DrawParams { kind: kind::EYE, colour2: [0.0, 1.0, 0.0, 1.0], p1: [0.0, 100.0, 0.0, 0.0], ..Default::default() }, texture: None });
    let a = Rgba::from_raw(SIZE, SIZE, r.render(&draws, &Camera::portrait(0.0)).colour);
    let b = Rgba::from_raw(SIZE, SIZE, r.render(&draws, &Camera::portrait(70.0)).colour);
    contact_sheet(&[vec![a, b]], 512, &[], &[]).save_png("out/tmp/landmarks.png".as_ref());
}
