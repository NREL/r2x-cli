use crate::logger;
use crate::plugin_manifest::PluginManifest;
use colored::Colorize;

/// Show help for the run command when invoked with no arguments
pub fn show_run_help() -> Result<(), String> {
    let manifest = PluginManifest::load().map_err(|e| format!("Failed to load manifest: {}", e))?;

    println!();
    println!("{}", "No pipeline or plugin specified.".bold());
    println!();

    // Show installed plugins
    if !manifest.is_empty() {
        println!("{}", "Installed plugins:".bold());
        let plugins = manifest.list_plugins();
        for (name, plugin) in &plugins {
            let plugin_type = plugin.plugin_type.as_deref().unwrap_or("unknown");
            let desc = plugin.description.as_deref().unwrap_or("No description");
            println!(
                "  {} {} - {}",
                name.cyan(),
                format!("({})", plugin_type).dimmed(),
                desc
            );
        }
        println!();
    } else {
        println!("{}", "No plugins installed.".yellow());
        println!("Install plugins with: r2x install <package>");
        println!();
    }

    // Show usage hints
    println!("{}", "Usage:".bold());
    println!("  Run a pipeline:");
    println!("    r2x run <pipeline.yaml> [pipeline-name]");
    println!();
    println!("  Run a plugin directly:");
    println!("    r2x run --plugin <plugin-name> [OPTIONS]");
    println!();
    println!("  Get plugin help:");
    println!("    r2x run --plugin <plugin-name> --show-help");
    println!();
    println!("  List pipelines in YAML:");
    println!("    r2x run <pipeline.yaml> --list");
    println!();
    println!("  Print resolved pipeline config:");
    println!("    r2x run <pipeline.yaml> --print <pipeline-name>");
    println!();

    Ok(())
}

/// Show detailed help for a specific plugin
pub fn show_plugin_help(plugin_name: &str) -> Result<(), String> {
    let manifest = PluginManifest::load().map_err(|e| format!("Failed to load manifest: {}", e))?;

    let plugin = manifest
        .plugins
        .get(plugin_name)
        .ok_or_else(|| format!("Plugin '{}' not found in manifest", plugin_name))?;

    logger::step(&format!("Plugin: {}", plugin_name));

    if let Some(desc) = &plugin.description {
        println!("\n{}", desc);
    }

    if let Some(doc) = &plugin.doc {
        println!("\n{}", doc);
    }

    if let Some(plugin_type) = &plugin.plugin_type {
        println!("\nType: {}", plugin_type);
    }

    if let Some(io_type) = &plugin.io_type {
        println!("I/O: {}", io_type);
    }

    // Check if plugin requires data store
    let needs_store = check_needs_datastore(plugin);

    if needs_store {
        println!("\nRequires data store: yes");
        println!("\nData Store Arguments:");
        println!("  --store-path <PATH>       Path to store directory (required)");
        println!("  --store-name <NAME>       Name of the store (optional)");
    }

    // Show callable parameters
    if let Some(obj) = &plugin.obj {
        println!("\nCallable: {}.{}", obj.module, obj.name);
        if let Some(call_method) = &plugin.call_method {
            println!("Method: {}", call_method);
        }

        if !obj.parameters.is_empty() {
            println!("\nCallable Parameters:");
            for (name, param) in &obj.parameters {
                let annotation = param.annotation.as_deref().unwrap_or("Any");
                let required = if param.is_required {
                    "required"
                } else {
                    "optional"
                };
                let default = param
                    .default
                    .as_deref()
                    .map(|d| format!(" (default: {})", d))
                    .unwrap_or_default();
                println!(
                    "  --{:<20} {:<15} {}{}",
                    name, annotation, required, default
                );
            }
        }
    }

    // Show config parameters
    if let Some(config) = &plugin.config {
        println!("\nConfiguration Class: {}.{}", config.module, config.name);
        if !config.parameters.is_empty() {
            println!("\nConfiguration Parameters:");
            for (name, param) in &config.parameters {
                let annotation = param.annotation.as_deref().unwrap_or("Any");
                let required = if param.is_required {
                    "required"
                } else {
                    "optional"
                };
                let default = param
                    .default
                    .as_deref()
                    .map(|d| format!(" (default: {})", d))
                    .unwrap_or_default();
                println!(
                    "  --{:<20} {:<15} {}{}",
                    name, annotation, required, default
                );
            }
        }
    }

    println!("\nUsage:");
    println!("  r2x run --plugin {} [OPTIONS]", plugin_name);
    println!("\nExamples:");
    println!("  r2x run --plugin {} --show-help", plugin_name);

    if needs_store {
        println!(
            "  r2x run --plugin {} --store-path /path/to/store <other args>",
            plugin_name
        );
    } else {
        println!("  r2x run --plugin {} <args>", plugin_name);
    }

    Ok(())
}

/// Check if a plugin requires a DataStore
fn check_needs_datastore(plugin: &crate::plugin_manifest::Plugin) -> bool {
    let mut needs_store = plugin.requires_store.unwrap_or(false);

    if let Some(obj) = &plugin.obj {
        for param in obj.parameters.values() {
            if let Some(annotation) = &param.annotation {
                if annotation.contains("DataStore") || annotation.contains("data_store") {
                    needs_store = true;
                    break;
                }
            }
        }
    }
    if !needs_store {
        if let Some(config) = &plugin.config {
            for param in config.parameters.values() {
                if let Some(annotation) = &param.annotation {
                    if annotation.contains("DataStore") || annotation.contains("data_store") {
                        needs_store = true;
                        break;
                    }
                }
            }
        }
    }

    needs_store
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_show_run_help() {
        // Test run help display
    }

    #[test]
    fn test_show_plugin_help() {
        // Test plugin help display
    }
}
