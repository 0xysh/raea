//! LightBench's procedural scene (docs/lightbench.feature, Background): a ground, a detailed torus knot, a wall that
//! occludes it from some lights, and a pillar outside the view with a light behind it, so its shadow falls into the
//! frame from a caster the camera can't see. Every triangle goes into the BVH; the meshes are drawn by Bevy.

use crate::{bvh, Opts};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
    render::{extract_resource::ExtractResource, RenderApp},
};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

/// A light as the shader reads it: where, the distance past which it adds nothing, its colour times its power, and
/// the radius inside which its falloff stops growing.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, Debug)]
pub struct GpuLight {
    pub pos: [f32; 3],
    pub range: f32,
    pub power: [f32; 3],
    pub radius: f32,
}

/// The BVH, made once at startup and handed to the render world.
#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct SceneGpu {
    pub nodes: Arc<Vec<bvh::Node>>,
    pub tris: Arc<Vec<bvh::Tri>>,
}

/// Where the off-screen pillar stands and the light behind it.
pub const PILLAR: Vec3 = Vec3::new(4.6, 2.0, 3.0);
const PILLAR_LIGHT: Vec3 = Vec3::new(6.2, 2.4, 4.4);

pub fn spawn(mut commands: Commands, opts: Res<Opts>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let mut tris: Vec<[Vec3; 3]> = Vec::new();
    if opts.scene == 1 {
        crate::field::spawn(&mut commands, &opts, &mut meshes, &mut mats, &mut tris);
    }
    let mut put = |mesh: Mesh, at: Transform, color: Color, tris: &mut Vec<[Vec3; 3]>| {
        tris.extend(triangles(&mesh, &at));
        let material = mats.add(StandardMaterial { base_color: color, perceptual_roughness: 0.6, ..default() });
        commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material), at));
    };
    put(Plane3d::default().mesh().size(160.0, 160.0).build(), Transform::IDENTITY, Color::srgb(0.62, 0.62, 0.6), &mut tris);
    if opts.scene == 0 {
        put(knot(2, 3, opts.detail, opts.detail / 32, 0.33), Transform::from_xyz(0.0, 1.75, 0.0).with_scale(Vec3::splat(0.5)), Color::srgb(0.85, 0.42, 0.18), &mut tris);
        put(Cuboid::new(0.2, 2.5, 3.0).mesh().build(), Transform::from_xyz(-2.5, 1.25, 0.0), Color::srgb(0.35, 0.42, 0.55), &mut tris);
    }
    if opts.pillar {
        put(Cuboid::new(0.4, 4.0, 0.4).mesh().build(), Transform::from_translation(PILLAR), Color::srgb(0.9, 0.9, 0.88), &mut tris);
    }
    let t0 = std::time::Instant::now();
    let built = bvh::build(&tris);
    info!("lightbench: {} triangles, {} BVH nodes, depth {}, built in {:.0} ms", tris.len(), built.nodes.len(), built.depth(), t0.elapsed().as_secs_f64() * 1000.0);
    commands.insert_resource(SceneGpu { nodes: Arc::new(built.nodes), tris: Arc::new(built.tris) });
}

/// The lights at time `t` (seconds): the pillar's light first, then `n - 1` on a ring that turns, each a different
/// colour, their power split so the frame's brightness doesn't change with how many there are.
pub fn lights(n: u32, t: f32) -> Vec<GpuLight> {
    let mut out = vec![GpuLight { pos: PILLAR_LIGHT.into(), range: 14.0, power: [30.0, 28.0, 24.0], radius: 0.05 }];
    let ring = n.saturating_sub(1).max(1);
    for i in 0..n.saturating_sub(1) {
        let f = i as f32 / ring as f32;
        let a = f * std::f32::consts::TAU + t * 0.3;
        let h = 1.3 + 1.6 * (0.5 + 0.5 * (f * 17.0 + t * 0.7).sin());
        let c = Color::hsl(f * 360.0, 0.7, 0.6).to_linear();
        let lum = 0.2126 * c.red + 0.7152 * c.green + 0.0722 * c.blue;
        let p = 60.0 / ring as f32 / lum.max(1e-3);
        out.push(GpuLight { pos: [3.6 * a.cos(), h, 3.6 * a.sin()], range: 9.0, power: [c.red * p, c.green * p, c.blue * p], radius: 0.05 });
    }
    out
}

/// A (p, q) torus knot `seg` segments long and `ring` around, its tube `r` thick, in the xy plane.
fn knot(p: u32, q: u32, seg: u32, ring: u32, r: f32) -> Mesh {
    let (pos, nor, idx) = knot_data(p, q, seg, ring, r);
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
        .with_inserted_indices(Indices::U32(idx))
}

/// The knot's positions, normals and triangles: a closed mesh, every vertex shared.
pub fn knot_data(p: u32, q: u32, seg: u32, ring: u32, r: f32) -> (Vec<Vec3>, Vec<Vec3>, Vec<u32>) {
    let (p, q) = (p as f32, q as f32);
    let c = |t: f32| Vec3::new((2.0 + (q * t).cos()) * (p * t).cos(), (2.0 + (q * t).cos()) * (p * t).sin(), -(q * t).sin());
    let (mut pos, mut nor, mut idx) = (Vec::new(), Vec::new(), Vec::new());
    let e = 1e-3;
    for i in 0..seg {
        let t = i as f32 / seg as f32 * std::f32::consts::TAU;
        let (c0, cp, cm) = (c(t), c(t + e), c(t - e));
        let tan = (cp - cm).normalize();
        let b = (cp - cm).cross(cp - 2.0 * c0 + cm).normalize();
        let n = b.cross(tan);
        for j in 0..ring {
            let a = j as f32 / ring as f32 * std::f32::consts::TAU;
            let dir = n * a.cos() + b * a.sin();
            pos.push(c0 + dir * r);
            nor.push(dir);
            let (i1, j1) = ((i + 1) % seg, (j + 1) % ring);
            let (v00, v10, v01, v11) = (i * ring + j, i1 * ring + j, i * ring + j1, i1 * ring + j1);
            idx.extend([v00, v10, v11, v00, v11, v01]);
        }
    }
    (pos, nor, idx)
}

/// A mesh's triangles in world space.
fn triangles(mesh: &Mesh, at: &Transform) -> Vec<[Vec3; 3]> {
    let Some(VertexAttributeValues::Float32x3(p)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return vec![] };
    let m = at.to_matrix();
    let w: Vec<Vec3> = p.iter().map(|v| m.transform_point3(Vec3::from(*v))).collect();
    let idx: Vec<usize> = match mesh.indices() {
        Some(i) => i.iter().collect(),
        None => (0..w.len()).collect(),
    };
    idx.chunks_exact(3).map(|t| [w[t[0]], w[t[1]], w[t[2]]]).collect()
}
