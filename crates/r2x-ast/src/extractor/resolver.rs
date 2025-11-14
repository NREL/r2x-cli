use super::*;

impl PluginExtractor {
    pub(crate) fn resolve_config_fields(
        &self,
        _config_class: &str,
        _package_root: &std::path::Path,
        _package_name: &str,
    ) -> Result<Vec<r2x_manifest::ConfigField>> {
        Ok(Vec::new())
    }

    pub(crate) fn resolve_entry_parameters(
        &self,
        _entry_class: &str,
        _package_root: &std::path::Path,
        _package_name: &str,
    ) -> Result<Vec<r2x_manifest::ArgumentSpec>> {
        Ok(Vec::new())
    }
}
