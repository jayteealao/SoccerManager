// Portrait shader for the regen face PoC.
// Kinds: 0 skin, 1 eye, 2 hair card, 3 coil shell, 4 twisted tube.

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
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) wpos: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) aux: vec4<f32>,
    @location(4) vdepth: f32,
    @location(5) aux2: vec4<f32>,
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
    o.vdepth = -(G.view * vec4<f32>(v.pos, 1.0)).z;
    return o;
}

struct FOut {
    @location(0) colour: vec4<f32>,
    @location(1) depth: vec4<f32>,
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
    let iris_r = 0.905;
    var albedo = vec3<f32>(0.78, 0.74, 0.70);
    let iris = smoothstep(iris_r - 0.01, iris_r + 0.01, c);
    let radial = fbm(d * 60.0);
    let iris_col = P.colour2.rgb * (0.6 + 0.8 * radial) * mix(0.55, 1.0, smoothstep(iris_r, iris_r + 0.04, c));
    albedo = mix(albedo, iris_col, iris);
    albedo = mix(albedo, vec3<f32>(0.01), smoothstep(0.978, 0.985, c));
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
    if (P.kind == 1u) {
        c = shade_eye(i);
    } else if (P.kind == 4u) {
        c = shade_tube(i).colour;
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
    } else {
        h = shade_card(i);
    }
    // Alpha-to-coverage turns alpha into MSAA coverage; sharpen it a little.
    let a = clamp((h.alpha - P.p0.x) / max(fwidth(h.alpha), 1e-3) + 0.5, 0.0, 1.0);
    if (a <= 0.0) { discard; }
    var o: FOut;
    o.colour = vec4<f32>(tonemap(h.colour), a);
    o.depth = vec4<f32>(depth_out(i.vdepth).rgb, a);
    return o;
}
