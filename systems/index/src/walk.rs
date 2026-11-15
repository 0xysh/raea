//! Listing a folder or a drive (docs/index.feature IX5): every picture and video under it, with its size and dates,
//! fast, without opening a file. Packages (a Photos library, an app, a Final Cut library), hidden folders, caches and
//! raea's own working folders are skipped: Photos has its own source, and the rest isn't anyone's media.

use crate::Found;
use std::path::Path;
use std::time::UNIX_EPOCH;

pub const PICTURES: &[&str] = &["jpg", "jpeg", "heic", "heif", "png", "webp", "gif", "tif", "tiff", "bmp", "avif", "dng", "cr2", "cr3", "nef", "arw", "raf", "orf", "rw2"];
pub const VIDEOS: &[&str] = &["mp4", "mov", "m4v", "avi", "mkv", "webm", "mts", "m2ts", "3gp"];

/// What a file is by its extension: "picture", "video", or nothing
pub fn kind_of(p: &Path) -> Option<&'static str> {
    let ext = p.extension()?.to_str()?.to_ascii_lowercase();
    if PICTURES.contains(&ext.as_str()) {
        Some("picture")
    } else if VIDEOS.contains(&ext.as_str()) {
        Some("video")
    } else {
        None
    }
}

const PACKAGES: &[&str] = &["photoslibrary", "app", "fcpbundle", "imovielibrary", "photolibrary", "aplibrary", "bundle", "framework", "xcassets", "mlpackage", "mlmodelc"];
const SKIP: &[&str] = &["node_modules", "target", "Caches", "Library", ".Trash", "proxies", "scene-frames"];

fn skipped(name: &str, depth: usize) -> bool {
    if name.starts_with('.') {
        return true;
    }
    if let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) {
        if PACKAGES.contains(&ext.to_ascii_lowercase().as_str()) {
            return true;
        }
    }
    // a home folder's own Library is skipped; one deeper (a project called Library) is kept
    SKIP.contains(&name) && (name != "Library" || depth <= 1)
}

/// Every picture and video under `root`, `on` called with each batch of `every` as they're found
pub fn walk(root: &Path, every: usize, on: &mut dyn FnMut(Vec<Found>)) -> u64 {
    let mut batch = Vec::with_capacity(every);
    let mut n = 0u64;
    let it = walkdir::WalkDir::new(root).follow_links(false).into_iter().filter_entry(|e| e.depth() == 0 || !skipped(&e.file_name().to_string_lossy(), e.depth()));
    for e in it.flatten() {
        // a link to a file counts as the file (a link to a folder isn't followed: no loops)
        let link = e.path_is_symlink();
        if !e.file_type().is_file() && !link {
            continue;
        }
        let Some(kind) = kind_of(e.path()) else { continue };
        let Ok(m) = (if link { std::fs::metadata(e.path()) } else { e.metadata().map_err(std::io::Error::other) }) else { continue };
        if !m.is_file() {
            continue;
        }
        if m.len() < 1024 {
            continue;
        }
        let secs = |t: std::io::Result<std::time::SystemTime>| t.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64);
        let mtime = secs(m.modified()).unwrap_or(0);
        batch.push(Found {
            locator: e.path().to_string_lossy().into_owned(),
            kind: kind.into(),
            name: e.file_name().to_string_lossy().into_owned(),
            size: m.len(),
            mtime,
            taken: secs(m.created()).or(Some(mtime)),
            ..Found::default()
        });
        n += 1;
        if batch.len() >= every {
            on(std::mem::replace(&mut batch, Vec::with_capacity(every)));
        }
    }
    if !batch.is_empty() {
        on(batch);
    }
    n
}
