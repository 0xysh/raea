//! Whether a job is done (docs/make.feature MK1), from what's on disk and nothing the agent says: the deliverables, the
//! notes, the record of renders and checks, and the judge's rounds. Pure over a `Job` so it is tested on fixtures.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use raea_core_brief::make::{self, Make};
use raea_core_facts::{Level, record::Entry};
use serde_json::{Value, json};

/// The heading the notes end with, so the next job (and the skill) learns from this one.
pub const SKILL_HEADING: &str = "What the skill got wrong";

/// Everything status reads, gathered by `lib.rs` (or a test).
pub struct Job<'a> {
    pub make: &'a Make,
    /// the brief's folder: deliverables, NOTES.md and review/ are relative to it
    pub dir: &'a Path,
    /// the project's folder (record paths are relative to it), when it is a raea project
    pub project: Option<&'a Path>,
    pub record: &'a [Entry],
    /// review/judge.log.jsonl's rounds, oldest first
    pub rounds: &'a [Value],
    /// the latest round's judge.json, when there is one
    pub verdict: Option<&'a Value>,
    /// when the judge last wrote its log
    pub judged_at: Option<SystemTime>,
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

fn is_picture(p: &Path) -> bool {
    matches!(p.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref(), Some("png" | "jpg" | "jpeg" | "exr"))
}

/// One deliverable: a file, `a|b` (either), or `dir/` (three or more pictures in it). Returns the files that answer it.
fn answered(dir: &Path, want: &str) -> Result<Vec<PathBuf>, String> {
    if let Some(folder) = want.strip_suffix('/') {
        let pics: Vec<PathBuf> = std::fs::read_dir(dir.join(folder)).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| is_picture(p)).collect();
        return if pics.len() >= 3 { Ok(pics) } else { Err(format!("{want} holds {} picture{}; three or more", pics.len(), if pics.len() == 1 { "" } else { "s" })) };
    }
    let found: Vec<PathBuf> = want.split('|').map(|w| dir.join(w)).filter(|p| p.is_file()).collect();
    if found.is_empty() { Err(want.replace('|', " or ")) } else { Ok(found) }
}

fn notes_text(dir: &Path) -> Option<String> {
    std::fs::read_to_string(dir.join("NOTES.md")).ok()
}

/// AnyKit's stall: three rounds in a row without beating the best mean before them. Not the judge's own `stall`
/// (the same defect twice: change the approach), which is advice to the agent, not a reason to let it stop.
pub fn stalled(rounds: &[Value]) -> bool {
    let means: Vec<f64> = rounds.iter().filter_map(|r| r["mean"].as_f64()).collect();
    means.len() >= 4 && {
        let (before, last) = means.split_at(means.len() - 3);
        last.iter().cloned().fold(f64::MIN, f64::max) <= before.iter().cloned().fold(f64::MIN, f64::max)
    }
}

fn gate(name: &str, of: Option<&str>, clear: bool, says: String) -> Value {
    let mut g = json!({"gate": name, "clear": clear, "says": says});
    if let Some(of) = of {
        g["of"] = json!(of);
    }
    g
}

/// Tier 1: each composition the plan names has a render or check newer than it that found no error.
fn checks(job: &Job) -> Vec<Value> {
    job.make
        .compositions
        .iter()
        .map(|c| {
            let file = job.dir.join(c);
            let rel = job.project.map(|p| raea_core_facts::record::rel(p, &file)).unwrap_or_else(|| c.clone());
            let Some(last) = job.record.iter().rev().find(|e| e.of == rel) else {
                return gate("checks", Some(c), false, format!("never rendered or checked: raea render {c} (or raea scene lint {c})"));
            };
            let changed = mtime(&file).and_then(|m| m.duration_since(SystemTime::UNIX_EPOCH).ok()).map(|d| d.as_secs());
            if changed.is_some_and(|t| t > last.at) {
                return gate("checks", Some(c), false, format!("changed since its last {} ({}): run it again", last.by, last.id));
            }
            let errors: Vec<&str> = last.findings.iter().filter(|f| f.level == Level::Error).map(|f| f.says.as_str()).collect();
            match errors.as_slice() {
                [] => gate("checks", Some(c), true, format!("clean at {} ({})", last.id, last.by)),
                [one] => gate("checks", Some(c), false, format!("{} found: {one}", last.id)),
                many => gate("checks", Some(c), false, format!("{} found {} errors, first: {}", last.id, many.len(), many[0])),
            }
        })
        .collect()
}

/// Tier 3: a fresh critic has judged a round newer than the newest deliverable, and each blocking defect it named is
/// fixed (the round passed) or answered in NOTES.md by its name. Its score gates once the judge is calibrated.
fn critic(job: &Job, newest: Option<SystemTime>, notes: &str) -> Vec<Value> {
    let Some(last) = job.rounds.last() else {
        return vec![gate("critic", None, false, "no round yet: raea render EDIT.tsx --review review/r1, then raea judge review/r1 --by claude".into())];
    };
    let mut out = Vec::new();
    let fresh = match (job.judged_at, newest) {
        (Some(j), Some(n)) => j >= n,
        (Some(_), None) => true,
        _ => false,
    };
    out.push(gate("critic", None, fresh, if fresh { format!("round {} judged after the newest deliverable", job.rounds.len()) } else { "the newest deliverable came after the last round: review and judge it again".into() }));
    let blocking: Vec<&str> = raea_app_judge::rubric::blocking(last["defects"].as_array().map(Vec::as_slice).unwrap_or_default());
    let open: Vec<&str> = if last["pass"] == true { Vec::new() } else { blocking.into_iter().filter(|d| !notes.contains(d)).collect() };
    out.push(gate("defects", None, open.is_empty(), if open.is_empty() { "no blocking defect left unanswered".into() } else { format!("blocking, neither fixed nor answered in NOTES.md: {}", open.join(", ")) }));
    if job.verdict.is_some_and(|v| v["gating"] == true) {
        let p = raea_app_judge::progress(job.rounds);
        let done = p["done"] == true;
        out.push(gate("score", None, done, format!("the calibrated judge: {} passes in a row, two needed", p["passesInARow"])));
    }
    out
}

/// The job's status: what's missing, each gate, the judge's rounds, and whether the agent may stop.
pub fn status(job: &Job) -> Value {
    let mut missing = Vec::new();
    let mut newest: Option<SystemTime> = None;
    for want in make::deliverables(job.make) {
        match answered(job.dir, &want) {
            Ok(files) => newest = files.iter().filter_map(|f| mtime(f)).chain(newest).max(),
            Err(why) => missing.push(why),
        }
    }
    let notes = notes_text(job.dir).unwrap_or_default();
    if !notes.to_lowercase().contains(&SKILL_HEADING.to_lowercase()) {
        missing.push(format!("NOTES.md with a \"{SKILL_HEADING}\" section (one line per thing that cost time; what the guide got wrong for this job)"));
    }
    let mut gates = checks(job);
    gates.extend(critic(job, newest, &notes));
    let cap = job.make.rounds.unwrap_or(make::ROUNDS) as usize;
    let capped = job.rounds.len() >= cap;
    let stalled = stalled(job.rounds);
    let clear = missing.is_empty() && gates.iter().all(|g| g["clear"] == true);
    let mut why = Vec::new();
    if capped && !clear {
        why.push(format!("capped: {} critic rounds, the most this job gets; delivered with what's still missing", job.rounds.len()));
    }
    if stalled && !clear {
        why.push("stalled: three rounds without beating the best before them; delivered with what's still missing".into());
    }
    let advice = raea_app_judge::progress(job.rounds);
    json!({
        "kind": job.make.kind,
        "done": clear || capped || stalled,
        "short": !clear && (capped || stalled),
        "missing": missing,
        "gates": gates,
        "judge": {"rounds": job.rounds.len(), "cap": cap, "passesInARow": advice["passesInARow"], "changeApproach": advice["stall"], "again": advice["again"]},
        "plan": make::doctor(job.make),
        "why": why,
    })
}

/// The status for a person or an agent to read.
pub fn text(s: &Value) -> String {
    let mut t = vec![format!("make: {} {}", s["kind"].as_str().unwrap_or("?"), if s["done"] == true { if s["short"] == true { "delivered short" } else { "done" } } else { "not done" })];
    for w in s["why"].as_array().into_iter().flatten() {
        t.push(format!("  {}", w.as_str().unwrap_or("")));
    }
    let list = |k: &str| s[k].as_array().cloned().unwrap_or_default();
    if !list("plan").is_empty() {
        t.push("plan:".into());
        t.extend(list("plan").iter().map(|p| format!("  {}", p.as_str().unwrap_or(""))));
    }
    if !list("missing").is_empty() {
        t.push("missing:".into());
        t.extend(list("missing").iter().map(|m| format!("  {}", m.as_str().unwrap_or(""))));
    }
    t.push("gates:".into());
    for g in list("gates") {
        let of = g["of"].as_str().map(|o| format!(" {o}")).unwrap_or_default();
        t.push(format!("  {} {}{of}: {}", if g["clear"] == true { "ok  " } else { "open" }, g["gate"].as_str().unwrap_or(""), g["says"].as_str().unwrap_or("")));
    }
    let j = &s["judge"];
    t.push(format!("judge: {} of {} rounds, {} passes in a row{}", j["rounds"], j["cap"], j["passesInARow"], if j["changeApproach"] == true { "; the same defect keeps coming back: change the approach for that part, not the dose" } else { "" }));
    t.join("\n")
}
