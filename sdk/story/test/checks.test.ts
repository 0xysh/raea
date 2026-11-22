import { expect, test } from "bun:test";
import { analyze, checkStory, type Curve, type Audience, beatsOf } from "../src/index.ts";
import { launch } from "./fixture.ts";

test("the launch story passes every exact check", () => {
  const f = checkStory(launch, "30s");
  expect(f.filter((x) => x.level !== "note")).toEqual([]);
});

test("derived: freedom falls then rises; the opening asks a question the last beat answers", () => {
  const w = analyze(launch, "30s");
  const curves = w.resource<Record<string, Curve>>("Curves")!;
  expect(curves.freedom.levels).toEqual([0, 0, -3, -3, 0]);
  expect(curves.freedom.shape).toBe("fall-rise");
  const aud = beatsOf(w).map((b) => w.must<Audience>(b.e, "Audience"));
  expect(aud[0].open).toEqual(["recipeRuns"]);
  expect(aud[3].answered).toEqual(["recipeRuns"]);
  expect(aud[3].open).toEqual([]);
});

test("the same seed gives the same story", () => {
  const a = analyze(launch, "30s", 7).toJSON(), b = analyze(launch, "30s", 7).toJSON();
  expect(a).toEqual(b);
});
