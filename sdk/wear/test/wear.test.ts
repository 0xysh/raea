import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { surfaceShader } from "@raea/render/3d/materials";
import { DEBUG_CAUSES, seasonOf, WEAR_CLASSES, wearShader, type WearCause } from "../src/index";
import { RECIPES } from "../src/recipes";

const baked = { maps: { causes0: "/s/causes0.png", causes1: "/s/causes1.png", causes2: "/s/causes2.png", causes3: "/s/causes3.png" }, density: 16 };
const ROOT = new URL("../../..", import.meta.url).pathname;

describe("the class table", () => {
  test("is the bake's, in its order, so causes2's code names the same class", () => {
    const rs = readFileSync(`${ROOT}systems/weathering/src/scene.rs`, "utf8");
    const table = rs.slice(rs.indexOf("const NAMES"), rs.indexOf("];", rs.indexOf("const NAMES")));
    const names = [...table.matchAll(/\("([a-z]+)", Class::/g)].map((m) => m[1]);
    expect(names.length).toBe(14);
    expect(WEAR_CLASSES.map((c) => c.name)).toEqual(names as never);
    // the code the shader decodes: 16 x (place + 1)
    expect(rs).toContain("16 * (NAMES.iter().position(|(_, c)| *c == self).unwrap_or(0) as u8 + 1)");
  });
  test("every class has a recipe and every recipe a class", () => {
    expect(Object.keys(RECIPES).sort()).toEqual(WEAR_CLASSES.map((c) => c.name).sort());
  });
});

describe("the shader", () => {
  const sh = wearShader({ baked, age: { years: 60, soot: 0.9, upkeep: 0.3 }, weather: { day: 300, wet: 0.4 } });

  test("is a surface hook raea's shader check takes, with the four maps sampled linear and clamped", () => {
    const spec = surfaceShader("MeshStandardMaterial", sh);
    expect(spec.wesl).toContain("fn surface(in: VertexOutput, pbr: ptr<function, PbrInput>)");
    expect(Object.keys(sh.textures)).toEqual(["c0", "c1", "c2", "c3"]);
    expect(sh.textures.c2.src).toBe("/s/causes2.png");
    expect(sh.textures.c0.sampler).toEqual({ filter: "linear", mipmap: "none", wrap: "clamp" });
    // a mesh the bake didn't give a second UV set is left as it was
    expect(sh.wesl).toMatch(/@if\(VERTEX_UVS_B\) \{\s+wear_surface\(in\.uv_b, pbr\);/);
  });

  test("reads only the uniforms and textures it's given", () => {
    const used = new Set([...sh.wesl.matchAll(/\bu_([a-z]+)\(\)/g)].map((m) => m[1]));
    expect([...used].sort()).toEqual(Object.keys(sh.uniforms).sort());
    const tex = new Set([...sh.wesl.matchAll(/\btex_([a-z0-9]+)\b/g)].map((m) => m[1]));
    expect([...tex].sort()).toEqual(Object.keys(sh.textures).sort());
    expect(Object.keys(sh.uniforms).length).toBeLessThanOrEqual(16);
  });

  test("has one function per class and a case for each, by the bake's number", () => {
    WEAR_CLASSES.forEach((c, i) => {
      expect(sh.wesl).toContain(`fn wear_${c.name}(s: ptr<function, WearSurf>, c: WearCauses, g: WearGeo, m: WearMarks)`);
      expect(sh.wesl).toContain(`case ${i}: { wear_${c.name}(s, c, g, m); }`);
    });
  });

  test("takes only the classes asked for, the rest drawn as they are", () => {
    const iron = wearShader({ baked, classes: ["iron"] });
    expect(iron.wesl).toContain("case 5: { wear_iron(s, c, g, m); }");
    expect(iron.wesl).not.toContain("fn wear_plaster(");
    expect(iron.wesl).toContain("default: { return false; }");
    expect(() => wearShader({ baked, classes: ["marble" as never] })).toThrow(/no class "marble"/);
  });

  test("declares each function once", () => {
    const fns = [...sh.wesl.matchAll(/\bfn\s+([A-Za-z_][A-Za-z0-9_]*)/g)].map((m) => m[1]);
    expect(fns.filter((f, i) => fns.indexOf(f) !== i)).toEqual([]);
    const d = wearShader({ baked, debug: "class" }).wesl;
    const dfns = [...d.matchAll(/\bfn\s+([A-Za-z_][A-Za-z0-9_]*)/g)].map((m) => m[1]);
    expect(dfns.filter((f, i) => dfns.indexOf(f) !== i)).toEqual([]);
  });

  test("never names a WGSL reserved word", () => {
    const reserved = ["class", "filter", "type", "match", "mod", "module", "self", "set", "get", "new", "from", "target", "common", "enum", "union", "export", "final", "move", "pass", "precise", "static", "this", "typedef", "using", "where", "yield"];
    const idents = new Set([...sh.wesl.matchAll(/\b(?:let|var|fn)\s+([A-Za-z_][A-Za-z0-9_]*)|\b([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?:f32|i32|u32|bool|vec[234]|ptr|Wear)/g)].map((m) => m[1] ?? m[2]));
    for (const r of reserved) expect(idents.has(r)).toBe(false);
  });
});

describe("the uniforms", () => {
  test("carry age, the day's weather and the season's own share", () => {
    const sh = wearShader({ baked, age: { years: 60, soot: 0.9, upkeep: 0.3 }, weather: { day: 300, wet: 0.4, snow: 0.2 }, up: [0, 0, 1] });
    expect(sh.uniforms.up).toEqual([0, 0, 1]);
    expect(sh.uniforms.age).toEqual([60, 0.9, 0.3, 16]);
    expect(sh.uniforms.wx).toEqual([0.4, 0.2, 0, 0]);
    const [damp, dust, leaves, melt] = sh.uniforms.season;
    expect(leaves).toBeGreaterThan(0.85);
    expect(damp).toBe(0);
    expect(dust).toBe(0);
    expect(melt).toBe(0.5);
  });
  test("the season follows the day: spring's damp, summer's dust, autumn's leaves", () => {
    expect(seasonOf({ day: 100 }).damp).toBeGreaterThan(0.7);
    expect(seasonOf({ day: 200 }).dust).toBeGreaterThan(0.7);
    expect(seasonOf({ day: 200, wet: 1 }).dust).toBe(0);
    expect(seasonOf({ day: 20 })).toEqual({ damp: 0, dust: 0, leaves: 0, melt: 0 });
    expect(seasonOf({ day: 300, leaves: 0.1 }).leaves).toBe(0.1);
  });
  test("refuse what isn't a share or a day", () => {
    expect(() => wearShader({ baked, age: { soot: 2 } })).toThrow(/age.soot is 0..1/);
    expect(() => wearShader({ baked, age: { years: -1 } })).toThrow(/age.years/);
    expect(() => wearShader({ baked, weather: { day: 400 } })).toThrow(/weather.day/);
    expect(() => wearShader({ baked: { density: 4 } as never })).toThrow(/four maps/);
  });
});

describe("the debug mode", () => {
  test("draws each cause from a lit hook alone", () => {
    for (const cause of Object.keys(DEBUG_CAUSES) as WearCause[]) {
      const d = wearShader({ baked, debug: cause });
      surfaceShader("MeshStandardMaterial", d);
      expect(d.wesl).toContain("fn lit(in: VertexOutput, pbr: PbrInput, color: vec4<f32>) -> vec4<f32>");
      expect(d.wesl).not.toContain("fn surface(");
      expect(d.wesl).toContain(`out = vec4<f32>(${DEBUG_CAUSES[cause]}, 1.0);`);
    }
    // each class its own colour, in the table's order
    expect(wearShader({ baked, debug: "class" }).wesl).toContain("case 5: { return vec3<f32>(0.250, 0.300, 0.400); }");
    expect(() => wearShader({ baked, debug: "rain" as never })).toThrow(/debug is one of/);
  });
});
