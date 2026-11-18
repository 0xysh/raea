// raea GI's screen pass (lib.rs; docs/realism.feature LT8): each pixel's indirect diffuse light, read from the world
// cache (raea-sys-rt cache.wesl) at the pixel's own surface, with the sun's share taken out (raster draws the sun, its
// shadows with it). Written as the irradiance volume's light is (irradiance over pi), and 1 in alpha where a cell had
// any; 0 where the pixel is sky, outside the view, or its cell is new, so the main pass draws Bevy's own light there.
import bevy_render::view::View;
import raea_sys_rt::cache::{find, hash, radiance, cells, sunlit, LIFE, NONE};

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var normal_tex: texture_2d<f32>;
@group(0) @binding(3) var out_tex: texture_storage_2d<rgba16float, write>;

struct Params {
    /// toward the sun (xyz)
    sun_dir: vec4<f32>,
    /// the sun's light (rgb, lux), as the cache's cells were lit by it
    sun: vec4<f32>,
    /// the frame's salt
    run: vec4<u32>,
}
@group(0) @binding(4) var<uniform> p: Params;

const PI: f32 = 3.14159265;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(out_tex);
    if (any(id.xy >= size)) {
        return;
    }
    let px = vec2<i32>(id.xy);
    let at_px = vec2<f32>(id.xy) + 0.5;
    let uv = (at_px - view.viewport.xy) / view.viewport.zw;
    let d = textureLoad(depth_tex, px, 0);
    // sky (reverse z: 0 is infinitely far), or outside the view
    if (d <= 0.0 || any(uv < vec2(0.0)) || any(uv > vec2(1.0))) {
        textureStore(out_tex, px, vec4(0.0));
        return;
    }
    let w = view.world_from_clip * vec4(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, d, 1.0);
    let at = w.xyz / w.w;
    let n = normalize(textureLoad(normal_tex, px, 0).xyz * 2.0 - 1.0);
    var seed = hash(id.x * 1973u + id.y * 9277u + p.run.x);
    let k = find(at + n * 0.02, n, LIFE, &seed);
    if (k == NONE) {
        textureStore(out_tex, px, vec4(0.0));
        return;
    }
    let r = radiance[k];
    if (r.a < 1.0) {
        textureStore(out_tex, px, vec4(0.0));
        return;
    }
    // the sun as the cell saw it: on its own side, the share of its learning the sun was in view
    let sun = p.sun.xyz * max(dot(cells[k].n.xyz, p.sun_dir.xyz), 0.0) * sunlit[k];
    textureStore(out_tex, px, vec4(max(r.rgb - sun, vec3(0.0)) / PI, 1.0));
}
