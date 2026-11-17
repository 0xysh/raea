//! The bar's numbers (docs/realism.feature LT1): how far a frame's light is from its reference, in stops, and how much
//! a moving shot flickers.
//!
//! Both read the frames as they're delivered (8-bit sRGB after tone mapping), each pixel's luminance made linear
//! again (Rec. 709 weights). A stop is a factor of two in that luminance; pixels darker than FLOOR are compared at
//! FLOOR, so the noise in near-black doesn't count as error.

use std::path::Path;

/// A frame's linear luminance, row by row, and its size.
pub struct Luma {
    pub w: usize,
    pub h: usize,
    pub y: Vec<f32>,
}

/// The darkest luminance a stop is measured from (about sRGB 8/255).
const FLOOR: f32 = 0.0025;
/// The side of the squares the worst tile is found among, in pixels.
pub const TILE: usize = 64;

fn linear(c: u8) -> f32 {
    let v = c as f32 / 255.0;
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

pub fn load(p: &Path) -> Result<Luma, String> {
    let img = image::open(p).map_err(|e| format!("{}: {e}", p.display()))?.to_rgb8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let lut: Vec<f32> = (0..=255u8).map(linear).collect();
    let y = img.pixels().map(|px| 0.2126 * lut[px[0] as usize] + 0.7152 * lut[px[1] as usize] + 0.0722 * lut[px[2] as usize]).collect();
    Ok(Luma { w, h, y })
}

/// A frame against its reference: the mean distance in stops, the mean signed (brighter is positive), and the worst
/// tile's mean distance with where that tile is (its column and row, in tiles).
#[derive(Debug, Clone, Copy)]
pub struct Distance {
    pub mean: f32,
    pub bias: f32,
    pub worst: f32,
    pub worst_at: (usize, usize),
}

pub fn distance(a: &Luma, reference: &Luma) -> Result<Distance, String> {
    if (a.w, a.h) != (reference.w, reference.h) {
        return Err(format!("sizes differ: {}x{} against the reference's {}x{}", a.w, a.h, reference.w, reference.h));
    }
    let (tw, th) = (a.w.div_ceil(TILE), a.h.div_ceil(TILE));
    let mut tiles = vec![(0.0f64, 0usize); tw * th];
    let (mut sum, mut signed) = (0.0f64, 0.0f64);
    for (i, (&p, &r)) in a.y.iter().zip(&reference.y).enumerate() {
        let d = (p.max(FLOOR) / r.max(FLOOR)).log2() as f64;
        sum += d.abs();
        signed += d;
        let t = &mut tiles[(i / a.w / TILE) * tw + (i % a.w) / TILE];
        t.0 += d.abs();
        t.1 += 1;
    }
    let n = a.y.len().max(1) as f64;
    let (k, worst) = tiles.iter().enumerate().map(|(k, (s, c))| (k, s / (*c).max(1) as f64)).fold((0, 0.0), |b, t| if t.1 > b.1 { t } else { b });
    Ok(Distance { mean: (sum / n) as f32, bias: (signed / n) as f32, worst: worst as f32, worst_at: (k % tw, k / tw) })
}

/// Flicker over a moving shot: each pixel's second difference in time (this frame against the mean of the frames
/// either side, so steady motion and steady fades count for nothing), averaged, in 8-bit sRGB steps of luminance.
/// The frames in order; fewer than three measure nothing.
pub fn flicker(frames: &[Luma]) -> Option<f32> {
    if frames.len() < 3 {
        return None;
    }
    let srgb = |v: f32| if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 } * 255.0;
    let mut total = 0.0f64;
    for t in 1..frames.len() - 1 {
        let (p, c, n) = (&frames[t - 1].y, &frames[t].y, &frames[t + 1].y);
        let s: f64 = c.iter().zip(p).zip(n).map(|((&c, &p), &n)| (2.0 * srgb(c) - srgb(p) - srgb(n)).abs() as f64).sum();
        total += s / c.len().max(1) as f64;
    }
    Some((total / (frames.len() - 2) as f64) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(v: f32) -> Luma {
        Luma { w: 128, h: 64, y: vec![v; 128 * 64] }
    }

    #[test]
    fn a_frame_twice_as_bright_is_one_stop_brighter() {
        let d = distance(&flat(0.4), &flat(0.2)).unwrap();
        assert!((d.mean - 1.0).abs() < 1e-5 && (d.bias - 1.0).abs() < 1e-5 && (d.worst - 1.0).abs() < 1e-5);
    }

    #[test]
    fn the_worst_tile_is_found() {
        let mut a = flat(0.2);
        for y in 0..64 {
            for x in 64..128 {
                a.y[y * 128 + x] = 0.1;
            }
        }
        let d = distance(&a, &flat(0.2)).unwrap();
        assert_eq!(d.worst_at, (1, 0));
        assert!((d.worst - 1.0).abs() < 1e-5 && (d.mean - 0.5).abs() < 1e-5 && (d.bias + 0.5).abs() < 1e-5);
    }

    #[test]
    fn a_steady_fade_doesnt_flicker_and_a_blink_does() {
        let fade: Vec<Luma> = (0..5).map(|k| flat(0.1 + 0.02 * k as f32)).collect();
        let blink: Vec<Luma> = (0..5).map(|k| flat(if k % 2 == 0 { 0.1 } else { 0.2 })).collect();
        assert!(flicker(&fade).unwrap() < 1.0);
        assert!(flicker(&blink).unwrap() > 20.0);
    }
}
