use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid hex color '{0}': must be #RRGGBB format")]
    InvalidHexColor(String),

    #[error("invalid workspace name '{0}': must be [a-z0-9][a-z0-9-]*")]
    InvalidWorkspaceName(String),

    #[error("duplicate workspace name '{0}'")]
    DuplicateWorkspace(String),
}
