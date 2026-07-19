use std::fs;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tex_packer_core::config::{
    AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic, OfflineConfig, PackingStrategy,
    PageConfig, SkylineHeuristic, SortOrder, TransparentPolicy,
};

use crate::{BenchArgs, PackArgs};

#[derive(Debug)]
pub(crate) struct ResolvedPackConfig {
    offline: OfflineConfig,
    printable: FlatConfigDto,
}

impl ResolvedPackConfig {
    pub(crate) fn into_offline(self) -> OfflineConfig {
        self.offline
    }

    pub(crate) fn print(&self, format: &str) -> anyhow::Result<String> {
        match format {
            "yaml" => Ok(serde_yaml_ng::to_string(&self.printable)?),
            _ => Ok(serde_json::to_string_pretty(&self.printable)?),
        }
    }
}

pub(crate) fn build_pack_config(cli: &PackArgs) -> anyhow::Result<ResolvedPackConfig> {
    let mut draft = FlatConfigDto::from_cli(cli)?;

    if let Some(path) = &cli.config {
        let file = fs::read_to_string(path)
            .with_context(|| format!("read config file {}", path.display()))?;
        let patch = parse_yaml_patch(&file)
            .with_context(|| format!("parse config file {}", path.display()))?;
        apply_yaml_patch(&mut draft, patch, cli.mr_reference)
            .with_context(|| format!("apply config file {}", path.display()))?;
    }

    let offline = draft.to_offline_config()?;
    Ok(ResolvedPackConfig {
        offline,
        printable: draft,
    })
}

fn apply_yaml_patch(
    draft: &mut FlatConfigDto,
    patch: FlatConfigPatch,
    force_reference: bool,
) -> anyhow::Result<()> {
    draft.apply_patch(patch)?;
    // This is the only historical post-YAML CLI override.
    if force_reference {
        draft.mr_reference = true;
    }
    Ok(())
}

fn parse_yaml_patch(yaml: &str) -> anyhow::Result<FlatConfigPatch> {
    let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml)?;
    validate_yaml_structure(&value, "$")?;
    Ok(serde_yaml_ng::from_str(yaml)?)
}

fn validate_yaml_structure(value: &serde_yaml_ng::Value, path: &str) -> anyhow::Result<()> {
    use serde_yaml_ng::Value;

    match value {
        Value::Sequence(sequence) => {
            for (index, item) in sequence.iter().enumerate() {
                validate_yaml_structure(item, &format!("{path}[{index}]"))?;
            }
        }
        Value::Mapping(mapping) => {
            for (index, (key, item)) in mapping.iter().enumerate() {
                validate_yaml_structure(key, &format!("{path}.<key:{index}>"))?;
                if matches!(key, Value::String(key) if key == "<<") {
                    anyhow::bail!("YAML merge key `<<` is not supported at {path}");
                }
                let item_path = match key {
                    Value::String(key) => format!("{path}.{key}"),
                    _ => format!("{path}[value:{index}]"),
                };
                validate_yaml_structure(item, &item_path)?;
            }
        }
        Value::Tagged(_) => anyhow::bail!("YAML tags are not supported at {path}"),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
    Ok(())
}

pub(crate) fn build_bench_config(bench: &BenchArgs) -> anyhow::Result<OfflineConfig> {
    let mut draft = FlatConfigDto::default();
    draft.family.clone_from(&bench.algorithm);
    draft.auto_mode.clone_from(&bench.auto_mode);
    draft.time_budget_ms = bench.time_budget;
    draft
        .to_offline_config()
        .with_context(|| "invalid bench packer configuration")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DraftFamily {
    Skyline,
    MaxRects,
    Guillotine,
    Auto,
}

impl FromStr for DraftFamily {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "skyline" => Ok(Self::Skyline),
            "maxrects" => Ok(Self::MaxRects),
            "guillotine" => Ok(Self::Guillotine),
            "auto" => Ok(Self::Auto),
            _ => Err(()),
        }
    }
}

fn parse_field<T>(field: &'static str, raw: &str) -> anyhow::Result<T>
where
    T: FromStr,
{
    raw.parse()
        .map_err(|_| anyhow::anyhow!("unknown {field}: {raw}"))
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct FlatConfigDto {
    family: String,
    skyline: String,
    heuristic: String,
    g_choice: String,
    g_split: String,
    auto_mode: String,
    max_width: u32,
    max_height: u32,
    allow_rotation: bool,
    force_max_dimensions: bool,
    border_padding: u32,
    texture_padding: u32,
    texture_extrusion: u32,
    trim: bool,
    trim_threshold: u8,
    texture_outlines: bool,
    power_of_two: bool,
    square: bool,
    use_waste_map: bool,
    sort_order: String,
    time_budget_ms: Option<u64>,
    parallel: bool,
    mr_reference: bool,
    auto_mr_ref_time_ms_threshold: Option<u64>,
    auto_mr_ref_input_threshold: Option<usize>,
    transparent_policy: String,
}

impl Default for FlatConfigDto {
    fn default() -> Self {
        Self {
            family: "skyline".into(),
            skyline: "bl".into(),
            heuristic: "baf".into(),
            g_choice: "baf".into(),
            g_split: "slas".into(),
            auto_mode: "quality".into(),
            max_width: 1024,
            max_height: 1024,
            allow_rotation: true,
            force_max_dimensions: false,
            border_padding: 0,
            texture_padding: 2,
            texture_extrusion: 0,
            trim: true,
            trim_threshold: 0,
            texture_outlines: false,
            power_of_two: false,
            square: false,
            use_waste_map: false,
            sort_order: "area_desc".into(),
            time_budget_ms: None,
            parallel: false,
            mr_reference: false,
            auto_mr_ref_time_ms_threshold: None,
            auto_mr_ref_input_threshold: None,
            transparent_policy: "keep".into(),
        }
    }
}

impl FlatConfigDto {
    fn from_cli(cli: &PackArgs) -> anyhow::Result<Self> {
        let draft = Self {
            family: cli.algorithm.clone(),
            skyline: cli.skyline.clone(),
            heuristic: cli.heuristic.clone(),
            g_choice: cli.g_choice.clone(),
            g_split: cli.g_split.clone(),
            auto_mode: cli.auto_mode.clone(),
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
            sort_order: cli.sort_order.clone(),
            time_budget_ms: cli.time_budget,
            parallel: cli.parallel,
            mr_reference: cli.mr_reference,
            auto_mr_ref_time_ms_threshold: cli.auto_mr_ref_time_threshold,
            auto_mr_ref_input_threshold: cli.auto_mr_ref_input_threshold,
            transparent_policy: cli.transparent_policy.clone(),
        };
        draft.validate_enum_fields()?;
        Ok(draft)
    }

    fn apply_patch(&mut self, patch: FlatConfigPatch) -> anyhow::Result<()> {
        macro_rules! overlay {
            ($($field:ident),+ $(,)?) => {
                $(if let Some(value) = patch.$field { self.$field = value; })+
            };
        }

        overlay!(
            family,
            skyline,
            heuristic,
            g_choice,
            g_split,
            auto_mode,
            max_width,
            max_height,
            allow_rotation,
            force_max_dimensions,
            border_padding,
            texture_padding,
            texture_extrusion,
            trim,
            trim_threshold,
            texture_outlines,
            power_of_two,
            square,
            use_waste_map,
            sort_order,
            parallel,
            mr_reference,
            transparent_policy,
        );
        if let Some(value) = patch.time_budget_ms {
            self.time_budget_ms = Some(value);
        }
        if let Some(value) = patch.auto_mr_ref_time_ms_threshold {
            self.auto_mr_ref_time_ms_threshold = Some(value);
        }
        if let Some(value) = patch.auto_mr_ref_input_threshold {
            self.auto_mr_ref_input_threshold = Some(value);
        }
        self.validate_enum_fields()
    }

    fn validate_enum_fields(&self) -> anyhow::Result<()> {
        let _: DraftFamily = parse_field("algorithm family", &self.family)?;
        let _: SkylineHeuristic = parse_field("Skyline heuristic", &self.skyline)?;
        let _: MaxRectsHeuristic = parse_field("MaxRects heuristic", &self.heuristic)?;
        let _: GuillotineChoice = parse_field("Guillotine choice", &self.g_choice)?;
        let _: GuillotineSplit = parse_field("Guillotine split", &self.g_split)?;
        let _: AutoMode = parse_field("auto mode", &self.auto_mode)?;
        let _: SortOrder = parse_field("sort order", &self.sort_order)?;
        let _: TransparentPolicy = parse_field("transparent policy", &self.transparent_policy)?;
        Ok(())
    }

    fn to_offline_config(&self) -> anyhow::Result<OfflineConfig> {
        self.validate_enum_fields()?;
        let page = PageConfig::builder()
            .max_dimensions(self.max_width, self.max_height)
            .allow_rotation(self.allow_rotation)
            .border_padding(self.border_padding)
            .texture_padding(self.texture_padding)
            .texture_extrusion(self.texture_extrusion)
            .build()
            .with_context(|| "invalid page configuration")?;
        let strategy = self.packing_strategy()?;

        OfflineConfig::builder()
            .page_config(page)
            .force_max_dimensions(self.force_max_dimensions)
            .power_of_two(self.power_of_two)
            .square(self.square)
            .trim(self.trim)
            .trim_threshold(self.trim_threshold)
            .transparent_policy(parse_field("transparent policy", &self.transparent_policy)?)
            .outlines(self.texture_outlines)
            .sort_order(parse_field("sort order", &self.sort_order)?)
            .strategy(strategy)
            .build()
            .with_context(|| "invalid offline packer configuration")
    }

    fn packing_strategy(&self) -> anyhow::Result<PackingStrategy> {
        match parse_field("algorithm family", &self.family)? {
            DraftFamily::Skyline => Ok(PackingStrategy::Skyline {
                heuristic: parse_field("Skyline heuristic", &self.skyline)?,
                use_waste_map: self.use_waste_map,
            }),
            DraftFamily::MaxRects => Ok(PackingStrategy::MaxRects {
                heuristic: parse_field("MaxRects heuristic", &self.heuristic)?,
                reference: self.mr_reference,
            }),
            DraftFamily::Guillotine => Ok(PackingStrategy::Guillotine {
                choice: parse_field("Guillotine choice", &self.g_choice)?,
                split: parse_field("Guillotine split", &self.g_split)?,
            }),
            DraftFamily::Auto => Ok(PackingStrategy::Auto {
                mode: parse_field("auto mode", &self.auto_mode)?,
                time_budget: self.time_budget_ms.map(Duration::from_millis),
                parallel: self.parallel,
                reference_time_threshold: self
                    .auto_mr_ref_time_ms_threshold
                    .map(Duration::from_millis),
                reference_input_threshold: self.auto_mr_ref_input_threshold,
            }),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct FlatConfigPatch {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn apply_yaml(yaml: &str, mut base: FlatConfigDto) -> anyhow::Result<FlatConfigDto> {
        let patch = parse_yaml_patch(yaml)?;
        base.apply_patch(patch)?;
        Ok(base)
    }

    macro_rules! assert_enum_cases {
        ($key:literal, $field:ident, $field_type:ty, [$(($raw:literal, $expected:expr)),+ $(,)?]) => {
            $(
                let yaml = format!("{}: {}", $key, $raw);
                let cfg = apply_yaml(&yaml, FlatConfigDto::default()).unwrap();
                assert_eq!(
                    parse_field::<$field_type>(stringify!($field), &cfg.$field).unwrap(),
                    $expected,
                    "{}: {}",
                    $key,
                    $raw,
                );
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

        let cfg = apply_yaml(yaml, FlatConfigDto::default()).unwrap();
        let offline = cfg.to_offline_config().unwrap();

        assert_eq!(cfg.family, "guillotine");
        assert_eq!(cfg.skyline, "minwaste");
        assert_eq!(cfg.heuristic, "contactpoint");
        assert_eq!(cfg.g_choice, "worstlongsidefit");
        assert_eq!(cfg.g_split, "splitlongeraxis");
        assert_eq!(cfg.auto_mode, "fast");
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
        assert_eq!(cfg.sort_order, "name_asc");
        assert_eq!(cfg.time_budget_ms, Some(850));
        assert!(cfg.parallel);
        assert!(cfg.mr_reference);
        assert_eq!(cfg.auto_mr_ref_time_ms_threshold, Some(600));
        assert_eq!(cfg.auto_mr_ref_input_threshold, Some(900));
        assert_eq!(cfg.transparent_policy, "one_by_one");
        assert_eq!(
            *offline.strategy(),
            PackingStrategy::Guillotine {
                choice: GuillotineChoice::WorstLongSideFit,
                split: GuillotineSplit::SplitLongerAxis,
            }
        );
        assert_eq!(offline.page_config().max_dimensions(), (2048, 1536));
        assert!(!offline.page_config().allow_rotation());
        assert!(offline.force_max_dimensions());
        assert_eq!(offline.page_config().border_padding(), 7);
        assert_eq!(offline.page_config().texture_padding(), 11);
        assert_eq!(offline.page_config().texture_extrusion(), 3);
        assert!(!offline.trim_enabled());
        assert!(offline.outlines());
        assert!(offline.power_of_two());
        assert!(offline.square());
        assert_eq!(offline.sort_order(), SortOrder::NameAsc);
    }

    #[test]
    fn each_family_builds_only_its_selected_strategy() {
        let cases = [
            (
                "family: skyline\nskyline: mw\nuse_waste_map: true",
                PackingStrategy::Skyline {
                    heuristic: SkylineHeuristic::MinWaste,
                    use_waste_map: true,
                },
            ),
            (
                "family: maxrects\nheuristic: cp\nmr_reference: true",
                PackingStrategy::MaxRects {
                    heuristic: MaxRectsHeuristic::ContactPoint,
                    reference: true,
                },
            ),
            (
                "family: guillotine\ng_choice: wlsf\ng_split: las",
                PackingStrategy::Guillotine {
                    choice: GuillotineChoice::WorstLongSideFit,
                    split: GuillotineSplit::SplitLongerAxis,
                },
            ),
            (
                "family: auto\nauto_mode: fast\ntime_budget_ms: 250\nparallel: true\nauto_mr_ref_time_ms_threshold: 500\nauto_mr_ref_input_threshold: 900",
                PackingStrategy::Auto {
                    mode: AutoMode::Fast,
                    time_budget: Some(Duration::from_millis(250)),
                    parallel: true,
                    reference_time_threshold: Some(Duration::from_millis(500)),
                    reference_input_threshold: Some(900),
                },
            ),
        ];

        for (yaml, expected) in cases {
            let draft = apply_yaml(yaml, FlatConfigDto::default()).unwrap();
            let config = draft.to_offline_config().unwrap();
            assert_eq!(*config.strategy(), expected, "{yaml}");
        }
    }

    #[test]
    fn yaml_overrides_cli_draft_except_for_forced_reference() {
        let mut draft = FlatConfigDto {
            max_width: 2048,
            allow_rotation: true,
            mr_reference: true,
            ..Default::default()
        };
        let patch =
            parse_yaml_patch("max_width: 640\nallow_rotation: false\nmr_reference: false").unwrap();

        apply_yaml_patch(&mut draft, patch, true).unwrap();

        assert_eq!(draft.max_width, 640);
        assert!(!draft.allow_rotation);
        assert!(draft.mr_reference);
    }

    #[test]
    fn yaml_accepts_every_supported_enum_spelling() {
        assert_enum_cases!(
            "family",
            family,
            DraftFamily,
            [
                ("skyline", DraftFamily::Skyline),
                ("maxrects", DraftFamily::MaxRects),
                ("guillotine", DraftFamily::Guillotine),
                ("auto", DraftFamily::Auto),
            ]
        );
        assert_enum_cases!(
            "skyline",
            skyline,
            SkylineHeuristic,
            [
                ("bl", SkylineHeuristic::BottomLeft),
                ("bottomleft", SkylineHeuristic::BottomLeft),
                ("minwaste", SkylineHeuristic::MinWaste),
                ("mw", SkylineHeuristic::MinWaste),
            ]
        );
        assert_enum_cases!(
            "heuristic",
            heuristic,
            MaxRectsHeuristic,
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
            GuillotineChoice,
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
            GuillotineSplit,
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
            AutoMode,
            [("fast", AutoMode::Fast), ("quality", AutoMode::Quality),]
        );
        assert_enum_cases!(
            "sort_order",
            sort_order,
            SortOrder,
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
            TransparentPolicy,
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
            FlatConfigDto::default(),
        )
        .unwrap();

        assert_eq!(
            parse_field::<DraftFamily>("family", &cfg.family).unwrap(),
            DraftFamily::MaxRects
        );
        assert_eq!(
            parse_field::<SkylineHeuristic>("skyline", &cfg.skyline).unwrap(),
            SkylineHeuristic::MinWaste
        );
        assert_eq!(
            parse_field::<TransparentPolicy>("transparent policy", &cfg.transparent_policy)
                .unwrap(),
            TransparentPolicy::OneByOne
        );
    }

    #[test]
    fn yaml_boolean_scalars_use_yaml_1_2_resolution() {
        for raw in ["true", "True", "TRUE"] {
            let cfg = apply_yaml(&format!("trim: {raw}"), FlatConfigDto::default()).unwrap();
            assert!(cfg.trim, "trim: {raw}");
        }
        for raw in ["false", "False", "FALSE"] {
            let cfg = apply_yaml(&format!("trim: {raw}"), FlatConfigDto::default()).unwrap();
            assert!(!cfg.trim, "trim: {raw}");
        }
    }

    #[test]
    fn yaml_1_1_only_boolean_words_are_not_coerced() {
        for raw in [
            "y", "Y", "yes", "Yes", "YES", "n", "N", "no", "No", "NO", "on", "On", "ON", "off",
            "Off", "OFF",
        ] {
            let error = parse_yaml_patch(&format!("trim: {raw}"))
                .expect_err("YAML 1.1-only boolean words are strings in the current parser");
            let message = error.to_string();
            assert!(message.contains("invalid type: string"), "{raw}: {message}");
            assert!(message.contains("expected a boolean"), "{raw}: {message}");
        }
    }

    #[test]
    fn yaml_1_1_boolean_words_in_enum_fields_remain_strings() {
        for raw in ["yes", "no", "on", "off"] {
            let error = apply_yaml(&format!("family: {raw}"), FlatConfigDto::default())
                .expect_err("the token should reach enum parsing as a string");
            assert_eq!(
                error.to_string(),
                format!("unknown algorithm family: {raw}")
            );
        }
    }

    #[test]
    fn yaml_null_optional_values_leave_the_overlay_base_unchanged() {
        let base = FlatConfigDto {
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
    fn yaml_duplicate_keys_are_rejected() {
        let error = parse_yaml_patch("max_width: 640\nmax_width: 720")
            .expect_err("duplicate YAML keys must be rejected");
        let message = error.to_string();

        assert!(message.contains("duplicate"), "{message}");
        assert!(message.contains("max_width"), "{message}");
    }

    #[test]
    fn printed_yaml_round_trips_the_shared_field_names() {
        let source = FlatConfigDto {
            family: "guillotine".into(),
            skyline: "minwaste".into(),
            heuristic: "contactpoint".into(),
            g_choice: "worstlongsidefit".into(),
            g_split: "splitlongeraxis".into(),
            auto_mode: "fast".into(),
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
            sort_order: "name_asc".into(),
            time_budget_ms: Some(850),
            parallel: true,
            mr_reference: true,
            auto_mr_ref_time_ms_threshold: Some(600),
            auto_mr_ref_input_threshold: Some(900),
            transparent_policy: "one_by_one".into(),
        };
        let printed = serde_yaml_ng::to_string(&source).unwrap();
        let reparsed = apply_yaml(&printed, FlatConfigDto::default()).unwrap();

        assert_eq!(reparsed, source);
        assert!(printed.contains("heuristic:"));
        assert!(printed.contains("skyline:"));
        assert!(!printed.contains("mr_heuristic:"));
        assert!(!printed.contains("skyline_heuristic:"));
    }

    #[test]
    fn yaml_unknown_fields_must_be_rejected() {
        let error = apply_yaml(
            "max_width: 640\nfuture_option: enabled",
            FlatConfigDto::default(),
        )
        .expect_err("U7 must deny unknown fields");
        let message = error.to_string();

        assert!(
            message.contains("unknown field `future_option`"),
            "{message}"
        );
        assert!(message.contains("line 2"), "{message}");
    }

    #[test]
    fn yaml_tags_must_be_rejected() {
        let error = apply_yaml("max_width: !pixels 640", FlatConfigDto::default())
            .expect_err("U7 must reject YAML tags");
        let message = error.to_string();

        assert!(message.contains("YAML tags are not supported"), "{message}");
        assert!(message.contains("$.max_width"), "{message}");
    }

    #[test]
    fn yaml_merge_keys_must_be_rejected() {
        let error = apply_yaml(
            "defaults: &defaults { max_width: 640 }\n<<: *defaults",
            FlatConfigDto::default(),
        )
        .expect_err("U7 must reject YAML merge keys");
        let message = error.to_string();

        assert!(
            message.contains("YAML merge key `<<` is not supported"),
            "{message}"
        );
        assert!(message.contains('$'), "{message}");
    }

    #[test]
    fn yaml_nested_tags_are_rejected_before_typed_deserialization() {
        let error = parse_yaml_patch("future:\n  nested: !custom value")
            .expect_err("nested YAML tags must be rejected structurally");
        let message = error.to_string();

        assert!(message.contains("YAML tags are not supported"), "{message}");
        assert!(message.contains("$.future.nested"), "{message}");
    }

    #[test]
    fn yaml_nested_merge_keys_are_rejected_before_typed_deserialization() {
        let error = parse_yaml_patch("future:\n  <<: { max_width: 1 }")
            .expect_err("nested YAML merge keys must be rejected structurally");
        let message = error.to_string();

        assert!(
            message.contains("YAML merge key `<<` is not supported"),
            "{message}"
        );
        assert!(message.contains("$.future"), "{message}");
    }

    #[test]
    fn yaml_type_errors_retain_field_and_source_location() {
        let error = parse_yaml_patch("max_width: 640\ntrim: not-a-bool")
            .expect_err("invalid field types must retain YAML source context");
        let message = error.to_string();

        assert!(message.contains("trim"), "{message}");
        assert!(message.contains("expected a boolean"), "{message}");
        assert!(message.contains("line 2"), "{message}");
        assert!(message.contains("column"), "{message}");
    }

    #[test]
    fn printed_yaml_must_round_trip_heuristic_fields() {
        let source = FlatConfigDto {
            heuristic: "contactpoint".into(),
            skyline: "minwaste".into(),
            ..Default::default()
        };
        let printed = serde_yaml_ng::to_string(&source).unwrap();
        let reparsed = apply_yaml(&printed, FlatConfigDto::default()).unwrap();

        assert_eq!(reparsed.heuristic, source.heuristic);
        assert_eq!(reparsed.skyline, source.skyline);
    }

    #[test]
    fn printed_json_uses_the_same_flat_round_trip_contract() {
        let source = FlatConfigDto {
            family: "auto".into(),
            heuristic: "contactpoint".into(),
            skyline: "minwaste".into(),
            time_budget_ms: Some(350),
            ..Default::default()
        };
        let printed = serde_json::to_string_pretty(&source).unwrap();
        let patch = serde_json::from_str::<FlatConfigPatch>(&printed).unwrap();
        let mut reparsed = FlatConfigDto::default();
        reparsed.apply_patch(patch).unwrap();

        assert_eq!(reparsed, source);
        assert!(printed.contains("\"heuristic\""));
        assert!(printed.contains("\"skyline\""));
        assert!(!printed.contains("mr_heuristic"));
        assert!(!printed.contains("skyline_heuristic"));
    }
}
