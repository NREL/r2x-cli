//! Core library for the r2x-cli command and runtime binaries.
//!
//! The `r2x` command manages plugins, configuration, manifests, pipelines, and
//! managed Python runtime setup for the r2x ecosystem. This crate exposes the
//! shared command modules and domain operations used by both binaries and
//! integration tests.
//!
//! The installed package provides the `r2x` launcher and `r2x-runtime` payload.

pub mod commands;
pub mod common;
pub mod errors;
pub(crate) mod help;
mod install_source;
pub mod manifest_lookup;
pub mod package_verification;
pub mod pipeline_config;
pub mod plugins;
mod uv;

#[cfg(test)]
pub(crate) mod test_support;
