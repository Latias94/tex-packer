//! Editable GUI-owned configuration projected into validated core configuration.

use std::time::Duration;

use tex_packer_core::config::{
    AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic, OfflineConfig, PackingStrategy,
    PageConfig, SkylineHeuristic, SortOrder, TransparentPolicy,
};
use tex_packer_core::error::Result;

/// Strategy selected in the GUI while preserving each strategy's edit state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StrategyKind {
    Skyline,
    MaxRects,
    Guillotine,
    Auto,
}

/// Mutable form state owned by the GUI adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuiConfigDraft {
    pub(crate) max_width: u32,
    pub(crate) max_height: u32,
    pub(crate) allow_rotation: bool,
    pub(crate) force_max_dimensions: bool,
    pub(crate) border_padding: u32,
    pub(crate) texture_padding: u32,
    pub(crate) texture_extrusion: u32,
    pub(crate) trim: bool,
    pub(crate) trim_threshold: u8,
    pub(crate) transparent_policy: TransparentPolicy,
    pub(crate) outlines: bool,
    pub(crate) power_of_two: bool,
    pub(crate) square: bool,
    pub(crate) sort_order: SortOrder,
    pub(crate) strategy_kind: StrategyKind,
    pub(crate) skyline_heuristic: SkylineHeuristic,
    pub(crate) use_waste_map: bool,
    pub(crate) max_rects_heuristic: MaxRectsHeuristic,
    pub(crate) max_rects_reference: bool,
    pub(crate) guillotine_choice: GuillotineChoice,
    pub(crate) guillotine_split: GuillotineSplit,
    pub(crate) auto_mode: AutoMode,
    pub(crate) time_budget_ms: Option<u64>,
    pub(crate) parallel: bool,
    pub(crate) auto_reference_time_threshold_ms: Option<u64>,
    pub(crate) auto_reference_input_threshold: Option<usize>,
}

impl GuiConfigDraft {
    /// Validate the editable form and project only the selected workflow strategy.
    pub(crate) fn try_build_offline_config(&self) -> Result<OfflineConfig> {
        let page = PageConfig::builder()
            .max_dimensions(self.max_width, self.max_height)
            .allow_rotation(self.allow_rotation)
            .border_padding(self.border_padding)
            .texture_padding(self.texture_padding)
            .texture_extrusion(self.texture_extrusion)
            .build()?;

        let strategy = match self.strategy_kind {
            StrategyKind::Skyline => PackingStrategy::Skyline {
                heuristic: self.skyline_heuristic,
                use_waste_map: self.use_waste_map,
            },
            StrategyKind::MaxRects => PackingStrategy::MaxRects {
                heuristic: self.max_rects_heuristic,
                reference: self.max_rects_reference,
            },
            StrategyKind::Guillotine => PackingStrategy::Guillotine {
                choice: self.guillotine_choice,
                split: self.guillotine_split,
            },
            StrategyKind::Auto => PackingStrategy::Auto {
                mode: self.auto_mode,
                time_budget: self.time_budget_ms.map(Duration::from_millis),
                parallel: self.parallel,
                reference_time_threshold: self
                    .auto_reference_time_threshold_ms
                    .map(Duration::from_millis),
                reference_input_threshold: self.auto_reference_input_threshold,
            },
        };

        OfflineConfig::builder()
            .page_config(page)
            .force_max_dimensions(self.force_max_dimensions)
            .power_of_two(self.power_of_two)
            .square(self.square)
            .trim(self.trim)
            .trim_threshold(self.trim_threshold)
            .transparent_policy(self.transparent_policy)
            .outlines(self.outlines)
            .sort_order(self.sort_order)
            .strategy(strategy)
            .build()
    }
}

impl Default for GuiConfigDraft {
    fn default() -> Self {
        Self {
            max_width: 1024,
            max_height: 1024,
            allow_rotation: true,
            force_max_dimensions: false,
            border_padding: 0,
            texture_padding: 2,
            texture_extrusion: 0,
            trim: true,
            trim_threshold: 0,
            transparent_policy: TransparentPolicy::Keep,
            outlines: false,
            power_of_two: false,
            square: false,
            sort_order: SortOrder::AreaDesc,
            strategy_kind: StrategyKind::Skyline,
            skyline_heuristic: SkylineHeuristic::BottomLeft,
            use_waste_map: false,
            max_rects_heuristic: MaxRectsHeuristic::BestAreaFit,
            max_rects_reference: false,
            guillotine_choice: GuillotineChoice::BestAreaFit,
            guillotine_split: GuillotineSplit::SplitShorterLeftoverAxis,
            auto_mode: AutoMode::Quality,
            time_budget_ms: None,
            parallel: false,
            auto_reference_time_threshold_ms: None,
            auto_reference_input_threshold: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_draft_matches_validated_offline_defaults() {
        let config = GuiConfigDraft::default()
            .try_build_offline_config()
            .expect("default GUI configuration should be valid");

        assert_eq!(config.page_config().max_dimensions(), (1024, 1024));
        assert!(config.page_config().allow_rotation());
        assert!(config.trim_enabled());
        assert_eq!(config.trim_threshold(), Some(0));
        assert_eq!(config.transparent_policy(), Some(TransparentPolicy::Keep));
        assert_eq!(
            config.strategy(),
            &PackingStrategy::Skyline {
                heuristic: SkylineHeuristic::BottomLeft,
                use_waste_map: false,
            }
        );
    }

    #[test]
    fn selected_strategy_is_the_only_strategy_projected() {
        let mut draft = GuiConfigDraft {
            skyline_heuristic: SkylineHeuristic::MinWaste,
            use_waste_map: true,
            max_rects_heuristic: MaxRectsHeuristic::ContactPoint,
            max_rects_reference: true,
            guillotine_choice: GuillotineChoice::WorstLongSideFit,
            guillotine_split: GuillotineSplit::SplitLongerAxis,
            auto_mode: AutoMode::Fast,
            time_budget_ms: Some(750),
            parallel: true,
            auto_reference_time_threshold_ms: Some(300),
            auto_reference_input_threshold: Some(400),
            ..Default::default()
        };

        let cases = [
            (
                StrategyKind::Skyline,
                PackingStrategy::Skyline {
                    heuristic: SkylineHeuristic::MinWaste,
                    use_waste_map: true,
                },
            ),
            (
                StrategyKind::MaxRects,
                PackingStrategy::MaxRects {
                    heuristic: MaxRectsHeuristic::ContactPoint,
                    reference: true,
                },
            ),
            (
                StrategyKind::Guillotine,
                PackingStrategy::Guillotine {
                    choice: GuillotineChoice::WorstLongSideFit,
                    split: GuillotineSplit::SplitLongerAxis,
                },
            ),
            (
                StrategyKind::Auto,
                PackingStrategy::Auto {
                    mode: AutoMode::Fast,
                    time_budget: Some(Duration::from_millis(750)),
                    parallel: true,
                    reference_time_threshold: Some(Duration::from_millis(300)),
                    reference_input_threshold: Some(400),
                },
            ),
        ];

        for (kind, expected) in cases {
            draft.strategy_kind = kind;
            let config = draft
                .try_build_offline_config()
                .expect("strategy draft should be valid");
            assert_eq!(config.strategy(), &expected);
        }
    }

    #[test]
    fn auto_ignores_the_legacy_max_rects_reference_toggle() {
        let draft = GuiConfigDraft {
            strategy_kind: StrategyKind::Auto,
            max_rects_reference: true,
            auto_reference_time_threshold_ms: Some(250),
            auto_reference_input_threshold: Some(800),
            ..Default::default()
        };

        let config = draft
            .try_build_offline_config()
            .expect("auto draft should be valid");

        assert_eq!(
            config.strategy(),
            &PackingStrategy::Auto {
                mode: AutoMode::Quality,
                time_budget: None,
                parallel: false,
                reference_time_threshold: Some(Duration::from_millis(250)),
                reference_input_threshold: Some(800),
            }
        );
    }

    #[test]
    fn invalid_page_geometry_is_rejected_before_packing() {
        let draft = GuiConfigDraft {
            max_width: 32,
            max_height: 32,
            border_padding: 16,
            ..Default::default()
        };

        assert!(draft.try_build_offline_config().is_err());
    }
}
