use crate::model::Atlas;
use serde::Serialize;

/// Build a basic Apple plist (XML) with frames in a dict keyed by name.
/// Multi-page atlases include page id and size fields for each frame.
/// Use `to_plist_hash_with_pages` to inject texture filenames into meta.
pub fn to_plist_hash<K: ToString + Clone + Serialize>(atlas: &Atlas<K>) -> String {
    // Very basic Apple plist (XML) with frames in a dict keyed by name. Multi-page adds page id and size fields.
    let mut s = String::new();
    s.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>frames</key>
  <dict>
"#);
    for page in &atlas.pages {
        for fr in &page.frames {
            let name = fr.key.to_string();
            let frame = format!(
                "{{{{{},{}}},{{{},{}}}}}",
                fr.frame.x, fr.frame.y, fr.frame.w, fr.frame.h
            );
            let source = format!(
                "{{{{{},{}}},{{{},{}}}}}",
                fr.source.x, fr.source.y, fr.source.w, fr.source.h
            );
            s.push_str(&format!(
                "    <key>{}</key>\n    <dict>\n      <key>page</key><integer>{}</integer>\n      <key>pageSize</key><string>{{{}, {}}}</string>\n      <key>frame</key><string>{}</string>\n      <key>rotated</key><{} />\n      <key>trimmed</key><{} />\n      <key>spriteSourceSize</key><string>{}</string>\n      <key>sourceSize</key><string>{{{}, {}}}</string>\n      <key>pivot</key><string>{{{:.2}, {:.2}}}</string>\n    </dict>\n",
                xml_escape(&name),
                page.id,
                page.width, page.height,
                frame,
                if fr.rotated { "true" } else { "false" },
                if fr.trimmed { "true" } else { "false" },
                source,
                fr.source_size.0, fr.source_size.1,
                0.5, 0.5,
            ));
        }
    }
    s.push_str("  </dict>\n");
    s.push_str("  <key>meta</key>\n  <dict>\n");
    s.push_str(&format!(
        "    <key>app</key><string>{}</string>\n    <key>version</key><string>{}</string>\n    <key>format</key><string>{}</string>\n    <key>scale</key><real>{:.2}</real>\n    <key>allowRotation</key><{} />\n    <key>powerOfTwo</key><{} />\n    <key>square</key><{} />\n    <key>premultipliedAlpha</key><false />\n    <key>smartupdate</key><string></string>\n    <key>pages</key><array>\n{}    </array>\n",
        xml_escape(&atlas.meta.app),
        xml_escape(&atlas.meta.version),
        xml_escape(&atlas.meta.format),
        atlas.meta.scale,
        if atlas.meta.allow_rotation { "true" } else { "false" },
        if atlas.meta.power_of_two { "true" } else { "false" },
        if atlas.meta.square { "true" } else { "false" },
        atlas.pages.iter().map(|p| format!("      <string>{{{}, {}}}</string>\n", p.width, p.height)).collect::<String>()
    ));
    s.push_str("  </dict>\n</dict>\n</plist>\n");
    s
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Same as `to_plist_hash`, but includes single `textureFileName` / multi `textureFileNames` in meta.
pub fn to_plist_hash_with_pages<K: ToString + Clone + Serialize>(
    atlas: &Atlas<K>,
    page_names: &[String],
) -> String {
    // Same as to_plist_hash, but include filenames in meta for better engine compatibility.
    let mut s = String::new();
    s.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>frames</key>
  <dict>
"#);
    for page in &atlas.pages {
        for fr in &page.frames {
            let name = fr.key.to_string();
            let frame = format!(
                "{{{{{},{}}},{{{},{}}}}}",
                fr.frame.x, fr.frame.y, fr.frame.w, fr.frame.h
            );
            let source = format!(
                "{{{{{},{}}},{{{},{}}}}}",
                fr.source.x, fr.source.y, fr.source.w, fr.source.h
            );
            s.push_str(&format!(
                "    <key>{}</key>\n    <dict>\n      <key>page</key><integer>{}</integer>\n      <key>pageSize</key><string>{{{}, {}}}</string>\n      <key>frame</key><string>{}</string>\n      <key>rotated</key><{} />\n      <key>trimmed</key><{} />\n      <key>spriteSourceSize</key><string>{}</string>\n      <key>sourceSize</key><string>{{{}, {}}}</string>\n      <key>pivot</key><string>{{{:.2}, {:.2}}}</string>\n    </dict>\n",
                xml_escape(&name),
                page.id,
                page.width, page.height,
                frame,
                if fr.rotated { "true" } else { "false" },
                if fr.trimmed { "true" } else { "false" },
                source,
                fr.source_size.0, fr.source_size.1,
                0.5, 0.5,
            ));
        }
    }
    s.push_str("  </dict>\n");
    s.push_str("  <key>meta</key>\n  <dict>\n");
    let single = page_names.len() == 1;
    let images_xml = if single {
        page_names.first().map(|n| format!(
            "    <key>textureFileName</key><string>{}</string>\n    <key>realTextureFileName</key><string>{}</string>\n",
            xml_escape(n), xml_escape(n)
        )).unwrap_or_default()
    } else {
        let mut arr = String::new();
        arr.push_str("    <key>textureFileNames</key><array>\n");
        for n in page_names {
            arr.push_str(&format!("      <string>{}</string>\n", xml_escape(n)));
        }
        arr.push_str("    </array>\n");
        arr
    };
    s.push_str(&format!(
        "    <key>app</key><string>{}</string>\n    <key>version</key><string>{}</string>\n    <key>format</key><string>{}</string>\n    <key>scale</key><real>{:.2}</real>\n    <key>allowRotation</key><{} />\n    <key>powerOfTwo</key><{} />\n    <key>square</key><{} />\n    <key>premultipliedAlpha</key><false />\n    <key>smartupdate</key><string></string>\n{}",
        xml_escape(&atlas.meta.app),
        xml_escape(&atlas.meta.version),
        xml_escape(&atlas.meta.format),
        atlas.meta.scale,
        if atlas.meta.allow_rotation { "true" } else { "false" },
        if atlas.meta.power_of_two { "true" } else { "false" },
        if atlas.meta.square { "true" } else { "false" },
        images_xml
    ));
    if single {
        if let Some(p0) = atlas.pages.first() {
            s.push_str(&format!(
                "    <key>size</key><string>{{{}, {}}}</string>\n",
                p0.width, p0.height
            ));
        }
    }
    s.push_str("  </dict>\n</dict>\n</plist>\n");
    s
}
