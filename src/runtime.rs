use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════
// Data types
// ═══════════════════════════════════════════════════════════════════

/// A single wrapper entry in the runtime config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrapperEntry {
    pub binary_name: String,
    pub workspace: String,
    pub target_bin: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Resolved wrapper ready for exec — all env vars expanded, args assembled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedWrapper {
    pub workspace: String,
    pub target: String,
    pub args: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════════
// WrapperResolver trait — testable resolution without exec
// ═══════════════════════════════════════════════════════════════════

/// Resolve a wrapper name to an executable target + args.
/// Implement this trait for custom config backends (filesystem, in-memory).
pub trait WrapperResolver {
    /// Find a wrapper by name and return the resolved target + args.
    ///
    /// # Errors
    ///
    /// Returns an error if the wrapper is unknown or config can't be read.
    fn resolve(&self, name: &str) -> anyhow::Result<ResolvedWrapper>;
}

/// Resolves wrappers from YAML/JSON files in a directory.
pub struct FsResolver {
    pub config_dir: PathBuf,
}

impl FsResolver {
    #[must_use]
    pub fn from_xdg() -> Self {
        Self { config_dir: config_dir() }
    }
}

impl WrapperResolver for FsResolver {
    fn resolve(&self, name: &str) -> anyhow::Result<ResolvedWrapper> {
        let wrappers = load_wrappers(&self.config_dir)
            .with_context(|| format!("loading wrappers from {}", self.config_dir.display()))?;

        let entry = wrappers
            .iter()
            .find(|w| w.binary_name == name)
            .ok_or_else(|| {
                anyhow!("unknown wrapper '{name}' — no entry in {}", self.config_dir.display())
            })?;

        Ok(ResolvedWrapper {
            workspace: entry.workspace.clone(),
            target: expand_env(&entry.target_bin),
            args: entry.args.iter().map(|a| expand_env(a)).collect(),
        })
    }
}

/// In-memory resolver for testing. No filesystem, no env var reads.
pub struct MockResolver {
    pub entries: Vec<WrapperEntry>,
}

impl WrapperResolver for MockResolver {
    fn resolve(&self, name: &str) -> anyhow::Result<ResolvedWrapper> {
        let entry = self.entries
            .iter()
            .find(|w| w.binary_name == name)
            .ok_or_else(|| anyhow!("unknown wrapper '{name}'"))?;

        Ok(ResolvedWrapper {
            workspace: entry.workspace.clone(),
            target: expand_env(&entry.target_bin),
            args: entry.args.iter().map(|a| expand_env(a)).collect(),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════
// Exec — thin untestable layer (just sets env + calls exec)
// ═══════════════════════════════════════════════════════════════════

/// Resolve a wrapper by name and exec it. Replaces the current process.
///
/// # Errors
///
/// Returns an error if resolution fails or exec fails.
pub fn exec_wrapper(name: &str) -> anyhow::Result<()> {
    exec_with_resolver(&FsResolver::from_xdg(), name)
}

/// Resolve and exec using a specific resolver. Testable up to the exec call.
///
/// # Errors
///
/// Returns an error if resolution fails or exec fails.
pub fn exec_with_resolver(resolver: &dyn WrapperResolver, name: &str) -> anyhow::Result<()> {
    let resolved = resolver.resolve(name)?;

    // SAFETY: runs before exec(), which replaces the process. No other threads.
    unsafe { env::set_var("WORKSPACE", &resolved.workspace) };

    let extra_args: Vec<String> = env::args().skip(1).collect();

    let err = std::process::Command::new(&resolved.target)
        .args(&resolved.args)
        .args(&extra_args)
        .exec();

    Err(anyhow!("failed to exec {}: {err}", resolved.target))
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

/// Expand `$VARNAME` references in a string using the current environment.
///
/// Supports `$VAR` syntax (alphanumeric + underscore). Missing vars are
/// left as-is (`$UNKNOWN` stays `$UNKNOWN`). Bare `$` at end of string
/// is preserved.
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

/// Find the wrappers.d config directory via XDG.
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
/// Supports `.yaml`, `.yml`, and `.json` extensions.
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
            _ => serde_yaml_ng::from_str(&content)
                .with_context(|| format!("failed to parse YAML {}", path.display()))?,
        };
        entries.extend(batch);
    }
    Ok(entries)
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ─── expand_env ──────────────────────────────────────────────

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
    fn expand_env_multiple_vars() {
        unsafe { env::set_var("_WC_A", "hello") };
        unsafe { env::set_var("_WC_B", "world") };
        assert_eq!(expand_env("$_WC_A/$_WC_B"), "hello/world");
    }

    // ─── load_wrappers ──────────────────────────────────────────

    #[test]
    fn load_wrappers_from_yaml() {
        let dir = TempDir::new().unwrap();
        let yaml = serde_yaml_ng::to_string(&vec![WrapperEntry {
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
        let yaml = serde_yaml_ng::to_string(&vec![WrapperEntry {
            binary_name: "ghostty-pleme".into(), workspace: "pleme".into(),
            target_bin: "/bin/ghostty".into(), args: vec![],
        }]).unwrap();
        let json = serde_json::to_string(&vec![WrapperEntry {
            binary_name: "claude-pleme".into(), workspace: "pleme".into(),
            target_bin: "claude".into(), args: vec![],
        }]).unwrap();
        fs::write(dir.path().join("ghostty.yaml"), &yaml).unwrap();
        fs::write(dir.path().join("claude.json"), &json).unwrap();

        let entries = load_wrappers(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn load_wrappers_empty_dir() {
        let dir = TempDir::new().unwrap();
        assert!(load_wrappers(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn load_wrappers_nonexistent_dir() {
        assert!(load_wrappers(Path::new("/nonexistent")).unwrap().is_empty());
    }

    #[test]
    fn wrapper_entry_serialize_roundtrip() {
        let entry = WrapperEntry {
            binary_name: "claude-pleme".into(), workspace: "pleme".into(),
            target_bin: "claude".into(),
            args: vec!["--settings".into(), "/nix/store/abc/settings.json".into()],
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: WrapperEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, parsed);
    }

    // ─── WrapperResolver trait ──────────────────────────────────

    #[test]
    fn mock_resolver_finds_entry() {
        let resolver = MockResolver {
            entries: vec![
                WrapperEntry {
                    binary_name: "ghostty-pleme".into(),
                    workspace: "pleme".into(),
                    target_bin: "/nix/store/abc/bin/ghostty".into(),
                    args: vec!["--config-file=/config".into()],
                },
            ],
        };
        let resolved = resolver.resolve("ghostty-pleme").unwrap();
        assert_eq!(resolved.workspace, "pleme");
        assert_eq!(resolved.target, "/nix/store/abc/bin/ghostty");
        assert_eq!(resolved.args, vec!["--config-file=/config"]);
    }

    #[test]
    fn mock_resolver_unknown_wrapper() {
        let resolver = MockResolver { entries: vec![] };
        assert!(resolver.resolve("nonexistent").is_err());
    }

    #[test]
    fn mock_resolver_expands_env_in_args() {
        unsafe { env::set_var("_WC_TEST_HOME2", "/Users/test") };
        let resolver = MockResolver {
            entries: vec![WrapperEntry {
                binary_name: "ghostty-pleme".into(),
                workspace: "pleme".into(),
                target_bin: "/bin/ghostty".into(),
                args: vec!["--config-file=$_WC_TEST_HOME2/.config/ghostty/config-pleme".into()],
            }],
        };
        let resolved = resolver.resolve("ghostty-pleme").unwrap();
        assert_eq!(
            resolved.args,
            vec!["--config-file=/Users/test/.config/ghostty/config-pleme"]
        );
    }

    #[test]
    fn fs_resolver_from_yaml() {
        let dir = TempDir::new().unwrap();
        let yaml = serde_yaml_ng::to_string(&vec![WrapperEntry {
            binary_name: "ghostty-akeyless".into(),
            workspace: "akeyless".into(),
            target_bin: "/bin/ghostty".into(),
            args: vec!["--flag".into()],
        }]).unwrap();
        fs::write(dir.path().join("ghostty.yaml"), &yaml).unwrap();

        let resolver = FsResolver { config_dir: dir.path().to_path_buf() };
        let resolved = resolver.resolve("ghostty-akeyless").unwrap();
        assert_eq!(resolved.workspace, "akeyless");
        assert_eq!(resolved.target, "/bin/ghostty");
        assert_eq!(resolved.args, vec!["--flag"]);
    }

    #[test]
    fn fs_resolver_unknown_wrapper() {
        let dir = TempDir::new().unwrap();
        let resolver = FsResolver { config_dir: dir.path().to_path_buf() };
        let err = resolver.resolve("nonexistent").unwrap_err();
        assert!(err.to_string().contains("unknown wrapper"));
    }

    #[test]
    fn resolved_wrapper_equality() {
        let a = ResolvedWrapper {
            workspace: "pleme".into(),
            target: "/bin/ghostty".into(),
            args: vec!["--flag".into()],
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
