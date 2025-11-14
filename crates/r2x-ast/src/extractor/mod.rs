use anyhow::{anyhow, Result};
use ast_grep_core::AstGrep;
use ast_grep_language::Python;
use r2x_manifest::{
    ConfigSpec, IOContract, IOSlot, ImplementationType, InvocationSpec, PluginKind, PluginSpec,
    ResourceSpec, StoreMode, StoreSpec,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

mod args;
#[allow(dead_code)]
mod parameters;
#[allow(dead_code)]
mod resolver;

#[cfg(test)]
mod tests;

pub struct PluginExtractor {
    pub(crate) python_file_path: PathBuf,
    pub(crate) content: String,
    pub(crate) import_map: HashMap<String, String>,
}

impl PluginExtractor {
    pub fn new(python_file_path: PathBuf) -> Result<Self> {
        debug!("Initializing plugin extractor for: {:?}", python_file_path);

        let content = fs::read_to_string(&python_file_path)?;
        if !content.contains("PluginManifest") && !content.contains("manifest.add") {
            return Err(anyhow!(
                "No PluginManifest found in: {:?}",
                python_file_path
            ));
        }

        let import_map = Self::build_import_map_static(&content);

        Ok(PluginExtractor {
            python_file_path,
            content,
            import_map,
        })
    }

    pub fn extract_plugins(&self) -> Result<Vec<PluginSpec>> {
        debug!(
            "Extracting plugins via AST parsing from: {:?}",
            self.python_file_path
        );

        let sg = AstGrep::new(&self.content, Python);
        let root = sg.root();

        let manifest_add_calls: Vec<_> = root.find_all("manifest.add($$$_)").collect();

        if manifest_add_calls.is_empty() {
            return Err(anyhow!("No manifest.add() calls found"));
        }

        debug!("Found {} manifest.add() calls", manifest_add_calls.len());
        let mut plugins = Vec::new();

        for add_match in manifest_add_calls {
            let add_text = add_match.text();
            if let Ok(plugin) = self.extract_plugin_from_add_call(add_text.as_ref()) {
                debug!("Extracted plugin: {}", plugin.name);
                plugins.push(plugin);
            }
        }

        info!("Extracted {} plugins from manifest", plugins.len());
        Ok(plugins)
    }

    fn extract_plugin_from_add_call(&self, add_text: &str) -> Result<PluginSpec> {
        debug!("Parsing PluginSpec from manifest.add(): {}", add_text.lines().next().unwrap_or(""));

        let sg = AstGrep::new(add_text, Python);
        let root = sg.root();

        let plugin_spec_calls: Vec<_> = root
            .find_all("PluginSpec.$METHOD($$$ARGS)")
            .collect();

        if plugin_spec_calls.is_empty() {
            return Err(anyhow!("No PluginSpec helper call found in manifest.add()"));
        }

        let spec_match = &plugin_spec_calls[0];
        let env = spec_match.get_env();

        let method = env
            .get_match("$METHOD")
            .ok_or_else(|| anyhow!("Missing helper method"))?
            .text()
            .to_string();

        let kind = match method.as_str() {
            "parser" => PluginKind::Parser,
            "exporter" => PluginKind::Exporter,
            "function" => PluginKind::Modifier,
            "upgrader" => PluginKind::Upgrader,
            "utility" => PluginKind::Utility,
            _ => return Err(anyhow!("Unknown PluginSpec helper method: {}", method)),
        };

        debug!("Detected plugin kind: {:?}", kind);

        let call_text = spec_match.text();
        let kwargs = self.extract_keyword_arguments_from_text(call_text.as_ref())?;

        let name = self.find_kwarg_value(&kwargs, "name")?;
        let entry = self.find_kwarg_value(&kwargs, "entry")?;

        let description = kwargs
            .iter()
            .find(|arg| arg.name == "description")
            .map(|arg| arg.value.trim_matches('"').to_string());

        let method_param = kwargs
            .iter()
            .find(|arg| arg.name == "method")
            .map(|arg| arg.value.trim_matches('"').to_string());

        let invocation = InvocationSpec {
            implementation: ImplementationType::Class,
            method: method_param,
            constructor: Vec::new(),
            call: Vec::new(),
        };

        let io = self.infer_io_contract(&kind);

        let resources = self.extract_resources(&kwargs);

        Ok(PluginSpec {
            name,
            kind,
            entry,
            invocation,
            io,
            resources,
            upgrade: None,
            description,
            tags: Vec::new(),
        })
    }

    fn infer_io_contract(&self, kind: &PluginKind) -> IOContract {
        match kind {
            PluginKind::Parser => IOContract {
                consumes: vec![IOSlot::StoreFolder, IOSlot::ConfigFile],
                produces: vec![IOSlot::System],
            },
            PluginKind::Exporter => IOContract {
                consumes: vec![IOSlot::System, IOSlot::ConfigFile],
                produces: vec![IOSlot::Folder],
            },
            PluginKind::Modifier => IOContract {
                consumes: vec![IOSlot::System],
                produces: vec![IOSlot::System],
            },
            _ => IOContract {
                consumes: Vec::new(),
                produces: Vec::new(),
            },
        }
    }

    fn extract_resources(&self, kwargs: &[args::KwArg]) -> Option<ResourceSpec> {
        let config = kwargs
            .iter()
            .find(|arg| arg.name == "config")
            .map(|arg| {
                let config_class = arg.value.trim().to_string();
                let module = self
                    .import_map
                    .get(&config_class)
                    .cloned()
                    .unwrap_or_default();

                ConfigSpec {
                    module,
                    name: config_class,
                    fields: Vec::new(),
                }
            });

        let store = kwargs
            .iter()
            .find(|arg| arg.name == "store")
            .map(|arg| {
                let value = arg.value.trim();
                if value == "True" || value == "true" {
                    StoreSpec {
                        mode: StoreMode::Folder,
                        path: None,
                    }
                } else {
                    StoreSpec {
                        mode: StoreMode::Folder,
                        path: Some(value.trim_matches('"').to_string()),
                    }
                }
            });

        if config.is_some() || store.is_some() {
            Some(ResourceSpec { store, config })
        } else {
            None
        }
    }

    fn build_import_map_static(content: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }

            if line.starts_with("from ") && line.contains(" import ") {
                if let Some(import_idx) = line.find(" import ") {
                    let module = line[5..import_idx].trim();
                    let imports_part = line[import_idx + 8..].trim();

                    for import_item in imports_part.split(',') {
                        let import_item = import_item.trim();
                        if import_item.ends_with('\\') || import_item.is_empty() {
                            continue;
                        }

                        let class_name = if let Some(as_idx) = import_item.find(" as ") {
                            import_item[as_idx + 4..].trim()
                        } else {
                            import_item
                        };

                        let class_name = class_name
                            .trim_matches(|c| c == '(' || c == ')' || c == ',')
                            .trim();

                        if !class_name.is_empty() && !class_name.starts_with('#') {
                            map.insert(class_name.to_string(), module.to_string());
                            debug!("Mapped class {} to module {}", class_name, module);
                        }
                    }
                }
            }
        }

        debug!("Built import map with {} entries", map.len());
        map
    }

    pub fn resolve_references(
        &self,
        _plugin: &mut PluginSpec,
        _package_root: &std::path::Path,
        _package_name: &str,
    ) -> Result<()> {
        Ok(())
    }
}
