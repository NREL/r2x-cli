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

    /// Infer the plugin type from constructor class name
    pub fn infer_type(constructor: &str) -> &'static str {
        match constructor {
            "ParserPlugin" => "parser",
            "ExporterPlugin" => "exporter",
            "UpgraderPlugin" => "upgrader",
            "BasePlugin" => "function",
            _ => "function",
        }
    }

    /// Infer callable type from the callable name
    pub fn infer_callable_type_from_name(name: &str) -> &'static str {
        if name.contains("parser") || name.contains("parse") {
            "parser"
        } else if name.contains("export") {
            "exporter"
        } else if name.contains("upgrade") {
            "upgrader"
        } else {
            "function"
        }
    }

    /// Resolve enum values for known types
    pub fn resolve_enum_value(expr: &str) -> Option<String> {
        match expr {
            "IOType.STDOUT" => Some("stdout".to_string()),
            "IOType.STDIN" => Some("stdin".to_string()),
            "IOType.BOTH" => Some("both".to_string()),
            _ => None,
        }
    }
}
