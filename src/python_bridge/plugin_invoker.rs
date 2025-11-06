//! Plugin invocation and execution
//!
//! This module handles executing plugins with configuration and input data,
//! including argument building and special handling for different plugin types.

use crate::errors::BridgeError;
use crate::logger;
use crate::plugin_manifest::Plugin;
use pyo3::prelude::*;
use pyo3::types::PyModule;

impl super::Bridge {
    /// Invoke a plugin with configuration
    ///
    /// Calls a plugin callable with the provided configuration and optional stdin data.
    ///
    /// # Arguments
    /// * `target` - Entry point in format "module.path:callable_name"
    /// * `config_json` - Configuration as JSON string
    /// * `stdin_json` - Optional stdin data as JSON string
    /// * `plugin_metadata` - Optional plugin metadata for smart argument handling
    ///
    /// # Returns
    /// Plugin output as JSON string
    ///
    /// # Example
    /// ```ignore
    /// let output = bridge.invoke_plugin(
    ///     "r2x_reeds.plugins:parse_reeds",
    ///     r#"{"solve_year": 2030}"#,
    ///     None,
    ///     None
    /// )?;
    /// ```
    pub fn invoke_plugin(
        &self,
        target: &str,
        config_json: &str,
        stdin_json: Option<&str>,
        plugin_metadata: Option<&Plugin>,
    ) -> Result<String, BridgeError> {
        // Check if this is an upgrader plugin - route to dedicated handler
        if let Some(plugin) = plugin_metadata {
            if plugin.upgrader.is_some() {
                logger::debug("Routing to upgrader plugin handler");
                return self.invoke_upgrader_plugin(target, config_json, plugin_metadata);
            }
        }

        // Fall through to regular plugin invocation
        self.invoke_plugin_regular(target, config_json, stdin_json, plugin_metadata)
    }

    /// Invoke a regular (non-upgrader) plugin with configuration
    fn invoke_plugin_regular(
        &self,
        target: &str,
        config_json: &str,
        stdin_json: Option<&str>,
        plugin_metadata: Option<&Plugin>,
    ) -> Result<String, BridgeError> {
        pyo3::Python::with_gil(|py| {
            // Parse target (module:callable or module:Class.method)
            logger::debug(&format!("Parsing target: {}", target));
            let parts: Vec<&str> = target.split(':').collect();
            if parts.len() != 2 {
                return Err(BridgeError::InvalidEntryPoint(target.to_string()));
            }
            let module_path = parts[0];
            let callable_path = parts[1];
            logger::debug(&format!(
                "Module: {}, Callable: {}",
                module_path, callable_path
            ));

            // Import the module and JSON
            logger::debug(&format!("Importing module: {}", module_path));
            let module = PyModule::import(py, module_path)
                .map_err(|e| BridgeError::Import(module_path.to_string(), format!("{}", e)))?;
            logger::debug("Module imported successfully");
            let json_module = PyModule::import(py, "json")
                .map_err(|e| BridgeError::Import("json".to_string(), format!("{}", e)))?;
            let loads = json_module.getattr("loads")?;

            // Parse config JSON and stdin JSON
            logger::debug("Parsing config JSON");
            let config_dict = loads
                .call1((config_json,))?
                .downcast::<pyo3::types::PyDict>()
                .map_err(|e| BridgeError::Python(format!("Config must be a JSON object: {}", e)))?
                .clone();
            logger::debug("Config parsed successfully");

            let stdin_obj = if let Some(stdin) = stdin_json {
                logger::debug("Parsing stdin JSON");
                Some(loads.call1((stdin,))?)
            } else {
                None
            };

            // Build kwargs by processing each parameter based on metadata
            logger::debug("Building kwargs for plugin invocation");
            let kwargs =
                self.build_kwargs(py, &config_dict, stdin_obj.as_ref(), plugin_metadata)?;
            logger::debug("Kwargs built successfully");

            // Invoke the plugin
            logger::debug("Starting plugin invocation");
            let result_py = if callable_path.contains('.') {
                // Class.method pattern
                let parts: Vec<&str> = callable_path.split('.').collect();
                if parts.len() != 2 {
                    return Err(BridgeError::InvalidEntryPoint(target.to_string()));
                }
                let (class_name, method_name) = (parts[0], parts[1]);
                logger::debug(&format!("Class pattern: {}.{}", class_name, method_name));

                logger::debug(&format!("Getting class: {}", class_name));
                let class = module.getattr(class_name).map_err(|e| {
                    BridgeError::Python(format!("Failed to get class '{}': {}", class_name, e))
                })?;

                logger::debug(&format!("Instantiating class: {}", class_name));
                let instance = class.call((), Some(&kwargs)).map_err(|e| {
                    let error_msg = format!("{}", e);
                    let enhanced_msg = if error_msg.contains("missing") && error_msg.contains("required positional argument") {
                        format!(
                            "Failed to instantiate '{}': {}\n\nHint: This error may occur when plugin metadata is incomplete. Try running:\n  r2x sync\n\nThis will refresh the plugin metadata cache.",
                            class_name, error_msg
                        )
                    } else {
                        format!("Failed to instantiate '{}': {}", class_name, error_msg)
                    };
                    BridgeError::Python(enhanced_msg)
                })?;
                logger::debug("Class instantiated successfully");

                logger::debug(&format!("Getting method: {}", method_name));
                let method = instance.getattr(method_name).map_err(|e| {
                    BridgeError::Python(format!("Failed to get method '{}': {}", method_name, e))
                })?;

                // Call method (stdin passed to method for sysmods, not constructor)
                logger::debug(&format!("Calling method: {}", method_name));
                if let Some(stdin) = stdin_obj {
                    method.call1((stdin,)).map_err(|e| {
                        BridgeError::Python(format!(
                            "Method '{}.{}' failed: {}",
                            class_name, method_name, e
                        ))
                    })?
                } else {
                    method.call0().map_err(|e| {
                        BridgeError::Python(format!(
                            "Method '{}.{}' failed: {}",
                            class_name, method_name, e
                        ))
                    })?
                }
            } else {
                // Function pattern
                logger::debug(&format!("Function pattern: {}", callable_path));
                logger::debug(&format!("Getting function: {}", callable_path));
                let func = module.getattr(callable_path).map_err(|e| {
                    BridgeError::Python(format!(
                        "Failed to get function '{}': {}",
                        callable_path, e
                    ))
                })?;

                // For functions, pass kwargs (system comes from stdin)
                logger::debug("Calling function with kwargs");
                logger::step(&format!("Function kwargs before system: {:?}", kwargs));
                if let Some(stdin) = stdin_obj {
                    logger::step("Function has stdin - deserializing to System object");
                    // Deserialize stdin JSON to System object for sysmods
                    logger::debug("Deserializing stdin JSON to System object");

                    // Convert stdin dict back to JSON bytes
                    let dumps = json_module.getattr("dumps")?;
                    let json_str = dumps.call1((stdin,))?.extract::<String>()?;
                    let json_bytes = json_str.as_bytes();

                    // Import System and call from_json
                    let system_module = PyModule::import(py, "r2x_core.system")?;
                    let system_class = system_module.getattr("System")?;
                    let from_json = system_class.getattr("from_json")?;
                    let system_obj = from_json.call1((json_bytes,))?;

                    logger::debug("System object deserialized successfully");
                    kwargs.set_item("system", system_obj)?;
                } else {
                    logger::debug("Function has no stdin");
                }

                logger::step(&format!("Final function kwargs: {:?}", kwargs));
                func.call((), Some(&kwargs)).map_err(|e| {
                    BridgeError::Python(format!("Function '{}' failed: {}", callable_path, e))
                })?
            };
            logger::debug("Plugin execution completed");

            // Serialize result to JSON
            logger::debug("Serializing result to JSON");

            // Check if result has a to_json() method (e.g., System objects)
            if result_py.hasattr("to_json")? {
                // Call to_json() with no arguments (fname=None) which returns bytes
                let to_json_result = result_py.call_method0("to_json")?;

                // Check if the result is bytes (Python bytes object)
                if let Ok(json_bytes) = to_json_result.extract::<Vec<u8>>() {
                    let json_str = String::from_utf8(json_bytes).map_err(|e| {
                        BridgeError::Python(format!("Invalid UTF-8 in JSON output: {}", e))
                    })?;
                    Ok(json_str)
                } else {
                    // to_json() returned None or something else - fall back to json.dumps
                    let dumps = json_module.getattr("dumps")?;
                    let json_str = dumps.call1((result_py,))?.extract::<String>()?;
                    Ok(json_str)
                }
            } else {
                // Use standard JSON serialization
                let dumps = json_module.getattr("dumps")?;
                let json_str = dumps.call1((result_py,))?.extract::<String>()?;
                Ok(json_str)
            }
        })
    }

    /// Build kwargs for plugin invocation based on parameter metadata
    fn build_kwargs<'py>(
        &self,
        py: Python<'py>,
        config_dict: &pyo3::Bound<'py, pyo3::types::PyDict>,
        stdin_obj: Option<&pyo3::Bound<'py, PyAny>>,
        plugin_metadata: Option<&Plugin>,
    ) -> Result<pyo3::Bound<'py, pyo3::types::PyDict>, BridgeError> {
        let kwargs = pyo3::types::PyDict::new(py);

        // Get plugin obj metadata
        let obj = match plugin_metadata.and_then(|m| m.obj.as_ref()) {
            Some(o) => o,
            None => {
                // No metadata, pass config as-is
                for (k, v) in config_dict {
                    kwargs.set_item(k, v)?;
                }
                return Ok(kwargs);
            }
        };

        // Check if we need to instantiate a config class
        let mut needs_config_class = false;
        let mut config_param_name = String::new();
        for (param_name, param_meta) in &obj.parameters {
            let annotation = param_meta.annotation.as_deref().unwrap_or("");
            if param_name == "config" || annotation.contains("Config") {
                needs_config_class = true;
                config_param_name = param_name.clone();
                break;
            }
        }

        // First pass: collect parameters for config class and handle special parameters
        let mut config_instance: Option<pyo3::Bound<'py, PyAny>> = None;
        if needs_config_class && plugin_metadata.is_some() {
            // Check if config is already provided as a nested dict (pipeline mode)
            // or if we need to collect it from top-level params (direct plugin mode)
            let config_params = if let Ok(Some(existing_config)) = config_dict.get_item("config") {
                // Pipeline mode: config is already nested
                if let Ok(config_dict_value) = existing_config.downcast::<pyo3::types::PyDict>() {
                    config_dict_value.clone()
                } else {
                    // config exists but isn't a dict - create empty and let Python validate
                    pyo3::types::PyDict::new(py)
                }
            } else {
                // Direct plugin mode: collect all config dict items that aren't special parameters
                let params = pyo3::types::PyDict::new(py);
                for (key, value) in config_dict.iter() {
                    let key_str = key.extract::<String>()?;
                    // Skip special parameters that aren't config parameters
                    if key_str != "data_store" && key_str != "store_path" {
                        params.set_item(key, value)?;
                    }
                }
                params
            };

            // Instantiate config class with collected parameters
            config_instance =
                Some(self.instantiate_config_class(py, &config_params, plugin_metadata)?);
            kwargs.set_item(&config_param_name, config_instance.as_ref().unwrap())?;
        }

        // Second pass: process special parameters
        for (param_name, param_meta) in &obj.parameters {
            let annotation = param_meta.annotation.as_deref().unwrap_or("");

            // Skip config - already processed
            if param_name == "config" || annotation.contains("Config") {
                continue;
            }

            // Handle data_store parameter - can come from "data_store" or "store_path"
            // Handle data_store parameter - can come from "store_path" first (user-friendly name), then "data_store"
            if param_name == "data_store" || annotation.contains("DataStore") {
                logger::step(&format!("Processing data_store parameter: {}", param_name));
                // Try to get from "store_path" first (user-friendly name), then "data_store"
                let value = config_dict
                    .get_item("store_path")?
                    .or_else(|| config_dict.get_item(param_name).ok().flatten());

                if let Some(value) = value {
                    logger::step(&format!("Found data_store value, instantiating DataStore"));
                    let store_instance =
                        self.instantiate_data_store(py, &value, config_instance.as_ref())?;
                    kwargs.set_item(param_name, store_instance)?;
                } else if param_meta.is_required {
                    return Err(BridgeError::Python(format!(
                        "Required parameter '{}' not provided (you can use 'store_path' or 'data_store')",
                        param_name
                    )));
                }
                continue;
            }

            // Handle system parameter from stdin
            if param_name == "system" && stdin_obj.is_some() {
                // System parameter comes from stdin (for sysmods)
                // Don't add to kwargs - it's passed to the method call, not constructor
                continue;
            }

            // Handle other parameters directly from config_dict
            if let Some(value) = config_dict.get_item(param_name)? {
                logger::step(&format!(
                    "Adding parameter '{}' to kwargs directly",
                    param_name
                ));
                kwargs.set_item(param_name, value)?;
            } else if !param_meta.is_required {
                // Optional parameter not provided - skip it
                continue;
            }
            // Required parameter not provided - let Python raise the error
        }

        logger::step(&format!("Final built kwargs (keys): {:?}", kwargs.keys()));
        Ok(kwargs)
    }

    /// Instantiate config class from dict
    fn instantiate_config_class<'py>(
        &self,
        py: Python<'py>,
        config_params: &pyo3::Bound<'py, pyo3::types::PyDict>,
        plugin_metadata: Option<&Plugin>,
    ) -> Result<pyo3::Bound<'py, PyAny>, BridgeError> {
        // Get config metadata
        let config_meta = match plugin_metadata.and_then(|m| m.config.as_ref()) {
            Some(c) => c,
            None => {
                return Err(BridgeError::Python(
                    "No config metadata available".to_string(),
                ))
            }
        };

        // Import config module and get class
        let config_module = PyModule::import(py, config_meta.module.as_str())
            .map_err(|e| BridgeError::Import(config_meta.module.clone(), format!("{}", e)))?;

        let config_class = config_module
            .getattr(config_meta.name.as_str())
            .map_err(|e| {
                BridgeError::Python(format!(
                    "Failed to get config class '{}': {}",
                    config_meta.name, e
                ))
            })?;

        // Instantiate with dict as kwargs
        config_class.call((), Some(config_params)).map_err(|e| {
            BridgeError::Python(format!(
                "Failed to instantiate config class '{}.{}': {}",
                config_meta.module, config_meta.name, e
            ))
        })
    }

    /// Instantiate DataStore from path string or dict, optionally using config for file mappings
    fn instantiate_data_store<'py>(
        &self,
        py: Python<'py>,
        value: &pyo3::Bound<'py, PyAny>,
        config_instance: Option<&pyo3::Bound<'py, PyAny>>,
    ) -> Result<pyo3::Bound<'py, PyAny>, BridgeError> {
        // Import DataStore from r2x_core.store
        let store_module = PyModule::import(py, "r2x_core.store")
            .map_err(|e| BridgeError::Import("r2x_core.store".to_string(), format!("{}", e)))?;

        let datastore_class = store_module.getattr("DataStore")?;

        // Check if value is a string path (most common case)
        if let Ok(path_str) = value.extract::<String>() {
            // If we have a config instance, use from_plugin_config to load file mappings
            if let Some(config) = config_instance {
                let from_plugin_config = datastore_class.getattr("from_plugin_config")?;
                from_plugin_config.call1((config, path_str)).map_err(|e| {
                    BridgeError::Python(format!(
                        "Failed to create DataStore from plugin config: {}",
                        e
                    ))
                })
            } else {
                // No config - create basic DataStore with just path
                datastore_class.call1((path_str,)).map_err(|e| {
                    BridgeError::Python(format!("Failed to instantiate DataStore: {}", e))
                })
            }
        } else if let Ok(store_dict) = value.downcast::<pyo3::types::PyDict>() {
            // Handle dict format for backward compatibility
            let path = store_dict.get_item("path")?.ok_or_else(|| {
                BridgeError::Python("DataStore dict missing 'path' field".to_string())
            })?;

            // If we have a config instance, use from_plugin_config to load file mappings
            if let Some(config) = config_instance {
                let from_plugin_config = datastore_class.getattr("from_plugin_config")?;
                from_plugin_config.call1((config, path)).map_err(|e| {
                    BridgeError::Python(format!(
                        "Failed to create DataStore from plugin config: {}",
                        e
                    ))
                })
            } else {
                // No config - create basic DataStore with just path
                datastore_class.call1((path,)).map_err(|e| {
                    BridgeError::Python(format!("Failed to instantiate DataStore: {}", e))
                })
            }
        } else {
            Err(BridgeError::Python(
                "DataStore value must be a string path or dict with 'path' field".to_string(),
            ))
        }
    }

    /// Invoke an upgrader plugin with path-based data source
    ///
    /// Upgrader plugins are classes that execute ordered upgrade steps.
    /// This instantiates the upgrader class and executes its steps:
    /// - FILE steps: Modify files in-place, no output
    /// - SYSTEM steps: Read JSON, transform it, chain outputs
    ///
    /// # Arguments
    /// * `target` - Entry point in format "module.path:UpgraderClassName"
    /// * `config_json` - Configuration as JSON string (must contain "path" key)
    ///
    /// # Returns
    /// Final upgraded JSON string if SYSTEM steps were executed, empty string otherwise
    fn invoke_upgrader_plugin(
        &self,
        target: &str,
        config_json: &str,
        plugin_metadata: Option<&Plugin>,
    ) -> Result<String, BridgeError> {
        pyo3::Python::with_gil(|py| {
            // Parse config and extract path
            let json_module = PyModule::import(py, "json")
                .map_err(|e| BridgeError::Import("json".to_string(), format!("{}", e)))?;
            let loads = json_module.getattr("loads")?;
            let dumps = json_module.getattr("dumps")?;

            let config_dict = loads
                .call1((config_json,))?
                .cast::<pyo3::types::PyDict>()
                .map_err(|e| BridgeError::Python(format!("Config must be a JSON object: {}", e)))?
                .clone();

            let path_str = config_dict
                .get_item("path")?
                .ok_or_else(|| {
                    BridgeError::Python("Upgrader plugin requires 'path' in config".to_string())
                })?
                .extract::<String>()
                .map_err(|e| BridgeError::Python(format!("'path' must be a string: {}", e)))?;

            logger::debug(&format!("Upgrader path: {}", path_str));

            // Parse target to get module and class name
            let parts: Vec<&str> = target.split(':').collect();
            if parts.len() != 2 {
                return Err(BridgeError::InvalidEntryPoint(target.to_string()));
            }

            // Resolve module path - handle relative imports
            let module_path = if parts[0].starts_with('.') {
                // Relative import: resolve using package name
                let package_name = plugin_metadata
                    .and_then(|p| p.package_name.as_ref())
                    .ok_or_else(|| {
                        BridgeError::Python(
                            "Cannot resolve relative import without package name".to_string(),
                        )
                    })?;

                // Replace hyphens with underscores in package name (e.g., r2x-sienna -> r2x_sienna)
                let normalized_package = package_name.replace('-', "_");

                // Concatenate: package + relative path (e.g., r2x_sienna + .upgrader = r2x_sienna.upgrader)
                let resolved = format!("{}{}", normalized_package, parts[0]);
                logger::debug(&format!(
                    "Resolving relative import '{}' using package '{}' -> '{}'",
                    parts[0], package_name, resolved
                ));
                resolved
            } else {
                logger::debug(&format!("Using absolute module path: '{}'", parts[0]));
                parts[0].to_string()
            };

            logger::debug(&format!("Final module path for import: '{}'", module_path));

            // Import upgrader module and get class
            let module = PyModule::import(py, &module_path)
                .map_err(|e| BridgeError::Import(module_path.to_string(), format!("{}", e)))?;
            let upgrader_class = module.getattr(parts[1]).map_err(|e| {
                BridgeError::Python(format!(
                    "Failed to get upgrader class '{}': {}",
                    parts[1], e
                ))
            })?;

            // Instantiate upgrader and get steps
            logger::debug("Instantiating upgrader class");
            let upgrader_instance = upgrader_class.call1((&path_str,)).map_err(|e| {
                BridgeError::Python(format!("Failed to instantiate upgrader: {}", e))
            })?;

            let list_steps_method = upgrader_instance.getattr("list_steps").map_err(|e| {
                BridgeError::Python(format!("Failed to get list_steps method: {}", e))
            })?;

            let steps_py = list_steps_method
                .call0()
                .map_err(|e| BridgeError::Python(format!("Failed to call list_steps: {}", e)))?;

            let steps_len = steps_py
                .len()
                .map_err(|e| BridgeError::Python(format!("Failed to get steps length: {}", e)))?;

            logger::debug(&format!("Upgrader has {} steps", steps_len));

            // Re-configure Python logging before executing steps to ensure handlers are active
            let log_file = crate::logger::get_log_path_string();
            let fmt = "[{time:YYYY-MM-DD HH:mm:ss}] [PYTHON] {level: <8} {message}";
            let verbosity = crate::logger::get_verbosity();
            let log_level = match verbosity {
                0 => "WARNING",
                1 => "INFO",
                2 => "DEBUG",
                _ => "TRACE",
            };
            let enable_console = crate::logger::get_log_python();
            let logger_module = PyModule::import(py, "r2x_core.logger").map_err(|e| {
                BridgeError::Import("r2x_core.logger".to_string(), format!("{}", e))
            })?;
            let setup_logging = logger_module
                .getattr("setup_logging")
                .map_err(|e| BridgeError::Python(format!("setup_logging not found: {}", e)))?;
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs.set_item("level", log_level)?;
            kwargs.set_item("log_file", &log_file)?;
            kwargs.set_item("fmt", fmt)?;
            kwargs.set_item("enable_console_log", enable_console)?;
            setup_logging
                .call((), Some(&kwargs))
                .map_err(|e| BridgeError::Python(format!("setup_logging() failed: {}", e)))?;
            logger::debug("Python logging re-configured for upgrade step execution");

            // Execute steps
            let mut executed_system_steps = false;
            let mut current_system_data: Option<pyo3::Bound<PyAny>> = None;

            // Import pathlib once for FILE steps
            let pathlib = PyModule::import(py, "pathlib").ok();

            for i in 0..steps_len {
                let step = steps_py
                    .get_item(i)
                    .map_err(|e| BridgeError::Python(format!("Failed to get step {}: {}", i, e)))?;

                let step_name = step.getattr("name")?.extract::<String>()?;
                let upgrade_type = step
                    .getattr("upgrade_type")?
                    .getattr("value")?
                    .extract::<String>()?;
                let step_func = step.getattr("func")?;

                logger::step(&format!(
                    "Executing upgrade step '{}' (type: {})",
                    step_name, upgrade_type
                ));

                if upgrade_type == "FILE" {
                    // FILE step: modify files in place
                    let pathlib_module = pathlib.as_ref().ok_or_else(|| {
                        BridgeError::Import(
                            "pathlib".to_string(),
                            "pathlib module not available".to_string(),
                        )
                    })?;
                    let path_obj = pathlib_module
                        .getattr("Path")?
                        .call1((&path_str,))
                        .map_err(|e| {
                            BridgeError::Python(format!("Failed to create Path: {}", e))
                        })?;

                    step_func.call1((path_obj,)).map_err(|e| {
                        BridgeError::Python(format!("FILE step '{}' failed: {}", step_name, e))
                    })?;
                } else if upgrade_type == "SYSTEM" {
                    // SYSTEM step: transform JSON data
                    executed_system_steps = true;

                    // Load JSON on first SYSTEM step
                    if current_system_data.is_none() {
                        let json_content = std::fs::read_to_string(&path_str).map_err(|e| {
                            BridgeError::Python(format!(
                                "Failed to read file '{}': {}",
                                path_str, e
                            ))
                        })?;

                        current_system_data = Some(loads.call1((json_content,)).map_err(|e| {
                            BridgeError::Python(format!("Failed to parse JSON: {}", e))
                        })?);
                    }

                    // Transform data through step
                    let data = current_system_data.as_ref().unwrap();
                    let result = step_func.call1((data,)).map_err(|e| {
                        BridgeError::Python(format!("SYSTEM step '{}' failed: {}", step_name, e))
                    })?;
                    current_system_data = Some(result);
                } else {
                    logger::warn(&format!(
                        "Unknown upgrade step type: '{}', skipping",
                        upgrade_type
                    ));
                }
            }

            // Return JSON output if SYSTEM steps were executed
            if executed_system_steps {
                let final_data = current_system_data.ok_or_else(|| {
                    BridgeError::Python("No SYSTEM step output available".to_string())
                })?;
                let json_str = dumps.call1((&final_data,))?.extract::<String>()?;
                logger::debug("Upgrader execution completed successfully");
                Ok(json_str)
            } else {
                logger::debug("No SYSTEM steps executed, returning empty output");
                Ok(String::new())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_plugin_invocation_placeholder() {
        // Plugin invocation tests would require actual plugins
        // This is a placeholder for integration testing
    }
}
