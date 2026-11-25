//! A voice session: lines of JSON in (what to say), lines of JSON out (what was heard, and how the saying goes).
//!
//!   in   {"say":{"id":"u1","text":"Got it. "}}   more of utterance u1's words (send as a model streams them)
//!        {"end":"u1"}                            u1 has all its words
//!        {"stop":true}                           stop talking now
//!        {"mute":true|false}                     stop or start listening
//!        {"quit":true}                           (or the input closing) say what's left, then end
//!   out  {"fetch":{"file","mb"}}                 a model downloading, the first time
//!        {"ready":{"voice","hears","rate"}}      it can hear and speak
//!        {"level":-37.5}                         the microphone's loudness (dBFS), a few times a second
//!        {"speech":"began"|"ended"}              the person started or stopped speaking
//!        {"heard":{"text","seconds","ms"}}       what they said, how long they spoke and how long hearing it took
//!        {"saying":{"id","first_ms"}}            u1's first sound, and how long after its first words came
//!        {"word":{"id","i","text"}}              a word of u1 starting to sound
//!        {"said":{"id"}}                         u1 was said to its end
//!        {"stopped":{"id"}}                      u1 was cut off (a stop, or the person spoke over it)
//!        {"error":"..."}
//!
//! While raea speaks it doesn't listen, unless `duplex` (headphones): a laptop's microphone hears its own speakers,
//! and raea would take its own voice for the person interrupting.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::audio::{microphone, microphone_file, Speaker};
use crate::speak::{Say, Words, RATE};
use crate::turns::{Turn, Turns, Vad, WIN};

pub struct Opts {
    pub from: Option<PathBuf>,
    pub to: Option<PathBuf>,
    pub voice: Option<String>,
    pub duplex: bool,
    pub fast: bool,
    pub pause_ms: usize,
}

pub fn emit(v: Value) {
    let mut o = std::io::stdout().lock();
    let _ = writeln!(o, "{v}");
    let _ = o.flush();
}

/// What a line of input asks for; None for a line that asks nothing known.
pub enum Ask {
    Say(Say),
    Mute(bool),
    Quit,
}

pub fn ask(line: &str) -> Option<Ask> {
    let v: Value = serde_json::from_str(line).ok()?;
    if let Some(s) = v.get("say") {
        return Some(Ask::Say(Say::Text { id: s.get("id")?.as_str()?.to_string(), text: s.get("text")?.as_str()?.to_string() }));
    }
    if let Some(id) = v.get("end").and_then(Value::as_str) {
        return Some(Ask::Say(Say::End(id.to_string())));
    }
    if v.get("stop").is_some() {
        return Some(Ask::Say(Say::Stop));
    }
    if let Some(m) = v.get("mute").and_then(Value::as_bool) {
        return Some(Ask::Mute(m));
    }
    v.get("quit").map(|_| Ask::Quit)
}

/// dBFS of a window, one decimal.
fn level(win: &[f32]) -> f64 {
    let rms = (win.iter().map(|s| s * s).sum::<f32>() / win.len().max(1) as f32).sqrt();
    ((20.0 * rms.max(1e-6).log10()) as f64 * 10.0).round() / 10.0
}

pub fn run(o: Opts) -> anyhow::Result<()> {
    let speaker = Speaker::open(RATE, o.to.clone(), o.fast)?;
    let words = Arc::new(Mutex::new(Words::default()));
    let talking = Arc::new(AtomicBool::new(false));
    let muted = Arc::new(AtomicBool::new(false));
    let closing = Arc::new(AtomicBool::new(false));
    let hearing = Arc::new(AtomicUsize::new(0));

    // the voice
    let (say_tx, say_rx) = channel::<Say>();
    let (voice_ready, voice_is) = channel();
    let (line, w, t, name) = (speaker.line.clone(), words.clone(), talking.clone(), o.voice.clone());
    let voice = std::thread::spawn(move || crate::speak::worker(say_rx, line, w, name, t, voice_ready, emit));

    // the ears: each turn's sound heard on its own thread, so the microphone is never kept waiting
    let (heard_tx, heard_rx) = channel::<Vec<f32>>();
    let (ears_ready, ears_are) = channel();
    let pending = hearing.clone();
    std::thread::spawn(move || {
        let loaded = raea_sys_ml::models::ensure(&crate::asr::SPEC, &mut crate::fetching(emit)).and_then(|d| crate::asr::Asr::load(&d));
        let mut asr = match loaded {
            Ok(a) => {
                let _ = ears_ready.send(Ok(()));
                a
            }
            Err(e) => {
                let _ = ears_ready.send(Err(e));
                return;
            }
        };
        for said in heard_rx {
            let t = Instant::now();
            match asr.hear(&said) {
                Ok(text) => emit(json!({ "heard": { "text": text, "seconds": (said.len() as f64 / crate::turns::RATE as f64 * 100.0).round() / 100.0, "ms": t.elapsed().as_millis() as u64 } })),
                Err(e) => emit(json!({ "error": format!("{e:#}") })),
            }
            pending.fetch_sub(1, Ordering::Relaxed);
        }
    });
    let mut vad = Vad::load(&crate::turns::model(&mut crate::fetching(emit))?)?;
    let name = voice_is.recv().map_err(|_| anyhow::anyhow!("the voice stopped before it could speak"))??;
    ears_are.recv().map_err(|_| anyhow::anyhow!("the ears stopped before they could hear"))??;

    // what's asked, line by line
    let (tx, m, c) = (say_tx.clone(), muted.clone(), closing.clone());
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            match ask(&line) {
                Some(Ask::Say(s)) => drop(tx.send(s)),
                Some(Ask::Mute(on)) => m.store(on, Ordering::Relaxed),
                Some(Ask::Quit) => break,
                None if line.trim().is_empty() => {}
                None => emit(json!({ "error": format!("not a line raea voice knows: {}", line.chars().take(120).collect::<String>()) })),
            }
        }
        let _ = tx.send(Say::Close);
        c.store(true, Ordering::Relaxed);
    });

    let (mic_tx, mic_rx) = channel::<Vec<f32>>();
    let _mic = match &o.from {
        Some(f) => {
            microphone_file(f, o.fast, mic_tx)?;
            None
        }
        None => Some(microphone(mic_tx)?),
    };
    emit(json!({ "ready": { "voice": name, "hears": "parakeet-tdt-0.6b-v3", "rate": RATE } }));

    let mut turns = Turns::new(o.pause_ms);
    let (mut buf, mut windows, mut mic_done) = (Vec::<f32>::new(), 0usize, false);
    let mut quiet_since = Instant::now();
    loop {
        match mic_rx.recv_timeout(Duration::from_millis(30)) {
            Ok(piece) => buf.extend(piece),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => mic_done = true,
        }
        while buf.len() >= WIN {
            let win: Vec<f32> = buf.drain(..WIN).collect();
            windows += 1;
            let speaking = talking.load(Ordering::Relaxed);
            if speaking {
                quiet_since = Instant::now();
            }
            // its own voice dies away in the room for a moment after the last sample
            let own = speaking || quiet_since.elapsed() < Duration::from_millis(250);
            if muted.load(Ordering::Relaxed) || (own && !o.duplex) {
                if turns.speaking() {
                    emit(json!({ "speech": "ended" }));
                }
                // muted, their words are dropped; talked over by raea, what they had said so far is still heard
                if muted.load(Ordering::Relaxed) {
                    turns.clear();
                } else if let Some(said) = turns.cut() {
                    hearing.fetch_add(1, Ordering::Relaxed);
                    let _ = heard_tx.send(said);
                }
                vad.reset();
                continue;
            }
            if windows % 4 == 0 {
                emit(json!({ "level": level(&win) }));
            }
            match turns.push(&win, vad.speech(&win)?) {
                Some(Turn::Began) => {
                    emit(json!({ "speech": "began" }));
                    if speaking {
                        let _ = say_tx.send(Say::Stop);
                    }
                }
                Some(Turn::Ended(said)) => {
                    emit(json!({ "speech": "ended" }));
                    hearing.fetch_add(1, Ordering::Relaxed);
                    let _ = heard_tx.send(said);
                }
                None => {}
            }
        }
        // the words whose sound has started
        {
            let mut w = words.lock().unwrap();
            let played = speaker.line.at().0;
            while w.told < w.starts.len() && w.told < w.words.len() && played >= w.start_at + (w.starts[w.told] * RATE as f64) as u64 {
                emit(json!({ "word": { "id": w.id, "i": w.told, "text": w.words[w.told] } }));
                w.told += 1;
            }
        }
        let idle = voice.is_finished() && hearing.load(Ordering::Relaxed) == 0;
        if closing.load(Ordering::Relaxed) && idle && (o.from.is_none() || mic_done) {
            break;
        }
    }
    speaker.close()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lines_a_session_takes() {
        assert!(matches!(ask(r#"{"say":{"id":"u1","text":"Hello. "}}"#), Some(Ask::Say(Say::Text { id, text })) if id == "u1" && text == "Hello. "));
        assert!(matches!(ask(r#"{"end":"u1"}"#), Some(Ask::Say(Say::End(id))) if id == "u1"));
        assert!(matches!(ask(r#"{"stop":true}"#), Some(Ask::Say(Say::Stop))));
        assert!(matches!(ask(r#"{"mute":true}"#), Some(Ask::Mute(true))));
        assert!(matches!(ask(r#"{"quit":true}"#), Some(Ask::Quit)));
        assert!(ask(r#"{"sing":"la"}"#).is_none() && ask("not json").is_none());
    }

    #[test]
    fn quiet_and_loud_in_dbfs() {
        assert_eq!(level(&[0.0; 512]), -120.0);
        assert_eq!(level(&[1.0; 512]), 0.0);
        assert_eq!(level(&[0.1; 512]), -20.0);
    }
}
