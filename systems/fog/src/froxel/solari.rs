//! Solari's lights in the froxel grid (docs/raytracing.feature RT19, froxel_solari.wesl). Under Solari the grid's
//! light pass sees only Bevy's lights, so a lamp Solari lights a room with (an emissive mesh) and the sky through a
//! skylight lit no air: photo 5's hall needed a point light beside each lamp for its glow. For a view with
//! SolariLighting and Solari's scene bound, a compute pass after the light pass writes what Solari's emissive meshes
//! and the sky scatter at each cell (resampled from Solari's light list, a shadow ray each through its scene, averaged
//! over the cell's last frames) into a texture of the atlas's layout, and the integrate pass adds it to the atlas's
//! light. A second pass blurs that light over each cell's neighbours before the medium scatters it.
//! RAEA_FOG_SOLARI=0 leaves it all out (the A/B); RAEA_FOG_SOLARI_CANDIDATES (8) lights drawn a cell,
//! RAEA_FOG_SOLARI_SKY (1) sky rays a cell, RAEA_FOG_SOLARI_FRAMES (16) frames its history averages,
//! RAEA_FOG_SOLARI_PARTS (lamps,sky) which of them light it, RAEA_FOG_SOLARI_FILTER=0 no blur.

use super::*;
use bevy::render::render_resource::binding_types::texture_storage_2d;
use bevy::render::view::{ViewUniform, ViewUniformOffset, ViewUniforms};
use bevy::solari::{realtime::SolariLighting, scene::RaytracingSceneBindings};

/// What the pass reads besides the grid (froxel_solari.wesl's `SolariFog`).
#[derive(ShaderType, Clone, Copy, Default)]
pub struct SolariFogParams {
    /// the last frame's view (unjittered) and its grid's slab, for the history
    pub prev_clip_from_world: Mat4,
    pub prev_slab: Vec4,
    /// x: the frame's number; y: 1 to start the history again; z: candidates a cell; w: frames the history averages
    pub frame: UVec4,
    /// x: what lights it (1 the emissive meshes, 2 the sky); y: sky rays a cell; z: 1 to blur
    pub more: UVec4,
}

/// A view's Solari light for the grid: three textures of the atlas's size, this frame's light and the last's (its
/// history, the two swapped each frame), and what the integrate pass adds.
#[derive(Component)]
pub struct SolariFog {
    textures: [(Texture, TextureView); 2],
    filtered: (Texture, TextureView),
    size: UVec2,
    frame: u32,
    prev: (Mat4, Vec4),
    buffer: UniformBuffer<SolariFogParams>,
}

impl SolariFog {
    fn now(&self) -> &TextureView {
        &self.textures[(self.frame & 1) as usize].1
    }
    fn was(&self) -> &TextureView {
        &self.textures[((self.frame + 1) & 1) as usize].1
    }
}

/// The pass's pipeline and the integrate pass's that adds its light, made once Solari's scene layout exists.
#[derive(Resource)]
pub struct SolariFogPipelines {
    layout: BindGroupLayoutDescriptor,
    integrate_layout: BindGroupLayoutDescriptor,
    solari: CachedComputePipelineId,
    filter: CachedComputePipelineId,
    integrate: CachedComputePipelineId,
}

fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("RAEA_FOG_SOLARI").as_deref() != Ok("0"))
}

fn setting(name: &str, default: u32) -> u32 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).filter(|v: &u32| *v > 0).unwrap_or(default)
}

fn make_pipelines(cache: &PipelineCache, assets: &AssetServer, scene: &RaytracingSceneBindings) -> SolariFogPipelines {
    let layout = BindGroupLayoutDescriptor::new(
        "froxel_solari_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<Params>(false),
                storage_buffer_read_only::<GpuVolume>(false),
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<SolariFogParams>(false),
                texture_2d(TextureSampleType::Float { filterable: false }),
                texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
            ),
        ),
    );
    let integrate_layout = BindGroupLayoutDescriptor::new(
        "froxel_integrate_solari_layout",
        &BindGroupLayoutEntries::with_indices(
            ShaderStages::COMPUTE,
            (
                (0, uniform_buffer::<Params>(false)),
                (1, texture_2d(TextureSampleType::Float { filterable: false })),
                (2, texture_storage_3d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly)),
                (6, texture_2d(TextureSampleType::Float { filterable: false })),
            ),
        ),
    );
    let shader: Handle<Shader> = assets.load("embedded://raea_sys_fog/froxel_solari.wesl");
    let [solari, filter] = ["solari", "filter_light"].map(|entry| {
        cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(format!("froxel_{entry}").into()),
            layout: vec![scene.bind_group_layout.clone(), layout.clone()],
            shader: shader.clone(),
            entry_point: Some(entry.into()),
            ..default()
        })
    });
    let integrate = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("froxel_integrate_solari".into()),
        layout: vec![integrate_layout.clone()],
        shader: assets.load("embedded://raea_sys_fog/froxel_grid.wesl"),
        shader_defs: vec!["SOLARI".into()],
        entry_point: Some("integrate".into()),
        ..default()
    });
    SolariFogPipelines { layout, integrate_layout, solari, filter, integrate }
}

fn texture(device: &RenderDevice, size: UVec2, i: usize) -> (Texture, TextureView) {
    let t = device.create_texture(&TextureDescriptor {
        label: Some(["froxel_solari_a", "froxel_solari_b", "froxel_solari_filtered"][i]),
        size: Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let v = t.create_view(&TextureViewDescriptor::default());
    (t, v)
}

/// Each Solari view's textures and parameters for this frame (the pipelines first, once Solari's scene exists).
#[allow(clippy::type_complexity)]
pub fn prepare(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    assets: Res<AssetServer>,
    pipelines: Option<Res<SolariFogPipelines>>,
    scene: Option<Res<RaytracingSceneBindings>>,
    mut views: Query<(Entity, &ViewGrid, &ExtractedView, &SolariLighting, Option<&mut SolariFog>)>,
) {
    if !on() {
        return;
    }
    let Some(scene) = scene else { return };
    if pipelines.is_none() {
        commands.insert_resource(make_pipelines(&cache, &assets, &scene));
        return;
    }
    let (candidates, frames, sky) = (setting("RAEA_FOG_SOLARI_CANDIDATES", 8), setting("RAEA_FOG_SOLARI_FRAMES", 16), setting("RAEA_FOG_SOLARI_SKY", 1));
    let parts = std::env::var("RAEA_FOG_SOLARI_PARTS").unwrap_or_else(|_| "lamps,sky".into());
    let parts = parts.contains("lamps") as u32 | (parts.contains("sky") as u32) << 1;
    let filter = (std::env::var("RAEA_FOG_SOLARI_FILTER").as_deref() != Ok("0")) as u32;
    for (e, grid, view, solari, fog) in &mut views {
        let g = grid.params.grid;
        let size = UVec2::new(g.x * g.w, g.y * g.z.div_ceil(g.w));
        let now = (view.clip_from_view * view.world_from_view.to_matrix().inverse(), grid.params.slab);
        let mut fresh = None;
        let fog = match fog {
            Some(f) if f.size == size => f.into_inner(),
            _ => fresh.insert(SolariFog { textures: [texture(&device, size, 0), texture(&device, size, 1)], filtered: texture(&device, size, 2), size, frame: 0, prev: now, buffer: default() }),
        };
        let reset = fog.frame == 0 || solari.reset;
        if fog.frame > 0 {
            fog.frame += 1;
        } else {
            // the first frame writes texture 1, so the next reads it as its history
            fog.frame = 1;
        }
        *fog.buffer.get_mut() = SolariFogParams { prev_clip_from_world: fog.prev.0, prev_slab: fog.prev.1, frame: UVec4::new(fog.frame, reset as u32, candidates, frames), more: UVec4::new(parts, sky, filter, 0) };
        fog.buffer.write_buffer(&device, &queue);
        fog.prev = now;
        if let Some(f) = fresh {
            commands.entity(e).insert(f);
        }
    }
}

/// After the light pass: Solari's light into the view's texture, and the integrate pass's pipeline and bind group that
/// add it, or None (no Solari here, or not ready) to integrate as before.
#[allow(clippy::too_many_arguments)]
pub fn draw<'a>(
    ctx: &mut RenderContext,
    cache: &'a PipelineCache,
    pipelines: Option<&SolariFogPipelines>,
    scene: Option<&RaytracingSceneBindings>,
    view_uniforms: &ViewUniforms,
    fog: Option<&SolariFog>,
    offset: Option<&ViewUniformOffset>,
    params: BindingResource,
    volumes: BindingResource,
    atlas: &TextureView,
    grid: &TextureView,
) -> Option<(&'a ComputePipeline, BindGroup)> {
    let (p, fog, offset) = (pipelines?, fog?, offset?);
    let (Some(scene_group), Some(pass), Some(filter), Some(integrate), Some(view)) = (
        scene.and_then(|s| s.bind_group.as_ref()),
        cache.get_compute_pipeline(p.solari),
        cache.get_compute_pipeline(p.filter),
        cache.get_compute_pipeline(p.integrate),
        view_uniforms.uniforms.binding(),
    ) else {
        return None;
    };
    let device = ctx.render_device().clone();
    let layout = cache.get_bind_group_layout(&p.layout);
    let fog_params = fog.buffer.binding()?;
    // the light found this frame into its history, then the history blurred and scattered
    let steps = [("froxel_solari", pass, fog.was(), fog.now()), ("froxel_solari_filter", filter, fog.now(), &fog.filtered.1)];
    for (label, pipeline, input, output) in steps {
        let group = device.create_bind_group(label, &layout, &BindGroupEntries::sequential((params.clone(), volumes.clone(), view.clone(), fog_params.clone(), input, output)));
        let mut c = ctx.command_encoder().begin_compute_pass(&ComputePassDescriptor { label: Some(label), timestamp_writes: None });
        c.set_pipeline(pipeline);
        c.set_bind_group(0, scene_group, &[]);
        c.set_bind_group(1, &group, &[offset.offset]);
        c.dispatch_workgroups(fog.size.x.div_ceil(8), fog.size.y.div_ceil(8), 1);
    }
    let group = device.create_bind_group(
        "froxel_integrate_solari",
        &cache.get_bind_group_layout(&p.integrate_layout),
        &BindGroupEntries::with_indices(((0, params), (1, atlas), (2, grid), (6, &fog.filtered.1))),
    );
    Some((integrate, group))
}
