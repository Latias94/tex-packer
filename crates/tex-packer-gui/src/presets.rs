//! Packer presets for common use cases

use tex_packer_core::prelude::*;

/// A packer preset with configuration and description
#[derive(Clone)]
pub struct PackerPreset {
    pub name: &'static str,
    pub description: &'static str,
    pub details: Vec<&'static str>,
    pub icon: &'static str,
    pub config: PackerConfig,
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
            config: PackerConfig::builder()
                .with_max_dimensions(2048, 2048)
                .allow_rotation(true)
                .trim(true)
                .texture_padding(2)
                .texture_extrusion(2)
                .family(AlgorithmFamily::Auto)
                .auto_mode(AutoMode::Quality)
                .time_budget_ms(Some(500))
                .build(),
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
            config: PackerConfig::builder()
                .with_max_dimensions(2048, 2048)
                .allow_rotation(true)
                .trim(true)
                .texture_padding(2)
                .texture_extrusion(2)
                .family(AlgorithmFamily::Skyline)
                .skyline_heuristic(SkylineHeuristic::MinWaste)
                .build(),
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
            config: PackerConfig::builder()
                .with_max_dimensions(4096, 4096)
                .allow_rotation(false)
                .trim(true)
                .texture_padding(1)
                .texture_extrusion(0)
                .family(AlgorithmFamily::MaxRects)
                .mr_heuristic(MaxRectsHeuristic::BestAreaFit)
                .build(),
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
            config: PackerConfig::builder()
                .with_max_dimensions(2048, 2048)
                .allow_rotation(true)
                .trim(true)
                .texture_padding(2)
                .texture_extrusion(2)
                .pow2(true)
                .square(true)
                .family(AlgorithmFamily::Auto)
                .auto_mode(AutoMode::Quality)
                .build(),
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
            config: PackerConfig::builder()
                .with_max_dimensions(4096, 4096)
                .allow_rotation(true)
                .trim(true)
                .texture_padding(2)
                .texture_extrusion(2)
                .pow2(false)
                .square(false)
                .family(AlgorithmFamily::Auto)
                .auto_mode(AutoMode::Quality)
                .build(),
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
            config: PackerConfig::builder()
                .with_max_dimensions(4096, 4096)
                .allow_rotation(true)
                .trim(true)
                .texture_padding(2)
                .texture_extrusion(2)
                .border_padding(2)
                .pow2(true)
                .family(AlgorithmFamily::Auto)
                .auto_mode(AutoMode::Quality)
                .build(),
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
            config: PackerConfig::builder()
                .with_max_dimensions(2048, 2048)
                .allow_rotation(true)
                .trim(false)
                .texture_padding(2)
                .texture_extrusion(2)
                .use_waste_map(false)
                .family(AlgorithmFamily::Skyline)
                .skyline_heuristic(SkylineHeuristic::BottomLeft)
                .build(),
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
            config: PackerConfig::builder()
                .with_max_dimensions(2048, 2048)
                .allow_rotation(true)
                .trim(true)
                .texture_padding(2)
                .texture_extrusion(2)
                .family(AlgorithmFamily::Auto)
                .auto_mode(AutoMode::Quality)
                .time_budget_ms(Some(5000))
                .mr_reference(true)
                .parallel(true)
                .build(),
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
