use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use handlebars::Handlebars;
use serde_json::json;
use tex_packer_core::export::{
    to_json_array, to_json_hash, to_plist_hash_with_pages, to_template_context,
};
use tex_packer_core::model::{Atlas, PackStats, PageId};
use tex_packer_core::offline::PackOutput;
use tracing::info;

use crate::PackArgs;

pub(crate) fn write_layout_metadata(cli: &PackArgs, atlas: &Atlas) -> anyhow::Result<()> {
    match cli.metadata.as_str() {
        "json-array" | "json" => {
            write_json(
                cli.out_dir.join(format!("{}.json", cli.name)),
                to_json_array(atlas),
            )?;
            info!(
                json_path = ?cli.out_dir.join(format!("{}.json", cli.name)),
                pages = atlas.pages().len(),
                "atlas written (layout-only)"
            );
        }
        "json-hash" => {
            write_json(
                cli.out_dir.join(format!("{}.json", cli.name)),
                to_json_hash(atlas),
            )?;
            info!(
                json_path = ?cli.out_dir.join(format!("{}.json", cli.name)),
                pages = atlas.pages().len(),
                "atlas written (layout-only)"
            );
        }
        "plist" => {
            let page_names = atlas_page_names(atlas, &cli.name);
            let plist = to_plist_hash_with_pages(atlas, &page_names);
            let plist_path = cli.out_dir.join(format!("{}.plist", cli.name));
            fs::write(&plist_path, plist)
                .with_context(|| format!("write {}", plist_path.display()))?;
            info!(
                ?plist_path,
                pages = atlas.pages().len(),
                "atlas written (layout-only)"
            );
        }
        "template" => anyhow::bail!("template metadata is not supported in --layout-only mode"),
        other => anyhow::bail!("unknown metadata format: {}", other),
    }
    Ok(())
}

pub(crate) fn write_output_pages(cli: &PackArgs, output: &PackOutput) -> anyhow::Result<()> {
    if output.pages().len() != output.atlas().pages().len() {
        anyhow::bail!(
            "rendered page count {} does not match atlas page count {}",
            output.pages().len(),
            output.atlas().pages().len()
        );
    }

    let single_page = output.atlas().pages().len() == 1;
    let mut written_page_ids = HashSet::with_capacity(output.pages().len());
    for rendered in output.pages() {
        let page_id = rendered.page_id();
        if !written_page_ids.insert(page_id) {
            anyhow::bail!("rendered page {page_id} appears more than once");
        }
        let page = output
            .atlas()
            .page(page_id)
            .with_context(|| format!("rendered page {page_id} is missing from atlas"))?;
        let actual_dimensions = rendered.rgba().dimensions();
        let expected_dimensions = (page.width(), page.height());
        if actual_dimensions != expected_dimensions {
            anyhow::bail!(
                "rendered page {page_id} dimensions {}x{} do not match atlas {}x{}",
                actual_dimensions.0,
                actual_dimensions.1,
                expected_dimensions.0,
                expected_dimensions.1
            );
        }

        let png_path = cli
            .out_dir
            .join(page_image_name(&cli.name, page_id, single_page));
        rendered
            .rgba()
            .save(&png_path)
            .with_context(|| format!("write {}", png_path.display()))?;
        info!(?png_path, %page_id, "wrote page");
    }
    Ok(())
}

pub(crate) fn write_pack_metadata(cli: &PackArgs, output: &PackOutput) -> anyhow::Result<()> {
    match cli.metadata.as_str() {
        "json-array" | "json" => {
            let json_path = cli.out_dir.join(format!("{}.json", cli.name));
            write_json(json_path.clone(), to_json_array(output.atlas()))?;
            info!(
                ?json_path,
                pages = output.atlas().pages().len(),
                "atlas written"
            );
        }
        "json-hash" => {
            let json_path = cli.out_dir.join(format!("{}.json", cli.name));
            write_json(json_path.clone(), to_json_hash(output.atlas()))?;
            info!(
                ?json_path,
                pages = output.atlas().pages().len(),
                "atlas written"
            );
        }
        "plist" => {
            let page_names = output_page_names(output, &cli.name);
            let plist = to_plist_hash_with_pages(output.atlas(), &page_names);
            let plist_path = cli.out_dir.join(format!("{}.plist", cli.name));
            fs::write(&plist_path, plist)
                .with_context(|| format!("write {}", plist_path.display()))?;
            info!(
                ?plist_path,
                pages = output.atlas().pages().len(),
                "atlas written"
            );
        }
        "template" => write_template(cli, output)?,
        other => anyhow::bail!("unknown metadata format: {}", other),
    }
    Ok(())
}

pub(crate) fn validate_metadata_mode(mode: &str) -> anyhow::Result<()> {
    match mode {
        "json-array" | "json" | "json-hash" | "plist" | "template" => Ok(()),
        other => anyhow::bail!("unknown metadata format: {}", other),
    }
}

pub(crate) fn export_layout_stats(stats_path: &PathBuf, atlas: &Atlas) -> anyhow::Result<()> {
    let value = stats_json(&atlas.stats());
    fs::write(stats_path, serde_json::to_string_pretty(&value)?)
        .with_context(|| format!("write {}", stats_path.display()))?;
    Ok(())
}

pub(crate) fn export_pack_stats(cli: &PackArgs, output: &PackOutput) -> anyhow::Result<()> {
    let Some(stats_path) = &cli.export_stats else {
        return Ok(());
    };

    let stats = output.stats();
    let value = stats_json(&stats);
    if !cli.dry_run {
        fs::write(stats_path, serde_json::to_string_pretty(&value)?)
            .with_context(|| format!("write {}", stats_path.display()))?;
        info!(?stats_path, "stats exported");
    } else {
        println!(
            "pages={} frames={} regions={} aliases={} content_area={} allocation_area={} page_area={} content_occupancy={:.2}% allocation_occupancy={:.2}%",
            stats.num_pages,
            stats.num_frames,
            stats.num_regions,
            stats.num_aliases,
            stats.content_area,
            stats.allocation_area,
            stats.page_area,
            stats.content_occupancy * 100.0,
            stats.allocation_occupancy * 100.0,
        );
    }
    Ok(())
}

fn stats_json(stats: &PackStats) -> serde_json::Value {
    json!({
        "pages": stats.num_pages,
        "frames": stats.num_frames,
        "regions": stats.num_regions,
        "aliases": stats.num_aliases,
        "rotated_regions": stats.num_rotated_regions,
        "trimmed_frames": stats.num_trimmed_frames,
        "page_area": stats.page_area,
        "content_area": stats.content_area,
        "allocation_area": stats.allocation_area,
        "content_occupancy": stats.content_occupancy,
        "allocation_occupancy": stats.allocation_occupancy,
    })
}

fn write_json(path: PathBuf, value: serde_json::Value) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(&value)?;
    fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn write_template(cli: &PackArgs, output: &PackOutput) -> anyhow::Result<()> {
    let rendered = render_template(cli, output)?;
    let out_path = template_output_path(cli);
    fs::write(&out_path, rendered).with_context(|| format!("write {}", out_path.display()))?;
    info!(
        ?out_path,
        pages = output.atlas().pages().len(),
        "template written"
    );
    Ok(())
}

pub(crate) fn render_template(cli: &PackArgs, output: &PackOutput) -> anyhow::Result<String> {
    let page_names = output_page_names(output, &cli.name);
    let context = to_template_context(output.atlas(), &page_names);
    let template_owned_from_file = if let Some(path) = &cli.template {
        Some(fs::read_to_string(path)?)
    } else {
        None
    };
    let template = if let Some(engine) = &cli.engine {
        builtin_template(engine)?
    } else if let Some(ref template) = template_owned_from_file {
        template.as_str()
    } else {
        include_str!("templates/unity.hbs")
    };

    let mut registry = Handlebars::new();
    registry.set_strict_mode(true);
    registry.register_template_string("tpl", template)?;
    Ok(registry.render("tpl", &context)?)
}

fn builtin_template(engine: &str) -> anyhow::Result<&'static str> {
    Ok(match engine.to_ascii_lowercase().as_str() {
        "unity" => include_str!("templates/unity.hbs"),
        "godot" => include_str!("templates/godot.hbs"),
        "phaser3" => include_str!("templates/phaser3_multiatlas.hbs"),
        "phaser3_single" => include_str!("templates/phaser3_singleatlas.hbs"),
        "spine" => include_str!("templates/spine_atlas.hbs"),
        "cocos" => include_str!("templates/cocos.hbs"),
        "unreal" => include_str!("templates/unreal.hbs"),
        other => anyhow::bail!("unknown engine template: {}", other),
    })
}

fn template_output_path(cli: &PackArgs) -> PathBuf {
    if let Some(engine) = &cli.engine {
        match engine.to_ascii_lowercase().as_str() {
            "spine" => cli.out_dir.join(format!("{}.atlas", cli.name)),
            "phaser3" => cli.out_dir.join(format!("{}.multiatlas.json", cli.name)),
            _ => cli.out_dir.join(format!("{}.template.json", cli.name)),
        }
    } else {
        cli.out_dir.join(format!("{}.template.json", cli.name))
    }
}

fn output_page_names(output: &PackOutput, atlas_name: &str) -> Vec<String> {
    atlas_page_names(output.atlas(), atlas_name)
}

fn atlas_page_names(atlas: &Atlas, atlas_name: &str) -> Vec<String> {
    let single_page = atlas.pages().len() == 1;
    atlas
        .pages()
        .iter()
        .map(|page| page_image_name(atlas_name, page.id(), single_page))
        .collect()
}

fn page_image_name(atlas_name: &str, page_id: PageId, single_page: bool) -> String {
    if single_page {
        format!("{atlas_name}.png")
    } else {
        format!("{atlas_name}_{page_id}.png")
    }
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
    use tex_packer_core::config::{OfflineConfig, PageConfig};
    use tex_packer_core::model::PackStats;
    use tex_packer_core::offline::{InputImage, OfflinePacker};

    use super::*;
    use crate::test_support::{TestDirectory, pack_args};

    #[test]
    fn stats_json_distinguishes_frames_from_regions() {
        let value = stats_json(&PackStats {
            num_pages: 1,
            num_frames: 3,
            num_regions: 2,
            num_aliases: 1,
            num_rotated_regions: 1,
            num_trimmed_frames: 2,
            page_area: 64,
            content_area: 20,
            allocation_area: 28,
            content_occupancy: 0.3125,
            allocation_occupancy: 0.4375,
        });

        assert_eq!(value["frames"], 3);
        assert_eq!(value["regions"], 2);
        assert_eq!(value["aliases"], 1);
        assert_eq!(value["content_area"], 20);
        assert_eq!(value["allocation_area"], 28);
        assert_eq!(value["content_occupancy"], 0.3125);
        assert_eq!(value["allocation_occupancy"], 0.4375);
        assert!(value.get("used_area").is_none());
        assert!(value.get("occupancy").is_none());
    }

    #[test]
    fn rendered_pages_are_resolved_and_written_by_page_id() {
        let output_dir = TestDirectory::new("rendered-pages").unwrap();
        let mut cli = pack_args(output_dir.path());
        cli.name = "atlas".into();
        cli.metadata = "json-array".into();
        let page = PageConfig::builder()
            .max_dimensions(4, 4)
            .texture_padding(0)
            .build()
            .unwrap();
        let config = OfflineConfig::builder().page_config(page).build().unwrap();
        let inputs = vec![
            InputImage {
                key: "red".into(),
                image: DynamicImage::ImageRgba8(RgbaImage::from_pixel(
                    4,
                    4,
                    Rgba([255, 0, 0, 255]),
                )),
            },
            InputImage {
                key: "green".into(),
                image: DynamicImage::ImageRgba8(RgbaImage::from_pixel(
                    4,
                    4,
                    Rgba([0, 255, 0, 255]),
                )),
            },
        ];
        let output = OfflinePacker::new(config).pack_images(inputs).unwrap();
        assert_eq!(output.atlas().pages().len(), 2);

        write_output_pages(&cli, &output).unwrap();
        write_pack_metadata(&cli, &output).unwrap();

        for page in output.atlas().pages() {
            let png_path = output_dir.path().join(page_image_name(
                &cli.name,
                page.id(),
                output.atlas().pages().len() == 1,
            ));
            assert!(png_path.is_file(), "missing {}", png_path.display());
            assert_eq!(
                image::open(&png_path).unwrap().dimensions(),
                (page.width(), page.height())
            );
        }
        let png_count = fs::read_dir(output_dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "png"))
            .count();
        assert_eq!(png_count, output.atlas().pages().len());

        let actual: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(output_dir.path().join("atlas.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(actual, to_json_array(output.atlas()));
    }
}
