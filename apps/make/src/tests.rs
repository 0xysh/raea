use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use raea_core_brief::{make::Make, read};
use raea_core_facts::record::Entry;
use serde_json::{Value, json};

use crate::status::{Job, status, stalled};

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("raea-make-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn put(d: &PathBuf, rel: &str, text: &str) {
    let p = d.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn kit() -> Make {
    read("make", r#"{"kind": "kit", "style": "1970s desk, maker Aurora", "objects": [{"id": "phone", "label": "Phone", "h": 0.12, "prompt": "a rotary phone"}]}"#).unwrap()
}

fn delivered(d: &PathBuf) {
    for f in ["out/kit/phone/phone.glb", "out/kit/phone/phone_lod1.glb", "out/kit/thumbs/phone.png", "out/kit/lineup.png", "out/kit/kit.json"] {
        put(d, f, "x");
    }
    put(d, "NOTES.md", "# Notes\n\n## What the skill got wrong\n- nothing yet\n");
}

fn round(pass: bool, mean: f64, defects: Value) -> Value {
    json!({"dir": "review/r1", "pass": pass, "mean": mean, "defects": defects})
}

fn run(make: &Make, d: &PathBuf, record: &[Entry], rounds: &[Value], judged_at: Option<SystemTime>) -> Value {
    status(&Job { make, dir: d, project: None, record, rounds, verdict: None, judged_at })
}

fn later() -> Option<SystemTime> {
    Some(SystemTime::now() + Duration::from_secs(5))
}

#[test]
fn nothing_made_is_not_done_and_says_what_is_missing() {
    let d = dir("empty");
    let s = run(&kit(), &d, &[], &[], None);
    assert_eq!(s["done"], false);
    let missing = s["missing"].to_string();
    assert!(missing.contains("out/kit/phone/phone_lod1.glb") && missing.contains("NOTES.md"), "{missing}");
    assert!(s["gates"].as_array().unwrap().iter().any(|g| g["gate"] == "critic" && g["clear"] == false));
}

#[test]
fn delivered_and_judged_after_is_done() {
    let d = dir("done");
    delivered(&d);
    let s = run(&kit(), &d, &[], &[round(true, 7.5, json!([]))], later());
    assert_eq!(s["done"], true, "{s:#}");
    assert_eq!(s["short"], false);
}

#[test]
fn a_round_older_than_the_newest_deliverable_is_stale() {
    let d = dir("stale");
    delivered(&d);
    let s = run(&kit(), &d, &[], &[round(true, 7.5, json!([]))], Some(SystemTime::UNIX_EPOCH));
    assert_eq!(s["done"], false);
    assert!(s["gates"].to_string().contains("came after the last round"), "{s:#}");
}

#[test]
fn a_blocking_defect_must_be_fixed_or_answered_in_the_notes() {
    let d = dir("defect");
    delivered(&d);
    let defects = json!([{"defect": "floating", "present": true, "level": "blocking"}]);
    let s = run(&kit(), &d, &[], &[round(false, 6.0, defects.clone())], later());
    assert_eq!(s["done"], false);
    assert!(s["gates"].to_string().contains("floating"));
    put(&d, "NOTES.md", "## What the skill got wrong\n- floating: the handset rests in its cradle; the judge read its shadow as a gap\n");
    let s = run(&kit(), &d, &[], &[round(false, 6.0, defects)], later());
    assert_eq!(s["done"], true, "{s:#}");
}

#[test]
fn at_the_cap_it_is_delivered_short() {
    let d = dir("cap");
    let rounds: Vec<Value> = (0..8).map(|i| round(false, 5.0 + i as f64 * 0.5, json!([]))).collect();
    let s = run(&kit(), &d, &[], &rounds, later());
    assert_eq!((s["done"].clone(), s["short"].clone()), (json!(true), json!(true)), "{s:#}");
    assert!(s["why"].to_string().contains("capped"));
}

#[test]
fn three_rounds_without_beating_the_best_is_a_stall() {
    let r = |m: f64| json!({"mean": m});
    assert!(stalled(&[r(5.0), r(6.5), r(6.0), r(6.4), r(6.5)]));
    assert!(!stalled(&[r(5.0), r(6.0), r(6.2), r(6.4), r(6.6)]));
    assert!(!stalled(&[r(5.0), r(5.0), r(5.0)]), "too few rounds to call it");
}

#[test]
fn a_composition_needs_a_clean_check_newer_than_it() {
    let d = dir("checks");
    delivered(&d);
    put(&d, "scene.tsx", "x");
    let mut make = kit();
    make.compositions = vec!["scene.tsx".into()];
    let rounds = [round(true, 7.5, json!([]))];
    let s = run(&make, &d, &[], &rounds, later());
    assert!(s["gates"].to_string().contains("never rendered"), "{s:#}");
    let entry = |at: u64, level: &str| -> Entry {
        serde_json::from_value(json!({"id": "20261123-x", "at": at, "by": "raea render", "of": "scene.tsx",
            "findings": [{"check": "float", "level": level, "says": "the mug floats 2 cm", "by": "raea scene lint"}]})).unwrap()
    };
    let future = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() + 60;
    let s = run(&make, &d, &[entry(future, "error")], &rounds, later());
    assert!(s["gates"].to_string().contains("the mug floats"), "{s:#}");
    let s = run(&make, &d, &[entry(future, "warn")], &rounds, later());
    assert_eq!(s["done"], true, "a warn doesn't block: {s:#}");
    let s = run(&make, &d, &[entry(1, "warn")], &rounds, later());
    assert!(s["gates"].to_string().contains("changed since"), "{s:#}");
}

#[test]
fn a_scene_wants_three_stills() {
    let d = dir("scene");
    let make: Make = read("make", r#"{"kind": "scene", "objects": [{"id": "stool", "label": "Stool", "h": 0.45, "route": "gen", "prompt": "a red stool"}]}"#).unwrap();
    put(&d, "out/stills/a.png", "x");
    let s = run(&make, &d, &[], &[], None);
    assert!(s["missing"].to_string().contains("holds 1 picture"), "{s:#}");
}
