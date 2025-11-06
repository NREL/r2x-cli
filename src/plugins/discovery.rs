//! Plugin discovery orchestration
//!
//! Manages the discovery and registration of plugins from packages,
//! handling caching, dependencies, and manifest updates.

use crate::logger;
use crate::plugin_manifest::PluginManifest;
use crate::plugins::{find_package_path, parse_plugin_json, utils, AstDiscovery};

/// Options for plugin discovery and registration
pub struct DiscoveryOptions {
    pub package: String,
    pub package_name_full: String,
    pub dependencies: Vec<String>,
    pub package_version: Option<String>,
    pub no_cache: bool,
}

/// Discover and register plugins from a package and its dependencies
pub fn discover_and_register_entry_points_with_deps(
    _uv_path: &str,
    _python_path: &str,
    opts: DiscoveryOptions,
) -> Result<usize, String> {
    let package = &opts.package;
    let package_name_full = &opts.package_name_full;
    let dependencies = &opts.dependencies;
    let no_cache = opts.no_cache;
    let package_version = opts.package_version.as_deref().unwrap_or("unknown");

    // Get venv path from config for entry_points.txt lookup
    let venv_path = crate::config_manager::Config::load()
        .ok()
        .map(|c| c.get_venv_path());

    // Load manifest
    let mut manifest = match PluginManifest::load() {
        Ok(m) => m,
        Err(e) => {
            logger::warn(&format!("Failed to load manifest: {}", e));
            PluginManifest::default()
        }
    };

    // Check if we already have plugins for this package in the manifest
    let existing_plugins: Vec<String> = manifest
        .plugins
        .iter()
        .filter(|(_, plugin)| plugin.package_name.as_deref() == Some(package_name_full))
        .map(|(key, _)| key.clone())
        .collect();

    // If we have cached plugins for this exact version, reuse them
    let plugin_entries = if !existing_plugins.is_empty() && !no_cache {
        existing_plugins
            .iter()
            .filter_map(|key| manifest.plugins.get(key).map(|p| (key.clone(), p.clone())))
            .collect()
    } else {
        let package_path = find_package_path(package_name_full)
            .map_err(|e| format!("Failed to locate package '{}': {}", package_name_full, e))?;

        let json = AstDiscovery::discover_plugins(
            &package_path,
            package_name_full,
            venv_path.as_deref(),
            Some(package_version),
        )
        .map_err(|e| format!("Failed to discover plugins for '{}': {}", package, e))?;

        parse_plugin_json(&json, package_name_full)
            .map_err(|e| format!("Failed to parse plugin JSON for '{}': {}", package, e))?
    };

    let mut total_plugins = plugin_entries.len();

    if total_plugins == 0 {
        logger::warn(&format!("No plugins found in package '{}'", package));
        return Ok(0);
    }

    logger::info(&format!(
        "Found {} plugin(s) in package '{}'",
        total_plugins, package
    ));

    // Register main package plugins with install_type: "explicit"
    for (key, mut plugin) in plugin_entries {
        plugin.install_type = Some("explicit".to_string());
        plugin.package_name = Some(package_name_full.to_string());
        let _ = manifest.add_plugin(key, plugin);
    }

    // Process r2x plugin dependencies
    if !dependencies.is_empty() {
        let r2x_dependencies: Vec<String> = dependencies
            .iter()
            .filter(|dep| utils::looks_like_r2x_plugin(dep))
            .cloned()
            .collect();

        for dep in r2x_dependencies {
            // Try manifest first for dependency plugins
            let existing_dep_plugins: Vec<String> = manifest
                .plugins
                .iter()
                .filter(|(_, plugin)| {
                    plugin.package_name.as_deref() == Some(&dep)
                })
                .map(|(key, _)| key.clone())
                .collect();
            let dep_plugin_entries = if !existing_dep_plugins.is_empty() && !no_cache {
                existing_dep_plugins
                    .iter()
                    .filter_map(|key| manifest.plugins.get(key).map(|p| (key.clone(), p.clone())))
                    .collect()
            } else {
                match find_package_path(&dep) {
                    Ok(dep_path) => {
                        match AstDiscovery::discover_plugins(
                            &dep_path,
                            &dep,
                            venv_path.as_deref(),
                            None, // Dependencies don't have version info
                        ) {
                            Ok(json) => match parse_plugin_json(&json, &dep) {
                                Ok(entries) => entries,
                                Err(e) => {
                                    logger::warn(&format!(
                                        "Failed to parse plugins from dependency '{}': {}",
                                        &dep, e
                                    ));
                                    Vec::new()
                                }
                            },
                            Err(e) => {
                                logger::warn(&format!(
                                    "Failed to discover plugins from dependency '{}': {}",
                                    &dep, e
                                ));
                                Vec::new()
                            }
                        }
                    }
                    Err(e) => {
                        logger::warn(&format!(
                            "Failed to locate dependency package '{}': {}",
                            &dep, e
                        ));
                        Vec::new()
                    }
                }
            };

            if !dep_plugin_entries.is_empty() {
                // Register dependency plugins with install_type: "dependency"
                for (key, mut plugin) in dep_plugin_entries {
                    plugin.install_type = Some("dependency".to_string());
                    plugin.installed_by = Some(package_name_full.to_string());
                    plugin.package_name = Some(dep.clone());
                    let _ = manifest.add_plugin(key, plugin);
                    total_plugins += 1;
                }
            }
        }
    }

    // Save the updated manifest with all plugins (explicit + dependencies)
    manifest
        .save()
        .map_err(|e| format!("Failed to save manifest: {}", e))?;

    Ok(total_plugins)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_r2x_plugin() {
        assert!(utils::looks_like_r2x_plugin("r2x-reeds"));
        assert!(utils::looks_like_r2x_plugin("r2x-plexos"));
        assert!(!utils::looks_like_r2x_plugin("numpy"));
    }
}
