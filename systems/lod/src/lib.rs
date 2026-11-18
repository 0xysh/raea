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
/// either side). `size` is the part's size across in metres, 0 to take it from its meshes' bounds. `edge` measures the
/// distances to the part's nearest edge rather than its middle: a big part (a city's tile, a terrain chunk) whose
/// middle is far from a camera standing in it. Bevy measures to one point, so each mesh's range is shifted every frame
/// by the gap between that point and the part's nearest edge, exact for the shot's camera (and its shadow passes,
/// which measure from it).
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
    pub edge: bool,
}

impl Default for Detail {
    fn default() -> Self {
        Self { near: 0.0, far: 0.0, min_px: 0.0, max_px: 0.0, fade: 0.1, shadow: true, size: 0.0, edge: false }
    }
}

/// a mesh's shadow left off by its part's Detail, so only what this took away is given back
#[derive(Component)]
struct ShadowLeft;

pub fn install(app: &mut App) {
    app.register_type::<Detail>().add_systems(PostUpdate, apply.after(VisibilitySystems::CalculateBounds).after(bevy::transform::TransformSystems::Propagate).before(VisibilitySystems::CheckVisibility));
    match std::env::var("RAEA_LOD_DEBUG").as_deref() {
        Ok("time") => {
            app.init_resource::<LodTime>();
            app.add_systems(PostUpdate, (stamp.before(apply), timed.after(apply)));
        }
        Ok(_) => {
            app.add_systems(PostUpdate, report.after(VisibilitySystems::CheckVisibility));
        }
        Err(_) => {}
    }
}

/// RAEA_LOD_DEBUG=time: apply's own time and the Details changed this frame, averaged every 100 frames, on stderr
fn stamp(mut t: ResMut<LodTime>) {
    t.0 = Some(std::time::Instant::now());
}

#[derive(Resource, Default)]
struct LodTime(Option<std::time::Instant>, f64, u32, u32);

fn timed(mut t: ResMut<LodTime>, changed: Query<(), Changed<Detail>>, parts: Query<(), With<Detail>>) {
    let Some(at) = t.0 else { return };
    t.1 += at.elapsed().as_secs_f64() * 1000.0;
    t.2 += 1;
    t.3 += changed.iter().count() as u32;
    if t.2 == 100 {
        eprintln!("raea lod time: apply {:.2} ms a frame, {:.0} of {} Details changed a frame", t.1 / 100.0, t.3 as f64 / 100.0, parts.iter().count());
        *t = LodTime::default();
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

/// A range moved out by `by` metres (a mesh's measuring point that far behind its part's nearest edge); a start at 0
/// and an endless end stay as they are.
pub fn shifted(mut r: VisibilityRange, by: f32) -> VisibilityRange {
    if r.start_margin.end > 0.0 {
        r.start_margin = r.start_margin.start + by..r.start_margin.end + by;
    }
    if r.end_margin.end < f32::MAX {
        r.end_margin = r.end_margin.start + by..r.end_margin.end + by;
    }
    r
}

/// how much farther `point` is from `eye` than the nearest point of the box `lo`..`hi` is (with the eye inside the box,
/// all of `point`'s distance: a camera standing in a part is 0 m from it)
pub fn edge_gap(eye: Vec3, point: Vec3, lo: Vec3, hi: Vec3) -> f32 {
    (eye.distance(point) - eye.distance(eye.clamp(lo, hi))).max(0.0)
}

/// how far a part's nearest edge (`d`) is from the bands its range fades over (0 inside one): the eye can move a third
/// of this before a range written for it could draw the part differently (the edge and each mesh's gap move at most
/// as far as the eye)
pub fn slack(d: f32, start: f32, end: f32, fade: f32) -> f32 {
    let h = fade.clamp(0.0, 1.0) * 0.5;
    [start, end].into_iter().filter(|b| *b > 0.0 && b.is_finite()).fold(f32::INFINITY, |s, b| {
        let (lo, hi) = (b * (1.0 - h), b * (1.0 + h));
        s.min(if d < lo { lo - d } else if d > hi { d - hi } else { 0.0 })
    })
}

type MeshRow<'a> = (&'a GlobalTransform, Option<&'a Aabb>, Option<&'a VisibilityRange>, Has<NotShadowCaster>, Has<ShadowLeft>);

/// What `apply` last did for a part: its meshes and their bounds, and the Detail, lens and eye it wrote their ranges for.
/// Bevy rebuilds its whole table of ranges whenever one changes and never reuses a range's slot (65,535 in all), so a
/// range is written only when it can draw differently, and an edge's gap is whole metres.
#[derive(Default)]
struct Held {
    meshes: Vec<Entity>,
    bounds: Option<(Vec3, Vec3)>,
    detail: Option<Detail>,
    focal: Option<f32>,
    eye: Vec3,
    slack: f32,
}

#[allow(clippy::too_many_arguments)]
fn apply(cams: Query<(&Camera, &Projection, &GlobalTransform, Has<bevy::camera::ShadowLodOrigin>), With<Camera3d>>, parts: Query<(Entity, &Detail, Ref<GlobalTransform>)>, added: Query<(), Added<Mesh3d>>, children: Query<&Children>, meshes: Query<MeshRow, With<Mesh3d>>, mut held: Local<std::collections::HashMap<Entity, Held>>, mut commands: Commands) {
    // the shot's camera (raea gives it the shadow LOD origin), else the first active 3D one: a range is one per mesh,
    // so sizes on screen and nearest edges are measured for one camera
    let shot = cams.iter().filter(|(c, ..)| c.is_active).max_by_key(|(.., origin)| *origin);
    let focal = shot.and_then(|(c, p, ..)| match p {
        Projection::Perspective(p) => c.physical_target_size().map(|s| s.y as f32 / (2.0 * (p.fov * 0.5).tan())),
        _ => None,
    });
    let eye = shot.map(|(_, _, at, _)| at.translation());
    // a part's meshes are walked again only when meshes arrived (a glTF loading its children) or its Detail changed
    let loaded = !added.is_empty();
    held.retain(|e, _| parts.contains(*e));
    for (part, d, at) in &parts {
        let h = held.entry(part).or_default();
        let new = h.detail.as_ref() != Some(d);
        let walk = loaded || new || h.meshes.is_empty();
        if walk {
            h.meshes = std::iter::once(part).chain(children.iter_descendants(part)).filter(|e| meshes.contains(*e)).collect();
        }
        if h.meshes.is_empty() {
            continue;
        }
        if walk || at.is_changed() {
            h.bounds = bounds(&h.meshes, &meshes);
        }
        let size = if d.size > 0.0 { d.size } else { h.bounds.map_or(0.0, |(lo, hi)| (hi - lo).length()) };
        let (start, end) = span(d, size, focal);
        let edge = match (d.edge, eye, h.bounds) {
            (true, Some(eye), Some((lo, hi))) => Some((eye, lo, hi)),
            _ => None,
        };
        let moved = edge.is_some_and(|(eye, ..)| eye.distance(h.eye) >= (h.slack / 3.0).max(0.5));
        if !(walk || at.is_changed() || h.focal != focal || moved) {
            continue;
        }
        h.detail = Some(d.clone());
        h.focal = focal;
        if let Some((eye, lo, hi)) = edge {
            h.eye = eye;
            h.slack = slack(eye.distance(eye.clamp(lo, hi)), start, end, d.fade);
        }
        for &e in &h.meshes {
            let Ok((gt, aabb, now, no_shadow, ours)) = meshes.get(e) else { continue };
            // measured from the mesh's origin, unless that lies outside its own bounds (a mesh baked in world
            // coordinates, as the city's tiles are): then from its bounds' centre
            let use_aabb = aabb.is_some_and(|a| a.center.length() > a.half_extents.length());
            let point = match aabb.filter(|_| use_aabb) {
                Some(a) => gt.affine().transform_point3(Vec3::from(a.center)),
                None => gt.translation(),
            };
            let gap = edge.map_or(0.0, |(eye, lo, hi)| edge_gap(eye, point, lo, hi).round());
            let want = shifted(range(start, end, d.fade, use_aabb), gap);
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

/// the meshes' bounds together, in the world; None before any has bounds
fn bounds(found: &[Entity], meshes: &Query<MeshRow, With<Mesh3d>>) -> Option<(Vec3, Vec3)> {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for (gt, aabb, ..) in found.iter().filter_map(|e| meshes.get(*e).ok()) {
        let Some(a) = aabb else { continue };
        let m = gt.affine();
        let c = m.transform_point3(Vec3::from(a.center));
        let h = Vec3::from(m.matrix3.x_axis.abs() * a.half_extents.x + m.matrix3.y_axis.abs() * a.half_extents.y + m.matrix3.z_axis.abs() * a.half_extents.z);
        lo = lo.min(c - h);
        hi = hi.max(c + h);
    }
    (lo.x <= hi.x).then_some((lo, hi))
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

    #[test]
    fn edge_distances_move_the_range_by_the_gap_to_the_nearest_edge() {
        // a 10 m box from 10 m to 20 m out, measured from its middle at 15 m: 5 m behind its nearest edge
        let gap = edge_gap(Vec3::ZERO, Vec3::new(15.0, 0.0, 0.0), Vec3::new(10.0, -1.0, -1.0), Vec3::new(20.0, 1.0, 1.0));
        assert!((gap - 5.0).abs() < 1e-4);
        // drawn to 12 m from the edge: to 17 m from the middle, so drawn here (middle at 15 m, edge at 10 m)
        let r = shifted(range(0.0, 12.0, 0.0, true), gap);
        assert_eq!(r.start_margin, 0.0..0.0);
        assert!(r.is_visible_at_all(15.0) && !r.is_visible_at_all(17.5));
        // standing inside it, 3 m from its middle: 0 m from it, so the whole 3 m is the gap; an endless end stays endless
        assert_eq!(edge_gap(Vec3::new(12.0, 0.0, 0.0), Vec3::new(15.0, 0.0, 0.0), Vec3::new(10.0, -1.0, -1.0), Vec3::new(20.0, 1.0, 1.0)), 3.0);
        assert_eq!(shifted(range(30.0, f32::INFINITY, 0.0, true), 5.0).end_margin, f32::MAX..f32::MAX);
    }

    #[test]
    fn slack_is_the_way_to_the_nearest_fading_band() {
        // drawn 120 m to 700 m, fading 10%: bands 114-126 m and 665-735 m
        assert!((slack(300.0, 120.0, 700.0, 0.1) - 174.0).abs() < 1e-3);
        assert_eq!(slack(120.0, 120.0, 700.0, 0.1), 0.0);
        // a start at 0 and an endless end have no band
        assert!((slack(50.0, 0.0, f32::INFINITY, 0.1)).is_infinite());
        assert!((slack(50.0, 0.0, 120.0, 0.1) - 64.0).abs() < 1e-3);
    }
}
