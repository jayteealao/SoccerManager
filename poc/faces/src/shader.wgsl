// Portrait shader for the regen face PoC.
// Kinds: 0 skin, 1 eye, 2 hair card, 3 coil shell, 4 twisted tube;
// PoC 8: 5 skin (chromophores, pre-integrated), 6 eye interior, 7 cornea,
// 8 eye occlusion, 9 Marschner hair card.

struct Globals {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    cam_pos: vec4<f32>,
    depth_range: vec4<f32>,
};

struct Params {
    kind: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
    colour: vec4<f32>,
    colour2: vec4<f32>,
    p0: vec4<f32>,
    p1: vec4<f32>,
};

@group(0) @binding(0) var<uniform> G: Globals;
@group(0) @binding(1) var<uniform> P: Params;
@group(0) @binding(2) var tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) aux: vec4<f32>,
    @location(4) aux2: vec4<f32>,
    @location(5) aux3: vec4<f32>,
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) wpos: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) aux: vec4<f32>,
    @location(4) vdepth: f32,
    @location(5) aux2: vec4<f32>,
    @location(6) aux3: vec4<f32>,
};

@vertex
fn vs_main(v: VIn) -> VOut {
    var o: VOut;
    o.clip = G.view_proj * vec4<f32>(v.pos, 1.0);
    o.wpos = v.pos;
    o.nrm = v.nrm;
    o.uv = v.uv;
    o.aux = v.aux;
    o.aux2 = v.aux2;
    o.aux3 = v.aux3;
    o.vdepth = -(G.view * vec4<f32>(v.pos, 1.0)).z;
    return o;
}

struct FOut {
    @location(0) colour: vec4<f32>,
    @location(1) depth: vec4<f32>,
    // Region mask: R = finishable skin, G = eyes, B = hair/brows/lashes.
    @location(2) mask: vec4<f32>,
};

// ---------------------------------------------------------------- noise ---

fn hash3(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.3183099 + vec3<f32>(0.1, 0.17, 0.13));
    q = q * 17.0;
    return fract(q.x * q.y * q.z * (q.x + q.y + q.z));
}

fn hash33(p: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(hash3(p), hash3(p + vec3<f32>(19.1, 7.3, 3.7)), hash3(p + vec3<f32>(5.9, 31.7, 11.3)));
}

fn vnoise(x: vec3<f32>) -> f32 {
    let i = floor(x);
    let f = fract(x);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(mix(hash3(i + vec3<f32>(0.0, 0.0, 0.0)), hash3(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), hash3(i + vec3<f32>(1.0, 1.0, 0.0)), u.x), u.y),
        mix(mix(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), hash3(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), hash3(i + vec3<f32>(1.0, 1.0, 1.0)), u.x), u.y),
        u.z);
}

fn fbm(x: vec3<f32>) -> f32 {
    var s = 0.0;
    var a = 0.5;
    var p = x;
    for (var i = 0; i < 4; i++) {
        s += a * vnoise(p);
        p = p * 2.03 + vec3<f32>(1.7, 9.2, 3.1);
        a *= 0.5;
    }
    return s;
}

// Bump mapping without tangents (Mikkelsen 2010).
fn perturb(n: vec3<f32>, p: vec3<f32>, h: f32) -> vec3<f32> {
    let dpx = dpdx(p);
    let dpy = dpdy(p);
    let r1 = cross(dpy, n);
    let r2 = cross(n, dpx);
    let det = dot(dpx, r1);
    let grad = sign(det) * (dpdx(h) * r1 + dpdy(h) * r2);
    return normalize(abs(det) * n - grad);
}

// ------------------------------------------------------------- lighting ---

const KEY: vec3<f32> = vec3<f32>(-0.45, 0.55, 0.70);
const FILL: vec3<f32> = vec3<f32>(0.75, 0.10, 0.55);
const RIM: vec3<f32> = vec3<f32>(0.35, 0.45, -0.85);

fn tonemap(c0: vec3<f32>) -> vec3<f32> {
    // ACES fit (Narkowicz 2015), with a little desaturation first.
    let c = mix(vec3<f32>(dot(c0, vec3<f32>(0.2126, 0.7152, 0.0722))), c0, 0.88);
    let x = c * 0.8;
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn depth_out(vd: f32) -> vec4<f32> {
    let d = clamp(1.0 - (vd - G.depth_range.x) / (G.depth_range.y - G.depth_range.x), 0.0, 1.0);
    return vec4<f32>(d, d, d, 1.0);
}

fn skin_light(n: vec3<f32>, v: vec3<f32>, albedo: vec3<f32>, ao: f32, gloss: f32) -> vec3<f32> {
    let lights = array<vec3<f32>, 3>(normalize(KEY), normalize(FILL), normalize(RIM));
    let power = array<f32, 3>(1.9, 0.45, 0.8);
    var c = vec3<f32>(0.0);
    for (var i = 0; i < 3; i++) {
        let l = lights[i];
        let ndl = dot(n, l);
        // Wrapped diffuse with a warm terminator approximates subsurface scatter.
        let wrap = clamp((ndl + 0.35) / 1.35, 0.0, 1.0);
        let sss = vec3<f32>(0.9, 0.35, 0.25) * clamp(0.3 - abs(ndl - 0.05), 0.0, 1.0) * 0.35;
        let h = normalize(l + v);
        let spec = pow(max(dot(n, h), 0.0), 16.0 + 40.0 * gloss) * (0.03 + 0.08 * gloss) * step(0.0, ndl);
        c += power[i] * (albedo * (wrap * wrap + sss) + vec3<f32>(spec));
    }
    let sky = mix(vec3<f32>(0.10, 0.09, 0.08), vec3<f32>(0.20, 0.22, 0.25), n.y * 0.5 + 0.5);
    c += albedo * sky * 1.2;
    let fres = pow(1.0 - max(dot(n, v), 0.0), 5.0) * 0.12;
    return c * ao + vec3<f32>(fres) * ao;
}

fn kajiya(t: vec3<f32>, n: vec3<f32>, v: vec3<f32>, base: vec3<f32>, ao: f32) -> vec3<f32> {
    let spec_k = P.p0.y;
    let lights = array<vec3<f32>, 3>(normalize(KEY), normalize(FILL), normalize(RIM));
    let power = array<f32, 3>(1.7, 0.5, 1.0);
    var c = vec3<f32>(0.0);
    for (var i = 0; i < 3; i++) {
        let l = lights[i];
        let tl = dot(t, l);
        let diff = sqrt(max(1.0 - tl * tl, 0.0)) * clamp(dot(n, l) * 0.5 + 0.5, 0.0, 1.0);
        let h = normalize(l + v);
        let t1 = normalize(t + n * 0.1);
        let t2 = normalize(t - n * 0.15);
        let s1 = pow(sqrt(max(1.0 - pow(dot(t1, h), 2.0), 0.0)), 80.0) * 0.09;
        let s2 = pow(sqrt(max(1.0 - pow(dot(t2, h), 2.0), 0.0)), 20.0) * 0.2;
        c += power[i] * (base * diff * 0.8 + (vec3<f32>(s1) + base * s2) * spec_k);
    }
    c += base * 0.25;
    return c * ao;
}

// ----------------------------------------------------------------- skin ---

fn shade_skin(i: VOut) -> vec3<f32> {
    let v = normalize(G.cam_pos.xyz - i.wpos);
    var n = normalize(i.nrm);
    let p = i.wpos;
    let age = P.p0.x;
    let ao = i.uv.x;
    let forehead = i.uv.y;
    let brow = i.aux.x;
    let lip = i.aux.y;
    let beard = i.aux.z;
    let scalp = i.aux.w;

    // Height field: pores, mottling, forehead lines, crow's feet.
    var h = (vnoise(p * 180.0) - 0.5) * 0.0010 + (fbm(p * 40.0) - 0.5) * 0.0012;
    let lines = 1.0 - abs(sin(p.y * 38.0 + (fbm(p * 5.0) - 0.5) * 3.0));
    h -= forehead * age * age * pow(lines, 3.0) * 0.006;
    let crow = i.aux2.y;
    let rays = 1.0 - abs(sin(atan2(p.y - 7.28, abs(p.x) - 0.3) * 14.0 + fbm(p * 9.0) * 2.0));
    h -= crow * age * age * pow(rays, 3.0) * 0.004;
    let cloth = i.aux2.x;
    h = mix(h, (vnoise(p * vec3<f32>(220.0, 60.0, 220.0)) - 0.5) * 0.002, cloth);
    n = perturb(n, p, h);

    var albedo = P.colour.rgb;
    let mottle = fbm(p * 12.0);
    albedo *= 0.92 + 0.16 * mottle;
    // Slightly redder cheeks and nose, sallower with age.
    albedo = mix(albedo, albedo * vec3<f32>(1.04, 0.95, 0.94), 0.3);
    albedo = mix(albedo, albedo * vec3<f32>(0.97, 0.96, 0.9), age * 0.5);
    // Lips.
    albedo = mix(albedo, albedo * vec3<f32>(0.78, 0.55, 0.57), lip * P.p0.w);
    // Stubble: hair colour shows through as a fine speckled shadow.
    let speck = smoothstep(0.35, 0.8, vnoise(p * 260.0)) * (0.6 + 0.4 * fbm(p * 20.0));
    albedo = mix(albedo, P.colour2.rgb, beard * P.p0.y * (0.35 + 0.5 * speck));
    // Shadow of short hair roots on the scalp.
    albedo = mix(albedo, P.colour2.rgb * 0.8, scalp * P.p1.x);
    // Eyebrows: streaks along the brow.
    let streak = smoothstep(0.3, 0.75, vnoise(vec3<f32>(p.x * 25.0, p.y * 160.0, p.z * 25.0)));
    albedo = mix(albedo, P.colour2.rgb, clamp(brow * P.p0.z * (0.8 + 0.2 * streak), 0.0, 0.95));

    // Lash line and lid shadow keep the eye opening readable.
    albedo = mix(albedo, vec3<f32>(0.02, 0.015, 0.012), i.aux2.z * 0.85);
    let gloss = 0.25 + 0.2 * forehead + 0.3 * lip - 0.2 * beard * P.p0.y;
    if (cloth > 0.5) {
        // Plain crew-neck training top.
        let fabric = vec3<f32>(0.035, 0.05, 0.09) * (0.85 + 0.3 * vnoise(p * 90.0));
        return skin_light(n, v, fabric, ao, 0.0) * 0.8;
    }
    return skin_light(n, v, albedo, ao, gloss);
}

fn shade_eye(i: VOut) -> vec3<f32> {
    let v = normalize(G.cam_pos.xyz - i.wpos);
    let n = normalize(i.nrm);
    let d = normalize(i.wpos - P.p1.xyz);
    let fwd = normalize(vec3<f32>(sign(P.p1.x) * 0.08, -0.03, 1.0));
    let c = dot(d, fwd);
    // p1.w: iris cosine threshold (0 = default); sets iris size on the eyeball.
    let iris_r = select(0.875, P.p1.w, P.p1.w > 0.0);
    var albedo = vec3<f32>(0.78, 0.74, 0.70);
    let iris = smoothstep(iris_r - 0.01, iris_r + 0.01, c);
    let radial = fbm(d * 60.0);
    let iris_col = P.colour2.rgb * (0.6 + 0.8 * radial) * mix(0.55, 1.0, smoothstep(iris_r, iris_r + 0.04, c));
    albedo = mix(albedo, iris_col, iris);
    let pupil = mix(iris_r, 1.0, 0.72);
    albedo = mix(albedo, vec3<f32>(0.01), smoothstep(pupil - 0.004, pupil + 0.004, c));
    // Wet specular.
    let h = normalize(normalize(KEY) + v);
    let spec = pow(max(dot(n, h), 0.0), 300.0) * 3.0;
    let diff = clamp(dot(n, normalize(KEY)) * 0.5 + 0.5, 0.0, 1.0);
    return albedo * (0.25 + 1.6 * diff) + vec3<f32>(spec);
}

// ----------------------------------------------------------------- hair ---

fn hair_base(strand: f32, along: f32) -> vec3<f32> {
    // Salt and pepper: a strand is grey if its random id is under the grey fraction.
    let grey = P.p0.w;
    var c = mix(P.colour.rgb, P.colour2.rgb, along);
    let is_grey = select(0.0, 1.0, strand < grey);
    let grey_col = mix(vec3<f32>(0.26, 0.25, 0.24), vec3<f32>(0.46, 0.45, 0.43), fract(strand * 13.7));
    return mix(c, grey_col, is_grey);
}

struct Hit {
    alpha: f32,
    colour: vec3<f32>,
};

fn shade_card(i: VOut) -> Hit {
    let t = textureSample(tex, samp, i.uv);
    let v = normalize(G.cam_pos.xyz - i.wpos);
    let tang = normalize(i.aux.xyz);
    var n = normalize(i.nrm);
    if (dot(n, v) < 0.0) { n = -n; }
    let base = hair_base(t.r, i.uv.y) * (0.75 + 0.5 * t.g);
    let ao = mix(0.35, 1.0, smoothstep(0.0, 0.7, i.uv.y));
    var h: Hit;
    h.alpha = t.a;
    h.colour = kajiya(tang, n, v, base, ao * mix(0.5, 1.0, t.a));
    return h;
}

// Coiled hair as stacked shells. Each hair is a helix around a jittered
// root; a shell at height h draws the ring of every helix at that height.
fn shade_shell(i: VOut) -> Hit {
    let root = i.aux.xyz;
    let hgt = i.aux.w;
    let max_len = i.uv.x;
    let n = normalize(i.nrm);
    let cell = P.p1.y;
    let radius = P.p0.w * 0.0 + P.p1.w;
    let freq = P.p1.x;
    let thick = P.p1.z;
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(n.y) > 0.9) { up = vec3<f32>(1.0, 0.0, 0.0); }
    let t1 = normalize(cross(n, up));
    let t2 = cross(n, t1);

    // Coils clump into small naps: vary length and shade per clump.
    let clump = vnoise(root / (cell * 3.2));
    let q = root / cell;
    let base_cell = floor(q);
    var best = 1e9;
    var best_id = 0.0;
    var best_dir = vec3<f32>(0.0);
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                let c = base_cell + vec3<f32>(f32(x), f32(y), f32(z));
                let rnd = hash33(c);
                let len = max_len * (0.55 + 0.35 * rnd.z + 0.4 * clump);
                if (hgt > len) { continue; }
                let f = (c + rnd) * cell;
                let phase = rnd.x * 6.2831 + hgt * freq;
                let r = radius * (0.7 + 0.6 * rnd.y);
                // Coils lean with the growth direction.
                let centre = f + (t1 * cos(phase) + t2 * sin(phase)) * r + t2 * hgt * 0.25;
                var d = root - centre;
                d -= n * dot(d, n);
                let dist = length(d);
                if (dist < best) {
                    best = dist;
                    best_id = rnd.x;
                    best_dir = d / max(dist, 1e-5);
                }
            }
        }
    }
    var h: Hit;
    h.alpha = smoothstep(thick, thick * 0.55, best) * i.uv.y;
    let v = normalize(G.cam_pos.xyz - i.wpos);
    // The coil's tangent runs around the helix.
    let tang = normalize(cross(n, best_dir) + n * 0.3);
    let along = clamp(hgt / max(max_len, 1e-4), 0.0, 1.0);
    let ao = mix(0.3, 1.0, pow(along, 0.7));
    let nn = normalize(n + best_dir * 0.6);
    h.colour = kajiya(tang, nn, v, hair_base(best_id, along) * (0.8 + 0.4 * clump), ao);
    return h;
}

fn shade_tube(i: VOut) -> Hit {
    let v = normalize(G.cam_pos.xyz - i.wpos);
    let along = i.uv.y;
    let tang = normalize(i.aux.xyz);
    // Two-strand twist: helical grooves around the tube.
    let s = fract(i.uv.x * 2.0 + along * P.p1.x);
    let lengthwise = vnoise(vec3<f32>(i.uv.x * 8.0, along * 60.0, fract(i.aux.w) * 50.0));
    let groove = smoothstep(0.0, 0.18, s) * smoothstep(1.0, 0.82, s);
    let fuzz = vnoise(i.wpos * 400.0);
    var n = perturb(normalize(i.nrm), i.wpos, groove * 0.004 + fuzz * 0.0015);
    if (dot(n, v) < 0.0) { n = -n; }
    let helix_t = normalize(tang + cross(normalize(i.nrm), tang) * 0.9);
    let ao = mix(0.35, 1.0, groove) * mix(0.5, 1.0, smoothstep(0.0, 0.3, along)) * (0.8 + 0.3 * lengthwise);
    var h: Hit;
    h.alpha = 1.0;
    h.colour = kajiya(helix_t, n, v, hair_base(fract(i.aux.w), along) * (0.8 + 0.4 * fuzz), ao);
    return h;
}

// -------------------------------------------------------------- entries ---

@fragment
fn fs_opaque(i: VOut) -> FOut {
    var o: FOut;
    var c: vec3<f32>;
    o.mask = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    if (P.kind == 1u) {
        c = shade_eye(i);
        o.mask = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    } else if (P.kind == 4u) {
        c = shade_tube(i).colour;
        o.mask = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    } else if (P.kind == 5u) {
        let r = shade_skin2(i);
        c = r.colour;
        o.mask = r.mask;
    } else if (P.kind == 6u) {
        c = shade_eye2(i);
        o.mask = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    } else {
        c = shade_skin(i);
    }
    o.colour = vec4<f32>(tonemap(c), 1.0);
    o.depth = depth_out(i.vdepth);
    return o;
}

@fragment
fn fs_hair(i: VOut) -> FOut {
    var h: Hit;
    if (P.kind == 3u) {
        h = shade_shell(i);
    } else if (P.kind == 9u) {
        h = shade_card2(i);
    } else {
        h = shade_card(i);
    }
    // Alpha-to-coverage turns alpha into MSAA coverage; sharpen it a little.
    let a = clamp((h.alpha - P.p0.x) / max(fwidth(h.alpha), 1e-3) + 0.5, 0.0, 1.0);
    if (a <= 0.0) { discard; }
    var o: FOut;
    o.colour = vec4<f32>(tonemap(h.colour), a);
    o.depth = vec4<f32>(depth_out(i.vdepth).rgb, a);
    o.mask = vec4<f32>(0.0, 0.0, 1.0, a);
    return o;
}

// =================================================================== PoC 8 ===

// Two-chromophore skin albedo; must match genes::chromophores.
fn chromophores(cm: f32, ch: f32) -> vec3<f32> {
    let f = exp(-cm / 3.456);
    let eu = vec3<f32>(1.0, 0.756, 1.0);
    let ph = vec3<f32>(0.0, 0.656, 1.0);
    let hb = vec3<f32>(0.482, 1.0, 0.0);
    return vec3<f32>(0.936, 1.0, 0.822) * exp(-(cm * ((1.0 - f) * eu + f * ph) + ch * hb));
}

fn ggx_spec(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, rough: f32) -> f32 {
    let h = normalize(l + v);
    let a = rough * rough;
    let a2 = a * a;
    let nh = max(dot(n, h), 0.0);
    let nl = max(dot(n, l), 0.0);
    let nv = max(dot(n, v), 1e-3);
    let dd = nh * nh * (a2 - 1.0) + 1.0;
    let D = a2 / (3.14159 * dd * dd);
    let vis = 0.5 / (nl * sqrt(nv * nv * (1.0 - a2) + a2) + nv * sqrt(nl * nl * (1.0 - a2) + a2) + 1e-5);
    let F = 0.028 + 0.972 * pow(1.0 - max(dot(v, h), 0.0), 5.0);
    return D * vis * F * nl;
}

struct SkinOut {
    colour: vec3<f32>,
    mask: vec4<f32>,
};

// Skin v2 (roadmap 5): chromophore albedo with regional blood and oil,
// pre-integrated subsurface diffuse (Penner 2011; LUT in the draw texture,
// indexed by N.L and curvature), diffuse from the smooth normal and
// specular from the bumped normal, dual-lobe GGX, cavity occlusion.
fn shade_skin2(i: VOut) -> SkinOut {
    let v = normalize(G.cam_pos.xyz - i.wpos);
    let n0 = normalize(i.nrm);
    let p = i.wpos;
    let age = P.p0.x;
    let ao = i.uv.x;
    let forehead = i.uv.y;
    let brow = i.aux.x;
    let lip = i.aux.y;
    let beard = i.aux.z;
    let scalp = i.aux.w;
    let cloth = i.aux2.x;
    let crow = i.aux2.y;
    let lashline = i.aux2.z;
    let curv = i.aux2.w;
    let hb = i.aux3.x;
    let oil = i.aux3.y;
    let wet = i.aux3.z;
    let cavity = i.aux3.w;

    var o: SkinOut;
    if (cloth > 0.5) {
        let fabric = vec3<f32>(0.035, 0.05, 0.09) * (0.85 + 0.3 * vnoise(p * 90.0));
        o.colour = skin_light(n0, v, fabric, ao, 0.0) * 0.8;
        o.mask = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        return o;
    }

    // Micro-relief: pores (sharper on the nose and cheeks), fine mottling,
    // forehead lines and crow's feet with age.
    let pore = vnoise(p * 220.0);
    var h = (pore - 0.5) * (0.0006 + 0.0006 * oil) + (fbm(p * 45.0) - 0.5) * 0.0008;
    let lines = 1.0 - abs(sin(p.y * 38.0 + (fbm(p * 5.0) - 0.5) * 3.0));
    h -= forehead * age * age * pow(lines, 3.0) * 0.006;
    let rays = 1.0 - abs(sin(atan2(p.y - 7.28, abs(p.x) - 0.3) * 14.0 + fbm(p * 9.0) * 2.0));
    h -= crow * age * age * pow(rays, 3.0) * 0.004;
    let n = perturb(n0, p, h);

    // Albedo from chromophores: melanin with low-frequency mottling; blood
    // raised on cheeks, nose, ears and lips; age adds sallowness.
    let mottle = fbm(p * 10.0) - 0.5;
    let cm = P.colour.x * (1.0 + 0.10 * mottle + 0.06 * scalp) ;
    let ch = P.colour.y + 0.12 * hb + 0.45 * lip * P.p0.w + 0.05 * mottle - 0.08 * age;
    var albedo = chromophores(cm, max(ch, 0.0));
    albedo *= mix(vec3<f32>(1.0), vec3<f32>(0.99, 0.97, 0.9), age * 0.6);
    // Stubble, scalp roots and brows, as in skin v1.
    let speck = smoothstep(0.35, 0.8, vnoise(p * 260.0)) * (0.6 + 0.4 * fbm(p * 20.0));
    albedo = mix(albedo, P.colour2.rgb, beard * P.p0.y * (0.35 + 0.5 * speck));
    albedo = mix(albedo, P.colour2.rgb * 0.8, scalp * P.p1.x);
    // Brow hairs as fine strokes: medial hairs point up, lateral ones
    // sweep outwards; stroke spacing ~0.25 mm with jittered presence.
    let bu = clamp((abs(p.x) - 0.06) / 0.5, 0.0, 1.0);
    let ang = mix(1.2, 0.25, bu);
    let dir = vec2<f32>(cos(ang) * sign(p.x), sin(ang));
    let q2 = vec2<f32>(p.x, p.y);
    let across = dot(q2, vec2<f32>(-dir.y, dir.x));
    let along = dot(q2, dir);
    let lane = across * 420.0 + 2.0 * vnoise(vec3<f32>(along * 30.0, across * 30.0, 1.0));
    let stroke = smoothstep(0.55, 0.9, 1.0 - abs(fract(lane) * 2.0 - 1.0));
    let present = smoothstep(0.35, 0.6, vnoise(vec3<f32>(floor(lane) * 0.37, along * 25.0, 2.0)));
    let hairs = stroke * present;
    let brow_a = clamp(brow * P.p0.z * (0.6 + 0.5 * hairs), 0.0, 0.95);
    albedo = mix(albedo, P.colour2.rgb, brow_a);
    albedo = mix(albedo, vec3<f32>(0.03, 0.02, 0.018), lashline * 0.6);

    let lights = array<vec3<f32>, 3>(normalize(KEY), normalize(FILL), normalize(RIM));
    let power = array<f32, 3>(1.9, 0.45, 0.8);
    let rough = mix(0.62, 0.48, oil) + 0.12 * beard * P.p0.y;
    let r_lo = mix(rough, 0.13, wet);
    var diff = vec3<f32>(0.0);
    var spec = 0.0;
    let cv = clamp(curv * P.p1.y, 0.0, 1.0);
    for (var k = 0; k < 3; k++) {
        let l = lights[k];
        let ndl = dot(n0, l);
        let sss = textureSampleLevel(tex, samp, vec2<f32>(ndl * 0.5 + 0.5, cv), 0.0).rgb;
        diff += power[k] * sss;
        spec += power[k] * (0.85 * ggx_spec(n, v, l, r_lo) + 0.15 * ggx_spec(n, v, l, r_lo * 0.5));
    }
    let sky = mix(vec3<f32>(0.10, 0.09, 0.08), vec3<f32>(0.20, 0.22, 0.25), n0.y * 0.5 + 0.5);
    let fres = 0.028 + 0.972 * pow(1.0 - max(dot(n, v), 0.0), 5.0);
    let env = mix(vec3<f32>(0.12, 0.12, 0.13), vec3<f32>(0.35, 0.37, 0.40), reflect(-v, n).y * 0.5 + 0.5) * fres * (1.0 - 0.6 * r_lo);
    let spec_occ = mix(1.0, cavity, 0.8) * ao;
    o.colour = albedo * (diff + sky * 1.2) * ao + (vec3<f32>(spec) * 0.6 + env * 0.7) * spec_occ;
    // The finish may change skin; brows, lash line and wet margins stay ours.
    let own = max(max(brow_a, lashline), wet);
    o.mask = vec4<f32>(1.0 - own, 0.0, brow_a, 1.0);
    return o;
}

// Eye interior (roadmap 7). p1.xyz = eyeball centre, p1.w = iris radius;
// p0.xyz = gaze axis, p0.w = cornea height over the iris; colour.x = limbal
// ring strength, colour.y = sclera ageing, colour.z = pupil fraction.
fn shade_eye2(i: VOut) -> vec3<f32> {
    let v = normalize(G.cam_pos.xyz - i.wpos);
    let fwd = normalize(P.p0.xyz);
    let e = i.wpos - P.p1.xyz;
    let rv = e - fwd * dot(e, fwd);
    let iris_r = P.p1.w;
    let rho = length(rv) / iris_r;
    var up = vec3<f32>(0.0, 1.0, 0.0);
    let ax = normalize(cross(up, fwd));
    let ay = cross(fwd, ax);
    let is_iris = i.aux.y + i.aux.z;
    var albedo: vec3<f32>;
    let n = normalize(i.nrm);
    if (is_iris > 0.5 || rho < 1.02) {
        // Refraction through the cornea: shift the iris lookup by the
        // difference between the refracted and straight rays over the depth
        // of the anterior chamber at this radius.
        let depth = P.p0.w * max(1.0 - rho * rho, 0.0) + 0.002;
        let nc = normalize(fwd + rv / iris_r * 0.55);
        let r = refract(-v, nc, 1.0 / 1.376);
        let t_r = depth / max(-dot(r, fwd), 0.2);
        let t_s = depth / max(dot(v, fwd), 0.2);
        let shift = (r - fwd * dot(r, fwd)) * t_r - (-v - fwd * dot(-v, fwd)) * t_s;
        let q = rv + shift;
        let qx = dot(q, ax) / iris_r;
        let qy = dot(q, ay) / iris_r;
        let rr = length(vec2<f32>(qx, qy));
        let ang = atan2(qy, qx);
        let fibres = fbm(vec3<f32>(ang * 9.0, rr * 3.0, 1.3)) ;
        let fine = vnoise(vec3<f32>(ang * 60.0, rr * 8.0, 7.0));
        let collarette = smoothstep(0.08, 0.0, abs(rr - 0.48 - 0.04 * fibres));
        let crypt = smoothstep(0.62, 0.8, vnoise(vec3<f32>(ang * 14.0, rr * 10.0, 3.0))) * smoothstep(0.4, 0.6, rr);
        var iris_c = P.colour2.rgb * (0.55 + 0.7 * fibres + 0.25 * fine);
        iris_c = mix(iris_c, P.colour2.rgb * 1.5 + vec3<f32>(0.06, 0.05, 0.02), collarette * 0.5);
        iris_c *= 1.0 - 0.45 * crypt;
        // Limbal ring: dark band at the iris edge, fading with age.
        iris_c *= 1.0 - P.colour.x * smoothstep(0.78, 0.97, rr);
        let pupil = smoothstep(P.colour.z + 0.015, P.colour.z - 0.015, rr);
        albedo = mix(iris_c, vec3<f32>(0.008), pupil);
        let limbus = smoothstep(0.97, 1.03, rr);
        let sclera = vec3<f32>(0.80, 0.77, 0.73);
        albedo = mix(albedo, sclera, limbus);
    } else {
        // Sclera: warm off-white, a few vessels towards the corners, sallower
        // with age.
        var sclera = vec3<f32>(0.80, 0.77, 0.73) * (1.0 - vec3<f32>(0.0, 0.05, 0.15) * P.colour.y);
        let side = smoothstep(1.3, 2.4, rho);
        let vessel = smoothstep(0.72, 0.8, vnoise(e * 900.0)) * side;
        sclera = mix(sclera, vec3<f32>(0.62, 0.22, 0.2), vessel * 0.35);
        albedo = sclera * (1.0 - 0.1 * side);
    }
    let diff = clamp(dot(n, normalize(KEY)) * 0.5 + 0.5, 0.0, 1.0);
    return albedo * (0.22 + 0.95 * diff);
}

// Cornea (blended, premultiplied): Fresnel reflection of the studio and a
// sharp key-light glint.
fn shade_cornea(i: VOut) -> vec4<f32> {
    let v = normalize(G.cam_pos.xyz - i.wpos);
    let n = normalize(i.nrm);
    let F = 0.025 + 0.975 * pow(1.0 - max(dot(n, v), 0.0), 5.0);
    let r = reflect(-v, n);
    let env = mix(vec3<f32>(0.05, 0.05, 0.06), vec3<f32>(0.55, 0.58, 0.62), smoothstep(-0.2, 0.6, r.y));
    let h = normalize(normalize(KEY) + v);
    let glint = pow(max(dot(n, h), 0.0), 900.0) * 9.0 + pow(max(dot(n, h), 0.0), 120.0) * 0.25;
    let a = clamp(F * 0.8, 0.0, 1.0);
    return vec4<f32>(env * F + vec3<f32>(glint), a);
}

// Eye-occlusion shell: soft shadow from the lids and lashes on the eyeball,
// per fragment. aux.xyz = eye-local template position; colour.xy = inner and
// outer corner x; colour2 / p1 = cubic fits of the upper / lower margins.
fn lid_y(c: vec4<f32>, t: f32) -> f32 {
    return c.x + t * (c.y + t * (c.z + t * c.w));
}

fn shade_occlusion(i: VOut) -> vec4<f32> {
    if (i.aux.z < 0.0) { return vec4<f32>(0.0); }
    let t = (i.aux.x - P.colour.x) / (P.colour.y - P.colour.x);
    let tc = clamp(t, 0.0, 1.0);
    let du = lid_y(P.colour2, tc) - i.aux.y;
    let dl = i.aux.y - lid_y(P.p1, tc);
    let dc = min(t, 1.0 - t) * abs(P.colour.y - P.colour.x);
    let sh_u = (1.0 - smoothstep(0.0, 0.035, du)) * 0.8;
    let sh_l = (1.0 - smoothstep(0.0, 0.012, dl)) * 0.45;
    let sh_c = (1.0 - smoothstep(0.0, 0.03, dc)) * 0.5;
    let a = clamp(max(max(sh_u, sh_l), sh_c) * P.p0.x, 0.0, 0.92);
    return vec4<f32>(vec3<f32>(0.012, 0.008, 0.006) * a, a);
}

// Marschner hair (roadmap 6), after Karis 2016 and Frostbite 2019: R, TT
// and TRT lobes with melanin absorption (pbrt-v3 conversion done on the CPU,
// sigma_a in p1.xyz), plus a small multiple-scatter diffuse.
fn gauss(b: f32, x: f32) -> f32 {
    return exp(-0.5 * x * x / (b * b)) / (2.5066 * b);
}

fn marschner(t: vec3<f32>, v: vec3<f32>, l: vec3<f32>, sa: vec3<f32>) -> vec3<f32> {
    let beta = 0.2;
    let shift = -0.06;
    let sin_l = clamp(dot(l, t), -1.0, 1.0);
    let sin_v = clamp(dot(v, t), -1.0, 1.0);
    let th = 0.5 * (asin(sin_l) + asin(sin_v));
    let td = 0.5 * (asin(sin_l) - asin(sin_v));
    let lp = l - t * sin_l;
    let vp = v - t * sin_v;
    let cos_phi = dot(lp, vp) * inverseSqrt(dot(lp, lp) * dot(vp, vp) + 1e-4);
    let cos_half = sqrt(clamp(0.5 + 0.5 * cos_phi, 0.0, 1.0));
    let cos_td = max(cos(td), 0.2);
    let f0 = 0.0465; // ((1.55 - 1) / (1.55 + 1))^2
    let fr = f0 + (1.0 - f0) * pow(1.0 - sqrt(clamp(0.5 + 0.5 * dot(l, v), 0.0, 1.0)), 5.0);
    let T = exp(-sa * (2.0 / cos_td));
    let r = gauss(beta, th - shift) * 0.25 * cos_half * fr;
    let tt = gauss(beta * 0.5, th + shift * 0.5) * exp(-3.65 * cos_phi - 3.98) * (1.0 - f0) * (1.0 - f0) * T;
    let trt = gauss(beta * 2.0, th + shift * 1.5) * exp(17.0 * cos_phi - 16.78) * (1.0 - f0) * (1.0 - f0) * f0 * T * T;
    return vec3<f32>(r) + tt + trt;
}

fn shade_card2(i: VOut) -> Hit {
    let t = textureSample(tex, samp, i.uv);
    let v = normalize(G.cam_pos.xyz - i.wpos);
    let tang = normalize(i.aux.xyz);
    var n = normalize(i.nrm);
    if (dot(n, v) < 0.0) { n = -n; }
    // Per-strand melanin variation; grey strands have almost none.
    let grey = select(0.0, 1.0, t.r < P.p0.w);
    var sa = P.p1.xyz * (0.75 + 0.5 * t.g);
    sa = mix(sa, vec3<f32>(0.22, 0.26, 0.32) * (0.8 + 0.4 * fract(t.r * 13.7)), grey);
    let lights = array<vec3<f32>, 3>(normalize(KEY), normalize(FILL), normalize(RIM));
    let power = array<f32, 3>(1.9, 0.5, 1.1);
    var c = vec3<f32>(0.0);
    let base = exp(-sa * 2.2);
    let ao = mix(0.35, 1.0, smoothstep(0.0, 0.7, i.uv.y));
    for (var k = 0; k < 3; k++) {
        let l = lights[k];
        c += power[k] * (marschner(tang, v, l, sa) * 2.2 + base * clamp(dot(n, l) * 0.5 + 0.5, 0.0, 1.0) * 0.28);
    }
    c += base * 0.18;
    var o: Hit;
    o.alpha = t.a;
    o.colour = c * ao;
    return o;
}

struct FOutS {
    @location(0) colour: vec4<f32>,
    @location(1) depth: vec4<f32>,
    @location(2) mask: vec4<f32>,
    @builtin(sample_mask) cov: u32,
};

// Stochastic transparency (roadmap 6): alpha picks how many of the four
// MSAA samples this fragment covers, with a pseudo-random sample set per
// fragment, layer and accumulation pass, so overlapping cards add up to
// opacity instead of sharing one dither pattern.
@fragment
fn fs_hair_stoch(i: VOut) -> FOutS {
    var h: Hit;
    var a: f32;
    if (P.kind == 3u) {
        h = shade_shell(i);
        a = h.alpha;
    } else if (P.kind == 9u) {
        h = shade_card2(i);
        a = smoothstep(P.p0.x - 0.25, P.p0.x + 0.25, h.alpha);
    } else {
        h = shade_card(i);
        a = smoothstep(P.p0.x - 0.25, P.p0.x + 0.25, h.alpha);
    }
    let rnd = hash3(floor(i.wpos * 3000.0) + vec3<f32>(G.depth_range.z * 17.0, i.clip.x * 0.37, i.clip.y * 0.53));
    let k = u32(clamp(floor(a * 4.0 + rnd), 0.0, 4.0));
    if (k == 0u) { discard; }
    let rot = u32(fract(rnd * 7.31) * 4.0);
    let base = (1u << k) - 1u;
    var o: FOutS;
    o.cov = ((base << rot) | (base >> (4u - rot))) & 15u;
    o.colour = vec4<f32>(tonemap(h.colour), 1.0);
    o.depth = depth_out(i.vdepth);
    o.mask = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    return o;
}

@fragment
fn fs_blend(i: VOut) -> FOut {
    var o: FOut;
    var c: vec4<f32>;
    if (P.kind == 7u) {
        c = shade_cornea(i);
    } else {
        c = shade_occlusion(i);
    }
    // Premultiplied: tonemap the emitted part only.
    o.colour = vec4<f32>(tonemap(c.rgb), c.a);
    o.depth = vec4<f32>(0.0);
    o.mask = vec4<f32>(0.0);
    return o;
}
