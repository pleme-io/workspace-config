use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};

/// A single wrapper entry in the runtime config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrapperEntry {
    /// Name of the wrapper binary (e.g. `ghostty-pleme`, `claude-akeyless`).
    pub binary_name: String,
    /// Workspace name to set in `WORKSPACE` env var.
    pub workspace: String,
    /// Target binary to exec.
    pub target_bin: String,
    /// Arguments to prepend before user args. Supports `$HOME` expansion.
    #[serde(default)]
    pub args: Vec<String>,
}

/// Expand `$VARNAME` references in a string using the current environment.
#[must_use]
pub fn expand_env(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            let mut var_name = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    var_name.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if var_name.is_empty() {
                result.push('$');
            } else if let Ok(val) = env::var(&var_name) {
                result.push_str(&val);
            } else {
                result.push('$');
                result.push_str(&var_name);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Find the wrappers.d config directory.
#[must_use]
pub fn config_dir() -> PathBuf {
    env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env::var("HOME").unwrap_or_default()).join(".config")
        })
        .join("workspace-config/wrappers.d")
}

/// Load all wrapper entries from YAML/JSON files in a directory.
///
/// Supports `.yaml`, `.yml`, and `.json` extensions (shikumi convention: YAML preferred).
///
/// # Errors
///
/// Returns an error if a config file can't be read or parsed.
pub fn load_wrappers(dir: &Path) -> anyhow::Result<Vec<WrapperEntry>> {
    let mut entries = Vec::new();
    if !dir.is_dir() {
        return Ok(entries);
    }
    let mut paths: Vec<_> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e == "yaml" || e == "yml" || e == "json")
        })
        .collect();
    paths.sort();
    for path in paths {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let batch: Vec<WrapperEntry> = match path.extension().and_then(|e| e.to_str()) {
            Some("json") => serde_json::from_str(&content)
                .with_context(|| format!("failed to parse JSON {}", path.display()))?,
            _ => serde_yaml::from_str(&content)
                .with_context(|| format!("failed to parse YAML {}", path.display()))?,
        };
        entries.extend(batch);
    }
    Ok(entries)
}

/// Execute a wrapper by name: set WORKSPACE, exec target binary.
///
/// # Errors
///
/// Returns an error if the wrapper is unknown or exec fails.
pub fn exec_wrapper(name: &str) -> anyhow::Result<()> {
    let dir = config_dir();
    let wrappers =
        load_wrappers(&dir).with_context(|| format!("loading wrappers from {}", dir.display()))?;

    let entry = wrappers
        .iter()
        .find(|w| w.binary_name == name)
        .ok_or_else(|| anyhow!("unknown wrapper '{name}' — no entry in {}", dir.display()))?;

    // Set workspace env var
    // SAFETY: This runs before exec(), which replaces the process. No other threads exist.
    unsafe { env::set_var("WORKSPACE", &entry.workspace) };

    // Expand env vars in args and target
    let target = expand_env(&entry.target_bin);
    let expanded_args: Vec<String> = entry.args.iter().map(|a| expand_env(a)).collect();

    // Collect extra args from command line (passthrough)
    let extra_args: Vec<String> = env::args().skip(1).collect();

    // Exec — replaces current process, never returns on success
    let err = std::process::Command::new(&target)
        .args(&expanded_args)
        .args(&extra_args)
        .exec();

    Err(anyhow!("failed to exec {target}: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn expand_env_home() {
        // SAFETY: single-threaded test
        unsafe { env::set_var("_WC_TEST_HOME", "/Users/testuser") };
        assert_eq!(
            expand_env("$_WC_TEST_HOME/.config/ghostty/config-pleme"),
            "/Users/testuser/.config/ghostty/config-pleme"
        );
    }

    #[test]
    fn expand_env_no_vars() {
        assert_eq!(expand_env("/nix/store/abc/bin/ghostty"), "/nix/store/abc/bin/ghostty");
    }

    #[test]
    fn expand_env_missing_var() {
        // SAFETY: single-threaded test
        unsafe { env::remove_var("_WC_NONEXISTENT_12345") };
        assert_eq!(expand_env("$_WC_NONEXISTENT_12345/path"), "$_WC_NONEXISTENT_12345/path");
    }

    #[test]
    fn expand_env_dollar_at_end() {
        assert_eq!(expand_env("path$"), "path$");
    }

    #[test]
    fn load_wrappers_from_yaml() {
        let dir = TempDir::new().unwrap();
        let yaml = serde_yaml::to_string(&vec![WrapperEntry {
            binary_name: "ghostty-pleme".into(),
            workspace: "pleme".into(),
            target_bin: "/bin/ghostty".into(),
            args: vec!["--config-file".into(), "/config".into()],
        }])
        .unwrap();
        fs::write(dir.path().join("ghostty.yaml"), &yaml).unwrap();

        let entries = load_wrappers(dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].binary_name, "ghostty-pleme");
        assert_eq!(entries[0].workspace, "pleme");
    }

    #[test]
    fn load_wrappers_from_json() {
        let dir = TempDir::new().unwrap();
        let json = serde_json::to_string(&vec![WrapperEntry {
            binary_name: "claude-pleme".into(),
            workspace: "pleme".into(),
            target_bin: "claude".into(),
            args: vec![],
        }])
        .unwrap();
        fs::write(dir.path().join("claude.json"), &json).unwrap();

        let entries = load_wrappers(dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].binary_name, "claude-pleme");
    }

    #[test]
    fn load_wrappers_mixed_formats() {
        let dir = TempDir::new().unwrap();
        let yaml = serde_yaml::to_string(&vec![WrapperEntry {
            binary_name: "ghostty-pleme".into(),
            workspace: "pleme".into(),
            target_bin: "/bin/ghostty".into(),
            args: vec![],
        }])
        .unwrap();
        let json = serde_json::to_string(&vec![WrapperEntry {
            binary_name: "claude-pleme".into(),
            workspace: "pleme".into(),
            target_bin: "claude".into(),
            args: vec![],
        }])
        .unwrap();
        fs::write(dir.path().join("ghostty.yaml"), &yaml).unwrap();
        fs::write(dir.path().join("claude.json"), &json).unwrap();

        let entries = load_wrappers(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn load_wrappers_empty_dir() {
        let dir = TempDir::new().unwrap();
        let entries = load_wrappers(dir.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn load_wrappers_nonexistent_dir() {
        let entries = load_wrappers(Path::new("/nonexistent")).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn wrapper_entry_serialize_roundtrip() {
        let entry = WrapperEntry {
            binary_name: "claude-pleme".into(),
            workspace: "pleme".into(),
            target_bin: "claude".into(),
            args: vec!["--settings".into(), "/nix/store/abc/settings.json".into()],
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: WrapperEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.binary_name, "claude-pleme");
        assert_eq!(parsed.args.len(), 2);
    }
}
