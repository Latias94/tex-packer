use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use handlebars::Handlebars;
use serde_json::json;
use tex_packer_core::{Atlas, PackOutput};
use tracing::info;

use crate::PackArgs;

pub(crate) fn write_layout_metadata(cli: &PackArgs, atlas: &Atlas<String>) -> anyhow::Result<()> {
    match cli.metadata.as_str() {
        "json-array" | "json" => {
            write_json(
                cli.out_dir.join(format!("{}.json", cli.name)),
                tex_packer_core::to_json_array(atlas),
            )?;
            info!(
                json_path = ?cli.out_dir.join(format!("{}.json", cli.name)),
                pages = atlas.pages.len(),
                "atlas written (layout-only)"
            );
        }
        "json-hash" => {
            write_json(
                cli.out_dir.join(format!("{}.json", cli.name)),
                tex_packer_core::to_json_hash(atlas),
            )?;
            info!(
                json_path = ?cli.out_dir.join(format!("{}.json", cli.name)),
                pages = atlas.pages.len(),
                "atlas written (layout-only)"
            );
        }
        "plist" => {
            let page_names = atlas_page_names(&atlas.pages, &cli.name);
            let plist = tex_packer_core::to_plist_hash_with_pages(atlas, &page_names);
            let plist_path = cli.out_dir.join(format!("{}.plist", cli.name));
            fs::write(&plist_path, plist)
                .with_context(|| format!("write {}", plist_path.display()))?;
            info!(
                ?plist_path,
                pages = atlas.pages.len(),
                "atlas written (layout-only)"
            );
        }
        "template" => anyhow::bail!("template metadata is not supported in --layout-only mode"),
        other => anyhow::bail!("unknown metadata format: {}", other),
    }
    Ok(())
}

pub(crate) fn write_output_pages(cli: &PackArgs, output: &PackOutput) -> anyhow::Result<()> {
    if output.pages.len() == 1 {
        let png_path = cli.out_dir.join(format!("{}.png", cli.name));
        output.pages[0]
            .rgba
            .save(&png_path)
            .with_context(|| format!("write {}", png_path.display()))?;
        info!(?png_path, "wrote page 0");
    } else {
        for page in &output.pages {
            let png_path = cli
                .out_dir
                .join(format!("{}_{}.png", cli.name, page.page.id));
            page.rgba
                .save(&png_path)
                .with_context(|| format!("write {}", png_path.display()))?;
            info!(?png_path, id = page.page.id, "wrote page");
        }
    }
    Ok(())
}

pub(crate) fn write_pack_metadata(cli: &PackArgs, output: &PackOutput) -> anyhow::Result<()> {
    match cli.metadata.as_str() {
        "json-array" | "json" => {
            let json_path = cli.out_dir.join(format!("{}.json", cli.name));
            write_json(
                json_path.clone(),
                tex_packer_core::to_json_array(&output.atlas),
            )?;
            info!(?json_path, pages = output.pages.len(), "atlas written");
        }
        "json-hash" => {
            let json_path = cli.out_dir.join(format!("{}.json", cli.name));
            write_json(
                json_path.clone(),
                tex_packer_core::to_json_hash(&output.atlas),
            )?;
            info!(?json_path, pages = output.pages.len(), "atlas written");
        }
        "plist" => {
            let page_names = output_page_names(output, &cli.name);
            let plist = tex_packer_core::to_plist_hash_with_pages(&output.atlas, &page_names);
            let plist_path = cli.out_dir.join(format!("{}.plist", cli.name));
            fs::write(&plist_path, plist)
                .with_context(|| format!("write {}", plist_path.display()))?;
            info!(?plist_path, pages = output.pages.len(), "atlas written");
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

pub(crate) fn export_layout_stats(
    stats_path: &PathBuf,
    atlas: &Atlas<String>,
) -> anyhow::Result<()> {
    let (used, total) = atlas_stats(atlas);
    let occupancy = occupancy(used, total);
    let value = json!({
        "pages": atlas.pages.len(),
        "used_area": used,
        "total_area": total,
        "occupancy": occupancy,
    });
    fs::write(stats_path, serde_json::to_string_pretty(&value)?)
        .with_context(|| format!("write {}", stats_path.display()))?;
    Ok(())
}

pub(crate) fn export_pack_stats(cli: &PackArgs, output: &PackOutput) -> anyhow::Result<()> {
    let Some(stats_path) = &cli.export_stats else {
        return Ok(());
    };

    let (used_area, total_area) = pack_output_stats(output);
    let occupancy = occupancy(used_area, total_area);
    let value = json!({
        "pages": output.pages.len(),
        "used_area": used_area,
        "total_area": total_area,
        "occupancy": occupancy,
    });
    if !cli.dry_run {
        fs::write(stats_path, serde_json::to_string_pretty(&value)?)
            .with_context(|| format!("write {}", stats_path.display()))?;
        info!(?stats_path, "stats exported");
    } else {
        println!(
            "pages={} used_area={} total_area={} occupancy={:.2}%",
            output.pages.len(),
            used_area,
            total_area,
            occupancy * 100.0
        );
    }
    Ok(())
}

pub(crate) fn pack_output_stats(output: &PackOutput) -> (u64, u64) {
    atlas_stats(&output.atlas)
}

fn atlas_stats<K>(atlas: &Atlas<K>) -> (u64, u64) {
    let mut used = 0u64;
    let mut total = 0u64;
    for page in &atlas.pages {
        total += (page.width as u64) * (page.height as u64);
        for frame in &page.frames {
            used += (frame.frame.w as u64) * (frame.frame.h as u64);
        }
    }
    (used, total)
}

pub(crate) fn occupancy(used: u64, total: u64) -> f64 {
    if total > 0 {
        used as f64 / total as f64
    } else {
        0.0
    }
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
    info!(?out_path, pages = output.pages.len(), "template written");
    Ok(())
}

pub(crate) fn render_template(cli: &PackArgs, output: &PackOutput) -> anyhow::Result<String> {
    let page_names = output_page_names(output, &cli.name);
    let context = tex_packer_core::to_template_context(&output.atlas, &page_names);
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
    if output.pages.len() == 1 {
        vec![format!("{atlas_name}.png")]
    } else {
        output
            .pages
            .iter()
            .map(|page| format!("{}_{}.png", atlas_name, page.page.id))
            .collect()
    }
}

fn atlas_page_names<K>(pages: &[tex_packer_core::Page<K>], atlas_name: &str) -> Vec<String> {
    if pages.len() == 1 {
        vec![format!("{atlas_name}.png")]
    } else {
        pages
            .iter()
            .map(|page| format!("{}_{}.png", atlas_name, page.id))
            .collect()
    }
}
