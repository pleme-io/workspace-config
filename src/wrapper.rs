use crate::config::Workspace;

/// Generate a shell wrapper script for a workspace.
///
/// Sets `WORKSPACE` env var and execs Ghostty with the workspace config.
#[must_use]
pub fn generate_wrapper(ghostty_bin: &str, ws: &Workspace) -> String {
    format!(
        "#!/bin/bash\nexport WORKSPACE=\"{name}\"\nexec {ghostty_bin} --config-file=\"$HOME/.config/ghostty/config-{name}\" \"$@\"\n",
        name = ws.name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ThemeConfig, Workspace};

    #[test]
    fn wrapper_sets_workspace_env() {
        let ws = Workspace {
            name: "pleme".into(),
            display_name: "pleme".into(),
            theme: ThemeConfig::default(),
            extra_config: String::new(),
        };
        let script = generate_wrapper("/nix/store/abc/bin/ghostty", &ws);

        assert!(script.starts_with("#!/bin/bash\n"));
        assert!(script.contains("export WORKSPACE=\"pleme\""));
        assert!(script.contains("exec /nix/store/abc/bin/ghostty"));
        assert!(script.contains("--config-file=\"$HOME/.config/ghostty/config-pleme\""));
        assert!(script.contains("\"$@\""));
    }

    #[test]
    fn wrapper_ends_with_newline() {
        let ws = Workspace {
            name: "test".into(),
            display_name: "Test".into(),
            theme: ThemeConfig::default(),
            extra_config: String::new(),
        };
        let script = generate_wrapper("/bin/ghostty", &ws);
        assert!(script.ends_with('\n'));
    }
}
