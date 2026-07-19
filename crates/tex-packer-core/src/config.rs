use std::str::FromStr;
use std::time::Duration;

use crate::error::{Result, TexPackerError};

/// MaxRects placement heuristics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum MaxRectsHeuristic {
    #[default]
    BestAreaFit,
    BestShortSideFit,
    BestLongSideFit,
    BottomLeft,
    ContactPoint,
}

impl FromStr for MaxRectsHeuristic {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SkylineHeuristic {
    #[default]
    BottomLeft,
    MinWaste,
}

impl FromStr for SkylineHeuristic {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "bl" | "bottomleft" => Ok(Self::BottomLeft),
            "minwaste" | "mw" => Ok(Self::MinWaste),
            _ => Err(()),
        }
    }
}

/// Guillotine free-rectangle choice heuristics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GuillotineChoice {
    #[default]
    BestAreaFit,
    BestShortSideFit,
    BestLongSideFit,
    WorstAreaFit,
    WorstShortSideFit,
    WorstLongSideFit,
}

impl FromStr for GuillotineChoice {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
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

/// Guillotine split-axis heuristics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GuillotineSplit {
    #[default]
    SplitShorterLeftoverAxis,
    SplitLongerLeftoverAxis,
    SplitMinimizeArea,
    SplitMaximizeArea,
    SplitShorterAxis,
    SplitLongerAxis,
}

impl FromStr for GuillotineSplit {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
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

/// Auto portfolio presets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AutoMode {
    Fast,
    #[default]
    Quality,
}

impl FromStr for AutoMode {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "fast" => Ok(Self::Fast),
            "quality" => Ok(Self::Quality),
            _ => Err(()),
        }
    }
}

/// Sorting orders for deterministic offline packing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SortOrder {
    #[default]
    AreaDesc,
    MaxSideDesc,
    HeightDesc,
    WidthDesc,
    NameAsc,
    None,
}

impl FromStr for SortOrder {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
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

/// Policy for fully transparent images when trimming is enabled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TransparentPolicy {
    #[default]
    Keep,
    OneByOne,
    Skip,
}

impl FromStr for TransparentPolicy {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "keep" => Ok(Self::Keep),
            "one_by_one" | "1x1" | "onebyone" => Ok(Self::OneByOne),
            "skip" => Ok(Self::Skip),
            _ => Err(()),
        }
    }
}

/// Shelf selection policies for runtime packing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ShelfPolicy {
    #[default]
    NextFit,
    FirstFit,
}

impl FromStr for ShelfPolicy {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "next_fit" | "nextfit" => Ok(Self::NextFit),
            "first_fit" | "firstfit" => Ok(Self::FirstFit),
            _ => Err(()),
        }
    }
}

/// A selected offline packing strategy and only the options relevant to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackingStrategy {
    Skyline {
        heuristic: SkylineHeuristic,
        use_waste_map: bool,
    },
    MaxRects {
        heuristic: MaxRectsHeuristic,
        reference: bool,
    },
    Guillotine {
        choice: GuillotineChoice,
        split: GuillotineSplit,
    },
    Auto {
        mode: AutoMode,
        time_budget: Option<Duration>,
        parallel: bool,
        reference_time_threshold: Option<Duration>,
        reference_input_threshold: Option<usize>,
    },
}

impl Default for PackingStrategy {
    fn default() -> Self {
        Self::Skyline {
            heuristic: SkylineHeuristic::BottomLeft,
            use_waste_map: false,
        }
    }
}

/// A selected runtime packing strategy and only the options relevant to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeStrategy {
    Guillotine {
        choice: GuillotineChoice,
        split: GuillotineSplit,
    },
    Shelf {
        policy: ShelfPolicy,
    },
    Skyline {
        heuristic: SkylineHeuristic,
    },
}

impl Default for RuntimeStrategy {
    fn default() -> Self {
        Self::Guillotine {
            choice: GuillotineChoice::BestAreaFit,
            split: GuillotineSplit::SplitShorterLeftoverAxis,
        }
    }
}

/// Validated geometry shared by offline and runtime workflows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageConfig {
    max_width: u32,
    max_height: u32,
    allow_rotation: bool,
    border_padding: u32,
    texture_padding: u32,
    texture_extrusion: u32,
    usable_width: u32,
    usable_height: u32,
    allocation_extra: u32,
    content_offset: u32,
    trailing_extra: u32,
}

impl PageConfig {
    pub fn builder() -> PageConfigBuilder {
        PageConfigBuilder::default()
    }

    pub fn max_width(&self) -> u32 {
        self.max_width
    }

    pub fn max_height(&self) -> u32 {
        self.max_height
    }

    pub fn max_dimensions(&self) -> (u32, u32) {
        (self.max_width, self.max_height)
    }

    pub fn allow_rotation(&self) -> bool {
        self.allow_rotation
    }

    pub fn border_padding(&self) -> u32 {
        self.border_padding
    }

    pub fn texture_padding(&self) -> u32 {
        self.texture_padding
    }

    pub fn texture_extrusion(&self) -> u32 {
        self.texture_extrusion
    }

    pub(crate) fn usable_dimensions(&self) -> (u32, u32) {
        (self.usable_width, self.usable_height)
    }

    pub(crate) fn content_offset(&self) -> u32 {
        self.content_offset
    }

    pub(crate) fn trailing_extra(&self) -> u32 {
        self.trailing_extra
    }

    pub(crate) fn checked_reservation(
        &self,
        content_width: u32,
        content_height: u32,
    ) -> Option<(u32, u32)> {
        Some((
            content_width.checked_add(self.allocation_extra)?,
            content_height.checked_add(self.allocation_extra)?,
        ))
    }
}

impl Default for PageConfig {
    fn default() -> Self {
        Self {
            max_width: 1024,
            max_height: 1024,
            allow_rotation: true,
            border_padding: 0,
            texture_padding: 2,
            texture_extrusion: 0,
            usable_width: 1024,
            usable_height: 1024,
            allocation_extra: 2,
            content_offset: 1,
            trailing_extra: 1,
        }
    }
}

/// Builder for [`PageConfig`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageConfigBuilder {
    max_width: u32,
    max_height: u32,
    allow_rotation: bool,
    border_padding: u32,
    texture_padding: u32,
    texture_extrusion: u32,
}

impl Default for PageConfigBuilder {
    fn default() -> Self {
        Self {
            max_width: 1024,
            max_height: 1024,
            allow_rotation: true,
            border_padding: 0,
            texture_padding: 2,
            texture_extrusion: 0,
        }
    }
}

impl PageConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_dimensions(mut self, width: u32, height: u32) -> Self {
        self.max_width = width;
        self.max_height = height;
        self
    }

    pub fn allow_rotation(mut self, enabled: bool) -> Self {
        self.allow_rotation = enabled;
        self
    }

    pub fn border_padding(mut self, pixels: u32) -> Self {
        self.border_padding = pixels;
        self
    }

    pub fn texture_padding(mut self, pixels: u32) -> Self {
        self.texture_padding = pixels;
        self
    }

    pub fn texture_extrusion(mut self, pixels: u32) -> Self {
        self.texture_extrusion = pixels;
        self
    }

    pub fn build(self) -> Result<PageConfig> {
        if self.max_width == 0 || self.max_height == 0 {
            return Err(TexPackerError::InvalidDimensions {
                width: self.max_width,
                height: self.max_height,
            });
        }

        let border_twice = self.border_padding.checked_mul(2).ok_or_else(|| {
            invalid_config("border_padding overflows while reserving both page edges")
        })?;
        if border_twice >= self.max_width || border_twice >= self.max_height {
            return Err(invalid_config(format!(
                "border_padding ({}) leaves no usable area in {}x{}",
                self.border_padding, self.max_width, self.max_height
            )));
        }

        let usable_width = self.max_width - border_twice;
        let usable_height = self.max_height - border_twice;
        let allocation_extra = self
            .texture_extrusion
            .checked_mul(2)
            .and_then(|value| value.checked_add(self.texture_padding))
            .ok_or_else(|| {
                invalid_config(
                    "texture padding and extrusion overflow the coordinate representation",
                )
            })?;
        let leading_padding = self.texture_padding / 2;
        let trailing_padding = self.texture_padding - leading_padding;
        let content_offset = self
            .texture_extrusion
            .checked_add(leading_padding)
            .ok_or_else(|| {
                invalid_config("content offset overflows the coordinate representation")
            })?;
        let trailing_extra = self
            .texture_extrusion
            .checked_add(trailing_padding)
            .ok_or_else(|| {
                invalid_config("trailing reservation overflows the coordinate representation")
            })?;
        let minimum_reservation = allocation_extra.checked_add(1).ok_or_else(|| {
            invalid_config(
                "a one-pixel texture reservation overflows the coordinate representation",
            )
        })?;

        if minimum_reservation > usable_width || minimum_reservation > usable_height {
            return Err(invalid_config(format!(
                "padding and extrusion require at least {minimum_reservation}x{minimum_reservation} pixels, but only {usable_width}x{usable_height} are usable"
            )));
        }

        Ok(PageConfig {
            max_width: self.max_width,
            max_height: self.max_height,
            allow_rotation: self.allow_rotation,
            border_padding: self.border_padding,
            texture_padding: self.texture_padding,
            texture_extrusion: self.texture_extrusion,
            usable_width,
            usable_height,
            allocation_extra,
            content_offset,
            trailing_extra,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrimProjection {
    Disabled,
    Enabled {
        threshold: u8,
        transparent_policy: TransparentPolicy,
    },
}

/// Validated configuration for offline image and layout workflows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineConfig {
    page: PageConfig,
    force_max_dimensions: bool,
    power_of_two: bool,
    square: bool,
    trim: TrimProjection,
    outlines: bool,
    sort_order: SortOrder,
    strategy: PackingStrategy,
}

impl OfflineConfig {
    pub fn builder() -> OfflineConfigBuilder {
        OfflineConfigBuilder::default()
    }

    pub fn page_config(&self) -> &PageConfig {
        &self.page
    }

    pub fn force_max_dimensions(&self) -> bool {
        self.force_max_dimensions
    }

    pub fn power_of_two(&self) -> bool {
        self.power_of_two
    }

    pub fn square(&self) -> bool {
        self.square
    }

    pub fn trim_enabled(&self) -> bool {
        matches!(self.trim, TrimProjection::Enabled { .. })
    }

    pub fn trim_threshold(&self) -> Option<u8> {
        match self.trim {
            TrimProjection::Disabled => None,
            TrimProjection::Enabled { threshold, .. } => Some(threshold),
        }
    }

    pub fn transparent_policy(&self) -> Option<TransparentPolicy> {
        match self.trim {
            TrimProjection::Disabled => None,
            TrimProjection::Enabled {
                transparent_policy, ..
            } => Some(transparent_policy),
        }
    }

    pub fn outlines(&self) -> bool {
        self.outlines
    }

    pub fn sort_order(&self) -> SortOrder {
        self.sort_order
    }

    pub fn strategy(&self) -> &PackingStrategy {
        &self.strategy
    }
}

impl Default for OfflineConfig {
    fn default() -> Self {
        Self {
            page: PageConfig::default(),
            force_max_dimensions: false,
            power_of_two: false,
            square: false,
            trim: TrimProjection::Enabled {
                threshold: 0,
                transparent_policy: TransparentPolicy::Keep,
            },
            outlines: false,
            sort_order: SortOrder::AreaDesc,
            strategy: PackingStrategy::default(),
        }
    }
}

/// Builder for [`OfflineConfig`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineConfigBuilder {
    page: PageConfig,
    force_max_dimensions: bool,
    power_of_two: bool,
    square: bool,
    trim: bool,
    trim_threshold: u8,
    transparent_policy: TransparentPolicy,
    outlines: bool,
    sort_order: SortOrder,
    strategy: PackingStrategy,
}

impl Default for OfflineConfigBuilder {
    fn default() -> Self {
        Self {
            page: PageConfig::default(),
            force_max_dimensions: false,
            power_of_two: false,
            square: false,
            trim: true,
            trim_threshold: 0,
            transparent_policy: TransparentPolicy::Keep,
            outlines: false,
            sort_order: SortOrder::AreaDesc,
            strategy: PackingStrategy::default(),
        }
    }
}

impl OfflineConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn page_config(mut self, config: PageConfig) -> Self {
        self.page = config;
        self
    }

    pub fn force_max_dimensions(mut self, enabled: bool) -> Self {
        self.force_max_dimensions = enabled;
        self
    }

    pub fn power_of_two(mut self, enabled: bool) -> Self {
        self.power_of_two = enabled;
        self
    }

    pub fn square(mut self, enabled: bool) -> Self {
        self.square = enabled;
        self
    }

    pub fn trim(mut self, enabled: bool) -> Self {
        self.trim = enabled;
        self
    }

    pub fn trim_threshold(mut self, threshold: u8) -> Self {
        self.trim_threshold = threshold;
        self
    }

    pub fn transparent_policy(mut self, policy: TransparentPolicy) -> Self {
        self.transparent_policy = policy;
        self
    }

    pub fn outlines(mut self, enabled: bool) -> Self {
        self.outlines = enabled;
        self
    }

    pub fn sort_order(mut self, order: SortOrder) -> Self {
        self.sort_order = order;
        self
    }

    pub fn strategy(mut self, strategy: PackingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn build(self) -> Result<OfflineConfig> {
        if self.power_of_two && !self.force_max_dimensions {
            self.page
                .max_width
                .checked_next_power_of_two()
                .ok_or_else(|| {
                    invalid_config(format!(
                        "power-of-two page width overflows for maximum width {}",
                        self.page.max_width
                    ))
                })?;
            self.page
                .max_height
                .checked_next_power_of_two()
                .ok_or_else(|| {
                    invalid_config(format!(
                        "power-of-two page height overflows for maximum height {}",
                        self.page.max_height
                    ))
                })?;
        }

        let trim = if self.trim {
            TrimProjection::Enabled {
                threshold: self.trim_threshold,
                transparent_policy: self.transparent_policy,
            }
        } else {
            TrimProjection::Disabled
        };
        let strategy = normalize_strategy(self.strategy);

        Ok(OfflineConfig {
            page: self.page,
            force_max_dimensions: self.force_max_dimensions,
            power_of_two: self.power_of_two,
            square: self.square,
            trim,
            outlines: self.outlines,
            sort_order: self.sort_order,
            strategy,
        })
    }
}

fn normalize_strategy(strategy: PackingStrategy) -> PackingStrategy {
    match strategy {
        PackingStrategy::Auto {
            mode,
            time_budget,
            parallel,
            reference_time_threshold,
            reference_input_threshold,
        } => PackingStrategy::Auto {
            mode,
            time_budget: time_budget.filter(|budget| !budget.is_zero()),
            parallel,
            reference_time_threshold,
            reference_input_threshold,
        },
        other => other,
    }
}

/// Validated configuration for runtime atlas workflows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeConfig {
    page: PageConfig,
    strategy: RuntimeStrategy,
}

impl RuntimeConfig {
    pub fn builder() -> RuntimeConfigBuilder {
        RuntimeConfigBuilder::default()
    }

    pub fn page_config(&self) -> &PageConfig {
        &self.page
    }

    pub fn strategy(&self) -> &RuntimeStrategy {
        &self.strategy
    }
}

/// Builder for [`RuntimeConfig`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeConfigBuilder {
    page: PageConfig,
    strategy: RuntimeStrategy,
}

impl RuntimeConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn page_config(mut self, config: PageConfig) -> Self {
        self.page = config;
        self
    }

    pub fn strategy(mut self, strategy: RuntimeStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn build(self) -> Result<RuntimeConfig> {
        Ok(RuntimeConfig {
            page: self.page,
            strategy: self.strategy,
        })
    }
}

fn invalid_config(message: impl Into<String>) -> TexPackerError {
    TexPackerError::InvalidConfig(message.into())
}
