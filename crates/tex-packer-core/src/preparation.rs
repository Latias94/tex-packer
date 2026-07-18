use crate::config::{OfflineConfig, SortOrder, TransparentPolicy};
use crate::error::{Result, TexPackerError};
use crate::model::Rect;
use crate::offline::{InputImage, LayoutItem};
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
    inputs: Vec<InputImage>,
    config: &OfflineConfig,
) -> Result<Vec<PreparedItem<RgbaImage>>> {
    let preparation = ImagePreparation::from(config);
    let mut out = Vec::with_capacity(inputs.len());
    for input in inputs {
        let (width, height) = (input.image.width(), input.image.height());
        validate_item_dimensions(&input.key, width, height)?;
        let rgba = input.image.into_rgba8();
        if let Some(item) = prepare_image(input.key, rgba, preparation) {
            out.push(item);
        }
    }
    sort_prepared(&mut out, preparation.sort_order);
    Ok(out)
}

fn prepare_image(
    key: String,
    rgba: RgbaImage,
    preparation: ImagePreparation,
) -> Option<PreparedItem<RgbaImage>> {
    let (width, height) = rgba.dimensions();
    let (rect, trimmed, source) = match preparation.trim {
        Some(trim) => prepare_trimmed_geometry(&rgba, trim)?,
        None => (
            Rect::new(0, 0, width, height),
            false,
            Rect::new(0, 0, width, height),
        ),
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

fn prepare_trimmed_geometry(rgba: &RgbaImage, trim: TrimOptions) -> Option<(Rect, bool, Rect)> {
    let (trim_rect, source) = compute_trim_rect(rgba, trim.threshold);
    match (trim_rect, trim.transparent_policy) {
        (Some(rect), _) => Some((Rect::new(0, 0, rect.w, rect.h), true, source)),
        (None, TransparentPolicy::Keep) => {
            let (width, height) = rgba.dimensions();
            let full = Rect::new(0, 0, width, height);
            Some((full, false, full))
        }
        (None, TransparentPolicy::OneByOne) => {
            let pixel = Rect::new(0, 0, 1, 1);
            Some((pixel, true, pixel))
        }
        (None, TransparentPolicy::Skip) => None,
    }
}

pub(crate) fn prepare_layout_items(
    items: Vec<LayoutItem>,
    config: &OfflineConfig,
) -> Result<Vec<PreparedItem<()>>> {
    let preparation = ImagePreparation::from(config);
    let mut prepared = Vec::with_capacity(items.len());
    for item in items {
        validate_item_dimensions(&item.key, item.w, item.h)?;
        let rect = Rect::new(0, 0, item.w, item.h);
        let source = item.source.unwrap_or(rect);
        let orig_size = item.source_size.unwrap_or((item.w, item.h));
        prepared.push(PreparedItem {
            key: item.key,
            payload: (),
            rect,
            trimmed: item.trimmed,
            source,
            orig_size,
        });
    }
    sort_prepared(&mut prepared, preparation.sort_order);
    Ok(prepared)
}

fn validate_item_dimensions(key: &str, width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0 {
        return Err(TexPackerError::InvalidItemDimensions {
            key: key.to_string(),
            width,
            height,
        });
    }
    Ok(())
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

pub(crate) fn compute_trim_rect(rgba: &RgbaImage, threshold: u8) -> (Option<Rect>, Rect) {
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
