//! raea GI's world radiance cache, its first form (RAEA_GI=1; docs/realism.feature LT2): what a frame of probe rays
//! costs on this GPU, and the light they give through Bevy's irradiance volume. A grid of probes around the camera
//! (RAEA_GI_GRID, 32x16x32 by default; RAEA_GI_SPACING metres apart, 2) is refreshed by a ray budget a frame
//! (RAEA_GI_RAYS, 131072): 64 rays a probe, so the budget sets how many probes a frame (gi.wesl). The grid moves with
//! the camera a probe at a time, each probe kept by its place in the world, so what the probes still covered have
//! learned stays; a probe new to the grid traces RAEA_GI_WARM rounds (16) when first reached. When the sun or the sky
//! changes, every mean starts again and every probe traces the warm rounds that frame, so a still is lit from its
//! first frame. Those frames aren't timed.
//! RAEA_GI_STATS=FILE: the timed frames' GPU milliseconds (the pass's own timestamps), written there once a second.
//! RAEA_GI_SHOW=volume: the probes light the scene through Bevy's irradiance volume (see `install`). Unset, nothing
//! changes.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use bevy::{
    asset::RenderAssetUsages,
    camera::Camera3d,
    image::Image,
    light::{EnvironmentMapLight, IrradianceVolume, LightProbe},
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::{
            binding_types::{acceleration_structure, sampler, storage_buffer_sized, texture_cube, texture_storage_3d, uniform_buffer},
            *,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{FallbackImage, GpuImage},
        Render, RenderApp, RenderStartup, RenderSystems,
    },
};

/// rays a probe each round (gi.wesl's RAYS: a workgroup a probe, a ray a thread)
const RAYS: u32 = 64;

fn env<T: std::str::FromStr>(name: &str, or: T) -> T {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(or)
}

fn grid_res() -> UVec3 {
    let v: Vec<u32> = std::env::var("RAEA_GI_GRID").unwrap_or_default().split('x').filter_map(|s| s.parse().ok()).collect();
    if let [x, y, z] = v[..] { UVec3::new(x, y, z).max(UVec3::ONE) } else { UVec3::new(32, 16, 32) }
}

pub fn install(app: &mut App) {
    if std::env::var("RAEA_GI").as_deref() != Ok("1") {
        return;
    }
    bevy::asset::embedded_asset!(app, "gi.wesl");
    crate::install(app);
    let res = grid_res();
    let mut voxels = Image::new_fill(
        Extent3d { width: res.x, height: 2 * res.y, depth_or_array_layers: 3 * res.z },
        TextureDimension::D3,
        &[0u8; 8],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    voxels.texture_descriptor.usage |= TextureUsages::STORAGE_BINDING;
    let voxels = app.world_mut().resource_mut::<Assets<Image>>().add(voxels);
    let spacing = env("RAEA_GI_SPACING", 2.0f32);
    // Shown through Bevy's irradiance volume only when asked: it takes one more sampler in every material's fragment
    // stage, and a hooked material over StandardMaterial already has Metal's 16 (Bevy's view 9, StandardMaterial 6, the
    // hook's own), so on the Mac the pipeline fails to build (11-17, Berlin). The deferred lighting pass (D4) has room.
    if std::env::var("RAEA_GI_SHOW").as_deref() == Ok("volume") {
        app.world_mut().spawn((Name::new("raea gi"), LightProbe::default(), IrradianceVolume { voxels: voxels.clone(), intensity: 1.0, affects_lightmapped_meshes: false }, Transform::from_scale(res.as_vec3() * spacing)));
    } else {
        eprintln!("raea gi: effect dropped (gi-not-shown): the probes are traced and timed but light nothing; Bevy's irradiance volume would take a 17th fragment sampler on Metal (RAEA_GI_SHOW=volume tries it)");
    }
    app.insert_resource(Grid { center: Vec3::ZERO, spacing, res, voxels, sky: None, sun: Vec4::ZERO, sun_light: Vec3::ZERO })
        .add_plugins(ExtractResourcePlugin::<Grid>::default())
        .add_systems(PostUpdate, follow.after(bevy::transform::TransformSystems::Propagate));
    let Some(render) = app.get_sub_app_mut(RenderApp) else { return };
    render.init_resource::<State>().add_systems(RenderStartup, init).add_systems(Render, update.in_set(RenderSystems::PrepareBindGroups));
}

/// The grid this frame: where it stands, its voxels, the sky and the sun its rays see.
#[derive(Resource, Clone, ExtractResource)]
#[extract_app(bevy::render::RenderApp)]
struct Grid {
    center: Vec3,
    spacing: f32,
    res: UVec3,
    voxels: Handle<Image>,
    /// the camera's environment: its specular and diffuse cubemaps, intensity, and the turn into their frame
    sky: Option<(AssetId<Image>, AssetId<Image>, f32, Quat)>,
    /// toward the sun, and 1 when a sky is bound
    sun: Vec4,
    /// the sun's illuminance times its linear colour (lux)
    sun_light: Vec3,
}

/// the volume kept around the active 3D camera, a probe's spacing at a time (a quarter of the grid's height below the
/// eye, three quarters above: the street and the fronts, not the ground under it); the camera's sky and the sun
fn follow(
    cams: Query<(&Camera, &GlobalTransform, Option<&EnvironmentMapLight>), With<Camera3d>>,
    lights: Query<(&DirectionalLight, &GlobalTransform)>,
    mut volume: Query<&mut Transform, With<IrradianceVolume>>,
    mut grid: ResMut<Grid>,
) {
    let Some((_, cam, env)) = cams.iter().find(|(c, _, _)| c.is_active) else { return };
    let extent = grid.res.as_vec3() * grid.spacing;
    let eye = cam.translation() + Vec3::Y * extent.y * 0.25;
    let center = (eye / grid.spacing).round() * grid.spacing;
    if grid.center != center {
        grid.center = center;
    }
    for mut t in &mut volume {
        if t.translation != center {
            t.translation = center;
        }
    }
    let sky = env.map(|e| (e.specular_map.id(), e.diffuse_map.id(), e.intensity, e.rotation.inverse()));
    if grid.sky != sky {
        grid.sky = sky;
    }
    let (dir, light) = lights
        .iter()
        .find(|(l, _)| l.shadow_maps_enabled)
        .or_else(|| lights.iter().next())
        .map(|(l, t)| (t.back().as_vec3(), Vec3::from_slice(&l.color.to_linear().to_f32_array()[..3]) * l.illuminance))
        .unwrap_or((Vec3::Y, Vec3::ZERO));
    let sun = dir.extend(if sky.is_some() { 1.0 } else { 0.0 });
    if grid.sun != sun || grid.sun_light != light {
        grid.sun = sun;
        grid.sun_light = light;
    }
}

#[derive(ShaderType, Clone, Copy)]
struct Params {
    grid: Vec4,
    res: UVec4,
    /// the first probe, rounds each, the frame's salt
    run: UVec4,
    sun_dir: Vec4,
    sun: Vec4,
    sky_turn: Vec4,
}

#[derive(Resource)]
struct Pipeline {
    layout: BindGroupLayoutDescriptor,
    /// the probes' rays (gi.wesl `main`), and their cubes into the volume (`resolve`)
    id: CachedComputePipelineId,
    resolve: CachedComputePipelineId,
    sampler: Sampler,
}

fn init(mut commands: Commands, assets: Res<AssetServer>, cache: Res<PipelineCache>, device: Res<RenderDevice>) {
    let layout = BindGroupLayoutDescriptor::new(
        "raea_gi_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<Params>(false),
                acceleration_structure(),
                texture_cube(TextureSampleType::Float { filterable: true }),
                texture_cube(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                storage_buffer_sized(false, None),
                texture_storage_3d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                storage_buffer_sized(false, None),
            ),
        ),
    );
    let shader = assets.load("embedded://raea_sys_rt/gi.wesl");
    let [id, resolve] = ["main", "resolve"].map(|entry| {
        cache.queue_compute_pipeline(ComputePipelineDescriptor { label: Some(format!("raea_gi_{entry}").into()), layout: vec![layout.clone()], shader: shader.clone(), entry_point: Some(entry.into()), ..default() })
    });
    let sampler = device.create_sampler(&SamplerDescriptor { label: Some("raea gi sky"), mag_filter: FilterMode::Linear, min_filter: FilterMode::Linear, ..default() });
    commands.insert_resource(Pipeline { layout, id, resolve, sampler });
}

/// The cache between frames: its means and which probe each slot holds (gi.wesl's `cubes` and `tags`), where the next
/// frame's probes start, what it was last lit by, and its timer.
#[derive(Resource, Default)]
struct State {
    cubes: Option<(Buffer, Buffer, u32)>,
    frame: u32,
    key: Option<[u32; 9]>,
    timer: Option<Timer>,
    seen: Vec<f64>,
    written: Option<std::time::Instant>,
}

struct Timer {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    read: wgpu::Buffer,
    period: f64,
    /// a frame's timestamps on their way back
    pending: Option<Arc<AtomicBool>>,
}

/// what the grid is lit from: the sun and the sky (and its size); any change starts every mean again. Not its place:
/// a probe is kept by where it is in the world, so the grid moving starts only the probes new to it (gi.wesl `tags`).
fn key(g: &Grid) -> [u32; 9] {
    let (s, i) = g.sky.map(|(s, _, i, _)| (s, i)).unwrap_or((AssetId::default(), 0.0));
    let h = { use std::hash::{Hash, Hasher}; let mut h = std::collections::hash_map::DefaultHasher::new(); s.hash(&mut h); h.finish() as u32 };
    let f = |v: f32| v.to_bits();
    [f(g.sun.x), f(g.sun.y), f(g.sun.z), f(g.sun_light.x), f(g.sun_light.y), f(g.sun_light.z), h, f(i), g.res.x ^ (g.res.y << 10) ^ (g.res.z << 20)]
}

#[allow(clippy::too_many_arguments)]
fn update(
    grid: Option<Res<Grid>>,
    scene: Option<Res<crate::Scene>>,
    images: Res<RenderAssets<GpuImage>>,
    fallback: Res<FallbackImage>,
    p: Option<Res<Pipeline>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut state: ResMut<State>,
) {
    let (Some(grid), Some(scene), Some(p)) = (grid, scene, p) else { return };
    let (Some(pipeline), Some(resolve)) = (cache.get_compute_pipeline(p.id), cache.get_compute_pipeline(p.resolve)) else { return };
    let Some(voxels) = images.get(grid.voxels.id()) else { return };
    let (sky, sky_diffuse) = match grid.sky {
        Some((s, d, _, _)) => match (images.get(s), images.get(d)) {
            (Some(s), Some(d)) => (&s.texture_view, &d.texture_view),
            _ => return,
        },
        None => (&fallback.cube.texture_view, &fallback.cube.texture_view),
    };
    let total = grid.res.x * grid.res.y * grid.res.z;
    if state.cubes.as_ref().is_none_or(|(_, _, n)| *n != total) {
        let buffer = |label: &'static str, faces: u64| device.create_buffer(&BufferDescriptor { label: Some(label), size: total as u64 * faces * 16, usage: BufferUsages::STORAGE | BufferUsages::COPY_DST, mapped_at_creation: false });
        state.cubes = Some((buffer("raea gi cubes", 6), buffer("raea gi tags", 1), total));
        state.key = None;
    }
    collect(&mut state);
    let k = key(&grid);
    let fresh = state.key != Some(k);
    state.key = Some(k);
    let budget = env("RAEA_GI_RAYS", 131072u32).max(RAYS);
    let warm = env("RAEA_GI_WARM", 16u32).max(1);
    let (start, probes, rounds) = if fresh { (0, total, warm) } else { ((state.frame.wrapping_mul(budget / RAYS)) % total, (budget / RAYS).min(total), 1) };
    state.frame = state.frame.wrapping_add(1);
    let (turn, intensity) = grid.sky.map(|(_, _, i, q)| (Vec4::from(q), i)).unwrap_or((Vec4::W, 0.0));
    let params = Params {
        grid: grid.center.extend(grid.spacing),
        res: grid.res.extend(total),
        run: UVec4::new(start, rounds, state.frame, warm),
        sun_dir: grid.sun,
        sun: grid.sun_light.extend(intensity),
        sky_turn: turn,
    };
    let mut uniform = UniformBuffer::from(params);
    uniform.write_buffer(&device, &queue);
    if state.timer.is_none() && device.features().contains(WgpuFeatures::TIMESTAMP_QUERY) {
        state.timer = Some(timer(&device, &queue));
    }
    let (cubes, tags, _) = state.cubes.as_ref().unwrap();
    let group = device.create_bind_group(
        "raea_gi",
        &cache.get_bind_group_layout(&p.layout),
        &BindGroupEntries::sequential((uniform.binding().unwrap(), scene.0.as_binding(), sky, sky_diffuse, &p.sampler, cubes.as_entire_binding(), &voxels.texture_view, tags.as_entire_binding())),
    );
    // a steady frame is timed when the last one's times are in
    let timed = !fresh && state.timer.as_ref().is_some_and(|t| t.pending.is_none());
    let mut enc = device.create_command_encoder(&CommandEncoderDescriptor { label: Some("raea gi") });
    if fresh {
        enc.clear_buffer(cubes, 0, None);
        enc.clear_buffer(tags, 0, None);
    }
    {
        let writes = state.timer.as_ref().filter(|_| timed).map(|t| wgpu::ComputePassTimestampWrites { query_set: &t.set, beginning_of_pass_write_index: Some(0), end_of_pass_write_index: Some(1) });
        let mut pass = enc.begin_compute_pass(&ComputePassDescriptor { label: Some("raea_gi"), timestamp_writes: writes });
        pass.set_bind_group(0, &group, &[]);
        pass.set_pipeline(pipeline);
        pass.dispatch_workgroups(probes, 1, 1);
        // every probe's cube into the volume (the grid may have moved), timed with the rays: it is the pass's cost too
        pass.set_pipeline(resolve);
        pass.dispatch_workgroups(total.div_ceil(64), 1, 1);
    }
    if let Some(t) = state.timer.as_ref().filter(|_| timed) {
        enc.resolve_query_set(&t.set, 0..2, &t.resolve, 0);
        enc.copy_buffer_to_buffer(&t.resolve, 0, &t.read, 0, 16);
    }
    queue.submit([enc.finish()]);
    if let Some(t) = state.timer.as_mut().filter(|_| timed) {
        let done = Arc::new(AtomicBool::new(false));
        let d = done.clone();
        t.read.slice(..).map_async(wgpu::MapMode::Read, move |r| d.store(r.is_ok(), Ordering::Release));
        t.pending = Some(done);
    }
    report(&mut state, budget, total);
}

fn timer(device: &RenderDevice, queue: &RenderQueue) -> Timer {
    let d = device.wgpu_device();
    let set = d.create_query_set(&wgpu::QuerySetDescriptor { label: Some("raea gi timer"), ty: wgpu::QueryType::Timestamp, count: 2 });
    let resolve = d.create_buffer(&wgpu::BufferDescriptor { label: Some("raea gi timer resolve"), size: 16, usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
    let read = d.create_buffer(&wgpu::BufferDescriptor { label: Some("raea gi timer read"), size: 16, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    Timer { set, resolve, read, period: queue.get_timestamp_period() as f64, pending: None }
}

/// the last timed frame's GPU milliseconds, once its readback is done
fn collect(state: &mut State) {
    let Some(t) = state.timer.as_mut() else { return };
    let Some(done) = t.pending.as_ref() else { return };
    if !done.load(Ordering::Acquire) {
        return;
    }
    let ms = {
        let Ok(view) = t.read.slice(..).get_mapped_range() else { return };
        let ts: [u64; 2] = [u64::from_le_bytes(view[0..8].try_into().unwrap()), u64::from_le_bytes(view[8..16].try_into().unwrap())];
        ts[1].saturating_sub(ts[0]) as f64 * t.period / 1e6
    };
    t.read.unmap();
    t.pending = None;
    if ms > 0.0 {
        state.seen.push(ms);
    }
}

/// once a second: the timed frames so far, to RAEA_GI_STATS's file
fn report(state: &mut State, budget: u32, total: u32) {
    let Some(path) = std::env::var_os("RAEA_GI_STATS") else { return };
    if state.written.is_some_and(|t| t.elapsed().as_secs_f64() < 1.0) || state.seen.is_empty() {
        return;
    }
    state.written = Some(std::time::Instant::now());
    let mut v = state.seen.clone();
    v.sort_by(f64::total_cmp);
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    // the ray scene's CPU milliseconds a call: the instance list, the TLAS, a BLAS batch (lib.rs CPU_US)
    let cpu = |k: usize| {
        let (us, n) = (crate::CPU_US[2 * k].load(Ordering::Relaxed), crate::CPU_US[2 * k + 1].load(Ordering::Relaxed));
        format!("{:.3}", us as f64 / n.max(1) as f64 / 1000.0)
    };
    let text = format!(
        "{{\"rays_a_frame\":{},\"probes\":{total},\"probes_a_frame\":{},\"frames\":{},\"gpu_ms_p50\":{:.3},\"gpu_ms_p90\":{:.3},\"gpu_ms_max\":{:.3},\"cpu_ms_gather\":{},\"cpu_ms_tlas\":{},\"cpu_ms_blas_batch\":{},\"blas_batches\":{}}}\n",
        (budget / RAYS).min(total) * RAYS,
        (budget / RAYS).min(total),
        v.len(),
        at(0.5),
        at(0.9),
        at(1.0),
        cpu(0),
        cpu(1),
        cpu(2),
        crate::CPU_US[5].load(Ordering::Relaxed)
    );
    let _ = std::fs::write(path, text);
}
