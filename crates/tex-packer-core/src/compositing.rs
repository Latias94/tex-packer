use image::{Rgba, RgbaImage};

/// Blit a sub-rectangle from `src` into `canvas` at destination (dx, dy),
/// optionally rotated 90° clockwise, then apply pixel extrusion around the
/// blitted content area and optional red outlines for debugging.
///
/// - (sx, sy, sw, sh): source rectangle within `src`
/// - (dx, dy): destination top-left in `canvas` where content area begins
/// - rotated: if true, rotate 90° CW during blit
/// - extrude: number of pixels to extrude around the content
/// - outlines: if true, draw a red 1px outline around the content area
pub fn blit_rgba(
    src: &RgbaImage,
    canvas: &mut RgbaImage,
    dx: u32,
    dy: u32,
    sx: u32,
    sy: u32,
    sw: u32,
    sh: u32,
    rotated: bool,
    extrude: u32,
    outlines: bool,
) {
    let (cw, ch) = canvas.dimensions();
    // destination (rendered) size may differ when rotated
    let (rw, rh) = if rotated { (sh, sw) } else { (sw, sh) };

    // main blit
    for yy in 0..rh {
        for xx in 0..rw {
            let (ix, iy) = if rotated {
                (sx + yy, sy + (sh - 1 - xx))
            } else {
                (sx + xx, sy + yy)
            };
            if dx + xx < cw && dy + yy < ch {
                let px = *src.get_pixel(ix, iy);
                canvas.put_pixel(dx + xx, dy + yy, px);
            }
        }
    }

    if outlines {
        // red outline on frame bounds
        let red = Rgba([255, 0, 0, 255]);
        for xx in 0..rw {
            if dx + xx < cw && dy < ch {
                canvas.put_pixel(dx + xx, dy, red);
            }
            let by = dy + rh.saturating_sub(1);
            if dx + xx < cw && by < ch {
                canvas.put_pixel(dx + xx, by, red);
            }
        }
        for yy in 0..rh {
            if dx < cw && dy + yy < ch {
                canvas.put_pixel(dx, dy + yy, red);
            }
            let rx = dx + rw.saturating_sub(1);
            if rx < cw && dy + yy < ch {
                canvas.put_pixel(rx, dy + yy, red);
            }
        }
    }

    if extrude > 0 {
        // edges
        for e in 1..=extrude {
            // top row
            if dy >= e && dy < ch {
                for xx in 0..rw {
                    if dx + xx < cw {
                        let p = *canvas.get_pixel(dx + xx, dy);
                        if dy >= e {
                            canvas.put_pixel(dx + xx, dy - e, p);
                        }
                    }
                }
            }
            // bottom row
            if dy + rh - 1 < ch && dy + rh - 1 + e < ch {
                for xx in 0..rw {
                    if dx + xx < cw {
                        let p = *canvas.get_pixel(dx + xx, dy + rh - 1);
                        canvas.put_pixel(dx + xx, dy + rh - 1 + e, p);
                    }
                }
            }
            // left col
            if dx >= e && dx < cw {
                for yy in 0..rh {
                    if dy + yy < ch {
                        let p = *canvas.get_pixel(dx, dy + yy);
                        canvas.put_pixel(dx - e, dy + yy, p);
                    }
                }
            }
            // right col
            if dx + rw - 1 < cw && dx + rw - 1 + e < cw {
                for yy in 0..rh {
                    if dy + yy < ch {
                        let p = *canvas.get_pixel(dx + rw - 1, dy + yy);
                        canvas.put_pixel(dx + rw - 1 + e, dy + yy, p);
                    }
                }
            }
        }
        // corners (copy the corner pixel) with bounds guards
        let c00 = if dx < cw && dy < ch {
            *canvas.get_pixel(dx, dy)
        } else {
            Rgba([0, 0, 0, 0])
        };
        let c10 = if dx + rw > 0 && dx + rw - 1 < cw && dy < ch {
            *canvas.get_pixel(dx + rw - 1, dy)
        } else {
            Rgba([0, 0, 0, 0])
        };
        let c01 = if dx < cw && dy + rh > 0 && dy + rh - 1 < ch {
            *canvas.get_pixel(dx, dy + rh - 1)
        } else {
            Rgba([0, 0, 0, 0])
        };
        let c11 = if dx + rw > 0 && dx + rw - 1 < cw && dy + rh > 0 && dy + rh - 1 < ch {
            *canvas.get_pixel(dx + rw - 1, dy + rh - 1)
        } else {
            Rgba([0, 0, 0, 0])
        };
        if dx >= 1 && dy >= 1 {
            for ex in 1..=extrude {
                for ey in 1..=extrude {
                    if dx >= ex && dy >= ey {
                        canvas.put_pixel(dx - ex, dy - ey, c00);
                    }
                }
            }
        }
        if dy >= 1 && dx + rw - 1 < cw {
            for ex in 1..=extrude {
                for ey in 1..=extrude {
                    if dy >= ey && dx + rw - 1 + ex < cw {
                        canvas.put_pixel(dx + rw - 1 + ex, dy - ey, c10);
                    }
                }
            }
        }
        if dx >= 1 && dy + rh - 1 < ch {
            for ex in 1..=extrude {
                for ey in 1..=extrude {
                    if dx >= ex && dy + rh - 1 + ey < ch {
                        canvas.put_pixel(dx - ex, dy + rh - 1 + ey, c01);
                    }
                }
            }
        }
        if dx + rw - 1 < cw && dy + rh - 1 < ch {
            for ex in 1..=extrude {
                for ey in 1..=extrude {
                    if dx + rw - 1 + ex < cw && dy + rh - 1 + ey < ch {
                        canvas.put_pixel(dx + rw - 1 + ex, dy + rh - 1 + ey, c11);
                    }
                }
            }
        }
    }
}
