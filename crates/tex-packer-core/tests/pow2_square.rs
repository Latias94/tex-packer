use rand::{Rng, SeedableRng};
use tex_packer_core::prelude::*;

fn is_pow2(v: u32) -> bool {
    v != 0 && (v & (v - 1)) == 0
}

fn max_allocation_extents(page: &Page, cfg: &PageConfig) -> (u32, u32) {
    page.regions()
        .iter()
        .fold((0, 0), |(width, height), region| {
            let allocation = region.allocation();
            (
                width.max(allocation.x + allocation.w + cfg.border_padding()),
                height.max(allocation.y + allocation.h + cfg.border_padding()),
            )
        })
}

#[test]
fn pow2_resizes_page_dimensions() {
    let page = PageConfig::builder()
        .max_dimensions(300, 180)
        .texture_padding(4)
        .texture_extrusion(2)
        .border_padding(5)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .power_of_two(true)
        .build()
        .expect("valid offline config");
    let inputs = vec![("a", 64, 32), ("b", 40, 80), ("c", 10, 10)];
    let atlas = tex_packer_core::pack_layout(inputs, cfg.clone()).expect("pack");
    assert!(!atlas.pages().is_empty());
    let p = &atlas.pages()[0];
    let (min_w, min_h) = max_allocation_extents(p, cfg.page_config());
    assert!(is_pow2(p.width()));
    assert!(is_pow2(p.height()));
    assert!(p.width() >= min_w);
    assert!(p.height() >= min_h);
}

#[test]
fn square_resizes_page_dimensions() {
    let page = PageConfig::builder()
        .max_dimensions(300, 180)
        .texture_padding(2)
        .texture_extrusion(0)
        .border_padding(0)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .square(true)
        .build()
        .expect("valid offline config");
    let inputs = vec![("a", 120, 16), ("b", 40, 40)];
    let atlas = tex_packer_core::pack_layout(inputs, cfg.clone()).expect("pack");
    let p = &atlas.pages()[0];
    assert_eq!(p.width(), p.height());
    let (min_w, min_h) = max_allocation_extents(p, cfg.page_config());
    let min_side = min_w.max(min_h);
    assert!(p.width() >= min_side && p.height() >= min_side);
}

#[test]
fn pow2_and_square_combo() {
    let page = PageConfig::builder()
        .max_dimensions(500, 300)
        .texture_padding(3)
        .texture_extrusion(1)
        .border_padding(7)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .power_of_two(true)
        .square(true)
        .build()
        .expect("valid offline config");
    let inputs = vec![("x", 123, 77), ("y", 200, 20)];
    let atlas = tex_packer_core::pack_layout(inputs, cfg.clone()).expect("pack");
    let p = &atlas.pages()[0];
    assert_eq!(p.width(), p.height());
    assert!(is_pow2(p.width()));
    let (min_w, min_h) = max_allocation_extents(p, cfg.page_config());
    let need = min_w.max(min_h);
    assert!(p.width() >= need);
}

#[test]
fn force_max_dimensions_exact() {
    let page = PageConfig::builder()
        .max_dimensions(256, 192)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .force_max_dimensions(true)
        .build()
        .expect("valid offline config");
    let inputs = vec![("a", 10, 10)];
    let atlas = tex_packer_core::pack_layout(inputs, cfg.clone()).expect("pack");
    let p = &atlas.pages()[0];
    assert_eq!(p.width(), 256);
    assert_eq!(p.height(), 192);
}

#[test]
fn random_no_overlap_pow2_square() {
    let page = PageConfig::builder()
        .max_dimensions(512, 512)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .power_of_two(true)
        .square(true)
        .build()
        .expect("valid offline config");
    let mut rng = rand::rngs::StdRng::seed_from_u64(2024);
    let mut items: Vec<(String, u32, u32)> = Vec::new();
    for i in 0..200u32 {
        let w = rng.gen_range(1..=64);
        let h = rng.gen_range(1..=64);
        items.push((format!("r{}", i), w, h));
    }
    let atlas = tex_packer_core::pack_layout(items, cfg.clone()).expect("pack");
    for page in atlas.pages() {
        // no overlap between authoritative physical allocations
        for i in 0..page.regions().len() {
            for j in (i + 1)..page.regions().len() {
                let a = page.regions()[i].allocation();
                let b = page.regions()[j].allocation();
                let ax2 = a.x + a.w;
                let ay2 = a.y + a.h;
                let bx2 = b.x + b.w;
                let by2 = b.y + b.h;
                let overlap = !(a.x >= bx2 || b.x >= ax2 || a.y >= by2 || b.y >= ay2);
                assert!(!overlap, "frames overlap: {:?} vs {:?}", a, b);
            }
        }
        // within page bounds
        for region in page.regions() {
            assert!(region.allocation().right() < page.width());
            assert!(region.allocation().bottom() < page.height());
        }
        assert_eq!(page.width(), page.height());
        assert!(is_pow2(page.width()));
    }
}
