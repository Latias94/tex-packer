use std::path::{Path, PathBuf};

use anyhow::Context;
use globset::{Glob, GlobSetBuilder};
use image::{DynamicImage, ImageReader};
use tex_packer_core::InputImage;
use tracing::error;
use walkdir::WalkDir;

pub(crate) fn gather_paths(
    path: &Path,
    include: &[String],
    exclude: &[String],
) -> anyhow::Result<Vec<PathBuf>> {
    let include_set = build_glob_set(include)?;
    let exclude_set = build_glob_set(exclude)?;
    let mut paths = Vec::new();

    if path.is_file() {
        if !should_skip(path, include_set.as_ref(), exclude_set.as_ref()) && is_image(path) {
            paths.push(path.to_path_buf());
        }
    } else {
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            if path.is_file()
                && !should_skip(path, include_set.as_ref(), exclude_set.as_ref())
                && is_image(path)
            {
                paths.push(path.to_path_buf());
            }
        }
    }

    Ok(paths)
}

fn build_glob_set(patterns: &[String]) -> anyhow::Result<Option<globset::GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(Some(builder.build()?))
}

fn should_skip(
    path: &Path,
    include: Option<&globset::GlobSet>,
    exclude: Option<&globset::GlobSet>,
) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if let Some(exclude) = exclude {
        if exclude.is_match(&normalized) {
            return true;
        }
    }
    if let Some(include) = include {
        if !include.is_match(&normalized) {
            return true;
        }
    }
    false
}

fn is_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase()),
        Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "tga" | "gif")
    )
}

pub(crate) fn load_images_with_progress(
    paths: &[PathBuf],
    progress: bool,
) -> anyhow::Result<Vec<InputImage>> {
    use indicatif::{ProgressBar, ProgressStyle};

    let bar = if progress {
        let bar = ProgressBar::new(paths.len() as u64);
        bar.set_style(ProgressStyle::with_template(
            "{spinner:.green} loading {pos}/{len} [{elapsed_precise}] {wide_msg}",
        )?);
        Some(bar)
    } else {
        None
    };

    let mut images = Vec::with_capacity(paths.len());
    for path in paths {
        let msg = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if let Some(bar) = &bar {
            bar.set_message(msg.to_string());
        }

        match load_image(path).with_context(|| format!("load image {}", path.display())) {
            Ok(image) => {
                let key = path.to_string_lossy().replace('\\', "/");
                images.push(InputImage { key, image });
            }
            Err(error) => {
                error!(?path, error = %error, "skip image");
            }
        }

        if let Some(bar) = &bar {
            bar.inc(1);
        }
    }

    if let Some(bar) = &bar {
        bar.finish_and_clear();
    }

    Ok(images)
}

fn load_image(path: &Path) -> anyhow::Result<DynamicImage> {
    Ok(ImageReader::open(path)?.with_guessed_format()?.decode()?)
}
