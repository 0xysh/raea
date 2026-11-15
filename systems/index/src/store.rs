//! Where the fast pass keeps what it found (docs/index.feature IX1): a SQLite list of sources and items, and one file
//! of f16 rows per model, appended as batches finish, so search reads the first pictures while the rest are going.
//! An item is keyed by where it lives (its source and locator: a path, a Photos asset, a Drive file id), never by a
//! hash of a whole original. A row's number is its place in the model's file; a removed item's rows stay in the file
//! and are left out of search (compacted later).

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The fast pass's model, and the name of its rows' file
pub const MODEL: &str = "siglip2-b16-256@768";
pub const DIM: usize = 768;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sources (id TEXT PRIMARY KEY, kind TEXT NOT NULL, root TEXT NOT NULL, label TEXT NOT NULL, created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY, source TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE, locator TEXT NOT NULL,
  kind TEXT NOT NULL, name TEXT NOT NULL, size INTEGER NOT NULL, mtime INTEGER NOT NULL, taken INTEGER, w INTEGER, h INTEGER, dur REAL,
  sha256 TEXT, state INTEGER NOT NULL DEFAULT 0, err TEXT, UNIQUE (source, locator));
CREATE INDEX IF NOT EXISTS items_state ON items (source, state);
CREATE TABLE IF NOT EXISTS rows (model TEXT NOT NULL, row INTEGER NOT NULL, item INTEGER NOT NULL, t REAL NOT NULL DEFAULT 0, PRIMARY KEY (model, row));
CREATE INDEX IF NOT EXISTS rows_item ON rows (item);";

/// An item's state: listed, embedded (searchable), or failed with its reason
pub const LISTED: i64 = 0;
pub const DONE: i64 = 1;
pub const FAILED: i64 = 2;

pub struct Store {
    pub dir: PathBuf,
    pub db: Connection,
}

/// What a source holds, by state
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct Counts {
    pub found: u64,
    pub done: u64,
    pub failed: u64,
    pub rows: u64,
}

/// An item waiting for the fast pass
#[derive(Debug, Clone)]
pub struct Pending {
    pub id: i64,
    pub locator: String,
    pub kind: String,
}

impl Store {
    /// The index in `dir` ($RAEA_INDEX_HOME, else ~/.raea/library/fast), made if it isn't there
    pub fn open(dir: &Path) -> Result<Store> {
        std::fs::create_dir_all(dir.join("thumbs")).with_context(|| format!("making {}", dir.display()))?;
        let db = Connection::open(dir.join("index.db"))?;
        db.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;")?;
        db.execute_batch(SCHEMA)?;
        Ok(Store { dir: dir.to_path_buf(), db })
    }

    pub fn default_dir() -> PathBuf {
        std::env::var_os("RAEA_INDEX_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".raea/library/fast"))
    }

    pub fn vectors(&self, model: &str) -> PathBuf {
        self.dir.join(format!("{}.f16", model.replace('@', "-")))
    }

    pub fn thumb(&self, item: i64) -> PathBuf {
        self.dir.join("thumbs").join(format!("{}/{item}.jpg", item % 256))
    }

    /// A source by its kind and root; the same root again is the same source
    pub fn source(&self, kind: &str, root: &str, label: &str) -> Result<String> {
        if let Some(id) = self.db.query_row("SELECT id FROM sources WHERE kind = ?1 AND root = ?2", params![kind, root], |r| r.get(0)).optional()? {
            return Ok(id);
        }
        let n: i64 = self.db.query_row("SELECT COUNT(*) FROM sources", [], |r| r.get(0))?;
        let id = format!("ix{}-{n}", kind.chars().next().unwrap_or('s'));
        self.db.execute("INSERT INTO sources (id, kind, root, label, created) VALUES (?1, ?2, ?3, ?4, strftime('%s','now'))", params![id, kind, root, label])?;
        Ok(id)
    }

    /// Lists items in one transaction: new ones as LISTED, a changed one (size or mtime) back to LISTED; the rest kept
    pub fn list(&mut self, source: &str, items: &[crate::Found]) -> Result<usize> {
        let tx = self.db.transaction()?;
        let mut added = 0;
        {
            let mut up = tx.prepare_cached(
                "INSERT INTO items (source, locator, kind, name, size, mtime, taken, w, h, dur) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT (source, locator) DO UPDATE SET size = excluded.size, mtime = excluded.mtime, state = 0, err = NULL
                 WHERE items.size != excluded.size OR items.mtime != excluded.mtime",
            )?;
            for f in items {
                added += up.execute(params![source, f.locator, f.kind, f.name, f.size as i64, f.mtime, f.taken, f.w, f.h, f.dur])?;
            }
        }
        tx.commit()?;
        Ok(added)
    }

    pub fn pending(&self, source: &str, limit: usize) -> Result<Vec<Pending>> {
        let mut q = self.db.prepare_cached("SELECT id, locator, kind FROM items WHERE source = ?1 AND state = 0 ORDER BY taken DESC, id LIMIT ?2")?;
        let rows = q.query_map(params![source, limit as i64], |r| Ok(Pending { id: r.get(0)?, locator: r.get(1)?, kind: r.get(2)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// One batch's vectors: appended to the model's file, each row tied to its item and moment, the items marked done
    pub fn append(&mut self, model: &str, batch: &[(i64, f64, Vec<f32>)], dims: &[(i64, u32, u32)]) -> Result<()> {
        let file = self.vectors(model);
        let mut f = OpenOptions::new().create(true).append(true).open(&file)?;
        let start = f.metadata()?.len() / (DIM as u64 * 2);
        let mut bytes = Vec::with_capacity(batch.len() * DIM * 2);
        for (_, _, v) in batch {
            for x in v {
                bytes.extend_from_slice(&half::f16::from_f32(*x).to_le_bytes());
            }
        }
        f.write_all(&bytes)?;
        f.sync_data()?;
        let tx = self.db.transaction()?;
        {
            let mut row = tx.prepare_cached("INSERT OR REPLACE INTO rows (model, row, item, t) VALUES (?1, ?2, ?3, ?4)")?;
            for (i, (item, t, _)) in batch.iter().enumerate() {
                row.execute(params![model, start as i64 + i as i64, item, t])?;
            }
            let mut done = tx.prepare_cached("UPDATE items SET state = 1, w = COALESCE(w, ?2), h = COALESCE(h, ?3) WHERE id = ?1")?;
            for (item, w, h) in dims {
                done.execute(params![item, w, h])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn fail(&self, item: i64, why: &str) -> Result<()> {
        self.db.execute("UPDATE items SET state = 2, err = ?2 WHERE id = ?1", params![item, why])?;
        Ok(())
    }

    pub fn counts(&self, source: Option<&str>) -> Result<Counts> {
        let (w, p) = match source {
            Some(s) => ("WHERE source = ?1", vec![s.to_string()]),
            None => ("", vec![]),
        };
        let q = format!("SELECT COUNT(*), SUM(state = 1), SUM(state = 2) FROM items {w}");
        let (found, done, failed): (i64, Option<i64>, Option<i64>) = self.db.query_row(&q, rusqlite::params_from_iter(p.iter()), |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        let rows: i64 = self.db.query_row("SELECT COUNT(*) FROM rows", [], |r| r.get(0))?;
        Ok(Counts { found: found as u64, done: done.unwrap_or(0) as u64, failed: failed.unwrap_or(0) as u64, rows: rows as u64 })
    }
}
