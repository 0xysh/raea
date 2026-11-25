//! `raea light steady EDIT.tsx --frame N -o DIR`: whether the light holds still. Frame N is
//! drawn again and again (`raea render --hold`), so the scene and the camera don't move, and whatever still changes on
//! screen is the renderer's own. Two things change: grain, a pixel or two wide and new every frame, which a denoiser
//! and the eye average away; and blotches, patches tens of pixels wide that drift over lit floors and reflections for
//! a quarter of a second or more, which nothing averages away. Solari's world cache makes them when its cells are
//! lit by too few rays (MALL RAT, 11-24: "this noisy splotchy thing in the lights and reflections").
//!
//! The two are told apart by size. Each SMALL x SMALL and each LARGE x LARGE square's mean luminance is followed over
//! the held frames, once the light has built: its deviation in time over its mean. Grain's falls by the square's side
//! (LARGE / SMALL = 4 times less in the large squares); a blotch as wide as the large square keeps its deviation.
//! `blotch` is the large squares' deviation (rms over the lit ones, percent), `worst` the worst TILE's, and `spread`
//! the large squares' deviation over the small squares' (0.25 is grain alone, toward 1 is blotches). A run over
//! `--max` fails, as does a light whose level drifts (`drift`, stops between the first and the last quarter of the
//! measured frames: a cache gaining or losing energy the longer it runs), and DIR/steady.png shows where: each large square by its deviation, white at MAX_SHOWN percent.

use crate::metric::{self, Luma};
use raea_runtime_bundle::{flag, R};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use crate::USAGE;

const SMALL: usize = 8;
const LARGE: usize = 32;
/// The large squares a side of the tile the worst is found among.
const TILE: usize = 4;
/// A square darker than this (linear) isn't lit enough for its deviation to show.
const LIT: f64 = 0.004;
/// The deviation steady.png draws as white, percent.
const MAX_SHOWN: f64 = 6.0;
/// What a run may show before it fails, percent, unless `--max` says: a held raster frame reads 0, Solari with a
/// built cache under 1.
const MAX: f64 = 1.5;
/// How far the held frame's level may move between the first and the last quarter of the measured frames, in stops.
const MAX_DRIFT: f64 = 0.05;

/// Each square's mean and deviation in time over `frames`, side `k`: (w, h, [(mean, deviation / mean)]).
fn squares(frames: &[Luma], k: usize) -> (usize, usize, Vec<(f64, f64)>) {
    let first = frames[0].blocks(k);
    let mut sums = vec![(0.0f64, 0.0f64); first.y.len()];
    for f in frames {
        for (s, &v) in sums.iter_mut().zip(&f.blocks(k).y) {
            s.0 += v as f64;
            s.1 += (v as f64) * (v as f64);
        }
    }
    let n = frames.len() as f64;
    let out = sums.into_iter().map(|(a, b)| { let m = a / n; (m, (b / n - m * m).max(0.0).sqrt() / m.max(LIT)) }).collect();
    (first.w, first.h, out)
}

fn rms(v: impl Iterator<Item = f64>) -> f64 {
    let (s, n) = v.fold((0.0, 0usize), |(s, n), x| (s + x * x, n + 1));
    if n == 0 { 0.0 } else { (s / n as f64).sqrt() }
}

/// The measure of held frames (the module's doc), and steady.png's picture: the large squares' deviation.
pub fn measure(frames: &[Luma]) -> R<(Value, image::GrayImage)> {
    if frames.len() < 8 {
        return Err(format!("{} held frames: 8 at least say whether the light holds still", frames.len()));
    }
    let (_, _, small) = squares(frames, SMALL);
    let (w, h, large) = squares(frames, LARGE);
    let lit = |s: &&(f64, f64)| s.0 > LIT;
    let (blotch, grain) = (rms(large.iter().filter(lit).map(|s| s.1)), rms(small.iter().filter(lit).map(|s| s.1)));
    let mut worst = (0.0f64, 0usize, 0usize);
    for ty in 0..h.div_ceil(TILE) {
        for tx in 0..w.div_ceil(TILE) {
            let cells = (0..TILE * TILE).filter_map(|i| large.get((ty * TILE + i / TILE) * w + tx * TILE + i % TILE).filter(|_| tx * TILE + i % TILE < w)).filter(lit);
            let v = rms(cells.map(|s| s.1));
            if v > worst.0 {
                worst = (v, tx * TILE * LARGE, ty * TILE * LARGE);
            }
        }
    }
    // at the frame's size, so it lays over the frame
    let map = image::GrayImage::from_fn((w * LARGE) as u32, (h * LARGE) as u32, |x, y| {
        let s = large[y as usize / LARGE * w + x as usize / LARGE];
        image::Luma([if s.0 > LIT { (s.1 * 100.0 / MAX_SHOWN * 255.0).min(255.0) as u8 } else { 0 }])
    });
    let pct = |v: f64| (v * 1e4).round() / 100.0;
    // the light's level over the held frames: a cache gaining or losing energy is a ramp, not patches
    let level = |f: &[Luma]| f.iter().map(|f| f.y.iter().map(|&v| v as f64).sum::<f64>() / f.y.len().max(1) as f64).sum::<f64>() / f.len() as f64;
    let q = (frames.len() / 4).max(1);
    let drift = (level(&frames[frames.len() - q..]).max(1e-6) / level(&frames[..q]).max(1e-6)).log2();
    let report = json!({
        "drift": (drift * 1e3).round() / 1e3,
        "frames": frames.len(), "blotch": pct(blotch), "grain": pct(grain), "spread": ((blotch / grain.max(1e-9)) * 100.0).round() / 100.0,
        "worst": pct(worst.0), "worst_at": [worst.1, worst.2], "lit": ((large.iter().filter(lit).count() as f64 / large.len().max(1) as f64) * 100.0).round() / 100.0,
    });
    Ok((report, map))
}

/// The PNG frames of a folder, in order.
fn held(dir: &Path) -> R<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "png") && p.file_name().is_some_and(|n| n != "steady.png")).collect();
    v.sort();
    Ok(v)
}

/// Frame `frame` of a composition held for `n` frames into `out` (`raea render --hold`), run in `dir` under `env`.
pub fn hold(dir: &Path, file: &str, composition: Option<&str>, frame: &str, n: usize, env: &[(&str, String)], out: &Path) -> R<()> {
    let _ = std::fs::remove_dir_all(out);
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut cmd = std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    cmd.current_dir(dir).envs(env.iter().map(|(k, v)| (*k, v)));
    cmd.args(["render", file, "--hold", &format!("{frame}:{n}"), "-o"]).arg(format!("{}/", out.display()));
    if let Some(c) = composition {
        cmd.args(["--composition", c]);
    }
    let run = cmd.args(["--despite", "clips", "raea light steady measures light, not placement"]).output().map_err(|e| e.to_string())?;
    if !run.status.success() {
        let err = String::from_utf8_lossy(&run.stderr);
        return Err(format!("the held render failed: {}", err.lines().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(" | ")));
    }
    Ok(())
}

/// The held frames of `dir` measured: DIR/steady.png and steady.json written, and the report, its "steady" saying
/// whether the light held (`max`: the blotch a run may show, percent; MAX when None).
pub fn report(dir: &Path, max: Option<f64>) -> R<Value> {
    let max = max.unwrap_or(MAX);
    // the second half: the first is the light building from nothing, which a render's lead-in covers
    let files = held(dir)?;
    let frames: Vec<Luma> = files[files.len() / 2..].iter().map(|p| metric::load(p)).collect::<R<_>>()?;
    let (mut report, map) = measure(&frames)?;
    let png = dir.join("steady.png");
    map.save(&png).map_err(|e| format!("{}: {e}", png.display()))?;
    let get = |k: &str| report[k].as_f64().unwrap_or(0.0);
    let ok = get("blotch") <= max && get("worst") <= max * 3.0 && get("drift").abs() <= MAX_DRIFT;
    report["max"] = json!(max);
    report["max_drift"] = json!(MAX_DRIFT);
    report["steady"] = json!(ok);
    report["picture"] = json!(png);
    std::fs::write(dir.join("steady.json"), serde_json::to_string_pretty(&report).unwrap_or_default()).map_err(|e| e.to_string())?;
    Ok(report)
}

/// A report as one line.
pub fn line(r: &Value) -> String {
    format!(
        "drift {} stops (max {}), blotch {}% (max {}), worst {}% at {},{}, grain {}%, spread {} over {} held frames; {}",
        r["drift"], r["max_drift"], r["blotch"], r["max"], r["worst"], r["worst_at"][0], r["worst_at"][1], r["grain"], r["spread"], r["frames"], r["picture"].as_str().unwrap_or("")
    )
}

/// What a light that doesn't hold still is told.
pub const WHY: &str = "On Solari this is the world cache lit by too few rays, or gaining or losing energy the longer it runs: bake it (cache.bake, then cache.load), or give it a longer history";

pub fn steady(args: &[String]) -> R<String> {
    let first = args.first().ok_or(format!("usage: {USAGE}"))?;
    let max = flag(args, "--max").map(|m| m.parse::<f64>().map_err(|_| format!("--max takes a percent, not {m}"))).transpose()?;
    let dir = if Path::new(first).is_dir() {
        PathBuf::from(first)
    } else {
        let frame = flag(args, "--frame").ok_or(format!("usage: {USAGE}"))?;
        let n: usize = flag(args, "--frames").map(|n| n.parse().map_err(|_| format!("--frames takes a count, not {n}"))).transpose()?.unwrap_or(96);
        let out = PathBuf::from(flag(args, "-o").ok_or(format!("usage: {USAGE}"))?).join("held");
        hold(Path::new("."), first, flag(args, "--composition").as_deref(), &frame, n, &[], &out)?;
        out
    };
    let r = report(&dir, max)?;
    if r["steady"] == json!(true) { Ok(format!("STEADY  {}", line(&r))) } else { Err(format!("the light doesn't hold still: {}. {WHY}", line(&r))) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frames of a flat grey, each pixel moved by `grain` (new every frame and pixel) and each PATCH x PATCH patch by
    /// `blotch` (new every frame, the same across the patch).
    fn frames(grain: f32, blotch: f32) -> Vec<Luma> {
        const PATCH: usize = 40;
        let (w, h) = (320, 240);
        let mut seed = 7u32;
        let mut rnd = move || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        (0..24)
            .map(|_| {
                let patches: Vec<f32> = (0..(w / PATCH + 1) * (h / PATCH + 1)).map(|_| rnd() * blotch).collect();
                Luma { w, h, y: (0..w * h).map(|i| 0.3 * (1.0 + rnd() * grain + patches[(i / w / PATCH) * (w / PATCH + 1) + (i % w) / PATCH])).collect() }
            })
            .collect()
    }

    #[test]
    fn grain_is_steady_and_blotches_are_not() {
        let still = measure(&frames(0.0, 0.0)).unwrap().0;
        assert_eq!(still["blotch"], 0.0);
        assert_eq!(still["drift"], 0.0);
        // a light that doubles over the held frames: about 0.8 of a stop between the first and the last quarter
        let mut rising = frames(0.0, 0.0);
        let n = rising.len() as f32;
        rising.iter_mut().enumerate().for_each(|(t, f)| f.y.iter_mut().for_each(|v| *v *= 1.0 + t as f32 / n));
        assert!(measure(&rising).unwrap().0["drift"].as_f64().unwrap() > 0.5);
        // heavy grain, 30% a pixel: under 1% across a 32 px square, and a spread near 0.25
        let grain = measure(&frames(0.3, 0.0)).unwrap().0;
        assert!(grain["blotch"].as_f64().unwrap() < 1.0, "{grain}");
        assert!(grain["spread"].as_f64().unwrap() < 0.35, "{grain}");
        // patches moving 5%: the large squares keep it
        let blotch = measure(&frames(0.05, 0.05)).unwrap().0;
        assert!(blotch["blotch"].as_f64().unwrap() > 2.0, "{blotch}");
        assert!(blotch["spread"].as_f64().unwrap() > 0.6, "{blotch}");
    }
}
