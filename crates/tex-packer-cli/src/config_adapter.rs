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

#[cfg(test)]
mod tests {
    use super::*;
    use tex_packer_core::config::{SortOrder, TransparentPolicy};

    fn apply_yaml(yaml: &str, base: PackerConfig) -> anyhow::Result<PackerConfig> {
        serde_yaml::from_str::<YamlConfig>(yaml)?.apply_to(base)
    }

    macro_rules! assert_enum_cases {
        ($key:literal, $field:ident, [$(($raw:literal, $expected:expr)),+ $(,)?]) => {
            $(
                let yaml = format!("{}: {}", $key, $raw);
                let cfg = apply_yaml(&yaml, PackerConfig::default()).unwrap();
                assert_eq!(cfg.$field, $expected, "{}: {}", $key, $raw);
            )+
        };
    }

    #[test]
    fn yaml_maps_every_supported_key() {
        let yaml = r#"
family: guillotine
skyline: minwaste
heuristic: contactpoint
g_choice: worstlongsidefit
g_split: splitlongeraxis
auto_mode: fast
max_width: 2048
max_height: 1536
allow_rotation: false
force_max_dimensions: true
border_padding: 7
texture_padding: 11
texture_extrusion: 3
trim: false
trim_threshold: 17
texture_outlines: true
power_of_two: true
square: true
use_waste_map: true
sort_order: name_asc
time_budget_ms: 850
parallel: true
mr_reference: true
auto_mr_ref_time_ms_threshold: 600
auto_mr_ref_input_threshold: 900
transparent_policy: one_by_one
"#;

        let cfg = apply_yaml(yaml, PackerConfig::default()).unwrap();

        assert_eq!(cfg.family, AlgorithmFamily::Guillotine);
        assert_eq!(cfg.skyline_heuristic, SkylineHeuristic::MinWaste);
        assert_eq!(cfg.mr_heuristic, MaxRectsHeuristic::ContactPoint);
        assert_eq!(cfg.g_choice, GuillotineChoice::WorstLongSideFit);
        assert_eq!(cfg.g_split, GuillotineSplit::SplitLongerAxis);
        assert_eq!(cfg.auto_mode, AutoMode::Fast);
        assert_eq!(cfg.max_width, 2048);
        assert_eq!(cfg.max_height, 1536);
        assert!(!cfg.allow_rotation);
        assert!(cfg.force_max_dimensions);
        assert_eq!(cfg.border_padding, 7);
        assert_eq!(cfg.texture_padding, 11);
        assert_eq!(cfg.texture_extrusion, 3);
        assert!(!cfg.trim);
        assert_eq!(cfg.trim_threshold, 17);
        assert!(cfg.texture_outlines);
        assert!(cfg.power_of_two);
        assert!(cfg.square);
        assert!(cfg.use_waste_map);
        assert_eq!(cfg.sort_order, SortOrder::NameAsc);
        assert_eq!(cfg.time_budget_ms, Some(850));
        assert!(cfg.parallel);
        assert!(cfg.mr_reference);
        assert_eq!(cfg.auto_mr_ref_time_ms_threshold, Some(600));
        assert_eq!(cfg.auto_mr_ref_input_threshold, Some(900));
        assert_eq!(cfg.transparent_policy, TransparentPolicy::OneByOne);
    }

    #[test]
    fn yaml_accepts_every_supported_enum_spelling() {
        assert_enum_cases!(
            "family",
            family,
            [
                ("skyline", AlgorithmFamily::Skyline),
                ("maxrects", AlgorithmFamily::MaxRects),
                ("guillotine", AlgorithmFamily::Guillotine),
                ("auto", AlgorithmFamily::Auto),
            ]
        );
        assert_enum_cases!(
            "skyline",
            skyline_heuristic,
            [
                ("bl", SkylineHeuristic::BottomLeft),
                ("bottomleft", SkylineHeuristic::BottomLeft),
                ("minwaste", SkylineHeuristic::MinWaste),
                ("mw", SkylineHeuristic::MinWaste),
            ]
        );
        assert_enum_cases!(
            "heuristic",
            mr_heuristic,
            [
                ("baf", MaxRectsHeuristic::BestAreaFit),
                ("bestareafit", MaxRectsHeuristic::BestAreaFit),
                ("bssf", MaxRectsHeuristic::BestShortSideFit),
                ("bestshortsidefit", MaxRectsHeuristic::BestShortSideFit),
                ("blsf", MaxRectsHeuristic::BestLongSideFit),
                ("bestlongsidefit", MaxRectsHeuristic::BestLongSideFit),
                ("bl", MaxRectsHeuristic::BottomLeft),
                ("bottomleft", MaxRectsHeuristic::BottomLeft),
                ("cp", MaxRectsHeuristic::ContactPoint),
                ("contactpoint", MaxRectsHeuristic::ContactPoint),
            ]
        );
        assert_enum_cases!(
            "g_choice",
            g_choice,
            [
                ("baf", GuillotineChoice::BestAreaFit),
                ("bestareafit", GuillotineChoice::BestAreaFit),
                ("bssf", GuillotineChoice::BestShortSideFit),
                ("bestshortsidefit", GuillotineChoice::BestShortSideFit),
                ("blsf", GuillotineChoice::BestLongSideFit),
                ("bestlongsidefit", GuillotineChoice::BestLongSideFit),
                ("waf", GuillotineChoice::WorstAreaFit),
                ("worstareafit", GuillotineChoice::WorstAreaFit),
                ("wssf", GuillotineChoice::WorstShortSideFit),
                ("worstshortsidefit", GuillotineChoice::WorstShortSideFit),
                ("wlsf", GuillotineChoice::WorstLongSideFit),
                ("worstlongsidefit", GuillotineChoice::WorstLongSideFit),
            ]
        );
        assert_enum_cases!(
            "g_split",
            g_split,
            [
                ("slas", GuillotineSplit::SplitShorterLeftoverAxis),
                (
                    "splitshorterleftoveraxis",
                    GuillotineSplit::SplitShorterLeftoverAxis
                ),
                ("llas", GuillotineSplit::SplitLongerLeftoverAxis),
                (
                    "splitlongerleftoveraxis",
                    GuillotineSplit::SplitLongerLeftoverAxis
                ),
                ("minas", GuillotineSplit::SplitMinimizeArea),
                ("splitminimizearea", GuillotineSplit::SplitMinimizeArea),
                ("maxas", GuillotineSplit::SplitMaximizeArea),
                ("splitmaximizearea", GuillotineSplit::SplitMaximizeArea),
                ("sas", GuillotineSplit::SplitShorterAxis),
                ("splitshorteraxis", GuillotineSplit::SplitShorterAxis),
                ("las", GuillotineSplit::SplitLongerAxis),
                ("splitlongeraxis", GuillotineSplit::SplitLongerAxis),
            ]
        );
        assert_enum_cases!(
            "auto_mode",
            auto_mode,
            [("fast", AutoMode::Fast), ("quality", AutoMode::Quality),]
        );
        assert_enum_cases!(
            "sort_order",
            sort_order,
            [
                ("area_desc", SortOrder::AreaDesc),
                ("max_side_desc", SortOrder::MaxSideDesc),
                ("height_desc", SortOrder::HeightDesc),
                ("width_desc", SortOrder::WidthDesc),
                ("name_asc", SortOrder::NameAsc),
                ("none", SortOrder::None),
            ]
        );
        assert_enum_cases!(
            "transparent_policy",
            transparent_policy,
            [
                ("keep", TransparentPolicy::Keep),
                ("one_by_one", TransparentPolicy::OneByOne),
                ("1x1", TransparentPolicy::OneByOne),
                ("onebyone", TransparentPolicy::OneByOne),
                ("skip", TransparentPolicy::Skip),
            ]
        );
    }

    #[test]
    fn yaml_enum_values_are_ascii_case_insensitive() {
        let cfg = apply_yaml(
            "family: MAXRECTS\nskyline: MINWASTE\ntransparent_policy: ONEBYONE",
            PackerConfig::default(),
        )
        .unwrap();

        assert_eq!(cfg.family, AlgorithmFamily::MaxRects);
        assert_eq!(cfg.skyline_heuristic, SkylineHeuristic::MinWaste);
        assert_eq!(cfg.transparent_policy, TransparentPolicy::OneByOne);
    }

    #[test]
    fn yaml_boolean_scalars_use_yaml_1_2_resolution() {
        for raw in ["true", "True", "TRUE"] {
            let cfg = apply_yaml(&format!("trim: {raw}"), PackerConfig::default()).unwrap();
            assert!(cfg.trim, "trim: {raw}");
        }
        for raw in ["false", "False", "FALSE"] {
            let cfg = apply_yaml(&format!("trim: {raw}"), PackerConfig::default()).unwrap();
            assert!(!cfg.trim, "trim: {raw}");
        }
    }

    #[test]
    fn yaml_1_1_only_boolean_words_are_not_coerced() {
        for raw in [
            "y", "Y", "yes", "Yes", "YES", "n", "N", "no", "No", "NO", "on", "On", "ON", "off",
            "Off", "OFF",
        ] {
            let error = serde_yaml::from_str::<YamlConfig>(&format!("trim: {raw}"))
                .expect_err("YAML 1.1-only boolean words are strings in the current parser");
            let message = error.to_string();
            assert!(message.contains("invalid type: string"), "{raw}: {message}");
            assert!(message.contains("expected a boolean"), "{raw}: {message}");
        }
    }

    #[test]
    fn yaml_1_1_boolean_words_in_enum_fields_remain_strings() {
        for raw in ["yes", "no", "on", "off"] {
            let error = apply_yaml(&format!("family: {raw}"), PackerConfig::default())
                .expect_err("the token should reach enum parsing as a string");
            assert_eq!(
                error.to_string(),
                format!("unknown algorithm family: {raw}")
            );
        }
    }

    #[test]
    fn yaml_null_optional_values_leave_the_overlay_base_unchanged() {
        let base = PackerConfig {
            time_budget_ms: Some(400),
            auto_mr_ref_time_ms_threshold: Some(200),
            auto_mr_ref_input_threshold: Some(800),
            ..Default::default()
        };

        let cfg = apply_yaml(
            "time_budget_ms: null\nauto_mr_ref_time_ms_threshold: null\nauto_mr_ref_input_threshold: null",
            base,
        )
        .unwrap();

        assert_eq!(cfg.time_budget_ms, Some(400));
        assert_eq!(cfg.auto_mr_ref_time_ms_threshold, Some(200));
        assert_eq!(cfg.auto_mr_ref_input_threshold, Some(800));
    }

    #[test]
    fn yaml_unknown_fields_are_currently_ignored() {
        let cfg = apply_yaml(
            "max_width: 640\nfuture_option: enabled",
            PackerConfig::default(),
        )
        .unwrap();

        assert_eq!(cfg.max_width, 640);
    }

    #[test]
    fn yaml_tags_are_currently_ignored() {
        let cfg = apply_yaml(
            "max_width: !pixels 640\nallow_rotation: !switch false",
            PackerConfig::default(),
        )
        .unwrap();

        assert_eq!(cfg.max_width, 640);
        assert!(!cfg.allow_rotation);
    }

    #[test]
    fn yaml_merge_keys_are_currently_ignored_without_expansion() {
        let cfg = apply_yaml(
            r#"
defaults: &defaults
  max_width: 640
  allow_rotation: false
<<: *defaults
max_height: 720
"#,
            PackerConfig::default(),
        )
        .unwrap();

        assert_eq!(cfg.max_width, PackerConfig::default().max_width);
        assert_eq!(cfg.allow_rotation, PackerConfig::default().allow_rotation);
        assert_eq!(cfg.max_height, 720);
    }

    #[test]
    fn yaml_duplicate_keys_are_rejected() {
        let error = serde_yaml::from_str::<YamlConfig>("max_width: 640\nmax_width: 720")
            .expect_err("duplicate YAML keys must be rejected");
        let message = error.to_string();

        assert!(message.contains("duplicate field `max_width`"), "{message}");
    }

    #[test]
    fn printed_yaml_round_trips_the_shared_field_names() {
        let source = PackerConfig {
            max_width: 2048,
            max_height: 1536,
            allow_rotation: false,
            force_max_dimensions: true,
            border_padding: 7,
            texture_padding: 11,
            texture_extrusion: 3,
            trim: false,
            trim_threshold: 17,
            texture_outlines: true,
            power_of_two: true,
            square: true,
            use_waste_map: true,
            family: AlgorithmFamily::Guillotine,
            mr_heuristic: MaxRectsHeuristic::ContactPoint,
            skyline_heuristic: SkylineHeuristic::MinWaste,
            g_choice: GuillotineChoice::WorstLongSideFit,
            g_split: GuillotineSplit::SplitLongerAxis,
            auto_mode: AutoMode::Fast,
            sort_order: SortOrder::NameAsc,
            time_budget_ms: Some(850),
            parallel: true,
            mr_reference: true,
            auto_mr_ref_time_ms_threshold: Some(600),
            auto_mr_ref_input_threshold: Some(900),
            transparent_policy: TransparentPolicy::OneByOne,
        };
        let printed = serde_yaml::to_string(&source).unwrap();
        let reparsed = apply_yaml(&printed, PackerConfig::default()).unwrap();

        assert_eq!(reparsed.max_width, source.max_width);
        assert_eq!(reparsed.max_height, source.max_height);
        assert_eq!(reparsed.allow_rotation, source.allow_rotation);
        assert_eq!(reparsed.force_max_dimensions, source.force_max_dimensions);
        assert_eq!(reparsed.border_padding, source.border_padding);
        assert_eq!(reparsed.texture_padding, source.texture_padding);
        assert_eq!(reparsed.texture_extrusion, source.texture_extrusion);
        assert_eq!(reparsed.trim, source.trim);
        assert_eq!(reparsed.trim_threshold, source.trim_threshold);
        assert_eq!(reparsed.texture_outlines, source.texture_outlines);
        assert_eq!(reparsed.power_of_two, source.power_of_two);
        assert_eq!(reparsed.square, source.square);
        assert_eq!(reparsed.use_waste_map, source.use_waste_map);
        assert_eq!(reparsed.family, source.family);
        assert_eq!(reparsed.g_choice, source.g_choice);
        assert_eq!(reparsed.g_split, source.g_split);
        assert_eq!(reparsed.auto_mode, source.auto_mode);
        assert_eq!(reparsed.sort_order, source.sort_order);
        assert_eq!(reparsed.time_budget_ms, source.time_budget_ms);
        assert_eq!(reparsed.parallel, source.parallel);
        assert_eq!(reparsed.mr_reference, source.mr_reference);
        assert_eq!(
            reparsed.auto_mr_ref_time_ms_threshold,
            source.auto_mr_ref_time_ms_threshold
        );
        assert_eq!(
            reparsed.auto_mr_ref_input_threshold,
            source.auto_mr_ref_input_threshold
        );
        assert_eq!(reparsed.transparent_policy, source.transparent_policy);

        assert_eq!(reparsed.mr_heuristic, PackerConfig::default().mr_heuristic);
        assert_eq!(
            reparsed.skyline_heuristic,
            PackerConfig::default().skyline_heuristic
        );
        assert!(printed.contains("mr_heuristic:"));
        assert!(printed.contains("skyline_heuristic:"));
    }

    #[test]
    #[ignore = "U7: reject unknown YAML fields instead of silently ignoring them"]
    fn yaml_unknown_fields_must_be_rejected() {
        apply_yaml("future_option: enabled", PackerConfig::default())
            .expect_err("U7 must deny unknown fields");
    }

    #[test]
    #[ignore = "U7: reject YAML tags instead of silently discarding them"]
    fn yaml_tags_must_be_rejected() {
        apply_yaml("max_width: !pixels 640", PackerConfig::default())
            .expect_err("U7 must reject YAML tags");
    }

    #[test]
    #[ignore = "U7: reject YAML merge keys instead of silently ignoring them"]
    fn yaml_merge_keys_must_be_rejected() {
        apply_yaml(
            "defaults: &defaults { max_width: 640 }\n<<: *defaults",
            PackerConfig::default(),
        )
        .expect_err("U7 must reject YAML merge keys");
    }

    #[test]
    #[ignore = "U7: print YAML through the CLI DTO aliases for a complete round trip"]
    fn printed_yaml_must_round_trip_heuristic_fields() {
        let source = PackerConfig {
            mr_heuristic: MaxRectsHeuristic::ContactPoint,
            skyline_heuristic: SkylineHeuristic::MinWaste,
            ..Default::default()
        };
        let printed = serde_yaml::to_string(&source).unwrap();
        let reparsed = apply_yaml(&printed, PackerConfig::default()).unwrap();

        assert_eq!(reparsed.mr_heuristic, source.mr_heuristic);
        assert_eq!(reparsed.skyline_heuristic, source.skyline_heuristic);
    }
}
