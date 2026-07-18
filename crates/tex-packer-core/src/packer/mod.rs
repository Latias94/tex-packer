use crate::config::{PackingStrategy, PageConfig};
use crate::geometry::{ContentSize, PhysicalPlacement};

mod guillotine;
mod maxrects;
mod skyline;

use guillotine::GuillotinePacker;
use maxrects::MaxRectsPacker;
use skyline::SkylinePacker;

pub(crate) struct PlacementEngine {
    policy: PlacementPolicy,
}

enum PlacementPolicy {
    Skyline(SkylinePacker),
    MaxRects(MaxRectsPacker),
    Guillotine(GuillotinePacker),
}

impl PlacementEngine {
    pub(crate) fn from_strategy(
        page_config: &PageConfig,
        strategy: &PackingStrategy,
    ) -> Option<Self> {
        let policy = match *strategy {
            PackingStrategy::Skyline {
                heuristic,
                use_waste_map,
            } => PlacementPolicy::Skyline(SkylinePacker::new(
                page_config.clone(),
                heuristic,
                use_waste_map,
            )),
            PackingStrategy::MaxRects {
                heuristic,
                reference,
            } => PlacementPolicy::MaxRects(MaxRectsPacker::new(
                page_config.clone(),
                heuristic,
                reference,
            )),
            PackingStrategy::Guillotine { choice, split } => PlacementPolicy::Guillotine(
                GuillotinePacker::new(page_config.clone(), choice, split),
            ),
            PackingStrategy::Auto { .. } => return None,
        };
        Some(Self { policy })
    }

    pub(crate) fn try_place(&mut self, content: ContentSize) -> Option<PhysicalPlacement> {
        match &mut self.policy {
            PlacementPolicy::Skyline(engine) => engine.try_place(content),
            PlacementPolicy::MaxRects(engine) => engine.try_place(content),
            PlacementPolicy::Guillotine(engine) => engine.try_place(content),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic, SkylineHeuristic,
    };
    use crate::geometry::intersects;

    fn strategies() -> Vec<PackingStrategy> {
        vec![
            PackingStrategy::Skyline {
                heuristic: SkylineHeuristic::BottomLeft,
                use_waste_map: false,
            },
            PackingStrategy::Skyline {
                heuristic: SkylineHeuristic::MinWaste,
                use_waste_map: true,
            },
            PackingStrategy::MaxRects {
                heuristic: MaxRectsHeuristic::BestAreaFit,
                reference: false,
            },
            PackingStrategy::MaxRects {
                heuristic: MaxRectsHeuristic::BestAreaFit,
                reference: true,
            },
            PackingStrategy::Guillotine {
                choice: GuillotineChoice::BestAreaFit,
                split: GuillotineSplit::SplitShorterLeftoverAxis,
            },
        ]
    }

    fn page_config() -> PageConfig {
        PageConfig::builder()
            .max_dimensions(64, 64)
            .allow_rotation(true)
            .texture_padding(0)
            .texture_extrusion(0)
            .build()
            .expect("valid test page")
    }

    #[test]
    fn failed_attempt_leaves_each_engine_unchanged() {
        let page = page_config();
        for strategy in strategies() {
            let mut after_failure =
                PlacementEngine::from_strategy(&page, &strategy).expect("concrete strategy");
            assert!(
                after_failure.try_place(ContentSize::new(65, 65)).is_none(),
                "oversized content must fail"
            );
            let actual = after_failure
                .try_place(ContentSize::new(13, 7))
                .expect("small content fits after failure");

            let mut fresh =
                PlacementEngine::from_strategy(&page, &strategy).expect("concrete strategy");
            let expected = fresh
                .try_place(ContentSize::new(13, 7))
                .expect("small content fits in fresh engine");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn successful_attempt_commits_one_disjoint_allocation() {
        let page = page_config();
        for strategy in strategies() {
            let mut engine =
                PlacementEngine::from_strategy(&page, &strategy).expect("concrete strategy");
            let first = engine
                .try_place(ContentSize::new(13, 7))
                .expect("first content fits");
            let second = engine
                .try_place(ContentSize::new(13, 7))
                .expect("second content fits");
            assert_ne!(first.allocation, second.allocation);
            assert!(!intersects(&first.allocation, &second.allocation));
        }
    }

    #[test]
    fn placement_reports_authoritative_content_and_allocation_geometry() {
        let page = PageConfig::builder()
            .max_dimensions(64, 64)
            .allow_rotation(false)
            .border_padding(3)
            .texture_padding(5)
            .texture_extrusion(2)
            .build()
            .expect("valid padded page");

        for strategy in strategies() {
            let mut engine =
                PlacementEngine::from_strategy(&page, &strategy).expect("concrete strategy");
            let placement = engine
                .try_place(ContentSize::new(10, 8))
                .expect("content fits");
            assert_eq!(placement.allocation.w, 19);
            assert_eq!(placement.allocation.h, 17);
            assert_eq!(placement.content.w, 10);
            assert_eq!(placement.content.h, 8);
            assert_eq!(placement.content.x, placement.allocation.x + 4);
            assert_eq!(placement.content.y, placement.allocation.y + 4);
            assert!(!placement.rotated);
        }
    }

    #[test]
    fn auto_must_be_resolved_before_engine_construction() {
        let strategy = PackingStrategy::Auto {
            mode: AutoMode::Quality,
            time_budget: None,
            parallel: false,
            reference_time_threshold: None,
            reference_input_threshold: None,
        };
        assert!(PlacementEngine::from_strategy(&page_config(), &strategy).is_none());
    }
}
