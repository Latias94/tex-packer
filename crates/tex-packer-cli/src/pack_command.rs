use std::fs;

use anyhow::Context;
use tex_packer_core::{InputImage, PackOutput, PackerConfig, pack_images};
use tracing::{info, instrument};

use crate::PackArgs;
use crate::config_adapter;
use crate::input_loader::{gather_paths, load_images_with_progress};
use crate::output_writer::{
    export_layout_stats, export_pack_stats, occupancy, pack_output_stats, render_template,
    validate_metadata_mode, write_layout_metadata, write_output_pages, write_pack_metadata,
};

#[instrument(skip_all)]
pub(crate) fn run_pack(cli: &PackArgs, show_progress: bool) -> anyhow::Result<()> {
    fs::create_dir_all(&cli.out_dir)
        .with_context(|| format!("create out_dir {}", cli.out_dir.display()))?;

    let cfg = config_adapter::build_pack_config(cli)?;
    if cli.print_config {
        print_config(&cfg, &cli.print_config_format)?;
        return Ok(());
    }

    let paths = gather_paths(&cli.input, &cli.include, &cli.exclude)?;
    let inputs = load_images_with_progress(&paths, show_progress)?;
    info!(count = inputs.len(), "loaded input images");

    if cli.layout_only {
        return run_layout_only(cli, cfg, inputs);
    }

    run_image_pack(cli, cfg, inputs)
}

fn print_config(cfg: &PackerConfig, format: &str) -> anyhow::Result<()> {
    match format {
        "yaml" => println!("{}", serde_yaml::to_string(cfg)?),
        _ => println!("{}", serde_json::to_string_pretty(cfg)?),
    }
    Ok(())
}

fn run_layout_only(
    cli: &PackArgs,
    cfg: PackerConfig,
    inputs: Vec<InputImage>,
) -> anyhow::Result<()> {
    let items = inputs
        .iter()
        .map(|input| layout_item_from_image(input, &cfg))
        .collect::<Vec<_>>();
    let atlas = tex_packer_core::pack_layout_items(items, cfg)?;

    write_layout_metadata(cli, &atlas)?;
    if let Some(stats_path) = &cli.export_stats {
        export_layout_stats(stats_path, &atlas)?;
    }
    Ok(())
}

fn layout_item_from_image(
    input: &InputImage,
    cfg: &PackerConfig,
) -> tex_packer_core::pipeline::LayoutItem<String> {
    let rgba = input.image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let (trimmed_width, trimmed_height, source, trimmed) = if cfg.trim {
        let (trim_opt, source_rect) = tex_packer_core::compute_trim_rect(&rgba, cfg.trim_threshold);
        match trim_opt {
            Some(rect) => (rect.w, rect.h, source_rect, true),
            None => (
                width,
                height,
                tex_packer_core::Rect::new(0, 0, width, height),
                false,
            ),
        }
    } else {
        (
            width,
            height,
            tex_packer_core::Rect::new(0, 0, width, height),
            false,
        )
    };

    tex_packer_core::pipeline::LayoutItem {
        key: input.key.clone(),
        w: trimmed_width,
        h: trimmed_height,
        source: Some(source),
        source_size: Some((width, height)),
        trimmed,
    }
}

fn run_image_pack(
    cli: &PackArgs,
    cfg: PackerConfig,
    inputs: Vec<InputImage>,
) -> anyhow::Result<()> {
    let output = pack_images(inputs, cfg)?;

    if !cli.dry_run {
        write_output_pages(cli, &output)?;
        write_pack_metadata(cli, &output)?;
    } else {
        validate_metadata_mode(cli.metadata.as_str())?;
        if matches!(cli.metadata.as_str(), "template") {
            // Preserve old dry-run template behavior: render in memory to validate template inputs.
            let _ = render_template(cli, &output)?;
        }
    }

    log_pack_stats(&output);
    export_pack_stats(cli, &output)?;
    Ok(())
}

fn log_pack_stats(output: &PackOutput) {
    let (used_area, total_area) = pack_output_stats(output);
    let occupancy = occupancy(used_area, total_area);
    info!(
        pages = output.pages.len(),
        used_area,
        total_area,
        occupancy = format!("{:.2}%", occupancy * 100.0),
        "stats"
    );
}
