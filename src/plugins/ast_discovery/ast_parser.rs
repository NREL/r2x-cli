//! AST parsing using ast-grep
//!
//! Extracts function definitions and plugin list from Python code.

use crate::errors::BridgeError;
use crate::plugins::config::PluginTypeConfig;
use crate::plugins::utils;
use std::path::Path;
use std::process::Command;

/// Extract the register_plugin() function content using ast-grep
pub fn extract_register_plugin_function(plugins_py: &Path) -> Result<String, BridgeError> {
    // Use ast-grep to extract the function
    // Pattern: def register_plugin() -> ...
    let output = Command::new("ast-grep")
        .arg("run")
        .arg("--pattern")
        .arg("def register_plugin()")
        .arg(plugins_py.parent().unwrap_or_else(|| Path::new(".")))
        .output()
        .map_err(|e| {
            BridgeError::Initialization(format!(
                "Failed to run ast-grep: {}. Make sure ast-grep is installed.",
                e
            ))
        })?;

    if !output.status.success() {
        return Err(BridgeError::PluginNotFound(format!(
            "register_plugin() function not found in {}",
            plugins_py.display()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

/// Extract plugin definitions from the plugins array
pub fn extract_plugins_list(func_content: &str) -> Result<Vec<String>, BridgeError> {
    let mut plugins = Vec::new();

    // Find plugins=[ ... ]
    if let Some(plugins_start) = func_content.find("plugins=[") {
        let rest = &func_content[plugins_start + 9..]; // Skip "plugins=["

        // Find matching closing bracket
        if let Some(end_pos) = utils::find_matching_bracket(rest, 0) {
            let plugins_content = &rest[..end_pos];

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

    if plugins.is_empty() {
        return Err(BridgeError::PluginNotFound(
            "No plugin definitions found in plugins array".to_string(),
        ));
    }

    Ok(plugins)
}
