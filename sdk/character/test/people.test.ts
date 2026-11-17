// raea/people (src/people.ts, docs/character.feature CH50-CH53): people drawn from roles, their days, where they are
// at a minute, the levels of detail and how they meet an event.
import { expect, test } from "bun:test";
import { census, cityDay, dayOf, levelAt, populate, react, snapshot, whereAt, type Folk, type Place, type Role } from "../src/people";

const folk: Folk = {
  names: { given: { f: ["Anna", "Martha", "Frieda"], m: ["Karl", "Otto", "Paul"] }, family: ["Schulz", "Müller", "Krüger"] },
  irises: { blue: 2, brown: 1 }, build: [-0.8, 0.6],
  facialHair: { young: { none: 3, moustache: 1 }, middle: { none: 1, moustache: 2 }, old: { moustache: 2, "beard-full": 1 } },
};
const places: Place[] = [
  ...Array.from({ length: 20 }, (_, i) => ({ id: `h${i}`, kind: "home", at: [i * 30, 0, 0] as [number, number, number], holds: 4 })),
  { id: "bakery", kind: "bakery", at: [100, 0, 40], opens: [6 * 60, 19 * 60] },
  { id: "bakery2", kind: "bakery", at: [500, 0, 40], opens: [6 * 60, 19 * 60] },
  { id: "works", kind: "munitions", at: [2500, 0, 1800], holds: 200 },
  { id: "post1", kind: "post", at: [300, 0, 10] },
  { id: "post2", kind: "post", at: [400, 0, -60] },
];
const roles: Role[] = [
  {
    id: "housewife", title: "Housewife", women: 1, ages: [22, 70], dress: { "winter-coat": 2, shawl: 1 }, carry: { basket: 0.8 },
    traits: { conscientiousness: 0.6 },
    day: [{ what: "queue for bread", at: "bakery", from: 6 * 60, minutes: 120, jitter: 30, acts: ["wait", "look at the shop door"] },
      { what: "market", at: "bakery", from: 10 * 60 + 30, minutes: 45, chance: 0.5 }],
    habits: [{ id: "church", act: "church", from: 9 * 60, to: 11 * 60, at: "home", minutes: 60, chance: 0.9, days: [0] }],
  },
  {
    id: "munitions-worker", title: "Munitions worker", women: 0.8, ages: [16, 45], dress: { overall: 1 }, work: { at: "munitions" },
    modes: { walk: 1, tram: 3 }, day: [{ what: "shift", at: "work", from: 6 * 60, to: 18 * 60, jitter: 5 }],
  },
  {
    id: "sentry", title: "Landsturm sentry", women: 0, ages: [38, 48], rank: "landsturmmann", dress: { "greatcoat-m15": 1 }, gait: "march",
    traits: { conscientiousness: 0.75 },
    day: [{ what: "patrol", at: "post", from: 8 * 60, to: 16 * 60, round: { stops: 6, dwell: 50 }, jitter: 2 }],
  },
];
const mix = { housewife: 50, "munitions-worker": 20, sentry: 2 };

test("the same seed draws the same people, a different one others", () => {
  const a = populate({ roles, places, folk, mix, seed: "leipziger" }), b = populate({ roles, places, folk, mix, seed: "leipziger" });
  const c = populate({ roles, places, folk, mix, seed: "jerusalemer" });
  expect(a.length).toBe(72);
  expect(JSON.stringify(a)).toBe(JSON.stringify(b));
  expect(a.map((p) => p.name).join()).not.toBe(c.map((p) => p.name).join());
});

test("a role's sex, ages, rank and gait hold for everyone drawn from it", () => {
  const ps = populate({ roles, places, folk, mix, seed: 3 });
  for (const p of ps.filter((x) => x.role === "housewife")) { expect(p.sex).toBe("f"); expect(p.age).toBeGreaterThanOrEqual(22); expect(p.age).toBeLessThanOrEqual(70); }
  for (const p of ps.filter((x) => x.role === "sentry")) { expect(p.rank).toBe("landsturmmann"); expect(p.gait).toBe("march"); expect(p.sex).toBe("m"); }
  // workers have a workplace; faces are crowd faces with a seed of their own
  expect(ps.filter((x) => x.role === "munitions-worker").every((p) => p.work === "works")).toBe(true);
  expect(new Set(ps.map((p) => (p.face.kind === "crowd" ? p.face.seed : 0))).size).toBe(ps.length);
});

test("a day covers 00:00 to 24:00 with no gaps, and a trip joins every change of place", () => {
  const ps = populate({ roles, places, folk, mix, seed: 5 });
  const d = cityDay(ps, roles, places, { key: "1917-12-03", weekday: 1 });
  for (const p of ps) {
    const day = d.days.get(p.id)!;
    expect(day[0].from).toBe(0);
    expect(day[day.length - 1].to).toBe(1440);
    for (let i = 1; i < day.length; i++) expect(day[i].from).toBe(day[i - 1].to);
    let at = p.home;
    for (const a of day) {
      if (a.kind === "trip") { expect(a.leave).toBe(at); at = a.arrive; } else expect(a.at).toBe(at);
    }
  }
});

test("a far commute in the cold goes by tram; a short errand is walked", () => {
  const ps = populate({ roles, places, folk, mix, seed: 9 });
  const d = cityDay(ps, roles, places, { key: "1917-12-04", weekday: 2 }, undefined, 0.9);
  const trips = [...d.days.values()].flat().filter((a) => a.kind === "trip");
  const far = trips.filter((t) => t.kind === "trip" && t.metres > 2500), near = trips.filter((t) => t.kind === "trip" && t.metres < 900);
  expect(far.filter((t) => t.kind === "trip" && t.mode === "tram").length / far.length).toBeGreaterThan(0.6);
  expect(near.every((t) => t.kind === "trip" && t.mode === "walk")).toBe(true);
});

test("punctual people start their shift closer to its time than careless ones", () => {
  const ps = populate({ roles, places, folk, mix: { "munitions-worker": 200 }, seed: 11 });
  const d = cityDay(ps, roles, places, { key: "1917-12-05", weekday: 3 });
  const late = (p: (typeof ps)[number]) => Math.abs(d.days.get(p.id)!.find((a) => a.kind === "stay" && a.what === "shift")!.from - 360);
  const exact = ps.filter((p) => p.traits.conscientiousness > 0.65), loose = ps.filter((p) => p.traits.conscientiousness < 0.35);
  const mean = (xs: number[]) => xs.reduce((a, b) => a + b, 0) / xs.length;
  expect(mean(exact.map(late))).toBeLessThan(mean(loose.map(late)));
});

test("a patrol is a round of posts; Sunday brings church", () => {
  const ps = populate({ roles, places, folk, mix, seed: 13 });
  const sun = cityDay(ps, roles, places, { key: "1917-12-02", weekday: 0 });
  const sentry = ps.find((p) => p.role === "sentry")!;
  const posts = sun.days.get(sentry.id)!.filter((a) => a.kind === "stay" && a.what === "patrol");
  expect(posts.length).toBe(6);
  const church = ps.filter((p) => p.habits.includes("church")).filter((p) => sun.days.get(p.id)!.some((a) => a.kind === "stay" && a.what === "church"));
  expect(church.length).toBeGreaterThan(0);
  const mon = cityDay(ps, roles, places, { key: "1917-12-03", weekday: 1 });
  expect([...mon.days.values()].flat().some((a) => a.kind === "stay" && a.what === "church")).toBe(false);
});

test("whereAt walks a trip's path; snapshot and census count everyone", () => {
  const ps = populate({ roles, places, folk, mix, seed: 17 });
  const d = cityDay(ps, roles, places, { key: "1917-12-03", weekday: 1 });
  const day = d.days.get(ps.find((p) => p.role === "munitions-worker")!.id)!;
  const trip = day.find((a) => a.kind === "trip")!;
  const mid = whereAt(day, d.places, (trip.from + trip.to) / 2);
  expect(mid.activity).toBe(trip);
  const s = snapshot(d, 7 * 60, [0, 0, 0], { near: 150, middle: 1500 });
  expect(s.length).toBe(ps.length);
  expect(s.some((x) => x.level === "far")).toBe(true);
  const c = census(d);
  for (let h = 0; h < 24; h++) expect([...c.values()].reduce((a, v) => a + v[h], 0)).toBe(ps.length);
});

test("levels change with hysteresis at their edges", () => {
  const range = { near: 150, middle: 1500 };
  expect(levelAt(140, range)).toBe("near");
  expect(levelAt(160, range, "near")).toBe("near");
  expect(levelAt(170, range, "near")).toBe("middle");
  expect(levelAt(140, range, "middle")).toBe("middle");
  expect(levelAt(130, range, "middle")).toBe("near");
});

test("a shock underfoot turns eyes to the ground; the nervous flinch, the far feel nothing", () => {
  const ps = populate({ roles, places, folk, mix, seed: 19 });
  const e = { kind: "tremor", at: [0, 0, 0] as [number, number, number], strength: 1 };
  const nervous = ps.find((p) => p.traits.neuroticism > 0.65)!;
  const near = react(nervous, e, [5, 0, 0]);
  expect(near.acts[0]).toBe("look down at the ground");
  expect(near.acts.some((a) => a === "flinch and duck" || a === "cover the ears")).toBe(true);
  expect(react(nervous, e, [5000, 0, 0]).acts.length).toBe(0);
});

test("dayOf is the same for the same person and date", () => {
  const ps = populate({ roles, places, folk, mix, seed: 23 });
  const pm = new Map(places.map((p) => [p.id, p])), byKind = new Map<string, Place[]>();
  for (const p of places) byKind.set(p.kind, [...(byKind.get(p.kind) ?? []), p]);
  const inp = { person: ps[0], role: roles[0], date: { key: "1918-01-15", weekday: 2 }, places: pm, byKind };
  expect(JSON.stringify(dayOf(inp))).toBe(JSON.stringify(dayOf(inp)));
});

test("a name the place rejects (a real resident's) is never given", () => {
  const avoid = (_g: string, f: string) => f === "Müller" || f === "Schulz";
  const ps = populate({ roles, places, folk: { ...folk, avoid }, mix, seed: 29 });
  expect(ps.every((p) => !p.name.endsWith(" Müller") && !p.name.endsWith(" Schulz"))).toBe(true);
  // every family name rejected: named by role and number instead
  const none = populate({ roles, places, folk: { ...folk, avoid: () => true }, mix: { sentry: 1 }, seed: 31 });
  expect(none[0].name).toBe("Landsturm sentry p0");
});

test("people inside are counted, not drawn; a queue stands in a line along the front, first come nearest", () => {
  const inside = places.map((p) => (p.kind === "home" || p.kind === "munitions" ? { ...p, inside: true } : p.kind === "bakery" ? { ...p, inside: true, facing: 0 } : p));
  const queueRoles = roles.map((r) => (r.id === "housewife" ? { ...r, day: r.day.map((b) => (b.what === "queue for bread" ? { ...b, outside: true } : b)) } : r));
  const ps = populate({ roles: queueRoles, places: inside, folk, mix, seed: 37 });
  const d = cityDay(ps, queueRoles, inside, { key: "1917-12-03", weekday: 1 });
  const s = snapshot(d, 2 * 60, [0, 0, 0], { near: 150, middle: 1500 });
  expect(s.every((x) => x.level === "inside")).toBe(true);
  const q = snapshot(d, 7 * 60 + 15, [0, 0, 0], { near: 150, middle: 1500 }).filter((x) => x.doing === "queue for bread");
  expect(q.length).toBeGreaterThan(5);
  expect(q.every((x) => x.level !== "inside")).toBe(true);
  // no two in the queue on the same spot; each 0.65 m from the next along the front
  for (const door of new Set(q.map((x) => (x.activity.kind === "stay" ? x.activity.at : "")))) {
    const at = q.filter((x) => x.activity.kind === "stay" && x.activity.at === door).map((x) => x.at[0]).sort((a, b) => a - b);
    for (let k = 1; k < at.length; k++) expect(at[k] - at[k - 1]).toBeCloseTo(0.65, 5);
  }
});
