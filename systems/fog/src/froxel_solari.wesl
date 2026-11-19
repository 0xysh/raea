// The froxel grid's light from Solari (docs/raytracing.feature RT19, froxel/solari.rs): at each cell of the atlas, the
// light Solari's own lights scatter toward the eye, found as Solari finds a surface's: its emissive meshes (CANDIDATES
// drawn, one kept by its share of the cell's light, resampled importance sampling, and one shadow ray to it through
// Solari's scene), and the sky through whatever opens onto it (SKY rays in directions drawn from the medium's phase,
// so the phase cancels). Bevy's lights (the sun's shafts through its shadow maps, the point and spot lights) stay the
// light pass's; Solari's directional lights are left out here, as they'd count the sun twice.
// `solari`: each frame's estimate of the light arriving (phase-weighted, before the medium's scattering) goes into the
// cell's history, found where the cell's centre was in the last frame's view, an average of its last `frames` frames.
// `filter`: the history blurred over the cell's 3 x 3 x 3 neighbours (a tent), then scattered by the medium at the
// cell's centre, which is what the integrate pass adds. The light is blurred, not the light the medium scatters, so a
// dense edge (the steam's) stays sharp while its light settles.

import bevy_render::view::View;
import bevy_render::maths::{PI_2, orthonormalize};
import bevy_pbr::render::utils::{rand_f, rand_vec2f};
import bevy_solari::scene::bindings::{trace_ray, light_sources, RAY_T_MIN, RAY_T_MAX};
import bevy_solari::scene::sampling::{generate_random_light_sample, trace_visibility, sample_environment_map_light, ResolvedLightSample};

enable wgpu_ray_query;

// the grid's (views.rs Params: what this pass reads of it, a prefix of the uniform)
struct Params {
    view_from_clip: mat4x4<f32>,
    grid: vec4<u32>,
    slab: vec4<f32>,
    viewport: vec4<f32>,
    ambient: vec4<f32>,
    counts: vec4<u32>,
    heroes: vec4<u32>,
}

struct Volume {
    local_from_world: mat4x4<f32>,
    scatter: vec4<f32>,
    ambient: vec4<f32>,
    shape: vec4<f32>,
}

// froxel/solari.rs SolariFogParams
struct SolariFog {
    // the last frame's view (unjittered) and its grid's slab, for the history
    prev_clip_from_world: mat4x4<f32>,
    prev_slab: vec4<f32>,
    // x: the frame's number; y: 1 to start the history again; z: candidates a cell; w: frames the history averages
    frame: vec4<u32>,
    // x: what lights it (1 the emissive meshes, 2 the sky); y: sky rays a cell; z: 1 to filter
    more: vec4<u32>,
}

@group(1) @binding(0) var<uniform> params: Params;
@group(1) @binding(1) var<storage, read> volumes: array<Volume>;
@group(1) @binding(2) var<uniform> view: View;
@group(1) @binding(3) var<uniform> fog: SolariFog;
// solari: the last frame's history; filter: this frame's
@group(1) @binding(4) var history: texture_2d<f32>;
// solari: this frame's history; filter: what the integrate pass adds
@group(1) @binding(5) var out: texture_storage_2d<rgba16float, write>;

const FRAC_4_PI: f32 = 0.07957747154594767;
const LUMA = vec3(0.2126, 0.7152, 0.0722);

fn henyey_greenstein(g: f32, cos_theta: f32) -> f32 {
    let denom = 1.0 + g * g - 2.0 * g * cos_theta;
    return FRAC_4_PI * (1.0 - g * g) / (denom * sqrt(denom));
}

// a direction toward the light drawn from the phase about the view's direction V (pdf: the phase itself)
fn sample_phase(g: f32, V: vec3<f32>, rng: ptr<function, u32>) -> vec3<f32> {
    let u = rand_vec2f(rng);
    var cos_theta = 1.0 - 2.0 * u.x;
    if (abs(g) > 1e-3) {
        let s = (1.0 - g * g) / (1.0 - g + 2.0 * g * u.x);
        cos_theta = clamp((1.0 + g * g - s * s) / (2.0 * g), -1.0, 1.0);
    }
    let sin_theta = sqrt(max(0.0, 1.0 - cos_theta * cos_theta));
    let phi = u.y * PI_2;
    return orthonormalize(V) * vec3(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);
}

// what a light sample sends to point P toward the eye, before its pdf: emitted radiance x its cosine / distance^2 x
// the phase
fn toward_eye(s: ResolvedLightSample, P: vec3<f32>, V: vec3<f32>, g: f32) -> vec3<f32> {
    let ray = s.world_position.xyz - P;
    let d2 = max(dot(ray, ray), 1e-6);
    let wi = ray * inverseSqrt(d2);
    let cos_light = saturate(dot(-wi, s.world_normal));
    return s.radiance * (cos_light / d2) * henyey_greenstein(g, dot(wi, V));
}

// a point at view depth `depth` through a frame pixel
fn view_point(pixel: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec2((pixel.x - params.viewport.x) / params.viewport.z * 2.0 - 1.0, 1.0 - (pixel.y - params.viewport.y) / params.viewport.w * 2.0);
    let q4 = params.view_from_clip * vec4(ndc, 1.0, 1.0);
    let q = q4.xyz / q4.w;
    return q * (depth / -q.z);
}

// the world point at a place in the cell (0-1 each way: across, down, deep)
fn cell_point(cell: vec2<u32>, slice: u32, at: vec3<f32>) -> vec3<f32> {
    let tile = params.slab.x;
    let last = params.viewport.xy + params.viewport.zw - 0.5;
    let pixel = min(params.viewport.xy + (vec2<f32>(cell) + at.xy) * tile, last);
    let depth = params.slab.y * exp(params.slab.w * (f32(slice) + at.z) / f32(params.grid.z));
    return (view.world_from_view * vec4(view_point(pixel, depth), 1.0)).xyz;
}

// the medium at P: its scattering per metre (each volume's colour x scattering x density, thinned going up) in rgb,
// and its anisotropy weighted by each volume's share in a
fn medium(P: vec3<f32>) -> vec4<f32> {
    var sigma_s = vec3(0.0);
    var g = 0.0;
    var share = 0.0;
    for (var v = 0u; v < params.counts.x; v += 1u) {
        let vol = &volumes[v];
        let local = ((*vol).local_from_world * vec4(P, 1.0)).xyz;
        if (any(abs(local) > vec3(0.5))) {
            continue;
        }
        let s = (*vol).ambient.rgb * exp(-(*vol).shape.x * (local.y + 0.5) * (*vol).shape.y);
        sigma_s += s;
        g += (*vol).ambient.a * dot(s, LUMA);
        share += dot(s, LUMA);
    }
    return vec4(sigma_s, select(0.0, g / share, share > 0.0));
}

// an atlas texel's cell and slice, or slice >= the grid's depth where the texel holds none
fn texel_cell(t: vec2<u32>) -> vec3<u32> {
    let slice = (t.y / params.grid.y) * params.grid.w + t.x / params.grid.x;
    return vec3(t.x % params.grid.x, t.y % params.grid.y, slice);
}

fn cell_texel(cell: vec2<u32>, slice: u32) -> vec2<u32> {
    let cols = params.grid.w;
    return vec2(cell.x + (slice % cols) * params.grid.x, cell.y + (slice / cols) * params.grid.y);
}

@compute @workgroup_size(8, 8, 1)
fn solari(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(out);
    if (id.x >= size.x || id.y >= size.y) {
        return;
    }
    let c = texel_cell(id.xy);
    if (c.z >= params.grid.z) {
        textureStore(out, id.xy, vec4(0.0));
        return;
    }
    var rng = (id.x * 1973u + id.y * 9277u + fog.frame.x * 26699u) | 1u;

    // this frame's point: anywhere in the cell, so the history averages the whole cell
    let P = cell_point(c.xy, c.z, vec3(rand_vec2f(&rng), rand_f(&rng)));
    let V = normalize(P - view.world_position);
    let m = medium(P);
    let held = any(m.rgb > vec3(0.0));
    let g = m.a;

    var light = vec3(0.0);
    if (held) {
        // the emissive meshes: CANDIDATES drawn, one kept by its share of the light here, one shadow ray to it
        let count = select(0u, fog.frame.z, (fog.more.x & 1u) != 0u && arrayLength(&light_sources) > 0u);
        var kept: ResolvedLightSample;
        var kept_weight = 0.0;
        var w_sum = 0.0;
        for (var i = 0u; i < count; i += 1u) {
            let s = generate_random_light_sample(&rng).resolved_light_sample;
            // a directional light (w 0) is the light pass's
            if (s.world_position.w == 0.0) {
                continue;
            }
            let p_hat = dot(toward_eye(s, P, V, g), LUMA);
            let w = p_hat * s.inverse_pdf;
            w_sum += w;
            if (w > 0.0 && rand_f(&rng) * w_sum < w) {
                kept = s;
                kept_weight = p_hat;
            }
        }
        if (kept_weight > 0.0) {
            light += toward_eye(kept, P, V, g) * (w_sum / (f32(count) * kept_weight)) * trace_visibility(P, kept.world_position);
        }
        // the sky, through what opens onto it: directions drawn from the phase, so the phase cancels
        let rays = select(0u, fog.more.y, (fog.more.x & 2u) != 0u);
        var sky = vec3(0.0);
        for (var i = 0u; i < rays; i += 1u) {
            let L = sample_phase(g, V, &rng);
            let hit = trace_ray(P, L, RAY_T_MIN, RAY_T_MAX, RAY_FLAG_TERMINATE_ON_FIRST_HIT);
            if (hit.kind == RAY_QUERY_INTERSECTION_NONE) {
                sky += sample_environment_map_light(L);
            }
        }
        light += sky / f32(max(rays, 1u));
    }
    let now = light * view.exposure;

    // the history: where the cell's centre was in the last frame's view
    var n = 1.0;
    var result = now;
    if (fog.frame.y == 0u) {
        let C = cell_point(c.xy, c.z, vec3(0.5));
        let was = fog.prev_clip_from_world * vec4(C, 1.0);
        if (was.w > 0.0) {
            let ndc = was.xy / was.w;
            let px = vec2((ndc.x + 1.0) * 0.5 * params.viewport.z, (1.0 - ndc.y) * 0.5 * params.viewport.w);
            let pc = vec2<i32>(floor(px / params.slab.x));
            let ps = i32(floor(f32(params.grid.z) * log(was.w / fog.prev_slab.y) / fog.prev_slab.w));
            if (all(pc >= vec2(0)) && pc.x < i32(params.grid.x) && pc.y < i32(params.grid.y) && ps >= 0 && ps < i32(params.grid.z)) {
                let h = textureLoad(history, cell_texel(vec2<u32>(pc), u32(ps)), 0);
                n = min(h.a + 1.0, f32(fog.frame.w));
                result = mix(h.rgb, now, 1.0 / n);
            }
        }
    }
    // a cell outside every volume keeps no history (and the filter leaves it out)
    textureStore(out, id.xy, select(vec4(0.0), vec4(result, n), held));
}

@compute @workgroup_size(8, 8, 1)
fn filter_light(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(out);
    if (id.x >= size.x || id.y >= size.y) {
        return;
    }
    let c = texel_cell(id.xy);
    if (c.z >= params.grid.z) {
        textureStore(out, id.xy, vec4(0.0));
        return;
    }
    let sigma_s = medium(cell_point(c.xy, c.z, vec3(0.5))).rgb;
    if (all(sigma_s <= vec3(0.0))) {
        textureStore(out, id.xy, vec4(0.0));
        return;
    }
    var sum = vec3(0.0);
    var weight = 0.0;
    let reach = select(0, 1, fog.more.z != 0u);
    for (var dz = -reach; dz <= reach; dz += 1) {
        let s = i32(c.z) + dz;
        if (s < 0 || s >= i32(params.grid.z)) {
            continue;
        }
        for (var dy = -reach; dy <= reach; dy += 1) {
            let y = i32(c.y) + dy;
            if (y < 0 || y >= i32(params.grid.y)) {
                continue;
            }
            for (var dx = -reach; dx <= reach; dx += 1) {
                let x = i32(c.x) + dx;
                if (x < 0 || x >= i32(params.grid.x)) {
                    continue;
                }
                let h = textureLoad(history, cell_texel(vec2(u32(x), u32(y)), u32(s)), 0);
                // a tent (1 2 1 each way), and only cells with light found (a > 0)
                let w = f32((2 - abs(dx)) * (2 - abs(dy)) * (2 - abs(dz))) * select(0.0, 1.0, h.a > 0.0);
                sum += h.rgb * w;
                weight += w;
            }
        }
    }
    textureStore(out, id.xy, vec4(sigma_s * sum / max(weight, 1e-6), 1.0));
}
