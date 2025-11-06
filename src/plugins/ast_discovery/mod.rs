//! AST-based plugin discovery using ast-grep
//!
//! This module provides static analysis based plugin discovery by:
//! 1. Using ast-grep to parse Python source code without runtime
//! 2. Extracting plugin definitions from the register_plugin() function
//! 3. Resolving imports to build full module paths
//! 4. Serializing to JSON matching Pydantic's model_dump_json() output
//!
//! This approach is ~227x faster than Python-based discovery and requires
//! no Python interpreter startup.

pub mod ast_parser;
pub mod constructor_parser;
pub mod decorator_processor;
pub mod file_finder;
pub mod import_resolver;
pub mod json_builder;
pub mod parameter_extractor;

use crate::errors::BridgeError;
use crate::logger;
use import_resolver::ImportMap;
use serde_json::json;
use std::path::Path;

/// AST-based plugin discovery orchestrator
pub struct AstDiscovery;

impl AstDiscovery {
    /// Discover plugins from a Python package using AST parsing
    ///
    /// # Arguments
    /// * `package_path` - Path to the installed package (e.g., site-packages/r2x_reeds)
    /// * `package_name_full` - Full package name (e.g., "r2x-reeds")
    ///
    /// # Returns
    /// JSON string matching the format that Python's Package.model_dump_json() would produce
    pub fn discover_plugins(
        package_path: &Path,
        package_name_full: &str,
        venv_path: Option<&str>,
        package_version: Option<&str>,
    ) -> Result<String, BridgeError> {
        let start_time = std::time::Instant::now();

        logger::info(&format!("AST discovery started for: {}", package_name_full));

        // Try to find the plugin file using entry_points.txt first
        let plugins_py = if let Some(venv) = venv_path {
            match file_finder::find_plugins_py_via_entry_points(package_name_full, package_version, venv)
            {
                Ok(path) => path,
                Err(_) => file_finder::find_plugins_py(package_path)?,
            }
        } else {
            file_finder::find_plugins_py(package_path)?
        };

        let full_file_content = std::fs::read_to_string(&plugins_py).map_err(|e| {
            BridgeError::PluginNotFound(format!("Failed to read plugins.py: {}", e))
        })?;

        let func_content = ast_parser::extract_register_plugin_function(&plugins_py)?;

        // Build import map from full file content (includes all imports)
        let import_map = import_resolver::build_import_map(&full_file_content)?;

        let package_json = json_builder::extract_package_json(
            &func_content,
            &import_map,
            package_name_full,
            package_path,
            ast_parser::extract_plugins_list,
            |plugin_def, import_map, package_path, infer_type| {
                constructor_parser::parse_plugin_constructor(
                    plugin_def,
                    import_map,
                    package_path,
                    infer_type,
                )
            },
            json_builder::infer_plugin_type,
        )?;

        let elapsed = start_time.elapsed();
        logger::info(&format!(
            "AST discovery completed in {:.2}ms for {}",
            elapsed.as_secs_f64() * 1000.0,
            package_name_full
        ));

        Ok(package_json)
    }
}

// Re-export ImportMap for use by constructor_parser
pub use import_resolver::ImportMap;
