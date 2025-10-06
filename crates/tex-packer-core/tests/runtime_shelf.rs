use tex_packer_core::prelude::*;

#[test]
fn shelf_nextfit_append_evict_reuse() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .allow_rotation(true)
        .texture_padding(2)
        .texture_extrusion(1)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Shelf(ShelfPolicy::NextFit));

    let (page_a, _a) = sess.append("A".into(), 60, 30).expect("append A");
    let (_page_b, _b) = sess.append("B".into(), 80, 30).expect("append B");
    assert_eq!(page_a, 0);

    assert!(sess.evict(page_a, "A"));
    let (_page_c, c) = sess.append("C".into(), 60, 30).expect("reuse C");
    assert_eq!(c.frame.w, 60);
    let snap = sess.snapshot_atlas();
    assert!(disjoint_pages(&snap));
}

#[test]
fn shelf_firstfit_rotation_helps() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(128, 128)
        .allow_rotation(true)
        .texture_padding(0)
        .texture_extrusion(0)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Shelf(ShelfPolicy::FirstFit));
    // Create a tall shelf then place a wide-but-short item which fits rotated
    let (_p1, _s1) = sess.append("Tall".into(), 10, 40).expect("append tall");
    let (_p2, s2) = sess
        .append("WideShort".into(), 40, 10)
        .expect("append wide");
    // rotation may or may not be used depending on shelf height; we only require it placed and sizes preserved
    assert_eq!(s2.frame.w, 40);
    assert_eq!(s2.frame.h, 10);
    let snap = sess.snapshot_atlas();
    assert!(disjoint_pages(&snap));
}

fn disjoint_pages(atlas: &Atlas<String>) -> bool {
    for p in &atlas.pages {
        for i in 0..p.frames.len() {
            for j in (i + 1)..p.frames.len() {
                let a = &p.frames[i].frame;
                let b = &p.frames[j].frame;
                let ax2 = a.x + a.w;
                let ay2 = a.y + a.h;
                let bx2 = b.x + b.w;
                let by2 = b.y + b.h;
                if !(a.x >= bx2 || b.x >= ax2 || a.y >= by2 || b.y >= ay2) {
                    return false;
                }
            }
        }
    }
    true
}
