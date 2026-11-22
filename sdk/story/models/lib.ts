// What the corpus rows (rows.ts) and a held-out story's rows (validate.ts) share: a telling as prose, the story as the
// extractor answers it, and exact labels for a telling a viewer could see.
import { checkStory, fromData, type AnyStory } from "../src/index.ts";

type J = Record<string, any>;

export const name = (k: string) => (k === "narrator" ? "NARRATOR" : k.replace(/([a-z])([A-Z])/g, "$1 $2").toUpperCase());

/** A telling as a script a person would write: numbered beats, what's seen, who says what. No keys, no annotations. */
export function prose(beats: J[]): string {
  return beats.map((b, n) => [`${n + 1}. ${b.onScreen ?? ""}`.trimEnd(), ...(b.says ?? []).map((l: J) => `   ${name(l.who)}: ${l.says}`)].join("\n")).join("\n");
}

export const withTelling = (s: J, beats: J[]): J => ({ ...s, tellings: { main: beats } });
export const breaks = (s: J, beats: J[]) => {
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
export const bare = (s: J): J => ({
  ...(s.title ? { title: s.title } : {}),
  ...(s.idea ? { idea: s.idea } : {}),
  values: s.values, facts: s.facts ?? {}, agents: s.agents, world: s.world,
  tellings: { main: s.tellings.main.map(({ onScreen: _o, says: _s, id: _i, ...b }: J) => b) },
});

export const CHECKS = ["turns", "and-then", "surprise", "cause", "dead-end", "setup", "belief", "rhythm", "arc", "claim", "idea"];
export const criticSchema = {
  type: "object", additionalProperties: false, required: ["breaks"],
  properties: { breaks: { type: "array", items: { type: "object", additionalProperties: false, required: ["beat", "check", "level"], properties: { beat: { anyOf: [{ type: "integer", minimum: 1 }, { type: "null" }] }, check: { type: "string", enum: CHECKS }, level: { type: "string", enum: ["error", "warn"] } } } } },
};
