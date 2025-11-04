use crate::logger;
use crate::plugin_cache::PluginMetadataCache;
use crate::plugin_manifest::PluginManifest;
use crate::GlobalOpts;
use colored::*;

pub fn clean_manifest(yes: bool, _opts: &GlobalOpts) -> Result<(), String> {
    let mut manifest =
        PluginManifest::load().map_err(|e| format!("Failed to load manifest: {}", e))?;

    if manifest.is_empty() {
        logger::warn("Manifest is empty.");
        return Ok(());
    }

    let total = manifest.plugins.len();
    logger::debug(&format!("Manifest has {} plugin entries.", total));

    if !yes {
        println!("To actually clean, run with --yes flag.");
        return Ok(());
    }

    manifest.plugins.clear();
    manifest
        .save()
        .map_err(|e| format!("Failed to save manifest: {}", e))?;

    // Also clear the metadata cache when cleaning manifest
    if let Err(e) = PluginMetadataCache::clear() {
        logger::debug(&format!("Note: Failed to clear metadata cache: {}", e));
    }

    println!("{}", format!("Removed {} plugin(s)", total).dimmed());

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_clean_manifest_empty() {
        // Test cleaning empty manifest
    }

    #[test]
    fn test_clean_manifest_with_plugins() {
        // Test cleaning manifest with plugins
    }

    #[test]
    fn test_clean_manifest_requires_yes_flag() {
        // Test that --yes flag is required
    }
}
