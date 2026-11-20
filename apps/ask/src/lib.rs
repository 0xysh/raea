//! `raea ask` (docs/ask.feature AK2-AK3): questions about a piece's picture and sound. `prepare` reads what raea knows
//! of the piece and keeps the questions that apply; `runner` plans the calls, cuts each clip once and asks in parallel;
//! `record` keeps every call and answer for labels and calibration; `verbs` is the command. `raea check`, the app's
//! critique and the fingerprint's double-check go through the same runner.

pub mod prepare;
pub mod record;
pub mod runner;
pub mod verbs;

pub use verbs::{USAGE, cli};

#[cfg(test)]
mod tests;
