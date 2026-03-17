use std::collections::BTreeMap;
use std::io::Write;

use crate::config::Workspace;

/// Generate an `Info.plist` for a macOS `.app` bundle.
///
/// Uses the `plist` crate for valid XML output instead of string interpolation.
///
/// # Errors
///
/// Returns an error if plist serialization fails.
pub fn generate_info_plist(
    bundle_id_prefix: &str,
    ws: &Workspace,
) -> anyhow::Result<Vec<u8>> {
    let mut dict = BTreeMap::new();
    dict.insert(
        "CFBundleName".to_owned(),
        plist::Value::String(format!("Ghostty {}", ws.display_name)),
    );
    dict.insert(
        "CFBundleExecutable".to_owned(),
        plist::Value::String(format!("ghostty-{}", ws.name)),
    );
    dict.insert(
        "CFBundleIdentifier".to_owned(),
        plist::Value::String(format!("{bundle_id_prefix}.ghostty-{}", ws.name)),
    );
    dict.insert(
        "CFBundleVersion".to_owned(),
        plist::Value::String("1.0".to_owned()),
    );
    dict.insert(
        "CFBundlePackageType".to_owned(),
        plist::Value::String("APPL".to_owned()),
    );

    let value = plist::Value::Dictionary(dict.into_iter().collect());
    let mut buf = Vec::new();
    value.to_writer_xml(&mut buf)?;
    // Ensure trailing newline
    if !buf.ends_with(b"\n") {
        buf.write_all(b"\n")?;
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ThemeConfig, Workspace};

    #[test]
    fn plist_contains_required_keys() {
        let ws = Workspace {
            name: "pleme".into(),
            display_name: "pleme".into(),
            theme: ThemeConfig::default(),
            extra_config: String::new(),
        };
        let bytes = generate_info_plist("io.pleme", &ws).unwrap();
        let xml = String::from_utf8(bytes).unwrap();

        assert!(xml.contains("<key>CFBundleName</key>"));
        assert!(xml.contains("<string>Ghostty pleme</string>"));
        assert!(xml.contains("<key>CFBundleExecutable</key>"));
        assert!(xml.contains("<string>ghostty-pleme</string>"));
        assert!(xml.contains("<key>CFBundleIdentifier</key>"));
        assert!(xml.contains("<string>io.pleme.ghostty-pleme</string>"));
        assert!(xml.contains("<key>CFBundleVersion</key>"));
        assert!(xml.contains("<string>1.0</string>"));
        assert!(xml.contains("<key>CFBundlePackageType</key>"));
        assert!(xml.contains("<string>APPL</string>"));
    }

    #[test]
    fn plist_is_valid_xml() {
        let ws = Workspace {
            name: "test".into(),
            display_name: "Test WS".into(),
            theme: ThemeConfig::default(),
            extra_config: String::new(),
        };
        let bytes = generate_info_plist("com.example", &ws).unwrap();
        let xml = String::from_utf8(bytes).unwrap();

        assert!(xml.starts_with("<?xml version=\"1.0\""));
        assert!(xml.contains("<!DOCTYPE plist"));
        assert!(xml.contains("<plist version=\"1.0\">"));
        assert!(xml.contains("</plist>"));
    }
}
