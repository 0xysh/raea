import { expect, test } from "bun:test";
import { LENSES, readThrough, story, value } from "../src/index.ts";
import { launch } from "./fixture.ts";

test("the launch story passes McKee, Pixar, Storynomics and PAS, mapped as expected", () => {
  const r = readThrough(launch, Object.values(LENSES), "30s");
  for (const x of r) expect([x.lens, x.findings.filter((f) => f.level === "error")]).toEqual([x.lens, []]);
  const by = Object.fromEntries(r.map((x) => [x.lens, x.map]));
  expect(by.mckee.inciting).toEqual(["b1"]);
  expect(by.mckee.crisis).toEqual(["b2"]);
  expect(by.mckee.climax).toEqual(["b3"]);
  expect(by.pas).toEqual({ problem: ["b1"], agitate: ["b2"], solve: ["b3"] });
});

test("a brand that saves the day itself fails Storynomics", () => {
  const brandHero = { ...launch, world: { ...launch.world, ships: { ...launch.world.ships, who: "raea" as const, with: ["team" as const] } } };
  const r = readThrough(brandHero, [LENSES.storynomics], "30s")[0];
  expect(r.passes).toBe(false);
  expect(r.findings.some((f) => f.check === "storynomics:hero")).toBe(true);
});

test("a story with no dilemma gets a crisis proposed from the hero's want and need", () => {
  const flat = story({
    values: { v: value("low", "high") },
    agents: { kid: { want: "win the race", need: "learn to lose", mask: "hero" } },
    world: { a: { who: "kid", does: "trains", routine: true }, b: { who: "kid", does: "falls at the start", because: "a", turns: { v: -2 } }, c: { who: "kid", does: "finishes last, laughing", because: "b", turns: { v: 2 }, ends: { kid: "need" } } },
    tellings: { t: [{ shows: ["a"] }, { shows: ["b"] }, { shows: ["c"] }] },
  });
  const r = readThrough(flat, [LENSES.mckee], "t")[0];
  expect(r.findings.some((f) => f.check === "mckee:crisis" && f.level === "error")).toBe(true);
  expect(r.proposals.find((p) => p.stage === "crisis")?.onScreen).toBe("kid must choose: win the race, or learn to lose");
});
