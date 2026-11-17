//! `raea light check SCENES.json -o DIR` (docs/realism.feature LT1): the bar raea Light is measured by. Each scene
//! is rendered on each light path (RAEA_LIGHT, engine/render/raytrace force.rs) and against its reference, the path
//! tracer held RAEA_LIGHT_SPP draws a frame: how far each path's stills are from it in stops (mean, bias, worst tile),
//! how much its moving shot flickers, and what a frame costs. The numbers go to DIR/report.json and a table.
//!
//! A scene in SCENES.json: { "name", "dir" (its project, where its renders run; ~ is the home folder), "file" (in dir),
//! "composition", "stills": [frame, ...], "shot": [first, last] (optional), "reference": false (optional: its
//! reference can't be made yet, and why, in "why") }.

pub mod metric;

use raea_runtime_bundle::{flag, R};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const USAGE: &str = "raea light check SCENES.json -o DIR [--scene NAME] [--paths raster,solari] [--spp N] [--fresh]";

/// The light paths a check compares, as RAEA_LIGHT names them.
const PATHS: [&str; 2] = ["raster", "solari"];

struct Scene {
    name: String,
    dir: PathBuf,
    file: String,
    composition: String,
    stills: Vec<i64>,
    shot: Option<(i64, i64)>,
    reference: Result<(), String>,
}

fn home(p: &str) -> PathBuf {
    match (p.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(h)) => PathBuf::from(h).join(rest),
        _ => PathBuf::from(p),
    }
}

fn scenes(path: &Path) -> R<Vec<Scene>> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let list = v["scenes"].as_array().ok_or(format!("{}: no \"scenes\" list", path.display()))?;
    list.iter()
        .map(|s| {
            let text = |k: &str| s[k].as_str().map(str::to_string).ok_or(format!("{}: a scene without \"{k}\"", path.display()));
            let frames = |k: &str| s[k].as_array().map(|a| a.iter().filter_map(Value::as_i64).collect::<Vec<_>>());
            let shot = frames("shot").filter(|f| f.len() == 2).map(|f| (f[0], f[1]));
            let reference = if s["reference"] == json!(false) { Err(s["why"].as_str().unwrap_or("not yet").to_string()) } else { Ok(()) };
            Ok(Scene { name: text("name")?, dir: home(&text("dir")?), file: text("file")?, composition: text("composition")?, stills: frames("stills").unwrap_or_default(), shot, reference })
        })
        .collect()
}

/// Frames `at` of a scene drawn on `path` into `out` (frame-00000.png onward, in `at`'s order), by this raea; the
/// seconds it took.
fn render(scene: &Scene, path: &str, spp: u32, at: &[i64], out: &Path) -> R<f64> {
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let at: Vec<String> = at.iter().map(i64::to_string).collect();
    let t = Instant::now();
    let run = std::process::Command::new(exe)
        .current_dir(&scene.dir)
        .env("RAEA_LIGHT", path)
        .env("RAEA_LIGHT_SPP", spp.to_string())
        .args(["render", &scene.file, "--composition", &scene.composition, "--at", &at.join(","), "-o"])
        .arg(format!("{}/", out.display()))
        .output()
        .map_err(|e| e.to_string())?;
    if !run.status.success() {
        let err = String::from_utf8_lossy(&run.stderr);
        let last: Vec<&str> = err.lines().rev().take(4).collect();
        return Err(format!("{} on {path}: render failed: {}", scene.name, last.into_iter().rev().collect::<Vec<_>>().join(" | ")));
    }
    Ok(t.elapsed().as_secs_f64())
}

fn frames(dir: &Path, n: usize) -> R<Vec<metric::Luma>> {
    (0..n).map(|k| metric::load(&dir.join(format!("frame-{k:05}.png")))).collect()
}

fn round(x: f32, k: f32) -> f64 {
    ((x * k).round() / k) as f64
}

/// One scene: its reference (made once, kept unless --fresh), then each path's stills against it and its shot.
fn check(scene: &Scene, paths: &[String], spp: u32, out: &Path, fresh: bool) -> R<Value> {
    let root = out.join(&scene.name);
    let ref_dir = root.join("reference");
    let mut report = json!({ "name": scene.name, "stills": scene.stills, "paths": {} });
    let reference = match &scene.reference {
        Err(why) => {
            report["reference"] = json!({ "made": false, "why": why });
            None
        }
        Ok(()) => {
            if fresh || !ref_dir.join("frame-00000.png").is_file() {
                let s = render(scene, "pathtracer", spp, &scene.stills, &ref_dir)?;
                report["reference"] = json!({ "made": true, "spp": spp, "seconds": round(s as f32, 10.0) });
            } else {
                report["reference"] = json!({ "made": true, "kept": true });
            }
            Some(frames(&ref_dir, scene.stills.len())?)
        }
    };
    for path in paths {
        let dir = root.join(path);
        let secs = render(scene, path, 1, &scene.stills, &dir)?;
        let mut r = json!({ "seconds_a_still": round(secs as f32 / scene.stills.len().max(1) as f32, 100.0) });
        if let Some(refs) = &reference {
            let got = frames(&dir, scene.stills.len())?;
            let d: Vec<metric::Distance> = got.iter().zip(refs).map(|(a, b)| metric::distance(a, b)).collect::<R<_>>()?;
            let n = d.len().max(1) as f32;
            r["mean_stops"] = json!(round(d.iter().map(|x| x.mean).sum::<f32>() / n, 1000.0));
            r["bias_stops"] = json!(round(d.iter().map(|x| x.bias).sum::<f32>() / n, 1000.0));
            r["worst_tile_stops"] = json!(round(d.iter().map(|x| x.worst).fold(0.0, f32::max), 1000.0));
            r["per_still"] = json!(d.iter().map(|x| json!({ "mean": round(x.mean, 1000.0), "bias": round(x.bias, 1000.0), "worst": round(x.worst, 1000.0), "worst_tile": [x.worst_at.0, x.worst_at.1] })).collect::<Vec<_>>());
        }
        if let Some((a, b)) = scene.shot {
            let at: Vec<i64> = (a..=b).collect();
            let shot = root.join(format!("{path}-shot"));
            let secs = render(scene, path, 1, &at, &shot)?;
            r["flicker"] = json!(metric::flicker(&frames(&shot, at.len())?).map(|f| round(f, 1000.0)));
            r["seconds_a_shot_frame"] = json!(round(secs as f32 / at.len() as f32, 100.0));
        }
        // the live frame time is measured in play, not here: LT1 leaves it to `raea play`'s own numbers
        r["live_ms"] = Value::Null;
        report["paths"][path] = r;
    }
    Ok(report)
}

/// The table a person reads: a line a scene and path.
fn table(reports: &[Value]) -> String {
    let mut s = format!("{:<18} {:<8} {:>7} {:>7} {:>7} {:>8} {:>8}\n", "scene", "path", "mean", "bias", "worst", "flicker", "s/still");
    for r in reports {
        let name = r["name"].as_str().unwrap_or("");
        let Some(paths) = r["paths"].as_object() else { continue };
        for (p, v) in paths {
            let f = |k: &str| v[k].as_f64().map_or("-".to_string(), |x| format!("{x:.3}"));
            s += &format!("{name:<18} {p:<8} {:>7} {:>7} {:>7} {:>8} {:>8}\n", f("mean_stops"), f("bias_stops"), f("worst_tile_stops"), f("flicker"), f("seconds_a_still"));
        }
        if r["reference"]["made"] == json!(false) {
            s += &format!("{name:<18} (no reference: {})\n", r["reference"]["why"].as_str().unwrap_or(""));
        }
    }
    s
}

pub fn light(args: &[String]) -> R<String> {
    if args.first().map(String::as_str) != Some("check") || args.len() < 2 {
        return Err(format!("usage: {USAGE}"));
    }
    let list = scenes(Path::new(&args[1]))?;
    let out = PathBuf::from(flag(args, "-o").ok_or(format!("usage: {USAGE}"))?);
    let only = flag(args, "--scene");
    let paths: Vec<String> = flag(args, "--paths").map_or_else(|| PATHS.iter().map(|p| p.to_string()).collect(), |p| p.split(',').map(str::to_string).collect());
    if let Some(p) = paths.iter().find(|p| !PATHS.contains(&p.as_str())) {
        return Err(format!("--paths: {p} isn't a light path to check (raster, solari)"));
    }
    let spp: u32 = flag(args, "--spp").map(|v| v.parse().map_err(|_| format!("--spp {v}: a count of samples"))).transpose()?.unwrap_or(256);
    let fresh = args.iter().any(|a| a == "--fresh");
    let mut reports = Vec::new();
    for s in list.iter().filter(|s| only.as_deref().is_none_or(|o| o == s.name)) {
        eprintln!("raea light: {} ...", s.name);
        reports.push(check(s, &paths, spp, &out, fresh)?);
    }
    if reports.is_empty() {
        return Err(format!("no scene named {}", only.unwrap_or_default()));
    }
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let report = json!({ "scenes": reports });
    std::fs::write(out.join("report.json"), serde_json::to_string_pretty(&report).unwrap_or_default()).map_err(|e| e.to_string())?;
    Ok(format!("{}{}", table(&reports), format_args!("raea light: wrote {}", out.join("report.json").display())))
}
