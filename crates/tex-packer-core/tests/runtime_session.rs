use tex_packer_core::prelude::*;

#[test]
fn runtime_append_evict_reuse_space() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .allow_rotation(true)
        .texture_padding(2)
        .texture_extrusion(1)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

    // Append two items
    let (page_a, a) = sess.append("A".into(), 40, 32).expect("append A");
    let (_page_b, b) = sess.append("B".into(), 48, 24).expect("append B");
    assert_eq!(page_a, 0);
    assert_eq!(a.frame.w, 40);
    assert_eq!(b.frame.h, 24);

    // Evict A, then insert C with similar size to ensure reuse
    assert!(sess.evict(page_a, "A"));
    let (_page_c, c) = sess.append("C".into(), 40, 32).expect("append C");

    // Snapshot and basic sanity: frames should be disjoint
    let snap = sess.snapshot_atlas();
    let mut frames = Vec::new();
    for p in &snap.pages {
        for f in &p.frames {
            frames.push(f.clone());
        }
    }
    assert!(disjoint(&frames));

    // C should fit; not asserting exact coords, but w/h preserved
    assert_eq!(c.frame.w, 40);
    assert_eq!(c.frame.h, 32);
}

fn disjoint(frames: &[Frame]) -> bool {
    for i in 0..frames.len() {
        for j in (i + 1)..frames.len() {
            let a = &frames[i].frame;
            let b = &frames[j].frame;
            let a_x2 = a.x + a.w;
            let a_y2 = a.y + a.h;
            let b_x2 = b.x + b.w;
            let b_y2 = b.y + b.h;
            let overlap = !(a.x >= b_x2 || b.x >= a_x2 || a.y >= b_y2 || b.y >= a_y2);
            if overlap {
                return false;
            }
        }
    }
    true
}
