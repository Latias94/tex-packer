use std::path::PathBuf;
use std::time::Duration;

use clap::{ArgAction, Parser, Subcommand};
use tex_packer_core::offline::OfflinePacker;

mod config_adapter;
mod input_loader;
mod output_writer;
mod pack_command;

#[cfg(test)]
mod test_support {
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use clap::Parser;

    use crate::PackArgs;

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    pub(crate) struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        pub(crate) fn new(label: &str) -> std::io::Result<Self> {
            let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "tex-packer-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path)?;
            Ok(Self { path })
        }

        pub(crate) fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            if let Ok(entries) = fs::read_dir(&self.path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let _ = fs::remove_file(path);
                    }
                }
            }
            let _ = fs::remove_dir(&self.path);
        }
    }

    pub(crate) fn pack_args(out_dir: &Path) -> PackArgs {
        PackArgs::try_parse_from([
            OsString::from("tex-packer-test"),
            OsString::from("unused-input"),
            OsString::from("--out-dir"),
            out_dir.as_os_str().to_owned(),
        ])
        .expect("test pack arguments must parse")
    }
}

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
    /// Show progress bars (disable with --progress false or --quiet)
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
    /// Time one pack and print content/allocation occupancy
    Bench(BenchArgs),
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct PackArgs {
    // Input/Output
    /// Input file or directory
    #[arg(help_heading = "Input/Output")]
    pub(crate) input: PathBuf,
    /// Output directory
    #[arg(
        short,
        long = "out-dir",
        alias = "out",
        default_value = "out",
        help_heading = "Input/Output"
    )]
    pub(crate) out_dir: PathBuf,
    /// Atlas base name (files will be name.png/.json)
    #[arg(short, long, default_value = "atlas", help_heading = "Input/Output")]
    pub(crate) name: String,
    /// YAML config file path (overrides algorithm-related options)
    #[arg(long, help_heading = "Input/Output")]
    pub(crate) config: Option<PathBuf>,
    /// Include patterns (glob). If set, only files matching any pattern are considered
    #[arg(long, help_heading = "Input/Output")]
    pub(crate) include: Vec<String>,
    /// Exclude patterns (glob). Files matching any pattern will be ignored
    #[arg(long, help_heading = "Input/Output")]
    pub(crate) exclude: Vec<String>,

    // Layout
    /// Max width
    #[arg(long, default_value_t = 1024, help_heading = "Layout")]
    pub(crate) max_width: u32,
    /// Max height
    #[arg(long, default_value_t = 1024, help_heading = "Layout")]
    pub(crate) max_height: u32,
    /// Force output size to max_width/max_height
    #[arg(long, default_value_t = false, help_heading = "Layout")]
    pub(crate) force_max_dimensions: bool,
    /// Resize page dims to power of two
    #[arg(long, default_value_t = false, help_heading = "Layout")]
    pub(crate) pow2: bool,
    /// Force square page
    #[arg(long, default_value_t = false, help_heading = "Layout")]
    pub(crate) square: bool,
    /// Sort order: area_desc|max_side_desc|height_desc|width_desc|name_asc|none
    #[arg(long, default_value = "area_desc", help_heading = "Layout")]
    pub(crate) sort_order: String,

    // Image Processing
    /// Allow rotation (90deg)
    #[arg(long, default_value_t = true, help_heading = "Image Processing")]
    pub(crate) allow_rotation: bool,
    /// Border padding (around entire page)
    #[arg(long, default_value_t = 0, help_heading = "Image Processing")]
    pub(crate) border_padding: u32,
    /// Padding between frames
    #[arg(long, default_value_t = 2, help_heading = "Image Processing")]
    pub(crate) texture_padding: u32,
    /// Extrude pixels around each frame
    #[arg(long, default_value_t = 0, help_heading = "Image Processing")]
    pub(crate) texture_extrusion: u32,
    /// Trim transparent borders
    #[arg(long, default_value_t = true, help_heading = "Image Processing")]
    pub(crate) trim: bool,
    /// Trim alpha threshold (0..=255)
    #[arg(long, default_value_t = 0, help_heading = "Image Processing")]
    pub(crate) trim_threshold: u8,
    /// Draw red outlines (debug)
    #[arg(long, default_value_t = false, help_heading = "Image Processing")]
    pub(crate) outlines: bool,
    /// Layout-only: compute placements and export metadata (no PNGs)
    #[arg(long, default_value_t = false, help_heading = "Export")]
    pub(crate) layout_only: bool,

    // Algorithms/Heuristics/Auto
    /// Algorithm: skyline | maxrects | guillotine | auto
    #[arg(long, value_parser = ["skyline", "maxrects", "guillotine", "auto"], default_value = "skyline", help_heading = "Algorithms")]
    pub(crate) algorithm: String,
    /// MaxRects heuristic: baf|bssf|blsf|bl|cp
    #[arg(long, default_value = "baf", help_heading = "Heuristics")]
    pub(crate) heuristic: String,
    /// Skyline heuristic: bl|minwaste
    #[arg(long, default_value = "bl", help_heading = "Heuristics")]
    pub(crate) skyline: String,
    /// Guillotine choice: baf|bssf|blsf|waf|wssf|wlsf
    #[arg(long, default_value = "baf", help_heading = "Heuristics")]
    pub(crate) g_choice: String,
    /// Guillotine split: slas|llas|minas|maxas|sas|las
    #[arg(long, default_value = "slas", help_heading = "Heuristics")]
    pub(crate) g_split: String,
    /// Auto mode: fast | quality
    #[arg(long, default_value = "quality", help_heading = "Auto/Portfolio")]
    pub(crate) auto_mode: String,
    /// Time budget for auto mode (ms)
    #[arg(long, help_heading = "Auto/Portfolio")]
    pub(crate) time_budget: Option<u64>,
    /// Evaluate auto candidates in parallel (requires core feature `parallel`)
    #[arg(long, default_value_t = false, help_heading = "Auto/Portfolio")]
    pub(crate) parallel: bool,
    /// Use waste map for skyline
    #[arg(long, default_value_t = false, help_heading = "Heuristics")]
    pub(crate) use_waste_map: bool,
    /// Policy for fully transparent images when trim is on: keep | one_by_one | skip
    #[arg(long, default_value = "keep", help_heading = "Image Processing")]
    pub(crate) transparent_policy: String,
    /// Use reference-accurate MaxRects split/prune (SplitFreeNode style)
    #[arg(long, default_value_t = false, help_heading = "Auto/Portfolio")]
    pub(crate) mr_reference: bool,
    /// Auto: enable mr_reference when time budget >= this (ms) (overrides default heuristic)
    #[arg(long, help_heading = "Auto/Portfolio")]
    pub(crate) auto_mr_ref_time_threshold: Option<u64>,
    /// Auto: enable mr_reference when inputs >= this count (overrides default heuristic)
    #[arg(long, help_heading = "Auto/Portfolio")]
    pub(crate) auto_mr_ref_input_threshold: Option<usize>,

    // Export
    /// Metadata format: json-array | json (alias) | json-hash | plist | template
    #[arg(long, default_value = "json-array", help_heading = "Export")]
    pub(crate) metadata: String,
    /// Built-in engine template: unity | godot | phaser3 | phaser3_single | spine | cocos | unreal
    #[arg(long, help_heading = "Export")]
    pub(crate) engine: Option<String>,
    /// External template file (handlebars), used when --metadata template
    #[arg(long, help_heading = "Export")]
    pub(crate) template: Option<PathBuf>,
    /// Export packing stats (JSON) to this file
    #[arg(long, help_heading = "Export")]
    pub(crate) export_stats: Option<PathBuf>,
    /// Print the merged configuration (after CLI/YAML) and exit
    #[arg(long, default_value_t = false, help_heading = "Export")]
    pub(crate) print_config: bool,
    /// Output format for --print-config: json|yaml
    #[arg(long, default_value = "json", value_parser = ["json", "yaml"], help_heading = "Export")]
    pub(crate) print_config_format: String,
    /// Dry run: compute layout and stats but do not write files
    #[arg(long, default_value_t = false, help_heading = "Export")]
    pub(crate) dry_run: bool,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct BenchArgs {
    /// Input directory
    pub(crate) input: PathBuf,
    /// Algorithm: skyline | maxrects | guillotine | auto
    #[arg(long, value_parser = ["skyline", "maxrects", "guillotine", "auto"], default_value = "auto")]
    pub(crate) algorithm: String,
    /// Auto mode: fast | quality
    #[arg(long, default_value = "quality")]
    pub(crate) auto_mode: String,
    /// Time budget for auto mode (ms)
    #[arg(long)]
    pub(crate) time_budget: Option<u64>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing_with_level(cli.quiet, cli.verbose);
    match &cli.command {
        Commands::Pack(args) => pack_command::run_pack(args, cli.progress && !cli.quiet),
        Commands::Template(args) => {
            let mut a = args.clone();
            a.metadata = "template".into();
            pack_command::run_pack(&a, cli.progress && !cli.quiet)
        }
        Commands::Layout(args) => {
            let mut a = args.clone();
            a.layout_only = true;
            pack_command::run_pack(&a, false)
        }
        Commands::Bench(b) => run_bench(b),
    }
}

fn run_bench(b: &BenchArgs) -> anyhow::Result<()> {
    use std::time::Instant;
    let images = input_loader::gather_paths(&b.input, &[], &[])?;
    let inputs = input_loader::load_images_with_progress(&images, false)?;
    let cfg = config_adapter::build_bench_config(b)?;
    let start = Instant::now();
    let out = OfflinePacker::new(cfg).pack_images(inputs)?;
    let dur = start.elapsed();
    let stats = out.stats();
    println!(
        "pages={} content_occupancy={:.2}% allocation_occupancy={:.2}% time={}",
        out.atlas().pages().len(),
        stats.content_occupancy * 100.0,
        stats.allocation_occupancy * 100.0,
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
