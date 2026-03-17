use serde::Deserialize;

use crate::error::ConfigError;
use crate::validate;

/// Top-level input: all workspaces plus shared settings.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSet {
    /// Path to the base Ghostty config (e.g. `~/.config/ghostty/config`).
    pub base_config_path: String,
    /// Absolute path to the Ghostty binary.
    pub ghostty_bin: String,
    /// Bundle ID prefix for macOS .app bundles (e.g. `io.pleme`).
    pub bundle_id_prefix: String,
    /// Workspace definitions keyed by name.
    pub workspaces: Vec<Workspace>,
}

/// A single workspace definition.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    /// Internal name (used for file names, env var value, wrapper binary name).
    pub name: String,
    /// Human-readable name (title bar, Spotlight).
    pub display_name: String,
    /// Optional theme color overrides.
    #[serde(default)]
    pub theme: ThemeConfig,
    /// Extra Ghostty config lines appended verbatim.
    #[serde(default)]
    pub extra_config: String,
}

/// Optional color overrides — all fields default to `None`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeConfig {
    pub cursor_color: Option<HexColor>,
    pub selection_background: Option<HexColor>,
    pub background: Option<HexColor>,
}

/// Validated `#RRGGBB` hex color.
#[derive(Debug, Clone)]
pub struct HexColor(String);

impl HexColor {
    /// Create a new `HexColor`, validating the format.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::InvalidHexColor` if the string is not `#RRGGBB`.
    pub fn new(s: &str) -> Result<Self, ConfigError> {
        validate::hex_color(s)?;
        Ok(Self(s.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(&s).map_err(serde::de::Error::custom)
    }
}

impl WorkspaceSet {
    /// Validate all workspaces in the set.
    ///
    /// # Errors
    ///
    /// Returns validation errors for invalid workspace names or duplicate names.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut seen = std::collections::HashSet::new();
        for ws in &self.workspaces {
            validate::workspace_name(&ws.name)?;
            if !seen.insert(&ws.name) {
                return Err(ConfigError::DuplicateWorkspace(ws.name.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_full_workspace_set() {
        let json = r##"{
            "baseConfigPath": "/home/user/.config/ghostty/config",
            "ghosttyBin": "/nix/store/abc/bin/ghostty",
            "bundleIdPrefix": "io.pleme",
            "workspaces": [
                {
                    "name": "pleme",
                    "displayName": "pleme",
                    "theme": {
                        "cursorColor": "#A3BE8C",
                        "selectionBackground": "#4C566A",
                        "background": "#2E3842"
                    },
                    "extraConfig": ""
                }
            ]
        }"##;
        let ws: WorkspaceSet = serde_json::from_str(json).unwrap();
        assert_eq!(ws.workspaces.len(), 1);
        assert_eq!(ws.workspaces[0].name, "pleme");
        assert_eq!(
            ws.workspaces[0].theme.cursor_color.as_ref().unwrap().as_str(),
            "#A3BE8C",
        );
    }

    #[test]
    fn deserialize_minimal_workspace() {
        let json = r#"{
            "baseConfigPath": "/config",
            "ghosttyBin": "/bin/ghostty",
            "bundleIdPrefix": "io.test",
            "workspaces": [
                {
                    "name": "dev",
                    "displayName": "Development"
                }
            ]
        }"#;
        let ws: WorkspaceSet = serde_json::from_str(json).unwrap();
        assert!(ws.workspaces[0].theme.cursor_color.is_none());
        assert!(ws.workspaces[0].extra_config.is_empty());
    }

    #[test]
    fn validate_duplicate_names() {
        let ws = WorkspaceSet {
            base_config_path: String::new(),
            ghostty_bin: String::new(),
            bundle_id_prefix: String::new(),
            workspaces: vec![
                Workspace {
                    name: "dev".into(),
                    display_name: "Dev".into(),
                    theme: ThemeConfig::default(),
                    extra_config: String::new(),
                },
                Workspace {
                    name: "dev".into(),
                    display_name: "Dev 2".into(),
                    theme: ThemeConfig::default(),
                    extra_config: String::new(),
                },
            ],
        };
        assert!(ws.validate().is_err());
    }

    #[test]
    fn invalid_hex_color_rejected() {
        let json = r#"{
            "baseConfigPath": "/config",
            "ghosttyBin": "/bin/ghostty",
            "bundleIdPrefix": "io.test",
            "workspaces": [{
                "name": "bad",
                "displayName": "Bad",
                "theme": { "cursorColor": "red" }
            }]
        }"#;
        let result: Result<WorkspaceSet, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
