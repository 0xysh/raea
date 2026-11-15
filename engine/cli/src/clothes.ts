// `raea clothes`: clothes for SOMA's body from words (raea docs/character.feature CH40 to CH45). make has an image
// model dress the mannequin's four views, one view a call, and measures the pictures onto SOMA's template vertices
// (a garment rides any SOMA body); wear puts garments on a body, innermost first. The work is the host's (op
// "clothes": engine/cli/src/wardrobe.ts on the native raea); this file reads the words and says what was done.

type Ask = (op: string, args: Record<string, unknown>) => any;

/** the body's parts a garment made by procedure covers (tools/clothes/garment.py's PARTS) */
export const PARTS = ["chest", "foot", "foreArm", "hand", "head", "hips", "neck", "shin", "thigh", "toes", "torso", "upperArm", "waist"];

export const CLOTHES_USAGE = `raea clothes   (the project's garments: assets/clothes/NAME.json, their pictures in assets/clothes/NAME/)
raea clothes make NAME "WHAT IT IS, AS WORN" [--over UNDER[,UNDER...]] [--max-cents N]   (drawn over the garments named; N: the most one picture may cost, 10 unless said)
raea clothes remeasure NAME   (measured again from its kept pictures, after raea's measuring changed: nothing drawn or paid for)
raea clothes cover NAME --parts PART[:SHARE],... --colour #RRGGBB [--stand MM] [--over UNDER]   (by procedure, no picture: ${PARTS.join(", ")}; a limb's share is how far along it from its top, foreArm:0.5)
raea clothes wear OUT_DIR GARMENT... [--body DIR]   (GARMENT: a garment's name or its .json, innermost first; DIR: a SOMA body folder, SOMA's own unless said)
`;

const flagOf = (args: string[], name: string) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
/** the words that aren't flags or a flag's value */
const plain = (args: string[]) => args.filter((a, i) => !a.startsWith("--") && !args[i - 1]?.startsWith("--"));
const NAME = /^[a-z0-9][a-z0-9_-]*$/i;

export function clothesVerb(ask: Ask, args: string[]): string {
  const q = (op: string, a: Record<string, unknown> = {}) => ask("clothes", { op, ...a });
  const [sub, ...rest] = args;
  switch (sub) {
    case undefined:
    case "list": {
      const gs: { name: string; layer: number; vertices: number; over?: string }[] = q("list").garments;
      if (!gs.length) return 'No garments yet. raea clothes make NAME "what it is, as worn" makes one.\n';
      return gs.map((g) => `${g.name}  layer ${g.layer}, ${g.vertices} of SOMA's vertices${g.over ? `, over ${g.over}` : ""}`).join("\n") + "\n";
    }
    case "make": {
      const [name, words, ...more] = plain(rest);
      if (!name || !words || more.length) throw new Error(`raea clothes make NAME "WHAT IT IS, AS WORN" [--over UNDER] [--max-cents N]\n${CLOTHES_USAGE}`);
      if (!NAME.test(name)) throw new Error(`${name}: a garment's name is letters, digits, - and _ (it names its file)`);
      const over = flagOf(rest, "--over"), cents = Number(flagOf(rest, "--max-cents") ?? 10);
      if (over !== undefined && !over.split(",").every((o) => NAME.test(o))) throw new Error(`--over ${over}: name garments already made, innermost first, with commas between (raea clothes lists them)`);
      if (!(cents > 0)) throw new Error("--max-cents takes the most one picture may cost, in cents");
      return q("make", { name, words, over, cents }).said;
    }
    case "remeasure": {
      const [name, ...more] = plain(rest);
      if (!name || more.length) throw new Error(`raea clothes remeasure NAME   (measured again from its kept pictures; nothing is drawn or paid for)\n${CLOTHES_USAGE}`);
      if (!NAME.test(name)) throw new Error(`${name}: a garment's name is letters, digits, - and _ (it names its file)`);
      return q("remeasure", { name }).said;
    }
    case "cover": {
      const [name, ...more] = plain(rest);
      const parts = flagOf(rest, "--parts"), colour = flagOf(rest, "--colour"), over = flagOf(rest, "--over"), mm = Number(flagOf(rest, "--stand") ?? 6);
      if (!name || more.length || !parts || !colour) throw new Error(`raea clothes cover NAME --parts PART[:SHARE],... --colour #RRGGBB [--stand MM] [--over UNDER]\n${CLOTHES_USAGE}`);
      if (!NAME.test(name)) throw new Error(`${name}: a garment's name is letters, digits, - and _ (it names its file)`);
      for (const p of parts.split(",")) {
        const [part, share] = p.trim().split(":");
        if (!PARTS.includes(part)) throw new Error(`--parts: no part ${part}; the parts are ${PARTS.join(", ")}`);
        if (share !== undefined && !(Number(share) > 0 && Number(share) <= 1)) throw new Error(`--parts ${p}: a share is how far along the limb, above 0 and at most 1`);
      }
      if (!/^#?[0-9a-f]{6}$/i.test(colour)) throw new Error(`--colour ${colour}: a colour is #RRGGBB`);
      if (!(mm > 0 && mm <= 80)) throw new Error("--stand takes how far the cloth stands off the skin, in millimetres (above 0, at most 80)");
      if (over !== undefined && !NAME.test(over)) throw new Error(`--over ${over}: name a garment already made (raea clothes lists them)`);
      return q("cover", { name, parts: parts.replace(/\s+/g, ""), colour: colour.startsWith("#") ? colour : `#${colour}`, stand: mm / 1000, over }).said;
    }
    case "wear": {
      const [out, ...garments] = plain(rest);
      if (!out || !garments.length) throw new Error(`raea clothes wear OUT_DIR GARMENT... [--body DIR]\n${CLOTHES_USAGE}`);
      return q("wear", { out, garments, body: flagOf(rest, "--body") }).said;
    }
    default:
      throw new Error(CLOTHES_USAGE);
  }
}
