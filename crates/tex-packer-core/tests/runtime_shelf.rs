use tex_packer_core::PageId;
use tex_packer_core::prelude::*;

fn runtime_config(page: PageConfigBuilder, policy: ShelfPolicy) -> RuntimeConfig {
    RuntimeConfig::builder()
        .page_config(page.build().expect("valid page config"))
        .strategy(RuntimeStrategy::Shelf { policy })
        .build()
        .expect("valid runtime config")
}

#[test]
fn shelf_nextfit_append_evict_reuse() {
    let cfg = runtime_config(
        PageConfig::builder()
            .max_dimensions(256, 256)
            .allow_rotation(true)
            .texture_padding(2)
            .texture_extrusion(1),
        ShelfPolicy::NextFit,
    );
    let mut sess = AtlasSession::new(cfg);

    let a = sess.append("A".into(), 60, 30).expect("append A");
    sess.append("B".into(), 80, 30).expect("append B");
    assert_eq!(a.page_id(), PageId::new(0));

    assert!(sess.evict(a.page_id(), "A"));
    let c = sess.append("C".into(), 60, 30).expect("reuse C");
    assert_eq!(c.content().w, 60);
    let snap = sess.snapshot_atlas().expect("valid runtime snapshot");
    assert!(disjoint_pages(&snap));
}

#[test]
fn shelf_firstfit_rotation_helps() {
    let cfg = runtime_config(
        PageConfig::builder()
            .max_dimensions(128, 128)
            .allow_rotation(true)
            .texture_padding(0)
            .texture_extrusion(0),
        ShelfPolicy::FirstFit,
    );
    let mut sess = AtlasSession::new(cfg);
    // Create a tall shelf then place a wide-but-short item which fits rotated
    sess.append("Tall".into(), 10, 40).expect("append tall");
    let s2 = sess
        .append("WideShort".into(), 40, 10)
        .expect("append wide");
    // rotation may or may not be used depending on shelf height; we only require it placed and sizes preserved
    assert_eq!(s2.content().w, 40);
    assert_eq!(s2.content().h, 10);
    let snap = sess.snapshot_atlas().expect("valid runtime snapshot");
    assert!(disjoint_pages(&snap));
}

fn disjoint_pages(atlas: &Atlas) -> bool {
    for page in atlas.pages() {
        for i in 0..page.regions().len() {
            for j in (i + 1)..page.regions().len() {
                let a = page.regions()[i].allocation();
                let b = page.regions()[j].allocation();
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
