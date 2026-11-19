// the split emission gathered over frames at the output's size, with no clamp (RAEA_SOLARI_EMISSIVE=taa): each frame's
// jittered samples weighted by their distance to this output pixel's centre, added to its history reprojected by the
// motion, which is dropped where the depth there changed (a disocclusion) or the camera cut
@group(0) @binding(9) var depth: texture_depth_2d;
@group(0) @binding(10) var motion: texture_2d<f32>;
@group(0) @binding(11) var hist_in: texture_2d<f32>;
@group(0) @binding(12) var hist_sampler: sampler;
@group(0) @binding(13) var hist_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(14) var hist_depth_in: texture_storage_2d<r32float, read>;
@group(0) @binding(15) var hist_depth_out: texture_storage_2d<r32float, write>;
// the most frames' weight the history holds: past it, it fades as 1 / HISTORY_CAP a frame
const HISTORY_CAP: f32 = 16.0;
// the history at `uv` through a Catmull-Rom filter in five bilinear taps (Jimenez's), so a moving view isn't blurred a
// little more each frame as a bilinear read would; never below zero (its lobes ring on a hard edge)
fn history_at(uv: vec2<f32>, size: vec2<f32>) -> vec4<f32> {
    let at = uv * size;
    let t1 = floor(at - 0.5) + 0.5;
    let f = at - t1;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let t0 = (t1 - 1.0) / size;
    let t3 = (t1 + 2.0) / size;
    let t12 = (t1 + w2 / w12) / size;
    var r = textureSampleLevel(hist_in, hist_sampler, vec2<f32>(t12.x, t0.y), 0.0) * (w12.x * w0.y);
    r += textureSampleLevel(hist_in, hist_sampler, vec2<f32>(t0.x, t12.y), 0.0) * (w0.x * w12.y);
    r += textureSampleLevel(hist_in, hist_sampler, t12, 0.0) * (w12.x * w12.y);
    r += textureSampleLevel(hist_in, hist_sampler, vec2<f32>(t3.x, t12.y), 0.0) * (w3.x * w12.y);
    r += textureSampleLevel(hist_in, hist_sampler, vec2<f32>(t12.x, t3.y), 0.0) * (w12.x * w3.y);
    let sum = w12.x * w0.y + w0.x * w12.y + w12.x * w12.y + w3.x * w12.y + w12.x * w3.y;
    return max(r / sum, vec4<f32>(0.0));
}
@compute @workgroup_size(8, 8, 1)
fn restore_taa(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = vec2<u32>(params[1].xy);
    if (id.x >= size.x || id.y >= size.y) { return; }
    let p = vec2<i32>(id.xy);
    let out_size = params[1].xy;
    let in_size = params[1].zw;
    // this output pixel in the jittered input, where the input's sample centres are whole numbers
    let q = (vec2<f32>(id.xy) + 0.5) * in_size / out_size - params[3].xy - 0.5;
    let near = vec2<i32>(round(q));
    var sum = vec3<f32>(0.0);
    var weight = 0.0;
    // the depths about this pixel: its history is kept if its depth lies among them, so a twig's pixel that showed
    // sky through it last frame keeps what it gathered, and a surface that moved off what it was showing loses it
    var zlo = 1e9;
    var zhi = 0.0;
    // the nearest surface about this pixel, whose motion it takes: at a silhouette the sample nearest the pixel may be
    // the surface behind, and the history read from where that one was crawled along the edge (photo 2's window)
    var front = clamp(near, vec2<i32>(0), vec2<i32>(in_size) - 1);
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let i = near + vec2<i32>(dx, dy);
            if (any(i < vec2<i32>(0)) || any(i >= vec2<i32>(in_size))) { continue; }
            let zi = textureLoad(depth, i, 0);
            zlo = min(zlo, zi);
            zhi = max(zhi, zi);
            if (zi >= zhi) { front = i; }
            // the distance in output pixels, a Blackman-Harris-like falloff, nothing past one pixel
            let d = (vec2<f32>(i) - q) * out_size / in_size;
            let d2 = dot(d, d);
            if (d2 >= 1.0) { continue; }
            // params[2].w: how fast a sample's weight falls with its distance (RAEA_EMIT_TAA_SHARP)
            let w = exp(-params[2].w * d2);
            sum += w * textureLoad(emit, i).rgb;
            weight += w;
        }
    }
    let here = clamp(near, vec2<i32>(0), vec2<i32>(in_size) - 1);
    let z = textureLoad(depth, here, 0);
    // where this pixel was last frame (Bevy's motion is this frame's uv less last frame's)
    let uv = (vec2<f32>(id.xy) + 0.5) / out_size;
    let moved = textureLoad(motion, front, 0).xy;
    let was = uv - moved;
    var hist = vec4<f32>(0.0);
    let inside = all(was >= vec2<f32>(0.0)) && all(was <= vec2<f32>(1.0));
    if (params[0].z < 0.5 && inside) {
        let h = history_at(was, out_size);
        let hz = textureLoad(hist_depth_in, min(vec2<i32>(was * out_size), vec2<i32>(out_size) - 1)).r;
        // reverse depth is 1 / distance up to a constant: within 5% of the depths about it. Only where the pixel moved:
        // nothing is uncovered where nothing moves, and where a twig hides sky in some of the jittered frames, a test
        // of depth drops whichever is rarer about it (sky, mostly), which lost a forest's canopy a quarter of its light
        // RAEA_EMIT_TAA_DEBUG=2 never takes the history, 3 always does (a test of what the depth test costs)
        let still = all(abs(moved * in_size) < vec2<f32>(0.01));
        let among = hz >= zlo * 0.95 && hz <= zhi * 1.05;
        let same = (still || among || params[0].w == 3.0) && params[0].w != 2.0;
        // a history that fails the test still counts as one frame's: dropped outright, a pixel on a moving silhouette's
        // trailing side was drawn from its frame's samples alone, and crawled (photo 2's window, 300 pixels a frame
        // jumping 80 levels); kept at full weight, whatever moved off it would trail behind
        var cap = select(1.0, HISTORY_CAP, same);
        if (params[0].w == 2.0) { cap = 0.0; }
        hist = vec4<f32>(h.rgb, min(h.a, cap));
    }
    var e: vec3<f32>;
    var held: f32;
    if (hist.a + weight > 1e-4) {
        e = (hist.rgb * hist.a + sum) / (hist.a + weight);
        held = hist.a + weight;
    } else {
        // no sample near and no history: the frame's emission between its samples, as `restore` puts it back
        let i = vec2<i32>(floor(q));
        let t = q - floor(q);
        e = mix(mix(emitted(i), emitted(i + vec2<i32>(1, 0)), t.x), mix(emitted(i + vec2<i32>(0, 1)), emitted(i + vec2<i32>(1, 1)), t.x), t.y);
        held = 0.0;
    }
    textureStore(hist_out, p, vec4<f32>(e, held));
    textureStore(hist_depth_out, p, vec4<f32>(z, 0.0, 0.0, 0.0));
    let c = textureLoad(color, p);
    // RAEA_EMIT_TAA_DEBUG=1: this frame's sample weight, the history's weight (of HISTORY_CAP) and whether the history
    // was taken, as red, green and blue
    if (params[0].w == 1.0) {
        textureStore(color, p, vec4<f32>(weight, hist.a / HISTORY_CAP, f32(hist.a > 0.0), c.a));
        return;
    }
    textureStore(color, p, vec4<f32>(c.rgb + e, c.a));
}
