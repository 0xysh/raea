//! A frame of a take asked for as JSON: the engine's native call `character.take`, which raea/character's `take`
//! makes each frame. The body's own motion comes as its poses at the times the take reads it (the frame's, the
//! grab's, the release's); the answer is the pose to draw and where the thing is.

use glam::{Quat, Vec3};
use serde_json::{Value, json};

use crate::body::stand_for;
use crate::fit::{Mark, fit};
use crate::hand::{Grip, HoldOptions, find_holds};
use crate::rig::{Pose, Side, Skeleton};
use crate::shape::{Solid, Thing};
use crate::take::{Release, Take, perform};

pub const TAKE_ARGS: &str = "{ joints: { name: string; parent: number; head: [number, number, number]; tail: [number, number, number] }[]; t: number; base: { t: number; root: [number, number, number]; turns: [number, number, number, number][] }[]; thing: { ball?: number; box?: [number, number, number]; capsule?: [number, number]; cylinder?: [number, number]; at: [number, number, number]; turn?: [number, number, number, number] }; start: number; grab?: number; carried?: number; carry?: [number, number, number]; hand?: \"L\" | \"R\"; lower?: number; avoid?: { ball?: number; box?: [number, number, number]; capsule?: [number, number]; cylinder?: [number, number]; at: [number, number, number]; turn?: [number, number, number, number] }[]; put?: { at: number; down: number; place: [number, number, number] }; throw?: { at: number; velocity: [number, number, number]; floor?: number } }";
pub const TAKE_ANSWER: &str = "{ pose: { root: [number, number, number]; turns: [number, number, number, number][] }; thing: { at: [number, number, number]; turn: [number, number, number, number] }; phase: \"Before\" | \"Reach\" | \"Lift\" | \"Carry\" | \"Release\" | \"After\"; hands: 1 | 2; miss: number; moved: number; limited: boolean; stand: { at: [number, number, number]; yaw: number } }";

fn num(v: &Value, what: &str) -> Result<f32, String> {
    v.as_f64().map(|n| n as f32).ok_or_else(|| format!("character.take: {what} isn't a number"))
}

fn vec3(v: &Value, what: &str) -> Result<Vec3, String> {
    let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(|| format!("character.take: {what} isn't [x, y, z]"))?;
    Ok(Vec3::new(num(&a[0], what)?, num(&a[1], what)?, num(&a[2], what)?))
}

fn quat(v: &Value, what: &str) -> Result<Quat, String> {
    let a = v.as_array().filter(|a| a.len() == 4).ok_or_else(|| format!("character.take: {what} isn't [x, y, z, w]"))?;
    Ok(Quat::from_xyzw(num(&a[0], what)?, num(&a[1], what)?, num(&a[2], what)?, num(&a[3], what)?).normalize())
}

fn solid(v: &Value) -> Result<Solid, String> {
    if !v["ball"].is_null() {
        return Ok(Solid::Ball { r: num(&v["ball"], "thing.ball")? });
    }
    if !v["box"].is_null() {
        return Ok(Solid::Box { half: vec3(&v["box"], "thing.box")? });
    }
    // a capsule or a cylinder: half its length along its own y, then its radius
    let rod = ["capsule", "cylinder"].into_iter().find(|k| !v[*k].is_null()).ok_or("character.take: the thing needs a shape: ball, box, capsule or cylinder")?;
    let a = v[rod].as_array().filter(|a| a.len() == 2).ok_or_else(|| format!("character.take: thing.{rod} isn't [half its length, radius]"))?;
    let (half, r) = (num(&a[0], rod)?, num(&a[1], rod)?);
    Ok(if rod == "capsule" { Solid::Capsule { half, r } } else { Solid::Cylinder { half, r } })
}

pub fn take_json(v: &Value) -> Result<Value, String> {
    let skel = Skeleton::from_value(v)?;
    let mut base: Vec<(f32, Pose)> = Vec::new();
    for b in v["base"].as_array().ok_or("character.take: no base poses")? {
        let turns = b["turns"].as_array().ok_or("character.take: a base pose has no turns")?;
        let mut pose = skel.rest();
        pose.root = vec3(&b["root"], "a base pose's root")?;
        for (slot, turn) in pose.turns.iter_mut().zip(turns) {
            *slot = quat(turn, "a base pose's turn")?;
        }
        base.push((num(&b["t"], "a base pose's t")?, pose));
    }
    if base.is_empty() {
        return Err("character.take: no base poses".into());
    }
    // the body's own motion at a time: the pose given nearest it
    let own = |t: f32| base.iter().min_by(|a, b| (a.0 - t).abs().total_cmp(&(b.0 - t).abs())).map(|b| b.1.clone()).unwrap_or_else(|| skel.rest());

    let placed = |shape: &Value| -> Result<Thing, String> {
        let turn = if shape["turn"].is_null() { Quat::IDENTITY } else { quat(&shape["turn"], "a thing's turn")? };
        Ok(Thing { solid: solid(shape)?, at: vec3(&shape["at"], "a thing's at")?, turn })
    };
    let thing = placed(&v["thing"])?;
    let start = num(&v["start"], "start")?;
    let mut options = HoldOptions::default();
    if v["hand"] == "L" {
        options.hand = Side::L;
    }
    if !v["lower"].is_null() {
        options.lower = num(&v["lower"], "lower")?;
    }
    // the holds are found from where the body stands when it starts to reach
    let grip = find_holds(&thing, own(start).root, options);
    let mut take = Take::new(thing, grip, start);
    if !v["grab"].is_null() {
        take.grab = num(&v["grab"], "grab")?;
        take.carried = take.grab + 0.5;
    }
    if !v["carried"].is_null() {
        take.carried = num(&v["carried"], "carried")?;
    }
    if !v["carry"].is_null() {
        take.carry = vec3(&v["carry"], "carry")?;
    }
    for solid in v["avoid"].as_array().into_iter().flatten() {
        take.avoid.push(placed(solid)?);
    }
    if let Some(p) = v.get("put").filter(|p| !p.is_null()) {
        take.release = Some(Release::Put { at: num(&p["at"], "put.at")?, down: num(&p["down"], "put.down")?, place: vec3(&p["place"], "put.place")? });
    } else if let Some(t) = v.get("throw").filter(|t| !t.is_null()) {
        let floor = if t["floor"].is_null() { 0.0 } else { num(&t["floor"], "throw.floor")? };
        take.release = Some(Release::Throw { at: num(&t["at"], "throw.at")?, velocity: vec3(&t["velocity"], "throw.velocity")?, floor });
    }

    let f = perform(&skel, &own, &take, num(&v["t"], "t")?);
    // where the body would stand to take it, for a scene that walks it there first
    let (stand, facing) = stand_for(&thing, own(start).root);
    let turns: Vec<[f32; 4]> = f.pose.turns.iter().map(|q| q.to_array()).collect();
    Ok(json!({
        "pose": { "root": f.pose.root.to_array(), "turns": turns },
        "thing": { "at": f.thing.at.to_array(), "turn": f.thing.turn.to_array() },
        "phase": format!("{:?}", f.phase),
        "hands": if matches!(grip, Grip::One(_)) { 1 } else { 2 },
        "miss": f.miss,
        "moved": f.moved,
        "limited": f.limited,
        "stand": { "at": stand.to_array(), "yaw": facing },
    }))
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
    let turns: Vec<[f32; 4]> = pose.turns.iter().map(|q| q.to_array()).collect();
    Ok(json!({ "pose": { "root": pose.root.to_array(), "turns": turns }, "miss": miss }))
}
