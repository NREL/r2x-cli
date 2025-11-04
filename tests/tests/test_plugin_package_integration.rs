//! Integration test for Plugin Package loading via Rust bridge
//!
//! Tests the new Package-based plugin registration system:
//! 1. Bridge initialization
//! 2. Loading plugin package from r2x_plugin entry point
//! 3. JSON deserialization into structs
//! 4. Verification of plugin metadata

use r2x::python_bridge::Bridge;

#[test]
fn test_bridge_initialization() {
    // Test that bridge can be initialized
    let bridge = Bridge::get();
    assert!(bridge.is_ok(), "Bridge should initialize successfully");
}

#[test]
fn test_load_reeds_plugin_package() {
    let bridge = Bridge::get().expect("Bridge should initialize");

    // Test loading r2x-reeds plugin package
    let result = bridge.load_plugin_package("reeds");
    assert!(
        result.is_ok(),
        "Should successfully load reeds plugin package"
    );

    let json_str = result.unwrap();
    assert!(!json_str.is_empty(), "Package JSON should not be empty");
    assert!(
        json_str.len() > 100,
        "Package JSON should contain substantial data"
    );
}

#[test]
fn test_reeds_package_json_structure() {
    let bridge = Bridge::get().expect("Bridge should initialize");
    let json_str = bridge
        .load_plugin_package("reeds")
        .expect("Should load package");

    // Parse JSON
    let value: serde_json::Value = serde_json::from_str(&json_str).expect("JSON should be valid");

    // Verify structure
    assert_eq!(value["name"], "reeds", "Package name should be 'reeds'");

    let plugins = value["plugins"]
        .as_array()
        .expect("Should have plugins array");
    assert!(
        !plugins.is_empty(),
        "Package should contain at least one plugin"
    );

    // Check first plugin (should be ReEDSParser)
    let first = &plugins[0];
    assert_eq!(
        first["name"], "reeds",
        "First plugin should be named 'reeds'"
    );
    assert_eq!(
        first["plugin_type"], "class",
        "Parser should be of type 'class'"
    );

    // Check callable metadata
    let obj = &first["obj"];
    assert_eq!(
        obj["module"], "r2x_reeds.parser",
        "Module should be r2x_reeds.parser"
    );
    assert_eq!(
        obj["name"], "ReEDSParser",
        "Callable name should be ReEDSParser"
    );
    assert_eq!(obj["type"], "class", "Should be a class");

    // Check parameters exist
    assert!(
        obj["parameters"].is_object(),
        "Should have parameters object"
    );

    // Check config class exists
    assert!(
        first["config"].is_object(),
        "Parser should have config metadata"
    );
    assert_eq!(
        first["config"]["name"], "ReEDSConfig",
        "Config should be ReEDSConfig"
    );
}

#[test]
fn test_reeds_package_contains_multiple_plugins() {
    let bridge = Bridge::get().expect("Bridge should initialize");
    let result = bridge.load_plugin_package("reeds");
    if let Err(ref e) = result {
        println!(
            "Error in test_reeds_package_contains_multiple_plugins: {}",
            e
        );
    }
    let json_str = result.expect("Should load package");

    let value: serde_json::Value = serde_json::from_str(&json_str).expect("JSON should be valid");

    // Should have parser, upgrader, and several sysmod plugins
    let plugins = value["plugins"].as_array().expect("Should have plugins");
    assert!(
        plugins.len() > 3,
        "Package should have multiple plugins (parser, upgrader, sysmods), found {}",
        plugins.len()
    );

    // Collect plugin types
    let types: Vec<_> = plugins
        .iter()
        .filter_map(|p| p["plugin_type"].as_str())
        .collect();

    // Should have both class (parser/upgrader) and function (sysmods) plugins
    assert!(
        types.contains(&"class"),
        "Should have at least one class-type plugin"
    );
    assert!(
        types.contains(&"function"),
        "Should have at least one function-type plugin (sysmod)"
    );
}

#[test]
fn test_reeds_sysmod_plugins() {
    let bridge = Bridge::get().expect("Bridge should initialize");
    let result = bridge.load_plugin_package("reeds");
    if let Err(ref e) = result {
        println!("Error in test_reeds_sysmod_plugins: {}", e);
    }
    let json_str = result.expect("Should load package");

    let value: serde_json::Value = serde_json::from_str(&json_str).expect("JSON should be valid");

    let plugins = value["plugins"].as_array().expect("Should have plugins");

    // Find function-type plugins (sysmods)
    let sysmods: Vec<_> = plugins
        .iter()
        .filter(|p| p["plugin_type"] == "function")
        .collect();

    assert!(
        !sysmods.is_empty(),
        "Should have at least one sysmod plugin"
    );

    // Check first sysmod
    let sysmod = sysmods[0];
    let obj = &sysmod["obj"];

    assert!(obj["module"].is_string(), "Sysmod should have module");
    assert!(obj["name"].is_string(), "Sysmod should have name");
    assert_eq!(obj["type"], "function", "Sysmod should be function type");
}

#[test]
fn test_package_metadata() {
    let bridge = Bridge::get().expect("Bridge should initialize");
    let result = bridge.load_plugin_package("reeds");
    if let Err(ref e) = result {
        println!("Error in test_package_metadata: {}", e);
    }
    let json_str = result.expect("Should load package");

    let value: serde_json::Value = serde_json::from_str(&json_str).expect("JSON should be valid");

    // Check metadata exists
    assert!(
        value["metadata"].is_object(),
        "Package should have metadata"
    );
    assert!(
        value["metadata"]["version"].is_string(),
        "Should have version in metadata"
    );
}

#[test]
fn test_callable_has_parameters() {
    let bridge = Bridge::get().expect("Bridge should initialize");
    let result = bridge.load_plugin_package("reeds");
    if let Err(ref e) = result {
        println!("Error in test_callable_has_parameters: {}", e);
    }
    let json_str = result.expect("Should load package");

    let value: serde_json::Value = serde_json::from_str(&json_str).expect("JSON should be valid");

    // Check first plugin's callable
    let obj = &value["plugins"][0]["obj"];
    let params = &obj["parameters"];

    assert!(params.is_object(), "Callable should have parameters object");
    assert!(
        !params.as_object().unwrap().is_empty(),
        "Should have at least one parameter"
    );

    // Check parameter structure
    for (param_name, param_info) in params.as_object().unwrap() {
        assert!(
            param_info["is_required"].is_boolean(),
            "Parameter {} should have is_required flag",
            param_name
        );
        assert!(
            param_info.get("annotation").is_some(),
            "Parameter {} should have annotation",
            param_name
        );
    }
}

#[test]
fn test_nonexistent_package() {
    let bridge = Bridge::get().expect("Bridge should initialize");

    // Try to load non-existent package
    let result = bridge.load_plugin_package("nonexistent_package_xyz");

    assert!(
        result.is_err(),
        "Should fail when loading non-existent package"
    );
}
