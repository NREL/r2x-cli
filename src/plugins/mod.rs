// Re-export public operations
pub mod ast_discovery;
pub mod clean;
pub mod config;
pub mod discovery;
pub mod install;
pub mod list;
pub mod package_resolver;
pub mod package_spec;
pub mod plugin_parser;
pub mod remove;
pub mod sync;
pub mod utils;

// Re-export public functions from submodules
pub use ast_discovery::AstDiscovery;
pub use clean::clean_manifest;
pub use config::PluginTypeConfig;
pub use install::{install_plugin, show_install_help, GitOptions};
pub use list::list_plugins;
pub use package_resolver::find_package_path;
pub use plugin_parser::parse_plugin_json;
pub use remove::remove_plugin;
pub use sync::sync_manifest;

#[cfg(test)]
mod tests {

    #[test]
    fn test_plugins_module() {
        // Module-level tests
    }
}
