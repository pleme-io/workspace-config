use crate::config::Workspace;
use crate::runtime::WrapperEntry;

/// Generate a `WrapperEntry` for a ghostty workspace.
#[must_use]
pub fn ghostty_wrapper_entry(ghostty_bin: &str, ws: &Workspace) -> WrapperEntry {
    WrapperEntry {
        binary_name: format!("ghostty-{}", ws.name),
        workspace: ws.name.clone(),
        target_bin: ghostty_bin.to_owned(),
        args: vec![
            format!("--config-file=$HOME/.config/ghostty/config-{}", ws.name),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ThemeConfig, Workspace};

    #[test]
    fn wrapper_entry_fields() {
        let ws = Workspace {
            name: "pleme".into(),
            display_name: "pleme".into(),
            theme: ThemeConfig::default(),
            extra_config: String::new(),
        };
        let entry = ghostty_wrapper_entry("/nix/store/abc/bin/ghostty", &ws);

        assert_eq!(entry.binary_name, "ghostty-pleme");
        assert_eq!(entry.workspace, "pleme");
        assert_eq!(entry.target_bin, "/nix/store/abc/bin/ghostty");
        assert_eq!(entry.args, vec!["--config-file=$HOME/.config/ghostty/config-pleme"]);
    }

    #[test]
    fn wrapper_entry_serializes_to_json() {
        let ws = Workspace {
            name: "akeyless".into(),
            display_name: "akeyless".into(),
            theme: ThemeConfig::default(),
            extra_config: String::new(),
        };
        let entry = ghostty_wrapper_entry("/bin/ghostty", &ws);
        let json = serde_json::to_string_pretty(&entry).unwrap();

        assert!(json.contains("\"binaryName\": \"ghostty-akeyless\""));
        assert!(json.contains("\"workspace\": \"akeyless\""));
    }
}
