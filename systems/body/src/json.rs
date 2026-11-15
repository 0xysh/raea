//! Poses fitted to points and hands shaped by number, asked for as JSON: the engine's native calls character.fit and
//! character.hand (raea/character's fitPose and handShape); and the readers the hands' calls share.

use glam::{Quat, Vec3};
use serde_json::{Value, json};

use crate::fit::{Mark, fit};
use crate::prior::{coefficients, oddness, shape};
use crate::rig::{Pose, Side, Skeleton};
use crate::soma::from_soma;

pub fn num(v: &Value, what: &str) -> Result<f32, String> {
    v.as_f64().map(|n| n as f32).ok_or_else(|| format!("character.take: {what} isn't a number"))
}

pub fn vec3(v: &Value, what: &str) -> Result<Vec3, String> {
    let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(|| format!("character.take: {what} isn't [x, y, z]"))?;
    Ok(Vec3::new(num(&a[0], what)?, num(&a[1], what)?, num(&a[2], what)?))
}

pub fn quat(v: &Value, what: &str) -> Result<Quat, String> {
    let a = v.as_array().filter(|a| a.len() == 4).ok_or_else(|| format!("character.take: {what} isn't [x, y, z, w]"))?;
    Ok(Quat::from_xyzw(num(&a[0], what)?, num(&a[1], what)?, num(&a[2], what)?, num(&a[3], what)?).normalize())
}

pub const FIT_ARGS: &str = "{ joints: { name: string; parent: number; head: [number, number, number]; tail: [number, number, number] }[]; marks: { joint: string; offset?: [number, number, number]; at: [number, number, number]; weight?: number }[]; from?: { root: [number, number, number]; turns: [number, number, number, number][] }; passes?: number }";
pub const FIT_ANSWER: &str = "{ pose: { root: [number, number, number]; turns: [number, number, number, number][] }; miss: number }";

/// `character.fit`: a pose fitted to points of the body seen somewhere (fit.rs).
pub fn fit_json(v: &Value) -> Result<Value, String> {
    let skel = Skeleton::from_value(v)?;
    let mut from = skel.rest();
    if !v["from"].is_null() {
        from.root = vec3(&v["from"]["root"], "from.root")?;
        for (slot, turn) in from.turns.iter_mut().zip(v["from"]["turns"].as_array().into_iter().flatten()) {
            *slot = quat(turn, "from's turn")?;
        }
    }
    let mut marks = Vec::new();
    for m in v["marks"].as_array().ok_or("character.fit: no marks")? {
        let name = m["joint"].as_str().ok_or("character.fit: a mark names no joint")?;
        let joint = skel.joint(name).ok_or_else(|| format!("character.fit: the body has no joint {name}"))?;
        let offset = if m["offset"].is_null() { Vec3::ZERO } else { vec3(&m["offset"], "a mark's offset")? };
        let weight = if m["weight"].is_null() { 1.0 } else { num(&m["weight"], "a mark's weight")? };
        marks.push(Mark { joint, offset, at: vec3(&m["at"], "a mark's at")?, weight });
    }
    let passes = v["passes"].as_u64().unwrap_or(8) as usize;
    let (pose, miss) = fit(&skel, &from, &marks, passes);
    Ok(json!({ "pose": posed(&pose), "miss": miss }))
}

pub const HAND_ARGS: &str = "{ joints: { name: string; parent: number; head: [number, number, number]; tail: [number, number, number] }[]; side: \"L\" | \"R\"; z?: number[]; pose?: { root: [number, number, number]; turns: [number, number, number, number][] } }";
pub const HAND_ANSWER: &str = "{ pose: { root: [number, number, number]; turns: [number, number, number, number][] }; z: number[]; odd: number }";

/// `character.hand`: a hand shaped by the hand prior's coefficients (prior.rs) on a pose, and that hand read back:
/// its coefficients and how odd it is.
pub fn hand_json(v: &Value) -> Result<Value, String> {
    let skel = Skeleton::from_value(v)?;
    let side = if v["side"] == "L" { Side::L } else { Side::R };
    let mut pose = skel.rest();
    if !v["pose"].is_null() {
        pose.root = vec3(&v["pose"]["root"], "pose.root")?;
        for (slot, turn) in pose.turns.iter_mut().zip(v["pose"]["turns"].as_array().into_iter().flatten()) {
            *slot = quat(turn, "the pose's turn")?;
        }
    }
    if let Some(z) = v["z"].as_array() {
        let z = z.iter().map(|n| num(n, "a number of z")).collect::<Result<Vec<f32>, String>>()?;
        shape(&skel, &mut pose.turns, side, &z).ok_or("character.hand: the body has no hand on that side")?;
    }
    let (z, _) = coefficients(&skel, &pose, side).ok_or("character.hand: the body has no hand on that side")?;
    let odd = oddness(&skel, &pose, side).unwrap_or(0.0);
    Ok(json!({ "pose": posed(&pose), "z": z, "odd": odd }))
}

pub const SOMA_ARGS: &str = "{ joints: { name: string; parent: number; head: [number, number, number]; tail: [number, number, number] }[]; names: string[]; at: [number, number, number][]; eyes?: [number, number, number] }";
pub const SOMA_ANSWER: &str = "{ pose: { root: [number, number, number]; turns: [number, number, number, number][] } }";

/// `character.soma`: the pose that has the body's bones pointing as a SOMA body's do (soma.rs).
pub fn soma_json(v: &Value) -> Result<Value, String> {
    let skel = Skeleton::from_value(v)?;
    let mut names = Vec::new();
    for name in v["names"].as_array().ok_or("character.soma: no names")? {
        names.push(name.as_str().ok_or("character.soma: a name isn't text")?.to_string());
    }
    let mut at = Vec::with_capacity(names.len());
    for p in v["at"].as_array().ok_or("character.soma: no places")? {
        at.push(vec3(p, "a SOMA joint's place")?);
    }
    if at.len() != names.len() {
        return Err(format!("character.soma: {} names for {} places", names.len(), at.len()));
    }
    let eyes = if v["eyes"].is_null() { None } else { Some(vec3(&v["eyes"], "eyes")?) };
    let pose = from_soma(&skel, &names, &at, eyes).ok_or("character.soma: the body or the SOMA joints lack the hips or a leg")?;
    Ok(json!({ "pose": posed(&pose) }))
}

/// a pose as the calls answer it: the root's place and every joint's turn
pub fn posed(pose: &Pose) -> Value {
    let turns: Vec<[f32; 4]> = pose.turns.iter().map(|q| q.to_array()).collect();
    json!({ "root": pose.root.to_array(), "turns": turns })
}
