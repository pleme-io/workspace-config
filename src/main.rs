use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use workspace_config::config::WorkspaceSet;
use workspace_config::{ghostty, plist, runtime, wrapper};

fn main() -> Result<()> {
    // Multicall: if invoked as anything other than "workspace-config", run as wrapper
    let argv0 = std::env::args().next().unwrap_or_default();
    let binary_name = std::path::Path::new(&argv0)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("workspace-config");

    if binary_name != "workspace-config" {
        return runtime::exec_wrapper(binary_name);
    }

    let cli = Cli::parse();

    match cli.command {
        Command::GenerateAll {
            input,
            config_dir,
            wrapper_dir,
            app_dir,
        } => generate_all(&input, &config_dir, &wrapper_dir, &app_dir),
        Command::Validate { input } => validate(&input),
        Command::Exec { name } => runtime::exec_wrapper(&name),
    }
}

#[derive(Parser)]
#[command(name = "workspace-config", about = "Typed config generator for Ghostty workspace isolation")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate all workspace artifacts (configs, runtime config, app bundles).
    GenerateAll {
        /// Path to JSON input file.
        #[arg(long)]
        input: PathBuf,
        /// Output directory for Ghostty config files.
        #[arg(long)]
        config_dir: PathBuf,
        /// Output directory for runtime wrapper config.
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
    /// Execute a wrapper by name (reads ~/.config/workspace-config/wrappers.d/).
    Exec {
        /// Wrapper binary name (e.g. ghostty-pleme).
        name: String,
    },
}

fn generate_all(
    input: &PathBuf,
    config_dir: &PathBuf,
    wrapper_dir: &PathBuf,
    app_dir: &PathBuf,
) -> Result<()> {
    let json =
        fs::read_to_string(input).with_context(|| format!("failed to read {}", input.display()))?;
    let jd = &mut serde_json::Deserializer::from_str(&json);
    let ws_set: WorkspaceSet = serde_path_to_error::deserialize(jd)
        .context("failed to parse workspace JSON")?;
    ws_set.validate().context("workspace validation failed")?;

    fs::create_dir_all(config_dir)?;
    fs::create_dir_all(wrapper_dir)?;
    fs::create_dir_all(app_dir)?;

    let mut wrapper_entries = Vec::new();

    for ws in &ws_set.workspaces {
        // Ghostty config file
        let config_content = ghostty::generate_config(&ws_set.base_config_path, ws);
        fs::write(config_dir.join(format!("config-{}", ws.name)), &config_content)?;

        // Wrapper entry for runtime config
        wrapper_entries.push(wrapper::ghostty_wrapper_entry(&ws_set.ghostty_bin, ws));

        // macOS .app bundle
        let app_name = format!("Ghostty {}.app", ws.display_name);
        let app_base = app_dir.join(&app_name);
        let macos_dir = app_base.join("Contents/MacOS");
        fs::create_dir_all(&macos_dir)?;

        // Info.plist
        let plist_bytes = plist::generate_info_plist(&ws_set.bundle_id_prefix, ws)?;
        fs::write(app_base.join("Contents/Info.plist"), &plist_bytes)?;
    }

    // Write runtime wrapper config as YAML (shikumi convention)
    let wrappers_yaml = serde_yaml_ng::to_string(&wrapper_entries)?;
    fs::write(wrapper_dir.join("wrappers.yaml"), &wrappers_yaml)?;

    // Write binary names list (consumed by Nix to create symlinks)
    let names: Vec<&str> = wrapper_entries.iter().map(|e| e.binary_name.as_str()).collect();
    fs::write(wrapper_dir.join("binary-names"), names.join("\n") + "\n")?;

    // .app bundle executables need to be symlinks to workspace-config at Nix level
    // (can't create cross-derivation symlinks here — Nix module handles this)
    // Write a marker so Nix knows which binaries go in which .app
    for ws in &ws_set.workspaces {
        let app_name = format!("Ghostty {}.app", ws.display_name);
        let marker_path = app_dir
            .join(&app_name)
            .join("Contents/MacOS")
            .join(format!(".wrapper-name-{}", ws.name));
        fs::write(&marker_path, format!("ghostty-{}", ws.name))?;
        // Set the executable permission on the marker so the .app structure is valid
        fs::set_permissions(&marker_path, fs::Permissions::from_mode(0o755))?;
    }

    Ok(())
}

fn validate(input: &PathBuf) -> Result<()> {
    let json =
        fs::read_to_string(input).with_context(|| format!("failed to read {}", input.display()))?;
    let jd = &mut serde_json::Deserializer::from_str(&json);
    let ws_set: WorkspaceSet = serde_path_to_error::deserialize(jd)
        .context("failed to parse workspace JSON")?;
    ws_set.validate().context("workspace validation failed")?;
    eprintln!("valid: {} workspace(s)", ws_set.workspaces.len());
    Ok(())
}
