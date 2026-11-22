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
/// it), and raea_sys_stage::told, the staging (RAEA_STAGE_CHECK=0), each said on stderr.
pub(crate) fn told(frames: &[(i64, Frame)], w: u32, h: u32, fps: f64) -> Vec<raea_core_facts::Finding> {
    if frames.iter().all(|(_, f)| f.scene.is_empty()) {
        return Vec::new();
    }
    let (nums, all): (Vec<i64>, _) = (frames.iter().map(|(f, _)| *f).collect(), all_items(frames));
    *SEEN.lock().unwrap() = seen(&all, &nums, w, h);
    let mut found = if std::env::var("RAEA_CLIP_CHECK").as_deref() == Ok("0") { Vec::new() } else { raea_sys_contact::told(&all, &nums, w, h, fps) };
    found.extend(raea_sys_stage::told(&all, &nums, w, h, fps));
    found
}

/// A render of at most this many frames is stills (`--frame`, `--at`): its manifest names each frame's parts on screen
const STILLS: usize = 16;
/// More than this many parts mounting or unmounting between two frames is a remount
const REMOUNT: usize = 20;

/// What the manifest says of a render's parts (render.rs's write_manifest takes it): for stills, each frame's parts on
/// screen and their boxes (`parts`); for a longer render, the frames where more than REMOUNT parts mount or unmount
/// (`remounts`). SG9 re-renders the frame after each remount alone and names the parts under a patch that differs.
pub(crate) static SEEN: std::sync::Mutex<Option<serde_json::Value>> = std::sync::Mutex::new(None);

fn seen(all: &[Vec<raea_core_scene::Item>], nums: &[i64], w: u32, h: u32) -> Option<serde_json::Value> {
    if all.len() <= STILLS {
        let parts: Vec<_> = all.iter().zip(nums).map(|(its, f)| serde_json::json!({"frame": f, "parts": raea_sys_stage::on_screen(its, w, h)})).collect();
        return Some(serde_json::json!({"parts": parts}));
    }
    let counts: Vec<_> = all.iter().map(|its| counted(its)).collect();
    let remounts: Vec<i64> = (1..counts.len()).filter(|&k| changed(&counts[k], &counts[k - 1]) > REMOUNT).map(|k| nums[k]).collect();
    Some(serde_json::json!({"remounts": remounts}))
}

type Counted<'a> = std::collections::HashMap<(&'a str, &'a str, u64), i64>;

/// A frame's parts as a count by what they are: kind, name, what they draw.
fn counted(its: &[raea_core_scene::Item]) -> Counted<'_> {
    let mut m = Counted::new();
    for it in its {
        *m.entry((it.kind, raea_core_scene::base_ident(it), raea_core_scene::spawn_sig(it))).or_default() += 1;
    }
    m
}

/// How many parts mount or unmount from one frame's count to the next's.
fn changed(a: &Counted, b: &Counted) -> usize {
    let off = |x: &Counted, y: &Counted| x.iter().map(|(k, n)| (n - y.get(k).copied().unwrap_or(0)).max(0) as usize).sum::<usize>();
    off(a, b) + off(b, a)
}

/// `--frame N`, 0 when it isn't given (scene lint and render --review).
pub fn frame_of(args: &[String]) -> Result<i64, String> {
    match flag(args, "--frame") {
        Some(f) => f.parse().map_err(|_| "--frame takes a frame number".to_string()),
        None => Ok(0),
    }
}
