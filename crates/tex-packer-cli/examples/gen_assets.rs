use image::{Rgba, RgbaImage};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::fs;
use std::path::PathBuf;

fn ensure_dir(p: &PathBuf) -> anyhow::Result<()> {
    fs::create_dir_all(p)?;
    Ok(())
}

fn save(img: &RgbaImage, path: &PathBuf) -> anyhow::Result<()> {
    img.save(path)?;
    Ok(())
}

fn solid(w: u32, h: u32, c: [u8; 4]) -> RgbaImage {
    RgbaImage::from_pixel(w, h, Rgba(c))
}

fn random_color_opaque(rng: &mut impl Rng) -> [u8; 4] {
    [rng.gen(), rng.gen(), rng.gen(), 255]
}

fn draw_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, c: [u8; 4]) {
    let (W, H) = img.dimensions();
    for yy in y.min(H)..(y.saturating_add(h)).min(H) {
        for xx in x.min(W)..(x.saturating_add(w)).min(W) {
            img.put_pixel(xx, yy, Rgba(c));
        }
    }
}

fn draw_ellipse(img: &mut RgbaImage, cx: i32, cy: i32, rx: f32, ry: f32, c: [u8; 4]) {
    let (W, H) = img.dimensions();
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let dx = (x - cx) as f32;
            let dy = (y - cy) as f32;
            if (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry) <= 1.0 {
                img.put_pixel(x as u32, y as u32, Rgba(c));
            }
        }
    }
}

fn draw_soft_circle(img: &mut RgbaImage, cx: i32, cy: i32, r: f32, rgb: [u8; 3]) {
    let (W, H) = img.dimensions();
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let dx = (x - cx) as f32;
            let dy = (y - cy) as f32;
            let d = (dx * dx + dy * dy).sqrt();
            if d <= r {
                // soft alpha near edge 0..1
                let edge = (r - d) / (r.max(1.0));
                let a = (edge.clamp(0.0, 1.0) * 255.0) as u8;
                img.put_pixel(x as u32, y as u32, Rgba([rgb[0], rgb[1], rgb[2], a]));
            }
        }
    }
}

// --- simple 3x5 bitmap font for digits '0'..'9' ---
const FONT_3X5: [[u8; 5]; 10] = [
    // each row is 3 bits (MSB left)
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b010, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
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
    // compute scale to fit within ~70% of min dimension
    let target_w = (w as f32 * 0.7).max(1.0);
    let target_h = (h as f32 * 0.7).max(1.0);
    let mut scale_w = (target_w / (3.0 * len as f32 + (len as f32 - 1.0))).floor() as u32;
    let mut scale_h = (target_h / 5.0).floor() as u32;
    let mut scale = scale_w.min(scale_h).max(1);
    // try to make small images still visible
    if w.min(h) <= 16 {
        scale = scale.max(2);
    }
    let text_w = len * (3 * scale + scale) - scale; // last char no trailing space
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

fn draw_text_centered_scaled_in_rect(
    img: &mut RgbaImage,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    s: &str,
    color: [u8; 4],
    outline: bool,
) {
    if w == 0 || h == 0 {
        return;
    }
    let len = s.chars().count().max(1) as u32;
    let target_w = (w as f32 * 0.7).max(1.0);
    let target_h = (h as f32 * 0.7).max(1.0);
    let mut scale_w = (target_w / (3.0 * len as f32 + (len as f32 - 1.0))).floor() as u32;
    let mut scale_h = (target_h / 5.0).floor() as u32;
    let mut scale = scale_w.min(scale_h).max(1);
    if w.min(h) <= 16 {
        scale = scale.max(2);
    }
    let text_w = len * (3 * scale + scale) - scale;
    let text_h = 5 * scale;
    let cx = x + w / 2;
    let cy = y + h / 2;
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

fn draw_border_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: [u8; 4]) {
    if w == 0 || h == 0 {
        return;
    }
    for xx in x..x.saturating_add(w).min(img.width()) {
        if y < img.height() {
            img.put_pixel(xx, y, Rgba(color));
        }
        let by = y.saturating_add(h).saturating_sub(1);
        if by < img.height() {
            img.put_pixel(xx, by, Rgba(color));
        }
    }
    for yy in y..y.saturating_add(h).min(img.height()) {
        if x < img.width() {
            img.put_pixel(x, yy, Rgba(color));
        }
        let rx = x.saturating_add(w).saturating_sub(1);
        if rx < img.width() {
            img.put_pixel(rx, yy, Rgba(color));
        }
    }
}

fn gen_basic_sizes(out: &PathBuf, rng: &mut impl Rng) -> anyhow::Result<()> {
    ensure_dir(out)?;
    for i in 0..120u32 {
        let w = rng.gen_range(16..=164);
        let h = rng.gen_range(16..=164);
        let mut img = solid(w, h, [0, 0, 0, 0]);
        draw_rect(&mut img, 0, 0, w, h, random_color_opaque(rng));
        draw_border_full(&mut img, [0, 0, 0, 255]);
        let label = format!("{}", i);
        draw_text_centered_scaled(&mut img, w / 2, h / 2, &label, [255, 255, 255, 255], true);
        save(&img, &out.join(format!("basic_{:03}.png", i)))?;
    }
    fs::write(
        out.join("README.txt"),
        "Basic opaque rectangles with varied sizes.",
    )?;
    Ok(())
}

fn gen_thin_bars(out: &PathBuf, rng: &mut impl Rng) -> anyhow::Result<()> {
    ensure_dir(out)?;
    for i in 0..80u32 {
        let horiz = rng.gen_bool(0.5);
        let (w, h) = if horiz {
            (rng.gen_range(64..=256), rng.gen_range(4..=12))
        } else {
            (rng.gen_range(4..=12), rng.gen_range(64..=256))
        };
        let mut img = solid(w, h, [0, 0, 0, 0]);
        draw_rect(&mut img, 0, 0, w, h, random_color_opaque(rng));
        draw_border_full(&mut img, [0, 0, 0, 255]);
        let label = format!("{}", i);
        draw_text_centered_scaled(&mut img, w / 2, h / 2, &label, [255, 255, 255, 255], true);
        save(&img, &out.join(format!("thin_{:03}.png", i)))?;
    }
    fs::write(
        out.join("README.txt"),
        "Very thin horizontal/vertical bars to stress skyline/rotation.",
    )?;
    Ok(())
}

fn gen_trim_cases(out: &PathBuf, rng: &mut impl Rng) -> anyhow::Result<()> {
    ensure_dir(out)?;
    for i in 0..80u32 {
        let w = rng.gen_range(48..=192);
        let h = rng.gen_range(48..=192);
        let mut img = solid(w, h, [0, 0, 0, 0]);
        let bw = rng.gen_range(16..=(w / 2).max(16));
        let bh = rng.gen_range(16..=(h / 2).max(16));
        let offx = rng.gen_range(0..(w - bw + 1));
        let offy = rng.gen_range(0..(h - bh + 1));
        draw_rect(&mut img, offx, offy, bw, bh, random_color_opaque(rng));
        // edge feather for some
        if rng.gen_bool(0.5) {
            let mut soft = img.clone();
            draw_soft_circle(
                &mut soft,
                (w / 2) as i32,
                (h / 2) as i32,
                (bw.min(bh) as f32) / 2.0,
                [rng.gen(), rng.gen(), rng.gen()],
            );
            img = soft;
        }
        // draw border around content rect & center number inside content rect (避免影响 trim 边界)
        draw_border_rect(&mut img, offx, offy, bw, bh, [0, 0, 0, 255]);
        let label = format!("{}", i);
        draw_text_centered_scaled_in_rect(
            &mut img,
            offx,
            offy,
            bw,
            bh,
            &label,
            [255, 255, 255, 255],
            true,
        );
        save(&img, &out.join(format!("trim_{:03}.png", i)))?;
    }
    fs::write(out.join("README.txt"), "Images with transparent borders and internal colored/soft shapes to test trimming & thresholds.")?;
    Ok(())
}

fn gen_irregular(out: &PathBuf, rng: &mut impl Rng) -> anyhow::Result<()> {
    ensure_dir(out)?;
    for i in 0..150u32 {
        let w = rng.gen_range(32..=256);
        let h = rng.gen_range(32..=256);
        let mut img = solid(w, h, [0, 0, 0, 0]);
        let n = rng.gen_range(3..=9);
        for _ in 0..n {
            let cx = rng.gen_range(0..w) as i32;
            let cy = rng.gen_range(0..h) as i32;
            let rw = rng.gen_range(6..=w.min(96)) as f32;
            let rh = rng.gen_range(6..=h.min(96)) as f32;
            if rng.gen_bool(0.5) {
                draw_rect(
                    &mut img,
                    cx.max(0) as u32,
                    cy.max(0) as u32,
                    rw as u32,
                    rh as u32,
                    [rng.gen(), rng.gen(), rng.gen(), rng.gen_range(120..=255)],
                );
            } else {
                draw_ellipse(
                    &mut img,
                    cx,
                    cy,
                    rw / 2.0,
                    rh / 2.0,
                    [rng.gen(), rng.gen(), rng.gen(), rng.gen_range(120..=255)],
                );
            }
        }
        draw_border_full(&mut img, [0, 0, 0, 255]);
        let label = format!("{}", i);
        draw_text_centered_scaled(&mut img, w / 2, h / 2, &label, [255, 255, 255, 255], true);
        save(&img, &out.join(format!("irregular_{:03}.png", i)))?;
    }
    fs::write(out.join("README.txt"), "Irregular blotches (rects/ellipses) with varying alpha to stress trimming & packing quality.")?;
    Ok(())
}

fn gen_large_near_limit(out: &PathBuf) -> anyhow::Result<()> {
    ensure_dir(out)?;
    // near 1024x1024 with small islands; useful for single-page stress
    let mut img = solid(1024, 1024, [0, 0, 0, 0]);
    draw_rect(&mut img, 50, 50, 300, 60, [255, 0, 0, 255]);
    draw_rect(&mut img, 700, 400, 80, 300, [0, 255, 0, 255]);
    draw_soft_circle(&mut img, 800, 800, 120.0, [0, 0, 255]);
    draw_border_full(&mut img, [0, 0, 0, 255]);
    draw_text_centered_scaled(
        &mut img,
        1024 / 2,
        1024 / 2,
        "0",
        [255, 255, 255, 255],
        true,
    );
    save(&img, &out.join("large_0.png"))?;
    fs::write(
        out.join("README.txt"),
        "Large near-limit canvas with sparse content to test trim + page sizing.",
    )?;
    Ok(())
}

fn gen_pow2_mixed(out: &PathBuf, rng: &mut impl Rng) -> anyhow::Result<()> {
    ensure_dir(out)?;
    let sizes = [16, 32, 64, 128, 256];
    for i in 0..60u32 {
        let w = *sizes.choose(rng).unwrap_or(&64);
        let h = *sizes.choose(rng).unwrap_or(&64);
        let mut img = solid(w, h, [0, 0, 0, 0]);
        draw_rect(&mut img, 0, 0, w, h, random_color_opaque(rng));
        draw_border_full(&mut img, [0, 0, 0, 255]);
        let label = format!("{}", i);
        draw_text_centered_scaled(&mut img, w / 2, h / 2, &label, [255, 255, 255, 255], true);
        save(&img, &out.join(format!("pow2_{:03}.png", i)))?;
    }
    fs::write(
        out.join("README.txt"),
        "Power-of-two opaque blocks (various sizes).",
    )?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Usage: cargo run -p tex-packer-cli --example gen_assets -- [out_root]
    // Default out_root: assets/generated
    let out_root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("assets/generated"));
    ensure_dir(&out_root)?;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0xDEADBEEF);
    gen_basic_sizes(&out_root.join("basic"), &mut rng)?;
    gen_thin_bars(&out_root.join("thin"), &mut rng)?;
    gen_trim_cases(&out_root.join("trim"), &mut rng)?;
    gen_irregular(&out_root.join("irregular"), &mut rng)?;
    gen_large_near_limit(&out_root.join("large"))?;
    gen_pow2_mixed(&out_root.join("pow2_mixed"), &mut rng)?;

    // top-level note
    fs::write(
        out_root.join("README.txt"),
        "Generated test image sets: basic, thin, trim, irregular, large, pow2_mixed.",
    )?;
    println!("Generated assets under {}", out_root.display());
    Ok(())
}
