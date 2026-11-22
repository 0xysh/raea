// Training rows for the three story models (docs/story.feature ST6b), from the checked corpus (corpus.ts). The split is
// by story (a story's rows are all in one of train, val and test), so a held-out number is on stories never seen.
//
//   extract   script prose -> the story as data (no onScreen or lines in the answer: they're the input)
//   generate  a story with one beat cut, and where -> that beat, as JSON under the story's own beat schema
//   critic    script prose -> where it breaks: [{beat, check, level}], labelled exactly by checkStory on the structure
//             behind the prose. Negatives are cuts a viewer could see: a beat cut, two beats swapped, a beat from
//             another story spliced in. A cut that breaks nothing is labelled clean, so the critic must read, not count.
//
//   bun models/rows.ts [--in ~/.vd/story] [--out ~/.vd/story/rows]
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { beatSchema, checkStory, fromData, rng, storySchema, type AnyStory } from "../src/index.ts";

const arg = (k: string, d: string) => { const i = process.argv.indexOf(k); return i > 0 ? process.argv[i + 1] : d; };
const IN = arg("--in", join(homedir(), ".vd/story"));
const OUT = arg("--out", join(IN, "rows"));
mkdirSync(OUT, { recursive: true });

type J = Record<string, any>;
const stories: { i: number; s: J; premise: J }[] = readFileSync(join(IN, "stories.jsonl"), "utf8").split("\n").filter(Boolean).map((l) => JSON.parse(l)).map((d) => ({ i: d.i, s: d.story, premise: d.premise }));

/** The split a story falls in, by its index. */
export const split = (i: number) => { const h = (Math.imul(i + 1, 2654435761) >>> 0) % 10; return h === 0 ? "test" : h === 1 ? "val" : "train"; };

const name = (k: string) => (k === "narrator" ? "NARRATOR" : k.replace(/([a-z])([A-Z])/g, "$1 $2").toUpperCase());

/** A telling as a script a person would write: numbered beats, what's seen, who says what. No keys, no annotations. */
export function prose(beats: J[]): string {
  return beats.map((b, n) => [`${n + 1}. ${b.onScreen ?? ""}`.trimEnd(), ...(b.says ?? []).map((l: J) => `   ${name(l.who)}: ${l.says}`)].join("\n")).join("\n");
}

const withTelling = (s: J, beats: J[]): J => ({ ...s, tellings: { main: beats } });
const breaks = (s: J, beats: J[]) => {
  const st = fromData(withTelling(s, beats)).story as AnyStory;
  const keys = beats.map((b, i) => b.id ?? `b${i}`);
  const firstShowing = (ev: string) => beats.findIndex((b) => (b.shows ?? []).includes(ev));
  return checkStory(st, "main")
    .filter((f) => f.level !== "note")
    .map((f) => {
      let beat: number | null = keys.indexOf(f.about);
      if (beat < 0) beat = s.world?.[f.about] ? firstShowing(f.about) : -1;
      return { beat: beat >= 0 ? beat + 1 : null, check: f.check, level: f.level };
    })
    .sort((a, b) => (a.beat ?? 0) - (b.beat ?? 0) || a.check.localeCompare(b.check));
};

/** The story as the extractor answers it: everything but the words on screen. */
const bare = (s: J): J => ({
  ...(s.title ? { title: s.title } : {}),
  ...(s.idea ? { idea: s.idea } : {}),
  values: s.values, facts: s.facts ?? {}, agents: s.agents, world: s.world,
  tellings: { main: s.tellings.main.map(({ onScreen: _o, says: _s, id: _i, ...b }: J) => b) },
});

const out: Record<string, Record<string, J[]>> = { extract: {}, generate: {}, critic: {} };
const push = (task: string, sp: string, row: J) => ((out[task][sp] ??= []).push(row));
let clean = 0, broken = 0;

for (const { i, s } of stories) {
  const sp = split(i);
  const beats: J[] = s.tellings.main;
  if (!beats?.length) continue;
  const r = rng(i * 7919 + 17);
  const p = prose(beats);
  push("extract", sp, { i, kind: "script", prompt: p, answer: JSON.stringify(bare(s)) });
  // a transcript has the words alone: the same story from its lines, as a launch film's speech reads
  const spoken = beats.flatMap((b) => (b.says ?? []).map((l: J) => `${name(l.who)}: ${l.says}`));
  if (spoken.length >= 2) push("extract", sp, { i, kind: "lines", prompt: spoken.join("\n"), answer: JSON.stringify(bare(s)) });

  // generate: each beat cut in turn on test and val; one at random on train (keeps the set balanced by story)
  const cuts = sp === "train" ? [Math.floor(r() * beats.length)] : beats.map((_, k) => k);
  for (const k of cuts) {
    const rest = beats.filter((_, n) => n !== k);
    const partial = { ...s, tellings: { main: rest } };
    push("generate", sp, { i, at: k, prompt: JSON.stringify({ story: partial, write: `the beat that goes at place ${k + 1} of ${beats.length}` }), answer: JSON.stringify(beats[k]), schema: beatSchema(s) });
  }

  // critic: the story as written, then cuts a viewer could see
  const variants: { kind: string; beats: J[]; world?: J }[] = [{ kind: "clean", beats }];
  const k = Math.floor(r() * beats.length);
  if (beats.length > 2) variants.push({ kind: "cut", beats: beats.filter((_, n) => n !== k) });
  if (beats.length > 2) { const a = Math.floor(r() * (beats.length - 1)); const b2 = [...beats]; [b2[a], b2[a + 1]] = [b2[a + 1], b2[a]]; variants.push({ kind: "swap", beats: b2 }); }
  const other = stories[Math.floor(r() * stories.length)];
  if (other.i !== i && other.s.tellings?.main?.length) {
    const ob = other.s.tellings.main[Math.floor(r() * other.s.tellings.main.length)];
    const firstAgent = Object.keys(s.agents)[0];
    const ev = `spliced${other.i}`;
    const at = 1 + Math.floor(r() * (beats.length - 1));
    const b2 = [...beats.slice(0, at), { shows: [ev], onScreen: ob.onScreen, says: (ob.says ?? []).map((l: J) => ({ ...l, who: "narrator" })) }, ...beats.slice(at)];
    variants.push({ kind: "splice", beats: b2, world: { ...s.world, [ev]: { who: firstAgent, does: "something unrelated happens" } } });
  }
  for (const v of variants) {
    const base = v.world ? { ...s, world: v.world } : s;
    let label: J[];
    try { label = breaks(base, v.beats); } catch { continue; }
    label.length ? broken++ : clean++;
    push("critic", sp, { i, kind: v.kind, prompt: prose(v.beats), answer: JSON.stringify({ breaks: label }) });
  }
}

const CHECKS = ["turns", "and-then", "surprise", "cause", "dead-end", "setup", "belief", "rhythm", "arc", "claim", "idea"];
const criticSchema = {
  type: "object", additionalProperties: false, required: ["breaks"],
  properties: { breaks: { type: "array", items: { type: "object", additionalProperties: false, required: ["beat", "check", "level"], properties: { beat: { anyOf: [{ type: "integer", minimum: 1 }, { type: "null" }] }, check: { type: "string", enum: CHECKS }, level: { type: "string", enum: ["error", "warn"] } } } } },
};
writeFileSync(join(OUT, "schemas.json"), JSON.stringify({ extract: storySchema(), critic: criticSchema }, null, 1));
for (const [task, sps] of Object.entries(out)) {
  mkdirSync(join(OUT, task), { recursive: true });
  for (const [sp, rows] of Object.entries(sps)) writeFileSync(join(OUT, task, `${sp}.jsonl`), rows.map((x) => JSON.stringify(x)).join("\n") + "\n");
  console.log(task, Object.fromEntries(Object.entries(sps).map(([k, v]) => [k, v.length])));
}
console.log(`critic labels: ${clean} clean, ${broken} with a break`);
