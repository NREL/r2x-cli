//! AST parsing using ast-grep
//!
//! Extracts function definitions and plugin list from Python code.

use crate::errors::BridgeError;
use crate::plugins::config::PluginTypeConfig;
use crate::plugins::utils;
use std::path::Path;

/// Extract the register_plugin() function content from the file
pub fn extract_register_plugin_function(plugins_py: &Path) -> Result<String, BridgeError> {
    // Read the entire file
    let content = std::fs::read_to_string(plugins_py)
        .map_err(|e| BridgeError::PluginNotFound(format!("Failed to read {}: {}", plugins_py.display(), e)))?;

    // Find the register_plugin function
    if let Some(start) = content.find("def register_plugin(") {
        // Find the colon that ends the function definition
        if let Some(colon_pos) = content[start..].find(':') {
            let func_start = start + colon_pos + 1;

            // Extract the function body by finding the indentation level
            let lines: Vec<&str> = content[func_start..].lines().collect();
            let mut func_lines = Vec::new();
            let mut base_indent = None;

            for line in lines {
                // Skip empty lines at the start
                if func_lines.is_empty() && line.trim().is_empty() {
                    continue;
                }

                if !line.trim().is_empty() {
                    // Calculate indentation
                    let indent = line.len() - line.trim_start().len();

                    if base_indent.is_none() {
                        base_indent = Some(indent);
                    }

                    // If we hit a line with less indentation than the base, we're out of the function
                    if let Some(base) = base_indent {
                        if indent < base && !line.trim().is_empty() {
                            break;
                        }
                    }
                }

                func_lines.push(line);
            }

            return Ok(func_lines.join("\n"));
        }
    }

    Err(BridgeError::PluginNotFound(format!(
        "register_plugin() function not found in {}",
        plugins_py.display()
    )))
}

/// Extract plugin definitions from the plugins array
pub fn extract_plugins_list(func_content: &str) -> Result<Vec<String>, BridgeError> {
    let mut plugins = Vec::new();

    // Find plugins=[ ... ] - handle optional whitespace
    let plugins_pattern = "plugins=";
    if let Some(plugins_start) = func_content.find(plugins_pattern) {
        let rest = &func_content[plugins_start + plugins_pattern.len()..];

        // Skip any whitespace to find the opening bracket
        let bracket_start = rest.trim_start();
        if bracket_start.starts_with('[') {
            // Find matching closing bracket
            if let Some(end_pos) = utils::find_matching_bracket(bracket_start, 0) {
                let plugins_content = &bracket_start[1..end_pos]; // Skip opening bracket

                // Search for plugin type constructors
                for keyword in PluginTypeConfig::PLUGIN_CLASSES {
                    let mut search_from = 0;
                    while let Some(pos) = plugins_content[search_from..].find(keyword) {
                        let actual_pos = search_from + pos;

                        // Find the opening parenthesis
                        if let Some(paren_pos) = plugins_content[actual_pos..].find('(') {
                            // Find matching closing parenthesis
                            let paren_start = actual_pos + paren_pos;
                            if let Some(paren_end) = utils::find_matching_paren(&plugins_content, paren_start)
                            {
                                let plugin_def = plugins_content[actual_pos..=paren_end].to_string();
                                if !plugin_def.is_empty() && !plugins.contains(&plugin_def) {
                                    plugins.push(plugin_def);
                                }
                                search_from = actual_pos + 1;
                            } else {
                                search_from = actual_pos + 1;
                            }
                        } else {
                            search_from = actual_pos + keyword.len();
                        }
                    }
                }
            }
        }
    }

    if plugins.is_empty() {
        return Err(BridgeError::PluginNotFound(
            "No plugin definitions found in plugins array".to_string(),
        ));
    }

    Ok(plugins)
}
