//! The rig, as sdk/body/src/rig.ts has it: MakeHuman's game_engine skeleton (docs/skeleton.feature), poses and
//! forward kinematics. The body's frame: y up, facing +z, x its left. Every joint's rest turn is none, so a joint's
//! turn is read in the body's axes as they are at rest.

use std::collections::HashMap;

use glam::{Quat, Vec3};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    L,
    R,
}

impl Side {
    pub const BOTH: [Side; 2] = [Side::L, Side::R];
    pub fn sx(self) -> &'static str {
        if self == Side::L { "l" } else { "r" }
    }
    /// +1 on the body's left (+x), -1 on its right
    pub fn sign(self) -> f32 {
        if self == Side::L { 1.0 } else { -1.0 }
    }
}

pub const FINGERS: [&str; 5] = ["thumb", "index", "middle", "ring", "pinky"];

#[derive(Clone, Debug)]
pub struct Joint {
    pub name: String,
    /// the parent's index; None for the root (the pelvis). Parents come before their children
    pub parent: Option<usize>,
    /// where the joint and its bone's end are at rest, metres, the body's frame
    pub head: Vec3,
    pub tail: Vec3,
}

#[derive(Clone, Debug)]
pub struct Skeleton {
    pub joints: Vec<Joint>,
    index: HashMap<String, usize>,
}

/// A pose: the root's place, and each joint's turn from rest against its parent.
#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    pub root: Vec3,
    pub turns: Vec<Quat>,
}

/// A pose drawn: each joint's place and turn in the body's frame.
#[derive(Clone, Debug)]
pub struct Posed {
    pub pos: Vec<Vec3>,
    pub rot: Vec<Quat>,
}

impl Skeleton {
    pub fn new(joints: Vec<Joint>) -> Self {
        let index = joints.iter().enumerate().map(|(i, j)| (j.name.clone(), i)).collect();
        Skeleton { joints, index }
    }

    /// A character's skeleton.json (projects/psx/export.py): `joints` of name, parent (-1 for the root), head, tail.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("skeleton.json: {e}"))?;
        let list = v["joints"].as_array().ok_or("skeleton.json: no joints")?;
        let vec3 = |a: &serde_json::Value| -> Option<Vec3> {
            let a = a.as_array()?;
            Some(Vec3::new(a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32, a.get(2)?.as_f64()? as f32))
        };
        let mut joints = Vec::with_capacity(list.len());
        for (i, j) in list.iter().enumerate() {
            let name = j["name"].as_str().ok_or_else(|| format!("skeleton.json: joint {i} has no name"))?;
            let parent = j["parent"].as_i64().ok_or_else(|| format!("skeleton.json: {name} has no parent"))?;
            if parent >= i as i64 {
                return Err(format!("skeleton.json: {name}'s parent comes after it"));
            }
            let head = vec3(&j["head"]).ok_or_else(|| format!("skeleton.json: {name} has no head"))?;
            let tail = vec3(&j["tail"]).unwrap_or(head);
            joints.push(Joint { name: name.to_string(), parent: usize::try_from(parent).ok(), head, tail });
        }
        Ok(Skeleton::new(joints))
    }

    pub fn joint(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }

    fn three(&self, names: [String; 3]) -> Option<[usize; 3]> {
        Some([self.joint(&names[0])?, self.joint(&names[1])?, self.joint(&names[2])?])
    }

    /// shoulder, elbow, wrist
    pub fn arm(&self, s: Side) -> Option<[usize; 3]> {
        self.three(["upperarm", "lowerarm", "hand"].map(|n| format!("{n}_{}", s.sx())))
    }

    /// a finger's three joints, knuckle first (the thumb's first is its metacarpal)
    pub fn finger(&self, f: &str, s: Side) -> Option<[usize; 3]> {
        self.three([1, 2, 3].map(|k| format!("{f}_0{k}_{}", s.sx())))
    }

    pub fn rest(&self) -> Pose {
        Pose { root: self.joints.first().map_or(Vec3::ZERO, |j| j.head), turns: vec![Quat::IDENTITY; self.joints.len()] }
    }

    /// a joint's place at rest against its parent's
    pub fn offset(&self, i: usize) -> Vec3 {
        match self.joints[i].parent {
            Some(p) => self.joints[i].head - self.joints[p].head,
            None => Vec3::ZERO,
        }
    }

    /// Forward kinematics: each joint's place and turn.
    pub fn fk(&self, p: &Pose) -> Posed {
        let n = self.joints.len();
        let (mut pos, mut rot) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for i in 0..n {
            let t = p.turns.get(i).copied().unwrap_or(Quat::IDENTITY);
            match self.joints[i].parent {
                None => {
                    rot.push(t);
                    pos.push(p.root);
                }
                Some(par) => {
                    let (pr, pp): (Quat, Vec3) = (rot[par], pos[par]);
                    rot.push((pr * t).normalize());
                    pos.push(pp + pr * self.offset(i));
                }
            }
        }
        Posed { pos, rot }
    }

    /// the local turn that gives joint `i` the world turn `world`, against its drawn parent
    pub fn local_for(&self, w: &Posed, i: usize, world: Quat) -> Quat {
        let pr = self.joints[i].parent.map_or(Quat::IDENTITY, |p| w.rot[p]);
        (pr.inverse() * world).normalize()
    }
}

/// A stand-in body of the rig's joints at an adult's proportions (1.81 m), so the solvers run with no character on
/// disk: tests, examples, and a scene that hasn't loaded its body yet. A real character brings its own skeleton.json.
pub fn mannequin() -> Skeleton {
    // name, parent, head, tail; the left side only (x > 0), the right is its mirror
    const MID: [(&str, &str, [f32; 3], [f32; 3]); 6] = [
        ("pelvis", "", [0.0, 1.00, 0.01], [0.0, 1.09, -0.03]),
        ("spine_01", "pelvis", [0.0, 1.09, -0.03], [0.0, 1.16, -0.02]),
        ("spine_02", "spine_01", [0.0, 1.16, -0.02], [0.0, 1.22, -0.04]),
        ("spine_03", "spine_02", [0.0, 1.22, -0.04], [0.0, 1.36, -0.06]),
        ("neck_01", "spine_03", [0.0, 1.57, 0.01], [0.0, 1.66, 0.04]),
        ("head", "neck_01", [0.0, 1.66, 0.04], [0.0, 1.82, 0.04]),
    ];
    const LIMB: [(&str, &str, [f32; 3], [f32; 3]); 8] = [
        ("clavicle", "spine_03", [0.024, 1.490, 0.030], [0.190, 1.463, 0.013]),
        ("upperarm", "clavicle", [0.190, 1.463, 0.013], [0.401, 1.235, 0.022]),
        ("lowerarm", "upperarm", [0.401, 1.235, 0.022], [0.534, 1.109, 0.213]),
        ("hand", "lowerarm", [0.534, 1.109, 0.213], [0.545, 1.093, 0.241]),
        ("thigh", "pelvis", [0.120, 0.975, -0.004], [0.160, 0.579, 0.030]),
        ("calf", "thigh", [0.160, 0.579, 0.030], [0.192, 0.093, 0.004]),
        ("foot", "calf", [0.192, 0.093, 0.004], [0.196, 0.029, 0.138]),
        ("ball", "foot", [0.196, 0.029, 0.138], [0.192, 0.034, 0.211]),
    ];
    // each finger's knuckle, middle joint, end joint and tip
    const HAND: [(&str, [[f32; 3]; 4]); 5] = [
        ("thumb", [[0.535, 1.102, 0.259], [0.513, 1.094, 0.281], [0.501, 1.081, 0.314], [0.495, 1.064, 0.336]]),
        ("index", [[0.563, 1.070, 0.313], [0.566, 1.055, 0.335], [0.566, 1.039, 0.352], [0.562, 1.019, 0.371]]),
        ("middle", [[0.580, 1.058, 0.295], [0.587, 1.032, 0.317], [0.590, 1.010, 0.335], [0.590, 0.988, 0.352]]),
        ("ring", [[0.588, 1.049, 0.277], [0.598, 1.025, 0.293], [0.602, 1.004, 0.307], [0.604, 0.979, 0.319]]),
        ("pinky", [[0.589, 1.040, 0.257], [0.596, 1.020, 0.266], [0.597, 1.006, 0.272], [0.600, 0.987, 0.280]]),
    ];
    let mut out: Vec<(String, String, Vec3, Vec3)> = Vec::new();
    for (n, p, h, t) in MID {
        out.push((n.into(), p.into(), h.into(), t.into()));
    }
    let on = |v: [f32; 3], s: Side| Vec3::new(v[0] * s.sign(), v[1], v[2]);
    let sided = |n: &str, s: Side| if MID.iter().any(|m| m.0 == n) { n.to_string() } else { format!("{n}_{}", s.sx()) };
    for group in [&LIMB[..4], &LIMB[4..]] {
        for s in Side::BOTH {
            for (n, p, h, t) in group {
                out.push((sided(n, s), sided(p, s), on(*h, s), on(*t, s)));
            }
        }
    }
    for s in Side::BOTH {
        for (f, pts) in HAND {
            for k in 0..3 {
                let parent = if k == 0 { format!("hand_{}", s.sx()) } else { format!("{f}_0{k}_{}", s.sx()) };
                out.push((format!("{f}_0{}_{}", k + 1, s.sx()), parent, on(pts[k], s), on(pts[k + 1], s)));
            }
        }
    }
    let names: Vec<String> = out.iter().map(|j| j.0.clone()).collect();
    Skeleton::new(
        out.into_iter()
            .map(|(name, parent, head, tail)| Joint { parent: names.iter().position(|n| *n == parent), name, head, tail })
            .collect(),
    )
}
