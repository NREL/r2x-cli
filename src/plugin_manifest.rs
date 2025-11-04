use crate::errors::ManifestError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Plugin metadata from package serialization
///
/// This structure breaks down plugin metadata into concise, queryable fields
/// while preserving full JSON metadata for invocation.
///
/// Note: The plugin name is stored as the HashMap key in PluginManifest,
/// not duplicated here to avoid inconsistency
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Plugin {
    /// Package name this plugin belongs to (e.g., "r2x-reeds")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,

    /// Plugin type: "parser", "exporter", "sysmod", "upgrader"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_type: Option<String>,

    /// Brief description of what the plugin does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Plugin documentation string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,

    /// IO type: "stdin", "stdout", "both", or null
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_type: Option<String>,

    /// Method to call on the callable object (e.g., "build_system", "export")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_method: Option<String>,

    /// Whether this plugin requires a data store
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_store: Option<bool>,

    /// Callable object metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obj: Option<CallableMetadata>,

    /// Configuration schema metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ConfigMetadata>,

    /// Upgrader-specific metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrader: Option<UpgraderMetadata>,

    /// Installation type: "explicit" (user-installed) or "dependency" (transitively installed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_type: Option<String>,

    /// Name of the package that caused this plugin to be installed as a dependency
    /// Only set when install_type is "dependency"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_by: Option<String>,
}

/// Callable object metadata (parsed from obj JSON)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CallableMetadata {
    /// Python module path (e.g., "r2x_reeds.parser")
    pub module: String,

    /// Callable name (e.g., "ReEDSParser")
    pub name: String,

    /// Callable type: "class" or "function"
    #[serde(rename = "type")]
    pub callable_type: String,

    /// Return annotation (e.g., "None", "System")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_annotation: Option<String>,

    /// Parameters as a map of parameter name to metadata
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub parameters: HashMap<String, ParameterMetadata>,
}

/// Configuration schema metadata (parsed from config JSON)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigMetadata {
    /// Config module path (e.g., "r2x_reeds.config")
    pub module: String,

    /// Config class name (e.g., "ReEDSConfig")
    pub name: String,

    /// Return annotation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_annotation: Option<String>,

    /// Config parameters as a map of parameter name to metadata
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub parameters: HashMap<String, ParameterMetadata>,
}

/// Parameter metadata for a callable or config
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParameterMetadata {
    /// Type annotation (e.g., "str | None", "int", "System")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<String>,

    /// Default value as JSON string (e.g., "null", "true", "5", "\"base\"")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Whether this parameter is required
    pub is_required: bool,
}

/// Upgrader-specific metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpgraderMetadata {
    /// Version strategy as JSON (kept as JSON for now due to complexity)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_strategy_json: Option<String>,

    /// Version reader as JSON (kept as JSON for now due to complexity)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_reader_json: Option<String>,

    /// Upgrade steps as JSON (kept as JSON for now due to complexity)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrade_steps_json: Option<String>,
}

/// NOTE: package-level registry removed — manifest now stores only `plugins`.
/// The old `PluginPackage` type has been removed to simplify the registry.
/// Plugin registry manifest
///
/// **WARNING: This file is auto-managed by the r2x CLI.**
/// **Do not edit manually - use `r2x plugins` commands instead.**
///
/// The manifest tracks installed plugins and their metadata for dynamic CLI generation.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PluginManifest {
    #[serde(default)]
    pub plugins: HashMap<String, Plugin>,
}

impl Plugin {
    /// Validate plugin metadata
    fn validate(&self, _name: &str) -> Result<(), ManifestError> {
        // Description is optional now, so no validation needed
        // All other fields are optional and derived from package metadata
        Ok(())
    }
}

impl PluginManifest {
    /// Get the path to the manifest file
    pub fn path() -> PathBuf {
        // On Unix/macOS: use ~/.cache/r2x/manifest.toml
        // On Windows: use AppData/Local/r2x/manifest.toml
        #[cfg(not(target_os = "windows"))]
        {
            dirs::home_dir()
                .expect("Could not determine home directory")
                .join(".cache")
                .join("r2x")
                .join("manifest.toml")
        }

        #[cfg(target_os = "windows")]
        {
            dirs::cache_dir()
                .expect("Could not determine cache directory")
                .join("r2x")
                .join("manifest.toml")
        }
    }

    /// Load manifest from disk, returning empty manifest if file doesn't exist
    pub fn load() -> Result<Self, ManifestError> {
        let path = Self::path();
        if !path.exists() {
            return Ok(PluginManifest::default());
        }

        let content = std::fs::read_to_string(&path)?;
        let manifest: PluginManifest = toml::from_str(&content)?;

        // Validate all plugins on load (fail fast)
        for (name, plugin) in &manifest.plugins {
            plugin.validate(name)?;
        }

        // No package-level validation — manifest contains only `plugins` now.

        Ok(manifest)
    }

    /// Save manifest to disk with validation
    pub fn save(&self) -> Result<(), ManifestError> {
        // Validate before saving (fail fast)
        for (name, plugin) in &self.plugins {
            plugin.validate(name)?;
        }

        // No package-level validation required.

        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Add or update a plugin in the manifest
    pub fn add_plugin(&mut self, name: String, plugin: Plugin) -> Result<(), ManifestError> {
        plugin.validate(&name)?;
        self.plugins.insert(name, plugin);
        Ok(())
    }

    /// Remove a plugin from the manifest
    pub fn remove_plugin(&mut self, name: &str) -> bool {
        self.plugins.remove(name).is_some()
    }

    /// Remove all plugins belonging to a package from the manifest
    /// Returns the number of plugins removed
    pub fn remove_plugins_by_package(&mut self, package_name: &str) -> usize {
        let to_remove: Vec<String> = self
            .plugins
            .iter()
            .filter(|(_, plugin)| {
                plugin
                    .package_name
                    .as_ref()
                    .map(|pkg| pkg == package_name)
                    .unwrap_or(false)
            })
            .map(|(name, _)| name.clone())
            .collect();

        let count = to_remove.len();
        for name in to_remove {
            self.plugins.remove(&name);
        }
        count
    }

    /// Get a plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<&Plugin> {
        self.plugins.get(name)
    }

    /// List all plugins with their names
    pub fn list_plugins(&self) -> Vec<(&str, &Plugin)> {
        self.plugins.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }

    /// Check if manifest has no plugins
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Check if a specific plugin exists
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }
}

impl PluginManifest {
    /// Serialize this PluginManifest to a JSON string
    /// Return a pretty JSON string representation of the manifest. This is
    /// intended for fast rendering by CLI/UI consumers that prefer JSON.
    /// On serialization errors this returns an empty JSON object ("{}").
    pub fn to_json_string(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Save the manifest as JSON to the given path. Returns filesystem IO
    /// errors via `ManifestError::Io`.
    pub fn save_json<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ManifestError> {
        let s = serde_json::to_string_pretty(&self).unwrap_or_else(|_| "{}".to_string());
        std::fs::write(path, s).map_err(ManifestError::Io)?;
        Ok(())
    }

    /// Return the manifest JSON for CLI/UI consumers without initializing Python.
    /// Loads the manifest from disk and returns pretty JSON. On error returns "{}".
    pub fn get_manifest_json() -> String {
        match PluginManifest::load() {
            Ok(manifest) => {
                serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".to_string())
            }
            Err(_) => "{}".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_new() {
        let manifest = PluginManifest::default();
        assert!(manifest.is_empty());
    }

    #[test]
    fn test_add_plugin() {
        let mut manifest = PluginManifest::default();
        let plugin = Plugin {
            package_name: None,
            plugin_type: None,
            description: None,
            doc: None,
            io_type: None,
            call_method: None,
            requires_store: None,
            obj: None,
            config: None,
            upgrader: None,
            install_type: None,
            installed_by: None,
        };
        manifest
            .add_plugin("test-plugin".to_string(), plugin)
            .unwrap();
        assert!(!manifest.is_empty());
        assert!(manifest.has_plugin("test-plugin"));
    }

    #[test]
    fn test_remove_plugin() {
        let mut manifest = PluginManifest::default();
        let plugin = Plugin {
            package_name: None,
            plugin_type: None,
            description: None,
            doc: None,
            io_type: None,
            call_method: None,
            requires_store: None,
            obj: None,
            config: None,
            upgrader: None,
            install_type: None,
            installed_by: None,
        };
        manifest
            .add_plugin("test-plugin".to_string(), plugin)
            .unwrap();
        assert!(manifest.remove_plugin("test-plugin"));
        assert!(manifest.is_empty());
        assert!(!manifest.remove_plugin("non-existent"));
    }
}
