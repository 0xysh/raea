//! Where hands go on a thing, and how they lie there. Hold points are found the way musthafa45/Ik-Object-Holding's
//! ColliderEdgeDetector finds them (MIT): from the thing's centre, out to the holder's left and right, a ray cast
//! back in meets the surface, and that is where each hand goes. Here the points are kept in the thing's own frame, so
//! they ride with it, and each carries the way the surface faces, which sets the palm. A thing narrower than a hand's
//! span is taken with one hand.

use glam::{Mat3, Quat, Vec3};

use crate::rig::{Side, Skeleton};
use crate::shape::Thing;

/// where one hand holds a thing, in the thing's own frame
#[derive(Clone, Copy, Debug)]
pub struct Hold {
    pub side: Side,
    pub at: Vec3,
    /// the way the surface faces there, outward
    pub normal: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub enum Grip {
    One(Hold),
    Two(Hold, Hold),
}

impl Grip {
    pub fn holds(&self) -> impl Iterator<Item = Hold> {
        let (a, b) = match *self {
            Grip::One(h) => (Some(h), None),
            Grip::Two(l, r) => (Some(l), Some(r)),
        };
        a.into_iter().chain(b)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HoldOptions {
    /// the hand that takes a thing small enough for one
    pub hand: Side,
    /// the widest thing one hand takes, metres across
    pub span: f32,
    /// how far below the thing's middle the hands go, metres (a heavy box is held low)
    pub lower: f32,
}

impl Default for HoldOptions {
    fn default() -> Self {
        HoldOptions { hand: Side::R, span: 0.11, lower: 0.0 }
    }
}

/// the level way from a holder to a thing (the way the holder faces to take it)
pub fn facing(thing: &Thing, holder: Vec3) -> Vec3 {
    let f = thing.at - holder;
    Vec3::new(f.x, 0.0, f.z).try_normalize().unwrap_or(Vec3::Z)
}

/// The holds a holder standing at `holder` takes on a thing: one on its left and one on its right, or one hand's.
pub fn find_holds(thing: &Thing, holder: Vec3, o: HoldOptions) -> Grip {
    // the body's left is its up crossed with its facing (y up, facing +z, x left)
    let left = Vec3::Y.cross(facing(thing, holder));
    let far = thing.solid.bound() + 0.1;
    let hold = |side: Side| {
        let out = left * side.sign();
        let from = thing.at + out * far - Vec3::Y * o.lower;
        // no surface on that line (the lowered ray passes under a ball): the thing's edge that way, as the original does
        let (at, normal) = thing.cast(from, -out, far).map_or((thing.at + out * thing.solid.bound(), out), |h| (h.at, h.normal));
        Hold { side, at: thing.local(at), normal: thing.turn.inverse() * normal }
    };
    let (l, r) = (hold(Side::L), hold(Side::R));
    if (l.at - r.at).length() <= o.span { Grip::One(if o.hand == Side::L { l } else { r }) } else { Grip::Two(l, r) }
}

/// A hand's frame at rest, in the body's axes: along (wrist to the middle knuckle), the way the palm faces, and where
/// the palm's skin is from the wrist.
#[derive(Clone, Copy, Debug)]
pub struct HandRest {
    pub along: Vec3,
    pub palm: Vec3,
    pub mid: Vec3,
}

pub fn hand_rest(skel: &Skeleton, s: Side) -> Option<HandRest> {
    let hand = skel.arm(s)?[2];
    let knuckle = |f: &str| Some(skel.joints[skel.finger(f, s)?[0]].head);
    let h = skel.joints[hand].head;
    let reach = knuckle("middle")? - h;
    let along = reach.normalize();
    let line = knuckle("index")? - knuckle("pinky")?;
    let across = (line - along * line.dot(along)).normalize();
    let palm = if s == Side::L { along.cross(across) } else { across.cross(along) };
    // the palm's middle: a little over half way to the knuckles, out to the skin
    Some(HandRest { along, palm, mid: reach * 0.55 + palm * 0.012 })
}

/// The wrist's place and the hand's turn that lay the palm on a hold: the palm against the surface, the fingers
/// pointing the way the holder faces, as near as the surface lets them.
pub fn wrist_for(skel: &Skeleton, thing: &Thing, hold: &Hold, facing: Vec3) -> Option<(Vec3, Quat)> {
    let rest = hand_rest(skel, hold.side)?;
    let (at, n) = (thing.world(hold.at), (thing.turn * hold.normal).normalize());
    let along = (facing - n * facing.dot(n)).try_normalize().unwrap_or_else(|| Vec3::Y.cross(n).normalize_or(Vec3::X));
    let frame = |a: Vec3, p: Vec3| Mat3::from_cols(a, p, a.cross(p));
    let turn = Quat::from_mat3(&(frame(along, -n) * frame(rest.along, rest.palm).transpose())).normalize();
    Some((at - turn * rest.mid, turn))
}

/// What the fingers of one hand found on a thing.
#[derive(Clone, Copy, Debug, Default)]
pub struct Touch {
    /// thumb, index, middle, ring, pinky: on the thing
    pub on: [bool; 5],
    /// how deep the worst finger is inside it at the pose it came with, metres (0: none is)
    pub inside: f32,
}

/// each joint's most bend from rest, degrees: fingers, then the thumb
const BEND: [[f32; 3]; 2] = [[80.0, 100.0, 70.0], [45.0, 55.0, 70.0]];
/// and how far each opens back past rest (the rest hand is a little curled, so a finger laid on a flat face opens)
const OPEN: [[f32; 3]; 2] = [[-40.0, -25.0, -20.0], [-30.0, -20.0, -20.0]];
/// how far a finger that meets nothing still closes, as a share of its most: a loose hand, not a fist
const LOOSE: f32 = 0.35;

/// The fingers of a hand curled onto a thing: each joint, knuckle first, bends toward the palm until its bone meets
/// the surface, so the finger wraps whatever shape is there. The hand is taken to be at `wrist` (its place and turn:
/// where its hold has it), not where the arm has got it to, so the shape the fingers close to is the hold's and
/// doesn't change as the hand comes in. `close` (0 to 1) is how far along that closing is, for the frames a hand
/// takes to shut. The pose's own finger turns are what it closes from.
pub fn curl(skel: &Skeleton, turns: &mut [Quat], wrist: (Vec3, Quat), s: Side, thing: &Thing, close: f32) -> Option<Touch> {
    let hand = skel.arm(s)?[2];
    let rest = hand_rest(skel, s)?;
    let mut touch = Touch::default();
    for (n, name) in crate::rig::FINGERS.iter().enumerate() {
        let js = skel.finger(name, s)?;
        let (thumb, thick) = (n == 0, if n == 0 { 0.010 } else { 0.008 });
        // the axis each joint bends about, at rest: the one that carries its bone toward the palm's side (the thumb's,
        // toward the space in front of the palm, where it meets the fingers)
        let axis = |k: usize| {
            let j = &skel.joints[js[k]];
            let bone = (j.tail - j.head).normalize();
            let toward = if thumb { (skel.joints[hand].head + rest.mid + rest.palm * 0.05 - j.head).normalize() } else { rest.palm };
            bone.cross(toward).normalize_or(Vec3::X)
        };
        let base = [turns[js[0]], turns[js[1]], turns[js[2]]];
        // the least clearance of bone k's skin with the finger bent by `bend` (radians a joint)
        let clear = |bend: [f32; 3], k: usize| {
            let (mut pp, mut pr) = wrist;
            let mut least = f32::MAX;
            for i in 0..=k {
                let j = &skel.joints[js[i]];
                let p = pp + pr * skel.offset(js[i]);
                let q = (pr * base[i] * Quat::from_axis_angle(axis(i), bend[i])).normalize();
                if i == k {
                    for along in [0.45, 0.75, 1.0] {
                        least = least.min(thing.distance(p + q * ((j.tail - j.head) * along)) - thick);
                    }
                }
                (pp, pr) = (p, q);
            }
            least
        };
        // each joint starts opened and bends until its bone meets the surface (the last step halved down to the
        // contact), so the finger wraps whatever is there; the joints after it wait opened, out of the way
        let open = OPEN[thumb as usize].map(f32::to_radians);
        let mut bend = open;
        for k in 0..3 {
            let most = BEND[thumb as usize][k].to_radians();
            let start = clear(bend, k);
            if start < 0.0 {
                // in the thing even opened: nothing a curl can fix; said, not hidden
                touch.inside = touch.inside.max(-start);
                touch.on[n] = true;
                continue;
            }
            let step = 3.0f32.to_radians();
            let (mut lo, mut hit) = (open[k], None);
            while lo < most {
                let hi = (lo + step).min(most);
                bend[k] = hi;
                if clear(bend, k) < 0.0 {
                    hit = Some(hi);
                    break;
                }
                lo = hi;
            }
            match hit {
                Some(mut hi) => {
                    for _ in 0..8 {
                        let mid = (lo + hi) / 2.0;
                        bend[k] = mid;
                        if clear(bend, k) < 0.0 { hi = mid } else { lo = mid }
                    }
                    bend[k] = lo;
                    touch.on[n] = true;
                }
                None => bend[k] = most * if touch.on[n] { 1.0 } else { LOOSE },
            }
        }
        for k in 0..3 {
            turns[js[k]] = (base[k] * Quat::from_axis_angle(axis(k), bend[k] * close.clamp(0.0, 1.0))).normalize();
        }
    }
    Some(touch)
}
