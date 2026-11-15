//! The character's body in Rust (docs/hands.feature): what sdk/body is in TypeScript, for the engine. Plain maths on
//! glam, no engine.
//!
//! - `rig`: the game_engine skeleton, poses, forward kinematics, a hand's frame, and a stand-in body.
//! - `arm`: two-bone IK with a pole, a reach that leans the trunk, the wrist's twist shared with the forearm, and
//!   SOMA-X's twist joints.
//! - `limits`: each joint's range as anatomical angles (sdk/body's), a turn brought inside it, a pose's violations.
//! - `fit`: a pose fitted to points seen (SOMA-X's analytical pose inversion), for motion that arrives as positions.
//! - `prior`: SOMA-X's hand pose prior on raea's fingers: a hand's shape from a few numbers, and how odd a hand is.

pub mod arm;
pub mod fit;
pub mod limits;
pub mod prior;
pub mod rig;

pub use arm::{ArmOptions, IkReport, forearm_twist, reach, twist_joints, two_bone};
pub use fit::{Mark, fit, marks_of};
pub use limits::{Angles, angles_of, clamp_turn, violations};
pub use rig::{HandRest, Pose, Posed, Side, Skeleton, hand_rest, mannequin};
