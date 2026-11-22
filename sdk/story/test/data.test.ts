import { expect, test } from "bun:test";
import { beatSchema, checkStory, fromData, toData } from "../src/index.ts";
import { launch } from "./fixture.ts";

test("a story round-trips through JSON and reads back clean", () => {
  const j = toData(launch);
  const { story, errors } = fromData(j);
  expect(errors).toEqual([]);
  expect(checkStory(story!, "30s").filter((f) => f.level === "error")).toEqual(checkStory(launch, "30s").filter((f) => f.level === "error"));
});

test("fromData names a bad reference and a cause out of order", () => {
  const j = toData(launch) as any;
  j.tellings["30s"][1].shows.push("nope");
  j.world.week.because = "ships";
  const msgs = fromData(j).errors.map((e) => e.at);
  expect(msgs).toContain("tellings.30s[1].shows[2]");
  expect(msgs).toContain("world.week.because");
});

test("a beat's schema holds the story's own keys", () => {
  const s = beatSchema(launch) as any;
  expect(s.properties.shows.items.enum).toEqual(["week", "doubles", "tries", "ships"]);
  expect(s.properties.pays.anyOf[0].enum).toEqual(["how"]);
});
