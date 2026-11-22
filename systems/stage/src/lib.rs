//! The stage checks a render runs on its frames' scenes (docs/stage-guards.feature SG10, SG12, SG13, SG14): 3D type
//! that can't be read, parts that snap, a camera that lurches or reverses, and frames with nothing composed in them.
//! Each reads the parts every frame holds (raea_core_scene's items, as raea-sys-contact does) through the frame's
//! camera, and reports the part, the frames and the fix. They read the scene, not the pixels, so they run without
//! drawing (`raea scene stage`) as well as at the end of every render.

use bevy::math::Vec3;
use raea_core_facts::{Finding, Level};
use raea_core_scene::{Item, base_ident};
use serde_json::Value;

mod framing;
mod motion;
mod text;
mod view;
pub use framing::on_screen;
pub use view::View;

/// Who says these findings, in a render.
pub const BY: &str = "raea render";

/// The checks on a render's frames, said on stderr and kept with its findings (RAEA_STAGE_CHECK=0 skips them).
pub fn told(all: &[Vec<Item>], frames: &[i64], w: u32, h: u32, fps: f64) -> Vec<Finding> {
    if std::env::var("RAEA_STAGE_CHECK").as_deref() == Ok("0") || all.iter().all(|its| its.is_empty()) {
        return Vec::new();
    }
    let t = std::time::Instant::now();
    let found = check(all, frames, w, h, fps, BY);
    for f in &found {
        eprintln!("raea: {}", f.line());
    }
    raea_core_facts::finding::said(&found);
    if raea_io_trace::stat("RAEA_TIMES") {
        eprintln!("raea times: the stage checks {:.0} ms", t.elapsed().as_secs_f64() * 1e3);
    }
    found
}

/// Every stage check on the frames (`all[k]` is frame `frames[k]`'s parts), worst first.
pub fn check(all: &[Vec<Item>], frames: &[i64], w: u32, h: u32, fps: f64, by: &str) -> Vec<Finding> {
    let st = Stage::new(all, frames, w, h, fps, by);
    let mut out = text::check(&st);
    out.extend(motion::camera(&st));
    out.extend(motion::snaps(&st));
    out.extend(framing::check(&st));
    raea_core_facts::finding::ranked(merged(out))
}

/// Findings of one check over the same frames as one, the others named after it: a code listing's twenty lines too
/// small to read are one fault, not twenty.
fn merged(fs: Vec<Finding>) -> Vec<Finding> {
    let (mut out, mut more): (Vec<Finding>, Vec<Vec<String>>) = (Vec::new(), Vec::new());
    for f in fs {
        match out.iter().position(|o| o.check == f.check && o.level == f.level && o.frames == f.frames) {
            Some(i) => more[i].push(f.says.split('\'').nth(1).unwrap_or(&f.check).to_string()),
            None => {
                out.push(f);
                more.push(Vec::new());
            }
        }
    }
    for (f, m) in out.iter_mut().zip(more).filter(|(_, m)| !m.is_empty()) {
        let named: Vec<String> = m.iter().take(4).map(|n| format!("'{n}'")).collect();
        f.says = format!("{}; and {} more the same: {}{}", f.says, m.len(), named.join(", "), if m.len() > 4 { ", …" } else { "" });
    }
    out
}

/// The frames as the checks read them: each one's parts and camera, and where the shots cut.
pub(crate) struct Stage<'a> {
    pub all: &'a [Vec<Item>],
    pub frames: &'a [i64],
    pub views: Vec<Option<View>>,
    /// cut[k]: frame k starts a new shot (the first frame, a gap in the frames, a camera jump or a new camera)
    pub cut: Vec<bool>,
    pub w: f32,
    pub h: f32,
    pub fps: f64,
    pub by: &'a str,
}

impl<'a> Stage<'a> {
    fn new(all: &'a [Vec<Item>], frames: &'a [i64], w: u32, h: u32, fps: f64, by: &'a str) -> Stage<'a> {
        let views: Vec<Option<View>> = all.iter().map(|its| View::of(its, w, h)).collect();
        let cut = motion::cuts(&views, frames);
        Stage { all, frames, views, cut, w: w as f32, h: h as f32, fps: fps.max(1.0), by }
    }

    /// A finding about frames `a..=b` (indices into the frames), at `level`.
    pub fn finding(&self, check: &str, level: Level, a: usize, b: usize, says: String) -> Finding {
        Finding::new(check, level, says, self.by).frames(self.frames[a], self.frames[b] + 1)
    }

    /// Seconds from frame index a to b, inclusive of both.
    pub fn secs(&self, a: usize, b: usize) -> f64 {
        (self.frames[b] - self.frames[a] + 1) as f64 / self.fps
    }

    /// A frame stepped alone (raea render --at, raea check --quick's stills): neither neighbour was, so it stands for
    /// its moment, and a fault in it counts as lasting.
    pub fn alone(&self, k: usize) -> bool {
        let f = self.frames[k];
        (k == 0 || self.frames[k - 1] + 1 != f) && self.frames.get(k + 1).is_none_or(|n| *n != f + 1)
    }

    /// Whether every frame from a to b was stepped: what a reveal and a reading time are judged over.
    pub fn stepped(&self, a: usize, b: usize) -> bool {
        self.frames[b] - self.frames[a] == (b - a) as i64
    }

    /// A frame index as a time in the piece, "1:10.06" (minutes, seconds and frames).
    pub fn at(&self, k: usize) -> String {
        let f = self.frames[k];
        let fps = self.fps.round().max(1.0) as i64;
        let s = f / fps;
        format!("{}:{:02}.{:02}", s / 60, s % 60, f % fps)
    }
}

/// What a part is called in a finding: its name, else what it draws, cut short.
pub(crate) fn called(it: &Item) -> String {
    let s = base_ident(it);
    let s = if s.is_empty() { it.kind } else { s };
    let mut out: String = s.chars().take(40).collect();
    if s.chars().count() > 40 {
        out.push('…');
    }
    out
}

/// A number prop.
pub(crate) fn num(it: &Item, k: &str, d: f64) -> f64 {
    it.props.get(k).and_then(Value::as_f64).unwrap_or(d)
}

/// A part's own scale (its matrix's), the largest axis.
pub(crate) fn scale_of(it: &Item) -> f32 {
    let (s, _, _) = it.world.to_scale_rotation_translation();
    s.abs().max_element()
}

/// Runs of consecutive indices where `bad` holds, as (first, last), broken at each cut.
pub(crate) fn runs(st: &Stage, n: usize, bad: impl Fn(usize) -> bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut from: Option<usize> = None;
    for k in 0..n {
        let b = bad(k);
        if let Some(f) = from {
            if !b || st.cut[k] {
                out.push((f, k - 1));
                from = None;
            }
        }
        if b && from.is_none() {
            from = Some(k);
        }
    }
    if let Some(f) = from {
        out.push((f, n - 1));
    }
    out
}

/// The world point a part stands at.
pub(crate) fn origin(it: &Item) -> Vec3 {
    it.world.transform_point3(Vec3::ZERO)
}

#[cfg(test)]
pub(crate) mod test_items {
    use super::*;
    use bevy::math::Mat4;
    use serde_json::json;
    use std::sync::Arc;

    pub fn item(kind: &'static str, props: Value, world: Mat4) -> Item {
        Item { kind, props: Arc::new(props.as_object().cloned().unwrap_or_default()), shape: None, at: None, material: None, world }
    }

    /// A camera at `eye` looking at `at`, 50 degrees.
    pub fn cam(eye: [f32; 3], at: [f32; 3]) -> Item {
        item("camera", json!({"position": eye, "lookAt": at, "fov": 50}), Mat4::IDENTITY)
    }
}
