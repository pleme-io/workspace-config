use crate::config::Workspace;

/// Generate a Ghostty workspace config file.
///
/// Output matches the existing Nix string-concat format exactly:
/// ```text
/// # Ghostty Workspace: pleme
/// # Managed by Nix (blackmatter.components.ghostty.workspaces)
/// config-file = /path/to/base/config
/// title = pleme
/// cursor-color = #A3BE8C
/// ```
#[must_use]
pub fn generate_config(base_config_path: &str, ws: &Workspace) -> String {
    let mut lines = vec![
        format!("# Ghostty Workspace: {}", ws.name),
        "# Managed by Nix (blackmatter.components.ghostty.workspaces)".to_owned(),
        format!("config-file = {base_config_path}"),
        format!("title = {}", ws.display_name),
    ];

    if let Some(ref color) = ws.theme.cursor_color {
        lines.push(format!("cursor-color = {}", color.as_str()));
    }
    if let Some(ref color) = ws.theme.selection_background {
        lines.push(format!("selection-background = {}", color.as_str()));
    }
    if let Some(ref color) = ws.theme.background {
        lines.push(format!("background = {}", color.as_str()));
    }

    if !ws.extra_config.is_empty() {
        for line in ws.extra_config.lines() {
            lines.push(line.to_owned());
        }
    }

    let mut output = lines.join("\n");
    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{HexColor, ThemeConfig, Workspace};

    #[test]
    fn full_config_output() {
        let ws = Workspace {
            name: "pleme".into(),
            display_name: "pleme".into(),
            theme: ThemeConfig {
                cursor_color: Some(HexColor::new("#A3BE8C").unwrap()),
                selection_background: Some(HexColor::new("#4C566A").unwrap()),
                background: Some(HexColor::new("#2E3842").unwrap()),
            },
            extra_config: String::new(),
        };
        let output = generate_config("/home/user/.config/ghostty/config", &ws);
        assert_eq!(
            output,
            "# Ghostty Workspace: pleme\n\
             # Managed by Nix (blackmatter.components.ghostty.workspaces)\n\
             config-file = /home/user/.config/ghostty/config\n\
             title = pleme\n\
             cursor-color = #A3BE8C\n\
             selection-background = #4C566A\n\
             background = #2E3842\n"
        );
    }

    #[test]
    fn minimal_config_output() {
        let ws = Workspace {
            name: "dev".into(),
            display_name: "Development".into(),
            theme: ThemeConfig::default(),
            extra_config: String::new(),
        };
        let output = generate_config("/config", &ws);
        assert_eq!(
            output,
            "# Ghostty Workspace: dev\n\
             # Managed by Nix (blackmatter.components.ghostty.workspaces)\n\
             config-file = /config\n\
             title = Development\n"
        );
    }

    #[test]
    fn config_with_extra_lines() {
        let ws = Workspace {
            name: "test".into(),
            display_name: "Test".into(),
            theme: ThemeConfig::default(),
            extra_config: "font-size = 14\nwindow-padding-x = 8".into(),
        };
        let output = generate_config("/config", &ws);
        assert!(output.contains("font-size = 14\n"));
        assert!(output.contains("window-padding-x = 8\n"));
    }
}
