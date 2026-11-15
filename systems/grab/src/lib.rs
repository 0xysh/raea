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
//! - `fit`: a pose fitted to points seen (SOMA-X's analytical pose inversion), for motion that arrives as positions.
//! - `interact`: which thing a body takes: the nearest in reach that its eyes see.
//! - `limits`: each joint's range as anatomical angles (sdk/body's), a turn brought inside it, a pose's violations.
//! - `prior`: SOMA-X's hand pose prior on raea's fingers: a hand's shape from a few numbers, and how odd a hand is.
//! - `take`: reach, grab, lift, carry, put down or throw, as one timeline.
//! - `json`: a take asked for as JSON, the engine's native call `character.take` (raea/character's `take`).

pub mod arm;
pub mod body;
pub mod fit;
pub mod hand;
pub mod interact;
pub mod json;
pub mod limits;
pub mod prior;
pub mod rig;
pub mod shape;
pub mod take;

pub use arm::{ArmOptions, IkReport, forearm_twist, reach, twist_joints, two_bone};
pub use json::{FIT_ANSWER, FIT_ARGS, HAND_ANSWER, HAND_ARGS, TAKE_ANSWER, TAKE_ARGS, fit_json, hand_json, take_json};
pub use body::{clear, look, roomy, stand_for, stoop};
pub use fit::{Mark, fit, marks_of};
pub use interact::pick;
pub use hand::{Grip, Hold, HoldOptions, Touch, curl, find_holds, wrist_for};
pub use limits::{Angles, angles_of, clamp_turn, violations};
pub use rig::{Pose, Posed, Side, Skeleton, mannequin};
pub use shape::{Solid, Thing};
pub use take::{Frame, Phase, Release, Take, perform};
