use crate::config::{OfflineConfig, SortOrder, TransparentPolicy};
use crate::model::Rect;
use crate::pipeline::{InputImage, LayoutItem};
use image::RgbaImage;

#[derive(Debug, Clone, Copy)]
struct TrimOptions {
    threshold: u8,
    transparent_policy: TransparentPolicy,
}

#[derive(Debug, Clone, Copy)]
struct ImagePreparation {
    trim: Option<TrimOptions>,
    sort_order: SortOrder,
}

impl From<&OfflineConfig> for ImagePreparation {
    fn from(config: &OfflineConfig) -> Self {
        let trim = config
            .trim_threshold()
            .zip(config.transparent_policy())
            .map(|(threshold, transparent_policy)| TrimOptions {
                threshold,
                transparent_policy,
            });

        Self {
            trim,
            sort_order: config.sort_order(),
        }
    }
}

pub(crate) struct PreparedItem<T> {
    pub(crate) key: String,
    pub(crate) payload: T,
    pub(crate) rect: Rect,
    pub(crate) trimmed: bool,
    pub(crate) source: Rect,
    pub(crate) orig_size: (u32, u32),
}

pub(crate) fn prepare_images(
    inputs: &[InputImage],
    config: &OfflineConfig,
) -> Vec<PreparedItem<RgbaImage>> {
    let preparation = ImagePreparation::from(config);
    let mut out = Vec::with_capacity(inputs.len());
    for input in inputs {
        let rgba = input.image.to_rgba8();
        if let Some(item) = prepare_image(input.key.clone(), rgba, preparation) {
            out.push(item);
        }
    }
    sort_prepared(&mut out, preparation.sort_order);
    out
}

fn prepare_image(
    key: String,
    rgba: RgbaImage,
    preparation: ImagePreparation,
) -> Option<PreparedItem<RgbaImage>> {
    let (width, height) = rgba.dimensions();
    let (rect, trimmed, source) = if let Some(trim) = preparation.trim {
        let (trim_rect_opt, source_rect) = compute_trim_rect(&rgba, trim.threshold);
        match trim_rect_opt {
            Some(rect) => (Rect::new(0, 0, rect.w, rect.h), true, source_rect),
            None => match trim.transparent_policy {
                TransparentPolicy::Keep => (
                    Rect::new(0, 0, width, height),
                    false,
                    Rect::new(0, 0, width, height),
                ),
                TransparentPolicy::OneByOne => (Rect::new(0, 0, 1, 1), true, Rect::new(0, 0, 1, 1)),
                TransparentPolicy::Skip => return None,
            },
        }
    } else {
        (
            Rect::new(0, 0, width, height),
            false,
            Rect::new(0, 0, width, height),
        )
    };

    Some(PreparedItem {
        key,
        payload: rgba,
        rect,
        trimmed,
        source,
        orig_size: (width, height),
    })
}

pub(crate) fn prepare_layout<K: Into<String>>(
    inputs: Vec<(K, u32, u32)>,
    config: &OfflineConfig,
) -> Vec<PreparedItem<()>> {
    let preparation = ImagePreparation::from(config);
    let mut prepared = inputs
        .into_iter()
        .map(|(key, width, height)| {
            let key = key.into();
            let rect = Rect::new(0, 0, width, height);
            PreparedItem {
                key,
                payload: (),
                rect,
                trimmed: false,
                source: rect,
                orig_size: (width, height),
            }
        })
        .collect::<Vec<_>>();
    sort_prepared(&mut prepared, preparation.sort_order);
    prepared
}

pub(crate) fn prepare_layout_items<K: Into<String>>(
    items: Vec<LayoutItem<K>>,
    config: &OfflineConfig,
) -> Vec<PreparedItem<()>> {
    let preparation = ImagePreparation::from(config);
    let mut prepared = items
        .into_iter()
        .map(|item| {
            let key = item.key.into();
            let rect = Rect::new(0, 0, item.w, item.h);
            let source = item.source.unwrap_or(rect);
            let orig_size = item.source_size.unwrap_or((item.w, item.h));
            PreparedItem {
                key,
                payload: (),
                rect,
                trimmed: item.trimmed,
                source,
                orig_size,
            }
        })
        .collect::<Vec<_>>();
    sort_prepared(&mut prepared, preparation.sort_order);
    prepared
}

fn sort_prepared<T>(prepared: &mut [PreparedItem<T>], sort_order: SortOrder) {
    match sort_order {
        SortOrder::None => {}
        SortOrder::NameAsc => {
            prepared.sort_by(|a, b| a.key.cmp(&b.key));
        }
        SortOrder::AreaDesc => {
            prepared.sort_by(|a, b| {
                (u64::from(b.rect.w) * u64::from(b.rect.h))
                    .cmp(&(u64::from(a.rect.w) * u64::from(a.rect.h)))
                    .then_with(|| a.key.cmp(&b.key))
            });
        }
        SortOrder::MaxSideDesc => {
            prepared.sort_by(|a, b| {
                b.rect
                    .w
                    .max(b.rect.h)
                    .cmp(&a.rect.w.max(a.rect.h))
                    .then_with(|| a.key.cmp(&b.key))
            });
        }
        SortOrder::HeightDesc => {
            prepared.sort_by(|a, b| b.rect.h.cmp(&a.rect.h).then_with(|| a.key.cmp(&b.key)));
        }
        SortOrder::WidthDesc => {
            prepared.sort_by(|a, b| b.rect.w.cmp(&a.rect.w).then_with(|| a.key.cmp(&b.key)));
        }
    }
}

pub fn compute_trim_rect(rgba: &RgbaImage, threshold: u8) -> (Option<Rect>, Rect) {
    let (width, height) = rgba.dimensions();
    let mut x1 = 0;
    let mut y1 = 0;
    let mut x2 = width.saturating_sub(1);
    let mut y2 = height.saturating_sub(1);

    while x1 < width {
        let mut all_transparent = true;
        for y in 0..height {
            if rgba.get_pixel(x1, y)[3] > threshold {
                all_transparent = false;
                break;
            }
        }
        if all_transparent {
            x1 += 1;
        } else {
            break;
        }
    }

    if x1 >= width {
        return (None, Rect::new(0, 0, width, height));
    }

    while x2 > x1 {
        let mut all_transparent = true;
        for y in 0..height {
            if rgba.get_pixel(x2, y)[3] > threshold {
                all_transparent = false;
                break;
            }
        }
        if all_transparent {
            x2 -= 1;
        } else {
            break;
        }
    }

    while y1 < height {
        let mut all_transparent = true;
        for x in x1..=x2 {
            if rgba.get_pixel(x, y1)[3] > threshold {
                all_transparent = false;
                break;
            }
        }
        if all_transparent {
            y1 += 1;
        } else {
            break;
        }
    }

    while y2 > y1 {
        let mut all_transparent = true;
        for x in x1..=x2 {
            if rgba.get_pixel(x, y2)[3] > threshold {
                all_transparent = false;
                break;
            }
        }
        if all_transparent {
            y2 -= 1;
        } else {
            break;
        }
    }

    let trimmed_width = x2 - x1 + 1;
    let trimmed_height = y2 - y1 + 1;
    (
        Some(Rect::new(0, 0, trimmed_width, trimmed_height)),
        Rect::new(x1, y1, trimmed_width, trimmed_height),
    )
}
