//! Configuration for plugin discovery
//!
//! Defines supported plugin types and their mappings.

/// Supported plugin types in r2x ecosystem
pub struct PluginTypeConfig;

impl PluginTypeConfig {
    /// List of recognized plugin class names
    pub const PLUGIN_CLASSES: &'static [&'static str] = &[
        "ParserPlugin",
        "UpgraderPlugin",
        "BasePlugin",
        "ExporterPlugin",
    ];
}
