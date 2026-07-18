//! Packing statistics

use tex_packer_core::PackOutput;

/// Statistics from a packing operation
#[derive(Debug, Clone)]
pub struct PackStats {
    pub num_images: usize,
    pub num_pages: usize,
    pub total_area: u64,
    pub used_area: u64,
    pub occupancy: f32,
    pub pack_time_ms: u64,
    pub avg_page_width: u32,
    pub avg_page_height: u32,
}

impl PackStats {
    /// Calculate statistics from pack output
    pub fn from_output(output: &PackOutput, num_images: usize, pack_time_ms: u64) -> Self {
        let stats = output.stats();

        Self {
            num_images,
            num_pages: stats.num_pages,
            total_area: stats.total_page_area,
            used_area: stats.used_region_area,
            occupancy: (stats.occupancy * 100.0) as f32,
            pack_time_ms,
            avg_page_width: stats.avg_page_width as u32,
            avg_page_height: stats.avg_page_height as u32,
        }
    }

    /// Format as a compact status string
    pub fn status_string(&self) -> String {
        format!(
            "{} images | {} pages | {:.1}% occupancy | {}ms",
            self.num_images, self.num_pages, self.occupancy, self.pack_time_ms
        )
    }

    /// Format as detailed multi-line string
    pub fn detailed_string(&self) -> String {
        format!(
            "Images: {}\nPages: {}\nTotal Area: {} px²\nUsed Area: {} px²\nOccupancy: {:.2}%\nPack Time: {} ms\nAvg Page Size: {}x{}",
            self.num_images,
            self.num_pages,
            self.total_area,
            self.used_area,
            self.occupancy,
            self.pack_time_ms,
            self.avg_page_width,
            self.avg_page_height
        )
    }
}
