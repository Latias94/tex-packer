use crate::config::{GuillotineChoice, GuillotineSplit};
use crate::geometry::{
    area_fit_score, area_u128, bottom_ex_u32, contains_rect, intersects, right_ex_u32,
};
use crate::model::Rect;

/// Scores a candidate placement using Guillotine choice semantics.
///
/// The first value is the primary score and the second value is the deterministic
/// tie-breaker. Lower is better for both values.
pub(crate) fn guillotine_score(
    choice: &GuillotineChoice,
    free: &Rect,
    w: u32,
    h: u32,
) -> (i128, i128) {
    let area_fit = area_fit_score(free, w, h);
    let leftover_h = free.w as i128 - w as i128;
    let leftover_v = free.h as i128 - h as i128;
    let short_fit = leftover_h.abs().min(leftover_v.abs());
    let long_fit = leftover_h.abs().max(leftover_v.abs());
    match choice {
        GuillotineChoice::BestAreaFit => (area_fit, short_fit),
        GuillotineChoice::BestShortSideFit => (short_fit, long_fit),
        GuillotineChoice::BestLongSideFit => (long_fit, short_fit),
        GuillotineChoice::WorstAreaFit => (-area_fit, -short_fit),
        GuillotineChoice::WorstShortSideFit => (-short_fit, -long_fit),
        GuillotineChoice::WorstLongSideFit => (-long_fit, -short_fit),
    }
}

/// Splits a free rectangle after a Guillotine placement.
pub(crate) fn guillotine_split(
    split: &GuillotineSplit,
    free: &Rect,
    placed: &Rect,
) -> (Option<Rect>, Option<Rect>) {
    let w_right = right_ex_u32(free).saturating_sub(right_ex_u32(placed));
    let h_bottom = bottom_ex_u32(free).saturating_sub(bottom_ex_u32(placed));
    let split_horizontal = match split {
        GuillotineSplit::SplitShorterLeftoverAxis => h_bottom < w_right,
        GuillotineSplit::SplitLongerLeftoverAxis => h_bottom > w_right,
        GuillotineSplit::SplitMinimizeArea => {
            area_u128(w_right, free.h) <= area_u128(free.w, h_bottom)
        }
        GuillotineSplit::SplitMaximizeArea => {
            area_u128(w_right, free.h) >= area_u128(free.w, h_bottom)
        }
        GuillotineSplit::SplitShorterAxis => free.h < free.w,
        GuillotineSplit::SplitLongerAxis => free.h > free.w,
    };

    let mut bottom = Rect::new(
        free.x,
        bottom_ex_u32(placed),
        0,
        free.h.saturating_sub(placed.h),
    );
    let mut right = Rect::new(
        right_ex_u32(placed),
        free.y,
        free.w.saturating_sub(placed.w),
        0,
    );

    if split_horizontal {
        bottom.w = free.w;
        right.h = placed.h;
    } else {
        bottom.w = placed.w;
        right.h = free.h;
    }

    (non_empty(bottom), non_empty(right))
}

/// Subtracts `placed` from every intersecting rectangle in `free`.
pub(crate) fn subtract_intersections<I>(free: I, placed: &Rect) -> Vec<Rect>
where
    I: IntoIterator<Item = Rect>,
{
    let mut out = Vec::new();
    for candidate in free {
        if intersects(&candidate, placed) {
            subtract_rect(candidate, placed, &mut out);
        } else {
            push_non_empty(&mut out, candidate);
        }
    }
    out
}

/// Removes zero-area rectangles and rectangles contained by another rectangle.
pub(crate) fn prune_contained(free: &mut Vec<Rect>) {
    let mut i = 0;
    while i < free.len() {
        if free[i].w == 0 || free[i].h == 0 {
            free.swap_remove(i);
            continue;
        }

        let mut j = i + 1;
        let mut remove_i = false;
        while j < free.len() {
            if free[j].w == 0 || free[j].h == 0 {
                free.swap_remove(j);
                continue;
            }

            if contains_rect(&free[j], &free[i]) {
                remove_i = true;
                break;
            }
            if contains_rect(&free[i], &free[j]) {
                free.swap_remove(j);
                continue;
            }
            j += 1;
        }

        if remove_i {
            free.swap_remove(i);
        } else {
            i += 1;
        }
    }
}

/// Merges adjacent rectangles that share an edge and span.
pub(crate) fn merge_adjacent(free: &mut Vec<Rect>) {
    let mut merged = true;
    while merged {
        merged = false;
        'outer: for i in 0..free.len() {
            for j in i + 1..free.len() {
                let a = free[i];
                let b = free[j];

                if a.y == b.y && a.h == b.h {
                    if right_ex_u32(&a) == b.x {
                        free[i] = Rect::new(a.x, a.y, a.w.saturating_add(b.w), a.h);
                        free.swap_remove(j);
                        merged = true;
                        break 'outer;
                    }
                    if right_ex_u32(&b) == a.x {
                        free[i] = Rect::new(b.x, a.y, a.w.saturating_add(b.w), a.h);
                        free.swap_remove(j);
                        merged = true;
                        break 'outer;
                    }
                }

                if a.x == b.x && a.w == b.w {
                    if bottom_ex_u32(&a) == b.y {
                        free[i] = Rect::new(a.x, a.y, a.w, a.h.saturating_add(b.h));
                        free.swap_remove(j);
                        merged = true;
                        break 'outer;
                    }
                    if bottom_ex_u32(&b) == a.y {
                        free[i] = Rect::new(a.x, b.y, a.w, a.h.saturating_add(b.h));
                        free.swap_remove(j);
                        merged = true;
                        break 'outer;
                    }
                }
            }
        }
    }
}

pub(crate) fn push_non_empty(free: &mut Vec<Rect>, rect: Rect) {
    if rect.w > 0 && rect.h > 0 {
        free.push(rect);
    }
}

fn subtract_rect(free: Rect, placed: &Rect, out: &mut Vec<Rect>) {
    let free_right = right_ex_u32(&free);
    let free_bottom = bottom_ex_u32(&free);
    let placed_right = right_ex_u32(placed);
    let placed_bottom = bottom_ex_u32(placed);

    let ix1 = free.x.max(placed.x);
    let iy1 = free.y.max(placed.y);
    let ix2 = free_right.min(placed_right);
    let iy2 = free_bottom.min(placed_bottom);

    if iy1 > free.y {
        push_non_empty(out, Rect::new(free.x, free.y, free.w, iy1 - free.y));
    }
    if iy2 < free_bottom {
        push_non_empty(out, Rect::new(free.x, iy2, free.w, free_bottom - iy2));
    }
    if ix1 > free.x {
        push_non_empty(
            out,
            Rect::new(free.x, iy1, ix1 - free.x, iy2.saturating_sub(iy1)),
        );
    }
    if ix2 < free_right {
        push_non_empty(
            out,
            Rect::new(ix2, iy1, free_right - ix2, iy2.saturating_sub(iy1)),
        );
    }
}

fn non_empty(rect: Rect) -> Option<Rect> {
    (rect.w > 0 && rect.h > 0).then_some(rect)
}
