//! Find plugin files in installed packages
//!
//! Handles locating plugins.py or plugin.py files,
//! using entry_points.txt when available.

use crate::errors::BridgeError;
use std::path::{Path, PathBuf};

/// Find plugins.py file using entry_points.txt lookup
pub fn find_plugins_py_via_entry_points(
    package_name_full: &str,
    package_version: Option<&str>,
    venv_path: &str,
) -> Result<PathBuf, BridgeError> {
    // Construct the dist-info directory path
    let normalized_name = package_name_full.replace('-', "_");
    let version = package_version.unwrap_or("0.0.0");
    let dist_info_name = format!("{}-{}.dist-info", normalized_name, version);

    // Find the dist-info directory in site-packages
    let venv_lib = PathBuf::from(venv_path).join("lib");
    let python_version_dir = std::fs::read_dir(&venv_lib).ok().and_then(|entries| {
        entries
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with("python"))
    });

    if let Some(py_dir) = python_version_dir {
        let site_packages = py_dir.path().join("site-packages");
        let dist_info_path = site_packages.join(&dist_info_name);
        let entry_points_file = dist_info_path.join("entry_points.txt");

        if entry_points_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&entry_points_file) {
                if let Some(module_path) = parse_entry_points(&content) {
                    // Convert module path (e.g., "r2x_sienna_to_plexos.plugin") to file path
                    // Replace dots with slashes and add .py extension
                    let file_path = module_path.replace('.', "/") + ".py";
                    let possible_path = site_packages.join(&file_path);

                    if possible_path.exists() {
                        return Ok(possible_path);
                    }
                }
            }
        }
    }

    Err(BridgeError::PluginNotFound(
        "entry_points.txt not found".to_string(),
    ))
}

/// Parse entry_points.txt and extract the r2x_plugin module path
fn parse_entry_points(content: &str) -> Option<String> {
    let mut in_r2x_plugin = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[r2x_plugin]" {
            in_r2x_plugin = true;
            continue;
        }

        if in_r2x_plugin {
            if line.starts_with('[') {
                // New section, stop parsing r2x_plugin
                break;
            }

            if !line.is_empty() && !line.starts_with('#') {
                // Parse "name = module.path:function"
                if let Some(eq_pos) = line.find('=') {
                    let value = line[eq_pos + 1..].trim();
                    if let Some(colon_pos) = value.find(':') {
                        let module_path = value[..colon_pos].trim();
                        return Some(module_path.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Find plugins.py (or plugin.py) in the package directory
///
/// Handles both normal installs (site-packages/r2x_reeds/plugins.py)
/// and editable installs (src/r2x_reeds/plugins.py where path points to src/)
/// Also handles both naming conventions: plugins.py and plugin.py
pub fn find_plugins_py(package_path: &Path) -> Result<PathBuf, BridgeError> {
    // Try both naming conventions: plugins.py and plugin.py
    let filenames = ["plugins.py", "plugin.py"];

    // First try direct path (for normal site-packages installs)
    for filename in &filenames {
        let plugins_file = package_path.join(filename);
        if plugins_file.exists() {
            return Ok(plugins_file);
        }
    }

    // For editable installs, search in subdirectories
    // The path typically points to 'src/', so look for package directories
    match std::fs::read_dir(package_path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        for filename in &filenames {
                            let plugins_file = path.join(filename);
                            if plugins_file.exists() {
                                return Ok(plugins_file);
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {}
    }

    Err(BridgeError::PluginNotFound(format!(
        "plugins.py or plugin.py not found in: {}",
        package_path.display()
    )))
}
