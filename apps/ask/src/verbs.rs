//! `raea ask` (docs/ask.feature AK3): questions about a piece's picture and sound, from the command line or a batch,
//! prepared (prepare.rs), asked in parallel (runner.rs) and kept (record.rs). It prints as answers come: words and a
//! probability a line, or with `--json` one JSON object a line (an answer, a call `--dry` would make, then the summary).
//! `--gate` exits 1 when a calibrated question flags (`--gate any`: when any does), for a pipeline.

use std::collections::{BTreeMap, HashMap};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use raea_core_ask::{Kind, Question, Window, library};
use serde_json::{Value, json};

use crate::prepare::{self, Piece};
use crate::runner::{self, Entry, Event, Opts};

pub const USAGE: &str = "raea ask FILE [QUESTION...] [--choose A,B,C] [--grade] [--rate N] [--flag ANSWERS] [--set NAME] [--std ID] [--from S] [--to S] [--at S] [--with FILE2] [--brief brief.ts] [--words words.json] [--voice yes|no] [--state TEXT] [--dry] [--json] [--gate [any]] [--jobs N] | ask --batch FILE [--dry] [--json] [--gate [any]] [--jobs N] | ask list [SET] [--json]   (questions about a piece's picture and sound, answered by clef-omni and paid from your raea plan's pool when the app is signed in, else RAEA_AGENT_KEY's, else your own Cloudflare keys: your own words, or the standard sets (rubric, av, defects, platform, decide); every question about one clip goes in one call, --batch (a JSON list or JSON lines, - for stdin) asks many clips in parallel; --dry says what would be sent and spends nothing; answers are advice until a question is calibrated)";

const VALUED: [&str; 15] = ["--choose", "--rate", "--flag", "--set", "--std", "--from", "--to", "--at", "--with", "--brief", "--words", "--voice", "--state", "--jobs", "--batch"];
const BARE: [&str; 4] = ["--grade", "--dry", "--json", "--gate"];

/// The arguments: the words that aren't flags, each flag's values, and whether `--gate` is `--gate any`.
#[derive(Default)]
struct Args {
    words: Vec<String>,
    flags: BTreeMap<String, Vec<String>>,
    gate_any: bool,
}

impl Args {
    fn parse(args: &[String]) -> Result<Args, String> {
        let mut a = Args::default();
        let mut i = 0;
        while i < args.len() {
            let s = args[i].as_str();
            if VALUED.contains(&s) {
                let v = args.get(i + 1).ok_or_else(|| format!("ask: {s} takes a value\nusage: {USAGE}"))?;
                a.flags.entry(s.into()).or_default().push(v.clone());
                i += 1;
            } else if BARE.contains(&s) {
                a.flags.entry(s.into()).or_default();
                if s == "--gate" && args.get(i + 1).is_some_and(|v| v == "any") {
                    a.gate_any = true;
                    i += 1;
                }
            } else if s.starts_with("--") {
                return Err(format!("ask doesn't take {s}\nusage: {USAGE}"));
            } else {
                a.words.push(s.into());
            }
            i += 1;
        }
        Ok(a)
    }
    fn has(&self, f: &str) -> bool {
        self.flags.contains_key(f)
    }
    fn one(&self, f: &str) -> Option<&str> {
        self.flags.get(f).and_then(|v| v.last()).map(String::as_str)
    }
    fn num(&self, f: &str) -> Result<Option<f64>, String> {
        self.one(f).map(|v| v.parse::<f64>().map_err(|_| format!("ask: {f} takes seconds, got {v:?}"))).transpose()
    }
    fn list(&self, f: &str) -> Vec<String> {
        self.flags.get(f).into_iter().flatten().flat_map(|v| v.split(',')).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    }
}

/// The craft rubric's text, where the guides are installed, for the `rubric` set.
pub fn rubric_md() -> Option<String> {
    std::fs::read_to_string(raea_io_home::home().join("sdk/guides/craft/rubric.md")).ok()
}

/// A question's id from its words: `own-` and its first words, so the same words keep their id.
pub fn own_id(words: &str) -> String {
    let slug: Vec<String> = words.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).take(6).map(str::to_lowercase).collect();
    let mut id = format!("own-{}", slug.join("-"));
    id.truncate(raea_core_ask::limits::ID_LEN);
    id
}

/// The questions an entry names: its own words (shaped by the kind flags), its sets and its standard ids.
fn named(own: &[Question], sets: &[String], std: &[String], rubric: Option<&str>) -> Result<Vec<Question>, String> {
    let mut qs = own.to_vec();
    for s in sets {
        qs.extend(library::set(s, rubric)?);
    }
    for id in std {
        qs.push(library::find(id, rubric).ok_or_else(|| format!("ask: no standard question {id} (raea ask list)"))?);
    }
    Ok(qs)
}

fn shaped(words: &str, a: &Args) -> Result<Question, String> {
    let mut q = Question::plain(&own_id(words), words);
    if let Some(c) = a.one("--choose") {
        q.kind = Kind::Choose;
        q.options = c.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    } else if let Some(n) = a.one("--rate") {
        let n: usize = n.parse().map_err(|_| format!("ask: --rate takes how many levels, got {n:?}"))?;
        q.kind = Kind::Rate;
        q.options = (1..=n).map(|i| i.to_string()).collect();
    } else if a.has("--grade") {
        q.kind = Kind::Grade;
    }
    q.flags = a.list("--flag");
    if let Some(why) = raea_core_ask::limits::check(&q) {
        return Err(format!("ask: {why}"));
    }
    Ok(q)
}

/// The window from --from, --to and --at (three seconds around it).
fn window(a: &Args) -> Result<Option<Window>, String> {
    if let Some(at) = a.num("--at")? {
        return Ok(Some(Window { from: Some((at - 1.5).max(0.0)), to: Some(at + 1.5) }));
    }
    let (from, to) = (a.num("--from")?, a.num("--to")?);
    Ok((from.is_some() || to.is_some()).then_some(Window { from, to }))
}

fn voice(v: Option<&str>) -> Result<Option<bool>, String> {
    match v {
        None => Ok(None),
        Some("yes" | "true") => Ok(Some(true)),
        Some("no" | "false") => Ok(Some(false)),
        Some(v) => Err(format!("ask: --voice takes yes or no, got {v:?}")),
    }
}

/// Pieces read once a run, by what they were read with.
type Pieces = HashMap<String, Arc<Piece>>;

#[allow(clippy::too_many_arguments)]
fn piece(pieces: &mut Pieces, file: &str, brief: Option<&str>, words: Option<&str>, voice: Option<bool>, state: Option<String>) -> Result<Arc<Piece>, String> {
    let key = format!("{file}\u{1f}{brief:?}\u{1f}{words:?}\u{1f}{voice:?}\u{1f}{state:?}");
    if let Some(p) = pieces.get(&key) {
        return Ok(p.clone());
    }
    let p = Arc::new(Piece::read(Path::new(file), brief.map(Path::new), words.map(Path::new), voice, state).map_err(|e| format!("ask: {e}"))?);
    pieces.insert(key, p.clone());
    Ok(p)
}

fn entry(p: Arc<Piece>, window: Option<Window>, qs: Vec<Question>, with: Vec<PathBuf>, rubric: Option<&str>) -> Entry {
    let (questions, dropped) = prepare::keep(qs, &p.facts(), rubric);
    Entry { piece: p, window, questions, dropped, with }
}

/// The command line's one entry.
fn from_args(a: &Args, rubric: Option<&str>, pieces: &mut Pieces) -> Result<Vec<Entry>, String> {
    let Some((file, words)) = a.words.split_first() else { return Err(USAGE.into()) };
    let own = words.iter().map(|w| shaped(w, a)).collect::<Result<Vec<_>, _>>()?;
    let qs = named(&own, &a.list("--set"), &a.list("--std"), rubric)?;
    if qs.is_empty() {
        return Err(format!("ask: nothing to ask: give a question in words, --set or --std (raea ask list)\nusage: {USAGE}"));
    }
    let p = piece(pieces, file, a.one("--brief"), a.one("--words"), voice(a.one("--voice"))?, a.one("--state").map(str::to_string))?;
    let with = a.flags.get("--with").into_iter().flatten().map(PathBuf::from).collect();
    Ok(vec![entry(p, window(a)?, qs, with, rubric)])
}

/// A batch: a JSON list of entries or JSON lines, from a file or stdin (`-`). Each entry is {file, window?, questions?
/// (words or questions), set?, std?, with?, brief?, words?, voice?, state?}.
fn from_batch(path: &str, rubric: Option<&str>, pieces: &mut Pieces) -> Result<Vec<Entry>, String> {
    let text = match path {
        "-" => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).map_err(|e| format!("ask: stdin: {e}"))?;
            s
        }
        p => std::fs::read_to_string(p).map_err(|e| format!("ask: {p}: {e}"))?,
    };
    let rows: Vec<Value> = match serde_json::from_str::<Value>(text.trim()) {
        Ok(Value::Array(a)) => a,
        Ok(v @ Value::Object(_)) => vec![v],
        _ => text.lines().filter(|l| !l.trim().is_empty()).enumerate().map(|(i, l)| serde_json::from_str(l).map_err(|e| format!("ask: {path} line {}: {e}", i + 1))).collect::<Result<_, _>>()?,
    };
    let strs = |v: &Value| -> Vec<String> {
        match v {
            Value::String(s) => s.split(',').map(|s| s.trim().to_string()).collect(),
            Value::Array(a) => a.iter().filter_map(Value::as_str).map(str::to_string).collect(),
            _ => vec![],
        }
    };
    let mut out = Vec::new();
    for (n, r) in rows.iter().enumerate() {
        let at = |e: String| format!("ask: {path} entry {}: {e}", n + 1);
        let file = r["file"].as_str().ok_or_else(|| at("it has no file".into()))?;
        let own = r["questions"].as_array().into_iter().flatten().map(|q| match q {
            Value::String(w) => Ok(Question::plain(&own_id(w), w)),
            q => serde_json::from_value::<Question>(q.clone()).map_err(|e| e.to_string()),
        });
        let own = own.collect::<Result<Vec<_>, _>>().map_err(at)?;
        let qs = named(&own, &strs(&r["set"]), &strs(&r["std"]), rubric).map_err(at)?;
        let window: Option<Window> = serde_json::from_value(r["window"].clone()).ok();
        let v = voice(r["voice"].as_str()).map_err(at)?.or(r["voice"].as_bool());
        let p = piece(pieces, file, r["brief"].as_str(), r["words"].as_str(), v, r["state"].as_str().map(str::to_string)).map_err(at)?;
        out.push(entry(p, window, qs, strs(&r["with"]).into_iter().map(PathBuf::from).collect(), rubric));
    }
    Ok(out)
}

/// `raea ask list [SET]`: the standard questions.
fn list(a: &Args, rubric: Option<&str>) -> Result<String, String> {
    let sets: Vec<String> = match a.words.get(1) {
        Some(s) => vec![s.clone()],
        None => library::SETS.iter().map(|s| s.to_string()).collect(),
    };
    let mut out = Vec::new();
    for s in &sets {
        let qs = library::set(s, rubric)?;
        if a.has("--json") {
            out.push(json!({"set": s, "questions": qs}).to_string());
            continue;
        }
        out.push(format!("{s}:"));
        for q in qs {
            let how = if q.calibrated() { "calibrated" } else { "advice" };
            out.push(format!("  {:<28} {:<6} {:<10} {}", q.id, format!("{:?}", q.kind).to_lowercase(), how, q.ask));
        }
    }
    Ok(out.join("\n"))
}

fn span(from: f64, to: Option<f64>) -> String {
    match to {
        Some(t) => format!("{from:.0}-{t:.0} s"),
        None if from > 0.0 => format!("from {from:.0} s"),
        None => "whole".into(),
    }
}

/// An event as a line of text.
fn line(e: &Event, files: usize) -> String {
    match e {
        Event::Would(v) => {
            let qs: Vec<&str> = v["questions"].as_array().into_iter().flatten().filter_map(|q| q["id"].as_str()).collect();
            let m = &v["media"];
            let kb = |k: &str| m[k].as_array().map(|a| a.iter().filter_map(Value::as_u64).map(|b| format!("{} KB", b / 1024)).collect::<Vec<_>>().join(", ")).unwrap_or_default();
            format!(
                "would ask {} ({}) about {}:\n  {}\n  videos [{}], stills [{}], sounds [{}], about ${:.4}\n  state: {}",
                qs.len(),
                span(v["from"].as_f64().unwrap_or(0.0), v["to"].as_f64()),
                v["file"].as_str().unwrap_or(""),
                qs.join(", "),
                kb("videos"),
                kb("stills"),
                kb("sounds"),
                v["estimate_usd"].as_f64().unwrap_or(0.0),
                v["state"].as_str().unwrap_or("")
            )
        }
        Event::Answer(a) => {
            let who = if files > 1 { format!("{}: ", a.file) } else { String::new() };
            let what = match (&a.says, &a.not_asked, &a.waits) {
                (Some(s), ..) => format!("{s}  [{}]{}", span(a.from, a.to), if a.flagged && a.calibrated { "  <- finding" } else if a.flagged { "  <- flagged" } else { "" }),
                (_, Some(n), _) => format!("not asked: {n}"),
                (.., Some(w)) => w.clone(),
                _ => "no answer".into(),
            };
            format!("{who}{:<34} {what}", a.id)
        }
    }
}

/// `raea ask ...` with its exit code: 0, or 1 when it couldn't ask or a gate failed.
pub fn cli(args: &[String]) -> i32 {
    match run(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn run(args: &[String]) -> Result<i32, String> {
    let a = Args::parse(args)?;
    let rubric = rubric_md();
    if a.words.first().is_some_and(|w| w == "list") && !Path::new("list").exists() {
        println!("{}", list(&a, rubric.as_deref())?);
        return Ok(0);
    }
    let mut pieces = Pieces::new();
    let entries = match a.one("--batch") {
        Some(b) => from_batch(b, rubric.as_deref(), &mut pieces)?,
        None => from_args(&a, rubric.as_deref(), &mut pieces)?,
    };
    let jobs = a.one("--jobs").map(|j| j.parse::<usize>().map_err(|_| format!("ask: --jobs takes a number, got {j:?}"))).transpose()?.unwrap_or(4);
    let (json_out, dry) = (a.has("--json"), a.has("--dry"));
    let files = pieces.len();
    let sum = runner::run(entries, &Opts::new(jobs, dry), &mut |e| {
        match (&e, json_out) {
            (Event::Answer(r), true) => println!("{}", serde_json::to_string(r).unwrap_or_default()),
            (Event::Would(v), true) => println!("{}", json!({"would": v})),
            (e, false) => println!("{}", line(e, files)),
        }
    });
    if json_out {
        println!("{}", json!({"summary": sum}));
    } else if dry {
        println!("{} call{}, about ${:.4}; nothing was sent", sum.calls, if sum.calls == 1 { "" } else { "s" }, sum.estimate);
    } else if sum.calls > 0 {
        let routes: Vec<&str> = sum.routes.iter().map(String::as_str).collect();
        let left = sum.pool_left.map(|l| format!(", ${l:.2} left this month")).unwrap_or_default();
        let waiting = if sum.waiting > 0 { format!("; {} wait for the pool", sum.waiting) } else { String::new() };
        println!(
            "asked {} question{} in {} call{} through {}: ${:.4}{left}{waiting}. Answers are advice until a question is calibrated (docs/ask.feature AK7).",
            sum.asked,
            if sum.asked == 1 { "" } else { "s" },
            sum.calls,
            if sum.calls == 1 { "" } else { "s" },
            routes.join(" and "),
            sum.cost
        );
    }
    if !dry && sum.calls == 0 {
        return Err("ask: nothing was asked (each question's line says why)".into());
    }
    // a gate fails on a flag (a calibrated one, or with `any` any), and on questions still waiting for the pool
    let flagged = if a.gate_any { sum.flagged } else { sum.flagged_calibrated };
    Ok(i32::from(a.has("--gate") && (flagged > 0 || sum.waiting > 0)))
}
