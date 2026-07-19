use crate::export_manifest::{ExportManifest, ExportPage};
use crate::model::Atlas;

/// Build a basic Apple plist (XML) with frames in a dict keyed by name.
/// Multi-page atlases include page id and size fields for each frame.
/// Use `to_plist_hash_with_pages` to inject texture filenames into meta.
/// Duplicate frame keys are not lossless after parsing into a plist dictionary.
pub fn to_plist_hash(atlas: &Atlas) -> String {
    let manifest = ExportManifest::from_atlas(atlas);
    render_plist_hash(&manifest, None)
}

fn render_plist_hash(manifest: &ExportManifest, page_names: Option<&[String]>) -> String {
    let mut s = String::new();
    s.push_str(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>frames</key>
  <dict>
"#,
    );
    for page in &manifest.pages {
        for frame in &page.frames {
            s.push_str(&format!(
                "    <key>{}</key>\n    <dict>\n      <key>page</key><integer>{}</integer>\n      <key>pageSize</key><string>{{{}, {}}}</string>\n      <key>frame</key><string>{}</string>\n      <key>rotated</key><{} />\n      <key>trimmed</key><{} />\n      <key>spriteSourceSize</key><string>{}</string>\n      <key>sourceSize</key><string>{{{}, {}}}</string>\n      <key>pivot</key><string>{{{:.2}, {:.2}}}</string>\n    </dict>\n",
                xml_escape(&frame.key),
                frame.page,
                frame.page_size.0, frame.page_size.1,
                plist_rect(frame.frame),
                if frame.rotated { "true" } else { "false" },
                if frame.trimmed { "true" } else { "false" },
                plist_rect(frame.sprite_source_size),
                frame.source_size.0, frame.source_size.1,
                frame.pivot.0, frame.pivot.1,
            ));
        }
    }
    s.push_str("  </dict>\n");
    s.push_str("  <key>meta</key>\n  <dict>\n");

    if let Some(page_names) = page_names {
        s.push_str(&plist_texture_files_xml(page_names));
    }

    s.push_str(&format!(
        "    <key>app</key><string>{}</string>\n    <key>version</key><string>{}</string>\n    <key>format</key><string>{}</string>\n    <key>scale</key><real>{:.2}</real>\n    <key>allowRotation</key><{} />\n    <key>powerOfTwo</key><{} />\n    <key>square</key><{} />\n    <key>premultipliedAlpha</key><false />\n    <key>smartupdate</key><string></string>\n",
        xml_escape(&manifest.meta.app),
        xml_escape(&manifest.meta.version),
        xml_escape(&manifest.meta.format),
        manifest.meta.scale,
        if manifest.meta.allow_rotation { "true" } else { "false" },
        if manifest.meta.power_of_two { "true" } else { "false" },
        if manifest.meta.square { "true" } else { "false" },
    ));

    if page_names.is_none() {
        s.push_str(&format!(
            "    <key>pages</key><array>\n{}    </array>\n",
            manifest.pages.iter().map(page_size_xml).collect::<String>()
        ));
    } else if manifest.pages.len() == 1
        && let Some(p0) = manifest.pages.first()
    {
        s.push_str(&format!(
            "    <key>size</key><string>{{{}, {}}}</string>\n",
            p0.width, p0.height
        ));
    }

    s.push_str("  </dict>\n</dict>\n</plist>\n");
    s
}

fn plist_rect(rect: crate::model::Rect) -> String {
    format!("{{{{{},{}}},{{{},{}}}}}", rect.x, rect.y, rect.w, rect.h)
}

fn page_size_xml(page: &ExportPage) -> String {
    format!(
        "      <string>{{{}, {}}}</string>\n",
        page.width, page.height
    )
}

fn plist_texture_files_xml(page_names: &[String]) -> String {
    let single = page_names.len() == 1;
    if single {
        page_names
            .first()
            .map(|name| {
                format!(
                    "    <key>textureFileName</key><string>{}</string>\n    <key>realTextureFileName</key><string>{}</string>\n",
                    xml_escape(name),
                    xml_escape(name)
                )
            })
            .unwrap_or_default()
    } else {
        let mut xml = String::from("    <key>textureFileNames</key><array>\n");
        for name in page_names {
            xml.push_str(&format!("      <string>{}</string>\n", xml_escape(name)));
        }
        xml.push_str("    </array>\n");
        xml
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Same as `to_plist_hash`, but includes single `textureFileName` / multi `textureFileNames` in meta.
pub fn to_plist_hash_with_pages(atlas: &Atlas, page_names: &[String]) -> String {
    let manifest = ExportManifest::from_atlas_with_page_names(atlas, page_names);
    render_plist_hash(&manifest, Some(page_names))
}
