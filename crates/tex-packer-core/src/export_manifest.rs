use crate::model::{Atlas, Meta, Rect};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub(crate) struct ExportManifest {
    pub(crate) pages: Vec<ExportPage>,
    pub(crate) meta: ExportMeta,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExportMeta {
    pub(crate) schema_version: &'static str,
    pub(crate) app: String,
    pub(crate) version: String,
    pub(crate) format: String,
    pub(crate) scale: f32,
    pub(crate) power_of_two: bool,
    pub(crate) square: bool,
    pub(crate) max_dim: (u32, u32),
    pub(crate) padding: (u32, u32),
    pub(crate) extrude: u32,
    pub(crate) allow_rotation: bool,
    pub(crate) trim_mode: String,
    pub(crate) background_color: Option<[u8; 4]>,
}

#[derive(Debug, Clone)]
pub(crate) struct ExportPage {
    pub(crate) id: u32,
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
    pub(crate) page: u32,
    pub(crate) page_size: (u32, u32),
}

impl ExportManifest {
    pub(crate) fn from_atlas(atlas: &Atlas) -> Self {
        let default_page_names: Vec<String> = atlas
            .pages()
            .iter()
            .map(|page| format!("page_{}.png", page.id().get()))
            .collect();
        Self::from_atlas_with_page_names(atlas, &default_page_names)
    }

    pub(crate) fn from_atlas_with_page_names(atlas: &Atlas, page_names: &[String]) -> Self {
        let pages = atlas
            .pages()
            .iter()
            .enumerate()
            .map(|(idx, page)| {
                let page_id = page.id().get();
                let image = page_names
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| format!("page_{page_id}.png"));
                let frames = page
                    .resolved_frames()
                    .map(|resolved| {
                        let frame = resolved.frame();
                        let region = resolved.region();
                        ExportFrame {
                            key: frame.key().to_owned(),
                            frame: region.content(),
                            rotated: region.rotated(),
                            trimmed: frame.trimmed(),
                            sprite_source_size: frame.source(),
                            source_size: frame.source_size(),
                            pivot: (0.5, 0.5),
                            page: resolved.page_id().get(),
                            page_size: resolved.page_size(),
                        }
                    })
                    .collect();
                ExportPage {
                    id: page_id,
                    width: page.width(),
                    height: page.height(),
                    image,
                    frames,
                }
            })
            .collect();

        Self {
            pages,
            meta: ExportMeta::from(atlas.meta()),
        }
    }
}

impl From<&Meta> for ExportMeta {
    fn from(meta: &Meta) -> Self {
        Self {
            schema_version: "1",
            app: meta.app().to_owned(),
            version: meta.version().to_owned(),
            format: meta.format().to_owned(),
            scale: meta.scale(),
            power_of_two: meta.power_of_two(),
            square: meta.square(),
            max_dim: meta.max_dimensions(),
            padding: meta.padding(),
            extrude: meta.extrude(),
            allow_rotation: meta.allow_rotation(),
            trim_mode: meta.trim_mode().to_owned(),
            background_color: meta.background_color(),
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

pub fn to_template_context(atlas: &Atlas, page_names: &[String]) -> TemplateContext {
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
