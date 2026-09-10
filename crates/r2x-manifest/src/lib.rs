//! Persistent plugin manifest for r2x-cli.
//!
//! The manifest records installed package metadata, plugin entry points,
//! configuration schemas, install relationships, and runtime bindings. This
//! crate owns the in-memory types, indexes, TOML persistence, package
//! discovery, and synchronization operations used by plugin management.
//!
//! The current representation uses `Arc<str>` for shared strings, `SmallVec`
//! for compact collections, precomputed hashes, and indexed package and plugin
//! lookups.

pub mod errors;
pub mod manifest;
pub mod package_discovery;
pub mod runtime;
#[cfg(test)]
mod sync;
pub mod types;
