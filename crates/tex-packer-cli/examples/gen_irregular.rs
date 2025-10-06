use image::{Rgba, RgbaImage};
use rand::{Rng, SeedableRng};
use std::fs;
use std::path::PathBuf;

// simple drawing helpers
fn draw_border_full(img: &mut RgbaImage, color: [u8; 4]) {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return;
    }
    for x in 0..w {
        img.put_pixel(x, 0, Rgba(color));
        img.put_pixel(x, h - 1, Rgba(color));
    }
    for y in 0..h {
        img.put_pixel(0, y, Rgba(color));
        img.put_pixel(w - 1, y, Rgba(color));
    }
}

const FONT_3X5: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];

fn draw_char_scaled(img: &mut RgbaImage, x: u32, y: u32, ch: char, color: [u8; 4], scale: u32) {
    if scale == 0 {
        return;
    }
    if let Some(d) = ch.to_digit(10) {
        let glyph = FONT_3X5[d as usize];
        for (row_i, row) in glyph.iter().enumerate() {
            for col in 0..3 {
                if (row >> (2 - col)) & 1 == 1 {
                    let px0 = x + col * scale;
                    let py0 = y + (row_i as u32) * scale;
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let px = px0 + dx;
                            let py = py0 + dy;
                            if px < img.width() && py < img.height() {
                                img.put_pixel(px, py, Rgba(color));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn draw_text_scaled(img: &mut RgbaImage, x: u32, y: u32, s: &str, color: [u8; 4], scale: u32) {
    let mut cx = x;
    for ch in s.chars() {
        draw_char_scaled(img, cx, y, ch, color, scale);
        cx += (3 * scale + scale);
    }
}

fn draw_text_centered_scaled(
    img: &mut RgbaImage,
    cx: u32,
    cy: u32,
    s: &str,
    color: [u8; 4],
    outline: bool,
) {
    let w = img.width();
    let h = img.height();
    if w == 0 || h == 0 {
        return;
    }
    let len = s.chars().count().max(1) as u32;
    let target_w = (w as f32 * 0.7).max(1.0);
    let target_h = (h as f32 * 0.7).max(1.0);
    let scale_w = (target_w / (3.0 * len as f32 + (len as f32 - 1.0))).floor() as u32;
    let scale_h = (target_h / 5.0).floor() as u32;
    let mut scale = scale_w.min(scale_h).max(1);
    if w.min(h) <= 16 {
        scale = scale.max(2);
    }
    let text_w = len * (3 * scale + scale) - scale;
    let text_h = 5 * scale;
    let x0 = cx.saturating_sub(text_w / 2);
    let y0 = cy.saturating_sub(text_h / 2);
    if outline {
        let ocol = [0, 0, 0, 255];
        let offs: &[(i32, i32)] = &[(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (ox, oy) in offs.iter().cloned() {
            let bx = (x0 as i32 + ox).max(0) as u32;
            let by = (y0 as i32 + oy).max(0) as u32;
            draw_text_scaled(img, bx, by, s, ocol, scale);
        }
    }
    draw_text_scaled(img, x0, y0, s, color, scale);
}

// Generate a set of irregular RGBA textures to a directory for stress testing.
// Usage: cargo run --example gen_irregular -p tex-packer-cli -- <out_dir> [count]
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        anyhow::bail!("usage: gen_irregular <out_dir> [count]");
    }
    let out = PathBuf::from(&args[0]);
    let count: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(200);
    fs::create_dir_all(&out)?;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0xC0FFEE);
    for i in 0..count {
        let w: u32 = rng.gen_range(16..=256);
        let h: u32 = rng.gen_range(16..=256);
        let mut img = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
        // Random blotches: draw several rectangles/circles with varying alpha to create irregular shapes
        let blotches = rng.gen_range(3..=10);
        for _ in 0..blotches {
            let cx = rng.gen_range(0..w);
            let cy = rng.gen_range(0..h);
            let rw = rng.gen_range(4..=w.max(4).min(64));
            let rh = rng.gen_range(4..=h.max(4).min(64));
            let color = Rgba([rng.gen(), rng.gen(), rng.gen(), rng.gen_range(96..=255)]);
            // Mix between rect and ellipse
            if rng.gen_bool(0.5) {
                let x1 = cx.saturating_sub(rw / 2);
                let y1 = cy.saturating_sub(rh / 2);
                let x2 = (cx + rw / 2).min(w - 1);
                let y2 = (cy + rh / 2).min(h - 1);
                for y in y1..=y2 {
                    for x in x1..=x2 {
                        img.put_pixel(x, y, color);
                    }
                }
            } else {
                let rx = (rw as f32) / 2.0;
                let ry = (rh as f32) / 2.0;
                for y in 0..h {
                    for x in 0..w {
                        let dx = (x as i32 - cx as i32) as f32;
                        let dy = (y as i32 - cy as i32) as f32;
                        let v = (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry);
                        if v <= 1.0 {
                            img.put_pixel(x, y, color);
                        }
                    }
                }
            }
        }
        draw_border_full(&mut img, [0, 0, 0, 255]);
        let label = format!("{}", i);
        draw_text_centered_scaled(&mut img, w / 2, h / 2, &label, [255, 255, 255, 255], true);
        let path = out.join(format!("irregular_{:04}.png", i));
        img.save(&path)?;
        if i % 50 == 0 {
            println!("wrote {:?}", path.file_name().unwrap());
        }
    }
    println!("Done. Wrote {} images to {}", count, out.display());
    Ok(())
}
