use super::*;
use crate::Bridge;
use pyo3::types::{PyDict, PyModule, PyString};
use pyo3::Py;
use r2x_logger as logger;
use r2x_manifest::{runtime::RuntimeBindings, types::{ArgumentSource, ConfigSpec}};

impl Bridge {
    /// Build kwargs for a set of arguments declared in the manifest.
    pub(super) fn build_kwargs<'py>(
        &self,
        py: pyo3::Python<'py>,
        args: &[r2x_manifest::ArgumentSpec],
        config_dict: &pyo3::Bound<'py, PyDict>,
        stdin_obj: Option<&pyo3::Bound<'py, PyAny>>,
        runtime_bindings: Option<&RuntimeBindings>,
    ) -> Result<pyo3::Bound<'py, PyDict>, BridgeError> {
        let kwargs = PyDict::new(py);

        let runtime = match runtime_bindings {
            Some(binding) => binding,
            None => {
                for (k, v) in config_dict {
                    kwargs.set_item(k, v)?;
                }
                if let Some(stdin) = stdin_obj {
                    kwargs.set_item("stdin", stdin)?;
                }
                return Ok(kwargs);
            }
        };

        let mut config_instance: Option<pyo3::Py<pyo3::PyAny>> = None;
        let mut store_instance: Option<pyo3::Py<pyo3::PyAny>> = None;
        let mut system_instance: Option<pyo3::Py<pyo3::PyAny>> = None;

        for arg in args {
            // Respect explicit config overrides first
            if let Some(value) = config_dict.get_item(&arg.name).ok().flatten() {
                kwargs.set_item(&arg.name, value)?;
                continue;
            }

            match arg.source {
                ArgumentSource::Config => {
                    if config_instance.is_none() {
                        let config_params =
                            Self::extract_config_params(py, config_dict, store_instance.as_ref())?;
                        let cfg = self.instantiate_config_class(
                            py,
                            &config_params,
                            runtime.config.as_ref(),
                        )?;
                        config_instance = Some(cfg.unbind());
                    }
                    if let Some(cfg) = &config_instance {
                        kwargs.set_item(&arg.name, cfg.bind(py))?;
                    }
                }
                ArgumentSource::Store
                | ArgumentSource::StoreManifest
                | ArgumentSource::StoreInline => {
                    if store_instance.is_none() {
                        let value = config_dict
                            .get_item("store_path")
                            .ok()
                            .flatten()
                            .or_else(|| config_dict.get_item("path").ok().flatten())
                            .or_else(|| config_dict.get_item("store").ok().flatten())
                            .or_else(|| {
                                runtime
                                    .resources
                                    .as_ref()
                                    .and_then(|res| res.store.as_ref())
                            .and_then(|s| s.default_path.as_ref())
                            .map(|p| PyString::new(py, p).into_any())
                            });

                    if let Some(val) = value {
                        let store = if let Some(cfg) = config_instance.as_ref() {
                            let cfg_bound = cfg.bind(py);
                            self.instantiate_data_store(
                                py,
                                &val,
                                Some(&cfg_bound),
                                runtime.config.as_ref(),
                            )?
                        } else {
                            self.instantiate_data_store(py, &val, None, runtime.config.as_ref())?
                        };
                        store_instance = Some(store.unbind());
                    } else if !arg.optional {
                        return Err(BridgeError::Python(
                            "Store path missing for plugin invocation".to_string(),
                        ));
                    }
                }

                    if let Some(store) = &store_instance {
                        kwargs.set_item(&arg.name, store.bind(py))?;
                    }
                }
                ArgumentSource::System => {
                    if system_instance.is_none() {
                        system_instance = self.build_system_from_stdin(py, stdin_obj)?;
                    }
                    if let Some(system) = &system_instance {
                        kwargs.set_item(&arg.name, system.bind(py))?;
                    }
                }
                ArgumentSource::Stdin => {
                    if let Some(stdin) = stdin_obj {
                        kwargs.set_item(&arg.name, stdin)?;
                    } else if !arg.optional {
                        return Err(BridgeError::Python(
                            "stdin payload required but not provided".to_string(),
                        ));
                    }
                }
                ArgumentSource::Path | ArgumentSource::ConfigPath => {
                    let value = config_dict
                        .get_item(&arg.name)
                        .ok()
                        .flatten()
                        .or_else(|| config_dict.get_item("path").ok().flatten());
                    if let Some(v) = value {
                        kwargs.set_item(&arg.name, v)?;
                    } else if let Some(default) = runtime
                        .resources
                        .as_ref()
                        .and_then(|res| res.store.as_ref())
                        .and_then(|s| s.default_path.as_ref())
                    {
                        kwargs.set_item(&arg.name, PyString::new(py, default))?;
                    } else if !arg.optional {
                        return Err(BridgeError::Python(format!(
                            "Missing required path argument '{}'",
                            arg.name
                        )));
                    }
                }
                ArgumentSource::Literal => {
                    if let Some(default) = &arg.default {
                        let py_val = json_value_to_py(py, default)?;
                        kwargs.set_item(&arg.name, py_val.bind(py))?;
                    }
                }
                ArgumentSource::Custom | ArgumentSource::Context => {
                    if let Some(val) = config_dict.get_item(&arg.name).ok().flatten() {
                        kwargs.set_item(&arg.name, val)?;
                    } else if let Some(default) = &arg.default {
                        let py_val = json_value_to_py(py, default)?;
                        kwargs.set_item(&arg.name, py_val.bind(py))?;
                    } else if !arg.optional {
                        logger::warn(&format!(
                            "Required parameter '{}' missing in config",
                            arg.name
                        ));
                    }
                }
            }
        }

        Ok(kwargs)
    }

    fn extract_config_params<'py>(
        py: pyo3::Python<'py>,
        config_dict: &pyo3::Bound<'py, PyDict>,
        store_instance: Option<&pyo3::Py<pyo3::PyAny>>,
    ) -> Result<pyo3::Bound<'py, PyDict>, BridgeError> {
        if let Ok(Some(existing_config)) = config_dict.get_item("config") {
            if let Ok(config_dict_value) = existing_config.cast::<PyDict>() {
                return Ok(config_dict_value.clone());
            }
        }

        let params = PyDict::new(py);
        for (key, value) in config_dict.iter() {
            let key_str = key.extract::<String>().unwrap_or_default();
            if key_str == "data_store" || key_str == "store_path" || key_str == "path" {
                continue;
            }
            params.set_item(key, value)?;
        }

        if let Some(store) = store_instance {
            params.set_item("data_store", store.bind(py))?;
        }

        Ok(params)
    }

    pub(super) fn instantiate_config_class<'py>(
        &self,
        py: pyo3::Python<'py>,
        config_params: &pyo3::Bound<'py, PyDict>,
        config_metadata: Option<&ConfigSpec>,
    ) -> Result<pyo3::Bound<'py, PyAny>, BridgeError> {
        let config_meta = config_metadata.ok_or_else(|| {
            BridgeError::Python("Plugin config metadata missing".to_string())
        })?;
        let model_path = config_meta
            .model
            .as_deref()
            .ok_or_else(|| BridgeError::Python("Config model path missing".to_string()))?;
        let (module, class_name) = split_qualified_target(model_path).ok_or_else(|| {
            BridgeError::Python(format!("Invalid config model path: {}", model_path))
        })?;

        let config_module = PyModule::import(py, &module).map_err(|e| {
            BridgeError::Python(format!(
                "Failed to import config module '{}': {}",
                module, e
            ))
        })?;
        let config_class = config_module.getattr(&class_name).map_err(|e| {
            BridgeError::Python(format!(
                "Failed to get config class '{}': {}",
                class_name, e
            ))
        })?;

        config_class.call((), Some(&config_params)).map_err(|e| {
            BridgeError::Python(format!(
                "Failed to instantiate config class '{}': {}",
                class_name, e
            ))
        })
    }

    pub(super) fn instantiate_data_store<'py>(
        &self,
        py: pyo3::Python<'py>,
        value: &pyo3::Bound<'py, PyAny>,
        config_instance: Option<&pyo3::Bound<'py, PyAny>>,
        _config_metadata: Option<&ConfigSpec>,
    ) -> Result<pyo3::Bound<'py, PyAny>, BridgeError> {
        let path = if let Ok(store_dict) = value.cast::<PyDict>() {
            let path = store_dict
                .get_item("path")?
                .or_else(|| store_dict.get_item("folder").ok().flatten())
                .ok_or_else(|| BridgeError::Python("data_store path missing".to_string()))?
                .extract::<String>()?;
            path
        } else if let Ok(path_str) = value.extract::<String>() {
            path_str
        } else {
            return Err(BridgeError::Python(
                "Invalid data_store format. Provide dict or store path".to_string(),
            ));
        };

        let data_store_module = PyModule::import(py, "r2x_core.store")?;
        let data_store_class = data_store_module.getattr("DataStore")?;

        if let Some(config) = config_instance {
            let from_config = data_store_class
                .getattr("from_plugin_config")
                .map_err(|e| {
                    BridgeError::Python(format!("DataStore missing from_plugin_config: {}", e))
                })?;
            match from_config.call1((config, path.clone())) {
                Ok(store) => Ok(store),
                Err(err) => Err(transform_data_store_error(py, err)),
            }
        } else {
            data_store_class
                .call1((path.clone(),))
                .map_err(|err| transform_data_store_error(py, err))
        }
    }

    fn build_system_from_stdin<'py>(
        &self,
        py: pyo3::Python<'py>,
        stdin_obj: Option<&pyo3::Bound<'py, PyAny>>,
    ) -> Result<Option<pyo3::Py<pyo3::PyAny>>, BridgeError> {
        let Some(stdin) = stdin_obj else {
            return Ok(None);
        };
        let json_module = PyModule::import(py, "json").map_err(|e| {
            BridgeError::Import("json".to_string(), format!("Failed to import json: {}", e))
        })?;
        let dumps = json_module
            .getattr("dumps")
            .map_err(|e| BridgeError::Python(format!("json.dumps not available: {}", e)))?;
        let json_str = dumps
            .call1((stdin,))?
            .extract::<String>()
            .map_err(|e| BridgeError::Python(format!("Failed to serialize stdin: {}", e)))?;
        let system_module = PyModule::import(py, "r2x_core.system")
            .map_err(|e| BridgeError::Import("r2x_core.system".to_string(), format!("{}", e)))?;
        let system_class = system_module.getattr("System").map_err(|e| {
            BridgeError::Python(format!("Failed to access r2x_core.system.System: {}", e))
        })?;
        let from_json = system_class
            .getattr("from_json")
            .map_err(|e| BridgeError::Python(format!("System.from_json missing: {}", e)))?;
        let system_obj = from_json.call1((json_str.as_bytes(),))?;
        Ok(Some(system_obj.unbind()))
    }
}

fn split_qualified_target(target: &str) -> Option<(String, String)> {
    if let Some(idx) = target.rfind(':') {
        Some((target[..idx].to_string(), target[idx + 1..].to_string()))
    } else if let Some(idx) = target.rfind('.') {
        Some((target[..idx].to_string(), target[idx + 1..].to_string()))
    } else {
        None
    }
}

fn json_value_to_py(
    py: pyo3::Python<'_>,
    value: &serde_json::Value,
) -> pyo3::PyResult<Py<pyo3::PyAny>> {
    let json_module = PyModule::import(py, "json")?;
    let loads = json_module.getattr("loads")?;
    let json_str = serde_json::to_string(value).unwrap_or_else(|_| "null".to_string());
    let obj = loads.call1((json_str,))?;
    Ok(obj.unbind())
}

fn transform_data_store_error(py: pyo3::Python<'_>, err: pyo3::PyErr) -> BridgeError {
    if let Some(missing) = extract_missing_data_file(py, &err) {
        BridgeError::Python(format!(
            "Missing required data file: {}. Verify the data folder contains all expected outputs.",
            missing
        ))
    } else {
        BridgeError::Python(format!("Failed to instantiate DataStore: {}", err))
    }
}

fn extract_missing_data_file(py: pyo3::Python<'_>, err: &pyo3::PyErr) -> Option<String> {
    let mut current = err.value(py).getattr("__context__").ok();
    let mut depth = 0;
    loop {
        let Some(ctx) = current else { break };
        if ctx.is_none() {
            break;
        }
        if let Ok(repr) = ctx.str() {
            logger::debug(&format!(
                "Python exception context[{}]: {}",
                depth,
                repr.to_string()
            ));
        }
        if ctx.is_instance_of::<pyo3::exceptions::PyFileNotFoundError>() {
            if let Ok(text) = ctx.str() {
                return Some(text.to_string());
            }
        }
        current = ctx.getattr("__context__").ok();
        depth += 1;
    }
    None
}
