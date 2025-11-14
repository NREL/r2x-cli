use super::*;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_infer_argument_type_string() {
    let extractor = PluginExtractor {
        python_file_path: PathBuf::from("test.py"),
        content: "def register_plugin(): pass".to_string(),
        import_map: HashMap::new(),
    };

    assert_eq!(extractor.infer_argument_type(r#""hello""#), "string");
    assert_eq!(extractor.infer_argument_type("'hello'"), "string");
}

#[test]
fn test_infer_argument_type_number() {
    let extractor = PluginExtractor {
        python_file_path: PathBuf::from("test.py"),
        content: "def register_plugin(): pass".to_string(),
        import_map: HashMap::new(),
    };

    assert_eq!(extractor.infer_argument_type("42"), "number");
    assert_eq!(extractor.infer_argument_type("3.14"), "float");
}

#[test]
fn test_infer_argument_type_enum() {
    let extractor = PluginExtractor {
        python_file_path: PathBuf::from("test.py"),
        content: "def register_plugin(): pass".to_string(),
        import_map: HashMap::new(),
    };

    assert_eq!(extractor.infer_argument_type("IOType.STDOUT"), "enum_value");
}

#[test]
fn test_infer_argument_type_class() {
    let extractor = PluginExtractor {
        python_file_path: PathBuf::from("test.py"),
        content: "def register_plugin(): pass".to_string(),
        import_map: HashMap::new(),
    };

    assert_eq!(
        extractor.infer_argument_type("ReEDSParser"),
        "class_reference"
    );
    assert_eq!(extractor.infer_argument_type("MyClass"), "class_reference");
}

