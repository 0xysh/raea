// LightBench's lighting (docs/lightbench.feature LB1, LB3): each pixel of Bevy's G-buffer (the depth prepass and the
// deferred prepass's albedo and normal) lit by point lights, every light (the reference) or K of them chosen at random,
// each through one shadow ray in LightBench's own BVH. Both use `unshadowed` and `visible`, so they differ only in
// which lights they look at. The result goes into the camera's HDR main texture, before Bevy's tonemapping.

struct Params {
    world_from_clip: mat4x4<f32>,
    // xyz: the camera, w: exposure
    eye: vec4<f32>,
    // width, height, lights, frame
    size: vec4<u32>,
    // mode (0 every light, 1 uniform, 2 weighted), K, view (0 lit, 1 normal, 2 albedo, 3 depth, 4 rays, 5 history),
    // brute force
    mode: vec4<u32>,
    // the previous frame's camera, to find where a point was
    prev_clip_from_world: mat4x4<f32>,
    // xyz: the previous camera
    prev_eye: vec4<f32>,
    // raw (no history), reset (a camera cut, or the first frame), the history's length in frames, 0
    temporal: vec4<u32>,
}

struct Node { mn: vec3<f32>, a: u32, mx: vec3<f32>, count: u32 }
struct Light { pos: vec3<f32>, range: f32, power: vec3<f32>, radius: f32 }

@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var gbuf: texture_2d<u32>;
@group(0) @binding(3) var<storage, read> nodes: array<Node>;
// three a triangle: v0, v1 - v0, v2 - v0
@group(0) @binding(4) var<storage, read> tris: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> lights: array<Light>;
@group(0) @binding(6) var out: texture_storage_2d<rgba16float, write>;
// the light so far at each pixel (rgb, before albedo) and how many frames it holds (a); the surface it was for
// (its normal, and its distance from that frame's camera); last frame's to read, this frame's to write
@group(0) @binding(7) var hist_in: texture_2d<f32>;
@group(0) @binding(8) var geo_in: texture_2d<f32>;
@group(0) @binding(9) var hist_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(10) var geo_out: texture_storage_2d<rgba16float, write>;

const PI: f32 = 3.14159265;
const MAX_K: u32 = 16u;
const STACK: u32 = 48u;

var<private> seed: u32;
var<private> rays: u32;

fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}

fn rnd() -> f32 {
    seed = pcg(seed);
    return f32(seed >> 8u) / 16777216.0;
}

// Bevy's octahedral_decode (bevy_render utils), for the deferred prepass's normal
fn oct_decode(v: vec2<f32>) -> vec3<f32> {
    let f = v * 2.0 - 1.0;
    var n = vec3(f.xy, 1.0 - abs(f.x) - abs(f.y));
    let t = saturate(-n.z);
    let w = select(vec2(t), vec2(-t), n.xy >= vec2(0.0));
    n = vec3(n.xy + w, n.z);
    return normalize(n);
}

fn hit_box(o: vec3<f32>, inv: vec3<f32>, mn: vec3<f32>, mx: vec3<f32>, tmax: f32) -> bool {
    let t0 = (mn - o) * inv;
    let t1 = (mx - o) * inv;
    let lo = min(t0, t1);
    let hi = max(t0, t1);
    let near = max(max(lo.x, lo.y), max(lo.z, 0.0));
    let far = min(min(hi.x, hi.y), min(hi.z, tmax));
    return near <= far;
}

// Möller-Trumbore, both sides
fn hit_tri(o: vec3<f32>, d: vec3<f32>, i: u32, tmax: f32) -> bool {
    let v0 = tris[3u * i].xyz;
    let e1 = tris[3u * i + 1u].xyz;
    let e2 = tris[3u * i + 2u].xyz;
    let pv = cross(d, e2);
    let det = dot(e1, pv);
    if abs(det) < 1e-12 {
        return false;
    }
    let inv = 1.0 / det;
    let tv = o - v0;
    let u = dot(tv, pv) * inv;
    if u < 0.0 || u > 1.0 {
        return false;
    }
    let qv = cross(tv, e1);
    let v = dot(d, qv) * inv;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let t = dot(e2, qv) * inv;
    return t > 0.0 && t < tmax;
}

// whether anything lies on the ray before tmax: the BVH, or every triangle when brute force is asked for
fn occluded(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> bool {
    rays += 1u;
    if p.mode.w != 0u {
        let n = arrayLength(&tris) / 3u;
        for (var i = 0u; i < n; i++) {
            if hit_tri(o, d, i, tmax) {
                return true;
            }
        }
        return false;
    }
    let inv = 1.0 / select(d, vec3(1e-12), abs(d) < vec3(1e-12));
    var stack: array<u32, STACK>;
    var sp = 0u;
    var i = 0u;
    loop {
        let n = nodes[i];
        if hit_box(o, inv, n.mn, n.mx, tmax) {
            if n.count > 0u {
                for (var t = n.a; t < n.a + n.count; t++) {
                    if hit_tri(o, d, t, tmax) {
                        return true;
                    }
                }
            } else if sp < STACK {
                stack[sp] = n.a;
                sp++;
                i++;
                continue;
            }
        }
        if sp == 0u {
            break;
        }
        sp--;
        i = stack[sp];
    }
    return false;
}

// a light's light at a point before its shadow: its power times the cosine and the falloff (an inverse square that
// stops growing inside its radius and fades to nothing at its range); all zero where it can't reach
fn unshadowed(l: Light, pos: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    let v = l.pos - pos;
    let d2 = max(dot(v, v), 1e-8);
    let d = sqrt(d2);
    let ndl = dot(n, v / d);
    if ndl <= 0.0 || d >= l.range {
        return vec3(0.0);
    }
    let x = d / l.range;
    let win = saturate(1.0 - x * x * x * x);
    return l.power * (ndl * win * win / max(d2, l.radius * l.radius));
}

fn visible(o: vec3<f32>, l: Light) -> f32 {
    let v = l.pos - o;
    let d = length(v);
    return select(1.0, 0.0, occluded(o, v / d, d - 1e-3));
}

fn lum(c: vec3<f32>) -> f32 {
    return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.size.x || id.y >= p.size.y {
        return;
    }
    let px = vec2<i32>(id.xy);
    let depth = textureLoad(depth_tex, px, 0);
    // the sky (reverse Z: far is 0) keeps Bevy's clear colour, and holds no history
    if depth <= 0.0 {
        textureStore(hist_out, px, vec4(0.0));
        textureStore(geo_out, px, vec4(0.0, 0.0, 0.0, -1.0));
        return;
    }
    let g = textureLoad(gbuf, px, 0);
    let albedo = pow(unpack4x8unorm(g.r).rgb, vec3(2.2));
    let n = oct_decode(vec2(f32(g.a & 0xFFFu), f32((g.a >> 12u) & 0xFFFu)) / 4095.0);
    let uv = (vec2<f32>(id.xy) + 0.5) / vec2<f32>(p.size.xy);
    let w = p.world_from_clip * vec4(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let pos = w.xyz / w.w;
    let dist = length(pos - p.eye.xyz);
    let view = p.mode.z;
    if view == 1u {
        textureStore(out, px, vec4(n * 0.5 + 0.5, 1.0));
        return;
    }
    if view == 2u || view == 6u {
        textureStore(out, px, vec4(albedo, 1.0));
        return;
    }
    if view == 3u {
        textureStore(out, px, vec4(vec3(dist / 20.0), 1.0));
        return;
    }
    seed = pcg(id.x + id.y * p.size.x + p.size.w * p.size.x * p.size.y);
    rays = 0u;
    // off the surface along its normal, more the farther it is (depth precision)
    let o = pos + n * (2e-3 + 5e-4 * dist);
    let nl = p.size.z;
    let k = clamp(p.mode.y, 1u, MAX_K);
    var c = vec3(0.0);
    if p.mode.x == 0u {
        for (var i = 0u; i < nl; i++) {
            let u = unshadowed(lights[i], pos, n);
            if lum(u) > 0.0 {
                c += u * visible(o, lights[i]);
            }
        }
    } else if p.mode.x == 1u {
        // uniform: each light 1 / N
        for (var s = 0u; s < k; s++) {
            let i = min(u32(rnd() * f32(nl)), nl - 1u);
            let u = unshadowed(lights[i], pos, n);
            if lum(u) > 0.0 {
                c += u * visible(o, lights[i]) * f32(nl);
            }
        }
        c /= f32(k);
    } else {
        // weighted: K reservoirs over every light by its unshadowed light, so any light that adds light can be chosen
        var pick: array<u32, MAX_K>;
        var pw: array<f32, MAX_K>;
        var wsum = 0.0;
        for (var i = 0u; i < nl; i++) {
            let wi = lum(unshadowed(lights[i], pos, n));
            if wi <= 0.0 {
                continue;
            }
            wsum += wi;
            for (var s = 0u; s < k; s++) {
                if rnd() * wsum < wi {
                    pick[s] = i;
                    pw[s] = wi;
                }
            }
        }
        if wsum > 0.0 {
            for (var s = 0u; s < k; s++) {
                let l = lights[pick[s]];
                c += unshadowed(l, pos, n) * visible(o, l) * (wsum / pw[s]);
            }
            c /= f32(k);
        }
    }
    // the history: where this point was last frame, kept only if the same surface was there
    var held = 0.0;
    var acc = c;
    if p.temporal.x == 0u && p.temporal.y == 0u {
        let was = p.prev_clip_from_world * vec4(pos, 1.0);
        let ndc = was.xy / was.w;
        let at = vec2<i32>(floor(vec2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5) * vec2<f32>(p.size.xy)));
        if was.w > 0.0 && all(at >= vec2(0)) && all(at < vec2<i32>(p.size.xy)) {
            let geo = textureLoad(geo_in, at, 0);
            let expect = length(pos - p.prev_eye.xyz);
            if geo.w > 0.0 && abs(geo.w - expect) < 0.02 * expect + 0.01 && dot(geo.xyz, n) > 0.9 {
                let h = textureLoad(hist_in, at, 0);
                held = min(h.a, f32(p.temporal.z) - 1.0);
                acc = mix(h.rgb, c, 1.0 / (held + 1.0));
            }
        }
    }
    textureStore(hist_out, px, vec4(acc, held + 1.0));
    textureStore(geo_out, px, vec4(n, dist));
    if view == 4u {
        textureStore(out, px, vec4(vec3(f32(rays) / 16.0), 1.0));
        return;
    }
    if view == 5u {
        textureStore(out, px, vec4(vec3((held + 1.0) / f32(p.temporal.z)), 1.0));
        return;
    }
    textureStore(out, px, vec4(acc * albedo / PI * p.eye.w, 1.0));
}
