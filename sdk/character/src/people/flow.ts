// The people passing through (docs/character.feature CH52): a street carries more than the people who live on it.
// Each sidewalk holds walkers by its length, its busyness at its busiest hour and the hour's share of that; each is a
// person (a role drawn by the mix) walking the line at their own pace one way or the other, keeping to their right.
// Where a walker is at any minute is computed, not stepped, so any frame of a shot can be drawn alone.
import { walkSpeed } from "./day";
import { makePerson, type Folk } from "./persona";
import { hash, rng } from "./rng";
import type { Minute, Person, Place, Role, V3 } from "./types";

/** A sidewalk's centreline and how busy it is: walkers a metre at its busiest hour. */
export interface Walk {
  id: string;
  path: V3[];
  perMetre: number;
  /** its width (metres, 3): walkers keep to their right within it */
  width?: number;
}

/** A city street's walkers by hour, against its busiest (5 pm): the morning and evening rush, the noon errands, the
 *  empty night. Index h is h:00; a minute between is read between. */
export const STREET_HOURS = [0.03, 0.02, 0.02, 0.02, 0.04, 0.1, 0.3, 0.6, 0.7, 0.55, 0.6, 0.75, 0.9, 0.85, 0.75, 0.8, 0.9, 1, 0.85, 0.5, 0.3, 0.2, 0.1, 0.05];

export interface Passer {
  id: string;
  walk: string;
  at: V3;
  facing: number;
  person: Person;
}

export interface FlowInput {
  roles: Role[];
  folk: Folk;
  /** the roles' shares among people on the street */
  mix: Record<string, number>;
  seed: number | string;
  /** the hour profile (STREET_HOURS) */
  hours?: number[];
  /** how cold (0 mild to 1 bitter): pace */
  cold?: number;
  /** more or fewer than the walks' own numbers (1): a shot's busier street */
  busy?: number;
  /** only walks with a point within `within` metres of `near` */
  near?: V3;
  within?: number;
}

const people = new Map<string, Person>();

/** The share of the busiest hour at a minute of the day. */
export function busyAt(m: Minute, hours = STREET_HOURS): number {
  const h = (((m / 60) % 24) + 24) % 24, i = Math.floor(h), k = h - i;
  return hours[i] * (1 - k) + hours[(i + 1) % 24] * k;
}

/** Everyone walking the given sidewalks at minute `m` (fractional minutes are seconds): who, where, which way. */
export function passersBy(walks: Walk[], m: Minute, f: FlowInput): Passer[] {
  const busy = busyAt(m, f.hours), out: Passer[] = [];
  const roles = new Map(f.roles.map((r) => [r.id, r]));
  for (const w of walks) {
    if (f.near && f.within !== undefined && !w.path.some((p) => Math.hypot(p[0] - f.near![0], p[2] - f.near![2]) <= f.within!)) continue;
    const seg = w.path.slice(1).map((q, i) => Math.hypot(q[0] - w.path[i][0], q[2] - w.path[i][2]));
    const L = seg.reduce((a, b) => a + b, 0);
    if (L < 2) continue;
    // the line's slots at its busiest; a slot walks when its own draw is under the hour's share, so the walkers
    // thin out and come back as the day goes without anyone jumping
    const slots = Math.round(L * w.perMetre * (f.busy ?? 1));
    for (let k = 0; k < slots; k++) {
      const id = `${w.id}:${k}`, r = rng(`${f.seed}-${id}`);
      if (r.next() >= busy) continue;
      let p = people.get(`${f.seed}-${id}`);
      if (!p) {
        // who they are from a draw of their own, so a walker made earlier walks the same way
        const role = roles.get(rng(`${f.seed}-${id}-who`).weighted(f.mix))!;
        const home: Place = { id: `passing-${w.id}`, kind: "street", at: w.path[0] };
        p = makePerson(id, role, f.folk, home, hash(`${f.seed}-${id}`));
        people.set(`${f.seed}-${id}`, p);
      }
      const dir = r.next() < 0.5 ? 1 : -1, phase = r.next();
      const v = walkSpeed(p, f.cold ?? 0.5) * (0.9 + 0.2 * r.next());
      // along the line, wrapping at its ends (a walker leaves at one end as another comes in at the other)
      let s = (((phase * L + dir * v * m * 60) % L) + L) % L, i = 0;
      while (i < seg.length - 1 && s > seg[i]) s -= seg[i++];
      const A = w.path[i], B = w.path[i + 1], u = seg[i] ? s / seg[i] : 0;
      const tx = (B[0] - A[0]) / (seg[i] || 1), tz = (B[2] - A[2]) / (seg[i] || 1);
      // keep right: off the centreline by a quarter of the width or so, to the walker's right
      const off = dir * (w.width ?? 3) * (0.12 + 0.2 * r.next());
      const at: V3 = [A[0] + (B[0] - A[0]) * u - tz * off, A[1] + (B[1] - A[1]) * u, A[2] + (B[2] - A[2]) * u + tx * off];
      out.push({ id, walk: w.id, at, facing: Math.atan2(dir * tx, dir * tz), person: p });
    }
  }
  return out;
}
