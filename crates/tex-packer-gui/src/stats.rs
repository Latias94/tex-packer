//! Packing statistics

use tex_packer_core::model::Atlas;
use tex_packer_core::offline::PackOutput;

/// Statistics from a packing operation
#[derive(Debug, Clone)]
pub struct PackStats {
    pub num_images: usize,
    pub num_pages: usize,
    pub num_frames: usize,
    pub num_regions: usize,
    pub num_aliases: usize,
    pub page_area: u128,
    pub content_area: u128,
    pub allocation_area: u128,
    pub content_occupancy: f64,
    pub allocation_occupancy: f64,
    pub pack_time_ms: u64,
    pub avg_page_width: f64,
    pub avg_page_height: f64,
}

impl PackStats {
    /// Calculate statistics from pack output
    pub fn from_output(output: &PackOutput, num_images: usize, pack_time_ms: u64) -> Self {
        Self::from_atlas(output.atlas(), num_images, pack_time_ms)
    }

    fn from_atlas(atlas: &Atlas, num_images: usize, pack_time_ms: u64) -> Self {
        let stats = atlas.stats();
        let (width_sum, height_sum) = atlas.pages().iter().fold((0u128, 0u128), |sum, page| {
            (
                sum.0 + u128::from(page.width()),
                sum.1 + u128::from(page.height()),
            )
        });
        let page_count = stats.num_pages as f64;

        Self {
            num_images,
            num_pages: stats.num_pages,
            num_frames: stats.num_frames,
            num_regions: stats.num_regions,
            num_aliases: stats.num_aliases,
            page_area: stats.page_area,
            content_area: stats.content_area,
            allocation_area: stats.allocation_area,
            content_occupancy: stats.content_occupancy * 100.0,
            allocation_occupancy: stats.allocation_occupancy * 100.0,
            pack_time_ms,
            avg_page_width: if stats.num_pages == 0 {
                0.0
            } else {
                width_sum as f64 / page_count
            },
            avg_page_height: if stats.num_pages == 0 {
                0.0
            } else {
                height_sum as f64 / page_count
            },
        }
    }

    /// Format as a compact status string
    pub fn status_string(&self) -> String {
        format!(
            "{} images | {} pages | {:.1}% allocation | {}ms",
            self.num_images, self.num_pages, self.allocation_occupancy, self.pack_time_ms
        )
    }

    /// Format as detailed multi-line string
    pub fn detailed_string(&self) -> String {
        format!(
            "Images: {}\nPages: {}\nFrames: {}\nRegions: {}\nAliases: {}\nPage Area: {} px²\nContent Area: {} px²\nAllocation Area: {} px²\nContent Occupancy: {:.2}%\nAllocation Occupancy: {:.2}%\nPack Time: {} ms\nAvg Page Size: {:.0}x{:.0}",
            self.num_images,
            self.num_pages,
            self.num_frames,
            self.num_regions,
            self.num_aliases,
            self.page_area,
            self.content_area,
            self.allocation_area,
            self.content_occupancy,
            self.allocation_occupancy,
            self.pack_time_ms,
            self.avg_page_width,
            self.avg_page_height
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tex_packer_core::model::{Frame, FrameId, Meta, Page, PageId, Rect, Region, RegionId};

    #[test]
    fn reports_content_and_allocation_metrics_without_counting_aliases_twice() {
        let region_id = RegionId::new(4);
        let page = Page::try_new(
            PageId::new(12),
            20,
            10,
            vec![Region::new(
                region_id,
                Rect::new(2, 2, 4, 2),
                Rect::new(1, 1, 6, 4),
                false,
            )],
            vec![
                Frame::new(
                    FrameId::new(1),
                    "first".into(),
                    region_id,
                    false,
                    Rect::new(0, 0, 4, 2),
                    (4, 2),
                ),
                Frame::new(
                    FrameId::new(8),
                    "alias".into(),
                    region_id,
                    false,
                    Rect::new(0, 0, 4, 2),
                    (4, 2),
                ),
            ],
        )
        .expect("test page should be valid");
        let atlas =
            Atlas::try_new(vec![page], Meta::default()).expect("test atlas should be valid");

        let stats = PackStats::from_atlas(&atlas, 2, 17);

        assert_eq!(stats.num_frames, 2);
        assert_eq!(stats.num_regions, 1);
        assert_eq!(stats.num_aliases, 1);
        assert_eq!(stats.page_area, 200);
        assert_eq!(stats.content_area, 8);
        assert_eq!(stats.allocation_area, 24);
        assert_eq!(stats.content_occupancy, 4.0);
        assert_eq!(stats.allocation_occupancy, 12.0);
        assert_eq!(stats.avg_page_width, 20.0);
        assert_eq!(stats.avg_page_height, 10.0);
    }
}
