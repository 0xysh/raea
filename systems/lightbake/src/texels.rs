//! Each texel's place in the world (docs/bake.feature BK2). BK1's layout gives every texel the world point under its
//! centre (or, in a chart's edge band, the nearest point of the chart), its triangle's normal and the triangle, so the
//! texels aren't rasterized a second time. Here each point gets its ray origin, moved off its surface by an offset
//! from the model's size so the trace never hits the surface it starts on, and the texels inside other geometry (a
//! table leg's foot in the floor, a wall's end inside the next wall) are found by rays and marked: they neither go
//! black nor leak light into the bake, and BK4 fills them from their chart's neighbours.

use parry3d::glamx::Vec3;
use parry3d::query::{Ray, RayCast};
use parry3d::shape::TriMesh;
use rayon::prelude::*;
use serde_json::{Value, json};

use raea_sys_atlas::rays::{hemi, sample};
pub use raea_sys_atlas::{Layout, Texel};

/// Rays each texel casts for the inside test: cosine-weighted, the atlas's rotated R2 sequence per texel.
const RAYS: u32 = 16;

pub struct Texels {
    /// BK1's layout: the model with its TEXCOORD_1, the atlas's size and density, the texels' surface points
    pub layout: Layout,
    /// each texel's ray origin: its surface point moved `offset` along its normal
    pub origin: Vec<[f32; 3]>,
    /// the texels inside other geometry (their rays meet back faces), left for BK4 to fill
    pub inside: Vec<bool>,
    /// metres off the surface: 1e-5 of the model's size or of its farthest coordinate, at least 10 microns
    pub offset: f32,
    /// the model's triangles in world space, for the trace (BK3)
    pub mesh: TriMesh,
}

/// The request of a bake that only lights: texels a metre and the atlas's longest side. A model that is also worn
/// passes its wear bake's request instead, so its lightmap lies over that bake's atlas.
pub fn request(texels: f32, max_side: u32) -> Value {
    json!({ "env": { "density": texels, "max_side": max_side } })
}

/// Every texel of the model's atlas in the world: its origin and whether it's inside other geometry.
pub fn texels(glb: &[u8], req: &Value) -> Result<Texels, String> {
    let layout = raea_sys_atlas::layout(glb, req)?;
    let mesh = world_mesh(&layout.glb)?;
    let (mut lo, mut hi, mut far) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN), 0.0f32);
    for &v in mesh.vertices() {
        (lo, hi, far) = (lo.min(v), hi.max(v), far.max(v.abs().max_element()));
    }
    let offset = (1e-5 * (hi - lo).length().max(far)).max(1e-5);
    let origin: Vec<[f32; 3]> = layout.texels.iter().map(|t| (Vec3::from(t.p) + Vec3::from(t.n) * offset).to_array()).collect();
    let inside = layout.texels.par_iter().zip(&origin).enumerate().map(|(i, (t, &o))| inside(&mesh, Vec3::from(o), Vec3::from(t.n), i as u32)).collect();
    Ok(Texels { layout, origin, inside, offset, mesh })
}

/// Whether a texel is inside other geometry: of its rays that meet anything, at least half meet a back face and more
/// meet back faces than front ones. A foot inside a closed floor slab meets back faces every way; one under a floor
/// that is a single plane meets its underside upwards and nothing downwards, and is inside too.
fn inside(mesh: &TriMesh, o: Vec3, n: Vec3, seed: u32) -> bool {
    let (mut back, mut front) = (0, 0);
    for j in 0..RAYS {
        let d = hemi(n, sample(seed, j));
        if let Some(h) = mesh.cast_local_ray_and_get_normal(&Ray::new(o, d), f32::MAX, false) {
            let [a, b, c] = mesh.indices()[h.subshape as usize].map(|v| mesh.vertices()[v as usize]);
            if (b - a).cross(c - a).dot(d) > 0.0 { back += 1 } else { front += 1 }
        }
    }
    back * 2 >= RAYS && back > front
}

/// The layout's model as world triangles: every triangle primitive of the default scene's nodes, each node's
/// transform applied down its children.
fn world_mesh(glb: &[u8]) -> Result<TriMesh, String> {
    let g = gltf::Gltf::from_slice(glb).map_err(|e| format!("the layout's model: {e}"))?;
    let blob = g.blob.as_deref();
    let scene = g.default_scene().or_else(|| g.scenes().next()).ok_or("the layout's model has no scene")?;
    const ID: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    let mut stack: Vec<(gltf::Node, [[f32; 4]; 4])> = scene.nodes().map(|n| (n, ID)).collect();
    let (mut pos, mut tri) = (Vec::new(), Vec::new());
    while let Some((node, parent)) = stack.pop() {
        let l = node.transform().matrix();
        let m: [[f32; 4]; 4] = std::array::from_fn(|c| std::array::from_fn(|r| (0..4).map(|k| parent[k][r] * l[c][k]).sum()));
        for prim in node.mesh().iter().flat_map(|m| m.primitives()).filter(|p| p.mode() == gltf::mesh::Mode::Triangles) {
            let r = prim.reader(|b| if b.index() == 0 { blob } else { None });
            let base = pos.len() as u32;
            pos.extend(r.read_positions().ok_or("a primitive with no positions")?.map(|p| Vec3::from(std::array::from_fn(|i| m[0][i] * p[0] + m[1][i] * p[1] + m[2][i] * p[2] + m[3][i]))));
            let ix: Vec<u32> = match r.read_indices() {
                Some(i) => i.into_u32().collect(),
                None => (0..pos.len() as u32 - base).collect(),
            };
            tri.extend(ix.chunks_exact(3).map(|t| [base + t[0], base + t[1], base + t[2]]));
        }
        stack.extend(node.children().map(|c| (c, m)));
    }
    TriMesh::new(pos, tri).map_err(|e| format!("the layout's triangles: {e:?}"))
}

#[cfg(test)]
mod tests;
