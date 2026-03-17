use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use workspace_config::config::WorkspaceSet;
use workspace_config::{ghostty, plist, wrapper};

#[derive(Parser)]
#[command(name = "workspace-config", about = "Typed config generator for Ghostty workspace isolation")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate all workspace artifacts (configs, wrappers, app bundles).
    GenerateAll {
        /// Path to JSON input file.
        #[arg(long)]
        input: PathBuf,
        /// Output directory for Ghostty config files.
        #[arg(long)]
        config_dir: PathBuf,
        /// Output directory for wrapper scripts.
        #[arg(long)]
        wrapper_dir: PathBuf,
        /// Output directory for macOS .app bundles.
        #[arg(long)]
        app_dir: PathBuf,
    },
    /// Validate a JSON input file without generating output.
    Validate {
        /// Path to JSON input file.
        #[arg(long)]
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::GenerateAll {
            input,
            config_dir,
            wrapper_dir,
            app_dir,
        } => {
            let json = fs::read_to_string(&input)
                .with_context(|| format!("failed to read {}", input.display()))?;
            let ws_set: WorkspaceSet =
                serde_json::from_str(&json).context("failed to parse workspace JSON")?;
            ws_set.validate().context("workspace validation failed")?;

            fs::create_dir_all(&config_dir)?;
            fs::create_dir_all(&wrapper_dir)?;
            fs::create_dir_all(&app_dir)?;

            for ws in &ws_set.workspaces {
                // Config file
                let config_content =
                    ghostty::generate_config(&ws_set.base_config_path, ws);
                let config_path = config_dir.join(format!("config-{}", ws.name));
                fs::write(&config_path, &config_content)?;

                // Wrapper script
                let wrapper_content =
                    wrapper::generate_wrapper(&ws_set.ghostty_bin, ws);
                let wrapper_path = wrapper_dir.join(format!("ghostty-{}", ws.name));
                fs::write(&wrapper_path, &wrapper_content)?;
                fs::set_permissions(&wrapper_path, fs::Permissions::from_mode(0o755))?;

                // macOS .app bundle
                let app_name = format!("Ghostty {}.app", ws.display_name);
                let app_base = app_dir.join(&app_name);
                let macos_dir = app_base.join("Contents/MacOS");
                fs::create_dir_all(&macos_dir)?;

                // App wrapper (same as PATH wrapper)
                let app_wrapper_path = macos_dir.join(format!("ghostty-{}", ws.name));
                fs::write(&app_wrapper_path, &wrapper_content)?;
                fs::set_permissions(
                    &app_wrapper_path,
                    fs::Permissions::from_mode(0o755),
                )?;

                // Info.plist
                let plist_bytes =
                    plist::generate_info_plist(&ws_set.bundle_id_prefix, ws)?;
                let plist_path = app_base.join("Contents/Info.plist");
                fs::write(&plist_path, &plist_bytes)?;
            }

            Ok(())
        }
        Command::Validate { input } => {
            let json = fs::read_to_string(&input)
                .with_context(|| format!("failed to read {}", input.display()))?;
            let ws_set: WorkspaceSet =
                serde_json::from_str(&json).context("failed to parse workspace JSON")?;
            ws_set.validate().context("workspace validation failed")?;
            eprintln!(
                "valid: {} workspace(s)",
                ws_set.workspaces.len()
            );
            Ok(())
        }
    }
}
