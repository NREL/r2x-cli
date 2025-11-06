//! Decorator-based attribute extraction and processing
//!
//! This module handles extraction of upgrade steps from @register_step decorators
//! in Python source code. It recursively searches the package for decorated methods
//! and builds upgrade step JSON objects.

use crate::errors::BridgeError;
use crate::logger;
use serde_json::{json, Value};
use std::path::Path;

/// Extract decorator-based attributes like ClassName.steps from @register_step decorators
///
/// Searches the package directory tree for @ClassName.register_step(...) decorators
/// and extracts the upgrade step definitions.
///
/// # Arguments
/// * `class_name` - The class whose decorators we're looking for
/// * `attribute` - The attribute name (e.g., "steps")
/// * `package_path` - Root path to search
///
/// # Returns
/// JSON array of extracted upgrade steps
pub fn extract_decorator_based_attribute(
    class_name: &str,
    _attribute: &str,
    package_path: &Path,
) -> Result<Value, BridgeError> {
    let mut steps = Vec::new();
    search_decorators_recursive(package_path, class_name, &mut steps);

    if steps.is_empty() {
        return Err(BridgeError::PluginNotFound(format!(
            "No decorators found for {}.{}",
            class_name, _attribute
        )));
    }

    Ok(Value::Array(steps))
}

/// Recursively search for decorators in a directory tree
///
/// Traverses the directory structure looking for Python files that might
/// contain @ClassName.register_step decorators. Skips common non-source
/// directories like __pycache__ and venv.
///
/// # Arguments
/// * `path` - Current directory to search
/// * `class_name` - The class name to search for
/// * `steps` - Mutable vector to accumulate found steps
pub fn search_decorators_recursive(path: &Path, class_name: &str, steps: &mut Vec<Value>) {
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let entry_path = entry.path();

                let skip = entry_path
                    .file_name()
                    .map(|name| {
                        let name_str = name.to_string_lossy();
                        name_str.starts_with('.') || name_str == "__pycache__" || name_str == "venv"
                    })
                    .unwrap_or(false);

                if skip {
                    continue;
                }

                if entry_path.is_file()
                    && entry_path.extension().map_or(false, |ext| ext == "py")
                {
                    if let Ok(content) = std::fs::read_to_string(&entry_path) {
                        if let Ok(found_steps) = extract_steps_from_decorators(&content, class_name)
                        {
                            steps.extend(found_steps);
                        }
                    }
                } else if entry_path.is_dir() {
                    search_decorators_recursive(&entry_path, class_name, steps);
                }
            }
        }
    }
}

/// Extract upgrade steps from @ClassName.register_step(...) decorators in file content
///
/// Searches file content for decorator patterns and extracts each decorated method
/// as an upgrade step definition.
///
/// # Arguments
/// * `content` - Python source code to search
/// * `class_name` - The class whose decorators we're looking for
///
/// # Returns
/// Vector of JSON upgrade step objects
pub fn extract_steps_from_decorators(
    content: &str,
    class_name: &str,
) -> Result<Vec<Value>, BridgeError> {
    let mut steps = Vec::new();
    let decorator_pattern = format!("@{}.register_step(", class_name);

    let mut search_from = 0;
    while let Some(decorator_pos) = content[search_from..].find(&decorator_pattern) {
        let actual_pos = search_from + decorator_pos;

        let paren_start = actual_pos + decorator_pattern.len() - 1;
        if let Some(paren_end) = find_matching_paren(content, paren_start) {
            let decorator_args = &content[actual_pos + decorator_pattern.len()..paren_end];

            let rest_of_file = &content[paren_end..];
            if let Some(def_pos) = rest_of_file.find("def ") {
                let def_start = def_pos + 4;
                if let Some(paren_pos) = rest_of_file[def_start..].find('(') {
                    let func_name = rest_of_file[def_start..def_start + paren_pos]
                        .trim()
                        .to_string();

                    let step_json = build_upgrade_step_from_decorator(&func_name, decorator_args)?;
                    steps.push(step_json);
                }
            }
        }

        search_from = actual_pos + 1;
    }

    Ok(steps)
}

/// Build an UpgradeStep JSON object from decorator arguments
///
/// Parses decorator arguments like:
///   target_version=LATEST_COMMIT, upgrade_type=UpgradeType.FILE, priority=30
///
/// And constructs a complete UpgradeStep JSON object with defaults for
/// missing fields.
///
/// # Arguments
/// * `func_name` - Name of the decorated function
/// * `decorator_args` - Raw decorator arguments string
///
/// # Returns
/// JSON object representing the upgrade step
pub fn build_upgrade_step_from_decorator(
    func_name: &str,
    decorator_args: &str,
) -> Result<Value, BridgeError> {
    let mut step = json!({
        "name": func_name,
        "func": {
            "module": "r2x_reeds.upgrader.upgrade_steps",
            "name": func_name,
            "type": "function",
            "return_annotation": Value::Null,
            "parameters": {}
        },
        "target_version": "unknown",
        "upgrade_type": "FILE",
        "priority": 100
    });

    for arg in decorator_args.split(',') {
        let arg = arg.trim();
        if let Some(eq_pos) = arg.find('=') {
            let key = arg[..eq_pos].trim();
            let value = arg[eq_pos + 1..].trim();

            match key {
                "target_version" => {
                    let cleaned = if value.starts_with('"') && value.ends_with('"') {
                        value[1..value.len() - 1].to_string()
                    } else {
                        value.to_string()
                    };
                    step["target_version"] = Value::String(cleaned);
                }
                "upgrade_type" => {
                    if let Some(dot_pos) = value.find('.') {
                        let type_name = &value[dot_pos + 1..];
                        step["upgrade_type"] = Value::String(type_name.to_uppercase());
                    }
                }
                "priority" => {
                    if let Ok(priority) = value.parse::<i64>() {
                        step["priority"] = Value::Number(priority.into());
                    }
                }
                "min_version" => {
                    let cleaned = if value.starts_with('"') && value.ends_with('"') {
                        value[1..value.len() - 1].to_string()
                    } else {
                        value.to_string()
                    };
                    step["min_version"] = Value::String(cleaned);
                }
                "max_version" => {
                    let cleaned = if value.starts_with('"') && value.ends_with('"') {
                        value[1..value.len() - 1].to_string()
                    } else {
                        value.to_string()
                    };
                    step["max_version"] = Value::String(cleaned);
                }
                _ => {}
            }
        }
    }

    Ok(step)
}

/// Find matching closing parenthesis for an opening parenthesis
///
/// Counts nested parentheses to locate the matching closing paren.
///
/// # Arguments
/// * `content` - String containing the parentheses
/// * `start` - Position of the opening parenthesis
///
/// # Returns
/// Position of matching closing parenthesis, or None if not found
fn find_matching_paren(content: &str, start: usize) -> Option<usize> {
    if start >= content.len() || !content[start..].starts_with('(') {
        return None;
    }

    let mut paren_count = 1;
    let chars: Vec<char> = content.chars().collect();

    for i in (start + 1)..chars.len() {
        match chars[i] {
            '(' => paren_count += 1,
            ')' => {
                paren_count -= 1;
                if paren_count == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }

    None
}
