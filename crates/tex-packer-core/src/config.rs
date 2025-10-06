use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Algorithm families and packing configuration.
/// Key notes:
///   - `family` selects Skyline/MaxRects/Guillotine/Auto
///   - `mr_reference` toggles reference-accurate MaxRects split/prune (SplitFreeNode), improving packing on large sets at higher CPU cost
///   - `time_budget_ms` and `parallel` affect Auto portfolio evaluation
///     Top-level algorithm families.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlgorithmFamily {
    /// Skyline data structure (BL/MW; fast and good baseline). Optional waste-map recovery.
    Skyline,
    /// MaxRects free-list (high quality; many heuristics; best for offline).
    MaxRects,
    /// Guillotine splitting (flexible choice/split; competitive; useful in waste-map too).
    Guillotine,
    /// Try a small portfolio of candidates and pick the best result (pages, then total area).
    Auto,
}

impl FromStr for AlgorithmFamily {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "skyline" => Ok(Self::Skyline),
            "maxrects" => Ok(Self::MaxRects),
            "guillotine" => Ok(Self::Guillotine),
            "auto" => Ok(Self::Auto),
            _ => Err(()),
        }
    }
}

/// MaxRects placement heuristics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MaxRectsHeuristic {
    BestAreaFit,
    BestShortSideFit,
    BestLongSideFit,
    BottomLeft,
    ContactPoint,
}

impl FromStr for MaxRectsHeuristic {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "baf" | "bestareafit" => Ok(Self::BestAreaFit),
            "bssf" | "bestshortsidefit" => Ok(Self::BestShortSideFit),
            "blsf" | "bestlongsidefit" => Ok(Self::BestLongSideFit),
            "bl" | "bottomleft" => Ok(Self::BottomLeft),
            "cp" | "contactpoint" => Ok(Self::ContactPoint),
            _ => Err(()),
        }
    }
}

/// Skyline placement heuristics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SkylineHeuristic {
    BottomLeft,
    MinWaste,
}

impl FromStr for SkylineHeuristic {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "bl" | "bottomleft" => Ok(Self::BottomLeft),
            "minwaste" | "mw" => Ok(Self::MinWaste),
            _ => Err(()),
        }
    }
}

/// Guillotine free-rect choice heuristics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GuillotineChoice {
    BestAreaFit,
    BestShortSideFit,
    BestLongSideFit,
    WorstAreaFit,
    WorstShortSideFit,
    WorstLongSideFit,
}

impl FromStr for GuillotineChoice {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "baf" | "bestareafit" => Ok(Self::BestAreaFit),
            "bssf" | "bestshortsidefit" => Ok(Self::BestShortSideFit),
            "blsf" | "bestlongsidefit" => Ok(Self::BestLongSideFit),
            "waf" | "worstareafit" => Ok(Self::WorstAreaFit),
            "wssf" | "worstshortsidefit" => Ok(Self::WorstShortSideFit),
            "wlsf" | "worstlongsidefit" => Ok(Self::WorstLongSideFit),
            _ => Err(()),
        }
    }
}

/// Guillotine split axis heuristics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GuillotineSplit {
    SplitShorterLeftoverAxis,
    SplitLongerLeftoverAxis,
    SplitMinimizeArea,
    SplitMaximizeArea,
    SplitShorterAxis,
    SplitLongerAxis,
}

impl FromStr for GuillotineSplit {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "slas" | "splitshorterleftoveraxis" => Ok(Self::SplitShorterLeftoverAxis),
            "llas" | "splitlongerleftoveraxis" => Ok(Self::SplitLongerLeftoverAxis),
            "minas" | "splitminimizearea" => Ok(Self::SplitMinimizeArea),
            "maxas" | "splitmaximizearea" => Ok(Self::SplitMaximizeArea),
            "sas" | "splitshorteraxis" => Ok(Self::SplitShorterAxis),
            "las" | "splitlongeraxis" => Ok(Self::SplitLongerAxis),
            _ => Err(()),
        }
    }
}

/// Auto presets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AutoMode {
    Fast,
    Quality,
}

impl FromStr for AutoMode {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "fast" => Ok(Self::Fast),
            "quality" => Ok(Self::Quality),
            _ => Err(()),
        }
    }
}

/// Sorting orders for deterministic packing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    AreaDesc,
    MaxSideDesc,
    HeightDesc,
    WidthDesc,
    NameAsc,
    None,
}

impl FromStr for SortOrder {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "area_desc" => Ok(Self::AreaDesc),
            "max_side_desc" => Ok(Self::MaxSideDesc),
            "height_desc" => Ok(Self::HeightDesc),
            "width_desc" => Ok(Self::WidthDesc),
            "name_asc" => Ok(Self::NameAsc),
            "none" => Ok(Self::None),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackerConfig {
    /// Maximum page width in pixels.
    pub max_width: u32,
    /// Maximum page height in pixels.
    pub max_height: u32,
    /// Allow 90° rotations for placements where beneficial.
    pub allow_rotation: bool,
    /// Force final page dimensions to be exactly max_width/max_height.
    pub force_max_dimensions: bool,

    /// Pixels around entire page border.
    pub border_padding: u32,
    /// Pixels between frames.
    pub texture_padding: u32,
    /// Extrude edge pixels of each frame (for sampling safety).
    pub texture_extrusion: u32,

    /// Trim transparent borders (alpha <= trim_threshold).
    pub trim: bool,
    pub trim_threshold: u8,
    /// Draw red outlines on output pages (debug).
    pub texture_outlines: bool,

    /// Resize output page to power-of-two.
    pub power_of_two: bool,
    /// Force output page to be square (max(width,height)).
    pub square: bool,
    /// Use waste map in Skyline to recover gaps
    pub use_waste_map: bool,

    // algorithm selection
    #[serde(default = "default_family")]
    pub family: AlgorithmFamily,
    #[serde(default = "default_mr_heuristic")]
    pub mr_heuristic: MaxRectsHeuristic,
    #[serde(default = "default_skyline_heuristic")]
    pub skyline_heuristic: SkylineHeuristic,
    #[serde(default = "default_g_choice")]
    pub g_choice: GuillotineChoice,
    #[serde(default = "default_g_split")]
    pub g_split: GuillotineSplit,
    #[serde(default = "default_auto_mode")]
    pub auto_mode: AutoMode,
    #[serde(default = "default_sort_order")]
    pub sort_order: SortOrder,

    // portfolio/parallel controls
    /// Optional time budget for auto portfolio (milliseconds). None or 0 disables.
    #[serde(default)]
    pub time_budget_ms: Option<u64>,
    /// Enable parallel candidate evaluation when feature "parallel" is on.
    #[serde(default = "default_parallel")]
    pub parallel: bool,

    /// Use reference-accurate MaxRects split/prune (SplitFreeNode + staged prune).
    /// When false, uses a simpler but correct split/prune that may create more intermediate free rects.
    #[serde(default)]
    pub mr_reference: bool,

    /// Auto-mode: enable mr_reference when time budget >= this (ms). None => use default heuristic.
    #[serde(default)]
    pub auto_mr_ref_time_ms_threshold: Option<u64>,
    /// Auto-mode: enable mr_reference when inputs >= this count. None => use default heuristic.
    #[serde(default)]
    pub auto_mr_ref_input_threshold: Option<usize>,
}

impl Default for PackerConfig {
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
            texture_outlines: false,
            power_of_two: false,
            square: false,
            use_waste_map: false,
            family: default_family(),
            mr_heuristic: default_mr_heuristic(),
            skyline_heuristic: default_skyline_heuristic(),
            g_choice: default_g_choice(),
            g_split: default_g_split(),
            auto_mode: default_auto_mode(),
            sort_order: default_sort_order(),
            time_budget_ms: None,
            parallel: default_parallel(),
            mr_reference: false,
            auto_mr_ref_time_ms_threshold: None,
            auto_mr_ref_input_threshold: None,
        }
    }
}

impl PackerConfig {
    /// Validates the configuration parameters.
    ///
    /// Returns an error if:
    /// - Dimensions are zero or invalid
    /// - Padding configuration would leave no usable space
    /// - Other configuration constraints are violated
    pub fn validate(&self) -> crate::error::Result<()> {
        use crate::error::TexPackerError;

        // Validate dimensions
        if self.max_width == 0 || self.max_height == 0 {
            return Err(TexPackerError::InvalidDimensions {
                width: self.max_width,
                height: self.max_height,
            });
        }

        // Validate padding doesn't exceed available space
        let total_border = self.border_padding.saturating_mul(2);
        let total_padding_per_texture = self.texture_padding
            .saturating_add(self.texture_extrusion.saturating_mul(2));

        if total_border >= self.max_width || total_border >= self.max_height {
            return Err(TexPackerError::InvalidConfig(format!(
                "border_padding ({}) * 2 exceeds atlas dimensions ({}x{})",
                self.border_padding, self.max_width, self.max_height
            )));
        }

        // Check if there's at least 1x1 pixel of usable space after borders
        let usable_width = self.max_width.saturating_sub(total_border);
        let usable_height = self.max_height.saturating_sub(total_border);

        if usable_width == 0 || usable_height == 0 {
            return Err(TexPackerError::InvalidConfig(format!(
                "No usable space after border_padding: {}x{} - {} * 2 = {}x{}",
                self.max_width, self.max_height, self.border_padding,
                usable_width, usable_height
            )));
        }

        // Warn if padding per texture is very large relative to atlas size
        if total_padding_per_texture > usable_width / 2 || total_padding_per_texture > usable_height / 2 {
            // This is not an error, but might indicate misconfiguration
            // We'll allow it but it might result in poor packing
        }

        // trim_threshold is u8, so it's always valid (0-255)

        Ok(())
    }
}

fn default_family() -> AlgorithmFamily {
    AlgorithmFamily::Skyline
}
fn default_mr_heuristic() -> MaxRectsHeuristic {
    MaxRectsHeuristic::BestAreaFit
}
fn default_skyline_heuristic() -> SkylineHeuristic {
    SkylineHeuristic::BottomLeft
}
fn default_g_choice() -> GuillotineChoice {
    GuillotineChoice::BestAreaFit
}
fn default_g_split() -> GuillotineSplit {
    GuillotineSplit::SplitShorterLeftoverAxis
}
fn default_auto_mode() -> AutoMode {
    AutoMode::Quality
}
fn default_sort_order() -> SortOrder {
    SortOrder::AreaDesc
}
fn default_parallel() -> bool {
    false
}

/// Builder for `PackerConfig` for ergonomic construction.
#[derive(Debug, Default, Clone)]
pub struct PackerConfigBuilder {
    cfg: PackerConfig,
}

impl PackerConfigBuilder {
    pub fn new() -> Self {
        Self {
            cfg: PackerConfig::default(),
        }
    }
    pub fn with_max_dimensions(mut self, w: u32, h: u32) -> Self {
        self.cfg.max_width = w;
        self.cfg.max_height = h;
        self
    }
    pub fn allow_rotation(mut self, v: bool) -> Self {
        self.cfg.allow_rotation = v;
        self
    }
    pub fn force_max_dimensions(mut self, v: bool) -> Self {
        self.cfg.force_max_dimensions = v;
        self
    }
    pub fn border_padding(mut self, v: u32) -> Self {
        self.cfg.border_padding = v;
        self
    }
    pub fn texture_padding(mut self, v: u32) -> Self {
        self.cfg.texture_padding = v;
        self
    }
    pub fn texture_extrusion(mut self, v: u32) -> Self {
        self.cfg.texture_extrusion = v;
        self
    }
    pub fn trim(mut self, v: bool) -> Self {
        self.cfg.trim = v;
        self
    }
    pub fn trim_threshold(mut self, v: u8) -> Self {
        self.cfg.trim_threshold = v;
        self
    }
    pub fn outlines(mut self, v: bool) -> Self {
        self.cfg.texture_outlines = v;
        self
    }
    pub fn pow2(mut self, v: bool) -> Self {
        self.cfg.power_of_two = v;
        self
    }
    pub fn square(mut self, v: bool) -> Self {
        self.cfg.square = v;
        self
    }
    pub fn family(mut self, v: AlgorithmFamily) -> Self {
        self.cfg.family = v;
        self
    }
    pub fn skyline_heuristic(mut self, v: SkylineHeuristic) -> Self {
        self.cfg.skyline_heuristic = v;
        self
    }
    pub fn mr_heuristic(mut self, v: MaxRectsHeuristic) -> Self {
        self.cfg.mr_heuristic = v;
        self
    }
    pub fn g_choice(mut self, v: GuillotineChoice) -> Self {
        self.cfg.g_choice = v;
        self
    }
    pub fn g_split(mut self, v: GuillotineSplit) -> Self {
        self.cfg.g_split = v;
        self
    }
    pub fn auto_mode(mut self, v: AutoMode) -> Self {
        self.cfg.auto_mode = v;
        self
    }
    pub fn sort_order(mut self, v: SortOrder) -> Self {
        self.cfg.sort_order = v;
        self
    }
    pub fn time_budget_ms(mut self, v: Option<u64>) -> Self {
        self.cfg.time_budget_ms = v;
        self
    }
    pub fn parallel(mut self, v: bool) -> Self {
        self.cfg.parallel = v;
        self
    }
    pub fn mr_reference(mut self, v: bool) -> Self {
        self.cfg.mr_reference = v;
        self
    }
    pub fn auto_mr_ref_time_ms_threshold(mut self, v: Option<u64>) -> Self {
        self.cfg.auto_mr_ref_time_ms_threshold = v;
        self
    }
    pub fn auto_mr_ref_input_threshold(mut self, v: Option<usize>) -> Self {
        self.cfg.auto_mr_ref_input_threshold = v;
        self
    }
    pub fn use_waste_map(mut self, v: bool) -> Self {
        self.cfg.use_waste_map = v;
        self
    }
    pub fn build(self) -> PackerConfig {
        self.cfg
    }
}

impl PackerConfig {
    /// Create a fluent builder for `PackerConfig`.
    pub fn builder() -> PackerConfigBuilder {
        PackerConfigBuilder::new()
    }
}
