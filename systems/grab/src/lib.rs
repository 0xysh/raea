//! Procedural arms and hands (docs/hands.feature): a body reaches for a thing, takes it with one hand or two, carries
//! it, and puts it down or throws it, with no capture of that action. Plain maths on glam and raea-sys-body (the
//! rig, IK, limits), no engine: `perform` is a function of time, so a render and live play ask it the same question
//! and get the same frame.
//!
//! - `shape`: a thing as a signed distance: ray casts, normals, clearance.
//! - `hand`: hold points found on the thing, the wrist that lays a palm on one, fingers curled onto the surface.
//! - `trunk`: the body bent and squatted for a low thing with its feet planted, the head turned to the thing, hands
//!   and head kept out of the scene's solids, and where to stand to take a thing.
//! - `interact`: which thing a body takes: the nearest in reach that its eyes see.
//! - `take`: reach, grab, lift, carry, put down or throw, as one timeline.
//! - `json`: takes, fitted poses and hand shapes asked for as JSON: the engine's native calls character.take,
//!   character.fit and character.hand (raea/character's take, fitPose and handShape).

pub mod hand;
pub mod interact;
pub mod json;
pub mod shape;
pub mod take;
pub mod trunk;

pub use hand::{Grip, Hold, HoldOptions, Touch, curl, find_holds, wrist_for};
pub use interact::pick;
pub use json::{FIT_ANSWER, FIT_ARGS, HAND_ANSWER, HAND_ARGS, TAKE_ANSWER, TAKE_ARGS, fit_json, hand_json, take_json};
pub use shape::{Solid, Thing};
pub use take::{Frame, Key, Phase, Release, Take, perform};
pub use trunk::{clear, look, roomy, stand_for, stoop};
