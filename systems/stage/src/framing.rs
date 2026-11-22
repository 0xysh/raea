//! SG14, the frame is composed: frames where almost nothing of the scene is on screen, and the part a shot is about
//! cut by the frame's edge. Both are warnings while they are measured on more films: the page's own 2D (a headline over
//! an empty 3D) isn't known to them.

use std::collections::HashMap;

use bevy::math::Vec3;
use raea_core_facts::{Finding, Level};

use super::{Stage, called, num, runs, scale_of};
use crate::view::corners;

/// Under this share of the frame showing any part is an empty frame
const EMPTY: f32 = 0.08;
/// for this long, seconds; and not this near the piece's start or end (a fade)
const EMPTY_SECS: f64 = 1.0;
const FADE_SECS: f64 = 0.25;
/// The shot's subject cut: between these shares of its box outside the frame, for more than this many frames
const CUT_LO: f32 = 0.15;
const CUT_HI: f32 = 0.85;
const CUT_FRAMES: usize = 10;
/// A backdrop, not content: a slab wider than this in the world (a floor, a set's wall, a sky plane), its thinnest side
/// under SLAB of its widest; a frame of floor and fog is still an empty one
const BACKDROP_M: f32 = 12.0;
const SLAB: f32 = 0.05;
/// The grid the frame's coverage is counted on
const GX: usize = 48;
const GY: usize = 27;

/// A part's box in its own space: a shape's, a block of words' (estimated), a picture's; None when it isn't known.
pub(crate) fn own_box(it: &raea_core_scene::Item, boxes: &mut HashMap<String, Option<(Vec3, Vec3)>>) -> Option<(Vec3, Vec3)> {
    match it.kind {
        "mesh" => {
            let shape = it.shape.as_deref()?;
            *boxes.entry(shape.to_string()).or_insert_with(|| {
                let t = raea_core_shape::cached(shape).ok()?;
                let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for p in &t.positions {
                    lo = lo.min(Vec3::from(*p));
                    hi = hi.max(Vec3::from(*p));
                }
                (lo.x <= hi.x).then_some((lo, hi))
            })
        }
        "text" => {
            let words = it.props.get("text").and_then(serde_json::Value::as_str).unwrap_or("");
            let size = num(it, "size", 0.5) as f32;
            let w = words.split('\n').map(|l| l.chars().count()).max().unwrap_or(0) as f32 * 0.55 * size;
            let h = words.split('\n').count() as f32 * 1.2 * size;
            Some((Vec3::new(-w / 2.0, -h / 2.0, 0.0), Vec3::new(w / 2.0, h / 2.0, 0.0)))
        }
        // a picture on a card: a unit quad scaled by its props (ImagePlane, Screen)
        "panel" => Some((Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 0.0))),
        _ => None,
    }
}

/// A slab wider than BACKDROP_M in the world, its thinnest side under SLAB of its widest: a floor, a set's wall, a sky
/// plane, the backdrop a frame's parts stand in front of, not one of them.
pub(crate) fn backdrop(it: &raea_core_scene::Item, lo: Vec3, hi: Vec3) -> bool {
    let ext = (hi - lo) * it.world.to_scale_rotation_translation().0.abs();
    ext.max_element() > BACKDROP_M && ext.min_element() < SLAB * ext.max_element()
}

/// Each part of known size that shows in a frame: its name as the checks call it, and its box on screen, [x0, y0, x1,
/// y1] in px (SG9 names the parts under a patch that differs between a frame rendered in sequence and alone).
pub fn on_screen(its: &[raea_core_scene::Item], w: u32, h: u32) -> Vec<(String, [f32; 4])> {
    let Some(v) = crate::view::View::of(its, w, h) else { return Vec::new() };
    let mut boxes = HashMap::new();
    let seen = |it: &raea_core_scene::Item, boxes: &mut HashMap<_, _>| {
        let (lo, hi) = own_box(it, boxes)?;
        v.bounds(corners(&it.world, lo, hi)).filter(|b| v.share(*b) > 0.0)
    };
    its.iter().filter_map(|it| Some((called(it), seen(it, &mut boxes)?))).collect()
}

/// The coverage grid's cells under a box on screen ([x0, y0, x1, y1] px, clipped to the w x h frame), set.
fn mark(grid: &mut [bool], b: [f32; 4], w: f32, h: f32) {
    let (x0, y0) = ((b[0].max(0.0) / w * GX as f32) as usize, (b[1].max(0.0) / h * GY as f32) as usize);
    let (x1, y1) = (((b[2].min(w) / w * GX as f32).ceil() as usize).min(GX), ((b[3].min(h) / h * GY as f32).ceil() as usize).min(GY));
    for y in y0..y1 {
        for x in x0..x1 {
            grid[y * GX + x] = true;
        }
    }
}

pub(crate) fn check(st: &Stage, page: &[crate::Page]) -> Vec<Finding> {
    let n = st.all.len();
    let mut boxes = HashMap::new();
    // each frame: the share covered, whether it can be judged (no part of unknown size in view, no footage or 2D
    // over the 3D), and its subject: (name, its box on screen, the share of its box outside the frame)
    let mut covered = vec![1.0f32; n];
    let mut judged = vec![false; n];
    let mut subject: Vec<Option<(String, f32, bool)>> = vec![None; n];
    for (k, its) in st.all.iter().enumerate() {
        let Some(v) = st.views[k] else { continue };
        let mut grid = vec![false; GX * GY];
        let mut sure = true;
        let mut best: Option<(String, f32, f32, bool)> = None;
        for it in its {
            if matches!(it.kind, "plate" | "sprite" | "shape2d" | "grass" | "splat" | "meshseq") {
                sure = false;
                continue;
            }
            let Some((lo, hi)) = own_box(it, &mut boxes) else {
                // a model or other part of unknown size: judged only when its origin is well out of view
                if matches!(it.kind, "model" | "entity") {
                    if let Some([x, y, _]) = v.project(super::origin(it)) {
                        if x > -0.5 * st.w && x < 1.5 * st.w && y > -0.5 * st.h && y < 1.5 * st.h {
                            sure = false;
                        }
                    }
                }
                continue;
            };
            // a floor or wall round the camera is backdrop before it is a part behind it
            if backdrop(it, lo, hi) {
                continue;
            }
            let Some(b) = v.bounds(corners(&it.world, lo, hi)) else {
                // across the camera's plane it may fill the view; wholly behind it, it's out of view
                sure &= corners(&it.world, lo, hi).into_iter().all(|p| v.project(p).is_none());
                continue;
            };
            let inside = v.share(b);
            let whole = ((b[2] - b[0]) * (b[3] - b[1]) / (st.w * st.h)).max(1e-9);
            if inside > 0.0 {
                mark(&mut grid, b, st.w, st.h);
            }
            // the subject: the named part showing most, when it could fit in the frame
            if it.props.get("name").is_some() && inside > 0.0 && best.as_ref().is_none_or(|s| inside > s.1) {
                let fits = b[2] - b[0] < 0.9 * st.w && b[3] - b[1] < 0.9 * st.h;
                best = Some((called(it), inside, 1.0 - inside / whole, fits && scale_of(it) > 0.0));
            }
        }
        // the page's words and videos fill the frame too: a headline over an empty 3D isn't an empty frame
        for b in page.get(k).into_iter().flat_map(|p| p.words.iter().map(|w| w.bx).chain(p.videos.iter().copied()).chain(p.pictures.iter().copied())) {
            mark(&mut grid, b, st.w, st.h);
        }
        covered[k] = grid.iter().filter(|c| **c).count() as f32 / (GX * GY) as f32;
        judged[k] = sure;
        subject[k] = best.map(|(name, _, out, fits)| (name, out, fits));
    }
    let mut out = Vec::new();
    let total = st.secs(0, n.saturating_sub(1).max(0));
    for (a, b) in runs(st, n, |k| judged[k] && covered[k] < EMPTY) {
        let (t0, t1) = ((st.frames[a] - st.frames[0]) as f64 / st.fps, (st.frames[b] - st.frames[0] + 1) as f64 / st.fps);
        if st.secs(a, b) >= EMPTY_SECS && t0 > FADE_SECS && t1 < total - FADE_SECS {
            let least = covered[a..=b].iter().copied().fold(1.0, f32::min);
            let says = format!("almost nothing is on screen from {} to {} ({:.0}% of the frame shows any part)", st.at(a), st.at(b), least * 100.0);
            out.push(st.finding("frame-empty", Level::Warn, a, b, says).fix("aim the camera at what the shot is about, or bring it into the frame"));
        }
    }
    for (a, b) in runs(st, n, |k| subject[k].as_ref().is_some_and(|(_, out, fits)| *fits && *out > CUT_LO && *out < CUT_HI)) {
        let names: Vec<&str> = subject[a..=b].iter().filter_map(|s| s.as_ref().map(|s| s.0.as_str())).collect();
        if b + 1 - a > CUT_FRAMES && names.windows(2).all(|w| w[0] == w[1]) {
            let says = format!("'{}', the part the shot shows most, is cut by the frame's edge from {} to {}", names[0], st.at(a), st.at(b));
            out.push(st.finding("subject-cut", Level::Warn, a, b, says).fix("frame it whole (move the camera back or aim it), or move in until the cut is plainly meant"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::test_items::{cam, item};
    use super::super::*;
    use bevy::math::Mat4;
    use serde_json::json;

    fn run(frames: Vec<Vec<Item>>) -> Vec<Finding> {
        let n: Vec<i64> = (0..frames.len() as i64).collect();
        check(&frames, &n, 1920, 1080, 30.0, "test")
    }

    fn cube(name: &str, at: [f32; 3]) -> Item {
        let mut it = item("mesh", json!({"name": name}), Mat4::from_translation(Vec3::from(at)));
        it.shape = Some("box(1, 1, 1)".into());
        it
    }

    #[test]
    fn a_camera_aimed_past_everything_is_an_empty_frame() {
        let fs: Vec<Vec<Item>> = (0..120).map(|i| vec![cam([0.0, 0.0, 6.0], if (30..90).contains(&i) { [0.0, 40.0, 0.0] } else { [0.0; 3] }), cube("dash", [0.0; 3])]).collect();
        let f = run(fs);
        assert!(f.iter().any(|f| f.check == "frame-empty"), "{f:?}");
        let framed: Vec<Vec<Item>> = (0..120).map(|_| vec![cam([0.0, 0.0, 4.0], [0.0; 3]), cube("dash", [0.0; 3])]).collect();
        assert!(!run(framed).iter().any(|f| f.check == "frame-empty"));
    }

    #[test]
    fn words_or_a_video_in_the_page_fill_the_frame() {
        let fs: Vec<Vec<Item>> = (0..120).map(|i| vec![cam([0.0, 0.0, 6.0], if (30..90).contains(&i) { [0.0, 40.0, 0.0] } else { [0.0; 3] }), cube("dash", [0.0; 3])]).collect();
        let n: Vec<i64> = (0..120).collect();
        let headline = crate::Page { words: vec![crate::Words { text: "The rule".into(), bx: [360.0, 380.0, 1560.0, 700.0], allow_overlap: false }], videos: Vec::new(), pictures: Vec::new() };
        let f = crate::check_with(&fs, &n, &vec![headline; 120], 1920, 1080, 30.0, "test");
        assert!(!f.iter().any(|f| f.check == "frame-empty"), "{f:?}");
        let clip = crate::Page { words: Vec::new(), videos: Vec::new(), pictures: vec![[656.0, 0.0, 1264.0, 1080.0]] };
        assert!(!crate::check_with(&fs, &n, &vec![clip; 120], 1920, 1080, 30.0, "test").iter().any(|f| f.check == "frame-empty"));
    }

    #[test]
    fn a_floor_filling_the_frame_is_no_content() {
        let mut floor = item("mesh", json!({"name": "floor"}), Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)));
        floor.shape = Some("box(100, 0.1, 100)".into());
        // a 4 m crate to the left, filling a sixth of the frame; for two seconds the camera looks along the floor, away
        let mut crate_ = cube("crate", [-5.0, 0.0, 0.0]);
        crate_.shape = Some("box(4, 4, 4)".into());
        let fs: Vec<Vec<Item>> = (0..120).map(|i| vec![cam([0.0, 1.0, 6.0], if (30..90).contains(&i) { [40.0, -1.0, 6.0] } else { [-5.0, 0.0, 0.0] }), floor.clone(), crate_.clone()]).collect();
        assert!(run(fs).iter().any(|f| f.check == "frame-empty"));
    }

    #[test]
    fn parts_on_screen_are_named_with_their_boxes() {
        let its = vec![cam([0.0, 0.0, 6.0], [0.0; 3]), cube("dash", [0.0; 3]), cube("gone", [0.0, 0.0, 20.0])];
        let on = super::on_screen(&its, 1920, 1080);
        assert_eq!(on.len(), 1, "{on:?}");
        let (name, b) = &on[0];
        assert!(name.contains("dash"), "{name}");
        assert!(b[0] < 960.0 && b[2] > 960.0 && b[1] < 540.0 && b[3] > 540.0, "{b:?}");
    }

    #[test]
    fn the_shots_subject_half_out_of_frame_is_cut() {
        // the cube's centre near the frame's right edge
        let fs: Vec<Vec<Item>> = (0..30).map(|_| vec![cam([0.0, 0.0, 6.0], [0.0; 3]), cube("logo", [4.9, 0.0, 0.0])]).collect();
        assert!(run(fs).iter().any(|f| f.check == "subject-cut"));
    }
}
