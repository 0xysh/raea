//! The maps as PNGs, dilated over the gutters: each pass gives every empty texel the mean of its covered neighbours,
//! so a bilinear (or mipped) read at a chart's edge reads the chart and not the black between charts. GUTTER passes
//! fill a chart's own gutter from its own texels; charts are 2 * GUTTER apart, so they don't bleed into each other.

use std::path::Path;

use super::R;

pub(crate) fn dilate(m: &mut [[u8; 4]], covered: &mut [bool], w: u32, h: u32, passes: u32) {
    let (w, h) = (w as i64, h as i64);
    for _ in 0..passes {
        let mut fill: Vec<(usize, [u8; 4])> = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                if covered[i] {
                    continue;
                }
                let (mut s, mut k) = ([0u32; 4], 0u32);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (xx, yy) = (x + dx, y + dy);
                        if (dx, dy) == (0, 0) || xx < 0 || yy < 0 || xx >= w || yy >= h {
                            continue;
                        }
                        let j = (yy * w + xx) as usize;
                        if covered[j] {
                            for c in 0..4 {
                                s[c] += m[j][c] as u32;
                            }
                            k += 1;
                        }
                    }
                }
                if k > 0 {
                    fill.push((i, s.map(|v| ((v + k / 2) / k) as u8)));
                }
            }
        }
        for (i, v) in fill {
            m[i] = v;
            covered[i] = true;
        }
    }
}

pub(crate) fn png(p: &Path, m: &[[u8; 4]], w: u32, h: u32) -> R<()> {
    let raw: Vec<u8> = m.iter().flatten().copied().collect();
    let img = image::RgbaImage::from_raw(w, h, raw).ok_or("a map whose size isn't its texels")?;
    img.save(p).map_err(|e| format!("{}: {e}", p.display()))
}
