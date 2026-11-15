//! Searching the fast pass (docs/index.feature IX3): a query vector against every row of the model's file, brute
//! force over f16 on every core, best `k` kept, one hit per item (its best moment). Rows of removed or changed items
//! are left out.

use crate::store::{DIM, Store};
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Hit {
    pub item: i64,
    pub score: f32,
    /// the moment in a video, seconds; 0 for a picture
    pub t: f64,
    pub kind: String,
    pub name: String,
    pub locator: String,
    pub thumb: String,
}

/// The rows' vectors as f16 bits, and which item and moment each row is
pub struct Rows {
    pub bits: Vec<u16>,
    pub item: Vec<i64>,
    pub t: Vec<f64>,
}

impl Rows {
    pub fn load(store: &Store, model: &str) -> Result<Rows> {
        let raw = std::fs::read(store.vectors(model)).unwrap_or_default();
        let n = raw.len() / (DIM * 2);
        let bits: Vec<u16> = raw[..n * DIM * 2].chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
        let mut item = vec![-1i64; n];
        let mut t = vec![0f64; n];
        let mut q = store.db.prepare("SELECT r.row, r.item, r.t FROM rows r JOIN items i ON i.id = r.item WHERE r.model = ?1 AND i.state = 1")?;
        let mut it = q.query(rusqlite::params![model])?;
        while let Some(r) = it.next()? {
            let row: i64 = r.get(0)?;
            if (row as usize) < n {
                item[row as usize] = r.get(1)?;
                t[row as usize] = r.get(2)?;
            }
        }
        Ok(Rows { bits, item, t })
    }

    pub fn len(&self) -> usize {
        self.item.len()
    }
}

fn dot(bits: &[u16], q: &[f32]) -> f32 {
    let mut acc = [0f32; 8];
    for (c, qc) in bits.chunks_exact(8).zip(q.chunks_exact(8)) {
        for j in 0..8 {
            acc[j] += half::f16::from_bits(c[j]).to_f32() * qc[j];
        }
    }
    acc.iter().sum()
}

/// The `k` best items for the query, best first
pub fn search(store: &Store, rows: &Rows, q: &[f32], k: usize) -> Result<Vec<Hit>> {
    let scored: Vec<(usize, f32)> = rows
        .bits
        .par_chunks_exact(DIM)
        .enumerate()
        .filter(|(i, _)| rows.item[*i] >= 0)
        .map(|(i, v)| (i, dot(v, q)))
        .collect();
    // best row per item
    let mut best: HashMap<i64, (usize, f32)> = HashMap::new();
    for (i, s) in scored {
        let e = best.entry(rows.item[i]).or_insert((i, f32::MIN));
        if s > e.1 {
            *e = (i, s);
        }
    }
    let mut top: Vec<(i64, usize, f32)> = best.into_iter().map(|(item, (i, s))| (item, i, s)).collect();
    top.sort_by(|a, b| b.2.total_cmp(&a.2));
    top.truncate(k);
    let mut q = store.db.prepare_cached("SELECT kind, name, locator FROM items WHERE id = ?1")?;
    let mut hits = Vec::with_capacity(top.len());
    for (item, i, score) in top {
        let (kind, name, locator): (String, String, String) = q.query_row([item], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        hits.push(Hit { item, score, t: rows.t[i], kind, name, locator, thumb: store.thumb(item).to_string_lossy().into_owned() });
    }
    Ok(hits)
}
