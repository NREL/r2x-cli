//! Plugin constructor parsing module
//!
//! This module handles the parsing of plugin constructor calls and extraction
//! of their keyword arguments. It converts Python constructor syntax into
//! structured JSON data matching the Pydantic model format.

use crate::errors::BridgeError;
use crate::logger;
use crate::plugins::ast_discovery::ImportMap;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

/// Parse a plugin constructor and extract its arguments into JSON
///
/// # Arguments
/// * `plugin_def` - The full plugin constructor definition string
/// * `import_map` - Mapping of imported symbols to their modules
/// * `package_path` - Path to the package being analyzed
///
/// # Returns
/// JSON Value containing all plugin properties
pub fn parse_plugin_constructor(
    plugin_def: &str,
    import_map: &ImportMap,
    package_path: &Path,
    infer_plugin_type: fn(&str) -> &'static str,
) -> Result<Value, BridgeError> {
    let constructor_type = if let Some(type_end) = plugin_def.find('(') {
        plugin_def[..type_end].trim()
    } else {
        return Err(BridgeError::PluginNotFound(
            "Invalid plugin constructor format".to_string(),
        ));
    };

    let kwargs = extract_kwargs(plugin_def, import_map, package_path)?;

    let name = kwargs.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
        BridgeError::PluginNotFound("Plugin missing 'name' field".to_string())
    })?;

    let obj = kwargs.get("obj").cloned().ok_or_else(|| {
        BridgeError::PluginNotFound(format!("Plugin '{}' missing 'obj' field", name))
    })?;

    let io_type = kwargs.get("io_type").and_then(|v| v.as_str());

    let plugin_type = infer_plugin_type(constructor_type);

    let mut result = json!({
        "name": name,
        "plugin_type": plugin_type,
        "obj": obj,
    });

    if let Some(obj_val) = result.get_mut("obj") {
        if let Some(module) = obj_val.get("module").and_then(|v| v.as_str()) {
            if let Some(obj_name) = obj_val.get("name").and_then(|v| v.as_str()) {
                logger::debug(&format!("  Resolved obj: {}:{}", module, obj_name));
            }
        }
    }

    if let Some(call_method) = kwargs.get("call_method") {
        result["call_method"] = call_method.clone();
    }

    if let Some(config) = kwargs.get("config") {
        result["config"] = config.clone();
    }

    if let Some(io) = io_type {
        result["io_type"] = json!(io);
    }

    if let Some(requires_store) = kwargs.get("requires_store") {
        result["requires_store"] = requires_store.clone();
    }

    if let Some(version_strategy) = kwargs.get("version_strategy") {
        result["version_strategy"] = version_strategy.clone();
    }

    if let Some(version_reader) = kwargs.get("version_reader") {
        result["version_reader"] = version_reader.clone();
    }

    if let Some(upgrade_steps) = kwargs.get("upgrade_steps") {
        logger::debug(&format!(
            "Adding upgrade_steps with {} items",
            upgrade_steps.as_array().map(|a| a.len()).unwrap_or(0)
        ));
        result["upgrade_steps"] = upgrade_steps.clone();
    }

    Ok(result)
}

/// Extract keyword arguments from plugin constructor definition
///
/// Parses Python constructor syntax: PluginType(key1=value1, key2=value2, ...)
/// Handles nested parentheses, brackets, braces, and string literals.
///
/// # Arguments
/// * `plugin_def` - The plugin constructor string
/// * `import_map` - Symbol to module mapping
/// * `package_path` - Package path for resolving symbols
///
/// # Returns
/// HashMap of keyword arguments as JSON Values
pub fn extract_kwargs(
    plugin_def: &str,
    import_map: &ImportMap,
    package_path: &Path,
) -> Result<HashMap<String, Value>, BridgeError> {
    let mut kwargs = HashMap::new();

    let paren_start = plugin_def.find('(').ok_or_else(|| {
        BridgeError::PluginNotFound("No opening parenthesis in plugin constructor".to_string())
    })?;
    let content = &plugin_def[paren_start + 1..];

    let mut current_key = String::new();
    let mut current_value = String::new();
    let mut in_key = true;
    let mut paren_depth = 0;
    let mut bracket_depth = 0;
    let mut brace_depth = 0;
    let mut in_string = false;
    let mut string_char = ' ';

    for c in content.chars() {
        if in_key && (c == ' ' || c == '\t' || c == '\n' || c == '\r') && current_key.is_empty() {
            continue;
        }

        if in_string {
            current_value.push(c);
            if c == string_char && !current_value.ends_with("\\\"") {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' | '\'' => {
                in_string = true;
                string_char = c;
                current_value.push(c);
            }
            '=' if in_key => {
                in_key = false;
                current_key = current_key.trim().to_string();
            }
            '(' => {
                paren_depth += 1;
                current_value.push(c);
            }
            ')' if paren_depth > 0 => {
                paren_depth -= 1;
                current_value.push(c);
            }
            ')' => {
                if !current_key.is_empty() && !current_value.is_empty() {
                    let value = parse_kwarg_value(&current_value.trim(), import_map, package_path)?;
                    kwargs.insert(current_key.clone(), value);
                }
                break;
            }
            '[' => {
                bracket_depth += 1;
                current_value.push(c);
            }
            ']' => {
                bracket_depth -= 1;
                current_value.push(c);
            }
            '{' => {
                brace_depth += 1;
                current_value.push(c);
            }
            '}' => {
                brace_depth -= 1;
                current_value.push(c);
            }
            ',' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                if !current_key.is_empty() && !current_value.is_empty() {
                    let value = parse_kwarg_value(&current_value.trim(), import_map, package_path)?;
                    kwargs.insert(current_key.clone(), value);
                    current_key.clear();
                    current_value.clear();
                    in_key = true;
                }
            }
            _ => {
                if in_key {
                    current_key.push(c);
                } else {
                    current_value.push(c);
                }
            }
        }
    }

    let cleaned_kwargs: HashMap<String, Value> = kwargs
        .into_iter()
        .map(|(key, value)| {
            let cleaned_key = if let Some(last_colon) = key.rfind(':') {
                key[last_colon + 1..].trim().to_string()
            } else {
                key
            };
            (cleaned_key, value)
        })
        .collect();

    Ok(cleaned_kwargs)
}

/// Parse a single keyword argument value and convert to JSON
///
/// Handles various Python value types:
/// - Literals: None, True, False, strings, numbers
/// - Enum values: IOType.STDOUT -> "stdout"
/// - Class/function references: ReEDSParser -> full module path
/// - Decorator-based attributes: ClassName.steps
///
/// # Arguments
/// * `value` - The value string from the constructor
/// * `import_map` - Symbol to module mapping
/// * `package_path` - Package path for resolving decorator attributes
///
/// # Returns
/// Parsed value as serde_json::Value
pub fn parse_kwarg_value(
    value: &str,
    import_map: &ImportMap,
    package_path: &Path,
) -> Result<Value, BridgeError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Ok(Value::Null);
    }

    if trimmed == "None" {
        return Ok(Value::Null);
    }

    if trimmed == "True" {
        return Ok(Value::Bool(true));
    }

    if trimmed == "False" {
        return Ok(Value::Bool(false));
    }

    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        let string_content = &trimmed[1..trimmed.len() - 1];
        return Ok(Value::String(string_content.to_string()));
    }

    if let Some(enum_value) = resolve_enum_value(trimmed) {
        return Ok(Value::String(enum_value));
    }

    if trimmed.contains('.') && !trimmed.starts_with('[') {
        if trimmed.ends_with(".steps") {
            if let Some(class_name) = trimmed.strip_suffix(".steps") {
                return Err(BridgeError::PluginNotFound(format!(
                    "Decorator-based extraction for '{}' requires decorator_processor module",
                    class_name
                )));
            }
        }

        return Err(BridgeError::PluginNotFound(format!(
            "Attribute access '{}' requires Python runtime - unsupported in AST mode",
            trimmed
        )));
    }

    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return Ok(Value::Array(vec![]));
    }

    if import_map.symbols.contains_key(trimmed) {
        let (module, name) = &import_map.symbols[trimmed];
        return Ok(json!({
            "module": module,
            "name": name,
            "type": infer_callable_type_from_name(name),
            "return_annotation": Value::Null,
            "parameters": {}
        }));
    }

    Ok(Value::String(trimmed.to_string()))
}

/// Resolve Python enum values like IOType.STDOUT to their string representations
fn resolve_enum_value(expr: &str) -> Option<String> {
    match expr {
        "IOType.STDOUT" => Some("stdout".to_string()),
        "IOType.STDIN" => Some("stdin".to_string()),
        "IOType.BOTH" => Some("both".to_string()),
        _ => None,
    }
}

/// Infer callable type from name heuristic
/// Classes start with uppercase, functions are lowercase
fn infer_callable_type_from_name(name: &str) -> &'static str {
    if name.chars().next().map_or(false, |c| c.is_uppercase()) {
        "class"
    } else {
        "function"
    }
}
