//! Python runtime bridge for invoking r2x plugins.
//!
//! This crate initializes PyO3 from r2x-cli's configured virtual environment,
//! resolves `PYTHONHOME` and `site-packages`, loads plugin packages, and invokes
//! plugin entry points with serialized arguments and artifacts.
//!
//! Plugin discovery is delegated to `r2x-ast`; this crate owns runtime
//! execution and Python-side logging configuration.

pub mod errors;
pub mod plugin_invoker;
mod plugin_kwargs;
mod plugin_regular;
mod plugin_upgrader;
pub mod python_bridge;
pub mod utils;

#[cfg(test)]
mod tests {
    #[test]
    fn test_bridge_module_exports() {
        // Verify that key types are exported
        // Bridge and configure_python_venv should be publicly accessible
    }
}
