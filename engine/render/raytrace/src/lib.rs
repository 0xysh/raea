//! raea-render's ray tracing (docs/raytracing.feature): Bevy's Solari for a scene whose camera asks for it (RT3,
//! solari.rs), MetalFX's temporal denoiser on its lighting on the Mac (RT4, metalfx.rs), and its temporal scaler for a
//! view drawn below its size (RT9, upscale.rs). raea-render brings this
//! crate in under its `raytracing` feature and adds it to an engine only when the program asks.

use bevy::prelude::*;
use std::collections::HashMap;

mod solari;
#[cfg(target_os = "macos")]
mod metalfx;
#[cfg(target_os = "macos")]
mod emit_taa;
#[cfg(target_os = "macos")]
mod upscale;
// RAEA_LIGHT: one light path for every camera (docs/realism.feature LT1)
pub mod force;

/// RT9: the fraction of the view's size a side this camera's main pass draws at (a third to 1), which MetalFX's
/// temporal scaler brings up to the whole view after it (upscale.rs); 1 draws at the full size. <SolariLighting scale>
/// sets it. On the Mac only: elsewhere the view draws at its full size and raea says so once.
#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq)]
#[reflect(Component, Default)]
pub struct RenderScale {
    pub scale: f32,
}

impl Default for RenderScale {
    fn default() -> Self {
        RenderScale { scale: 1.0 }
    }
}

/// Solari's plugins, and RAEA_LIGHT's camera override when it names a path.
pub fn add(app: &mut App) {
    app.register_type::<RenderScale>();
    solari::add(app);
    force::add(app);
}

/// The program asks for Solari (a camera's SolariLighting or Pathtracer component; `prog` is raea-render's Program),
/// or RAEA_LIGHT forces it.
pub fn wanted(prog: &impl std::fmt::Debug) -> bool {
    force::needs_solari() || format!("{prog:?}").contains("bevy_solari::")
}

/// After app.finish(): MetalFX's denoiser after Solari's lighting and its scaler after the main pass, on the Mac;
/// elsewhere a RenderScale below 1 is said once to be ignored.
pub fn add_denoiser(app: &mut App) {
    #[cfg(target_os = "macos")]
    {
        metalfx::add(app);
        upscale::add(app);
    }
    #[cfg(not(target_os = "macos"))]
    app.add_systems(PostUpdate, |q: Query<&RenderScale>, mut said: Local<bool>| {
        if !*said && q.iter().any(|r| r.scale < 0.999) {
            *said = true;
            eprintln!("raea: <SolariLighting scale>: no scaler off the Mac; the view draws at its full size (docs/raytracing.feature RT9)");
        }
    });
}

/// The path tracer starts its accumulation again on the next draw (a reference frame of its own, RAEA_LIGHT_SPP,
/// docs/realism.feature LT1; it starts again by itself only when the camera moves).
pub fn restart_pathtracer(world: &mut World) {
    for mut p in world.query::<&mut bevy::solari::pathtracer::Pathtracer>().iter_mut(world) {
        p.reset = true;
    }
}

/// A cut to another camera (docs/rerender.feature RR2): Solari's and the path tracer's history start again, as on their
/// first frame, so the last shot's light isn't carried into this one (Bevy's `reset`, for "sudden camera cuts").
pub fn cut(world: &mut World) {
    for mut s in world.query::<&mut bevy::solari::prelude::SolariLighting>().iter_mut(world) {
        s.reset = true;
    }
    for mut p in world.query::<&mut bevy::solari::pathtracer::Pathtracer>().iter_mut(world) {
        p.reset = true;
    }
}
