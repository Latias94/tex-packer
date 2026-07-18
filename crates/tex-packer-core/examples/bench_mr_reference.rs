use rand::{Rng, SeedableRng};
use std::time::Instant;
use tex_packer_core::config::{MaxRectsHeuristic, PageConfig};
use tex_packer_core::model::Rect;
use tex_packer_core::packer::Packer;
use tex_packer_core::packer::maxrects::MaxRectsPacker;

fn run(n: usize, mr_ref: bool, seed: u64) {
    let cfg = PageConfig::builder()
        .max_dimensions(2048, 2048)
        .allow_rotation(true)
        .texture_padding(0)
        .texture_extrusion(0)
        .build()
        .expect("valid page config");

    let mut p = MaxRectsPacker::new(cfg.clone(), MaxRectsHeuristic::BestAreaFit, mr_ref);
    let mut used_area: u64 = 0;
    let page_area: u64 = (cfg.max_width() as u64) * (cfg.max_height() as u64);
    let mut placed = 0usize;
    let mut free_sum: u64 = 0;

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let start = Instant::now();
    for i in 0..n {
        let w: u32 = rng.gen_range(4..=96);
        let h: u32 = rng.gen_range(4..=96);
        let r = Rect::new(0, 0, w, h);
        if let Some(f) = <MaxRectsPacker as Packer<String>>::pack(&mut p, format!("r{}", i), &r) {
            used_area += (f.frame.w as u64) * (f.frame.h as u64);
            placed += 1;
            free_sum += p.free_list_len() as u64;
        } else {
            break;
        }
    }
    let elapsed = start.elapsed();
    let occ = if page_area > 0 {
        used_area as f64 / page_area as f64
    } else {
        0.0
    };
    let avg_free = if placed > 0 {
        free_sum as f64 / placed as f64
    } else {
        0.0
    };
    println!(
        "mr_reference={} placed={} occ={:.2}% avg_free={:.1} time={}ms",
        mr_ref,
        placed,
        occ * 100.0,
        avg_free,
        elapsed.as_millis()
    );
}

fn main() {
    // N=1000 and 5000
    println!("N=1000");
    run(1000, false, 1337);
    run(1000, true, 1337);
    println!("\nN=5000");
    run(5000, false, 4242);
    run(5000, true, 4242);
}
