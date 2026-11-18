//! raea GI's screen side (docs/realism.feature LT8, D6): the indirect diffuse light each pixel gets, drawn by the main
//! pass in place of the irradiance volume, the environment map's diffuse and the ambient light (raea's bevy_pbr
//! RaeaGiScreen, the mesh view group's binding 41, read texel for pixel: no sampler, so a hooked material at Metal's 16
//! keeps building, where Bevy's irradiance volume took a 17th).
//!
//! Its first form is a measuring step, as LT2's irradiance volume was meant to be: each pixel reads the world cache
//! (raea-sys-rt cache.wesl) at its own surface, from the depth and normal prepasses, and takes out the sun's share,
//! which raster draws with its shadows. What the cache learned from the sky and from a bounce more reaches the screen,
//! so its light can be measured against the reference (LT1). A pixel whose cell is new this frame draws Bevy's own
//! light until the cell has learned. Screen probes come after (LT8): a probe every 16 pixels tracing the screen, then
//! the TLAS, ending in the cache.
//!
//! On with RAEA_GI=1, on every 3D camera, which gets its depth and normal prepasses, with MSAA off.

use bevy::{
    core_pipeline::{
        prepass::{DepthPrepass, NormalPrepass, ViewPrepassTextures},
        schedule::Core3d,
        Core3dSystems,
    },
    diagnostic::FrameCount,
    pbr::RaeaGiScreen,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{
            binding_types::{texture_2d, texture_depth_2d, texture_storage_2d, uniform_buffer, uniform_buffer_sized},
            *,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        texture::{CachedTexture, TextureCache},
        view::{ViewUniform, ViewUniformOffset, ViewUniforms},
        Render, RenderApp, RenderStartup, RenderSystems,
    },
};
use raea_sys_rt::gi::WorldCache;

/// A camera whose pixels take raea GI's light (RAEA_GI=1). Brings the prepasses it reads.
#[derive(Component, Clone, Copy, Default, ExtractComponent)]
#[extract_app(bevy::render::RenderApp)]
#[require(DepthPrepass, NormalPrepass)]
pub struct GiScreen;

/// the screen's format: the light arriving (rgb), and 1 where a cell had any (a)
const FORMAT: TextureFormat = TextureFormat::Rgba16Float;

pub fn install(app: &mut App) {
    if std::env::var("RAEA_GI").as_deref() != Ok("1") {
        return;
    }
    bevy::asset::embedded_asset!(app, "screen.wesl");
    app.add_plugins(ExtractComponentPlugin::<GiScreen>::default()).add_systems(PostUpdate, cameras);
    let Some(render) = app.get_sub_app_mut(RenderApp) else { return };
    render
        .add_systems(RenderStartup, init)
        .add_systems(Render, textures.in_set(RenderSystems::PrepareResources))
        .add_systems(Core3d, draw.after(Core3dSystems::Prepass).before(Core3dSystems::MainPass));
}

/// Every 3D camera takes the screen's light, single-sampled: PrepassRead keeps MSAA off and the prepasses on whatever
/// the camera's row says (raea-sys-post's msaa_for_3d and sync).
fn cameras(mut commands: Commands, q: Query<Entity, (With<Camera3d>, Without<GiScreen>)>) {
    for e in &q {
        commands.entity(e).insert((GiScreen, raea_sys_compose::passes::PrepassRead, Msaa::Off));
    }
}

/// a view's screen and this frame's parameters
#[derive(Component)]
struct GiFrame {
    screen: CachedTexture,
    params: Buffer,
}

/// screen.wesl's Params, three vec4s: toward the sun, the sun's light (lux), the frame's salt
const PARAMS_BYTES: u64 = 48;

/// Each view's screen, at its target's size, and the lighting's binding to it. None until the world cache has run (its
/// first frame), so the main pass draws Bevy's own light until then.
fn textures(
    mut commands: Commands,
    mut cache: ResMut<TextureCache>,
    device: Res<RenderDevice>,
    frame: Res<FrameCount>,
    world: Option<Res<WorldCache>>,
    views: Query<(Entity, &ExtractedCamera), With<GiScreen>>,
) {
    let Some(world) = world else { return };
    let (d, l) = (world.sun_dir, world.sun_light);
    let words: [u32; 12] = [d.x.to_bits(), d.y.to_bits(), d.z.to_bits(), 0, l.x.to_bits(), l.y.to_bits(), l.z.to_bits(), 0, frame.0.wrapping_mul(0x9e37_79b9), 0, 0, 0];
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    for (e, camera) in &views {
        let Some(size) = camera.physical_target_size else { continue };
        let screen = cache.get(
            &device,
            TextureDescriptor {
                label: Some("raea_gi_screen"),
                size: Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: FORMAT,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );
        let params = device.create_buffer_with_data(&BufferInitDescriptor { label: Some("raea_gi_screen_params"), contents: &bytes, usage: BufferUsages::UNIFORM });
        commands.entity(e).insert((RaeaGiScreen(screen.default_view.clone()), GiFrame { screen, params }));
    }
}

#[derive(Resource)]
struct GiPipeline {
    layout: BindGroupLayoutDescriptor,
    id: CachedComputePipelineId,
}

fn init(mut commands: Commands, assets: Res<AssetServer>, cache: Res<PipelineCache>) {
    let layout = BindGroupLayoutDescriptor::new(
        "raea_gi_screen_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<ViewUniform>(true),
                texture_depth_2d(),
                texture_2d(TextureSampleType::Float { filterable: false }),
                texture_storage_2d(FORMAT, StorageTextureAccess::WriteOnly),
                uniform_buffer_sized(false, core::num::NonZero::new(PARAMS_BYTES)),
            ),
        ),
    );
    // group 1 is the world cache's own (raea-sys-rt cache.rs), bound as raea-sys-rt bound it this frame
    let id = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("raea_gi_screen".into()),
        layout: vec![layout.clone(), raea_sys_rt::cache::layout()],
        shader: assets.load("embedded://raea_sys_gi/screen.wesl"),
        entry_point: Some("main".into()),
        ..default()
    });
    commands.insert_resource(GiPipeline { layout, id });
}

/// Each view's screen from its prepasses and this frame's world cache, before the main pass reads it.
fn draw(
    view: ViewQuery<(&ViewPrepassTextures, &ViewUniformOffset, &GiFrame)>,
    pipeline: Option<Res<GiPipeline>>,
    world: Option<Res<WorldCache>>,
    uniforms: Res<ViewUniforms>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (Some(p), Some(world), Some(view_binding)) = (pipeline, world, uniforms.uniforms.binding()) else { return };
    let (prepass, offset, f) = view.into_inner();
    let (Some(rp), Some(depth), Some(normal)) = (cache.get_compute_pipeline(p.id), prepass.depth_only_view(), prepass.normal_view()) else { return };
    let group = ctx.render_device().create_bind_group(
        "raea_gi_screen",
        &cache.get_bind_group_layout(&p.layout),
        &BindGroupEntries::sequential((view_binding, depth, normal, &f.screen.default_view, f.params.as_entire_binding())),
    );
    let size = f.screen.texture.size();
    let mut pass = ctx.command_encoder().begin_compute_pass(&ComputePassDescriptor { label: Some("raea_gi_screen"), timestamp_writes: None });
    pass.set_pipeline(rp);
    pass.set_bind_group(0, &group, &[offset.offset]);
    pass.set_bind_group(1, &world.group, &[]);
    pass.dispatch_workgroups(size.width.div_ceil(8), size.height.div_ceil(8), 1);
}
