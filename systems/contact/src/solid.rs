//! A part's solid: its triangles voxelized in its own space, each cell's depth inside it.

use super::*;

/// A part's solid in its own space: each cell's depth inside it (0 outside, 1 at the surface).
pub(crate) struct Solid {
    /// the glTF node it was (empty for a mesh or a one-mesh model)
    pub(crate) name: String,
    pub(crate) min: Vec3,
    pub(crate) cell: f32,
    pub(crate) n: [usize; 3],
    pub(crate) depth: Vec<u16>,
    /// the cells at least one cell under the surface
    pub(crate) inside: Vec<u32>,
    /// points on its real surface (from its triangles), SURF_GRAIN of a cell apart: what the pierce test puts in
    /// another part
    pub(crate) surf: Vec<Vec3>,
    /// a thin part's finer solid, the pierce test's depths in it
    pub(crate) fine: Option<Box<Solid>>,
    pub(crate) lo: Vec3,
    pub(crate) hi: Vec3,
}

impl Solid {
    pub(crate) fn at(&self, p: Vec3) -> u16 {
        let q = (p - self.min) / self.cell;
        if q.x < 0.0 || q.y < 0.0 || q.z < 0.0 {
            return 0;
        }
        let (x, y, z) = (q.x as usize, q.y as usize, q.z as usize);
        if x >= self.n[0] || y >= self.n[1] || z >= self.n[2] {
            return 0;
        }
        self.depth[(z * self.n[1] + y) * self.n[0] + x]
    }
    pub(crate) fn centre(&self, i: u32) -> Vec3 {
        let i = i as usize;
        let (x, y, z) = (i % self.n[0], (i / self.n[0]) % self.n[1], i / (self.n[0] * self.n[1]));
        self.min + Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5) * self.cell
    }
    /// whether it has an inside at all: at RES, or in its fine solid (a sill a cell thick at RES)
    pub(crate) fn solid(&self) -> bool {
        !self.inside.is_empty() || self.fine.as_ref().is_some_and(|f| !f.inside.is_empty())
    }
    /// its volume, from the finer of its solids
    pub(crate) fn volume(&self) -> f32 {
        let s = self.fine.as_deref().filter(|f| !f.inside.is_empty()).unwrap_or(self);
        s.inside.len() as f32 * s.cell.powi(3)
    }
    /// how far under the surface a cell of this depth is, in the part's own units (the surface was thickened a cell
    /// to close pinholes)
    pub(crate) fn metres(&self, d: u16) -> f32 {
        (d as f32 - 1.5).max(0.0) * self.cell
    }
}

/// Triangles to a solid. The surface's cells are marked (each triangle sampled at half a cell) and thickened by one;
/// the outside is flooded in from the grid's top and sides, moving sideways and down but never up, so a model open
/// underneath (a column with no bottom face, a scanned bust) is still solid, while a cup or an arch stays open.
pub(crate) fn voxelize(tris: &[[Vec3; 3]]) -> Option<Solid> {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for t in tris {
        for p in t {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
    }
    let ext = hi - lo;
    if !ext.is_finite() || ext.max_element() <= 0.0 {
        return None;
    }
    let cell = ext.max_element() / RES;
    let thin = (ext.min_element() / THIN).max(ext.max_element() / (RES * THIN_MOST));
    let mut solid = grid(tris, lo, hi, cell);
    solid.surf = surface(tris, cell * SURF_GRAIN);
    if thin < 0.5 * cell {
        solid.fine = Some(Box::new(grid(tris, lo, hi, thin)));
    }
    Some(solid)
}

/// Points on the triangles, one per `grain` cube they cross.
pub(crate) fn surface(tris: &[[Vec3; 3]], grain: f32) -> Vec<Vec3> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for [a, b, c] in tris {
        let long = (*b - *a).length().max((*c - *b).length()).max((*a - *c).length());
        let m = ((long / (0.5 * grain)).ceil() as usize).clamp(1, 256);
        for i in 0..=m {
            for j in 0..=(m - i) {
                let (u, v) = (i as f32 / m as f32, j as f32 / m as f32);
                let p = *a + (*b - *a) * u + (*c - *a) * v;
                if seen.insert((p / grain).floor().as_ivec3().to_array()) {
                    out.push(p);
                }
            }
        }
    }
    out
}

/// The solid of triangles in the box lo..hi at this cell size.
pub(crate) fn grid(tris: &[[Vec3; 3]], lo: Vec3, hi: Vec3, cell: f32) -> Solid {
    let ext = hi - lo;
    // three cells clear all round: a face rounding into the second, thickened, still leaves the outside a way under
    let min = lo - Vec3::splat(3.0 * cell);
    let n = [((ext.x / cell) as usize) + 7, ((ext.y / cell) as usize) + 7, ((ext.z / cell) as usize) + 7];
    let idx = |x: usize, y: usize, z: usize| (z * n[1] + y) * n[0] + x;
    let mut wall = vec![false; n[0] * n[1] * n[2]];
    let mark = |wall: &mut Vec<bool>, p: Vec3| {
        let q = (p - min) / cell;
        let (x, y, z) = (q.x as usize, q.y as usize, q.z as usize);
        if x < n[0] && y < n[1] && z < n[2] {
            wall[idx(x, y, z)] = true;
        }
    };
    for [a, b, c] in tris {
        let long = (*b - *a).length().max((*c - *b).length()).max((*a - *c).length());
        let m = ((long / (0.5 * cell)).ceil() as usize).clamp(1, 1024);
        for i in 0..=m {
            for j in 0..=(m - i) {
                let (u, v) = (i as f32 / m as f32, j as f32 / m as f32);
                mark(&mut wall, *a + (*b - *a) * u + (*c - *a) * v);
            }
        }
    }
    // thickened a cell, so a pinhole doesn't let the outside in
    let mut thick = wall.clone();
    for z in 1..n[2] - 1 {
        for y in 1..n[1] - 1 {
            for x in 1..n[0] - 1 {
                if wall[idx(x, y, z)] {
                    for (dx, dy, dz) in [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
                        thick[idx((x as i64 + dx) as usize, (y as i64 + dy) as usize, (z as i64 + dz) as usize)] = true;
                    }
                }
            }
        }
    }
    // the outside: in from the top and the sides, then sideways and down
    let mut out = vec![false; wall.len()];
    let mut q = VecDeque::new();
    for z in 0..n[2] {
        for y in 0..n[1] {
            for x in 0..n[0] {
                let edge = x == 0 || x == n[0] - 1 || z == 0 || z == n[2] - 1 || y == n[1] - 1;
                if edge && !thick[idx(x, y, z)] {
                    out[idx(x, y, z)] = true;
                    q.push_back((x, y, z));
                }
            }
        }
    }
    while let Some((x, y, z)) = q.pop_front() {
        for (dx, dy, dz) in [(1i64, 0i64, 0i64), (-1, 0, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
            let (a, b, c) = (x as i64 + dx, y as i64 + dy, z as i64 + dz);
            if a < 0 || b < 0 || c < 0 || a >= n[0] as i64 || b >= n[1] as i64 || c >= n[2] as i64 {
                continue;
            }
            let k = idx(a as usize, b as usize, c as usize);
            if !out[k] && !thick[k] {
                out[k] = true;
                q.push_back((a as usize, b as usize, c as usize));
            }
        }
    }
    // each inside cell's depth: its steps from the outside
    let mut depth = vec![0u16; wall.len()];
    for z in 0..n[2] {
        for y in 0..n[1] {
            for x in 0..n[0] {
                if out[idx(x, y, z)] {
                    q.push_back((x, y, z));
                }
            }
        }
    }
    let mut seen = out;
    while let Some((x, y, z)) = q.pop_front() {
        let d = depth[idx(x, y, z)];
        for (dx, dy, dz) in [(1i64, 0i64, 0i64), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
            let (a, b, c) = (x as i64 + dx, y as i64 + dy, z as i64 + dz);
            if a < 0 || b < 0 || c < 0 || a >= n[0] as i64 || b >= n[1] as i64 || c >= n[2] as i64 {
                continue;
            }
            let k = idx(a as usize, b as usize, c as usize);
            if !seen[k] {
                seen[k] = true;
                depth[k] = d + 1;
                q.push_back((a as usize, b as usize, c as usize));
            }
        }
    }
    let inside: Vec<u32> = (0..depth.len() as u32).filter(|&i| depth[i as usize] >= 3).collect();
    Solid { name: String::new(), min, cell, n, depth, inside, surf: Vec::new(), fine: None, lo, hi }
}

/// Each file's or shape expression's solids, made once until [`crate::forget`]
static CACHE: OnceLock<Mutex<HashMap<String, Result<Arc<Vec<Solid>>, String>>>> = OnceLock::new();

/// Lets every made solid go (crate::forget).
pub(crate) fn forget_solids() {
    if let Some(c) = CACHE.get() {
        c.lock().unwrap().clear();
    }
}

/// What a part's solids are made from: its model's file, or its shape expression.
fn key_of(it: &Item) -> String {
    match it.kind {
        "model" => format!("model:{}", src_path(it.props.get("src").and_then(Value::as_str).unwrap_or(""))),
        _ => format!("mesh:{}", it.shape.as_deref().unwrap_or("")),
    }
}

/// A part's solids (one per mesh node of a model), made once per file or shape expression this process (an Err: not
/// checked, and why).
pub(crate) fn solid_of(it: &Item) -> Result<Arc<Vec<Solid>>, String> {
    let key = key_of(it);
    if let Some(s) = CACHE.get_or_init(Default::default).lock().unwrap().get(&key) {
        return s.clone();
    }
    let made = make(it, &key);
    CACHE.get_or_init(Default::default).lock().unwrap().insert(key, made.clone());
    made
}

fn make(it: &Item, key: &str) -> Result<Arc<Vec<Solid>>, String> {
    let groups: Vec<(String, Vec<[Vec3; 3]>)> = if it.kind == "model" {
        let (g, moves) = gltf_tris(&key["model:".len()..])?;
        if moves {
            return Err("it animates".into());
        }
        g
    } else {
        let t = raea_core_shape::cached(it.shape.as_deref().ok_or("no shape")?)?;
        vec![(String::new(), t.positions.chunks_exact(3).map(|c| [Vec3::from(c[0]), Vec3::from(c[1]), Vec3::from(c[2])]).collect())]
    };
    // the whole model, and each mesh on its own when there are several: a statue in open pieces is only solid
    // whole, a ruin's thin column only resolved alone
    let mut solids: Vec<Solid> = Vec::new();
    if groups.len() > 1 {
        let whole: Vec<[Vec3; 3]> = groups.iter().flat_map(|(_, t)| t.iter().copied()).collect();
        solids.extend(voxelize(&whole));
        solids.extend(groups.into_iter().filter_map(|(name, t)| voxelize(&t).map(|mut s| {
            s.name = name;
            s
        })));
    } else {
        solids.extend(groups.into_iter().filter_map(|(_, t)| voxelize(&t)));
    }
    solids.retain(Solid::solid);
    if solids.is_empty() { Err("it has no volume".to_string()) } else { Ok(Arc::new(solids)) }
}

/// The parts' solids over a check's frames (`check`): a shape's made at its part's first frame, shared by the parts
/// holding it, and let go after the last of them; a model's from the process's cache (its file, a few at most). A
/// film's shapes change as its words and parts do: a 13,175-frame film's 1,323 shapes held at once were 2.9 GB of cells
/// at its end, its render's peak (docs/stage-guards.feature SG4a).
pub(crate) struct Held {
    pub(crate) solids: Vec<Option<Arc<Vec<Solid>>>>,
    /// the parts that start on each frame, and those whose last frame it is
    starts: Vec<Vec<usize>>,
    ends: Vec<Vec<usize>>,
    shared: HashMap<String, Result<std::sync::Weak<Vec<Solid>>, String>>,
}

impl Held {
    pub(crate) fn new(parts: &[(String, Span)], frames: usize) -> Held {
        let (mut starts, mut ends) = (vec![Vec::new(); frames], vec![Vec::new(); frames]);
        for (i, (_, s)) in parts.iter().enumerate() {
            starts[s.first].push(i);
            ends[s.first + s.items.len() - 1].push(i);
        }
        Held { solids: vec![None; parts.len()], starts, ends, shared: HashMap::new() }
    }

    /// Frame k's new parts' solids.
    pub(crate) fn at(&mut self, parts: &[(String, Span)], k: usize) {
        for &i in &self.starts[k] {
            let Some(it) = parts[i].1.items.iter().flatten().next() else { continue };
            let key = key_of(it);
            let had = match self.shared.get(&key) {
                Some(Ok(w)) => w.upgrade().map(Ok),
                Some(Err(e)) => Some(Err(e.clone())),
                None => None,
            };
            let s = had.unwrap_or_else(|| {
                let s = if it.kind == "model" { solid_of(it) } else { make(it, &key) };
                self.shared.insert(key, s.as_ref().map(Arc::downgrade).map_err(Clone::clone));
                s
            });
            self.solids[i] = s.ok();
        }
    }

    /// After frame k: the solids of the parts it was the last frame of let go, but those `keep` names (an overlap still
    /// to be said with their pieces' names).
    pub(crate) fn after(&mut self, k: usize, keep: impl Fn(usize) -> bool) {
        for &i in &self.ends[k] {
            if !keep(i) {
                self.solids[i] = None;
            }
        }
    }
}

/// The pieces that hide others on frame k: every checked part there, see-through ones left out.
pub(crate) fn occluders<'a>(parts: &'a [(String, Span)], solids: &'a [Option<Arc<Vec<Solid>>>], k: usize) -> Vec<(&'a Solid, Mat4)> {
    parts.iter().enumerate().filter_map(|(i, (_, tr))| Some((tr.at(k)?, solids[i].as_ref()?)))
        .filter(|(it, _)| !excused(it) || it.props.get("allowOverlap").and_then(Value::as_bool) == Some(true))
        .flat_map(|(it, ss)| ss.iter().map(move |s| (s, it.world)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(shape: &str) -> Item {
        Item { kind: "mesh", props: Arc::new(serde_json::Map::new()), shape: Some(shape.into()), material: None, at: None, world: Mat4::IDENTITY }
    }

    #[test]
    fn a_shape_is_held_over_its_parts_frames_only() {
        let (a, b) = ("box(1, 1, 1)", "box(2, 1, 1)");
        // parts: a#0 on frames 0-1, a#1 on frame 0, b#0 on frames 1-2
        let all = vec![vec![mesh(a), mesh(a)], vec![mesh(a), mesh(b)], vec![mesh(b)]];
        let parts = spans_of(&all, &["mesh"]);
        let mut held = Held::new(&parts, all.len());
        held.at(&parts, 0);
        assert!(Arc::ptr_eq(held.solids[0].as_ref().unwrap(), held.solids[1].as_ref().unwrap()), "one shape's parts share its solid");
        held.after(0, |_| false);
        assert!(held.solids[0].is_some() && held.solids[1].is_none(), "a#1's last frame was 0");
        held.at(&parts, 1);
        held.after(1, |_| false);
        assert!(held.solids[0].is_none() && held.solids[2].is_some());
        assert!(held.shared["mesh:box(1, 1, 1)"].as_ref().is_ok_and(|w| w.upgrade().is_none()), "let go once no part holds it");
        held.at(&parts, 2);
        held.after(2, |i| i == 2);
        assert!(held.solids[2].is_some(), "kept for an overlap still to be said");
    }
}
