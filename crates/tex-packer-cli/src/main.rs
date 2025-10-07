use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use clap::{ArgAction, Parser, Subcommand};
use globset::{Glob, GlobSetBuilder};
use handlebars::Handlebars;
use image::{DynamicImage, ImageReader};
use serde::Deserialize;
use tex_packer_core::config::{
    AlgorithmFamily, AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic,
    SkylineHeuristic, SortOrder,
};
use tex_packer_core::{pack_images, InputImage, PackerConfig};
use tracing::{error, info};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(
    name = "tex-packer",
    about = "Pack images into a texture atlas",
    version,
    author
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Show progress bars (disable with --no-progress or --quiet)
    #[arg(long, default_value_t = true, action=ArgAction::Set, global=true, help_heading = "Logging/UX")]
    progress: bool,
    /// Increase verbosity (-v, -vv)
    #[arg(short, long, action=ArgAction::Count, global=true, help_heading = "Logging/UX")]
    verbose: u8,
    /// Quiet mode (overrides verbose)
    #[arg(
        short,
        long,
        default_value_t = false,
        global = true,
        help_heading = "Logging/UX"
    )]
    quiet: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Pack images into an atlas
    Pack(PackArgs),
    /// Render only template metadata (forces --metadata template)
    Template(PackArgs),
    /// Layout-only export (no PNGs): compute placements and export JSON/Plist
    Layout(PackArgs),
    /// Simple timing bench (packs once, prints time + occupancy)
    Bench(BenchArgs),
}

#[derive(Parser, Debug, Clone)]
struct PackArgs {
    // Input/Output
    /// Input file or directory
    #[arg(help_heading = "Input/Output")]
    input: PathBuf,
    /// Output directory
    #[arg(short, long, default_value = "out", help_heading = "Input/Output")]
    out_dir: PathBuf,
    /// Atlas base name (files will be name.png/.json)
    #[arg(short, long, default_value = "atlas", help_heading = "Input/Output")]
    name: String,
    /// YAML config file path (overrides algorithm-related options)
    #[arg(long, help_heading = "Input/Output")]
    config: Option<PathBuf>,
    /// Include patterns (glob). If set, only files matching any pattern are considered
    #[arg(long, help_heading = "Input/Output")]
    include: Vec<String>,
    /// Exclude patterns (glob). Files matching any pattern will be ignored
    #[arg(long, help_heading = "Input/Output")]
    exclude: Vec<String>,

    // Layout
    /// Max width
    #[arg(long, default_value_t = 1024, help_heading = "Layout")]
    max_width: u32,
    /// Max height
    #[arg(long, default_value_t = 1024, help_heading = "Layout")]
    max_height: u32,
    /// Force output size to max_width/max_height
    #[arg(long, default_value_t = false, help_heading = "Layout")]
    force_max_dimensions: bool,
    /// Resize page dims to power of two
    #[arg(long, default_value_t = false, help_heading = "Layout")]
    pow2: bool,
    /// Force square page
    #[arg(long, default_value_t = false, help_heading = "Layout")]
    square: bool,
    /// Sort order: area_desc|max_side_desc|height_desc|width_desc|name_asc|none
    #[arg(long, default_value = "area_desc", help_heading = "Layout")]
    sort_order: String,

    // Image Processing
    /// Allow rotation (90deg)
    #[arg(long, default_value_t = true, help_heading = "Image Processing")]
    allow_rotation: bool,
    /// Border padding (around entire page)
    #[arg(long, default_value_t = 0, help_heading = "Image Processing")]
    border_padding: u32,
    /// Padding between frames
    #[arg(long, default_value_t = 2, help_heading = "Image Processing")]
    texture_padding: u32,
    /// Extrude pixels around each frame
    #[arg(long, default_value_t = 0, help_heading = "Image Processing")]
    texture_extrusion: u32,
    /// Trim transparent borders
    #[arg(long, default_value_t = true, help_heading = "Image Processing")]
    trim: bool,
    /// Trim alpha threshold (0..=255)
    #[arg(long, default_value_t = 0, help_heading = "Image Processing")]
    trim_threshold: u8,
    /// Draw red outlines (debug)
    #[arg(long, default_value_t = false, help_heading = "Image Processing")]
    outlines: bool,
    /// Layout-only: compute placements and export metadata (no PNGs)
    #[arg(long, default_value_t = false, help_heading = "Export")]
    layout_only: bool,

    // Algorithms/Heuristics/Auto
    /// Algorithm: skyline | maxrects | guillotine | auto
    #[arg(long, value_parser = ["skyline", "maxrects", "guillotine", "auto"], default_value = "skyline", help_heading = "Algorithms")]
    algorithm: String,
    /// MaxRects heuristic: baf|bssf|blsf|bl|cp
    #[arg(long, default_value = "baf", help_heading = "Heuristics")]
    heuristic: String,
    /// Skyline heuristic: bl|minwaste
    #[arg(long, default_value = "bl", help_heading = "Heuristics")]
    skyline: String,
    /// Guillotine choice: baf|bssf|blsf|waf|wssf|wlsf
    #[arg(long, default_value = "baf", help_heading = "Heuristics")]
    g_choice: String,
    /// Guillotine split: slas|llas|minas|maxas|sas|las
    #[arg(long, default_value = "slas", help_heading = "Heuristics")]
    g_split: String,
    /// Auto mode: fast | quality
    #[arg(long, default_value = "quality", help_heading = "Auto/Portfolio")]
    auto_mode: String,
    /// Time budget for auto mode (ms)
    #[arg(long, help_heading = "Auto/Portfolio")]
    time_budget: Option<u64>,
    /// Evaluate auto candidates in parallel (requires core feature `parallel`)
    #[arg(long, default_value_t = false, help_heading = "Auto/Portfolio")]
    parallel: bool,
    /// Use waste map for skyline
    #[arg(long, default_value_t = false, help_heading = "Heuristics")]
    use_waste_map: bool,
    /// Policy for fully transparent images when trim is on: keep | one_by_one | skip
    #[arg(long, default_value = "keep", help_heading = "Image Processing")]
    transparent_policy: String,
    /// Use reference-accurate MaxRects split/prune (SplitFreeNode style)
    #[arg(long, default_value_t = false, help_heading = "Auto/Portfolio")]
    mr_reference: bool,
    /// Auto: enable mr_reference when time budget >= this (ms) (overrides default heuristic)
    #[arg(long, help_heading = "Auto/Portfolio")]
    auto_mr_ref_time_threshold: Option<u64>,
    /// Auto: enable mr_reference when inputs >= this count (overrides default heuristic)
    #[arg(long, help_heading = "Auto/Portfolio")]
    auto_mr_ref_input_threshold: Option<usize>,

    // Export
    /// Metadata format: json-array | json (alias) | json-hash | plist | template
    #[arg(long, default_value = "json-array", help_heading = "Export")]
    metadata: String,
    /// Built-in engine template: unity | godot | phaser3 | phaser3_single | spine | cocos | unreal
    #[arg(long, help_heading = "Export")]
    engine: Option<String>,
    /// External template file (handlebars), used when --metadata template
    #[arg(long, help_heading = "Export")]
    template: Option<PathBuf>,
    /// Export packing stats (JSON) to this file
    #[arg(long, help_heading = "Export")]
    export_stats: Option<PathBuf>,
    /// Print the merged configuration (after CLI/YAML) and exit
    #[arg(long, default_value_t = false, help_heading = "Export")]
    print_config: bool,
    /// Output format for --print-config: json|yaml
    #[arg(long, default_value = "json", value_parser = ["json", "yaml"], help_heading = "Export")]
    print_config_format: String,
    /// Dry run: compute layout and stats but do not write files
    #[arg(long, default_value_t = false, help_heading = "Export")]
    dry_run: bool,
}

#[derive(Parser, Debug, Clone)]
struct BenchArgs {
    /// Input directory
    input: PathBuf,
    /// Algorithm: skyline | maxrects | guillotine | auto
    #[arg(long, value_parser = ["skyline", "maxrects", "guillotine", "auto"], default_value = "auto")]
    algorithm: String,
    /// Auto mode: fast | quality
    #[arg(long, default_value = "quality")]
    auto_mode: String,
    /// Time budget for auto mode (ms)
    #[arg(long)]
    time_budget: Option<u64>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing_with_level(cli.quiet, cli.verbose);
    match &cli.command {
        Commands::Pack(args) => run_pack(args, cli.progress && !cli.quiet),
        Commands::Template(args) => {
            let mut a = args.clone();
            a.metadata = "template".into();
            run_pack(&a, cli.progress && !cli.quiet)
        }
        Commands::Layout(args) => {
            let mut a = args.clone();
            a.layout_only = true;
            run_pack(&a, false)
        }
        Commands::Bench(b) => run_bench(b),
    }
}

fn run_pack(cli: &PackArgs, show_progress: bool) -> anyhow::Result<()> {
    fs::create_dir_all(&cli.out_dir)
        .with_context(|| format!("create out_dir {}", cli.out_dir.display()))?;

    let (family, mr_heuristic, sky_heuristic, g_choice, g_split, auto_mode) = parse_algo(cli)?;

    // Load config file if provided; config file sets algorithm-related options en bloc
    let cfg = if let Some(path) = &cli.config {
        let file = fs::read_to_string(path)?;
        let y: YamlConfig = serde_yaml::from_str(&file)?;
        let mut tmp = y.into_packer_config(PackerConfig {
            max_width: cli.max_width,
            max_height: cli.max_height,
            allow_rotation: cli.allow_rotation,
            force_max_dimensions: cli.force_max_dimensions,
            border_padding: cli.border_padding,
            texture_padding: cli.texture_padding,
            texture_extrusion: cli.texture_extrusion,
            trim: cli.trim,
            trim_threshold: cli.trim_threshold,
            texture_outlines: cli.outlines,
            power_of_two: cli.pow2,
            square: cli.square,
            use_waste_map: cli.use_waste_map,
            family,
            mr_heuristic,
            skyline_heuristic: sky_heuristic,
            g_choice,
            g_split,
            auto_mode,
            sort_order: parse_sort_order(&cli.sort_order)?,
            time_budget_ms: cli.time_budget,
            parallel: cli.parallel,
            mr_reference: false,
            auto_mr_ref_time_ms_threshold: cli.auto_mr_ref_time_threshold,
            auto_mr_ref_input_threshold: cli.auto_mr_ref_input_threshold,
            transparent_policy: cli
                .transparent_policy
                .parse()
                .unwrap_or(tex_packer_core::config::TransparentPolicy::Keep),
        });
        if cli.mr_reference {
            tmp.mr_reference = true;
        }
        tmp
    } else {
        PackerConfig {
            max_width: cli.max_width,
            max_height: cli.max_height,
            allow_rotation: cli.allow_rotation,
            force_max_dimensions: cli.force_max_dimensions,
            border_padding: cli.border_padding,
            texture_padding: cli.texture_padding,
            texture_extrusion: cli.texture_extrusion,
            trim: cli.trim,
            trim_threshold: cli.trim_threshold,
            texture_outlines: cli.outlines,
            power_of_two: cli.pow2,
            square: cli.square,
            use_waste_map: cli.use_waste_map,
            family,
            mr_heuristic,
            skyline_heuristic: sky_heuristic,
            g_choice,
            g_split,
            auto_mode,
            sort_order: parse_sort_order(&cli.sort_order)?,
            time_budget_ms: cli.time_budget,
            parallel: cli.parallel,
            mr_reference: cli.mr_reference,
            auto_mr_ref_time_ms_threshold: cli.auto_mr_ref_time_threshold,
            auto_mr_ref_input_threshold: cli.auto_mr_ref_input_threshold,
            transparent_policy: cli
                .transparent_policy
                .parse()
                .unwrap_or(tex_packer_core::config::TransparentPolicy::Keep),
        }
    };

    if cli.print_config {
        match cli.print_config_format.as_str() {
            "yaml" => println!("{}", serde_yaml::to_string(&cfg)?),
            _ => println!("{}", serde_json::to_string_pretty(&cfg)?),
        }
        return Ok(());
    }

    let paths = gather_paths(&cli.input, &cli.include, &cli.exclude)?;
    let inputs = load_images_with_progress(&paths, show_progress)?;
    info!(count = inputs.len(), "loaded input images");
    // layout-only branch
    if cli.layout_only {
        use tex_packer_core::pipeline::LayoutItem;
        let mut items: Vec<LayoutItem<String>> = Vec::with_capacity(inputs.len());
        for inp in &inputs {
            let rgba = inp.image.to_rgba8();
            let (w, h) = rgba.dimensions();
            let (tw, th, source, trimmed) = if cfg.trim {
                let (trim_opt, src_rect) =
                    tex_packer_core::pipeline::compute_trim_rect(&rgba, cfg.trim_threshold);
                match trim_opt {
                    Some(r) => (r.w, r.h, src_rect, true),
                    None => (w, h, tex_packer_core::Rect::new(0, 0, w, h), false),
                }
            } else {
                (w, h, tex_packer_core::Rect::new(0, 0, w, h), false)
            };
            items.push(LayoutItem {
                key: inp.key.clone(),
                w: tw,
                h: th,
                source: Some(source),
                source_size: Some((w, h)),
                trimmed,
            });
        }
        let atlas = tex_packer_core::pack_layout_items(items, cfg.clone())?;
        // Write metadata only
        match cli.metadata.as_str() {
            "json-array" | "json" => {
                let json_path = cli.out_dir.join(format!("{}.json", cli.name));
                let json_value = tex_packer_core::to_json_array(&atlas);
                let json = serde_json::to_string_pretty(&json_value)?;
                fs::write(&json_path, json)
                    .with_context(|| format!("write {}", json_path.display()))?;
                info!(
                    ?json_path,
                    pages = atlas.pages.len(),
                    "atlas written (layout-only)"
                );
            }
            "json-hash" => {
                let json_path = cli.out_dir.join(format!("{}.json", cli.name));
                let json_value = tex_packer_core::to_json_hash(&atlas);
                let json = serde_json::to_string_pretty(&json_value)?;
                fs::write(&json_path, json)
                    .with_context(|| format!("write {}", json_path.display()))?;
                info!(
                    ?json_path,
                    pages = atlas.pages.len(),
                    "atlas written (layout-only)"
                );
            }
            "plist" => {
                let page_names: Vec<String> = if atlas.pages.len() == 1 {
                    vec![format!("{}.png", cli.name)]
                } else {
                    atlas
                        .pages
                        .iter()
                        .map(|p| format!("{}_{}.png", cli.name, p.id))
                        .collect()
                };
                let plist = tex_packer_core::to_plist_hash_with_pages(&atlas, &page_names);
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
        if let Some(stats_path) = &cli.export_stats {
            let (used, total) = {
                let mut u = 0;
                let mut t = 0;
                for p in &atlas.pages {
                    t += (p.width as u64) * (p.height as u64);
                    for f in &p.frames {
                        u += (f.frame.w as u64) * (f.frame.h as u64);
                    }
                }
                (u, t)
            };
            let occupancy = if total > 0 {
                used as f64 / total as f64
            } else {
                0.0
            };
            let value = serde_json::json!({"pages": atlas.pages.len(),"used_area": used, "total_area": total, "occupancy": occupancy});
            fs::write(stats_path, serde_json::to_string_pretty(&value)?)
                .with_context(|| format!("write {}", stats_path.display()))?;
        }
        return Ok(());
    }
    let out = pack_images(inputs, cfg.clone())?;

    if !cli.dry_run {
        // write png(s)
        if out.pages.len() == 1 {
            let png_path = cli.out_dir.join(format!("{}.png", cli.name));
            out.pages[0]
                .rgba
                .save(&png_path)
                .with_context(|| format!("write {}", png_path.display()))?;
            info!(?png_path, "wrote page 0");
        } else {
            for p in &out.pages {
                let png_path = cli.out_dir.join(format!("{}_{}.png", cli.name, p.page.id));
                p.rgba
                    .save(&png_path)
                    .with_context(|| format!("write {}", png_path.display()))?;
                info!(?png_path, id = p.page.id, "wrote page");
            }
        }
    }

    // stats
    let (used_area, total_area) = compute_stats(&out);
    let occupancy = if total_area > 0 {
        used_area as f64 / total_area as f64
    } else {
        0.0
    };
    info!(
        pages = out.pages.len(),
        used_area,
        total_area,
        occupancy = format!("{:.2}%", occupancy * 100.0),
        "stats"
    );

    match cli.metadata.as_str() {
        // Accept "json" as an alias of "json-array" to match layout-only behavior
        "json-array" | "json" => {
            if !cli.dry_run {
                let json_path = cli.out_dir.join(format!("{}.json", cli.name));
                let json_value = tex_packer_core::to_json_array(&out.atlas);
                let json = serde_json::to_string_pretty(&json_value)?;
                fs::write(&json_path, json)
                    .with_context(|| format!("write {}", json_path.display()))?;
                info!(?json_path, pages = out.pages.len(), "atlas written");
            }
        }
        "json-hash" => {
            if !cli.dry_run {
                let json_path = cli.out_dir.join(format!("{}.json", cli.name));
                let json_value = tex_packer_core::to_json_hash(&out.atlas);
                let json = serde_json::to_string_pretty(&json_value)?;
                fs::write(&json_path, json)
                    .with_context(|| format!("write {}", json_path.display()))?;
                info!(?json_path, pages = out.pages.len(), "atlas written");
            }
        }
        "plist" => {
            if !cli.dry_run {
                let plist_path = cli.out_dir.join(format!("{}.plist", cli.name));
                // Build page filenames for meta
                let page_names: Vec<String> = if out.pages.len() == 1 {
                    vec![format!("{}.png", cli.name)]
                } else {
                    out.pages
                        .iter()
                        .map(|p| format!("{}_{}.png", cli.name, p.page.id))
                        .collect()
                };
                let plist = tex_packer_core::to_plist_hash_with_pages(&out.atlas, &page_names);
                fs::write(&plist_path, plist)
                    .with_context(|| format!("write {}", plist_path.display()))?;
                info!(?plist_path, pages = out.pages.len(), "atlas written");
            }
        }
        "template" => {
            // Build context (pages + sprites) and render template
            let page_names: Vec<String> = if out.pages.len() == 1 {
                vec![format!("{}.png", cli.name)]
            } else {
                out.pages
                    .iter()
                    .map(|p| format!("{}_{}.png", cli.name, p.page.id))
                    .collect()
            };
            let ctx = build_template_context(&out, &page_names);

            let tpl_owned_from_file: Option<String> = if let Some(path) = &cli.template {
                Some(std::fs::read_to_string(path)?)
            } else {
                None
            };
            let tpl_ref: &str = if let Some(engine) = &cli.engine {
                match engine.to_ascii_lowercase().as_str() {
                    "unity" => include_str!("templates/unity.hbs"),
                    "godot" => include_str!("templates/godot.hbs"),
                    "phaser3" => include_str!("templates/phaser3_multiatlas.hbs"),
                    "phaser3_single" => include_str!("templates/phaser3_singleatlas.hbs"),
                    "spine" => include_str!("templates/spine_atlas.hbs"),
                    "cocos" => include_str!("templates/cocos.hbs"),
                    "unreal" => include_str!("templates/unreal.hbs"),
                    other => anyhow::bail!("unknown engine template: {}", other),
                }
            } else if let Some(ref s) = tpl_owned_from_file {
                s.as_str()
            } else {
                // default to unity if not specified
                include_str!("templates/unity.hbs")
            };

            let mut reg = Handlebars::new();
            reg.set_strict_mode(true);
            reg.register_template_string("tpl", tpl_ref)?;
            let rendered = reg.render("tpl", &ctx)?;

            if !cli.dry_run {
                let out_path = if let Some(engine) = &cli.engine {
                    match engine.to_ascii_lowercase().as_str() {
                        "spine" => cli.out_dir.join(format!("{}.atlas", cli.name)),
                        "phaser3" => cli.out_dir.join(format!("{}.multiatlas.json", cli.name)),
                        _ => cli.out_dir.join(format!("{}.template.json", cli.name)),
                    }
                } else {
                    cli.out_dir.join(format!("{}.template.json", cli.name))
                };
                fs::write(&out_path, rendered)
                    .with_context(|| format!("write {}", out_path.display()))?;
                info!(?out_path, pages = out.pages.len(), "template written");
            }
        }
        other => anyhow::bail!("unknown metadata format: {}", other),
    }

    if let Some(stats_path) = &cli.export_stats {
        let (used_area, total_area) = compute_stats(&out);
        let occupancy = if total_area > 0 {
            used_area as f64 / total_area as f64
        } else {
            0.0
        };
        let value = serde_json::json!({
            "pages": out.pages.len(),
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
                out.pages.len(),
                used_area,
                total_area,
                occupancy * 100.0
            );
        }
    }
    Ok(())
}

fn run_bench(b: &BenchArgs) -> anyhow::Result<()> {
    use std::time::Instant;
    // Minimal bench: build a tiny config from args; pack once and print time + occupancy
    let images = gather_paths(&b.input, &[], &[])?;
    let inputs = load_images_with_progress(&images, false)?;
    let family = match b.algorithm.to_ascii_lowercase().as_str() {
        "skyline" => AlgorithmFamily::Skyline,
        "maxrects" => AlgorithmFamily::MaxRects,
        "guillotine" => AlgorithmFamily::Guillotine,
        _ => AlgorithmFamily::Auto,
    };
    let auto_mode = match b.auto_mode.to_ascii_lowercase().as_str() {
        "fast" => AutoMode::Fast,
        _ => AutoMode::Quality,
    };
    let cfg = PackerConfig {
        family,
        auto_mode,
        time_budget_ms: b.time_budget,
        ..Default::default()
    };
    let start = Instant::now();
    let out = pack_images(inputs, cfg)?;
    let dur = start.elapsed();
    let (used, total) = compute_stats(&out);
    let occ = if total > 0 {
        used as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    println!(
        "pages={} occupancy={:.2}% time={}",
        out.pages.len(),
        occ,
        bench_fmt_dur(dur)
    );
    Ok(())
}

fn bench_fmt_dur(d: Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms >= 1.0 {
        format!("{:.1}ms", ms)
    } else {
        format!("{}us", d.as_micros())
    }
}

fn parse_algo(
    cli: &PackArgs,
) -> anyhow::Result<(
    AlgorithmFamily,
    MaxRectsHeuristic,
    SkylineHeuristic,
    GuillotineChoice,
    GuillotineSplit,
    AutoMode,
)> {
    let family = match cli.algorithm.to_ascii_lowercase().as_str() {
        "skyline" => AlgorithmFamily::Skyline,
        "maxrects" => AlgorithmFamily::MaxRects,
        "guillotine" => AlgorithmFamily::Guillotine,
        "auto" => AlgorithmFamily::Auto,
        other => anyhow::bail!("unknown algorithm: {}", other),
    };
    let h = match cli.heuristic.to_ascii_lowercase().as_str() {
        "baf" => MaxRectsHeuristic::BestAreaFit,
        "bssf" => MaxRectsHeuristic::BestShortSideFit,
        "blsf" => MaxRectsHeuristic::BestLongSideFit,
        "bl" => MaxRectsHeuristic::BottomLeft,
        "cp" => MaxRectsHeuristic::ContactPoint,
        other => anyhow::bail!("unknown heuristic: {}", other),
    };
    let sky = match cli.skyline.to_ascii_lowercase().as_str() {
        "bl" => SkylineHeuristic::BottomLeft,
        "minwaste" => SkylineHeuristic::MinWaste,
        other => anyhow::bail!("unknown skyline heuristic: {}", other),
    };
    let g_choice = match cli.g_choice.to_ascii_lowercase().as_str() {
        "baf" => GuillotineChoice::BestAreaFit,
        "bssf" => GuillotineChoice::BestShortSideFit,
        "blsf" => GuillotineChoice::BestLongSideFit,
        "waf" => GuillotineChoice::WorstAreaFit,
        "wssf" => GuillotineChoice::WorstShortSideFit,
        "wlsf" => GuillotineChoice::WorstLongSideFit,
        other => anyhow::bail!("unknown guillotine choice: {}", other),
    };
    let g_split = match cli.g_split.to_ascii_lowercase().as_str() {
        "slas" => GuillotineSplit::SplitShorterLeftoverAxis,
        "llas" => GuillotineSplit::SplitLongerLeftoverAxis,
        "minas" => GuillotineSplit::SplitMinimizeArea,
        "maxas" => GuillotineSplit::SplitMaximizeArea,
        "sas" => GuillotineSplit::SplitShorterAxis,
        "las" => GuillotineSplit::SplitLongerAxis,
        other => anyhow::bail!("unknown guillotine split: {}", other),
    };
    let auto_mode = match cli.auto_mode.to_ascii_lowercase().as_str() {
        "fast" => AutoMode::Fast,
        "quality" => AutoMode::Quality,
        other => anyhow::bail!("unknown auto mode: {}", other),
    };
    Ok((family, h, sky, g_choice, g_split, auto_mode))
}

#[allow(dead_code)]
fn fmt_dur(d: Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms >= 1.0 {
        format!("{:.1}ms", ms)
    } else {
        format!("{}µs", d.as_micros())
    }
}

fn gather_paths(
    path: &Path,
    include: &[String],
    exclude: &[String],
) -> anyhow::Result<Vec<PathBuf>> {
    // Build glob matchers
    let mut inc_set = None;
    if !include.is_empty() {
        let mut b = GlobSetBuilder::new();
        for pat in include {
            b.add(Glob::new(pat)?);
        }
        inc_set = Some(b.build()?);
    }
    let mut exc_set = None;
    if !exclude.is_empty() {
        let mut b = GlobSetBuilder::new();
        for pat in exclude {
            b.add(Glob::new(pat)?);
        }
        exc_set = Some(b.build()?);
    }
    let mut list: Vec<PathBuf> = Vec::new();
    if path.is_file() {
        if !should_skip(path, inc_set.as_ref(), exc_set.as_ref()) && is_image(path) {
            list.push(path.to_path_buf());
        }
    } else {
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() && !should_skip(p, inc_set.as_ref(), exc_set.as_ref()) && is_image(p) {
                list.push(p.to_path_buf());
            }
        }
    }
    Ok(list)
}

fn should_skip(
    p: &Path,
    include: Option<&globset::GlobSet>,
    exclude: Option<&globset::GlobSet>,
) -> bool {
    let s = p.to_string_lossy().replace('\\', "/");
    if let Some(ex) = exclude {
        if ex.is_match(&s) {
            return true;
        }
    }
    if let Some(inc) = include {
        if !inc.is_match(&s) {
            return true;
        }
    }
    false
}

fn is_image(p: &Path) -> bool {
    matches!(
        p.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase()),
        Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "tga" | "gif")
    )
}

fn load_images_with_progress(paths: &[PathBuf], progress: bool) -> anyhow::Result<Vec<InputImage>> {
    use indicatif::{ProgressBar, ProgressStyle};
    let bar = if progress {
        let b = ProgressBar::new(paths.len() as u64);
        b.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} loading {pos}/{len} [{elapsed_precise}] {wide_msg}",
            )
            .unwrap(),
        );
        Some(b)
    } else {
        None
    };
    let mut list = Vec::with_capacity(paths.len());
    for p in paths {
        let msg = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if let Some(b) = &bar {
            b.set_message(msg.to_string());
        }
        match load_image(p) {
            Ok(img) => {
                let key = p.to_string_lossy().replace('\\', "/");
                list.push(InputImage { key, image: img });
            }
            Err(e) => {
                error!(?p, error = %e, "skip image");
            }
        }
        if let Some(b) = &bar {
            b.inc(1);
        }
    }
    if let Some(b) = &bar {
        b.finish_and_clear();
    }
    Ok(list)
}

fn load_image(p: &Path) -> anyhow::Result<DynamicImage> {
    let img = ImageReader::open(p)?.with_guessed_format()?.decode()?;
    Ok(img)
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

fn init_tracing_with_level(quiet: bool, verbose: u8) {
    let level = if quiet {
        "error".to_string()
    } else {
        match verbose {
            0 => "info".into(),
            1 => "debug".into(),
            _ => "trace".into(),
        }
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(level)
        .with_target(false)
        .try_init();
}

use serde::Serialize;
#[derive(Serialize)]
struct TemplateSprite {
    name: String,
    frame: serde_json::Value,
    rotated: bool,
    trimmed: bool,
    sprite_source_size: serde_json::Value,
    source_size: serde_json::Value,
    pivot: serde_json::Value,
}

#[derive(Serialize)]
struct TemplatePage {
    image: String,
    size: serde_json::Value,
    sprites: Vec<TemplateSprite>,
}

#[derive(Serialize)]
struct TemplateContext {
    pages: Vec<TemplatePage>,
    meta: serde_json::Value,
}

fn build_template_context(
    out: &tex_packer_core::PackOutput,
    page_names: &[String],
) -> TemplateContext {
    let mut pages: Vec<TemplatePage> = Vec::new();
    for (idx, output_page) in out.pages.iter().enumerate() {
        let page = &output_page.page;
        let image = page_names
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("page_{}.png", page.id));
        let size = serde_json::json!({"w": page.width, "h": page.height});
        let mut sprites: Vec<TemplateSprite> = Vec::new();
        for fr in &page.frames {
            let frame = serde_json::json!({"x": fr.frame.x, "y": fr.frame.y, "w": fr.frame.w, "h": fr.frame.h});
            let sss = serde_json::json!({"x": fr.source.x, "y": fr.source.y, "w": fr.source.w, "h": fr.source.h});
            let ss = serde_json::json!({"w": fr.source_size.0, "h": fr.source_size.1});
            let pivot = serde_json::json!({"x": 0.5_f32, "y": 0.5_f32});
            sprites.push(TemplateSprite {
                name: fr.key.clone(),
                frame,
                rotated: fr.rotated,
                trimmed: fr.trimmed,
                sprite_source_size: sss,
                source_size: ss,
                pivot,
            });
        }
        pages.push(TemplatePage {
            image,
            size,
            sprites,
        });
    }
    let meta = serde_json::json!({
        "app": out.atlas.meta.app,
        "version": out.atlas.meta.version,
        "format": out.atlas.meta.format,
        "scale": out.atlas.meta.scale,
    });
    TemplateContext { pages, meta }
}

#[derive(Debug, Deserialize, Default)]
struct YamlConfig {
    family: Option<String>,
    skyline: Option<String>,
    heuristic: Option<String>,
    g_choice: Option<String>,
    g_split: Option<String>,
    auto_mode: Option<String>,
    max_width: Option<u32>,
    max_height: Option<u32>,
    allow_rotation: Option<bool>,
    force_max_dimensions: Option<bool>,
    border_padding: Option<u32>,
    texture_padding: Option<u32>,
    texture_extrusion: Option<u32>,
    trim: Option<bool>,
    trim_threshold: Option<u8>,
    texture_outlines: Option<bool>,
    power_of_two: Option<bool>,
    square: Option<bool>,
    use_waste_map: Option<bool>,
    sort_order: Option<String>,
    time_budget_ms: Option<u64>,
    parallel: Option<bool>,
    mr_reference: Option<bool>,
    auto_mr_ref_time_ms_threshold: Option<u64>,
    auto_mr_ref_input_threshold: Option<usize>,
    transparent_policy: Option<String>,
}

impl YamlConfig {
    fn into_packer_config(self, mut cfg: PackerConfig) -> PackerConfig {
        if let Some(v) = self.max_width {
            cfg.max_width = v;
        }
        if let Some(v) = self.max_height {
            cfg.max_height = v;
        }
        if let Some(v) = self.allow_rotation {
            cfg.allow_rotation = v;
        }
        if let Some(v) = self.force_max_dimensions {
            cfg.force_max_dimensions = v;
        }
        if let Some(v) = self.border_padding {
            cfg.border_padding = v;
        }
        if let Some(v) = self.texture_padding {
            cfg.texture_padding = v;
        }
        if let Some(v) = self.texture_extrusion {
            cfg.texture_extrusion = v;
        }
        if let Some(v) = self.trim {
            cfg.trim = v;
        }
        if let Some(v) = self.trim_threshold {
            cfg.trim_threshold = v;
        }
        if let Some(v) = self.texture_outlines {
            cfg.texture_outlines = v;
        }
        if let Some(v) = self.power_of_two {
            cfg.power_of_two = v;
        }
        if let Some(v) = self.square {
            cfg.square = v;
        }
        if let Some(v) = self.use_waste_map {
            cfg.use_waste_map = v;
        }
        if let Some(v) = self.sort_order {
            cfg.sort_order = parse_sort_order(&v).unwrap_or(cfg.sort_order);
        }
        if let Some(v) = self.time_budget_ms {
            cfg.time_budget_ms = Some(v);
        }
        if let Some(v) = self.parallel {
            cfg.parallel = v;
        }
        if let Some(v) = self.mr_reference {
            cfg.mr_reference = v;
        }
        if let Some(v) = self.family {
            cfg.family = v.parse().unwrap_or(cfg.family);
        }
        if let Some(v) = self.skyline {
            cfg.skyline_heuristic = v.parse().unwrap_or(cfg.skyline_heuristic);
        }
        if let Some(v) = self.heuristic {
            cfg.mr_heuristic = v.parse().unwrap_or(cfg.mr_heuristic);
        }
        if let Some(v) = self.g_choice {
            cfg.g_choice = v.parse().unwrap_or(cfg.g_choice);
        }
        if let Some(v) = self.g_split {
            cfg.g_split = v.parse().unwrap_or(cfg.g_split);
        }
        if let Some(v) = self.auto_mode {
            cfg.auto_mode = match v.to_ascii_lowercase().as_str() {
                "fast" => AutoMode::Fast,
                "quality" => AutoMode::Quality,
                _ => cfg.auto_mode,
            };
        }
        if let Some(v) = self.auto_mr_ref_time_ms_threshold {
            cfg.auto_mr_ref_time_ms_threshold = Some(v);
        }
        if let Some(v) = self.auto_mr_ref_input_threshold {
            cfg.auto_mr_ref_input_threshold = Some(v);
        }
        if let Some(v) = self.transparent_policy {
            cfg.transparent_policy = v.parse().unwrap_or(cfg.transparent_policy);
        }
        cfg
    }
}

fn parse_sort_order(s: &str) -> anyhow::Result<SortOrder> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "area_desc" => SortOrder::AreaDesc,
        "max_side_desc" => SortOrder::MaxSideDesc,
        "height_desc" => SortOrder::HeightDesc,
        "width_desc" => SortOrder::WidthDesc,
        "name_asc" => SortOrder::NameAsc,
        "none" => SortOrder::None,
        other => anyhow::bail!("unknown sort order: {}", other),
    })
}
