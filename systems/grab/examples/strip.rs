//! A filmstrip of a take, as an SVG of stick figures, to judge the motion by eye with no engine: the stand-in body
//! takes a box with both hands and throws it (top rows), and moves a cup with one (bottom rows). Each take is drawn
//! from the front quarter and from the side.
//!   cargo run -p raea-sys-grab --example strip -- out.svg

use std::fmt::Write;

use glam::{Quat, Vec3};
use raea_sys_body::*;
use raea_sys_grab::*;

/// a point of the body's frame on the page: turned about the vertical by `yaw`, y up
fn project(p: Vec3, yaw: f32, ox: f32, oy: f32) -> (f32, f32) {
    let q = Quat::from_rotation_y(yaw) * p;
    (ox + q.x * 150.0, oy - q.y * 150.0)
}

fn draw(svg: &mut String, s: &Skeleton, f: &Frame, yaw: f32, ox: f32, oy: f32) {
    let w = s.fk(&f.pose);
    let line = |svg: &mut String, a: Vec3, b: Vec3, colour: &str, width: f32| {
        let ((x1, y1), (x2, y2)) = (project(a, yaw, ox, oy), project(b, yaw, ox, oy));
        let _ = writeln!(svg, r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{colour}" stroke-width="{width}" stroke-linecap="round"/>"#);
    };
    // the thing: its box, or the box round it, by its own axes
    let half = match f.thing.solid {
        Solid::Ball { r } => Vec3::splat(r),
        Solid::Box { half } => half,
        Solid::Capsule { half, r } => Vec3::new(r, half + r, r),
        Solid::Cylinder { half, r } => Vec3::new(r, half, r),
    };
    let corner = |i: usize| f.thing.world(Vec3::new(if i & 1 == 0 { -half.x } else { half.x }, if i & 2 == 0 { -half.y } else { half.y }, if i & 4 == 0 { -half.z } else { half.z }));
    for i in 0..8 {
        for bit in [1, 2, 4] {
            if i & bit == 0 {
                line(svg, corner(i), corner(i | bit), "#c2410c", 1.5);
            }
        }
    }
    for (i, j) in s.joints.iter().enumerate() {
        let Some(p) = j.parent else { continue };
        let finger = i >= s.joint("thumb_01_l").unwrap_or(usize::MAX);
        let colour = if j.name.ends_with("_l") { "#1d4ed8" } else if j.name.ends_with("_r") { "#15803d" } else { "#111827" };
        line(svg, w.pos[p], w.pos[i], colour, if finger { 1.0 } else { 2.5 });
        if finger || j.name.starts_with("hand") || j.name == "head" {
            line(svg, w.pos[i], w.pos[i] + w.rot[i] * (j.tail - j.head), colour, if finger { 1.0 } else { 2.5 });
        }
    }
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "strip.svg".into());
    let s = mannequin();
    let stand = |_: f32| s.rest();

    let crate_ = Thing::new(Solid::Box { half: Vec3::new(0.16, 0.1, 0.12) }, Vec3::new(0.0, 1.05, 0.42));
    let mut throw = Take::new(crate_, find_holds(&crate_, s.rest().root, HoldOptions::default()), 0.2);
    throw.release = Some(Release::Throw { at: 1.9, velocity: Vec3::new(0.0, 1.5, 2.5), floor: 0.0 });

    let cup = Thing::new(Solid::Cylinder { half: 0.05, r: 0.035 }, Vec3::new(-0.2, 1.0, 0.38));
    let mut put = Take::new(cup, find_holds(&cup, s.rest().root, HoldOptions::default()), 0.2);
    put.release = Some(Release::Put { at: 1.7, down: 2.3, place: Vec3::new(0.05, 1.0, 0.4) });

    let times = [0.0, 0.45, 0.65, 0.8, 1.05, 1.3, 1.6, 1.9, 2.05, 2.3, 2.7];
    let (cell, row) = (190.0, 310.0);
    let mut svg = format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" font-family="monospace" font-size="11"><rect width="100%" height="100%" fill="#fafaf9"/>"##, cell * times.len() as f32, row * 4.0);
    for (r, (take, yaw, name)) in [(&throw, -0.6, "box, front quarter"), (&throw, -1.5708, "box, side"), (&put, -0.6, "cup, front quarter"), (&put, -1.5708, "cup, side")].into_iter().enumerate() {
        for (c, t) in times.iter().enumerate() {
            let f = perform(&s, &stand, take, *t);
            let (ox, oy) = (cell * (c as f32 + 0.5), row * (r as f32 + 1.0) - 14.0);
            draw(&mut svg, &s, &f, yaw, ox, oy);
            let _ = writeln!(svg, r##"<text x="{:.0}" y="{:.0}" fill="#44403c">{name} t={t:.2} {:?}</text><text x="{:.0}" y="{:.0}" fill="#78716c">miss {:.1} mm, moved {:.0} deg</text>"##, ox - cell / 2.0 + 6.0, oy - row + 30.0, f.phase, ox - cell / 2.0 + 6.0, oy - row + 44.0, f.miss * 1000.0, f.moved);
        }
    }
    svg.push_str("</svg>");
    std::fs::write(&out, svg).expect("the strip written");
    println!("{out}");
}
