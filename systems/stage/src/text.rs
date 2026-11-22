//! SG10, 3D type that reads: each RichText checked on every frame it shows, through the frame's camera. Its letters'
//! height on screen, its face against the line to the camera, its box against the title-safe area, whether its reveal
//! starts and finishes while it's up, and whether it's up long enough to be read. A RichText with `read={false}` is
//! scenery (a sign seen in passing) and isn't held to it.

use std::collections::BTreeMap;

use bevy::math::Vec3;
use raea_core_facts::{Finding, Level};
use serde_json::Value;

use super::{Stage, called, num, origin, runs};
use crate::view::corners;

/// A capital's height, as a share of the em (Inter's is 0.727; most text faces 0.66 to 0.75)
const CAP: f32 = 0.7;
/// The smallest capital on screen, in px of a 1080-high frame (scaled to the frame)
const MIN_CAP: f32 = 22.0;
/// The most its face may turn from the line to the camera, degrees: past it a warning (squashed), past TURN_ERROR an
/// error (squashed to under 64%: floor labels seen low)
const MAX_TURN: f32 = 35.0;
const TURN_ERROR: f32 = 50.0;
/// The title-safe area's inset from each edge, a share of the frame
const SAFE: f32 = 0.05;
/// How long a fault must last while the words are up to be said, seconds (a fly-past isn't a label)
const LASTS: f64 = 0.5;
/// A glyph counts as shown above this opacity, and fully in above the next
const SHOWN: f64 = 0.05;
const IN: f64 = 0.9;
/// Still enough to read: its words move less than this share of the frame's width a frame
const STILL: f32 = 0.01;
/// An average glyph's advance, a share of the em, for the block's width
const ADVANCE: f32 = 0.55;

/// One frame of one block of words, as the camera sees it.
#[derive(Clone, Debug)]
struct Seen {
    k: usize,
    /// a capital's height on screen, px
    cap: f32,
    /// its face's turn from the line to the camera, degrees (over 90: seen from behind)
    turn: f32,
    /// its box on screen, None when part of it is behind the camera
    b: Option<[f32; 4]>,
    /// its centre on screen
    c: [f32; 2],
    /// some glyph shown, every glyph fully in
    shown: bool,
    whole: bool,
}

pub(crate) fn check(st: &Stage) -> Vec<Finding> {
    let min_cap = MIN_CAP * st.h / 1080.0;
    // each block's frames, by what it's called, in frame order
    let mut by: BTreeMap<String, (Vec<Seen>, usize, bool)> = BTreeMap::new();
    for (k, its) in st.all.iter().enumerate() {
        let Some(v) = st.views[k] else { continue };
        for it in its.iter().filter(|i| i.kind == "text") {
            if it.props.get("read").and_then(Value::as_bool) == Some(false) {
                continue;
            }
            let words = it.props.get("text").and_then(Value::as_str).unwrap_or("");
            if words.trim().is_empty() {
                continue;
            }
            let o = origin(it);
            let Some([cx, cy, depth]) = v.project(o) else { continue };
            // its em on screen: the em's height carried by its matrix, at its depth
            let size = num(it, "size", 0.5) as f32;
            let up = it.world.transform_vector3(Vec3::Y * size);
            let cap = CAP * up.length() * v.focal / depth;
            let face = it.world.transform_vector3(Vec3::Z).normalize_or(Vec3::Z);
            let turn = face.dot((v.eye - o).normalize_or(Vec3::Z)).clamp(-1.0, 1.0).acos().to_degrees();
            let lines: Vec<&str> = words.split('\n').collect();
            let wide = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f32 * ADVANCE * size;
            let tall = lines.len() as f32 * num(it, "lineHeight", 1.2) as f32 * size;
            let align = num(it, "align", 0.5) as f32;
            let (x0, x1) = (-align * wide, (1.0 - align) * wide);
            let b = v.bounds(corners(&it.world, Vec3::new(x0, -tall / 2.0, 0.0), Vec3::new(x1, tall / 2.0, 0.0)));
            let os: Vec<f64> = it.props.get("glyphs").and_then(Value::as_array).map(|gs| gs.iter().map(|g| if g.get("v").and_then(Value::as_bool) == Some(false) { 0.0 } else { g.get("o").and_then(Value::as_f64).unwrap_or(1.0) }).collect()).unwrap_or_default();
            let shown = os.is_empty() || os.iter().any(|o| *o > SHOWN);
            let whole = os.iter().all(|o| *o > IN);
            let e = by.entry(called(it)).or_insert_with(|| (Vec::new(), words.split_whitespace().count(), it.props.get("at").is_some()));
            e.0.push(Seen { k, cap, turn, b, c: [cx, cy], shown, whole });
        }
    }
    let mut out = Vec::new();
    for (name, (seen, words, has_at)) in &by {
        // the frames it's up and in frame
        let up: Vec<&Seen> = seen.iter().filter(|s| s.shown && in_frame(st, s.c)).collect();
        // the frames of a run (up is in frame order)
        let during = |a: usize, b: usize| up.iter().skip_while(move |s| s.k < a).take_while(move |s| s.k <= b);
        let lasting = |bad: &dyn Fn(&Seen) -> bool| -> Vec<(usize, usize)> {
            let flagged: std::collections::HashSet<usize> = up.iter().filter(|s| bad(s)).map(|s| s.k).collect();
            runs(st, st.all.len(), |k| flagged.contains(&k)).into_iter().filter(|(a, b)| st.secs(*a, *b) >= LASTS || (a == b && st.alone(*a))).collect()
        };
        for (a, b) in lasting(&|s| s.cap < min_cap) {
            let least = during(a, b).map(|s| s.cap).fold(f32::MAX, f32::min);
            let f = st.finding("type-small", Level::Error, a, b, format!("the words '{name}' stand {least:.0} px high on screen from {} to {} (capitals at least {min_cap:.0} px at this frame size)", st.at(a), st.at(b)));
            out.push(fix(f, "bring the camera nearer, or make its size larger: a label that points at a part reads better as page type laid over the frame; scenery nobody is meant to read takes read={false}", least, min_cap, "px"));
        }
        for (a, b) in lasting(&|s| s.turn > MAX_TURN) {
            let most = during(a, b).map(|s| s.turn).fold(0.0, f32::max);
            let says = if most > 90.0 { format!("the words '{name}' are seen from behind (turned {most:.0}°) from {} to {}", st.at(a), st.at(b)) } else { format!("the words '{name}' are turned {most:.0}° from the camera from {} to {} (at most {MAX_TURN:.0}°), so they read squashed", st.at(a), st.at(b)) };
            let level = if most > TURN_ERROR { Level::Error } else { Level::Warn };
            out.push(fix(st.finding("type-angle", level, a, b, says), "turn the words to face the camera (their rotation), or move the camera round to face them", most, MAX_TURN, "°"));
        }
        let (mx, my) = (SAFE * st.w, SAFE * st.h);
        for (a, b) in lasting(&|s| s.whole && s.b.is_some_and(|b| b[0] < mx || b[1] < my || b[2] > st.w - mx || b[3] > st.h - my)) {
            let f = st.finding("type-safe", Level::Warn, a, b, format!("the words '{name}' run outside the title-safe area from {} to {}", st.at(a), st.at(b)));
            out.push(f.fix("move them in, or the camera back, so the whole block sits inside 5% of each edge"));
        }
        // its reveal: never started while it's in frame, or never finished before it goes (out of view with frames still
        // stepped after: a still's few frames end, the words don't go); only over frames all stepped (stills far apart
        // say nothing of what came between)
        let in_view: Vec<&Seen> = seen.iter().filter(|s| in_frame(st, s.c)).collect();
        if let (Some(f), Some(l)) = (in_view.first(), in_view.last()) && st.stepped(f.k, l.k) {
            let long = st.secs(f.k, l.k) >= 1.0;
            if *has_at && long && in_view.iter().all(|s| !s.shown) {
                let f = st.finding("type-reveal", Level::Error, f.k, l.k, format!("the words '{name}' are never shown: their reveal (`at`) never starts while they're up, {} to {}", st.at(f.k), st.at(l.k)));
                out.push(f.fix("give `at` in the frames of the Sequence they play in (useCurrentFrame), never a frame of another clock"));
            } else if l.k + 1 < st.all.len() && in_view.iter().any(|s| s.shown) && !in_view.iter().any(|s| s.whole) {
                let f = st.finding("type-reveal", Level::Error, f.k, l.k, format!("the words '{name}' never finish their reveal before they go ({} to {})", st.at(f.k), st.at(l.k)));
                out.push(f.fix("start the reveal earlier, make it quicker (`each`), or keep the words up longer"));
            }
        }
        // read: still, whole, big enough and facing the camera for their reading time, all told
        let need = *words as f64 / 3.0 + 0.5;
        let readable = up.windows(2).filter(|w| w[0].k + 1 == w[1].k && !st.cut[w[1].k] && readable(st, w[0], w[1], min_cap)).count() as f64 / st.fps;
        if let (Some(f), Some(l)) = (up.first(), up.last()) && st.stepped(f.k, l.k) {
            if readable + 1e-9 < need && up.iter().any(|s| s.whole) {
                let f = st.finding("type-time", Level::Warn, f.k, l.k, format!("the words '{name}' can be read for {readable:.1} s; their {words} words need {need:.1} s (words ÷ 3 a second, plus 0.5 s)"));
                out.push(fix(f, "hold them still, whole and facing the camera longer before they or the camera move", readable as f32, need as f32, "s"));
            }
        }
    }
    out
}

fn in_frame(st: &Stage, c: [f32; 2]) -> bool {
    c[0] >= 0.0 && c[0] <= st.w && c[1] >= 0.0 && c[1] <= st.h
}

fn readable(st: &Stage, a: &Seen, b: &Seen, min_cap: f32) -> bool {
    let moved = ((b.c[0] - a.c[0]).powi(2) + (b.c[1] - a.c[1]).powi(2)).sqrt() / st.w;
    b.whole && b.cap >= min_cap && b.turn <= MAX_TURN && moved < STILL
}

fn fix(f: Finding, how: &str, value: f32, limit: f32, unit: &str) -> Finding {
    let mut f = f.fix(how);
    f.measure = Some(raea_core_facts::finding::Measure { name: f.check.clone(), value: value as f64, unit: unit.into(), limit: Some(limit as f64) });
    f
}

#[cfg(test)]
mod tests {
    use super::super::test_items::{cam, item};
    use super::super::*;
    use bevy::math::{Mat4, Quat};
    use serde_json::json;

    fn run(frames: Vec<Vec<Item>>) -> Vec<Finding> {
        let n: Vec<i64> = (0..frames.len() as i64).collect();
        check(&frames, &n, 1920, 1080, 30.0, "test")
    }

    fn words(text: &str, size: f64, world: Mat4, glyph_o: f64) -> Item {
        let gs: Vec<Value> = text.chars().filter(|c| !c.is_whitespace()).map(|_| json!({"o": glyph_o})).collect();
        item("text", json!({"text": text, "size": size, "glyphs": gs, "at": 0}), world)
    }

    #[test]
    fn small_type_far_away_is_said_and_near_type_isnt() {
        // 0.1 m em at 10 m through 50 degrees: about 8 px capitals
        let far: Vec<Vec<Item>> = (0..30).map(|_| vec![cam([0.0, 0.0, 10.0], [0.0; 3]), words("Placement wall", 0.1, Mat4::IDENTITY, 1.0)]).collect();
        let f = run(far);
        assert!(f.iter().any(|f| f.check == "type-small" && f.level == Level::Error), "{f:?}");
        let near: Vec<Vec<Item>> = (0..90).map(|_| vec![cam([0.0, 0.0, 4.0], [0.0; 3]), words("Placement", 0.4, Mat4::IDENTITY, 1.0)]).collect();
        assert_eq!(run(near).iter().map(|f| f.check.as_str()).collect::<Vec<_>>(), Vec::<&str>::new());
    }

    #[test]
    fn type_on_the_floor_seen_low_is_turned_too_far() {
        // lying flat (facing +y), the camera low and in front
        let flat = Mat4::from_quat(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
        let fs: Vec<Vec<Item>> = (0..30).map(|_| vec![cam([0.0, 1.0, 5.0], [0.0; 3]), words("Floor", 0.6, flat, 1.0)]).collect();
        let f = run(fs);
        assert!(f.iter().any(|f| f.check == "type-angle"), "{f:?}");
        // and scenery isn't held to it
        let mut sign = words("Floor", 0.6, flat, 1.0);
        std::sync::Arc::make_mut(&mut sign.props).insert("read".into(), json!(false));
        assert!(run((0..30).map(|_| vec![cam([0.0, 1.0, 5.0], [0.0; 3]), sign.clone()]).collect()).is_empty());
    }

    #[test]
    fn a_reveal_that_never_starts_or_never_ends_is_said() {
        let never: Vec<Vec<Item>> = (0..60).map(|_| vec![cam([0.0, 0.0, 4.0], [0.0; 3]), words("Five stars", 0.4, Mat4::IDENTITY, 0.0)]).collect();
        assert!(run(never).iter().any(|f| f.check == "type-reveal" && f.says.contains("never shown")));
        // half revealed, then gone with the piece going on
        let half: Vec<Vec<Item>> = (0..60).map(|i| { let mut v = vec![cam([0.0, 0.0, 4.0], [0.0; 3])]; if i < 40 { v.push(words("Five stars", 0.4, Mat4::IDENTITY, 0.5)) } v }).collect();
        assert!(run(half).iter().any(|f| f.check == "type-reveal" && f.says.contains("never finish")));
        // a still's own frames (raea render --frame 10 steps its neighbours): half revealed in each, nothing gone
        let still: Vec<Vec<Item>> = (0..3).map(|_| vec![cam([0.0, 0.0, 4.0], [0.0; 3]), words("Five stars", 0.4, Mat4::IDENTITY, 0.5)]).collect();
        assert!(check(&still, &[9, 10, 11], 1920, 1080, 30.0, "test").is_empty());
    }

    #[test]
    fn stills_far_apart_are_each_judged_but_say_nothing_of_time() {
        // raea check --quick's three stills: small type in each is said; half revealed in each says nothing of the reveal
        let at = [75i64, 150, 299];
        let small: Vec<Vec<Item>> = at.iter().map(|_| vec![cam([0.0, 0.0, 10.0], [0.0; 3]), words("Placement wall", 0.1, Mat4::IDENTITY, 1.0)]).collect();
        assert!(check(&small, &at, 1920, 1080, 30.0, "test").iter().any(|f| f.check == "type-small"));
        let half: Vec<Vec<Item>> = at.iter().map(|_| vec![cam([0.0, 0.0, 4.0], [0.0; 3]), words("Five stars", 0.4, Mat4::IDENTITY, 0.5)]).collect();
        assert!(check(&half, &at, 1920, 1080, 30.0, "test").is_empty());
    }

    #[test]
    fn words_up_too_briefly_to_read_are_a_warning() {
        let brief: Vec<Vec<Item>> = (0..20).map(|_| vec![cam([0.0, 0.0, 4.0], [0.0; 3]), words("The best ad network for growing studios", 0.3, Mat4::IDENTITY, 1.0)]).collect();
        let f = run(brief);
        let t = f.iter().find(|f| f.check == "type-time").expect("type-time");
        assert_eq!(t.level, Level::Warn);
    }
}
