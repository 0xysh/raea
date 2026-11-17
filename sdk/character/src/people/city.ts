// A city's people at three levels of detail (docs/character.feature CH52): every person exists as data and a day;
// near the camera they are skinned characters performing their acts, further out agents moved along their trips with
// no skinning, and beyond that only counts (how many are at each kind of place in each hour). A person moves between
// levels by id, keeping their day.
import { dayOf, whereAt, type DayInput } from "./day";
import { makePerson, type Folk } from "./persona";
import { hash, rng } from "./rng";
import type { Activity, Level, Person, Place, Role, Router, Traits, V3 } from "./types";

/** How many of each role a place holds (or its share of the population, when `people` is given). */
export type Mix = Record<string, number>;

export interface CityInput {
  roles: Role[];
  places: Place[];
  folk: Folk;
  /** roles by count, or by share when `people` is given */
  mix: Mix;
  people?: number;
  seed: number | string;
}

/** Everyone in the place: each a role drawn by the mix, a home (households filled in order of a shuffle) and, for a
 *  role that works somewhere, a workplace of its kind within reach, filled up to what it holds. */
export function populate(c: CityInput): Person[] {
  const r = rng(c.seed);
  const roles = new Map(c.roles.map((x) => [x.id, x]));
  const counts: [string, number][] = c.people
    ? Object.entries(c.mix).map(([k, s]) => [k, Math.round((s / Object.values(c.mix).reduce((a, b) => a + b, 0)) * c.people!)])
    : Object.entries(c.mix);
  const homes = c.places.filter((p) => p.kind === "home");
  if (homes.length === 0) throw new Error("populate: no places of kind 'home'");
  const slots: Place[] = [];
  for (const h of homes) for (let k = 0; k < (h.holds ?? 3); k++) slots.push(h);
  for (let i = slots.length - 1; i > 0; i--) { const j = r.int(0, i); [slots[i], slots[j]] = [slots[j], slots[i]]; }
  const used = new Map<string, number>();
  const out: Person[] = [];
  let n = 0;
  for (const [rid, count] of counts) {
    const role = roles.get(rid);
    if (!role) throw new Error(`populate: the mix names a role no one defined: ${rid}`);
    for (let k = 0; k < count; k++, n++) {
      const home = slots[n % slots.length];
      let work: Place | undefined;
      if (role.work) {
        const cand = c.places.filter((p) => p.kind === role.work!.at && (used.get(p.id) ?? 0) < (p.holds ?? Infinity)
          && Math.hypot(p.at[0] - home.at[0], p.at[2] - home.at[2]) <= (role.work!.within ?? Infinity));
        work = cand.length ? cand[r.int(0, cand.length - 1)] : undefined;
        if (work) used.set(work.id, (used.get(work.id) ?? 0) + 1);
      }
      out.push(makePerson(`p${n}`, role, c.folk, home, hash(`${c.seed}-${n}`), work));
    }
  }
  return out;
}

/** The level a person is simulated at, from their distance to the camera; `was` gives hysteresis (a tenth of each
 *  range) so a person at the edge doesn't flip each frame. */
export function levelAt(dist: number, range: { near: number; middle: number }, was?: Level): Level {
  const h = 0.1;
  const near = was === "near" ? range.near * (1 + h) : range.near * (was ? 1 - h : 1);
  const middle = was === "far" ? range.middle * (1 - h) : range.middle * (was === "middle" || was === "near" ? 1 + h : 1);
  return dist <= near ? "near" : dist <= middle ? "middle" : "far";
}

export interface Day {
  people: Person[];
  days: Map<string, Activity[]>;
  places: Map<string, Place>;
}

/** Every person's day for a date. */
export function cityDay(people: Person[], roles: Role[], places: Place[], date: DayInput["date"], router?: Router, cold?: number): Day {
  const pm = new Map(places.map((p) => [p.id, p])), byKind = new Map<string, Place[]>();
  for (const p of places) byKind.set(p.kind, [...(byKind.get(p.kind) ?? []), p]);
  const rm = new Map(roles.map((x) => [x.id, x]));
  const days = new Map<string, Activity[]>();
  for (const person of people) days.set(person.id, dayOf({ person, role: rm.get(person.role)!, date, places: pm, byKind, router, cold }));
  return { people, days, places: pm };
}

/** Spacing of a queue along a front (metres) and how far out from the wall it stands. */
export const QUEUE_GAP = 0.65, QUEUE_OUT = 1.1;

/** Everyone at a minute: where they are, what they are doing, and the level they are at for a camera. A person inside
 *  a place (`Place.inside`, on a block not `outside`) is "inside": counted, never drawn. People out at the same door
 *  stand in a queue along its front, first come nearest the door, facing it. */
export function snapshot(d: Day, m: number, camera: V3, range: { near: number; middle: number }, was?: Map<string, Level>) {
  const rows = d.people.map((p) => ({ p, w: whereAt(d.days.get(p.id)!, d.places, m) }));
  const lines = new Map<string, { i: number; since: number }[]>();
  rows.forEach(({ w }, i) => {
    const a = w.activity;
    if (a.kind === "stay" && a.outside) lines.set(a.at, [...(lines.get(a.at) ?? []), { i, since: a.from }]);
  });
  const placed = new Map<number, { at: V3; facing: number }>();
  for (const [id, line] of lines) {
    const pl = d.places.get(id)!, f = pl.facing ?? 0;
    // the front runs across the facing; the queue runs along it from the door, a step out from the wall
    const out: V3 = [Math.sin(f), 0, Math.cos(f)], along: V3 = [Math.cos(f), 0, -Math.sin(f)];
    line.sort((a, b) => a.since - b.since || a.i - b.i).forEach((q, k) => {
      const s = (k + 1) * QUEUE_GAP;
      placed.set(q.i, { at: [pl.at[0] + out[0] * QUEUE_OUT + along[0] * s, pl.at[1], pl.at[2] + out[2] * QUEUE_OUT + along[2] * s], facing: f + Math.PI / 2 });
    });
  }
  return rows.map(({ p, w }, i) => {
    const q = placed.get(i), at = q?.at ?? w.at, facing = q?.facing ?? w.facing;
    const a = w.activity;
    const hidden = a.kind === "stay" && !a.outside && d.places.get(a.at)?.inside === true;
    const dist = Math.hypot(at[0] - camera[0], at[2] - camera[2]);
    return { id: p.id, ...w, at, facing, level: hidden ? ("inside" as const) : levelAt(dist, range, was?.get(p.id)) };
  });
}

/** The far level: how many people are at each kind of place (or on the move, by mode) in each hour of the day. */
export function census(d: Day): Map<string, number[]> {
  const out = new Map<string, number[]>();
  const add = (k: string, h: number) => { const a = out.get(k) ?? new Array(24).fill(0); a[h]++; out.set(k, a); };
  for (const p of d.people) {
    const day = d.days.get(p.id)!;
    for (let h = 0; h < 24; h++) {
      const a = day.find((x) => x.from <= h * 60 + 30 && h * 60 + 30 < x.to) ?? day[day.length - 1];
      add(a.kind === "trip" ? `moving:${a.mode}` : d.places.get(a.at)?.kind ?? "unknown", h);
    }
  }
  return out;
}

/** Something everyone near feels: a gun's report through the ground, a shout, a horse bolting. */
export interface Event { kind: string; at: V3; strength: number }

/** How a person meets an event, in the stage's words: a look toward it (or at the ground, for a shock underfoot),
 *  a flinch or a stop, a run for the nervous; how long they freeze. Nerves and age decide; distance softens it. */
export function react(p: Person, e: Event, at: V3, seed = 0): { acts: string[]; freeze: number } {
  const r = rng(`${p.seed}-${e.kind}-${seed}`);
  const d = Math.hypot(e.at[0] - at[0], e.at[2] - at[2]);
  const felt = e.strength / (1 + d / 50);
  if (felt < 0.08) return { acts: [], freeze: 0 };
  const t: Traits = p.traits;
  const acts: string[] = [];
  const under = e.kind === "tremor" || e.kind === "underground";
  acts.push(under ? "look down at the ground" : "look toward the sound");
  if (felt > 0.5 && t.neuroticism > 0.6) acts.push(r.chance(0.5) ? "flinch and duck" : "cover the ears");
  else if (felt > 0.25) acts.push("stop dead");
  if (felt > 0.7 && t.neuroticism > 0.75 && p.age < 60) acts.push("hurry away");
  else if (t.extraversion > 0.65 && felt > 0.2) acts.push("turn to the nearest person");
  return { acts, freeze: Math.round((0.6 + felt * (0.8 + t.neuroticism)) * 10) / 10 };
}
