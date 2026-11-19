//! RT4 (docs/raytracing.feature): MetalFX's temporal denoiser on Solari's lighting, on the Mac. No denoiser of Bevy's
//! runs on Metal (Solari's is DLSS Ray Reconstruction, NVIDIA's), so Solari draws its few rays a pixel as noise there;
//! MTLFXTemporalDenoisedScaler (macOS 26) takes that noisy colour and the frame's guide data and gives it back clean.
//!
//! Each frame, after Solari has lit the view and before the opaque pass draws the forward meshes, a compute pass writes the guide textures MetalFX reads from Bevy's G-buffer (diffuse and specular albedo,
//! world normal, roughness); then a command buffer of its own, recorded through wgpu's Metal backend, runs the
//! denoiser on the view's colour, depth and motion and copies its output back over the colour, before the forward
//! meshes and the tonemapping. The two command buffers go to wgpu's queue in that order, after Solari's.
//!
//! Drawn below its size (RT9, upscale.rs), the view's main pass holds only the top-left corner of its textures. Then
//! the denoiser waits for the forward meshes and runs where Bevy's DLSS does (EarlyPostProcess): from that corner, with
//! the projection's jitter, out to the whole view, denoising and upscaling in one pass, as MetalFX's denoised scaler is
//! made to. A pixel with nothing in the G-buffer (a forward mesh, the sky) gives the denoiser its own colour as its
//! albedo, so it has nothing there to smooth. RAEA_SOLARI_UPSCALE=two denoises the corner before the forward meshes and
//! leaves the scaling to MetalFX's temporal scaler (upscale.rs), the first way, to compare: the denoiser settled each
//! frame on the small grid, so the scaler gained nothing from the jitter (0.7 of the full size's fine detail at 0.67).
//!
//! Where there's no MetalFX denoiser (not macOS 26, not Metal, a device it doesn't support) the lighting draws as it
//! is and raea says so once. RAEA_SOLARI_DENOISE=0 turns it off, to see the noise.

use super::*;
use bevy::camera::MainPassResolutionOverride;
use bevy::core_pipeline::core_3d::main_opaque_pass_3d;
use bevy::ecs::schedule::Schedules;
use bevy::core_pipeline::prepass::ViewPrepassTextures;
use bevy::core_pipeline::schedule::{Core3d, Core3dSystems};
use bevy::render::render_resource::TextureFormat;
use bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy::render::camera::{ExtractedCamera, TemporalJitter};
use bevy::render::view::{ExtractedView, ViewTarget};
use bevy::solari::prelude::SolariLighting;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::Message;
use objc2_metal::{MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLOrigin, MTLPixelFormat, MTLSize, MTLTexture};
use objc2_metal_fx::{MTLFXTemporalDenoisedScaler, MTLFXTemporalDenoisedScalerBase, MTLFXTemporalDenoisedScalerDescriptor};
use wgpu::hal::api::Metal;

/// After app.finish(), when Solari's plugin has put its lighting system in Core3d: the denoiser right after that system
/// and before the opaque pass, as forward-drawn meshes (a <Character>'s skin, an extended material) come after it with
/// nothing in the G-buffer, and denoising them would smear them. Bevy doesn't export Solari's system, so it's found by
/// name and ordered against by its system type set.
pub(crate) fn add(app: &mut App) {
    if std::env::var("RAEA_SOLARI_DENOISE").is_ok_and(|v| v == "0") {
        return;
    }
    let Some(r) = app.get_sub_app_mut(bevy::render::RenderApp) else { return };
    let mut schedules = r.world_mut().resource_mut::<Schedules>();
    let Some(core) = schedules.get_mut(Core3d) else { return };
    let mut solari = None;
    for (_, s, _) in core.graph().systems.iter() {
        if format!("{}", s.name()).ends_with("solari_lighting") {
            solari = s.default_system_sets().into_iter().next();
            break;
        }
    }
    let Some(solari) = solari else {
        eprintln!("raea: <SolariLighting>: Solari's lighting isn't in the render schedule; no MetalFX denoiser");
        return;
    };
    core.add_systems(denoise.after(solari).before(main_opaque_pass_3d).in_set(Core3dSystems::MainPass));
    core.add_systems(denoise_up.before(super::upscale::upscale).in_set(Core3dSystems::EarlyPostProcess));
}

/// The emission taken out before the denoiser and put back after (see DECODE_WGSL): gathered over frames by default
/// (emit_taa.rs), each frame's alone with RAEA_SOLARI_EMISSIVE=split; =mask leaves it in, for the denoiser.
fn split() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("RAEA_SOLARI_EMISSIVE").as_deref() == Ok("split") || super::emit_taa::on())
}

/// A view drawn small is denoised and brought up in one pass (unless RAEA_SOLARI_UPSCALE=two).
fn one_pass() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("RAEA_SOLARI_UPSCALE").as_deref() != Ok("two"))
}

// Bevy's G-buffer (bevy_pbr deferred/functions.wesl, types.wesl) to MetalFX's guides: r = base colour (sRGB-ish, pow
// 1/2.2) and perceptual roughness as unorm4x8; b = reflectance, metallic, occlusion, clearcoat as unorm4x8; a = a 24-bit
// octahedral normal (two 12-bit halves) and flags; g = emissive as rgb9e5. Specular albedo is F0 (Bevy's 0.16
// reflectance² for dielectrics, the base colour for metals).
// What a surface emits is no noise. Denoised as lighting, a screen's picture or a sign's letters came out smeared, its
// texture taken for noise against a near-black albedo (photo 2's monitor). So the emission's share of each pixel's
// colour goes to MetalFX's denoise strength mask, which leaves a pixel marked 1 undenoised while still gathering its
// frames (which anti-aliases its edges), and the emission, compressed as e / (1 + e), is added to its albedo, so what
// the denoiser does smooth isn't swollen against a near-black one. A pixel with nothing in the G-buffer (a forward
// mesh, the sky) is raster-lit, with nothing to denoise, and is marked 1. Measured on photo 2's monitor:
// RAEA_SOLARI_EMISSIVE=split takes the emission (times the exposure, as Solari adds it) out of the colour into `emit`
// and `restore` puts it back after MetalFX, the first way, and kept out of the denoiser an emissive edge stayed
// stepped and crawled with the jitter (500 pixels jumping 80 levels a frame on the window's and curtain's edges);
// emission as the albedo where it led, the screen's dark parts kept their noise; added to every albedo with no mask,
// the picture blurred; split only where it was weak, pixels on the boundary counted it twice and sparkled
const DECODE_WGSL: &str = r#"
@group(0) @binding(0) var gbuf: texture_2d<u32>;
@group(0) @binding(1) var diffuse: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var specular: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var normal: texture_storage_2d<rgba16float, write>;
@group(0) @binding(4) var rough: texture_storage_2d<r32float, write>;
@group(0) @binding(5) var color: texture_storage_2d<rgba16float, read_write>;
// [0].x: the camera's exposure, .y: 1 to split all emission out (RAEA_SOLARI_EMISSIVE=split); [1]: the output's size
// and the input's (the corner of the colour the main pass drew); [2].xyz: the camera's back, world-space; [3].xy: the
// jitter MetalFX is told, in input pixels
@group(0) @binding(6) var<uniform> params: array<vec4<f32>, 4>;
// the emission `main` took out of the colour, at the input's size, for `restore` to put back
@group(0) @binding(7) var emit: texture_storage_2d<rgba16float, read_write>;
// MetalFX's denoise strength mask: 1 leaves a pixel undenoised
@group(0) @binding(8) var mask: texture_storage_2d<r32float, write>;
fn rgb9e5(v: u32) -> vec3<f32> {
    let scale = exp2(f32(i32(extractBits(v, 27u, 5u)) - 15 - 9));
    return vec3<f32>(f32(extractBits(v, 0u, 9u)), f32(extractBits(v, 9u, 9u)), f32(extractBits(v, 18u, 9u))) * scale;
}
fn unorm4x8(p: u32) -> vec4<f32> {
    return vec4<f32>(f32(p & 255u), f32((p >> 8u) & 255u), f32((p >> 16u) & 255u), f32(p >> 24u)) / 255.0;
}
fn octahedral(v: vec2<f32>) -> vec3<f32> {
    let f = v * 2.0 - 1.0;
    var n = vec3<f32>(f, 1.0 - abs(f.x) - abs(f.y));
    let t = saturate(-n.z);
    let w = select(vec2<f32>(t), vec2<f32>(-t), n.xy >= vec2<f32>(0.0));
    n = vec3<f32>(n.xy + w, n.z);
    return normalize(n);
}
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(diffuse);
    if (id.x >= size.x || id.y >= size.y) { return; }
    let p = vec2<i32>(id.xy);
    let g = textureLoad(gbuf, p, 0);
    if (all(g == vec4<u32>(0u))) {
        // nothing deferred here (a forward mesh, the sky): its colour as its albedo, so there's nothing to smooth
        let c = textureLoad(color, p).rgb;
        textureStore(diffuse, p, vec4<f32>(c / max(1.0, max(c.r, max(c.g, c.b))), 1.0));
        textureStore(specular, p, vec4<f32>(0.0, 0.0, 0.0, 1.0));
        textureStore(normal, p, vec4<f32>(params[2].xyz, 0.0));
        textureStore(rough, p, vec4<f32>(1.0, 0.0, 0.0, 0.0));
        textureStore(emit, p, vec4<f32>(0.0));
        textureStore(mask, p, vec4<f32>(1.0, 0.0, 0.0, 0.0));
        return;
    }
    let a = unorm4x8(g.r);
    let props = unorm4x8(g.b);
    let base = pow(a.rgb, vec3<f32>(2.2));
    let metallic = props.y;
    let f0 = mix(vec3<f32>(0.16 * props.x * props.x), base, metallic);
    let n = octahedral(vec2<f32>(f32(g.a & 0xFFFu), f32((g.a >> 12u) & 0xFFFu)) / 4095.0);
    textureStore(normal, p, vec4<f32>(n, 0.0));
    textureStore(rough, p, vec4<f32>(a.w, 0.0, 0.0, 0.0));
    let c = textureLoad(color, p);
    let e = rgb9e5(g.g) * params[0].x;
    textureStore(specular, p, vec4<f32>(f0, 1.0));
    if (params[0].y > 0.5) {
        let taken = min(e, c.rgb);
        textureStore(diffuse, p, vec4<f32>(base * (1.0 - metallic), 1.0));
        textureStore(color, p, vec4<f32>(c.rgb - taken, c.a));
        textureStore(emit, p, vec4<f32>(taken, 0.0));
        textureStore(mask, p, vec4<f32>(0.0));
        return;
    }
    let luma = vec3<f32>(0.2126, 0.7152, 0.0722);
    let share = dot(e, luma) / max(dot(c.rgb, luma), 1e-4);
    textureStore(diffuse, p, vec4<f32>(min(base * (1.0 - metallic) + e / (vec3<f32>(1.0) + e), vec3<f32>(1.0)), 1.0));
    textureStore(mask, p, vec4<f32>(smoothstep(0.3, 0.7, share), 0.0, 0.0, 0.0));
}
fn emitted(i: vec2<i32>) -> vec3<f32> {
    return textureLoad(emit, clamp(i, vec2<i32>(0), vec2<i32>(params[1].zw) - 1)).rgb;
}
@compute @workgroup_size(8, 8, 1)
fn restore(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = vec2<u32>(params[1].xy);
    if (id.x >= size.x || id.y >= size.y) { return; }
    let p = vec2<i32>(id.xy);
    // where this output pixel lies in the jittered G-buffer, between its pixels' centres
    let q = (vec2<f32>(id.xy) + 0.5) * params[1].zw / params[1].xy - params[3].xy - 0.5;
    let i = vec2<i32>(floor(q));
    let t = q - floor(q);
    let e = mix(mix(emitted(i), emitted(i + vec2<i32>(1, 0)), t.x), mix(emitted(i + vec2<i32>(0, 1)), emitted(i + vec2<i32>(1, 1)), t.x), t.y);
    let c = textureLoad(color, p);
    textureStore(color, p, vec4<f32>(c.rgb + e, c.a));
}
"#;

/// A view's denoiser and its textures, made again when its input's or output's size changes.
struct View {
    /// the main pass's size (the guides'), and the denoised frame's
    size: UVec2,
    out_size: UVec2,
    scaler: Retained<ProtocolObject<dyn MTLFXTemporalDenoisedScaler>>,
    diffuse: wgpu::Texture,
    specular: wgpu::Texture,
    normal: wgpu::Texture,
    rough: wgpu::Texture,
    emit: wgpu::Texture,
    mask: wgpu::Texture,
    out: wgpu::Texture,
    /// the split emission's history (RAEA_SOLARI_EMISSIVE=taa, emit_taa.rs)
    taa: Option<super::emit_taa::History>,
    fresh: bool,
}

#[derive(Default)]
struct Kit {
    /// the guides (and the emissive out), the emissive back in, from this frame or gathered over frames (emit_taa.rs),
    /// and the sampler that reads the gathered emission's history
    pipeline: Option<(wgpu::ComputePipeline, wgpu::ComputePipeline, wgpu::ComputePipeline, wgpu::Sampler)>,
    views: HashMap<Entity, View>,
    off: bool,
}
// the Metal objects are only touched on the render thread, in this system
unsafe impl Send for Kit {}
unsafe impl Sync for Kit {}

pub(super) fn texture(dev: &wgpu::Device, size: UVec2, format: wgpu::TextureFormat, usage: wgpu::TextureUsages, label: &str) -> wgpu::Texture {
    dev.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

/// A texture's MTLTexture, retained.
fn mtl(t: &wgpu::Texture) -> Option<Retained<ProtocolObject<dyn MTLTexture>>> {
    unsafe { t.as_hal::<Metal>() }.map(|t| t.raw_handle().retain())
}

impl View {
    fn new(dev: &wgpu::Device, size: UVec2, out_size: UVec2) -> Result<View, String> {
        let hal = unsafe { dev.as_hal::<Metal>() }.ok_or("the GPU isn't on Metal")?;
        let device = hal.raw_device();
        unsafe {
            if !MTLFXTemporalDenoisedScalerDescriptor::supportsDevice(device) {
                return Err("MetalFX has no denoiser for this GPU (it needs macOS 26 and Apple silicon)".into());
            }
            let d = MTLFXTemporalDenoisedScalerDescriptor::new();
            d.setColorTextureFormat(MTLPixelFormat::RGBA16Float);
            d.setDepthTextureFormat(MTLPixelFormat::Depth32Float);
            d.setMotionTextureFormat(MTLPixelFormat::RG16Float);
            d.setDiffuseAlbedoTextureFormat(MTLPixelFormat::RGBA16Float);
            d.setSpecularAlbedoTextureFormat(MTLPixelFormat::RGBA16Float);
            d.setNormalTextureFormat(MTLPixelFormat::RGBA16Float);
            d.setRoughnessTextureFormat(MTLPixelFormat::R32Float);
            d.setDenoiseStrengthMaskTextureEnabled(true);
            d.setDenoiseStrengthMaskTextureFormat(MTLPixelFormat::R32Float);
            d.setOutputTextureFormat(MTLPixelFormat::RGBA16Float);
            d.setInputWidth(size.x as _);
            d.setInputHeight(size.y as _);
            d.setOutputWidth(out_size.x as _);
            d.setOutputHeight(out_size.y as _);
            d.setRequiresSynchronousInitialization(true);
            let scaler = d.newTemporalDenoisedScalerWithDevice(device).ok_or("MetalFX made no denoiser")?;
            drop(hal);
            use wgpu::TextureUsages as U;
            let guide = U::STORAGE_BINDING | U::TEXTURE_BINDING;
            Ok(View {
                size,
                out_size,
                scaler,
                diffuse: texture(dev, size, wgpu::TextureFormat::Rgba16Float, guide, "metalfx diffuse albedo"),
                specular: texture(dev, size, wgpu::TextureFormat::Rgba16Float, guide, "metalfx specular albedo"),
                normal: texture(dev, size, wgpu::TextureFormat::Rgba16Float, guide, "metalfx normal"),
                rough: texture(dev, size, wgpu::TextureFormat::R32Float, guide, "metalfx roughness"),
                emit: texture(dev, size, wgpu::TextureFormat::Rgba16Float, guide, "metalfx split emission"),
                mask: texture(dev, size, wgpu::TextureFormat::R32Float, guide, "metalfx denoise strength"),
                out: texture(dev, out_size, wgpu::TextureFormat::Rgba16Float, guide | U::RENDER_ATTACHMENT | U::COPY_SRC, "metalfx output"),
                taa: super::emit_taa::on().then(|| super::emit_taa::History::new(dev, out_size)),
                fresh: true,
            })
        }
    }

    /// The denoiser on `cb`, then its output copied over the colour's corner of its size. `jitter` is the
    /// camera's this frame, in the main pass's pixels, as MetalFX takes it (Bevy's TemporalJitter negated, as its DLSS
    /// gives it).
    fn encode(&mut self, cb: &ProtocolObject<dyn MTLCommandBuffer>, color: &wgpu::Texture, depth: &wgpu::Texture, motion: &wgpu::Texture, view: &ExtractedView, jitter: Vec2, reset: bool) -> Result<(), &'static str> {
        let get = |t: &wgpu::Texture| mtl(t).ok_or("a texture isn't a Metal one");
        let (color, depth, motion) = (get(color)?, get(depth)?, get(motion)?);
        let (diffuse, specular, normal, rough, out) = (get(&self.diffuse)?, get(&self.specular)?, get(&self.normal)?, get(&self.rough)?, get(&self.out)?);
        let mask = get(&self.mask)?;
        let s = &*self.scaler;
        unsafe {
            s.setColorTexture(Some(&color));
            s.setDepthTexture(Some(&depth));
            s.setMotionTexture(Some(&motion));
            s.setDiffuseAlbedoTexture(Some(&diffuse));
            s.setSpecularAlbedoTexture(Some(&specular));
            s.setNormalTexture(Some(&normal));
            s.setRoughnessTexture(Some(&rough));
            s.setDenoiseStrengthMaskTexture(Some(&mask));
            s.setOutputTexture(Some(&out));
            // Bevy's motion is this frame's UV minus the last's; MetalFX's points from this pixel to where it was, in
            // pixels
            s.setMotionVectorScaleX(-(self.size.x as f32));
            s.setMotionVectorScaleY(-(self.size.y as f32));
            s.setJitterOffsetX(jitter.x);
            s.setJitterOffsetY(jitter.y);
            s.setDepthReversed(true);
            s.setShouldResetHistory(reset || self.fresh);
            matrices(s, view.world_from_view.to_matrix().inverse(), view.clip_from_view);
            s.encodeToCommandBuffer(cb);
            let blit = cb.blitCommandEncoder().ok_or("no blit encoder")?;
            let o = MTLOrigin { x: 0, y: 0, z: 0 };
            let size = MTLSize { width: self.out_size.x as _, height: self.out_size.y as _, depth: 1 };
            blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(&out, 0, 0, o, size, &color, 0, 0, o);
            blit.endEncoding();
        }
        self.fresh = false;
        Ok(())
    }
}

/// worldToViewMatrix and viewToClipMatrix, simd_float4x4 properties objc2-metal-fx leaves out (no simd types in
/// objc2): sent by hand, as four float32x4 columns (an aggregate of four vectors, passed as simd_float4x4 is). Only on
/// Apple silicon, and only where the scaler has them; objc2 checks a message's types in a debug build and doesn't know
/// simd's, so a debug build leaves them out.
#[allow(unused_variables)]
fn matrices(s: &ProtocolObject<dyn MTLFXTemporalDenoisedScaler>, world_to_view: Mat4, view_to_clip: Mat4) {
    #[cfg(all(target_arch = "aarch64", not(debug_assertions)))]
    unsafe {
        use objc2::{msg_send, sel};
        use std::arch::aarch64::float32x4_t;
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Float4x4([float32x4_t; 4]);
        unsafe impl objc2::Encode for Float4x4 {
            const ENCODING: objc2::Encoding = objc2::Encoding::Struct("?", &[]);
        }
        let m = |m: Mat4| Float4x4(m.to_cols_array_2d().map(|c| std::mem::transmute::<[f32; 4], float32x4_t>(c)));
        let has: bool = msg_send![s, respondsToSelector: sel!(setWorldToViewMatrix:)];
        if has {
            let _: () = msg_send![s, setWorldToViewMatrix: m(world_to_view)];
            let _: () = msg_send![s, setViewToClipMatrix: m(view_to_clip)];
        }
    }
}

type Viewed = (&'static ViewTarget, &'static ViewPrepassTextures, &'static ExtractedView, &'static ExtractedCamera, &'static SolariLighting, Option<&'static MainPassResolutionOverride>, Option<&'static TemporalJitter>, Has<super::upscale::Upscaled>);

/// Before the forward meshes: a view at its full size, or one drawn small when RAEA_SOLARI_UPSCALE=two (its corner).
fn denoise(view: ViewQuery<Viewed>, device: Res<RenderDevice>, mut kit: Local<Kit>, mut ctx: RenderContext) {
    let entity = view.entity();
    let item = view.into_inner();
    if item.7 && one_pass() {
        return;
    }
    run(entity, item, &device, &mut kit, &mut ctx, false);
}

/// After the main pass: a view drawn small, denoised from its corner out to its whole size.
fn denoise_up(view: ViewQuery<Viewed>, device: Res<RenderDevice>, mut kit: Local<Kit>, mut ctx: RenderContext) {
    let entity = view.entity();
    let item = view.into_inner();
    if !item.7 || !one_pass() {
        return;
    }
    run(entity, item, &device, &mut kit, &mut ctx, true);
}

type Item<'a> = (&'a ViewTarget, &'a ViewPrepassTextures, &'a ExtractedView, &'a ExtractedCamera, &'a SolariLighting, Option<&'a MainPassResolutionOverride>, Option<&'a TemporalJitter>, bool);

/// The guides from the G-buffer and the emissive out, at the main pass's size; MetalFX's denoiser, out to the whole
/// view when `up`; its output over the colour and the emissive back in.
fn run(entity: Entity, item: Item, device: &RenderDevice, kit: &mut Kit, ctx: &mut RenderContext, up: bool) {
    if kit.off {
        return;
    }
    let (target, pre, ext, cam, solari, part, jitter, _) = item;
    let (Some(depth), Some(motion), Some(gbuf)) = (pre.depth.as_ref(), pre.motion_vectors.as_ref(), pre.deferred.as_ref()) else { return };
    if target.main_texture_format() != TextureFormat::Rgba16Float {
        return;
    }
    let color: &wgpu::Texture = target.main_texture();
    let full = UVec2::new(color.width(), color.height());
    let size = part.map_or(full, |p| p.0);
    let out_size = if up { full } else { size };
    let jitter = jitter.map_or(Vec2::ZERO, |j| -j.offset * super::upscale::jitter_gain(true, true));
    let dev = device.wgpu_device().clone();
    if kit.views.get(&entity).map(|v| (v.size, v.out_size)) != Some((size, out_size)) {
        match View::new(&dev, size, out_size) {
            Ok(v) => {
                kit.views.insert(entity, v);
            }
            Err(e) => {
                kit.off = true;
                eprintln!("raea: <SolariLighting>: no MetalFX denoiser ({e}); the lighting draws un-denoised (docs/raytracing.feature RT4)");
                return;
            }
        }
    }
    let (pipeline, restore, restore_taa, hist_sampler) = kit
        .pipeline
        .get_or_insert_with(|| {
            let source = format!("{DECODE_WGSL}{}", super::emit_taa::WGSL);
            let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("metalfx guides"), source: wgpu::ShaderSource::Wgsl(source.into()) });
            let make = |entry: &str| dev.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some("metalfx guides"), layout: None, module: &module, entry_point: Some(entry), compilation_options: Default::default(), cache: None });
            (make("main"), make("restore"), make("restore_taa"), super::emit_taa::sampler(&dev))
        })
        .clone();
    let color_view = color.create_view(&Default::default());
    let back = ext.world_from_view.back();
    let reset = solari.reset || kit.views.get(&entity).is_none_or(|v| v.fresh);
    let words: [f32; 16] = [cam.exposure, if split() { 1.0 } else { 0.0 }, if reset { 1.0 } else { 0.0 }, super::emit_taa::debug(), out_size.x as f32, out_size.y as f32, size.x as f32, size.y as f32, back.x, back.y, back.z, super::emit_taa::sharp(), jitter.x, jitter.y, 0.0, 0.0];
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_ne_bytes()).collect();
    let params = wgpu::util::DeviceExt::create_buffer_init(&dev, &wgpu::util::BufferInitDescriptor { label: Some("metalfx params"), contents: &bytes, usage: wgpu::BufferUsages::UNIFORM });
    let v = kit.views.get_mut(&entity).unwrap();
    let views = [&v.diffuse, &v.specular, &v.normal, &v.rough].map(|t| t.create_view(&Default::default()));
    let mut entries = vec![wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&*gbuf.texture.default_view) }];
    for (i, tv) in views.iter().enumerate() {
        entries.push(wgpu::BindGroupEntry { binding: i as u32 + 1, resource: wgpu::BindingResource::TextureView(tv) });
    }
    entries.push(wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&color_view) });
    entries.push(wgpu::BindGroupEntry { binding: 6, resource: params.as_entire_binding() });
    let emit = v.emit.create_view(&Default::default());
    entries.push(wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&emit) });
    let mask = v.mask.create_view(&Default::default());
    entries.push(wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&mask) });
    let bind = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("metalfx guides"), layout: &pipeline.get_bind_group_layout(0), entries: &entries });
    {
        let mut pass = ctx.command_encoder().begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("metalfx guides"), timestamp_writes: None });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(size.x.div_ceil(8), size.y.div_ceil(8), 1);
    }
    // a command buffer of its own (wgpu won't mix its own commands and raw ones on one encoder), after the guides'
    let mut raw = dev.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("metalfx denoise") });
    let done = unsafe {
        raw.as_hal_mut::<Metal, _, _>(|e| match e.and_then(|e| e.raw_command_buffer()) {
            Some(cb) => v.encode(cb, color, &depth.texture.texture, &motion.texture.texture, ext, jitter, solari.reset),
            None => Err("wgpu gave no Metal command buffer"),
        })
    };
    ctx.add_command_buffer(raw.finish());
    // the emissive back in, after the denoiser's command buffer (add_command_buffer flushed the encoder before it)
    if split() {
    let mut entries = vec![
        wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&color_view) },
        wgpu::BindGroupEntry { binding: 6, resource: params.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&emit) },
    ];
    let taa_views = v.taa.as_ref().map(|h| h.views(&depth.texture.texture, &motion.texture.texture));
    if let Some(views) = &taa_views {
        entries.extend(super::emit_taa::entries(views, &hist_sampler));
    }
    let put = if taa_views.is_some() { &restore_taa } else { &restore };
    let back = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("metalfx emissive back"), layout: &put.get_bind_group_layout(0), entries: &entries });
    if let Some(h) = v.taa.as_mut() {
        h.advance();
    }
    {
        let mut pass = ctx.command_encoder().begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("metalfx emissive back"), timestamp_writes: None });
        pass.set_pipeline(put);
        pass.set_bind_group(0, &back, &[]);
        pass.dispatch_workgroups(out_size.x.div_ceil(8), out_size.y.div_ceil(8), 1);
    }
    }
    use std::sync::atomic::Ordering::Relaxed;
    super::upscale::DENOISED.store(done.is_ok(), Relaxed);
    super::upscale::DENOISING.store(done.is_ok(), Relaxed);
    if up {
        super::upscale::BROUGHT_UP.store(done.is_ok(), Relaxed);
    }
    if let Err(e) = done {
        kit.off = true;
        eprintln!("raea: <SolariLighting>: the MetalFX denoiser stopped ({e}); the lighting draws un-denoised");
    }
}
