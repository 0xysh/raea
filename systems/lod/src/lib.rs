//! Detail by distance and size on screen (docs/realism.feature LT21). A part (a Model, a Mesh3d: anything raea draws)
//! carries a `Detail`, from a TSX `detail` prop through <BevyComponent>, saying where it is drawn. Each frame it becomes
//! Bevy's VisibilityRange on every mesh under the part (a glTF's meshes are its children, and a range acts only on the
//! entity it's on), so a level cross-fades into the next with Bevy's dither, and past its last range a part isn't drawn
//! at all, in the camera's view and in the shadow passes (they measure from the camera: raea sets ShadowLodOrigin).
//! A size on screen becomes a distance for the active 3D camera from the part's size across (its meshes' bounds), the
//! camera's vertical field of view and its target's height in pixels, so a level looks the same at any output size.
//! `shadow: false` keeps a level out of the shadow maps, for a coarser level to cast its shadow.

use bevy::{
    camera::{
        primitives::Aabb,
        visibility::{VisibilityRange, VisibilitySystems},
    },
    light::NotShadowCaster,
    prelude::*,
};

/// Where a part is drawn. Distances are metres from the camera, 0 for none; sizes on screen are the part's size across
/// in pixels, 0 for none. Drawn from the farther of `near` and the distance where it shrinks to `max_px`, to the nearer
/// of `far` and the distance where it shrinks to `min_px`. Each edge cross-fades over `fade` of its distance (0.1: 5%
/// either side). `size` is the part's size across in metres, 0 to take it from its meshes' bounds.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
pub struct Detail {
    pub near: f32,
    pub far: f32,
    pub min_px: f32,
    pub max_px: f32,
    pub fade: f32,
    pub shadow: bool,
    pub size: f32,
}

impl Default for Detail {
    fn default() -> Self {
        Self { near: 0.0, far: 0.0, min_px: 0.0, max_px: 0.0, fade: 0.1, shadow: true, size: 0.0 }
    }
}

/// a mesh's shadow left off by its part's Detail, so only what this took away is given back
#[derive(Component)]
struct ShadowLeft;

pub fn install(app: &mut App) {
    app.register_type::<Detail>().add_systems(PostUpdate, apply.after(VisibilitySystems::CalculateBounds).after(bevy::transform::TransformSystems::Propagate).before(VisibilitySystems::CheckVisibility));
    if std::env::var_os("RAEA_LOD_DEBUG").is_some() {
        app.add_systems(PostUpdate, report.after(VisibilitySystems::CheckVisibility));
    }
}

/// RAEA_LOD_DEBUG=1: each range's distances and whether Bevy kept it in range of each camera, on stderr
fn report(views: Query<(Entity, Option<&Camera>, &GlobalTransform), Or<(With<Camera>, With<bevy::camera::ShadowLodOrigin>)>>, ranged: Query<(Entity, &VisibilityRange, &GlobalTransform, &ViewVisibility)>, ranges: Res<bevy::camera::visibility::VisibleEntityRanges>) {
    let cams: Vec<_> = views.iter().collect();
    eprintln!("raea lod: {} views ({} active cameras)", cams.len(), cams.iter().filter(|(_, c, _)| c.is_some_and(|c| c.is_active)).count());
    for (e, r, gt, vv) in &ranged {
        let seen: Vec<String> = cams.iter().map(|(v, c, at)| format!("{v}{}:{:.1}m:{}", if c.is_some_and(|c| c.is_active) { "*" } else { "" }, at.translation().distance(gt.translation()), ranges.entity_is_in_range_of_view(e, *v))).collect();
        eprintln!("raea lod: {e} {:?}..{:?} shown={} {}", r.start_margin, r.end_margin, vv.get(), seen.join(" "));
    }
}

/// The distances a Detail is drawn between, for a camera `focal` pixels long (its target's height over twice the
/// tangent of half its field of view; None for an orthographic camera, where sizes on screen don't change with
/// distance and only the metres count). The end is infinite when nothing limits it.
pub fn span(d: &Detail, size: f32, focal: Option<f32>) -> (f32, f32) {
    let at = |px: f32| focal.filter(|_| px > 0.0 && size > 0.0).map(|f| size * f / px);
    let start = at(d.max_px).map_or(d.near, |s| s.max(d.near));
    let far = if d.far > 0.0 { d.far } else { f32::INFINITY };
    let end = at(d.min_px).map_or(far, |e| e.min(far));
    (start, end.max(start))
}

/// Bevy's range for a span: each edge cross-faded over `fade` of its distance, centred on it, so one level's end and
/// the next one's start fade across each other.
pub fn range(start: f32, end: f32, fade: f32, use_aabb: bool) -> VisibilityRange {
    let h = fade.clamp(0.0, 1.0) * 0.5;
    let e = if end.is_finite() { end * (1.0 - h)..end * (1.0 + h) } else { f32::MAX..f32::MAX };
    let s = if start > 0.0 { start * (1.0 - h)..start * (1.0 + h) } else { 0.0..0.0 };
    VisibilityRange { start_margin: s.start.min(e.start)..s.end.min(e.start), end_margin: e, use_aabb }
}

type MeshRow<'a> = (&'a GlobalTransform, Option<&'a Aabb>, Option<&'a VisibilityRange>, Has<NotShadowCaster>, Has<ShadowLeft>);

fn apply(cams: Query<(&Camera, &Projection, Has<bevy::camera::ShadowLodOrigin>), With<Camera3d>>, parts: Query<(Entity, &Detail)>, children: Query<&Children>, meshes: Query<MeshRow, With<Mesh3d>>, mut commands: Commands) {
    // the shot's camera (raea gives it the shadow LOD origin), else the first active 3D one: a range is one per mesh,
    // so sizes on screen are measured for one camera
    let shot = cams.iter().filter(|(c, ..)| c.is_active).max_by_key(|(.., origin)| *origin);
    let focal = shot.and_then(|(c, p, _)| match p {
        Projection::Perspective(p) => c.physical_target_size().map(|s| s.y as f32 / (2.0 * (p.fov * 0.5).tan())),
        _ => None,
    });
    for (part, d) in &parts {
        let found: Vec<Entity> = std::iter::once(part).chain(children.iter_descendants(part)).filter(|e| meshes.contains(*e)).collect();
        if found.is_empty() {
            continue;
        }
        let size = if d.size > 0.0 { d.size } else { across(&found, &meshes) };
        let (start, end) = span(d, size, focal);
        for e in found {
            let Ok((_, aabb, now, no_shadow, ours)) = meshes.get(e) else { continue };
            // measured from the mesh's origin, unless that lies outside its own bounds (a mesh baked in world
            // coordinates, as the city's tiles are): then from its bounds' centre
            let use_aabb = aabb.is_some_and(|a| a.center.length() > a.half_extents.length());
            let want = range(start, end, d.fade, use_aabb);
            if now != Some(&want) {
                commands.entity(e).insert(want);
            }
            match (d.shadow, no_shadow, ours) {
                (false, false, _) => {
                    commands.entity(e).insert((NotShadowCaster, ShadowLeft));
                }
                (true, true, true) => {
                    commands.entity(e).remove::<(NotShadowCaster, ShadowLeft)>();
                }
                _ => {}
            }
        }
    }
}

/// the meshes' bounds together, in the world, as a size across (their box's diagonal); 0 before any has bounds
fn across(found: &[Entity], meshes: &Query<MeshRow, With<Mesh3d>>) -> f32 {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for (gt, aabb, ..) in found.iter().filter_map(|e| meshes.get(*e).ok()) {
        let Some(a) = aabb else { continue };
        let m = gt.affine();
        let c = m.transform_point3(Vec3::from(a.center));
        let h = Vec3::from(m.matrix3.x_axis.abs() * a.half_extents.x + m.matrix3.y_axis.abs() * a.half_extents.y + m.matrix3.z_axis.abs() * a.half_extents.z);
        lo = lo.min(c - h);
        hi = hi.max(c + h);
    }
    if lo.x > hi.x { 0.0 } else { (hi - lo).length() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metres_alone_are_kept() {
        let d = Detail { near: 10.0, far: 120.0, ..default() };
        assert_eq!(span(&d, 5.0, Some(1000.0)), (10.0, 120.0));
    }

    #[test]
    fn a_size_on_screen_becomes_a_distance() {
        // 4 m across, a 1000 px focal length: 40 px at 100 m, 4 px at 1000 m
        let d = Detail { min_px: 4.0, max_px: 40.0, ..default() };
        let (s, e) = span(&d, 4.0, Some(1000.0));
        assert!((s - 100.0).abs() < 1e-3 && (e - 1000.0).abs() < 1e-3);
        // an orthographic camera keeps only the metres
        assert_eq!(span(&d, 4.0, None), (0.0, f32::INFINITY));
    }

    #[test]
    fn the_nearer_end_and_the_farther_start_win() {
        let d = Detail { near: 150.0, far: 500.0, min_px: 4.0, max_px: 40.0, ..default() };
        assert_eq!(span(&d, 4.0, Some(1000.0)), (150.0, 500.0));
    }

    #[test]
    fn one_levels_end_fades_across_the_next_ones_start() {
        let a = range(0.0, 120.0, 0.1, false);
        let b = range(120.0, 700.0, 0.1, false);
        assert_eq!(a.end_margin, b.start_margin);
        assert!((a.end_margin.start - 114.0).abs() < 1e-3 && (a.end_margin.end - 126.0).abs() < 1e-3);
        assert_eq!(range(0.0, f32::INFINITY, 0.1, false).end_margin, f32::MAX..f32::MAX);
    }
}
