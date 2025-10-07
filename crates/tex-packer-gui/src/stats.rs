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
        let num_pages = output.pages.len();
        let mut total_area = 0u64;
        let mut used_area = 0u64;
        let mut total_width = 0u64;
        let mut total_height = 0u64;

        for page in &output.pages {
            let page_area = (page.page.width as u64) * (page.page.height as u64);
            total_area += page_area;
            total_width += page.page.width as u64;
            total_height += page.page.height as u64;

            // Calculate used area from frames
            for frame in &page.page.frames {
                let frame_area = (frame.frame.w as u64) * (frame.frame.h as u64);
                used_area += frame_area;
            }
        }

        let occupancy = if total_area > 0 {
            (used_area as f32 / total_area as f32) * 100.0
        } else {
            0.0
        };

        let avg_page_width = if num_pages > 0 {
            (total_width / num_pages as u64) as u32
        } else {
            0
        };

        let avg_page_height = if num_pages > 0 {
            (total_height / num_pages as u64) as u32
        } else {
            0
        };

        Self {
            num_images,
            num_pages,
            total_area,
            used_area,
            occupancy,
            pack_time_ms,
            avg_page_width,
            avg_page_height,
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
