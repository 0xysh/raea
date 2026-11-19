// A fog volume's shape as the froxel passes read it (froxel_light, froxel_hero, froxel_solari): how much of its density
// a point in its box has. froxel.rs packs `shape`: x the thinning per metre going up (`falloff`), y the box's height in
// metres, z the feathered share of the box's half-size at every face (`soft`), w how much narrower its floor is than
// its top (`taper`).

// The density at `local` (the box's own coordinates, -0.5 to 0.5) over the volume's density: 0 outside its shape.
// Thinning is exp(-falloff x metres above the floor). Taper narrows the box's width and depth linearly toward the floor,
// to 1 - taper of the top's (a plume rising off a tank). Soft ramps the density in from each face (the tapered sides
// too) over that share of the half-size with a smoothstep, so the box's edges don't show (0: a hard box).
fn volume_weight(shape: vec4<f32>, local: vec3<f32>) -> f32 {
    let up = local.y + 0.5;
    let wide = max(1.0 - shape.w * (1.0 - up), 1e-3);
    let across = abs(local.xz) / wide;
    if (any(across > vec2(0.5)) || abs(local.y) > 0.5) {
        return 0.0;
    }
    var w = exp(-shape.x * up * shape.y);
    if (shape.z > 0.0) {
        let inside = (vec3(0.5) - vec3(across.x, abs(local.y), across.y)) * 2.0 / shape.z;
        let e = clamp(inside, vec3(0.0), vec3(1.0));
        let s = e * e * (3.0 - 2.0 * e);
        w *= s.x * s.y * s.z;
    }
    return w;
}
