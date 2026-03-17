use crate::error::ConfigError;

/// Validate a `#RRGGBB` hex color string.
///
/// # Errors
///
/// Returns `ConfigError::InvalidHexColor` if the string doesn't match `#[0-9a-fA-F]{6}`.
pub fn hex_color(s: &str) -> Result<(), ConfigError> {
    let valid = s.len() == 7
        && s.starts_with('#')
        && s[1..].chars().all(|c| c.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(ConfigError::InvalidHexColor(s.to_owned()))
    }
}

/// Validate a workspace name: lowercase alphanumeric + hyphens, must start with alphanumeric.
///
/// # Errors
///
/// Returns `ConfigError::InvalidWorkspaceName` if the name doesn't match the pattern.
pub fn workspace_name(name: &str) -> Result<(), ConfigError> {
    let valid = !name.is_empty()
        && name.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if valid {
        Ok(())
    } else {
        Err(ConfigError::InvalidWorkspaceName(name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_hex_colors() {
        assert!(hex_color("#A3BE8C").is_ok());
        assert!(hex_color("#000000").is_ok());
        assert!(hex_color("#ffffff").is_ok());
        assert!(hex_color("#2E3842").is_ok());
    }

    #[test]
    fn invalid_hex_colors() {
        assert!(hex_color("red").is_err());
        assert!(hex_color("#FFF").is_err());
        assert!(hex_color("#GGGGGG").is_err());
        assert!(hex_color("A3BE8C").is_err());
        assert!(hex_color("").is_err());
        assert!(hex_color("#A3BE8C0").is_err());
    }

    #[test]
    fn valid_workspace_names() {
        assert!(workspace_name("pleme").is_ok());
        assert!(workspace_name("akeyless").is_ok());
        assert!(workspace_name("my-workspace").is_ok());
        assert!(workspace_name("dev1").is_ok());
        assert!(workspace_name("1test").is_ok());
    }

    #[test]
    fn invalid_workspace_names() {
        assert!(workspace_name("").is_err());
        assert!(workspace_name("-bad").is_err());
        assert!(workspace_name("Has Spaces").is_err());
        assert!(workspace_name("UPPER").is_err());
        assert!(workspace_name("under_score").is_err());
    }
}
