use crate::model::{Atlas, Meta, Rect};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub(crate) struct ExportManifest {
    pub(crate) pages: Vec<ExportPage>,
    pub(crate) meta: Meta,
}

#[derive(Debug, Clone)]
pub(crate) struct ExportPage {
    pub(crate) id: usize,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) image: String,
    pub(crate) frames: Vec<ExportFrame>,
}

#[derive(Debug, Clone)]
pub(crate) struct ExportFrame {
    pub(crate) key: String,
    pub(crate) frame: Rect,
    pub(crate) rotated: bool,
    pub(crate) trimmed: bool,
    pub(crate) sprite_source_size: Rect,
    pub(crate) source_size: (u32, u32),
    pub(crate) pivot: (f32, f32),
    pub(crate) page: usize,
    pub(crate) page_size: (u32, u32),
}

impl ExportManifest {
    pub(crate) fn from_atlas<K: ToString + Clone>(atlas: &Atlas<K>) -> Self {
        let default_page_names: Vec<String> = atlas
            .pages
            .iter()
            .map(|page| format!("page_{}.png", page.id))
            .collect();
        Self::from_atlas_with_page_names(atlas, &default_page_names)
    }

    pub(crate) fn from_atlas_with_page_names<K: ToString + Clone>(
        atlas: &Atlas<K>,
        page_names: &[String],
    ) -> Self {
        let pages = atlas
            .pages
            .iter()
            .enumerate()
            .map(|(idx, page)| {
                let image = page_names
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| format!("page_{}.png", page.id));
                let frames = page
                    .frames
                    .iter()
                    .map(|frame| ExportFrame {
                        key: frame.key.to_string(),
                        frame: frame.frame,
                        rotated: frame.rotated,
                        trimmed: frame.trimmed,
                        sprite_source_size: frame.source,
                        source_size: frame.source_size,
                        pivot: (0.5, 0.5),
                        page: page.id,
                        page_size: (page.width, page.height),
                    })
                    .collect();
                ExportPage {
                    id: page.id,
                    width: page.width,
                    height: page.height,
                    image,
                    frames,
                }
            })
            .collect();

        Self {
            pages,
            meta: atlas.meta.clone(),
        }
    }
}

impl ExportFrame {
    pub(crate) fn frame_value(&self) -> Value {
        rect_value(&self.frame)
    }

    pub(crate) fn sprite_source_size_value(&self) -> Value {
        rect_value(&self.sprite_source_size)
    }

    pub(crate) fn source_size_value(&self) -> Value {
        size_value(self.source_size)
    }

    pub(crate) fn pivot_value(&self) -> Value {
        json!({"x": self.pivot.0, "y": self.pivot.1})
    }

    pub(crate) fn page_size_value(&self) -> Value {
        size_value(self.page_size)
    }
}

pub(crate) fn rect_value(rect: &Rect) -> Value {
    json!({"x": rect.x, "y": rect.y, "w": rect.w, "h": rect.h})
}

pub(crate) fn size_value(size: (u32, u32)) -> Value {
    json!({"w": size.0, "h": size.1})
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateContext {
    pub pages: Vec<TemplatePage>,
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplatePage {
    pub image: String,
    pub size: Value,
    pub sprites: Vec<TemplateSprite>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateSprite {
    pub name: String,
    pub frame: Value,
    pub rotated: bool,
    pub trimmed: bool,
    pub sprite_source_size: Value,
    pub source_size: Value,
    pub pivot: Value,
}

pub fn to_template_context<K: ToString + Clone>(
    atlas: &Atlas<K>,
    page_names: &[String],
) -> TemplateContext {
    let manifest = ExportManifest::from_atlas_with_page_names(atlas, page_names);
    let pages = manifest
        .pages
        .iter()
        .map(|page| TemplatePage {
            image: page.image.clone(),
            size: size_value((page.width, page.height)),
            sprites: page
                .frames
                .iter()
                .map(|frame| TemplateSprite {
                    name: frame.key.clone(),
                    frame: frame.frame_value(),
                    rotated: frame.rotated,
                    trimmed: frame.trimmed,
                    sprite_source_size: frame.sprite_source_size_value(),
                    source_size: frame.source_size_value(),
                    pivot: frame.pivot_value(),
                })
                .collect(),
        })
        .collect();
    let meta = json!({
        "app": manifest.meta.app,
        "version": manifest.meta.version,
        "format": manifest.meta.format,
        "scale": manifest.meta.scale,
    });
    TemplateContext { pages, meta }
}
