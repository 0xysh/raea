//! A render's frames as their parts, and the checks a render runs on them at its end: raea-sys-contact's clipping and
//! raea-sys-stage's staging (docs/stage-guards.feature SG10-SG14). `raea scene` (raea-app-scene) steps them without
//! drawing.

use super::{Frame, flag};

/// A frame's 3D walked into its parts (raea-runtime-scene).
pub fn items(f: &Frame) -> Vec<raea_core_scene::Item> {
    raea_runtime_scene::items_of(&f.scene)
}

/// Each frame's items, walked once (a frame's tree was walked again for each kind of part: 11 times a live frame)
pub fn all_items(frames: &[(i64, Frame)]) -> Vec<Vec<raea_core_scene::Item>> {
    frames.iter().map(|(_, f)| items(f)).collect()
}

/// raea_sys_contact::told on a render's frames, the parts that clip where the camera sees them (RAEA_CLIP_CHECK=0 skips
/// it), and raea_sys_stage::told, the staging (RAEA_STAGE_CHECK=0), each said on stderr. `page` is the manifest's
/// frames (each one's words in the page, which the staging holds off the parts: SG11), None where they aren't known.
pub(crate) fn told(frames: &[(i64, Frame)], page: Option<&[serde_json::Value]>, w: u32, h: u32, fps: f64) -> Vec<raea_core_facts::Finding> {
    checked(frames.iter().map(|(n, f)| (*n, raea_runtime_scene::items_of(&f.scene))).collect(), page, w, h, fps)
}

/// told for frames nothing reads the 3D of after: each frame's scene let go as its parts are made, so the end checks
/// don't hold every frame's scene and every frame's parts at once (SG4a)
pub(crate) fn told_taking(frames: &mut [(i64, Frame)], page: Option<&[serde_json::Value]>, w: u32, h: u32, fps: f64) -> Vec<raea_core_facts::Finding> {
    checked(frames.iter_mut().map(|(n, f)| (*n, raea_runtime_scene::items_of(&std::mem::take(&mut f.scene)))).collect(), page, w, h, fps)
}

fn checked(frames: Vec<(i64, Vec<raea_core_scene::Item>)>, page: Option<&[serde_json::Value]>, w: u32, h: u32, fps: f64) -> Vec<raea_core_facts::Finding> {
    if frames.iter().all(|(_, its)| its.is_empty()) {
        return Vec::new();
    }
    // RAEA_MEM_MARKS: the footprint after each end step (raea-render's mem_mark)
    let mark = raea_render::engine::mem_mark;
    mark("end checks: each frame's parts made");
    let (nums, all): (Vec<i64>, Vec<_>) = frames.into_iter().unzip();
    *SEEN.lock().unwrap() = Some(raea_sys_stage::seen(&all, &nums, w, h));
    mark("end checks: what's on screen");
    let mut found = if std::env::var("RAEA_CLIP_CHECK").as_deref() == Ok("0") { Vec::new() } else { raea_sys_contact::told(&all, &nums, w, h, fps) };
    mark("end checks: clipping");
    found.extend(raea_sys_stage::told(&all, &nums, &page.map_or_else(Vec::new, |p| raea_sys_stage::pages(p, &nums, w, h)), w, h, fps));
    mark("end checks: staging");
    // the clipping check's solids, kept for the process, aren't needed again by a render
    raea_sys_contact::forget();
    drop(all);
    mark("end checks: done, their parts and solids let go");
    found
}

/// What the manifest says of a render's parts (raea_sys_stage::seen; write_manifest takes it): for stills each
/// frame's parts on screen, for a longer render where they remount.
pub(crate) static SEEN: std::sync::Mutex<Option<serde_json::Value>> = std::sync::Mutex::new(None);

/// `--frame N`, 0 when it isn't given (scene lint and render --review).
pub fn frame_of(args: &[String]) -> Result<i64, String> {
    match flag(args, "--frame") {
        Some(f) => f.parse().map_err(|_| "--frame takes a frame number".to_string()),
        None => Ok(0),
    }
}
