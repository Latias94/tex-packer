use std::fs;

use anyhow::Context;
use tex_packer_core::offline::{InputImage, OfflinePacker, PackOutput};
use tracing::{info, instrument};

use crate::PackArgs;
use crate::config_adapter;
use crate::input_loader::{gather_paths, load_images_with_progress};
use crate::output_writer::{
    export_layout_stats, export_pack_stats, render_template, validate_metadata_mode,
    write_layout_metadata, write_output_pages, write_pack_metadata,
};

#[instrument(skip_all)]
pub(crate) fn run_pack(cli: &PackArgs, show_progress: bool) -> anyhow::Result<()> {
    fs::create_dir_all(&cli.out_dir)
        .with_context(|| format!("create out_dir {}", cli.out_dir.display()))?;

    let resolved = config_adapter::build_pack_config(cli)?;
    if cli.print_config {
        println!("{}", resolved.print(&cli.print_config_format)?);
        return Ok(());
    }
    let packer = OfflinePacker::new(resolved.into_offline());

    let paths = gather_paths(&cli.input, &cli.include, &cli.exclude)?;
    let inputs = load_images_with_progress(&paths, show_progress)?;
    info!(count = inputs.len(), "loaded input images");

    if cli.layout_only {
        return run_layout_only(cli, &packer, inputs);
    }

    run_image_pack(cli, &packer, inputs)
}

fn run_layout_only(
    cli: &PackArgs,
    packer: &OfflinePacker,
    inputs: Vec<InputImage>,
) -> anyhow::Result<()> {
    let atlas = packer.layout_images(inputs)?;

    write_layout_metadata(cli, &atlas)?;
    if let Some(stats_path) = &cli.export_stats {
        export_layout_stats(stats_path, &atlas)?;
    }
    Ok(())
}

fn run_image_pack(
    cli: &PackArgs,
    packer: &OfflinePacker,
    inputs: Vec<InputImage>,
) -> anyhow::Result<()> {
    let output = packer.pack_images(inputs)?;

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
    let stats = output.stats();
    info!(
        pages = stats.num_pages,
        frames = stats.num_frames,
        regions = stats.num_regions,
        aliases = stats.num_aliases,
        content_area = stats.content_area,
        allocation_area = stats.allocation_area,
        page_area = stats.page_area,
        content_occupancy = format!("{:.2}%", stats.content_occupancy * 100.0),
        allocation_occupancy = format!("{:.2}%", stats.allocation_occupancy * 100.0),
        "stats"
    );
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, Rgba, RgbaImage};
    use tex_packer_core::config::{OfflineConfig, TransparentPolicy};
    use tex_packer_core::export::to_json_array;

    use super::*;
    use crate::test_support::{TestDirectory, pack_args};

    fn parity_inputs() -> Vec<InputImage> {
        let visible =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 2, Rgba([32, 160, 224, 255])));
        let transparent = DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 2, Rgba([0, 0, 0, 0])));
        vec![
            InputImage {
                key: "visible-a".into(),
                image: visible.clone(),
            },
            InputImage {
                key: "transparent".into(),
                image: transparent,
            },
            InputImage {
                key: "visible-b".into(),
                image: visible,
            },
        ]
    }

    #[test]
    fn layout_only_preserves_decoded_image_policy_and_writes_no_png() {
        let output_dir = TestDirectory::new("layout-only-parity").unwrap();
        let mut cli = pack_args(output_dir.path());
        cli.name = "layout".into();
        cli.metadata = "json-array".into();
        let config = OfflineConfig::builder()
            .transparent_policy(TransparentPolicy::Skip)
            .build()
            .unwrap();
        let expected = OfflinePacker::new(config.clone())
            .pack_images(parity_inputs())
            .unwrap();

        run_layout_only(&cli, &OfflinePacker::new(config), parity_inputs()).unwrap();

        let metadata_path = output_dir.path().join("layout.json");
        let actual: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(metadata_path).unwrap()).unwrap();
        assert_eq!(actual, to_json_array(expected.atlas()));

        let frames = actual["pages"][0]["frames"].as_array().unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0]["key"], "visible-a");
        assert_eq!(frames[1]["key"], "visible-b");
        assert_eq!(frames[0]["frame"], frames[1]["frame"]);
        assert!(fs::read_dir(output_dir.path()).unwrap().all(|entry| {
            entry
                .unwrap()
                .path()
                .extension()
                .is_none_or(|extension| extension != "png")
        }));
    }
}
