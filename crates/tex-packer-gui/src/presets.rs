//! Packer presets for common use cases

use crate::config_draft::{GuiConfigDraft, StrategyKind};
use tex_packer_core::config::{AutoMode, MaxRectsHeuristic, SkylineHeuristic};

/// A packer preset with configuration and description
#[derive(Clone)]
pub struct PackerPreset {
    pub name: &'static str,
    pub description: &'static str,
    pub details: Vec<&'static str>,
    pub icon: &'static str,
    pub draft: GuiConfigDraft,
    pub recommended_sizes: Vec<(u32, u32)>,
}

impl PackerPreset {
    /// Quality preset - best packing quality (default)
    pub fn quality() -> Self {
        Self {
            name: "Quality",
            description: "Best packing quality for production builds",
            details: vec![
                "• Algorithm: Auto (Quality mode)",
                "• Rotation: Enabled for better packing",
                "• Trim: Removes transparent borders",
                "• Padding: 2px between sprites",
                "• Extrusion: 2px to prevent bleeding",
                "• Time budget: 500ms for optimization",
                "",
                "Recommended for: Final game builds, asset publishing",
            ],
            icon: "💎",
            draft: GuiConfigDraft {
                max_width: 2048,
                max_height: 2048,
                texture_extrusion: 2,
                strategy_kind: StrategyKind::Auto,
                auto_mode: AutoMode::Quality,
                time_budget_ms: Some(500),
                ..Default::default()
            },
            recommended_sizes: vec![(1024, 1024), (2048, 2048), (4096, 4096)],
        }
    }

    /// Fast preset - quick iteration
    pub fn fast() -> Self {
        Self {
            name: "Fast",
            description: "Fast packing for rapid iteration and prototyping",
            details: vec![
                "• Algorithm: Skyline MinWaste",
                "• Rotation: Enabled",
                "• Trim: Enabled",
                "• Padding: 2px between sprites",
                "• Extrusion: 2px to prevent bleeding",
                "• Predictable performance",
                "",
                "Recommended for: Development, quick previews, iteration",
            ],
            icon: "⚡",
            draft: GuiConfigDraft {
                max_width: 2048,
                max_height: 2048,
                texture_extrusion: 2,
                strategy_kind: StrategyKind::Skyline,
                skyline_heuristic: SkylineHeuristic::MinWaste,
                ..Default::default()
            },
            recommended_sizes: vec![(1024, 1024), (2048, 2048)],
        }
    }

    /// Web Assets preset
    pub fn web_assets() -> Self {
        Self {
            name: "Web Assets",
            description: "Optimized for web: no rotation, minimal padding",
            details: vec![
                "• Algorithm: MaxRects BestAreaFit",
                "• Rotation: Disabled (web typically doesn't need it)",
                "• Trim: Enabled",
                "• Padding: 1px (minimal)",
                "• Extrusion: 0px (not needed for web)",
                "• Large atlas support (4096x4096)",
                "",
                "Recommended for: Web games, HTML5, icon sheets",
            ],
            icon: "🌐",
            draft: GuiConfigDraft {
                max_width: 4096,
                max_height: 4096,
                allow_rotation: false,
                texture_padding: 1,
                strategy_kind: StrategyKind::MaxRects,
                max_rects_heuristic: MaxRectsHeuristic::BestAreaFit,
                ..Default::default()
            },
            recommended_sizes: vec![(2048, 2048), (4096, 4096)],
        }
    }

    /// Unity Mobile preset
    pub fn unity_mobile() -> Self {
        Self {
            name: "Unity Mobile",
            description: "Power-of-2 square atlases for Unity mobile",
            details: vec![
                "• Algorithm: Auto (Quality mode)",
                "• Rotation: Enabled",
                "• Trim: Enabled",
                "• Padding: 2px between sprites",
                "• Extrusion: 2px to prevent bleeding",
                "• Power-of-2: Required for mobile GPU compression",
                "• Square: Unity prefers square textures",
                "",
                "Recommended for: Unity mobile games (iOS/Android)",
            ],
            icon: "📱",
            draft: GuiConfigDraft {
                max_width: 2048,
                max_height: 2048,
                texture_extrusion: 2,
                power_of_two: true,
                square: true,
                strategy_kind: StrategyKind::Auto,
                auto_mode: AutoMode::Quality,
                ..Default::default()
            },
            recommended_sizes: vec![(512, 512), (1024, 1024), (2048, 2048)],
        }
    }

    /// Godot preset
    pub fn godot() -> Self {
        Self {
            name: "Godot",
            description: "Optimized for Godot Engine (4.x)",
            details: vec![
                "• Algorithm: Auto (Quality mode)",
                "• Rotation: Enabled",
                "• Trim: Enabled",
                "• Padding: 2px between sprites",
                "• Extrusion: 2px to prevent bleeding",
                "• Power-of-2: Not required (Godot 4 supports any size)",
                "• Export: JSON Hash format",
                "",
                "Recommended for: Godot 4.x projects",
            ],
            icon: "🎮",
            draft: GuiConfigDraft {
                max_width: 4096,
                max_height: 4096,
                texture_extrusion: 2,
                strategy_kind: StrategyKind::Auto,
                auto_mode: AutoMode::Quality,
                ..Default::default()
            },
            recommended_sizes: vec![(2048, 2048), (4096, 4096)],
        }
    }

    /// Unreal Engine preset
    pub fn unreal() -> Self {
        Self {
            name: "Unreal Engine",
            description: "Optimized for Unreal Engine",
            details: vec![
                "• Algorithm: Auto (Quality mode)",
                "• Rotation: Enabled",
                "• Trim: Enabled",
                "• Padding: 2px between sprites",
                "• Extrusion: 2px to prevent bleeding",
                "• Border: 2px to avoid mipmap bleeding",
                "• Power-of-2: Recommended for Unreal",
                "",
                "Recommended for: Unreal Engine 4/5 projects",
            ],
            icon: "🎯",
            draft: GuiConfigDraft {
                max_width: 4096,
                max_height: 4096,
                border_padding: 2,
                texture_extrusion: 2,
                power_of_two: true,
                strategy_kind: StrategyKind::Auto,
                auto_mode: AutoMode::Quality,
                ..Default::default()
            },
            recommended_sizes: vec![(2048, 2048), (4096, 4096)],
        }
    }

    /// Runtime packing preset
    pub fn runtime() -> Self {
        Self {
            name: "Runtime",
            description: "Fast and predictable for runtime packing",
            details: vec![
                "• Algorithm: Skyline BottomLeft",
                "• Rotation: Enabled",
                "• Trim: Disabled (assumes pre-trimmed assets)",
                "• Padding: 2px between sprites",
                "• Extrusion: 2px to prevent bleeding",
                "• Waste Map: Disabled for consistent performance",
                "• Predictable timing",
                "",
                "Recommended for: Runtime dynamic atlas generation",
            ],
            icon: "🚀",
            draft: GuiConfigDraft {
                max_width: 2048,
                max_height: 2048,
                texture_extrusion: 2,
                trim: false,
                strategy_kind: StrategyKind::Skyline,
                skyline_heuristic: SkylineHeuristic::BottomLeft,
                ..Default::default()
            },
            recommended_sizes: vec![(2048, 2048), (4096, 4096)],
        }
    }

    /// Maximum quality preset (slow)
    pub fn maximum() -> Self {
        Self {
            name: "Maximum",
            description: "Best possible packing (slow, for offline builds)",
            details: vec![
                "• Algorithm: Auto (Quality mode)",
                "• Rotation: Enabled",
                "• Trim: Enabled",
                "• Padding: 2px between sprites",
                "• Extrusion: 2px to prevent bleeding",
                "• Time budget: 5000ms (5 seconds)",
                "• MaxRects Reference: Enabled for best quality",
                "• Parallel: Enabled (if compiled with feature)",
                "",
                "Recommended for: Final production builds, maximum efficiency",
            ],
            icon: "🏆",
            draft: GuiConfigDraft {
                max_width: 2048,
                max_height: 2048,
                texture_extrusion: 2,
                strategy_kind: StrategyKind::Auto,
                auto_mode: AutoMode::Quality,
                time_budget_ms: Some(5000),
                max_rects_reference: true,
                parallel: true,
                ..Default::default()
            },
            recommended_sizes: vec![(2048, 2048), (4096, 4096)],
        }
    }

    /// Get all available presets
    pub fn all() -> Vec<Self> {
        vec![
            Self::quality(), // Default
            Self::fast(),
            Self::web_assets(),
            Self::unity_mobile(),
            Self::godot(),
            Self::unreal(),
            Self::runtime(),
            Self::maximum(),
        ]
    }

    /// Get default preset (Quality)
    pub fn default() -> Self {
        Self::quality()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tex_packer_core::config::PackingStrategy;

    #[test]
    fn every_preset_builds_a_valid_offline_config() {
        let presets = PackerPreset::all();

        assert_eq!(presets.len(), 8);
        for preset in presets {
            preset
                .draft
                .try_build_offline_config()
                .unwrap_or_else(|error| panic!("{} preset is invalid: {error}", preset.name));
        }
    }

    #[test]
    fn presets_keep_their_selected_strategy() {
        let presets = PackerPreset::all();
        let expected = [
            StrategyKind::Auto,
            StrategyKind::Skyline,
            StrategyKind::MaxRects,
            StrategyKind::Auto,
            StrategyKind::Auto,
            StrategyKind::Auto,
            StrategyKind::Skyline,
            StrategyKind::Auto,
        ];

        for (preset, expected_kind) in presets.iter().zip(expected) {
            assert_eq!(preset.draft.strategy_kind, expected_kind, "{}", preset.name);
        }

        let maximum = presets
            .last()
            .expect("the maximum-quality preset should exist")
            .draft
            .try_build_offline_config()
            .expect("the maximum-quality preset should be valid");
        assert!(matches!(maximum.strategy(), PackingStrategy::Auto { .. }));
    }
}
