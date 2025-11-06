//! JSON building and type inference module
//!
//! Provides functions for building plugin JSON representations and inferring
//! type information from plugin definitions and callable names.

use crate::errors::BridgeError;
use crate::logger;
use crate::plugins::ast_discovery::ImportMap;
use serde_json::{Value, json};
use std::path::Path;

/// Extract Package JSON by parsing plugins from the register_plugin() function
///
/// Orchestrates the extraction of plugin definitions from a Python plugins file
/// and converts them into a JSON structure matching the Pydantic Package model.
///
/// # Arguments
/// * `func_content` - Content of the register_plugin() function
/// * `import_map` - Mapping of imported symbols to their modules
/// * `package_name_full` - Full package name (e.g., "r2x-reeds")
/// * `package_path` - Path to the package directory
///
/// # Returns
/// JSON string with format: {"name": "package-name", "plugins": [...], "metadata": {}}
pub fn extract_package_json(
    func_content: &str,
    import_map: &ImportMap,
    package_name_full: &str,
    package_path: &Path,
    extract_plugins_list: fn(&str) -> Result<Vec<String>, BridgeError>,
    parse_plugin_constructor: fn(
        &str,
        &ImportMap,
        &Path,
        fn(&str) -> &'static str,
    ) -> Result<Value, BridgeError>,
    infer_plugin_type: fn(&str) -> &'static str,
) -> Result<String, BridgeError> {
    let plugins_list = extract_plugins_list(func_content)?;

    let mut plugins = Vec::new();
    for (idx, plugin_def) in plugins_list.iter().enumerate() {
        match parse_plugin_constructor(plugin_def, import_map, package_path, infer_plugin_type) {
            Ok(plugin_json) => {
                if let Some(name) = plugin_json.get("name").and_then(|v| v.as_str()) {
                    logger::debug(&format!(
                        "  [{}/{}] Parsed: {} ({})",
                        idx + 1,
                        plugins_list.len(),
                        name,
                        plugin_json
                            .get("plugin_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    ));
                }
                plugins.push(plugin_json);
            }
            Err(e) => {
                logger::warn(&format!("Failed to parse plugin definition: {}", e));
            }
        }
    }

    logger::debug(&format!(
        "Successfully parsed {} plugins via AST",
        plugins.len()
    ));

    let package_json = json!({
        "name": package_name_full,
        "plugins": plugins,
        "metadata": {}
    });

    Ok(package_json.to_string())
}

/// Infer plugin type discriminator from constructor name
///
/// Maps Python constructor class names to their plugin type discriminators.
/// These are used in Pydantic's discriminated unions to determine the correct
/// plugin subclass during deserialization.
///
/// # Arguments
/// * `constructor` - The plugin class name (e.g., "ParserPlugin", "UpgraderPlugin")
///
/// # Returns
/// A static string slice representing the plugin type: "parser", "exporter", "upgrader", or "function"
pub fn infer_plugin_type(constructor: &str) -> &'static str {
    match constructor {
        "ParserPlugin" => "parser",
        "ExporterPlugin" => "exporter",
        "UpgraderPlugin" => "upgrader",
        "BasePlugin" => "function",
        _ => "function",
    }
}

/// Infer callable type from name heuristic
///
/// Determines whether a symbol name refers to a class or function based on naming conventions.
/// This is a best-effort heuristic: classes start with uppercase letters, functions are lowercase.
///
/// # Arguments
/// * `name` - The symbol name to analyze
///
/// # Returns
/// "class" if name starts with uppercase, "function" otherwise
pub fn infer_callable_type_from_name(name: &str) -> &'static str {
    if name.chars().next().map_or(false, |c| c.is_uppercase()) {
        "class"
    } else {
        "function"
    }
}
