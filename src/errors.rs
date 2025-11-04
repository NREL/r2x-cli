//! Centralized error types for the r2x project
//!
//! This module defines all error types used across the project,
//! providing a unified error handling interface.

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during Python bridge operations
#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Python error: {0}")]
    Python(String),

    #[error("Failed to import module '{0}': {1}")]
    Import(String, String),

    #[error("Python venv not found or invalid at: {0}")]
    VenvNotFound(PathBuf),

    #[error("r2x-core is not installed in the Python environment")]
    R2XCoreNotInstalled,

    #[error("Failed to serialize/deserialize data: {0}")]
    Serialization(String),

    #[error("Failed to initialize Python interpreter: {0}")]
    Initialization(String),

    #[error("Plugin '{0}' not found")]
    PluginNotFound(String),

    #[error("Invalid entry point format: {0}")]
    InvalidEntryPoint(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

impl From<pyo3::PyErr> for BridgeError {
    fn from(err: pyo3::PyErr) -> Self {
        BridgeError::Python(format!("{}", err))
    }
}

/// Errors that can occur during pipeline configuration operations
#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to parse pipeline YAML: {0}")]
    Parse(#[from] serde_yaml::Error),

    #[error("Variable '{0}' not found in variables section")]
    VariableNotFound(String),

    #[error("Pipeline '{0}' not found in YAML")]
    PipelineNotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Errors that can occur during plugin manifest operations
#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to parse manifest: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Failed to serialize manifest: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("Invalid plugin: {0}")]
    InvalidPlugin(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_error_display() {
        let err = BridgeError::PluginNotFound("test-plugin".to_string());
        assert_eq!(err.to_string(), "Plugin 'test-plugin' not found");
    }

    #[test]
    fn test_pipeline_error_display() {
        let err = PipelineError::PipelineNotFound("test-pipeline".to_string());
        assert_eq!(
            err.to_string(),
            "Pipeline 'test-pipeline' not found in YAML"
        );
    }

    #[test]
    fn test_manifest_error_display() {
        let err = ManifestError::InvalidPlugin("test".to_string());
        assert_eq!(err.to_string(), "Invalid plugin: test");
    }
}
