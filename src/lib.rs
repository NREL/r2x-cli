//! R2X library - expose modules for testing
//!
//! This library exposes core modules needed for testing and integration.

pub mod config_manager;
pub mod errors;
pub mod logger;
pub mod package_verification;
pub mod plugin_cache;
pub mod plugin_manifest;
pub mod python_bridge;

// Re-export common error types for convenience
pub use errors::{BridgeError, ManifestError, PipelineError};
