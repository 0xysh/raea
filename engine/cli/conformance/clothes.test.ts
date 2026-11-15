// `raea clothes`: what the command sends the host for a garment made from words or garments put on a body (docs/
// character.feature CH45), and what it refuses before anything is drawn or paid for. The host (src/wardrobe.ts, which
// runs Blender and the image model) is faked: this is the command's side.
import { describe, expect, test } from "bun:test";
import { clothesVerb } from "../src/clothes.ts";

/** clothesVerb with a host that keeps each op it was asked and answers as the host does */
function run(args: string[], garments: { name: string; layer: number; vertices: number; over?: string }[] = []) {
  const ops: Record<string, unknown>[] = [];
  const ask = (op: string, b: Record<string, unknown>) => {
    expect(op).toBe("clothes");
    ops.push(b);
    return b.op === "list" ? { garments } : { said: `did ${b.op}\n` };
  };
  return { out: clothesVerb(ask, args), ops };
}

describe("raea clothes", () => {
  test("alone, it lists the project's garments, and says how to make one when there are none", () => {
    expect(run([]).out).toContain('raea clothes make NAME "what it is, as worn"');
    const { out } = run(["list"], [{ name: "tee-jeans", layer: 0, vertices: 11873 }, { name: "bomber", layer: 1, vertices: 6120, over: "tee-jeans" }]);
    expect(out).toBe("tee-jeans  layer 0, 11873 of SOMA's vertices\nbomber  layer 1, 6120 of SOMA's vertices, over tee-jeans\n");
  });

  test("make sends the name, the words, the layer under it and the most a picture may cost", () => {
    const { ops } = run(["make", "bomber", "an olive bomber jacket, zipped", "--over", "tee-jeans", "--max-cents", "8"]);
    expect(ops).toEqual([{ op: "make", name: "bomber", words: "an olive bomber jacket, zipped", over: "tee-jeans", cents: 8 }]);
    expect(run(["make", "tee", "a white tee"]).ops[0]).toMatchObject({ over: undefined, cents: 10 });
  });

  test("cover sends the parts, the colour and the stand-off in metres, for a garment with no picture", () => {
    const { ops } = run(["cover", "shorts", "--parts", "hips, thigh:0.5", "--colour", "1f2a44", "--stand", "8", "--over", "pants"]);
    expect(ops).toEqual([{ op: "cover", name: "shorts", parts: "hips,thigh:0.5", colour: "#1f2a44", stand: 0.008, over: "pants" }]);
    expect(run(["cover", "tee", "--parts", "chest,torso,waist,upperArm:0.4", "--colour", "#ffffff"]).ops[0]).toMatchObject({ stand: 0.006, over: undefined });
  });

  test("wear sends the folder, the garments innermost first and the body", () => {
    const { ops } = run(["wear", "assets/characters/dressed", "tee-jeans", "bomber", "--body", "assets/characters/soma-headless"]);
    expect(ops).toEqual([{ op: "wear", out: "assets/characters/dressed", garments: ["tee-jeans", "bomber"], body: "assets/characters/soma-headless" }]);
  });

  test("what doesn't fit is said before anything is drawn", () => {
    const refused = (args: string[], says: string) => expect(() => run(args)).toThrow(says);
    refused(["make", "tee"], 'raea clothes make NAME "WHAT IT IS, AS WORN"');
    refused(["make", "tee", "a white tee", "and more"], 'raea clothes make NAME "WHAT IT IS, AS WORN"');
    refused(["make", "../tee", "a white tee"], "a garment's name is letters, digits, - and _");
    refused(["make", "tee", "a white tee", "--over", "a b"], "--over a b: name garments already made");
    expect(run(["make", "jacket", "a denim jacket", "--over", "tee,shorts"]).ops[0]).toMatchObject({ over: "tee,shorts" });
    refused(["make", "tee", "a white tee", "--max-cents", "0"], "--max-cents takes the most one picture may cost");
    refused(["cover", "tee", "--parts", "chest"], "raea clothes cover NAME --parts");
    refused(["cover", "tee", "--parts", "belly", "--colour", "#ffffff"], "--parts: no part belly");
    refused(["cover", "tee", "--parts", "shin:1.5", "--colour", "#ffffff"], "a share is how far along the limb");
    refused(["cover", "tee", "--parts", "chest", "--colour", "white"], "--colour white: a colour is #RRGGBB");
    refused(["cover", "tee", "--parts", "chest", "--colour", "#ffffff", "--stand", "200"], "--stand takes how far the cloth stands off");
    refused(["wear", "out"], "raea clothes wear OUT_DIR GARMENT...");
    refused(["dress"], "raea clothes make NAME");
  });
});
