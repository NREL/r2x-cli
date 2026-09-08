//! Public launcher regressions for UV-managed Python startup.

#![cfg(unix)]

use assert_cmd::cargo::cargo_bin;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn launcher_handles_version_without_uv_or_python() {
    let Ok(temp_dir) = TempDir::new() else {
        return;
    };
    let path = temp_dir.path().join("path-without-python-or-uv");
    if fs::create_dir_all(&path).is_err() {
        return;
    }

    let config_path = temp_dir.path().join("config.toml");
    let venv_path = temp_dir.path().join(".venv");
    if fs::write(
        &config_path,
        format!(
            "python_version = \"3.13\"\nvenv_path = \"{}\"\n",
            venv_path.display(),
        ),
    )
    .is_err()
    {
        return;
    }

    let output = Command::new(cargo_bin("r2x"))
        .arg("--version")
        .env("PATH", &path)
        .env("R2X_CONFIG", &config_path)
        .env("PYTHONHOME", "poison")
        .env("PYTHONPATH", "poison")
        .output();

    assert!(
        output.as_ref().is_ok_and(|output| output.status.success()),
        "launcher failed: {output:?}"
    );
    assert!(!venv_path.exists());
}
