//! Parameter extraction from callable definitions
//!
//! This module handles the extraction of function and class parameters
//! from Python source code using ast-grep for AST-based analysis.

use crate::errors::BridgeError;
use crate::logger;
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Extract parameters from a class or function using ast-grep
///
/// Attempts to locate a Python file containing the module and then
/// parse its __init__ or function signature to extract parameters.
///
/// # Arguments
/// * `module` - Full module path (e.g., "r2x_reeds.parser")
/// * `name` - Class or function name
/// * `package_path` - Path to search for the Python file
///
/// # Returns
/// Map of parameter names to their metadata
pub fn extract_callable_parameters(
    module: &str,
    name: &str,
    package_path: &Path,
) -> Result<serde_json::Map<String, Value>, BridgeError> {
    logger::debug(&format!(
        "extract_callable_parameters: module={}, name={}, package_path={}",
        module,
        name,
        package_path.display()
    ));

    let module_parts: Vec<&str> = module.split('.').collect();
    if module_parts.is_empty() {
        return Ok(serde_json::Map::new());
    }

    let file_name = format!("{}.py", module_parts[module_parts.len() - 1]);
    logger::debug(&format!(
        "extract_callable_parameters: looking for file {}",
        file_name
    ));

    let mut search_dirs = vec![package_path.to_string_lossy().to_string()];

    if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
        search_dirs.push(venv);
    }

    if let Ok(cwd) = std::env::current_dir() {
        search_dirs.push(cwd.to_string_lossy().to_string());
    }

    for dir_str in search_dirs {
        logger::debug(&format!(
            "extract_callable_parameters: searching in {}",
            dir_str
        ));
        let dir = PathBuf::from(&dir_str);
        if let Ok(file_content) = find_and_read_python_file(&dir, &file_name) {
            logger::debug(&format!(
                "extract_callable_parameters: found file, parsing signature"
            ));
            if let Ok(params) = parse_function_signature(&file_content, name) {
                logger::debug(&format!(
                    "extract_callable_parameters: extracted {} parameters",
                    params.len()
                ));
                return Ok(params);
            }
        }
    }

    logger::debug("extract_callable_parameters: no parameters found");
    Ok(serde_json::Map::new())
}

/// Find and read a Python file by name in a directory tree
///
/// Recursively searches for a file with the given name, skipping common
/// non-source directories like __pycache__, venv, and hidden directories.
///
/// # Arguments
/// * `start_path` - Root directory to search
/// * `file_name` - Name of the Python file to find
///
/// # Returns
/// File contents as a string
pub fn find_and_read_python_file(
    start_path: &Path,
    file_name: &str,
) -> Result<String, BridgeError> {
    use std::fs;

    logger::debug(&format!(
        "find_and_read_python_file: searching for {} in {}",
        file_name,
        start_path.display()
    ));

    if let Ok(entries) = fs::read_dir(start_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.') || name_str == "__pycache__" || name_str == "venv"
                    {
                        continue;
                    }
                }

                if path.is_file()
                    && path
                        .file_name()
                        .map_or(false, |n| n.to_string_lossy() == file_name)
                {
                    logger::debug(&format!(
                        "find_and_read_python_file: found file at {}",
                        path.display()
                    ));
                    return fs::read_to_string(&path).map_err(|e| {
                        BridgeError::PluginNotFound(format!(
                            "Failed to read {}: {}",
                            file_name, e
                        ))
                    });
                }

                if path.is_dir() {
                    if let Ok(content) = find_and_read_python_file(&path, file_name) {
                        return Ok(content);
                    }
                }
            }
        }
    }

    logger::debug(&format!(
        "find_and_read_python_file: file {} not found in {}",
        file_name,
        start_path.display()
    ));
    Err(BridgeError::PluginNotFound(format!(
        "File {} not found",
        file_name
    )))
}

/// Parse function/class signature using ast-grep to extract parameters
///
/// Uses ast-grep to find the class definition and then extracts the
/// __init__ method parameters (if it's a class) or function parameters.
///
/// # Arguments
/// * `file_content` - Full content of the Python file
/// * `name` - Name of the class or function to parse
///
/// # Returns
/// Map of parameter names to their metadata
pub fn parse_function_signature(
    file_content: &str,
    name: &str,
) -> Result<serde_json::Map<String, Value>, BridgeError> {
    let pattern = format!("class {}", name);
    logger::debug(&format!("parse_function_signature: looking for pattern: {}", pattern));

    let mut child = Command::new("ast-grep")
        .arg("run")
        .arg("--pattern")
        .arg(&pattern)
        .arg("--lang")
        .arg("python")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            BridgeError::Initialization(format!("Failed to spawn ast-grep: {}", e))
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(file_content.as_bytes());
    }

    let output = child.wait_with_output().map_err(|e| {
        BridgeError::Initialization(format!("Failed to run ast-grep: {}", e))
    })?;

    logger::debug(&format!(
        "parse_function_signature: ast-grep status: {}",
        output.status.success()
    ));

    if output.status.success() {
        logger::debug("parse_function_signature: calling extract_init_parameters");
        return extract_init_parameters(file_content, name);
    }

    Ok(serde_json::Map::new())
}

/// Extract __init__ method parameters from a class using ast-grep
///
/// Finds the __init__ method definition in a class and extracts all
/// typed parameters with their annotations and default values.
///
/// # Arguments
/// * `file_content` - Full Python source code
/// * `class_name` - Name of the class to analyze
///
/// # Returns
/// Map of parameter names to objects with "annotation" and "is_required" fields
pub fn extract_init_parameters(
    file_content: &str,
    class_name: &str,
) -> Result<serde_json::Map<String, Value>, BridgeError> {
    let mut parameters = serde_json::Map::new();

    if let Some(class_start) = file_content.find(&format!("class {}", class_name)) {
        let class_section = &file_content[class_start..];
        if let Some(init_pos) = class_section.find("def __init__") {
            let init_section = &class_section[init_pos..];

            if let Some(open_paren) = init_section.find('(') {
                if let Some(close_paren) = init_section.find(')') {
                    let init_start_byte = class_start + init_pos;
                    let params_start_byte = init_start_byte + open_paren;
                    let params_end_byte = init_start_byte + close_paren + 1;

                    logger::debug(&format!(
                        "extract_init_parameters: __init__ range: bytes {}-{}",
                        params_start_byte, params_end_byte
                    ));

                    let rule_content = r#"id: extract_typed_param
language: python
rule:
  any:
    - kind: typed_parameter
    - kind: typed_default_parameter
"#;

                    let temp_dir = std::env::temp_dir();
                    let rule_file = temp_dir.join("r2x_param_rule.yaml");
                    let py_file = temp_dir.join("r2x_param_source.py");

                    let rule_write_ok = std::fs::write(&rule_file, rule_content).is_ok();
                    let py_write_ok = std::fs::write(&py_file, file_content).is_ok();

                    logger::debug(&format!(
                        "extract_init_parameters: wrote rule={}, py={}",
                        rule_write_ok, py_write_ok
                    ));

                    if rule_write_ok && py_write_ok {
                        let output = Command::new("ast-grep")
                            .arg("scan")
                            .arg("-r")
                            .arg(rule_file.to_str().unwrap_or(""))
                            .arg("--json")
                            .arg(py_file.to_str().unwrap_or(""))
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .spawn();

                        logger::debug("extract_init_parameters: spawned ast-grep");

                        if let Ok(child) = output {
                            if let Ok(output) = child.wait_with_output() {
                                logger::debug(&format!(
                                    "extract_init_parameters: ast-grep exit: {}",
                                    output.status.success()
                                ));
                                if let Ok(stdout_str) = String::from_utf8(output.stdout) {
                                    logger::debug(&format!(
                                        "extract_init_parameters: stdout length: {}",
                                        stdout_str.len()
                                    ));
                                    if let Ok(matches) =
                                        serde_json::from_str::<Vec<Value>>(&stdout_str)
                                    {
                                        logger::debug(&format!(
                                            "extract_init_parameters: found {} matches",
                                            matches.len()
                                        ));

                                        for match_obj in matches {
                                            if let Some(range) = match_obj.get("range") {
                                                if let (Some(start), Some(end)) = (
                                                    range
                                                        .get("byteOffset")
                                                        .and_then(|o| o.get("start"))
                                                        .and_then(|v| v.as_u64()),
                                                    range
                                                        .get("byteOffset")
                                                        .and_then(|o| o.get("end"))
                                                        .and_then(|v| v.as_u64()),
                                                ) {
                                                    let start = start as usize;
                                                    let end = end as usize;

                                                    logger::debug(&format!(
                                                        "extract_init_parameters: checking match bytes {}-{} against range {}-{}",
                                                        start, end, params_start_byte, params_end_byte
                                                    ));

                                                    if start >= params_start_byte
                                                        && end <= params_end_byte
                                                    {
                                                        if let Some(text) =
                                                            match_obj.get("text").and_then(|v| {
                                                                v.as_str()
                                                            })
                                                        {
                                                            if let Some(colon_pos) = text.find(':')
                                                            {
                                                                let name = text[..colon_pos].trim();
                                                                let rest =
                                                                    text[colon_pos + 1..].trim();

                                                                let has_default =
                                                                    rest.contains('=');
                                                                let annotation = if has_default {
                                                                    rest.split('=')
                                                                        .next()
                                                                        .unwrap_or("")
                                                                        .trim()
                                                                } else {
                                                                    rest
                                                                };

                                                                let mut param_obj =
                                                                    serde_json::Map::new();
                                                                if !annotation.is_empty() {
                                                                    param_obj.insert(
                                                                        "annotation".to_string(),
                                                                        Value::String(
                                                                            annotation
                                                                                .to_string(),
                                                                        ),
                                                                    );
                                                                }
                                                                param_obj.insert(
                                                                    "is_required".to_string(),
                                                                    Value::Bool(!has_default),
                                                                );

                                                                parameters.insert(
                                                                    name.to_string(),
                                                                    Value::Object(param_obj),
                                                                );

                                                                logger::debug(&format!(
                                                                    "extract_init_parameters: extracted param '{}' (required: {})",
                                                                    name, !has_default
                                                                ));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        let _ = std::fs::remove_file(&rule_file);
                        let _ = std::fs::remove_file(&py_file);
                    }
                }
            }
        }
    }

    Ok(parameters)
}
