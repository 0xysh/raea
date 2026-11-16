//! `raea motion "LINE" [--keys KEYS.json] ...`: body motion made on this Mac by Kimodo (raea-sys-kimodo, on MLX): a
//! SOMA clip from a line of words and, with --keys, Kimodo constraints (full-body keyframes, hand or foot targets, a
//! root path: what sdk/body/tools/between.ts writes). The clip is the file sdk/body/tools/fromkimodo.ts reads, the
//! same one the cloud runner wrote (docs/character.feature CH15b, CH15c).

use std::path::PathBuf;

use raea_runtime_bundle::flag;

type R<T> = Result<T, String>;

pub const USAGE: &str = "raea motion \"LINE\" [--keys KEYS.json] [--seconds 5 | --frames N] [--seed 1] [--steps 100] [-o CLIP.json]";
pub const ABOUT: &str = "body motion made on this Mac by Kimodo (NVIDIA, on MLX): a SOMA clip from a line of words and, with --keys, Kimodo constraints (full-body keyframes, hand or foot targets, a root path, as sdk/body/tools/between.ts writes them), as the clip sdk/body/tools/fromkimodo.ts reads; the first use fetches the motion model (1.1 GB) into raea's models folder, and the first new line its text encoder (8.8 GB: Llama 3 8B in 8 bits with LLM2Vec's adapters); a line's words are read once and kept. Licences: Kimodo's weights NVIDIA's Open Model License, Meta Llama 3 the Meta Llama 3 Community License, LLM2Vec's adapters MIT";

fn number(args: &[String], name: &str, default: f64) -> R<f64> {
    match flag(args, name) {
        None => Ok(default),
        Some(v) => v.parse::<f64>().ok().filter(|n| n.is_finite() && *n >= 0.0).ok_or(format!("{name} takes a number, got `{v}`")),
    }
}

fn line(args: &[String]) -> String {
    const WITH_VALUE: [&str; 6] = ["--keys", "--seconds", "--frames", "--seed", "--steps", "-o"];
    let mut out = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if WITH_VALUE.contains(&args[i].as_str()) {
            i += 2;
            continue;
        }
        if !args[i].starts_with("--") {
            out.push(args[i].as_str());
        }
        i += 1;
    }
    out.join(" ")
}

/// what a call asks for, read from its words and flags
struct Ask {
    words: String,
    keys: Option<String>,
    constraints: Option<serde_json::Value>,
    frames: usize,
    seed: u64,
    steps: usize,
    out: Option<String>,
}

pub fn motion(args: &[String]) -> R<String> {
    let words = line(args);
    let keys = flag(args, "--keys");
    if words.trim().is_empty() && keys.is_none() {
        return Err(format!("{USAGE}\n  ({ABOUT})"));
    }
    let frames = match flag(args, "--frames") {
        Some(_) => number(args, "--frames", 150.0)? as usize,
        None => (number(args, "--seconds", 5.0)? * 30.0).round() as usize,
    };
    if !(2..=600).contains(&frames) {
        return Err(format!("a clip is 2 to 600 frames (20 s at 30 a second), asked {frames}"));
    }
    let constraints = match &keys {
        Some(p) => Some(serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?).map_err(|e| format!("{p}: {e}"))?),
        None => None,
    };
    let (seed, steps) = (number(args, "--seed", 1.0)? as u64, (number(args, "--steps", 100.0)? as usize).max(2));
    make(Ask { words, keys, constraints, frames, seed, steps, out: flag(args, "-o") })
}

#[cfg(not(target_os = "macos"))]
fn make(ask: Ask) -> R<String> {
    Err(format!("raea motion runs Kimodo on MLX, on Apple silicon's GPU: {:?} can be made on a Mac", ask.words))
}

#[cfg(target_os = "macos")]
fn make(ask: Ask) -> R<String> {
    // float32 as checked against PyTorch: MLX's TF32 matmuls off, before the first one
    if std::env::var_os("MLX_ENABLE_TF32").is_none() {
        unsafe { std::env::set_var("MLX_ENABLE_TF32", "0") };
    }
    let t0 = std::time::Instant::now();
    // a line every 512 MB of each file fetched
    let mut shown = std::collections::HashMap::<String, u64>::new();
    let mut progress = |file: &str, n: u64| {
        if shown.insert(file.to_string(), n >> 29) != Some(n >> 29) {
            eprintln!("raea motion: fetching {file}: {} MB", n >> 20);
        }
    };
    let mut k = raea_sys_kimodo::Kimodo::fetch(&mut progress).map_err(|e| format!("raea motion: {e}"))?;
    let (clean, emb) = k.encode(&ask.words).map_err(|e| format!("raea motion: {e}"))?;
    let load_s = t0.elapsed().as_secs_f64();
    let made_from = raea_sys_kimodo::Ask { line: &ask.words, frames: ask.frames, seed: ask.seed, steps: ask.steps, constraints: ask.constraints.as_ref(), cfg: (2.0, 2.0) };
    let t1 = std::time::Instant::now();
    let made = k.make(&made_from, emb.as_deref(), None).map_err(|e| format!("raea motion: {e}"))?;
    let gen_s = t1.elapsed().as_secs_f64();
    let clip = raea_sys_kimodo::decode::clip_json(&made.motion, &k.skeleton, ask.seed, ask.steps, &clean, ask.constraints.is_some());
    // without -o, the line's words name the file
    let out = PathBuf::from(ask.out.unwrap_or_else(|| {
        let slug = clean.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect::<Vec<_>>().join("-");
        format!("{}.clip.json", if slug.is_empty() { "keys".to_string() } else { slug.chars().take(60).collect() })
    }));
    std::fs::write(&out, serde_json::to_string(&clip).unwrap_or_default()).map_err(|e| format!("{}: {e}", out.display()))?;
    let r = &made.motion.root;
    let (a, b) = (r[0], r[r.len() - 1]);
    let travel = ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
    Ok(format!(
        "APPLIED  motion {:?}{}, {} frames ({:.1} s), seed {}, {} steps -> {} (made on this Mac in {gen_s:.1} s after {load_s:.1} s loading and reading the line; the hips end {travel:.2} m from where they start)",
        clean,
        if ask.keys.is_some() { " with keys" } else { "" },
        ask.frames,
        ask.frames as f64 / 30.0,
        ask.seed,
        ask.steps,
        out.display()
    ))
}
