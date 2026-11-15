//! One React runtime's JS thread as its settle sees it, and the settle's watch over it (#158).
//!
//! A render runs several runtimes in one process (the 3D rows' workers, the page's frames), and settle's no-progress
//! guard read the process's counts: every runtime's flushes, and the JS time of a thread ticking an empty timer queue
//! every 500 ms. A frame a runtime lost was then waited on for good, with no output and no exit. Each runtime now keeps
//! its own [`JsState`]: its flushes, its work (an event delivered, or a timers call with a timer due; an empty tick isn't
//! work), its current call, and since when it has waited with nothing scheduled. [`Settling`] is one settle's watch:
//! when to give up, when to ask a stepped range's lost frame again, and what to say when it gives up.

use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::time::{Duration, Instant};

use bevy::prelude::Resource;

/// One runtime's JS thread: written by the thread, read by its settle.
pub struct JsState {
    base: Instant,
    /// commits this runtime's React has flushed
    pub flushed: AtomicU64,
    /// calls that did something: an event delivered, or a timers call with a timer due
    pub work: AtomicU64,
    /// when the thread's current call began, in ms after `base` plus 1 (0: not in a call)
    pub call_since: AtomicU64,
    /// since when the thread has waited with nothing scheduled (no timer set, no event), as call_since (0: not idle)
    pub idle_since: AtomicU64,
    /// events delivered while the bridge's event loop wasn't waiting for one: it had stopped, so they queue unread
    pub unheard: AtomicU64,
}

impl Default for JsState {
    fn default() -> Self {
        JsState { base: Instant::now(), flushed: AtomicU64::new(0), work: AtomicU64::new(0), call_since: AtomicU64::new(0), idle_since: AtomicU64::new(0), unheard: AtomicU64::new(0) }
    }
}

impl JsState {
    fn now(&self) -> u64 {
        self.base.elapsed().as_millis() as u64 + 1
    }
    /// (flushed, work): what settle's guard counts as this runtime moving.
    pub fn counts(&self) -> (u64, u64) {
        (self.flushed.load(SeqCst), self.work.load(SeqCst))
    }
    /// How long ago `at` (call_since or idle_since) was set, if it is.
    pub fn since(&self, at: &AtomicU64) -> Option<Duration> {
        match at.load(SeqCst) {
            0 => None,
            t => Some(self.base.elapsed().saturating_sub(Duration::from_millis(t - 1))),
        }
    }
    /// The thread enters a call (`true`) or leaves it.
    pub fn in_call(&self, entering: bool) {
        self.call_since.store(if entering { self.now() } else { 0 }, SeqCst);
    }
    /// A timers call returned `next` (ms to its next timer, negative for none); `due` says it had a timer to run. With
    /// nothing scheduled the thread is idle from now until an event is delivered (a lost one leaves it idle).
    pub fn timers(&self, due: bool, next: f64) {
        if due {
            self.work.fetch_add(1, SeqCst);
        }
        if next < 0.0 {
            let _ = self.idle_since.compare_exchange(0, self.now(), SeqCst, SeqCst);
        } else {
            self.idle_since.store(0, SeqCst);
        }
    }
    /// An event was delivered: `heard` when the bridge's loop took it, not queued unread.
    pub fn delivered(&self, heard: bool) {
        self.idle_since.store(0, SeqCst);
        self.work.fetch_add(1, SeqCst);
        if !heard {
            self.unheard.fetch_add(1, SeqCst);
        }
    }
}

/// The runtime's JsState, in its App (ReactPlugin puts it there and on its JS thread).
#[derive(Resource, Clone)]
pub struct ReactJs(pub std::sync::Arc<JsState>);

/// The range of frames being stepped (step_frames: raea-frames from..to), while it is: a settle that finds its runtime
/// idle with its frame not drawn asks for the rest of the range again from that frame.
#[derive(Resource, Clone, Copy)]
pub struct Stepping {
    pub to: i64,
}

/// RAEA_FAULT_DROP_EVENT=NAME: the first event named NAME never reaches the composition, as if it had missed it (the
/// lost frame settle's nudge recovers); NAME* drops every one (the frame lost for good, which settle fails naming).
/// tools/settle-nudge.sh runs both.
pub struct DropFault {
    name: Option<String>,
    every: bool,
}

impl DropFault {
    pub fn from_env() -> Self {
        let spec = std::env::var("RAEA_FAULT_DROP_EVENT").ok().filter(|n| !n.is_empty());
        let every = spec.as_deref().is_some_and(|n| n.ends_with('*'));
        DropFault { name: spec.map(|n| format!("\"name\":\"{}\"", n.trim_end_matches('*'))), every }
    }
    /// Whether to drop this event (said on stderr).
    pub fn drops(&mut self, json: &str) -> bool {
        if !self.name.as_deref().is_some_and(|n| json.contains(n)) {
            return false;
        }
        eprintln!("raea-runtime-react: RAEA_FAULT_DROP_EVENT dropped {json}");
        if !self.every {
            self.name = None;
        }
        true
    }
}

/// A runtime stepping a range whose JS thread has waited this long with nothing scheduled, its frame neither drawn nor
/// held, has lost the frame: nothing would ever come, so settle asks for the rest of the range again from it.
pub const NUDGE_IDLE_SECS: f64 = 1.0;

/// How many times a frame is asked for again (settle still fails only after its time with nothing moving).
pub const NUDGES: u32 = 3;

/// One settle's watch over its runtime: refused when nothing moves for its time (no commit, no JS work, nothing made on
/// first use: a matchmove solve takes minutes; a call still running counts, up to the caller's hang limit); a thread
/// that only runs timers (an interval polling for what never comes) is work, but not for longer than `hold` without a
/// commit. A total-time limit refused film.tsx under load (10-27).
pub struct Settling {
    still: Instant,
    moved: (u64, u64, f64),
    drawn: Instant,
    drew: (u64, f64),
    nudges: u32,
    nudged: Instant,
    /// refused because the thread only ran timers, committing nothing
    timers_only: bool,
}

impl Default for Settling {
    fn default() -> Self {
        let now = Instant::now();
        Settling { still: now, moved: (u64::MAX, u64::MAX, f64::NAN), drawn: now, drew: (u64::MAX, f64::NAN), nudges: 0, nudged: now, timers_only: false }
    }
}

impl Settling {
    /// This pass's counts (flushes, work, seconds spent making), whether a call is running, the time allowed with nothing
    /// moving and with only timers: the seconds left, or None to give up.
    pub fn watch(&mut self, now: (u64, u64, f64), in_call: bool, limit: f64, hold: f64) -> Option<f64> {
        if now != self.moved || in_call {
            self.moved = now;
            self.still = Instant::now();
        }
        if (now.0, now.2) != self.drew || in_call {
            self.drew = (now.0, now.2);
            self.drawn = Instant::now();
        }
        let left = limit - self.still.elapsed().as_secs_f64();
        self.timers_only = left > 0.0 && self.drawn.elapsed().as_secs_f64() >= limit.max(hold);
        (left > 0.0 && !self.timers_only).then_some(left)
    }
    /// Whether to ask frame `want` again now (said on stderr): a range is stepped (`to`), the frame isn't held, and the
    /// runtime's thread, in no call, has waited on nothing for NUDGE_IDLE_SECS, at most NUDGES times, twice that apart
    /// and not in the settle's first 2 x NUDGE_IDLE_SECS (the range's own event may not be taken yet).
    pub fn nudge(&mut self, js: &JsState, want: i64, to: i64, held: bool, marked: (i64, i64)) -> bool {
        let idle = js.call_since.load(SeqCst) == 0 && js.since(&js.idle_since).is_some_and(|d| d.as_secs_f64() >= NUDGE_IDLE_SECS);
        if !idle || held || want < 0 || want >= to || self.nudges >= NUDGES || self.nudged.elapsed().as_secs_f64() < 2.0 * NUDGE_IDLE_SECS {
            return false;
        }
        self.nudges += 1;
        self.nudged = Instant::now();
        eprintln!("raea: frame {want}'s runtime went idle without drawing it (its marker at frame {} done {}): asked for frames {want}..{to} again ({} of {NUDGES})", marked.0, marked.1, self.nudges);
        true
    }
    /// The settle's refusal of frame `want` after `secs` with nothing moving (`hold`, when only timers ran), its marker as
    /// last seen, and what the runtime's JS thread was doing (nothing said for a replay's App, which has none).
    pub fn refusal(&self, js: Option<&JsState>, want: Option<i64>, secs: f64, hold: f64, marked: (i64, i64)) -> String {
        let frame = want.map_or("?".to_string(), |w| w.to_string());
        let quiet = if self.timers_only { format!("{:.0} s (its JS thread ran only timers, committing nothing)", secs.max(hold)) } else { format!("{secs} s") };
        format!("the composition made no progress on frame {frame} for {quiet}, its marker at frame {} (done {}).{} Does it throw? Its [js] lines are above, and `raea traces show last` has the run's log", marked.0, marked.1, self.thread(js))
    }
    /// What the thread was doing when the settle gave up, as a sentence ("" with no JS thread).
    fn thread(&self, js: Option<&JsState>) -> String {
        let Some(j) = js else { return String::new() };
        let idle = j.since(&j.idle_since).map_or("busy or waiting on a timer".to_string(), |d| format!("idle for {:.1} s with nothing scheduled", d.as_secs_f64()));
        let asked = if self.nudges > 0 { format!(", asked for the frame again {} times", self.nudges) } else { String::new() };
        let unheard = j.unheard.load(SeqCst);
        let stopped = if unheard > 0 { format!("; its event loop had stopped taking events ({unheard} went unread: an error thrown out of an event handler?)") } else { String::new() };
        format!(" Its JS thread was {idle}{asked}{stopped}.")
    }
}
