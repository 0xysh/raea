//! `raea make`: a scene, kit or shot made to its frozen plan (`make` on brief.ts, core/brief make.rs) and held to the
//! review loop (docs/make.feature). `status` decides, in code, what's missing and whether the agent may stop; the Stop
//! hook (apps/check hook.rs) asks it at the end of every turn.
//!
//!   raea make status [DIR] [--json]

pub mod status;

use std::path::{Path, PathBuf};

use raea_core_brief::make::Make;
use serde_json::Value;

pub const USAGE: &str = "raea make status [DIR] [--json]   (a scene, kit or shot held to its plan, `make` on brief.ts: what's missing, each gate, the judge's rounds, and whether it's done, decided in code; the agent's Stop hook holds it there)";

/// The brief.ts the job is read from: DIR's own, else the nearest above it inside its raea project.
pub fn brief_path(dir: &Path) -> Option<PathBuf> {
    let dir = dunce::canonicalize(dir).ok()?;
    let project = raea_core_facts::record::up(&dir, ".raea/project");
    for d in dir.ancestors() {
        let p = d.join("brief.ts");
        if p.is_file() {
            return Some(p);
        }
        if project.as_deref() == Some(d) {
            break;
        }
    }
    None
}

/// Whether a brief.ts has a job at all, read from its words: cheap enough for a hook at every turn's end, so the
/// module is only evaluated when it can matter.
pub fn has_make(brief: &Path) -> bool {
    std::fs::read_to_string(brief).is_ok_and(|t| t.contains("make:") || t.contains("make :"))
}

/// The plan from brief.ts, evaluated by `raea spec FILE --as brief` (the module runs; Rust checks it against core/brief).
pub fn read_make(brief: &Path) -> Result<Option<Make>, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let out = std::process::Command::new(exe).arg("spec").arg(brief).args(["--as", "brief"]).env("RAEA_RECORD", "off").output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let start = text.find('{').ok_or_else(|| format!("{} didn't read as a brief: {}", brief.display(), String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")))?;
    let v: Value = serde_json::from_str(&text[start..]).map_err(|e| format!("{}: {e}", brief.display()))?;
    let b = v.get("brief").cloned().unwrap_or(v);
    match b.get("make") {
        None | Some(Value::Null) => Ok(None),
        Some(m) => serde_json::from_value(m.clone()).map(Some).map_err(|e| format!("{}: make: {e}", brief.display())),
    }
}

/// The judge's rounds beside the brief (review/judge.log.jsonl, as `raea judge` keeps them), the newest round's
/// judge.json, and when the log was last written.
fn judged(dir: &Path) -> (Vec<Value>, Option<Value>, Option<std::time::SystemTime>) {
    let log = dir.join("review").join("judge.log.jsonl");
    let rounds: Vec<Value> = std::fs::read_to_string(&log).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    let verdict = rounds.last().and_then(|r| r["dir"].as_str()).and_then(|d| std::fs::read_to_string(Path::new(d).join("judge.json")).ok()).and_then(|t| serde_json::from_str(&t).ok());
    let at = std::fs::metadata(&log).and_then(|m| m.modified()).ok();
    (rounds, verdict, at)
}

/// The status of the job whose brief is at `brief`, with its plan already read.
pub fn status_of(brief: &Path, make: &Make) -> Value {
    let dir = brief.parent().unwrap_or(Path::new("."));
    let project = raea_core_facts::record::up(dir, ".raea/project");
    let record = project.as_deref().map(raea_core_facts::record::read).unwrap_or_default();
    let (rounds, verdict, judged_at) = judged(dir);
    let job = status::Job { make, dir, project: project.as_deref(), record: &record, rounds: &rounds, verdict: verdict.as_ref(), judged_at };
    let mut s = status::status(&job);
    s["brief"] = Value::String(brief.display().to_string());
    s
}

/// For the hook: the job's status from a folder, None when there's no job there (no brief.ts, or no `make` on it).
pub fn status_in(dir: &Path) -> Result<Option<Value>, String> {
    let Some(brief) = brief_path(dir).filter(|b| has_make(b)) else { return Ok(None) };
    let Some(make) = read_make(&brief)? else { return Ok(None) };
    Ok(Some(status_of(&brief, &make)))
}

pub fn cli(args: &[String]) -> Result<String, String> {
    match args.get(1).map(String::as_str) {
        Some("status") => {}
        _ => return Err(USAGE.into()),
    }
    let dir = args.iter().skip(2).find(|a| !a.starts_with("--")).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let brief = brief_path(&dir).ok_or_else(|| format!("raea make: no brief.ts at or above {} in its project", dir.display()))?;
    let make = read_make(&brief)?.ok_or_else(|| format!("raea make: {} has no `make` (scene(), kit() or shot() from raea/brief)", brief.display()))?;
    let s = status_of(&brief, &make);
    if args.iter().any(|a| a == "--json") {
        return Ok(serde_json::to_string_pretty(&s).unwrap_or_default());
    }
    Ok(status::text(&s))
}

#[cfg(test)]
mod tests;
