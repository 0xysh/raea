//! Procedural arms and hands (docs/hands.feature): a body reaches for a thing, takes it with one hand or two, carries
//! it, and puts it down or throws it, with no capture of that action. Plain maths on glam, no engine: `perform` is a
//! function of time, so a render and live play ask it the same question and get the same frame.
//!
//! - `rig`: the game_engine skeleton, poses, forward kinematics (sdk/body's, in Rust), and a stand-in body.
//! - `arm`: two-bone IK with a pole, a reach that leans the trunk, and the wrist's twist shared with the forearm.
//! - `shape`: a thing as a signed distance: ray casts, normals, clearance.
//! - `hand`: hold points found on the thing, the wrist that lays a palm on one, fingers curled onto the surface.
//! - `body`: the body bent and squatted for a low thing with its feet planted, hands kept out of the scene's solids,
//!   and where to stand to take a thing.
//! - `take`: reach, grab, lift, carry, put down or throw, as one timeline.
//! - `json`: a take asked for as JSON, the engine's native call `character.take` (raea/character's `take`).

pub mod arm;
pub mod body;
pub mod hand;
pub mod json;
pub mod rig;
pub mod shape;
pub mod take;

pub use arm::{ArmOptions, IkReport, forearm_twist, reach, two_bone};
pub use json::{TAKE_ANSWER, TAKE_ARGS, take_json};
pub use body::{clear, stand_for, stoop};
pub use hand::{Grip, Hold, HoldOptions, Touch, curl, find_holds, wrist_for};
pub use rig::{Pose, Posed, Side, Skeleton, mannequin};
pub use shape::{Solid, Thing};
pub use take::{Frame, Phase, Release, Take, perform};
