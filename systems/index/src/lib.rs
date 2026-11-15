//! Index (docs/index.feature): every picture and video a person has, searchable in minutes. This crate is the fast
//! pass: sources are listed (walk.rs), each picture's small copy decoded and embedded by SigLIP 2 B/16-256 on the
//! Neural Engine (apple.rs) while the next ones decode, its row appended as its batch finishes (store.rs), and rows
//! searched by words through SigLIP 2's text tower (text.rs, search.rs). Libraries' deep pass (EmbeddingGemma 2 per
//! shot, sound and words) follows in the feed's host.

#[cfg(target_vendor = "apple")]
pub mod apple;
pub mod cli;
#[cfg(target_vendor = "apple")]
pub mod frames;
pub mod search;
pub mod serve;
pub mod store;
pub mod text;
pub mod walk;

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::Instant;
use store::{MODEL, Store};

/// A picture or a video a source holds, before the fast pass reads it
#[derive(Debug, Clone, Default)]
pub struct Found {
    /// where it lives in its source: a path, a Photos asset id, a Drive file id
    pub locator: String,
    /// "picture" or "video"
    pub kind: String,
    pub name: String,
    pub size: u64,
    /// seconds since 1970
    pub mtime: i64,
    pub taken: Option<i64>,
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub dur: Option<f64>,
}

/// How a run is going, said as it goes
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Progress {
    pub source: String,
    pub found: u64,
    pub done: u64,
    pub failed: u64,
    /// pictures embedded a second since the fast pass started
    pub per_sec: f64,
    pub seconds: f64,
    /// the items finished since the last report, newest last, for the wall of thumbnails
    pub newest: Vec<i64>,
    pub stage: &'static str,
}

pub fn normalise(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
    v.iter_mut().for_each(|x| *x /= n);
}

/// SigLIP 2 B/16-256's folder in the models folder ($RAEA_MODELS, else ~/.local/share/raea-models, as perception's):
/// the image tower's Core ML package, the text tower's ONNX and the tokenizer
pub fn siglip_dir() -> PathBuf {
    raea_io_home::env_path("RAEA_MODELS", ".local/share/raea-models").join("siglip2-base-p16-256")
}

pub fn image_package() -> PathBuf {
    siglip_dir().join("SigLIP2Image.mlpackage")
}

/// What listing a source found
#[derive(Debug, Clone, Default)]
pub struct Listed {
    pub source: String,
    pub found: u64,
    pub added: u64,
    /// items whose file changed since the last listing: their old rows are gone
    pub changed: u64,
}

/// Lists a folder or a drive into the index: new files added, changed ones put back in line
pub fn list_folder(store: &mut Store, root: &Path, on: &mut dyn FnMut(&Progress)) -> Result<Listed> {
    let label = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| root.display().to_string());
    let source = store.source("folder", &root.to_string_lossy(), &label)?;
    let started = Instant::now();
    let mut l = Listed { source: source.clone(), ..Listed::default() };
    let mut err = None;
    walk::walk(root, 2000, &mut |batch| {
        l.found += batch.len() as u64;
        match store.list(&source, &batch) {
            Ok((a, c)) => {
                l.added += a as u64;
                l.changed += c as u64;
            }
            Err(e) => {
                err.get_or_insert(e);
            }
        }
        on(&Progress { source: source.clone(), found: l.found, stage: "listing", seconds: started.elapsed().as_secs_f64(), ..Progress::default() });
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(l)
}

/// The fast pass over a source's pictures and videos that wait for it: decoded on every core but two (a video as a few
/// keyframes, frames.rs), embedded about 32 frames at a time on the Neural Engine, each batch's rows written as it
/// finishes. An item's frames go in one batch, so an item counts as searchable once all of its rows are written.
#[cfg(target_vendor = "apple")]
pub fn embed_pictures(store: &mut Store, model: &apple::Model, source: &str, stop: &std::sync::Arc<std::sync::atomic::AtomicBool>, on: &mut dyn FnMut(&Progress)) -> Result<Progress> {
    use std::sync::atomic::Ordering;
    use rayon::prelude::*;
    const BATCH: usize = 32;
    let all: Vec<store::Pending> = store.pending(source, usize::MAX)?;
    let base = store.counts(Some(source))?;
    let (tx, rx) = std::sync::mpsc::sync_channel::<Result<Vec<apple::Ready>, (i64, String)>>(BATCH * 8);
    let thumbs: Vec<PathBuf> = all.iter().map(|p| store.thumb(p.id)).collect();
    let threads = std::thread::available_parallelism().map(|n| n.get().saturating_sub(2).max(2)).unwrap_or(4);
    let halt = stop.clone();
    let decoder = std::thread::spawn(move || {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().expect("a decoding pool");
        pool.install(|| {
            all.par_iter().zip(thumbs.par_iter()).for_each_with(tx, |tx, (p, thumb)| {
                if halt.load(Ordering::Relaxed) {
                    return;
                }
                let file = Path::new(&p.locator);
                let r = if p.kind == "video" {
                    frames::video(file, Some(thumb)).map(|f| f.px.into_iter().map(|(px, t)| apple::Ready { item: p.id, t, px, w: f.w, h: f.h }).collect())
                } else {
                    apple::picture(file, Some(thumb)).map(|(px, w, h)| vec![apple::Ready { item: p.id, t: 0.0, px, w, h }])
                };
                let r = r.map_err(|e| (p.id, e));
                let _ = tx.send(r);
            })
        })
    });
    let started = Instant::now();
    let mut last = Instant::now();
    let mut prog = Progress { source: source.into(), found: base.found, done: base.done, failed: base.failed, stage: "embedding", ..Progress::default() };
    let mut batch: Vec<apple::Ready> = Vec::with_capacity(BATCH + frames::FRAMES);
    let flush = |batch: &mut Vec<apple::Ready>, prog: &mut Progress, store: &mut Store| -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }
        let pxs: Vec<&objc2_core_video::CVPixelBuffer> = batch.iter().map(|r| &*r.px).collect();
        let vs = model.embed(&pxs).map_err(|e| anyhow::anyhow!(e))?;
        // each frame's row and its item's size, in one pass
        let (rows, dims): (Vec<(i64, f64, Vec<f32>)>, Vec<(i64, u32, u32)>) = batch.iter().zip(vs).map(|(r, v)| ((r.item, r.t, v), (r.item, r.w, r.h))).unzip();
        store.append(MODEL, &rows, &dims)?;
        // an item's frames sit together: one count and one tile each
        let mut items: Vec<i64> = batch.iter().map(|r| r.item).collect();
        items.dedup();
        prog.done += items.len() as u64;
        prog.newest.extend(items);
        batch.clear();
        Ok(())
    };
    for r in rx {
        if stop.load(Ordering::Relaxed) {
            prog.stage = "stopped";
            break;
        }
        match r {
            Ok(item) => {
                batch.extend(item);
                if batch.len() >= BATCH {
                    flush(&mut batch, &mut prog, store)?;
                }
            }
            Err((item, why)) => {
                store.fail(item, &why)?;
                prog.failed += 1;
            }
        }
        if last.elapsed().as_millis() >= 250 {
            prog.seconds = started.elapsed().as_secs_f64();
            prog.per_sec = (prog.done - base.done) as f64 / prog.seconds.max(1e-3);
            on(&prog);
            prog.newest.clear();
            last = Instant::now();
        }
    }
    flush(&mut batch, &mut prog, store)?;
    let _ = decoder.join();
    prog.seconds = started.elapsed().as_secs_f64();
    prog.per_sec = (prog.done - base.done) as f64 / prog.seconds.max(1e-3);
    if prog.stage != "stopped" {
        prog.stage = "done";
    }
    on(&prog);
    prog.newest.clear();
    Ok(prog)
}
