# validate-json-schema

A fast, ergonomic CLI tool and Rust library for validating files against JSON schemas. Supports both local schema files and remote schema URLs with automatic caching.

## Supported file types

- YAML
- JSON

## Instructions

### Installation

```bash
# Install from source
git clone https://github.com/yourusername/validate-json-schema
cd validate-json-schema
cargo install --path .
```

### Basic Usage

```bash
# Validate a YAML file against a local schema
validate-json-schema data.yml schema.json

# Validate a JSON file against a local schema
validate-json-schema data.json schema.json

# Validate against a remote schema (automatically cached)
validate-json-schema data.yml https://json.schemastore.org/package.json

# Verbose output with detailed information
validate-json-schema data.yml schema.json --verbose

# Clear the schema cache
validate-json-schema clear-cache
```

## Supported Input Formats

The tool automatically detects and supports:

- **YAML files** (`.yml`, `.yaml` extensions)
- **JSON files** (`.json` extension)
- **Auto-detection** based on file content for files without standard extensions

## 🔧 Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
validate-json-schema = "0.1.0"
```

### Basic Validation

```rust
use validate_json_schema::Validator;

// Create validator from schema
let validator = Validator::from_file("schema.json")?;

// Validate YAML content
let yaml_content = "name: John\nage: 30";
validator.validate_yaml(yaml_content)?;

// Validate JSON content
let json_content = r#"{"name": "John", "age": 30}"#;
validator.validate_json(json_content)?;

// Auto-detect format and validate
validator.validate_content(yaml_content)?;  // Detects YAML
validator.validate_content(json_content)?;  // Detects JSON

// Validate files with auto-detection
validator.validate_file("data.yml")?;   // YAML file
validator.validate_file("data.json")?;  // JSON file
```

### Remote Schema Support

```rust
use validate_json_schema::Validator;

// Load schema from URL (cached automatically)
let validator = Validator::from_url("https://json.schemastore.org/package.json")?;

// Validate package.json (YAML format)
validator.validate_yaml("name: my-package\nversion: 1.0.0")?;

// Validate package.json (JSON format)
validator.validate_json(r#"{"name": "my-package", "version": "1.0.0"}"#)?;
```

### Auto-Detection

```rust
use validate_json_schema::Validator;

// Auto-detect local vs remote schema
let validator = Validator::from_schema_input("https://example.com/schema.json")?; // Remote
let validator = Validator::from_schema_input("local-schema.json")?; // Local

// Convenience functions
use validate_json_schema::{validate_file_with_schema_input, validate_content_with_schema};

// Validate any file format against any schema source
validate_file_with_schema_input("data.yml", "https://example.com/schema.json")?;
validate_file_with_schema_input("data.json", "local-schema.json")?;

// Validate content with auto-detection
validate_content_with_schema(yaml_or_json_content, schema_content)?;
```
