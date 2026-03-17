use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn sample_json() -> String {
    r##"{
        "baseConfigPath": "/home/user/.config/ghostty/config",
        "ghosttyBin": "/nix/store/abc123/bin/ghostty",
        "bundleIdPrefix": "io.pleme",
        "workspaces": [
            {
                "name": "pleme",
                "displayName": "pleme",
                "theme": {
                    "cursorColor": "#A3BE8C",
                    "selectionBackground": "#4C566A",
                    "background": "#2E3842"
                }
            },
            {
                "name": "akeyless",
                "displayName": "akeyless",
                "theme": {
                    "cursorColor": "#88C0D0",
                    "selectionBackground": "#3B4252",
                    "background": "#2E3540"
                }
            }
        ]
    }"##
    .to_owned()
}

#[test]
fn validate_succeeds_on_valid_input() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("input.json");
    fs::write(&input, sample_json()).unwrap();

    Command::cargo_bin("workspace-config")
        .unwrap()
        .args(["validate", "--input", input.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("valid: 2 workspace(s)"));
}

#[test]
fn validate_fails_on_invalid_json() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("bad.json");
    fs::write(&input, "not json").unwrap();

    Command::cargo_bin("workspace-config")
        .unwrap()
        .args(["validate", "--input", input.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn validate_fails_on_bad_workspace_name() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("bad_name.json");
    fs::write(
        &input,
        r#"{
            "baseConfigPath": "/config",
            "ghosttyBin": "/bin/ghostty",
            "bundleIdPrefix": "io.test",
            "workspaces": [{
                "name": "Bad Name",
                "displayName": "Bad"
            }]
        }"#,
    )
    .unwrap();

    Command::cargo_bin("workspace-config")
        .unwrap()
        .args(["validate", "--input", input.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn generate_all_creates_expected_files() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("input.json");
    fs::write(&input, sample_json()).unwrap();

    let config_dir = dir.path().join("configs");
    let wrapper_dir = dir.path().join("wrappers");
    let app_dir = dir.path().join("apps");

    Command::cargo_bin("workspace-config")
        .unwrap()
        .args([
            "generate-all",
            "--input",
            input.to_str().unwrap(),
            "--config-dir",
            config_dir.to_str().unwrap(),
            "--wrapper-dir",
            wrapper_dir.to_str().unwrap(),
            "--app-dir",
            app_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Config files
    assert!(config_dir.join("config-pleme").exists());
    assert!(config_dir.join("config-akeyless").exists());

    let pleme_config = fs::read_to_string(config_dir.join("config-pleme")).unwrap();
    assert!(pleme_config.contains("# Ghostty Workspace: pleme"));
    assert!(pleme_config.contains("config-file = /home/user/.config/ghostty/config"));
    assert!(pleme_config.contains("title = pleme"));
    assert!(pleme_config.contains("cursor-color = #A3BE8C"));
    assert!(pleme_config.contains("selection-background = #4C566A"));
    assert!(pleme_config.contains("background = #2E3842"));

    // Wrapper scripts
    assert!(wrapper_dir.join("ghostty-pleme").exists());
    assert!(wrapper_dir.join("ghostty-akeyless").exists());

    let pleme_wrapper = fs::read_to_string(wrapper_dir.join("ghostty-pleme")).unwrap();
    assert!(pleme_wrapper.contains("#!/bin/bash"));
    assert!(pleme_wrapper.contains("export WORKSPACE=\"pleme\""));
    assert!(pleme_wrapper.contains("/nix/store/abc123/bin/ghostty"));

    // App bundles
    let pleme_app = app_dir.join("Ghostty pleme.app");
    assert!(pleme_app.join("Contents/MacOS/ghostty-pleme").exists());
    assert!(pleme_app.join("Contents/Info.plist").exists());

    let plist_content =
        fs::read_to_string(pleme_app.join("Contents/Info.plist")).unwrap();
    assert!(plist_content.contains("Ghostty pleme"));
    assert!(plist_content.contains("io.pleme.ghostty-pleme"));

    let akeyless_app = app_dir.join("Ghostty akeyless.app");
    assert!(akeyless_app.join("Contents/MacOS/ghostty-akeyless").exists());
    assert!(akeyless_app.join("Contents/Info.plist").exists());
}

#[test]
fn generate_all_wrapper_is_executable() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let input = dir.path().join("input.json");
    fs::write(&input, sample_json()).unwrap();

    let config_dir = dir.path().join("configs");
    let wrapper_dir = dir.path().join("wrappers");
    let app_dir = dir.path().join("apps");

    Command::cargo_bin("workspace-config")
        .unwrap()
        .args([
            "generate-all",
            "--input",
            input.to_str().unwrap(),
            "--config-dir",
            config_dir.to_str().unwrap(),
            "--wrapper-dir",
            wrapper_dir.to_str().unwrap(),
            "--app-dir",
            app_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let perms = fs::metadata(wrapper_dir.join("ghostty-pleme"))
        .unwrap()
        .permissions();
    assert_eq!(perms.mode() & 0o111, 0o111, "wrapper should be executable");
}
