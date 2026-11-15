//! Taking a thing: reach, grab, lift, carry, then put it down or throw it. The whole action is a function of time
//! (docs/character.md, principle 9): any frame can be asked for, and a render and live play agree. The body's own
//! motion comes in as `base` (a clip, a walk, a stand); this only moves what the hands need, and says how much.
//!
//! The stages are musthafa45/Ik-Object-Holding's (MIT): hands eased onto the hold points, the thing drawn to a carry
//! place in front of the chest, both let go on a drop or a throw. There the easing is a per-frame lerp, which depends
//! on the frame rate and on every frame before; here each ease is a curve of time.

use glam::{Quat, Vec3};

use crate::arm::{ArmOptions, forearm_twist, reach};
use crate::hand::{Grip, Touch, curl, facing, wrist_for};
use crate::rig::{Pose, Posed, Skeleton};
use crate::shape::Thing;

#[derive(Clone, Copy, Debug)]
pub enum Release {
    /// set down: from `at` the thing is moved to `place`, standing there from `down` on, and the hands leave it
    Put { at: f32, down: f32, place: Vec3 },
    /// let go at `at` with a velocity, metres a second (none: a drop); it falls to `floor`
    Throw { at: f32, velocity: Vec3, floor: f32 },
}

#[derive(Clone, Debug)]
pub struct Take {
    /// the thing where it stands before it is taken
    pub thing: Thing,
    pub grip: Grip,
    /// when the hands leave the body's own motion, when they close on the thing, and when it has reached the carry
    pub start: f32,
    pub grab: f32,
    pub carried: f32,
    /// where the thing's centre rides, from the chest: x to the body's left, y up, z the way it faces
    pub carry: Vec3,
    pub release: Option<Release>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Before,
    Reach,
    Lift,
    Carry,
    Release,
    After,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub pose: Pose,
    pub thing: Thing,
    pub phase: Phase,
    /// the worst a wrist ended from where its hold asks, metres
    pub miss: f32,
    /// the most any joint was turned from the body's own motion, degrees
    pub moved: f32,
    pub touch: [Option<Touch>; 2],
}

const GRAVITY: f32 = 9.81;
/// how long the hands take to go back to the body's own motion after letting go, seconds
const AWAY: f32 = 0.4;
/// how long the fingers take to shut, ending at the grab, seconds
const SHUT: f32 = 0.25;

fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}

fn span(t: f32, a: f32, b: f32) -> f32 {
    if b <= a { (t >= b) as u8 as f32 } else { ((t - a) / (b - a)).clamp(0.0, 1.0) }
}

/// the turn about the vertical that the body faces by
fn yaw(w: &Posed) -> Quat {
    let f = w.rot[0] * Vec3::Z;
    Quat::from_rotation_y(f.x.atan2(f.z))
}

impl Take {
    /// A take with the usual timing: reach for 0.6 s from `start`, lift for 0.5 s, carried in front of the belly (a
    /// one-handed thing, off to its hand's side).
    pub fn new(thing: Thing, grip: Grip, start: f32) -> Self {
        let carry = match grip {
            Grip::One(h) => Vec3::new(0.2 * h.side.sign(), -0.22, 0.3),
            Grip::Two(..) => Vec3::new(0.0, -0.2, 0.32 + thing.solid.bound() * 0.3),
        };
        Take { thing, grip, start, grab: start + 0.6, carried: start + 1.1, carry, release: None }
    }

    /// where the thing rides at a moment of the body's motion: off the chest, turned only with the body's facing, so
    /// what stood upright is carried upright
    fn riding(&self, skel: &Skeleton, base: &dyn Fn(f32) -> Pose, t: f32) -> Thing {
        let (now, then) = (skel.fk(&base(t)), skel.fk(&base(self.grab)));
        let chest = skel.joint("spine_03").unwrap_or(0);
        let (y, y0) = (yaw(&now), yaw(&then));
        self.thing.moved(now.pos[chest] + y * self.carry, (y * y0.inverse() * self.thing.turn).normalize())
    }

    /// where the thing is at `t`, and whether the hands are on it
    fn thing_at(&self, skel: &Skeleton, base: &dyn Fn(f32) -> Pose, t: f32) -> (Thing, Phase) {
        if t < self.start {
            return (self.thing, Phase::Before);
        }
        if t < self.grab {
            return (self.thing, Phase::Reach);
        }
        let carried = |t: f32| {
            let (ride, u) = (self.riding(skel, base, t), smooth(span(t, self.grab, self.carried)));
            self.thing.moved(self.thing.at.lerp(ride.at, u), self.thing.turn.slerp(ride.turn, u))
        };
        match self.release {
            Some(Release::Put { at, down, place }) if t >= at => {
                let from = carried(at);
                let u = smooth(span(t, at, down));
                // set down level with where it stood: its own turn about the vertical kept
                let thing = from.moved(from.at.lerp(place, u), from.turn);
                (thing, if t < down { Phase::Release } else { Phase::After })
            }
            Some(Release::Throw { at, velocity, floor }) if t >= at => {
                let from = carried(at);
                let rest = floor + from.solid.bound();
                // the fall, stopped where it lands
                let fall = |dt: f32| from.at + velocity * dt - Vec3::Y * (0.5 * GRAVITY * dt * dt);
                let land = (velocity.y + (velocity.y * velocity.y + 2.0 * GRAVITY * (from.at.y - rest).max(0.0)).sqrt()) / GRAVITY;
                (from.moved(fall((t - at).min(land)), from.turn), Phase::After)
            }
            _ => (carried(t), if t < self.carried { Phase::Lift } else { Phase::Carry }),
        }
    }

    /// when the hands let go, if they do
    fn let_go(&self) -> Option<f32> {
        match self.release? {
            Release::Put { down, .. } => Some(down),
            Release::Throw { at, .. } => Some(at),
        }
    }
}

/// The body and the thing at `t`. `base` is the body's own motion: its pose at any time.
pub fn perform(skel: &Skeleton, base: &dyn Fn(f32) -> Pose, take: &Take, t: f32) -> Frame {
    let own = base(t);
    let (thing, phase) = take.thing_at(skel, base, t);
    let mut frame = Frame { pose: own.clone(), thing, phase, miss: 0.0, moved: 0.0, touch: [None, None] };
    // how much the hands are the take's and not the body's own: none before, all from the grab, none again once away
    let gone = take.let_go();
    let weight = match gone {
        Some(g) if t >= g => 1.0 - smooth(span(t, g, g + AWAY)),
        _ => (t >= take.start) as u8 as f32,
    };
    if weight <= 0.0 {
        return frame;
    }
    // while letting go the hands ease off the place the thing was let go at, not after a thing in flight
    let held = match gone {
        Some(g) if t >= g => take.thing_at(skel, base, g).0,
        _ => thing,
    };
    let w0 = skel.fk(&own);
    let face = yaw(&w0) * Vec3::Z;
    let mut pose = own.clone();
    for hold in take.grip.holds() {
        let Some((at, turn)) = wrist_for(skel, &held, &hold, facing(&held, w0.pos[0]).lerp(face, 0.5).normalize_or(face)) else { continue };
        let Some(arm) = skel.arm(hold.side) else { continue };
        // the reach: the wrist leaves where the body has it and comes in onto the hold from off the surface
        let u = smooth(span(t, take.start, take.grab));
        let off = (held.turn * hold.normal) * 0.1 * (1.0 - u);
        let target = w0.pos[arm[2]].lerp(at + off, u);
        let end = w0.rot[arm[2]].slerp(turn, smooth(span(t, take.start, take.start + (take.grab - take.start) * 0.8)));
        // elbows out and down, the way arms hang
        let pole = yaw(&w0) * Vec3::new(0.7 * hold.side.sign(), -1.0, -0.4);
        let Some(solved) = reach(skel, &pose, hold.side, target, ArmOptions { pole: Some(pole), end: Some(end), soft: 0.02 }) else { continue };
        pose = forearm_twist(skel, &solved.pose, hold.side, 0.7);
        if t >= take.grab {
            frame.miss = frame.miss.max(solved.miss);
        }
        // the fingers shut over the last moments of the reach, and open again as the hand leaves
        let close = smooth(span(t, take.grab - SHUT, take.grab)) * weight;
        if close > 0.0 {
            frame.touch[hold.side as usize] = curl(skel, &mut pose.turns, (at, turn), hold.side, &held, close);
        }
    }
    if weight < 1.0 {
        for (mine, own) in pose.turns.iter_mut().zip(&own.turns) {
            *mine = own.slerp(*mine, weight);
        }
    }
    frame.moved = pose.turns.iter().zip(&own.turns).map(|(a, b)| a.angle_between(*b).to_degrees()).fold(0.0, f32::max);
    frame.pose = pose;
    frame
}
