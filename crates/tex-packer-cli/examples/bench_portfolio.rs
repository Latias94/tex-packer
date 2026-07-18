#![allow(clippy::match_like_matches_macro)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{env, fs};

use image::{DynamicImage, ImageReader};
use serde::Serialize;
use tex_packer_core::config::{
    GuillotineChoice, GuillotineSplit, MaxRectsHeuristic, OfflineConfig, PackingStrategy,
    PageConfig, SkylineHeuristic,
};
use tex_packer_core::{InputImage, pack_images};

#[derive(Debug, Serialize)]
struct BenchResult {
    name: String,
    pages: usize,
    total_area: u64,
    used_area: u64,
    occupancy: f64,
    ms: u128,
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: bench_portfolio <input_dir> [out_dir]");
        std::process::exit(2);
    }
    let input = Path::new(&args[1]);
    let out_dir = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        PathBuf::from("out")
    };
    fs::create_dir_all(&out_dir)?;

    let images = collect_images(input)?;
    println!("loaded {} images", images.len());

    let page = PageConfig::builder()
        .max_dimensions(2048, 2048)
        .allow_rotation(true)
        .border_padding(0)
        .texture_padding(2)
        .texture_extrusion(2)
        .build()?;

    let candidates = [
        (
            "skyline_mw",
            PackingStrategy::Skyline {
                heuristic: SkylineHeuristic::MinWaste,
                use_waste_map: false,
            },
        ),
        (
            "maxrects_baf",
            PackingStrategy::MaxRects {
                heuristic: MaxRectsHeuristic::BestAreaFit,
                reference: false,
            },
        ),
        (
            "maxrects_bl",
            PackingStrategy::MaxRects {
                heuristic: MaxRectsHeuristic::BottomLeft,
                reference: false,
            },
        ),
        (
            "maxrects_cp",
            PackingStrategy::MaxRects {
                heuristic: MaxRectsHeuristic::ContactPoint,
                reference: false,
            },
        ),
        (
            "guillotine_baf_slas",
            PackingStrategy::Guillotine {
                choice: GuillotineChoice::BestAreaFit,
                split: GuillotineSplit::SplitShorterLeftoverAxis,
            },
        ),
    ]
    .into_iter()
    .map(|(name, strategy)| {
        let config = OfflineConfig::builder()
            .page_config(page.clone())
            .outlines(false)
            .trim(true)
            .trim_threshold(0)
            .strategy(strategy)
            .build()?;
        Ok((name.to_owned(), config))
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

    let mut results: Vec<BenchResult> = Vec::new();
    for (name, cfg) in candidates {
        let start = Instant::now();
        // clone images to avoid moving them between trials
        let cloned: Vec<InputImage> = images
            .iter()
            .map(|i| InputImage {
                key: i.key.clone(),
                image: i.image.clone(),
            })
            .collect();
        match pack_images(cloned, cfg) {
            Ok(out) => {
                let (used, total) = compute_stats(&out);
                let occ = if total > 0 {
                    used as f64 / total as f64
                } else {
                    0.0
                };
                let dur = start.elapsed();
                let ms = dur.as_millis();
                println!(
                    "{:<20} pages={} occ={:.2}% time={}",
                    name,
                    out.pages.len(),
                    occ * 100.0,
                    fmt_dur(dur)
                );
                results.push(BenchResult {
                    name,
                    pages: out.pages.len(),
                    total_area: total,
                    used_area: used,
                    occupancy: occ,
                    ms,
                });
            }
            Err(e) => {
                eprintln!("{}: error: {}", name, e);
            }
        }
    }

    results.sort_by(|a, b| match a.pages.cmp(&b.pages) {
        std::cmp::Ordering::Equal => a.total_area.cmp(&b.total_area),
        other => other,
    });
    let json = serde_json::to_string_pretty(&results)?;
    fs::write(out_dir.join("bench_portfolio.json"), json)?;
    println!("wrote {}", out_dir.join("bench_portfolio.json").display());
    Ok(())
}

fn compute_stats(out: &tex_packer_core::PackOutput) -> (u64, u64) {
    let mut used: u64 = 0;
    let mut total: u64 = 0;
    for p in &out.atlas.pages {
        total += (p.width as u64) * (p.height as u64);
        for f in &p.frames {
            used += (f.frame.w as u64) * (f.frame.h as u64);
        }
    }
    (used, total)
}

fn collect_images(path: &Path) -> anyhow::Result<Vec<InputImage>> {
    let mut list = Vec::new();
    if path.is_file() {
        let img = load_image(path)?;
        let key = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("image")
            .to_string();
        list.push(InputImage { key, image: img });
    } else {
        visit_dir(path, path, &mut list)?;
    }
    Ok(list)
}

fn fmt_dur(d: Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms >= 1.0 {
        format!("{:.1}ms", ms)
    } else {
        format!("{}µs", d.as_micros())
    }
}

fn visit_dir(root: &Path, dir: &Path, out: &mut Vec<InputImage>) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let e = entry?;
        let p = e.path();
        if p.is_dir() {
            visit_dir(root, &p, out)?;
        } else if is_image(&p)
            && let Ok(img) = load_image(&p)
        {
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            out.push(InputImage {
                key: rel,
                image: img,
            });
        }
    }
    Ok(())
}

fn is_image(p: &Path) -> bool {
    match p
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
    {
        Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "tga" | "gif") => true,
        _ => false,
    }
}

fn load_image(p: &Path) -> anyhow::Result<DynamicImage> {
    let img = ImageReader::open(p)?.with_guessed_format()?.decode()?;
    Ok(img)
}
