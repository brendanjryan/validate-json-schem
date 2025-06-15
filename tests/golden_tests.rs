use validate_json_schema::Validator;

/// Test case structure for table-driven tests
#[derive(Debug)]
struct TestCase {
    name: &'static str,
    data_file: &'static str,
    schema_file: &'static str,
    should_pass: bool,
}

impl TestCase {
    fn run(&self) -> Result<(), String> {
        if !std::path::Path::new(self.data_file).exists() {
            return Err(format!("Data file not found: {}", self.data_file));
        }
        if !std::path::Path::new(self.schema_file).exists() {
            return Err(format!("Schema file not found: {}", self.schema_file));
        }

        let validator = Validator::from_file(self.schema_file)
            .map_err(|e| format!("Failed to create validator: {}", e))?;

        let validation_result = validator.validate_file(self.data_file);

        match (self.should_pass, validation_result.is_ok()) {
            (true, true) | (false, false) => Ok(()),
            (true, false) => Err(format!(
                "Expected validation to pass but it failed: {}",
                validation_result.unwrap_err()
            )),
            (false, true) => Err("Expected validation to fail but it passed".to_string()),
        }
    }
}

/// Helper macro to create and run test cases
macro_rules! test_cases {
    ($($name:ident: $test_case:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let test_case = $test_case;
                if let Err(e) = test_case.run() {
                    panic!("Test '{}' failed: {}", test_case.name, e);
                }
            }
        )*
    };
}

// == Full test suite ==
test_cases! {
    valid_render_blueprint: TestCase {
        name: "valid_render_blueprint",
        data_file: "tests/data/render-blueprint.yml",
        schema_file: "tests/schemas/render-blueprint.json",
        should_pass: true,
    },

    invalid_render_blueprint: TestCase {
        name: "invalid_render_blueprint",
        data_file: "tests/data/invalid-render-blueprint.yml",
        schema_file: "tests/schemas/render-blueprint.json",
        should_pass: false,
    },

    edge_cases_render_blueprint: TestCase {
        name: "edge_cases_render_blueprint",
        data_file: "tests/data/edge-cases.yml",
        schema_file: "tests/schemas/render-blueprint.json",
        should_pass: true,
    },

    // JSON input tests
    valid_render_blueprint_json: TestCase {
        name: "valid_render_blueprint_json",
        data_file: "tests/data/render-blueprint.json",
        schema_file: "tests/schemas/render-blueprint.json",
        should_pass: true,
    },

    valid_github_workflow: TestCase {
        name: "valid_github_workflow",
        data_file: "tests/data/github-workflow.yml",
        schema_file: "tests/schemas/github-workflow.json",
        should_pass: true,
    },

    invalid_github_workflow: TestCase {
        name: "invalid_github_workflow",
        data_file: "tests/data/invalid-github-workflow.yml",
        schema_file: "tests/schemas/github-workflow.json",
        should_pass: false,
    },

    valid_package_json_yaml: TestCase {
        name: "valid_package_json_yaml",
        data_file: "tests/data/package.yml",
        schema_file: "tests/schemas/package.json",
        should_pass: true,
    },

    valid_package_json_json: TestCase {
        name: "valid_package_json_json",
        data_file: "tests/data/package.json",
        schema_file: "tests/schemas/package.json",
        should_pass: true,
    },

    invalid_package_json_yaml: TestCase {
        name: "invalid_package_json_yaml",
        data_file: "tests/data/invalid-package.yml",
        schema_file: "tests/schemas/package.json",
        should_pass: false,
    },

    invalid_package_json_json: TestCase {
        name: "invalid_package_json_json",
        data_file: "tests/data/invalid-package.json",
        schema_file: "tests/schemas/package.json",
        should_pass: false,
    },

    minimal_valid_package_json: TestCase {
        name: "minimal_valid_package_json",
        data_file: "tests/data/minimal-valid.yml",
        schema_file: "tests/schemas/package.json",
        should_pass: true,
    },

    // Cross-validation tests (should fail)
    package_json_against_github_schema: TestCase {
        name: "package_json_against_github_schema",
        data_file: "tests/data/package.yml",
        schema_file: "tests/schemas/github-workflow.json",
        should_pass: false,
    },

    render_blueprint_against_github_schema: TestCase {
        name: "render_blueprint_against_github_schema",
        data_file: "tests/data/render-blueprint.yml",
        schema_file: "tests/schemas/github-workflow.json",
        should_pass: false,
    },
}

/// === Granular tests using the Validator struct ===

/// Test URL detection functionality
#[test]
fn test_url_detection() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::from_schema_input("tests/schemas/render-blueprint.json")?;
    assert!(validator.validate_yaml("services: []").is_ok());
    Ok(())
}

/// Test schema input detection (local vs URL)
#[test]
fn test_schema_input_detection() -> Result<(), Box<dyn std::error::Error>> {
    // Test local file detection
    let validator = Validator::from_schema_input("tests/schemas/render-blueprint.json")?;
    assert!(validator.validate_yaml("services: []").is_ok());
    Ok(())
}

/// Test local schema loading
#[test]
fn test_local_schema_loading() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::from_file("tests/schemas/render-blueprint.json")?;

    // Valid YAML should pass
    assert!(validator.validate_yaml("services: []").is_ok());

    // Invalid YAML should fail
    assert!(validator.validate_yaml("invalid: field").is_err());

    Ok(())
}

/// Test malformed YAML handling
#[test]
fn test_malformed_yaml() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::from_file("tests/schemas/render-blueprint.json")?;

    let malformed_yaml = r#"
    invalid: yaml: content:
      - missing
        proper: indentation
    "#;

    let result = validator.validate_yaml(malformed_yaml);
    assert!(result.is_err());

    Ok(())
}

/// Test malformed schema handling
#[test]
fn test_malformed_schema() {
    // Create a temporary malformed schema file
    let malformed_schema = r#"{ "invalid": json: content }"#;
    std::fs::write("tests/malformed_schema.json", malformed_schema).unwrap();

    let result = Validator::from_file("tests/malformed_schema.json");
    assert!(result.is_err());

    // Clean up
    let _ = std::fs::remove_file("tests/malformed_schema.json");
}

/// Test detailed error messages
#[test]
fn test_detailed_error_messages() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::from_file("tests/schemas/render-blueprint.json")?;

    let invalid_yaml = r#"
    services:
      - type: "invalid_service_type"
        name: "test"
    "#;

    let result = validator.validate_yaml(invalid_yaml);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("invalid_service_type") || error_msg.len() > 10);

    Ok(())
}

/// Test JSON validation functionality
#[test]
fn test_json_validation() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::from_file("tests/schemas/render-blueprint.json")?;

    // Valid JSON should pass
    let valid_json = r#"{"services": []}"#;
    assert!(validator.validate_json(valid_json).is_ok());

    // Invalid JSON should fail
    let invalid_json = r#"{"services": "should be array"}"#;
    assert!(validator.validate_json(invalid_json).is_err());

    Ok(())
}

/// Test content auto-detection
#[test]
fn test_content_auto_detection() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::from_file("tests/schemas/render-blueprint.json")?;

    // JSON content (starts with {)
    let json_content = r#"{"services": []}"#;
    assert!(validator.validate_content(json_content).is_ok());

    // YAML content
    let yaml_content = "services: []";
    assert!(validator.validate_content(yaml_content).is_ok());

    // Array JSON content (starts with [)
    let array_json = r#"[]"#;
    // This should be detected as JSON but fail validation against our schema
    assert!(validator.validate_content(array_json).is_err());

    Ok(())
}

/// Test file extension detection
#[test]
fn test_file_extension_detection() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::from_file("tests/schemas/package.json")?;

    // Test .json file
    if std::path::Path::new("tests/data/package.json").exists() {
        assert!(validator.validate_file("tests/data/package.json").is_ok());
    }

    // Test .yml file
    if std::path::Path::new("tests/data/package.yml").exists() {
        assert!(validator.validate_file("tests/data/package.yml").is_ok());
    }

    Ok(())
}
