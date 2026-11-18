//! LightBench (docs/lightbench.feature): a small Rust + Bevy renderer that proves lighting and geometry techniques
//! against its own all-lights reference before they move into raea. `lightbench --help` lists the switches; with
//! `--shot FILE` it draws headless, saves frame `--frames` as a PNG and exits, so every claim is a repeatable run.

mod bvh;
mod pass;
mod scene;

use bevy::{
    camera::{CameraMainTextureUsages, Hdr, RenderTarget},
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass},
    pbr::DefaultOpaqueRendererMethod,
    prelude::*,
    render::{
        render_resource::{TextureFormat, TextureUsages},
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    window::ExitCondition,
};

const HELP: &str = "lightbench [switches]
  --mode all|uniform|weighted   every light (the reference), or K chosen uniformly or by their light (all)
  --k K                         light samples a pixel, 1 to 16 (4)
  --lights N                    point lights, the pillar's first (16)
  --view lit|normal|albedo|depth|rays   what to show (lit)
  --brute                       test every triangle for each shadow ray instead of the BVH
  --detail N                    the knot's segments; its triangles are N * N / 16 (2048)
  --size WxH                    pixels (1280x720)
  --t SECONDS                   hold the lights where they are at this time (they move otherwise)
  --exposure X                  linear exposure (1)
  --shot FILE                   draw headless, save frame --frames as a PNG, exit
  --frames N                    the lit frame --shot saves (4)
  --bench N                     with --shot: time N more lit frames first and print the mean (frame time, its own run)
  --no-pillar                   leave out the pillar outside the view (its shadow should go with it)
  --raw                         no temporal accumulation: each frame's samples alone
  --history N                   frames the history holds at most (32)
  --camera still|orbit|cut      the camera holds, circles slowly, or jumps to a second view at lit frame --cut-at (still)
  --cut-at N                    the lit frame --camera cut jumps on (8)
  --view history                frames of history at each pixel, white when full";

#[derive(Resource, Clone)]
pub struct Opts {
    pub mode: u32,
    pub k: u32,
    pub lights: u32,
    pub view: u32,
    pub brute: bool,
    pub detail: u32,
    pub size: UVec2,
    pub t: Option<f32>,
    pub exposure: f32,
    pub shot: Option<String>,
    pub frames: u32,
    pub bench: u32,
    pub pillar: bool,
    pub raw: bool,
    pub history: u32,
    /// 0 still, 1 orbit, 2 cut
    pub camera: u32,
    pub cut_at: u32,
}

fn opts() -> Result<Opts, String> {
    let mut o = Opts { mode: 0, k: 4, lights: 16, view: 0, brute: false, detail: 2048, size: UVec2::new(1280, 720), t: None, exposure: 1.0, shot: None, frames: 4, bench: 0, pillar: true, raw: false, history: 32, camera: 0, cut_at: 8 };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or(format!("{a} needs a value"));
        let num = |v: String| v.parse::<f32>().map_err(|_| format!("{a}: {v} isn't a number"));
        match a.as_str() {
            "--mode" => {
                let v = val()?;
                o.mode = ["all", "uniform", "weighted"].iter().position(|m| *m == v).ok_or("--mode: all, uniform or weighted")? as u32;
            }
            "--view" => {
                let v = val()?;
                o.view = ["lit", "normal", "albedo", "depth", "rays", "history"].iter().position(|m| *m == v).ok_or("--view: lit, normal, albedo, depth, rays or history")? as u32;
            }
            "--k" => o.k = (num(val()?)? as u32).clamp(1, 16),
            "--lights" => o.lights = (num(val()?)? as u32).max(1),
            "--brute" => o.brute = true,
            "--detail" => o.detail = (num(val()?)? as u32).max(64),
            "--size" => {
                let v = val()?;
                let (w, h) = v.split_once('x').ok_or("--size: WxH")?;
                o.size = UVec2::new(num(w.into())? as u32, num(h.into())? as u32);
            }
            "--t" => o.t = Some(num(val()?)?),
            "--exposure" => o.exposure = num(val()?)?,
            "--shot" => o.shot = Some(val()?),
            "--frames" => o.frames = (num(val()?)? as u32).max(1),
            "--bench" => o.bench = num(val()?)? as u32,
            "--no-pillar" => o.pillar = false,
            "--raw" => o.raw = true,
            "--history" => o.history = (num(val()?)? as u32).max(1),
            "--cut-at" => o.cut_at = num(val()?)? as u32,
            "--camera" => {
                let v = val()?;
                o.camera = ["still", "orbit", "cut"].iter().position(|m| *m == v).ok_or("--camera: still, orbit or cut")? as u32;
            }
            "--help" | "-h" => return Err(HELP.into()),
            _ => return Err(format!("unknown switch {a}\n{HELP}")),
        }
    }
    Ok(o)
}

/// Whether this run times its frames (--bench), read before the app is built.
pub fn timing() -> bool {
    std::env::args().any(|a| a == "--bench")
}

/// The image a headless run draws into.
#[derive(Resource)]
struct Target(Handle<Image>);

fn main() {
    let o = match opts() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(if e == HELP { 0 } else { 2 });
        }
    };
    let mut app = App::new();
    if o.shot.is_some() {
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, ..default() })
                .disable::<bevy::winit::WinitPlugin>(),
        )
        .add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(std::time::Duration::ZERO));
    } else {
        let window = Window { title: "LightBench".into(), resolution: (o.size.x, o.size.y).into(), ..default() };
        app.add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(window), ..default() }));
    }
    app.insert_resource(o)
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .insert_resource(ClearColor(Color::srgb(0.02, 0.025, 0.035)))
        .add_plugins(pass::LightPass)
        .add_systems(Startup, (scene::spawn, camera))
        .add_systems(Update, (tick, shoot))
        .run();
}

fn camera(mut commands: Commands, o: Res<Opts>, mut images: ResMut<Assets<Image>>) {
    let usages = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC | TextureUsages::STORAGE_BINDING;
    let mut cam = commands.spawn((
        Camera3d::default(),
        Hdr,
        Msaa::Off,
        DepthPrepass,
        DeferredPrepass,
        CameraMainTextureUsages(usages),
        Projection::Perspective(PerspectiveProjection { fov: 45f32.to_radians(), ..default() }),
        eye(&o, 0),
    ));
    if o.shot.is_some() {
        let mut image = Image::new_target_texture(o.size.x, o.size.y, TextureFormat::Rgba8UnormSrgb, None);
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let handle = images.add(image);
        cam.insert(RenderTarget::Image(handle.clone().into()));
        commands.insert_resource(Target(handle));
    }
}

const EYE: Vec3 = Vec3::new(0.0, 2.2, 7.0);
const LOOK: Vec3 = Vec3::new(0.0, 1.3, 0.0);

/// Where the camera is on lit frame `n`: held, circling slowly, or cut to a second view at --cut-at.
fn eye(o: &Opts, n: u32) -> Transform {
    match o.camera {
        1 => {
            let a = n as f32 * 0.004;
            Transform::from_translation(LOOK + Quat::from_rotation_y(a) * (EYE - LOOK)).looking_at(LOOK, Vec3::Y)
        }
        2 if n >= o.cut_at => Transform::from_xyz(4.5, 3.2, 4.5).looking_at(LOOK, Vec3::Y),
        _ => Transform::from_translation(EYE).looking_at(LOOK, Vec3::Y),
    }
}

/// The camera and the lights where they are this frame, and the switches, for the render world. A cut is a jump of
/// more than a metre or 20 degrees in one frame: the history starts again.
fn tick(o: Res<Opts>, time: Res<Time>, mut frame: ResMut<pass::Frame>, mut cam: Query<&mut Transform, With<Camera3d>>) {
    let t = o.t.unwrap_or(time.elapsed_secs());
    let n = pass::DRAWN.load(std::sync::atomic::Ordering::Relaxed);
    let mut cut = false;
    if let Ok(mut at) = cam.single_mut() {
        let to = eye(&o, n);
        cut = at.translation.distance(to.translation) > 1.0 || at.rotation.angle_between(to.rotation) > 20f32.to_radians();
        *at = to;
    }
    *frame = pass::Frame {
        lights: scene::lights(o.lights, t),
        mode: o.mode,
        k: o.k,
        view: o.view,
        frame: frame.frame.wrapping_add(1),
        exposure: o.exposure,
        brute: o.brute,
        raw: o.raw,
        cut,
        history: o.history,
    };
}

fn shoot(mut commands: Commands, o: Res<Opts>, target: Option<Res<Target>>, mut done: Local<bool>, mut clock: Local<Option<(std::time::Instant, u32)>>) {
    let (Some(path), Some(target)) = (o.shot.clone(), target) else { return };
    // counted from the lighting pass's first frame, not from startup (its pipeline compiles meanwhile)
    let drawn = pass::DRAWN.load(std::sync::atomic::Ordering::Relaxed);
    if *done || drawn < o.frames + o.bench {
        if drawn >= o.frames && clock.is_none() {
            *clock = Some((std::time::Instant::now(), drawn));
        }
        return;
    }
    *done = true;
    if o.bench > 0 {
        let (t0, d0) = clock.unwrap();
        let ms = t0.elapsed().as_secs_f64() * 1000.0 / (drawn - d0).max(1) as f64;
        println!("FRAME {ms:.2} ms, the GPU waited for ({} frames, {}x{}, {} lights, mode {} k {})", drawn - d0, o.size.x, o.size.y, o.lights, ["all", "uniform", "weighted"][o.mode as usize], o.k);
    }
    commands.spawn(Screenshot::image(target.0.clone())).observe(move |shot: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
        match shot.image.clone().try_into_dynamic() {
            Ok(img) => match img.to_rgb8().save(&path) {
                Ok(()) => println!("SHOT {path}"),
                Err(e) => eprintln!("lightbench: saving {path}: {e}"),
            },
            Err(e) => eprintln!("lightbench: the frame: {e:?}"),
        }
        exit.write(AppExit::Success);
    });
}
