// The WESL that reads the bake's four cause maps (docs/wear.feature WR1-WR3, WR9) through the model's second UV set,
// and the marks every class shares: what years of rain, soot and neglect left at a point, the same in every season.
//   causes0 = sky (rain's reach), sun (the year's sun), cavity, edge (convex arris)
//   causes1 = flow (run-off passing), deposit (what it left), deposit kind (60 x: 1 grime, 2 rust, 3 copper, 4 zinc), touch
//   causes2 = ground (height over the walk, 0-4 m), element (the chart's number), open (the whole sky seen), class
//   causes3 = winter sun (December's), drift (driven snow), litter (where blown leaves and snow settle)
// The maps load as colour (sRGB) textures, so a colour channel is re-encoded to read the byte the bake wrote; alpha is
// stored linear. Element and class are numbers, read with textureLoad so a filtered read never blends two into a third.

/** The uniforms the module reads: `up` (the bake's up), `age` (years since the last repair, soot, upkeep, the bake's
 *  texels a metre), `wx` (wet, snow, storm, ice) and `season` (damp, dust, leaves, melt). */
export const CAUSES = /* wgsl */ `
fn wear_enc(l: f32) -> f32 { return select(1.055 * pow(max(l, 0.0), 1.0 / 2.4) - 0.055, 12.92 * l, l <= 0.0031308); }

struct WearCauses {
  sky: f32, sun: f32, cav: f32, edge: f32,
  flow: f32, dep: f32, kind: i32, touch: f32,
  ground: f32, elem: f32, open: f32, cls: i32,
  dec: f32, drift: f32, litter: f32
}

fn wear_causes(uv: vec2<f32>) -> WearCauses {
  let a = textureSample(tex_c0, samp_c0, uv);
  let f = textureSample(tex_c1, samp_c0, uv);
  let b = textureSample(tex_c2, samp_c0, uv);
  let d = textureSample(tex_c3, samp_c0, uv);
  let dims = vec2<f32>(textureDimensions(tex_c2));
  let at = vec2<i32>(clamp(floor(uv * dims), vec2<f32>(0.0), dims - vec2<f32>(1.0)));
  let x = textureLoad(tex_c2, at, 0);
  let kind = i32(round(wear_enc(f.b) * 255.0 / 60.0));
  let cls = i32(round(x.a * 255.0 / 16.0)) - 1;
  return WearCauses(
    wear_enc(a.r), wear_enc(a.g), wear_enc(a.b), a.a,
    wear_enc(f.r), wear_enc(f.g), kind, f.a,
    wear_enc(b.r) * 4.0, round(wear_enc(x.g) * 255.0), wear_enc(b.b), cls,
    wear_enc(d.r), wear_enc(d.g), wear_enc(d.b)
  );
}

// value noise over the face, in metres
fn wear_h21(p: vec2<f32>) -> f32 {
  var q = fract(p * vec2<f32>(123.34, 456.21));
  q += dot(q, q + 45.32);
  return fract(q.x * q.y);
}
fn wear_vn2(p: vec2<f32>) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let u = f * f * (3.0 - 2.0 * f);
  return mix(mix(wear_h21(i), wear_h21(i + vec2<f32>(1.0, 0.0)), u.x), mix(wear_h21(i + vec2<f32>(0.0, 1.0)), wear_h21(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}
fn wear_fbm2(p: vec2<f32>) -> f32 {
  var v = 0.0;
  var a = 0.5;
  var q = p;
  for (var i = 0; i < 4; i++) {
    v += a * wear_vn2(q);
    q = q * 2.03 + vec2<f32>(1.7, 9.2);
    a *= 0.5;
  }
  return v;
}

// where the point is: how far its face looks up, its place on the face in metres (along a wall and up it; two
// horizontal axes on a level face), its height over the walk, and how upright the face is
struct WearGeo { up: f32, p: vec2<f32>, h: f32, wall: f32 }

fn wear_geo(wp: vec3<f32>, nrm: vec3<f32>, c: WearCauses) -> WearGeo {
  let U = normalize(u_up());
  let n = normalize(nrm);
  let up = dot(n, U);
  let side = n - U * up;
  var p: vec2<f32>;
  if (length(side) > 0.3) {
    let t = normalize(cross(U, side));
    p = vec2<f32>(dot(wp, t), dot(wp, U));
  } else {
    let r = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(U.x) > 0.9);
    let e1 = normalize(cross(U, r));
    p = vec2<f32>(dot(wp, e1), dot(wp, cross(U, e1)));
  }
  return WearGeo(up, p, c.ground, 1.0 - smoothstep(0.5, 0.8, abs(up)));
}

// what years of weather left, by cause: the same in every season
struct WearMarks {
  age: f32, upkeep: f32, wear: f32,
  wash: f32, shelter: f32, tuck: f32, crust: f32,
  tear: f32, stain: f32, damp: f32, salt: f32, splash: f32,
  low: f32, chip: f32
}

fn wear_marks(c: WearCauses, g: WearGeo) -> WearMarks {
  let years = max(u_age().x, 0.0);
  // the marks come on fast in the first decades and then hold (soot crusts and paint fails within 20-30 years)
  let age = 1.0 - exp(-years / 25.0);
  // one draw per element (a front, a ledge), so neighbours differ: a house kept up beside one let go
  let upkeep = clamp(u_age().z + (wear_h21(vec2<f32>(c.elem, 7.0)) - 0.5) * 0.7, 0.0, 1.0);
  let wear = age * (1.0 - 0.7 * upkeep);
  let wash = smoothstep(0.35, 0.85, c.sky);
  let shelter = 1.0 - smoothstep(0.08, 0.5, c.sky);
  let air = smoothstep(0.04, 0.3, c.open);
  // tucked: less sky than a plain wall sees (about half), so under a cornice, in a reveal or a court
  let tuck = 1.0 - smoothstep(0.08, 0.3, c.open);
  // soot lies in streaks: run-off cleans narrow bands and leaves the rest
  let streaks = wear_fbm2(vec2<f32>(g.p.x * 2.1 + c.elem * 0.37, g.p.y * 0.11));
  let mottle = 0.35 + 0.9 * smoothstep(0.3, 0.75, streaks) * (0.85 + 0.3 * wear_vn2(g.p * 6.0));
  let crust = clamp(u_age().y * air * (0.12 + 0.25 * shelter + 0.5 * tuck + 0.3 * c.cav) * mottle * age * (1.0 - 0.6 * upkeep), 0.0, 0.85);
  let tear = smoothstep(0.6, 0.9, c.flow);
  let stain = smoothstep(0.04, 0.45, c.dep) * age;
  // rising damp to a tide line 0.6-1.2 m up, higher where the sun never dries it, its salts on top
  let tide = mix(0.6, 1.2, clamp(0.55 * (1.0 - c.sun) + 0.25 * wear_vn2(vec2<f32>(g.p.x * 0.7, 3.0)) + 0.2 * (1.0 - upkeep), 0.0, 1.0));
  let damp = (1.0 - smoothstep(tide - 0.2, tide, g.h)) * age * (0.5 + 0.5 * (1.0 - upkeep)) * g.wall;
  let salt = smoothstep(tide - 0.12, tide - 0.03, g.h) * (1.0 - smoothstep(tide - 0.01, tide + 0.06, g.h)) * age * g.wall;
  let splash = (1.0 - smoothstep(0.06, 0.42, g.h)) * (0.6 + 0.4 * wear_vn2(g.p * 13.0)) * age * g.wall;
  let low = 1.0 - smoothstep(1.4, 2.2, g.h);
  // chips need a real arris, so a bake of 12 texels a metre or finer (at 4 an edge spreads 25 cm); people and carts
  // reach below 2.2 m
  let chip = step(12.0, u_age().w) * smoothstep(0.75, 0.9, c.edge) * low * step(0.68 - 0.2 * (1.0 - upkeep), wear_vn2(g.p * 16.0 + vec2<f32>(c.elem * 9.0, 0.0))) * step(0.05, age);
  return WearMarks(age, upkeep, wear, wash, shelter, tuck, crust, tear, stain, damp, salt, splash, low, chip);
}

// a surface's own colour, roughness and metalness as the recipes change them
struct WearSurf { col: vec3<f32>, rough: f32, metal: f32 }
`;
