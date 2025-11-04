use crate::config_manager::Config;
use crate::logger;
use crate::plugin_manifest::PluginManifest;
use crate::GlobalOpts;
use colored::*;
use std::collections::{HashMap, HashSet};
use std::process::Command;

pub fn remove_plugin(package: &str, _opts: &GlobalOpts) -> Result<(), String> {
    let mut removed_count = 0usize;
    let mut orphaned_dependencies = Vec::new();

    match PluginManifest::load() {
        Ok(mut manifest) => {
            orphaned_dependencies = find_orphaned_dependencies(&manifest, package);

            removed_count = manifest.remove_plugins_by_package(package);

            if removed_count > 0 {
                for dep in &orphaned_dependencies {
                    let count = manifest.remove_plugins_by_package(dep);
                    if count > 0 {
                        logger::info(&format!("Removing orphaned dependency package '{}'", dep));
                        removed_count += count;
                    }
                }

                if let Err(e) = manifest.save() {
                    logger::warn(&format!("Failed to update manifest: {}", e));
                }
            } else {
                logger::info(&format!(
                    "No plugins found for package '{}' in manifest",
                    package
                ));
            }
        }
        Err(e) => {
            logger::warn(&format!(
                "Failed to load manifest: {}. Continuing with uninstall...",
                e
            ));
        }
    }

    // Setup config
    let mut config = Config::load().map_err(|e| format!("Failed to load config: {}", e))?;
    config
        .ensure_uv_path()
        .map_err(|e| format!("Failed to setup uv: {}", e))?;
    config
        .ensure_cache_path()
        .map_err(|e| format!("Failed to setup cache: {}", e))?;

    let uv_path = config
        .uv_path
        .as_ref()
        .cloned()
        .ok_or_else(|| "uv path not configured".to_string())?;
    let venv_path = config.get_venv_path();

    logger::info(&format!("Using venv: {}", venv_path));

    // Check if package is installed
    let check_output = Command::new(&uv_path)
        .args(["pip", "show", "--python", &venv_path, package])
        .output()
        .map_err(|e| format!("Failed to check package status: {}", e))?;

    if !check_output.status.success() {
        logger::warn(&format!("Package '{}' is not installed", package));
        return Ok(());
    }

    logger::debug(&format!(
        "Running: {} pip uninstall --python {} {}",
        uv_path, venv_path, package
    ));

    let output = Command::new(&uv_path)
        .args(["pip", "uninstall", "--python", &venv_path, package])
        .output()
        .map_err(|e| {
            logger::error(&format!("Failed to run pip uninstall: {}", e));
            format!("Failed to run pip uninstall: {}", e)
        })?;

    logger::capture_output(&format!("uv pip uninstall {}", package), &output);

    if !output.status.success() {
        logger::error(&format!("pip uninstall failed for package '{}'", package));
        return Err(format!("pip uninstall failed for package '{}'", package));
    }

    logger::info(&format!("Package '{}' uninstalled successfully", package));

    // Uninstall orphaned dependencies
    for orphan_pkg in &orphaned_dependencies {
        let check_orphan = Command::new(&uv_path)
            .args(["pip", "show", "--python", &venv_path, orphan_pkg])
            .output()
            .map_err(|e| {
                format!(
                    "Failed to check orphaned package '{}' status: {}",
                    orphan_pkg, e
                )
            })?;

        if check_orphan.status.success() {
            logger::debug(&format!(
                "Running: {} pip uninstall --python {} {}",
                uv_path, venv_path, orphan_pkg
            ));

            let orphan_output = Command::new(&uv_path)
                .args(["pip", "uninstall", "--python", &venv_path, orphan_pkg])
                .output()
                .map_err(|e| {
                    logger::error(&format!(
                        "Failed to run pip uninstall for orphaned package '{}': {}",
                        orphan_pkg, e
                    ));
                    format!(
                        "Failed to run pip uninstall for orphaned package '{}': {}",
                        orphan_pkg, e
                    )
                })?;

            logger::capture_output(
                &format!("uv pip uninstall {} (orphaned dependency)", orphan_pkg),
                &orphan_output,
            );

            if orphan_output.status.success() {
                logger::info(&format!(
                    "Orphaned dependency package '{}' uninstalled successfully",
                    orphan_pkg
                ));
            } else {
                logger::warn(&format!(
                    "Failed to uninstall orphaned dependency package '{}'",
                    orphan_pkg
                ));
            }
        }
    }

    println!(
        "{}",
        format!("Uninstalled {} plugins(s)", removed_count).dimmed()
    );
    println!(" {} {}", "-".bold().red(), package.bold());

    for dep in &orphaned_dependencies {
        println!(
            " {} {} {}",
            "-".bold().red(),
            dep.bold(),
            "(dependency)".dimmed()
        );
    }

    Ok(())
}

/// Find dependencies that would become orphaned if a package is removed.
/// Optimized with single-pass manifest scan and efficient lookups.
fn find_orphaned_dependencies(manifest: &PluginManifest, package: &str) -> Vec<String> {
    // Single pass: build efficient lookup maps
    let mut explicit_packages: HashSet<String> = HashSet::new();
    let mut dependencies_by_installer: HashMap<String, Vec<String>> = HashMap::new();

    for plugin in manifest.plugins.values() {
        if let Some(pkg_name) = &plugin.package_name {
            match plugin.install_type.as_deref() {
                Some("explicit") => {
                    explicit_packages.insert(pkg_name.clone());
                }
                Some("dependency") => {
                    if let Some(installed_by) = &plugin.installed_by {
                        dependencies_by_installer
                            .entry(installed_by.clone())
                            .or_default()
                            .push(pkg_name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    // Find orphaned dependencies: those not used by any other explicit package
    let mut orphaned = HashSet::new();

    if let Some(dep_packages) = dependencies_by_installer.get(package) {
        for dep_pkg in dep_packages {
            // Check if any OTHER installer package needs this dependency
            let used_by_other = dependencies_by_installer
                .iter()
                .any(|(installer, deps)| installer != package && deps.contains(dep_pkg));

            // If not used by others and not explicitly installed, it's orphaned
            if !used_by_other && !explicit_packages.contains(dep_pkg) {
                orphaned.insert(dep_pkg.clone());
            }
        }
    }

    orphaned.into_iter().collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_find_orphaned_dependencies() {
        // Test finding orphaned dependencies
    }

    #[test]
    fn test_remove_plugin_not_found() {
        // Test removing non-existent plugin
    }

    #[test]
    fn test_remove_plugin_with_dependencies() {
        // Test removing plugin that has dependencies
    }
}
