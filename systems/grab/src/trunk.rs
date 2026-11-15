//! What the rest of the body does for the hands. `look`: the head turned to what the hands are after (the original's
//! head aim), the neck and head sharing it, capped. `stoop`: for a thing lower than the arms reach, the hips come down
//! and the trunk bends forward at the hips and along the spine, with both feet kept where they stand by leg IK, so a
//! thing on the floor is taken by bending and squatting. `clear`: a hand's place moved the least that keeps it out of
//! the scene's solids (a table's top). `stand_for`: where the body should stand to take a thing, for whatever walks
//! it there (a take starts only when the body is where it can reach: docs/character.md, principle 6).

use glam::{Quat, Vec3};

use raea_sys_body::arm::{ArmOptions, two_bone};
use raea_sys_body::rig::HandRest;
use raea_sys_body::rig::{Pose, Posed, Side, Skeleton};
use crate::shape::Thing;

/// how far below the shoulders, in arm lengths, the hands go before the body bends for them
const EASY: f32 = 0.7;

/// The body bent and lowered for hands that must get down to `low` (a height, metres), by `amount` (0 to 1) of what
/// that needs. `own` drawn is `w`; `face` is the level way the body faces. `give` (0 to 1) trades the bend for more
/// squat, down to an upright, deep squat at 1: for a body that can't bend forward because something stands there
/// (`roomy` says when it can).
pub fn stoop(skel: &Skeleton, own: &Pose, w: &Posed, face: Vec3, low: f32, amount: f32, give: f32) -> Pose {
    let (Some(l), Some(r)) = (skel.arm(Side::L), skel.arm(Side::R)) else { return own.clone() };
    let shoulders = (w.pos[l[0]].y + w.pos[r[0]].y) / 2.0;
    let arm = skel.offset(l[1]).length() + skel.offset(l[2]).length();
    let short = (shoulders - EASY * arm) - low;
    if short <= 0.0 || amount <= 0.0 {
        return own.clone();
    }
    // the hips take most of it by coming down (a squat), the trunk the rest by bending; giving up the bend, the hips
    // come down by what the bend had lowered the shoulders
    let give = give.clamp(0.0, 1.0);
    let (drop, pitch) = ((short * 0.75).min(0.55), (short * 0.8).min(0.85));
    let (drop, pitch) = ((drop + 0.45 * pitch * give).min(0.82) * amount, pitch * (1.0 - give) * amount);
    let left = Vec3::Y.cross(face);
    let mut pose = own.clone();
    // the hips go back a little as the trunk goes forward, as a body keeps its balance
    pose.root = own.root - Vec3::Y * drop - face * (0.12 * pitch);
    pose.turns[0] = (Quat::from_axis_angle(left, pitch * 0.4) * own.turns[0]).normalize();
    for name in ["spine_01", "spine_02", "spine_03"] {
        if let Some(i) = skel.joint(name) {
            pose.turns[i] = (own.turns[i] * Quat::from_rotation_x(pitch * 0.2)).normalize();
        }
    }
    // the feet stay where they stand, turned as they were, the knees going forward
    for s in Side::BOTH {
        let x = s.sx();
        let leg = [format!("thigh_{x}"), format!("calf_{x}"), format!("foot_{x}")].map(|n| skel.joint(&n));
        let [Some(a), Some(b), Some(c)] = leg else { continue };
        let pole = face + left * (0.25 * s.sign());
        pose = two_bone(skel, &pose, [a, b, c], w.pos[c], ArmOptions { pole: Some(pole), end: Some(w.rot[c]), ..Default::default() }).pose;
    }
    pose
}

/// Whether the head's top, the neck and the chest are a hand's breadth off every solid.
pub fn roomy(skel: &Skeleton, pose: &Pose, solids: &[Thing]) -> bool {
    if solids.is_empty() {
        return true;
    }
    let at = skel.fk(pose);
    ["head", "neck_01", "spine_03"].iter().filter_map(|n| skel.joint(n)).all(|i| {
        let tip = at.pos[i] + at.rot[i] * (skel.joints[i].tail - skel.joints[i].head);
        solids.iter().all(|s| s.distance(at.pos[i]) > 0.09 && s.distance(tip) > 0.09)
    })
}

/// A wrist's place moved the least that keeps the hand (its wrist, palm and fingers' reach) clear of every solid.
pub fn clear(wrist: Vec3, turn: Quat, rest: &HandRest, solids: &[Thing]) -> Vec3 {
    const MARGIN: f32 = 0.02;
    let points = [Vec3::ZERO, rest.mid, rest.along * 0.17];
    let mut at = wrist;
    for _ in 0..4 {
        let mut worst: Option<(f32, Vec3)> = None;
        for solid in solids {
            for p in points {
                let p = at + turn * p;
                let d = solid.distance(p) - MARGIN;
                if d < worst.map_or(0.0, |w| w.0) {
                    worst = Some((d, solid.normal(p)));
                }
            }
        }
        let Some((d, normal)) = worst else { break };
        at -= normal * d;
    }
    at
}

/// Where a body coming from `from` should stand to take a thing, and the turn about the vertical to face it.
pub fn stand_for(thing: &Thing, from: Vec3) -> (Vec3, f32) {
    let to = Vec3::new(from.x - thing.at.x, 0.0, from.z - thing.at.z).try_normalize().unwrap_or(Vec3::NEG_Z);
    let far = thing.solid.bound() + 0.1;
    // how far the thing comes toward the body, and a forearm's length off that
    let edge = thing.cast(thing.at + to * far, -to, far).map_or(thing.solid.bound(), |h| (h.at - thing.at).dot(to));
    let at = thing.at + to * (edge + 0.3);
    (Vec3::new(at.x, from.y, at.z), (-to.x).atan2(-to.z))
}

/// The head turned toward a place by `weight` (0 to 1): the neck takes two fifths of the turn and the head the rest,
/// no more than 55 degrees in all, so eyes go to what the hands are after.
pub fn look(skel: &Skeleton, pose: &Pose, at: Vec3, weight: f32) -> Pose {
    let (Some(neck), Some(head)) = (skel.joint("neck_01"), skel.joint("head")) else { return pose.clone() };
    let mut out = pose.clone();
    for (joint, share) in [(neck, 0.4), (head, 1.0)] {
        let w = skel.fk(&out);
        let Some(to) = (at - w.pos[head]).try_normalize() else { break };
        let turn = Quat::from_rotation_arc(w.rot[head] * Vec3::Z, to);
        let (axis, angle) = turn.to_axis_angle();
        let angle = angle.min(55f32.to_radians()) * share * weight.clamp(0.0, 1.0);
        out.turns[joint] = skel.local_for(&w, joint, Quat::from_axis_angle(axis, angle) * w.rot[joint]);
    }
    out
}
