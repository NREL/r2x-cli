use crate::config_manager::Config;
use crate::logger;
use crate::plugin_cache::{CachedPackage, CachedPlugin, PluginMetadataCache};
use crate::plugin_manifest::PluginManifest;
use crate::python_bridge;
use std::fs;
use std::path::PathBuf;

/// Options for plugin discovery and registration
pub struct DiscoveryOptions {
    pub package: String,
    pub package_name_full: String,
    pub dependencies: Vec<String>,
    pub package_version: Option<String>,
    pub no_cache: bool,
}

pub fn discover_and_register_entry_points_with_deps(
    _uv_path: &str,
    _python_path: &str,
    opts: DiscoveryOptions,
) -> Result<usize, String> {
    let package = &opts.package;
    let package_name_full = &opts.package_name_full;
    let dependencies = &opts.dependencies;
    let no_cache = opts.no_cache;

    logger::debug(&format!("Registering plugins from package: '{}'", package));

    // Extract short name for entry point lookup (e.g., "reeds" from "r2x-reeds")
    let package_short_name = if package_name_full.starts_with("r2x-") {
        package_name_full.trim_start_matches("r2x-")
    } else {
        package_name_full
    };

    logger::debug(&format!(
        "Full package name: {}, short name: {}",
        package_name_full, package_short_name
    ));

    // Quick check: verify entry_points.txt exists before initializing Python bridge
    // This avoids 1.9s+ Python initialization for packages without plugins
    let has_entry_points = check_entry_points_exists(package_name_full);

    if !has_entry_points {
        logger::debug(&format!(
            "No entry_points.txt found for {} - skipping plugin load",
            package_name_full
        ));
        return Ok(0);
    }

    // Load or create manifest early to check cache
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

    // Check metadata cache with version-aware lookup
    let plugin_entries = if !existing_plugins.is_empty() {
        logger::debug(&format!(
            "Found {} plugin(s) in active manifest for '{}', reusing",
            existing_plugins.len(),
            package_name_full
        ));
        // Use existing plugins from manifest instead of reloading
        existing_plugins
            .iter()
            .filter_map(|key| manifest.plugins.get(key).map(|p| (key.clone(), p.clone())))
            .collect()
    } else {
        // Try metadata cache with version lookup
        let cache_version = opts.package_version.as_deref().unwrap_or("unknown");
        let mut metadata_cache = PluginMetadataCache::load().unwrap_or_else(|e| {
            logger::debug(&format!("Failed to load metadata cache: {}", e));
            PluginMetadataCache::default()
        });

        if !no_cache
            && metadata_cache
                .get_package(package_name_full, cache_version)
                .is_some()
        {
            let cached_package = metadata_cache
                .get_package(package_name_full, cache_version)
                .unwrap();
            logger::debug(&format!(
                "✓ Cache hit: Found {} plugin(s) for '{}@{}' in metadata cache",
                cached_package.plugins.len(),
                package_name_full,
                cache_version
            ));

            PluginMetadataCache::extract_plugins(cached_package)
        } else {
            // Cache miss or --no-cache - load from bridge and cache for future use
            if no_cache {
                logger::debug(&format!(
                    "⊘ Cache skipped (--no-cache): Loading '{}@{}' from package metadata",
                    package_name_full, cache_version
                ));
            } else {
                logger::debug(&format!(
                    "✗ Cache miss: Loading '{}@{}' from package metadata",
                    package_name_full, cache_version
                ));
            }
            let bridge = python_bridge::Bridge::get()
                .map_err(|e| format!("Failed to initialize Python bridge: {}", e))?;

            let plugin_entries = bridge
                .build_manifest_from_package(package_short_name, package_name_full)
                .map_err(|e| {
                    format!(
                        "Failed to load plugin package '{}': {}",
                        package_short_name, e
                    )
                })?;

            // Cache the plugins for future installs
            if !plugin_entries.is_empty() {
                let mut cached_package = CachedPackage::new(package_name_full.to_string());

                for (name, plugin) in &plugin_entries {
                    let cached_plugin = CachedPlugin {
                        name: name.clone(),
                        obj: plugin
                            .obj
                            .as_ref()
                            .map(|obj| crate::plugin_cache::CallableMetadata {
                                module: obj.module.clone(),
                                name: obj.name.clone(),
                                callable_type: obj.callable_type.clone(),
                                return_annotation: obj.return_annotation.clone(),
                                parameters: obj
                                    .parameters
                                    .iter()
                                    .map(|(k, v)| {
                                        (
                                            k.clone(),
                                            crate::plugin_cache::ParameterMetadata {
                                                annotation: v.annotation.clone(),
                                                default: v.default.as_ref().and_then(|d| {
                                                    serde_json::from_str::<serde_json::Value>(d)
                                                        .ok()
                                                        .filter(|val| !val.is_null())
                                                }),
                                                is_required: v.is_required,
                                            },
                                        )
                                    })
                                    .collect(),
                            }),
                        plugin_type: plugin.plugin_type.clone().unwrap_or_default(),
                        config: plugin
                            .config
                            .as_ref()
                            .and_then(|c| serde_json::to_value(c).ok()),
                        call_method: plugin.call_method.clone(),
                    };
                    cached_package.add_plugin(cached_plugin);
                }

                if let Err(e) = metadata_cache.set_package(
                    package_name_full.to_string(),
                    cache_version.to_string(),
                    cached_package,
                ) {
                    logger::debug(&format!(
                        "Warning: Failed to cache plugin metadata for '{}@{}': {}",
                        package_name_full, cache_version, e
                    ));
                } else if let Err(e) = metadata_cache.save() {
                    logger::debug(&format!("Warning: Failed to save metadata cache: {}", e));
                } else {
                    logger::debug(&format!(
                        "✓ Cached {} plugin(s) for '{}@{}'",
                        plugin_entries.len(),
                        package_name_full,
                        cache_version
                    ));
                }
            }

            plugin_entries
        }
    };

    let mut total_plugins = plugin_entries.len();

    if total_plugins == 0 {
        logger::warn(&format!(
            "No plugins found in package '{}'",
            package_short_name
        ));
        return Ok(0);
    }

    logger::info(&format!(
        "Found {} plugin(s) in package '{}'",
        total_plugins, package_short_name
    ));

    // Register main package plugins with install_type: "explicit"
    for (key, mut plugin) in plugin_entries {
        plugin.install_type = Some("explicit".to_string());
        plugin.package_name = Some(package_name_full.to_string());
        let _ = manifest.add_plugin(key.clone(), plugin);
        logger::debug(&format!("Registered: {}", key));
    }

    // Register r2x plugin dependencies (dependencies already fetched in parent function)

    if !dependencies.is_empty() {
        let total_deps = dependencies.len();
        logger::debug(&format!(
            "Found {} dependencies for '{}', checking for r2x plugins...",
            total_deps, package
        ));

        // Filter to only check r2x packages (pre-filter for performance)
        let start = std::time::Instant::now();
        let r2x_dependencies: Vec<String> = dependencies
            .iter()
            .filter(|dep| looks_like_r2x_plugin(dep))
            .cloned()
            .collect();
        logger::debug(&format!(
            "Filtering dependencies took: {:?}, result: {} r2x plugins from {} total",
            start.elapsed(),
            r2x_dependencies.len(),
            total_deps
        ));

        if r2x_dependencies.is_empty() {
            logger::debug("No r2x plugin dependencies found");
        } else {
            logger::debug(&format!(
                "Processing {} r2x plugin(s)...",
                r2x_dependencies.len()
            ));

            let mut metadata_cache = PluginMetadataCache::load().unwrap_or_else(|e| {
                logger::debug(&format!(
                    "Failed to load metadata cache for dependencies: {}",
                    e
                ));
                PluginMetadataCache::default()
            });

            for dep in r2x_dependencies {
                let dep_short_name = dep.strip_prefix("r2x-").unwrap_or(&dep);

                let dep_start = std::time::Instant::now();

                // Try metadata cache first (dependencies use "unknown" version), unless --no-cache
                let dep_plugin_entries = if !no_cache
                    && metadata_cache.get_package(&dep, "unknown").is_some()
                {
                    let cached_package = metadata_cache.get_package(&dep, "unknown").unwrap();
                    logger::debug(&format!(
                        "✓ Cache hit: Found {} plugin(s) for '{}' in metadata cache",
                        cached_package.plugins.len(),
                        &dep
                    ));

                    Ok(PluginMetadataCache::extract_plugins(cached_package))
                } else {
                    // Cache miss - load from bridge
                    logger::debug(&format!(
                        "✗ Cache miss: Loading '{}' from package metadata",
                        &dep
                    ));
                    let bridge = python_bridge::Bridge::get()
                        .map_err(|e| format!("Failed to initialize Python bridge: {}", e))?;

                    match bridge.build_manifest_from_package(dep_short_name, &dep) {
                        Ok(dep_plugin_entries) => {
                            // Cache the dependency plugins (unless --no-cache)
                            if !no_cache && !dep_plugin_entries.is_empty() {
                                let mut dep_cached_package = CachedPackage::new(dep.clone());

                                for (name, plugin) in &dep_plugin_entries {
                                    let cached_plugin = CachedPlugin {
                                        name: name.clone(),
                                        obj: plugin.obj.as_ref().map(|obj| crate::plugin_cache::CallableMetadata {
                                            module: obj.module.clone(),
                                            name: obj.name.clone(),
                                            callable_type: obj.callable_type.clone(),
                                            return_annotation: obj.return_annotation.clone(),
                                            parameters: obj.parameters.iter().map(|(k, v)| {
                                                (k.clone(), crate::plugin_cache::ParameterMetadata {
                                                    annotation: v.annotation.clone(),
                                                    default: v.default.as_ref().and_then(|d| {
                                                        serde_json::from_str::<serde_json::Value>(d).ok().filter(|val| !val.is_null())
                                                    }),
                                                    is_required: v.is_required,
                                                })
                                            }).collect(),
                                        }),
                                        plugin_type: plugin.plugin_type.clone().unwrap_or_default(),
                                        config: plugin.config.as_ref().and_then(|c| serde_json::to_value(c).ok()),
                                        call_method: plugin.call_method.clone(),
                                    };
                                    dep_cached_package.add_plugin(cached_plugin);
                                }

                                if let Err(e) = metadata_cache.set_package(
                                    dep.clone(),
                                    "unknown".to_string(),
                                    dep_cached_package,
                                ) {
                                    logger::debug(&format!(
                                        "Warning: Failed to cache dependency plugins for '{}': {}",
                                        dep, e
                                    ));
                                } else if let Err(e) = metadata_cache.save() {
                                    logger::debug(&format!(
                                        "Warning: Failed to save metadata cache after dependency load: {}",
                                        e
                                    ));
                                } else {
                                    logger::debug(&format!(
                                        "✓ Cached {} plugin(s) for '{}'",
                                        dep_plugin_entries.len(),
                                        dep
                                    ));
                                }
                            }
                            Ok(dep_plugin_entries)
                        }
                        Err(e) => Err(e),
                    }
                };

                match dep_plugin_entries {
                    Ok(dep_plugin_entries) => {
                        logger::debug(&format!("Loading {} took: {:?}", dep, dep_start.elapsed()));
                        if !dep_plugin_entries.is_empty() {
                            logger::debug(&format!(
                                "Found {} r2x plugin(s) in dependency '{}'",
                                dep_plugin_entries.len(),
                                dep
                            ));

                            for (key, mut plugin) in dep_plugin_entries {
                                plugin.install_type = Some("dependency".to_string());
                                plugin.installed_by = Some(package_name_full.to_string());
                                plugin.package_name = Some(dep.clone());
                                let _ = manifest.add_plugin(key.clone(), plugin);
                                total_plugins += 1;
                                logger::debug(&format!("Registered (dependency): {}", key));
                            }
                        }
                    }
                    Err(e) => {
                        logger::debug(&format!(
                            "Dependency '{}' failed to load as r2x plugin (took {:?}): {}",
                            dep,
                            dep_start.elapsed(),
                            e
                        ));
                    }
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

/// Quick check: does the package have an entry_points.txt file?
/// This is a fast file system check to avoid Python bridge initialization
fn check_entry_points_exists(package_name_full: &str) -> bool {
    // Get venv path
    let config = match Config::load() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let venv_path = PathBuf::from(config.get_venv_path());

    // Find site-packages directory
    // On Windows: venv\Lib\site-packages
    // On Unix: venv/lib/python3.x/site-packages
    let site_packages_path = if cfg!(windows) {
        venv_path.join("Lib").join("site-packages")
    } else {
        let site_packages = venv_path.join("lib");
        let entries = match fs::read_dir(&site_packages) {
            Ok(e) => e,
            Err(_) => return false,
        };

        let python_version_dir = match entries
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with("python"))
        {
            Some(d) => d,
            None => return false,
        };

        python_version_dir.path().join("site-packages")
    };

    // Convert package name format: "r2x-reeds" -> "r2x_reeds" for dist-info lookup
    let normalized_name = package_name_full.replace('-', "_");

    // Find dist-info directory matching the package name
    // Match exactly: package_name + "-" to avoid matching r2x_sienna when looking for r2x_sienna_to_plexos
    if let Ok(entries) = fs::read_dir(&site_packages_path) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let expected_prefix = format!("{}-", normalized_name);
            if file_name.starts_with(&expected_prefix) && file_name.ends_with(".dist-info") {
                let entry_points_path = entry.path().join("entry_points.txt");
                return entry_points_path.exists();
            }
        }
    }

    false
}

/// Check if a package name looks like it could be an r2x plugin package.
/// This is a fast pre-filter before attempting expensive Python bridge calls.
fn looks_like_r2x_plugin(package_name: &str) -> bool {
    // Only check packages that start with "r2x-" but skip infrastructure packages
    // that are never plugins
    if !package_name.starts_with("r2x-") {
        return false;
    }

    // Skip known infrastructure/dependency packages
    match package_name {
        "r2x-core" => false, // Core infrastructure, not a plugin
        "chronify" => false, // Time series dependency
        "infrasys" => false, // Infrastructure systems dependency
        "plexosdb" => false, // PLEXOS database dependency
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_entry_points_exists() {
        // Test entry points file detection
    }

    #[test]
    fn test_looks_like_r2x_plugin() {
        assert!(looks_like_r2x_plugin("r2x-reeds"));
        assert!(looks_like_r2x_plugin("r2x-plexos"));
        assert!(!looks_like_r2x_plugin("r2x-core"));
        assert!(!looks_like_r2x_plugin("chronify"));
        assert!(!looks_like_r2x_plugin("requests"));
    }

    #[test]
    fn test_discover_and_register() {
        // Test plugin discovery and registration
    }
}
