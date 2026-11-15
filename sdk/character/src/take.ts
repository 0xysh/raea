// @raea/character/take: a character takes a thing with its hands, with no capture of the action (docs/hands.feature):
// it reaches, grabs with one hand or two, lifts, carries, and puts the thing down or throws it. The engine works each
// frame out (raea-sys-grab, the native call character.take); this asks it and hands back the pose to give
// <Character> and where to draw the thing.
//
//   const box = { box: [0.16, 0.1, 0.12], at: [0, 1.05, 0.42] } as const;          // half sizes, metres, the body's frame
//   const now = take(kid, () => rest(kid), { thing: box, start: 0.5, throw: { at: 2.5, velocity: [0, 2, 3] } }, frame / fps);
//   <Character body="kid" pose={now.pose} />
//   <Mesh3d transform={Transform.fromXyz(...now.thing.at).withRotation(now.thing.turn)}>...</Mesh3d>
//
// `base` is the body's own motion, its pose at any time (a clip, a walk, a stand): the take only moves what the hands
// need and says how much (`moved`, degrees) and how far a wrist ended from its hold (`miss`, metres). A thing out of
// reach is missed and said so: `stand` is where to walk the body to first. A low thing (on the floor) is taken by
// bending and squatting, the feet staying where they stand. Frame-pure: any frame can be asked for.
import { native } from "@raea/app/native";
import type { Pose, Quat, Skeleton } from "./character/core";

type Vec3 = [number, number, number];

/** A thing to take: one shape (a ball's radius; a box's half sizes; a capsule's or a cylinder's half length along its
 * own y, then radius), where its centre is and how it is turned, in the frame the body is posed in. */
export interface TakeThing {
  ball?: number;
  box?: Vec3;
  capsule?: [number, number];
  cylinder?: [number, number];
  at: Vec3;
  turn?: Quat;
}

export interface TakeSpec {
  thing: TakeThing;
  /** when the hands leave the body's own motion, seconds */
  start: number;
  /** when they close on the thing (0.6 s after start) and when it has reached the carry (0.5 s after that) */
  grab?: number;
  carried?: number;
  /** where the thing rides, from the chest: x to the body's left, y up, z the way it faces */
  carry?: Vec3;
  /** the hand that takes a thing small enough for one (the right) */
  hand?: "L" | "R";
  /** how far below the thing's middle the hands go, metres */
  lower?: number;
  /** a thing held one way brings its holds, in its own frame: where a palm goes (`at`, on its surface), the way the
   * surface faces there (`normal`), optionally the way the fingers point (`along`) and the hand's shape as
   * handShape's first numbers (`shape`: about -1.2 is a loose fist round a handle). None: holds are found on its shape */
  holds?: { side: "L" | "R"; at: Vec3; normal: Vec3; along?: Vec3; shape?: number[] }[];
  /** what is done with the thing once carried: where it is at times (`carry`, from the chest as above) and how it
   * is tipped from how it is carried (`tilt`, in the body's axes); the hands follow. A drink is a key at the mouth */
  path?: { t: number; carry: Vec3; tilt?: Quat }[];
  /** how heavy it is, kilograms: held lower and closer, lifted slower, the trunk leaning back against it */
  weight?: number;
  /** the scene's solids the hands keep out of (a table's top, a shelf) */
  avoid?: TakeThing[];
  /** set down: moved to `place` from `at`, standing there from `down` */
  put?: { at: number; down: number; place: Vec3 };
  /** let go at `at` with a velocity, metres a second; it falls to `floor` (0) */
  throw?: { at: number; velocity: Vec3; floor?: number };
}

export interface Taken {
  pose: Pose;
  thing: { at: Vec3; turn: Quat };
  phase: "Before" | "Reach" | "Lift" | "Carry" | "Release" | "After";
  hands: 1 | 2;
  /** the worst a wrist ended from where its hold asks, metres */
  miss: number;
  /** the most any joint was turned from the body's own motion, degrees */
  moved: number;
  /** a wrist was held at the most it bends (70 degrees), so its palm isn't square on its hold */
  limited: boolean;
  /** where the body would stand to take the thing, and the turn about the vertical that faces it: walk it there first
   * when the thing is out of reach */
  stand: { at: Vec3; yaw: number };
}

/** The body and the thing at `t` seconds of a take. */
export function take(skel: Skeleton, base: (t: number) => Pose, spec: TakeSpec, t: number): Taken {
  const grab = spec.grab ?? spec.start + 0.6;
  // the moments the take reads the body's own motion at: now, the start, the grab, the release
  const times = [t, spec.start, grab, spec.put?.at, spec.put?.down, spec.throw?.at].filter((x): x is number => x !== undefined);
  const poses = [...new Set(times)].map((at) => {
    const p = base(at);
    return { t: at, root: p.root, turns: skel.joints.map((_, i) => p.turns[i] ?? ([0, 0, 0, 1] as Quat)) };
  });
  const joints = skel.joints.map((j) => ({ name: j.name, parent: j.parent, head: j.head, tail: j.tail }));
  return native("character.take", { ...spec, joints, t, base: poses });
}

/** A point of the body seen somewhere: on a joint (by name), `offset` from its head in the rest pose's axes (none: the
 * joint itself), seen at `at`. */
export interface Mark {
  joint: string;
  offset?: Vec3;
  at: Vec3;
  weight?: number;
}

/** A pose fitted to where points of the body were seen, so motion that arrives as positions (a capture, a tracker,
 * another rig's joints) plays on this body; `miss` is the marks' root-mean-square distance from where they were seen,
 * metres. It starts from `from` (the rest pose), and a turn the marks say nothing about stays as `from` has it. */
export function fitPose(skel: Skeleton, marks: Mark[], from?: Pose, passes?: number): { pose: Pose; miss: number } {
  const joints = skel.joints.map((j) => ({ name: j.name, parent: j.parent, head: j.head, tail: j.tail }));
  const start = from && { root: from.root, turns: skel.joints.map((_, i) => from.turns[i] ?? ([0, 0, 0, 1] as Quat)) };
  return native("character.fit", { joints, marks, from: start, passes });
}

/** A hand shaped by a model of real hand poses (SOMA-X's hand prior): `z` are its numbers, each in its own standard
 * deviations (none: the average relaxed hand; the first opens the hand when positive and closes it when negative).
 * The rest of `from` (the rest pose) is kept. `odd` says how far the hand is from the hands people make (0: the
 * average hand; the rig's own rest hand reads about 2). With no `z`, `from`'s own hand is read. */
export function handShape(skel: Skeleton, side: "L" | "R", z?: number[], from?: Pose): { pose: Pose; z: number[]; odd: number } {
  const joints = skel.joints.map((j) => ({ name: j.name, parent: j.parent, head: j.head, tail: j.tail }));
  const pose = from && { root: from.root, turns: skel.joints.map((_, i) => from.turns[i] ?? ([0, 0, 0, 1] as Quat)) };
  return native("character.hand", { joints, side, z, pose });
}
