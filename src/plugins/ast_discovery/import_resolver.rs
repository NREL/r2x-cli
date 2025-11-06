//! Resolve Python imports to module paths
//!
//! Parses import statements and builds a mapping of symbol names
//! to their module paths.

use crate::errors::BridgeError;
use std::collections::HashMap;

/// Import mapping from short name to full module path
#[derive(Debug, Clone)]
pub struct ImportMap {
    // Maps "ReEDSParser" -> ("r2x_reeds.parser", "ReEDSParser")
    pub symbols: HashMap<String, (String, String)>,
}

/// Build import map from Python import statements
pub fn build_import_map(func_content: &str) -> Result<ImportMap, BridgeError> {
    let mut symbols = HashMap::new();

    // Parse lines looking for: from MODULE import NAME[, NAME]
    for line in func_content.lines() {
        let line = line.trim();

        // Skip non-import lines
        if !line.starts_with("from ") {
            continue;
        }

        // Parse: from r2x_reeds.parser import ReEDSParser
        if let Some(import_idx) = line.find(" import ") {
            let module_part = &line[5..import_idx]; // Skip "from "
            let imports_part = &line[import_idx + 8..]; // Skip " import "

            // Handle comma-separated imports: import A, B, C
            for import_spec in imports_part.split(',') {
                let import_name = import_spec.trim();

                // Handle "import X as Y" - for now just use the first name
                let actual_name = if let Some(as_idx) = import_name.find(" as ") {
                    &import_name[as_idx + 4..]
                } else {
                    import_name
                };

                symbols.insert(
                    actual_name.to_string(),
                    (module_part.to_string(), actual_name.to_string()),
                );
            }
        }
    }

    Ok(ImportMap { symbols })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_map_single() {
        let content = "from r2x_reeds.parser import ReEDSParser";
        let map = build_import_map(content).unwrap();
        assert_eq!(map.symbols.len(), 1);
        assert!(map.symbols.contains_key("ReEDSParser"));
    }

    #[test]
    fn test_import_map_multiple() {
        let content = "from r2x_reeds.parser import ReEDSParser, Helper";
        let map = build_import_map(content).unwrap();
        assert_eq!(map.symbols.len(), 2);
        assert!(map.symbols.contains_key("ReEDSParser"));
        assert!(map.symbols.contains_key("Helper"));
    }

    #[test]
    fn test_import_map_empty_lines() {
        let content = "\n\nfrom r2x_reeds.parser import ReEDSParser\n";
        let map = build_import_map(content).unwrap();
        assert_eq!(map.symbols.len(), 1);
    }
}
