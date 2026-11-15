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
use raea_sys_body::shape::Thing;

/// how far below the shoulders, in arm lengths, the hands go before the body bends for them
const EASY: f32 = 0.88;

/// The body bent and lowered for hands that must get down to `low` (a height, metres), by `amount` (0 to 1) of what
/// that needs. `own` drawn is `w`; `face` is the level way the body faces. `give` (0 to 1) trades the bend for more
/// squat, down to an upright, deep squat at 1: for a body that can't bend forward because something stands there
/// (`roomy` says when it can).
pub fn stoop(skel: &Skeleton, own: &Pose, w: &Posed, face: Vec3, low: f32, amount: f32, give: f32, solids: &[Thing]) -> Pose {
    let (Some(l), Some(r)) = (skel.arm(Side::L), skel.arm(Side::R)) else { return own.clone() };
    let shoulders = (w.pos[l[0]].y + w.pos[r[0]].y) / 2.0;
    let arm = skel.offset(l[1]).length() + skel.offset(l[2]).length();
    let short = (shoulders - EASY * arm) - low;
    if short <= 0.0 || amount <= 0.0 {
        return own.clone();
    }
    // eased in from nothing, so the body starts and stops bending without a kink
    let short = short * short / (short + 0.1);
    // the hips take most of it by coming down (a squat), the trunk the rest by bending; giving up the bend, the hips
    // come down by what the bend had lowered the shoulders
    let give = give.clamp(0.0, 1.0);
    let (drop, pitch) = ((short * 0.75).min(0.55), (short * 0.8).min(0.85));
    let (drop, pitch) = ((drop + 0.45 * pitch * give).min(0.82) * amount, pitch * (1.0 - give) * amount);
    let left = Vec3::Y.cross(face);
    let knees: Vec<usize> = ["calf_l", "calf_r"].iter().filter_map(|n| skel.joint(n)).collect();
    // the hips go back a little as the trunk goes forward, as a body keeps its balance; and further back by how near
    // a knee would be to a solid (a squat in front of a table sits back from it): found once, then by as much
    let bent = |back: f32| {
        let mut pose = own.clone();
        pose.root = own.root - Vec3::Y * drop - face * (0.12 * pitch + back);
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
    };
    let pose = bent(0.0);
    let at = skel.fk(&pose);
    let mut near = f32::MAX;
    for &k in &knees {
        for solid in solids {
            near = near.min(solid.distance(at.pos[k]));
        }
    }
    if near < 0.09 { bent(((0.09 - near) * 1.3).min(0.26)) } else { pose }
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
pub fn clear(wrist: Vec3, turn: Quat, hand: &(Vec<Vec3>, f32), solids: &[Thing]) -> Vec3 {
    let (points, margin) = hand;
    let mut at = wrist;
    for _ in 0..4 {
        let mut worst: Option<(f32, Vec3)> = None;
        for solid in solids {
            for p in points {
                let p = at + turn * *p;
                let d = solid.distance(p) - margin;
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

/// The points of a hand that `clear` keeps off solids, from the wrist in the hand's rest axes, and how far off: the
/// hand's own skin (palm and fingers as they are at rest, 2 cm off, the room opened fingers need) where the body has samples of it; else the wrist,
/// the palm and the fingers' reach, 2 cm off.
pub fn hand_points(skel: &Skeleton, s: Side, rest: &HandRest) -> (Vec<Vec3>, f32) {
    let Some(hand) = skel.arm(s).map(|arm| arm[2]) else { return (Vec::new(), 0.0) };
    let wrist = skel.joints[hand].head;
    let mut joints = vec![hand];
    joints.extend(raea_sys_body::rig::FINGERS.iter().filter_map(|f| skel.finger(f, s)).flatten());
    let skin: Vec<Vec3> = joints.iter().flat_map(|&j| skel.skin_of(j)).map(|x| x.at - wrist).collect();
    if skin.is_empty() { (vec![Vec3::ZERO, rest.mid, rest.along * 0.17], 0.02) } else { (skin, 0.02) }
}

/// whether a part of the skin (a sample's name) is the arm's or the hand's
pub fn of_the_arm(part: &str) -> bool {
    part == "palm" || part.ends_with("Arm") || ["thumb", "index", "middle", "ring", "pinky"].iter().any(|f| part.starts_with(f))
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

/// The trunk as a solid, where a pose has it: a capsule from the hips to the top of the chest, as wide as the body
/// (three fifths of the way out to a shoulder). What an elbow must not swing into.
pub fn torso(skel: &Skeleton, at: &Posed) -> Option<Thing> {
    let (chest, shoulder) = (skel.joint("spine_03")?, skel.arm(Side::L)?[0]);
    let top = at.pos[chest] + at.rot[chest] * (skel.joints[chest].tail - skel.joints[chest].head);
    let up = top - at.pos[0];
    let wide = 0.6 * (skel.joints[shoulder].head.x - skel.joints[chest].head.x).abs();
    Some(Thing { solid: raea_sys_body::shape::Solid::Capsule { half: up.length() / 2.0, r: wide }, at: at.pos[0] + up / 2.0, turn: Quat::from_rotation_arc(Vec3::Y, up.normalize_or(Vec3::Y)) })
}

/// The body's own solids where a pose has them: the trunk and both thighs (each a capsule along its bone, a little
/// under half as wide as the trunk). What a carried thing, and the hands on it, must not pass through.
pub fn own_solids(skel: &Skeleton, at: &Posed) -> Vec<Thing> {
    let Some(trunk) = torso(skel, at) else { return Vec::new() };
    let raea_sys_body::shape::Solid::Capsule { r, .. } = trunk.solid else { return vec![trunk] };
    let mut out = vec![trunk];
    for s in Side::BOTH {
        let (Some(thigh), Some(calf)) = (skel.joint(&format!("thigh_{}", s.sx())), skel.joint(&format!("calf_{}", s.sx()))) else { continue };
        let bone = at.pos[calf] - at.pos[thigh];
        out.push(Thing { solid: raea_sys_body::shape::Solid::Capsule { half: bone.length() / 2.0, r: r * 0.75 }, at: at.pos[thigh] + bone / 2.0, turn: Quat::from_rotation_arc(Vec3::Y, bone.normalize_or(Vec3::NEG_Y)) });
    }
    out
}
