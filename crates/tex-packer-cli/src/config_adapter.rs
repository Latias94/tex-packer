use std::fs;
use std::str::FromStr;

use anyhow::Context;
use serde::Deserialize;
use tex_packer_core::PackerConfig;
use tex_packer_core::config::{
    AlgorithmFamily, AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic,
    SkylineHeuristic,
};

use crate::{BenchArgs, PackArgs};

type AlgorithmSelection = (
    AlgorithmFamily,
    MaxRectsHeuristic,
    SkylineHeuristic,
    GuillotineChoice,
    GuillotineSplit,
    AutoMode,
);

pub(crate) fn build_pack_config(cli: &PackArgs) -> anyhow::Result<PackerConfig> {
    let mut cfg = pack_config_from_cli(cli)?;

    if let Some(path) = &cli.config {
        let file = fs::read_to_string(path)
            .with_context(|| format!("read config file {}", path.display()))?;
        let yaml: YamlConfig = serde_yaml::from_str(&file)
            .with_context(|| format!("parse config file {}", path.display()))?;
        cfg = yaml
            .apply_to(cfg)
            .with_context(|| format!("apply config file {}", path.display()))?;

        // Preserve the historical CLI override: YAML may set false/true, while
        // the flag can still force reference MaxRects on because clap booleans
        // cannot distinguish "absent" from "false" here.
        if cli.mr_reference {
            cfg.mr_reference = true;
        }
    }

    cfg.validate()
        .with_context(|| "invalid packer configuration")?;
    Ok(cfg)
}

pub(crate) fn build_bench_config(bench: &BenchArgs) -> anyhow::Result<PackerConfig> {
    let cfg = PackerConfig {
        family: parse_field("algorithm", &bench.algorithm)?,
        auto_mode: parse_field("auto mode", &bench.auto_mode)?,
        time_budget_ms: bench.time_budget,
        ..Default::default()
    };
    cfg.validate()
        .with_context(|| "invalid bench packer configuration")?;
    Ok(cfg)
}

fn pack_config_from_cli(cli: &PackArgs) -> anyhow::Result<PackerConfig> {
    let (family, mr_heuristic, sky_heuristic, g_choice, g_split, auto_mode) =
        parse_algorithm_selection(cli)?;

    Ok(PackerConfig {
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
        sort_order: parse_field("sort order", &cli.sort_order)?,
        time_budget_ms: cli.time_budget,
        parallel: cli.parallel,
        mr_reference: cli.mr_reference,
        auto_mr_ref_time_ms_threshold: cli.auto_mr_ref_time_threshold,
        auto_mr_ref_input_threshold: cli.auto_mr_ref_input_threshold,
        transparent_policy: parse_field("transparent policy", &cli.transparent_policy)?,
    })
}

fn parse_algorithm_selection(cli: &PackArgs) -> anyhow::Result<AlgorithmSelection> {
    Ok((
        parse_field("algorithm", &cli.algorithm)?,
        parse_field("MaxRects heuristic", &cli.heuristic)?,
        parse_field("Skyline heuristic", &cli.skyline)?,
        parse_field("Guillotine choice", &cli.g_choice)?,
        parse_field("Guillotine split", &cli.g_split)?,
        parse_field("auto mode", &cli.auto_mode)?,
    ))
}

fn parse_field<T>(field: &'static str, raw: &str) -> anyhow::Result<T>
where
    T: FromStr,
{
    raw.parse()
        .map_err(|_| anyhow::anyhow!("unknown {field}: {raw}"))
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
    fn apply_to(self, mut cfg: PackerConfig) -> anyhow::Result<PackerConfig> {
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
            cfg.sort_order = parse_field("sort order", &v)?;
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
            cfg.family = parse_field("algorithm family", &v)?;
        }
        if let Some(v) = self.skyline {
            cfg.skyline_heuristic = parse_field("Skyline heuristic", &v)?;
        }
        if let Some(v) = self.heuristic {
            cfg.mr_heuristic = parse_field("MaxRects heuristic", &v)?;
        }
        if let Some(v) = self.g_choice {
            cfg.g_choice = parse_field("Guillotine choice", &v)?;
        }
        if let Some(v) = self.g_split {
            cfg.g_split = parse_field("Guillotine split", &v)?;
        }
        if let Some(v) = self.auto_mode {
            cfg.auto_mode = parse_field("auto mode", &v)?;
        }
        if let Some(v) = self.auto_mr_ref_time_ms_threshold {
            cfg.auto_mr_ref_time_ms_threshold = Some(v);
        }
        if let Some(v) = self.auto_mr_ref_input_threshold {
            cfg.auto_mr_ref_input_threshold = Some(v);
        }
        if let Some(v) = self.transparent_policy {
            cfg.transparent_policy = parse_field("transparent policy", &v)?;
        }
        Ok(cfg)
    }
}
