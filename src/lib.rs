//! # validate-json-schema
//!
//! A fast, ergonomic library for validating YAML and JSON files against JSON schemas.
//!
//! ## Features
//!
//! - 🔍 **Dual Format Support**: Validate both YAML and JSON input files
//! - 🌐 **Remote Schema Support**: Fetch schemas from URLs with automatic caching
//! - 🧠 **Smart Detection**: Automatic format detection based on file extension and content
//! - 🚀 **High Performance**: Built with Rust for maximum speed and safety
//! - 📋 **Detailed Errors**: Clear, actionable validation error messages
//!
//! ## Quick Start
//!
//! ```rust
//! use validate_json_schema::Validator;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create validator from schema string
//! let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
//! let validator = Validator::new(schema)?;
//!
//! // Validate YAML content
//! validator.validate_yaml("name: John")?;
//!
//! // Validate JSON content
//! validator.validate_json(r#"{"name": "John"}"#)?;
//!
//! // Auto-detect format
//! validator.validate_content("name: John")?;  // YAML
//! validator.validate_content(r#"{"name": "John"}"#)?;  // JSON
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use jsonschema::{Draft, JSONSchema};
use reqwest::blocking::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

/// Custom error types for validation operations
#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("Failed to read file: {0}")]
    FileRead(#[from] std::io::Error),

    #[error("Failed to parse YAML: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Invalid schema: {0}")]
    SchemaCompilation(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Cache directory error: {0}")]
    CacheDirectory(String),
}

/// A high-performance validator for YAML and JSON content against JSON schemas.
///
/// The validator compiles a JSON Schema once and can be reused to validate
/// multiple documents efficiently. Supports both local and remote schemas
/// with automatic caching.
///
/// # Examples
///
/// ```rust
/// use validate_json_schema::Validator;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
/// let validator = Validator::new(schema)?;
///
/// // Validate different formats
/// validator.validate_yaml("name: Alice")?;
/// validator.validate_json(r#"{"name": "Bob"}"#)?;
/// validator.validate_content("name: Charlie")?; // Auto-detects YAML
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Validator {
    schema: JSONSchema,
}

impl Validator {
    /// Create a new validator from a JSON schema string.
    ///
    /// # Errors
    ///
    /// Returns an error if the schema is invalid JSON or not a valid JSON Schema.
    pub fn new(schema_content: &str) -> Result<Self, ValidationError> {
        let schema_value: Value = serde_json::from_str(schema_content)?;
        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_value)
            .map_err(|e| ValidationError::SchemaCompilation(e.to_string()))?;

        Ok(Self { schema })
    }

    /// Create a validator from a local schema file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid JSON Schema.
    pub fn from_file<P: AsRef<Path>>(schema_path: P) -> Result<Self, ValidationError> {
        let schema_content = fs::read_to_string(schema_path)?;
        Self::new(&schema_content)
    }

    /// Create a validator from a remote schema URL.
    ///
    /// The schema will be downloaded and cached locally for future use.
    ///
    /// # Errors
    ///
    /// Returns an error if the URL is invalid, the request fails, or the
    /// response is not valid JSON Schema.
    pub fn from_url(schema_url: &str) -> Result<Self, ValidationError> {
        let schema_content = fetch_and_cache_schema(schema_url)?;
        Self::new(&schema_content)
    }

    /// Create a validator from either a local file path or remote URL.
    ///
    /// Automatically detects whether the input is a URL or file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the schema cannot be loaded or is invalid.
    pub fn from_schema_input(schema_input: &str) -> Result<Self, ValidationError> {
        if is_url(schema_input) {
            Self::from_url(schema_input)
        } else {
            Self::from_file(schema_input)
        }
    }

    /// Validate YAML content against the schema.
    ///
    /// # Errors
    ///
    /// Returns an error if the YAML is malformed or fails validation.
    pub fn validate_yaml(&self, yaml_content: &str) -> Result<(), ValidationError> {
        let yaml_value: Value = serde_yaml::from_str(yaml_content)?;
        self.validate_value(&yaml_value)
    }

    /// Validate JSON content against the schema.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is malformed or fails validation.
    pub fn validate_json(&self, json_content: &str) -> Result<(), ValidationError> {
        let json_value: Value = serde_json::from_str(json_content)?;
        self.validate_value(&json_value)
    }

    /// Validate content with automatic format detection.
    ///
    /// Detects JSON (starts with `{` or `[`) vs YAML and validates accordingly.
    ///
    /// # Errors
    ///
    /// Returns an error if the content is malformed or fails validation.
    pub fn validate_content(&self, content: &str) -> Result<(), ValidationError> {
        let trimmed = content.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            self.validate_json(content)
        } else {
            self.validate_yaml(content)
        }
    }

    /// Validate a YAML file against the schema.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, is malformed, or fails validation.
    pub fn validate_yaml_file<P: AsRef<Path>>(&self, yaml_path: P) -> Result<(), ValidationError> {
        let yaml_content = fs::read_to_string(yaml_path)?;
        self.validate_yaml(&yaml_content)
    }

    /// Validate a file with automatic format detection.
    ///
    /// Supports `.json`, `.yaml`, `.yml` extensions with fallback to content-based detection.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, is malformed, or fails validation.
    pub fn validate_file<P: AsRef<Path>>(&self, file_path: P) -> Result<(), ValidationError> {
        let path = file_path.as_ref();
        let content = fs::read_to_string(path)?;

        // Try extension-based detection first
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            match extension.to_lowercase().as_str() {
                "json" => return self.validate_json(&content),
                "yaml" | "yml" => return self.validate_yaml(&content),
                _ => {} // Fall through to content-based detection
            }
        }

        // Fall back to content-based detection
        self.validate_content(&content)
    }

    /// Internal method to validate a serde_json::Value against the schema.
    fn validate_value(&self, value: &Value) -> Result<(), ValidationError> {
        if let Err(errors) = self.schema.validate(value) {
            let error_messages: Vec<String> = errors
                .map(|error| {
                    let path = if error.instance_path.to_string().is_empty() {
                        "root".to_string()
                    } else {
                        error.instance_path.to_string()
                    };
                    format!("{}: {}", path, error)
                })
                .collect();

            let detailed_message = match error_messages.len() {
                1 => error_messages[0].clone(),
                n => format!("{} validation errors: {}", n, error_messages.join("; ")),
            };

            return Err(ValidationError::ValidationFailed(detailed_message));
        }

        Ok(())
    }
}

/// Check if a string is a URL (starts with http:// or https://).
fn is_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://")
}

/// Get the cache directory for schemas.
fn get_cache_dir() -> Result<PathBuf, ValidationError> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| {
            ValidationError::CacheDirectory("Could not determine cache directory".to_string())
        })?
        .join("validate-json-schema")
        .join("schemas");

    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).map_err(|e| {
            ValidationError::CacheDirectory(format!("Failed to create cache directory: {}", e))
        })?;
    }

    Ok(cache_dir)
}

/// Generate a cache filename from a URL using SHA-256 hash.
fn get_cache_filename(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = hasher.finalize();
    format!("{}.json", hex::encode(hash))
}

/// Fetch a schema from a URL and cache it locally.
fn fetch_and_cache_schema(url: &str) -> Result<String, ValidationError> {
    // Validate URL
    let _parsed_url = Url::parse(url)?;

    let cache_dir = get_cache_dir()?;
    let cache_filename = get_cache_filename(url);
    let cache_path = cache_dir.join(cache_filename);

    // Check if cached version exists
    if cache_path.exists() {
        return Ok(fs::read_to_string(cache_path)?);
    }

    // Fetch from remote
    let client = Client::builder()
        .user_agent("validate-json-schema/0.1.0")
        .timeout(Duration::from_secs(30))
        .build()?;

    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(ValidationError::ValidationFailed(format!(
            "HTTP {}: Failed to fetch schema from {}",
            response.status(),
            url
        )));
    }

    let schema_content = response.text()?;

    // Validate that it's valid JSON before caching
    let _: Value = serde_json::from_str(&schema_content)?;

    // Cache the schema
    fs::write(&cache_path, &schema_content)?;

    Ok(schema_content)
}

/// Clear the schema cache directory.
///
/// # Errors
///
/// Returns an error if the cache directory cannot be accessed or removed.
pub fn clear_schema_cache() -> Result<(), ValidationError> {
    let cache_dir = get_cache_dir()?;
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)?;
    }
    Ok(())
}

// Convenience functions for one-off validations

/// Validate YAML content against a JSON schema string.
pub fn validate_yaml_with_schema(
    yaml_content: &str,
    schema_content: &str,
) -> Result<(), ValidationError> {
    let validator = Validator::new(schema_content)?;
    validator.validate_yaml(yaml_content)
}

/// Validate JSON content against a JSON schema string.
pub fn validate_json_with_schema(
    json_content: &str,
    schema_content: &str,
) -> Result<(), ValidationError> {
    let validator = Validator::new(schema_content)?;
    validator.validate_json(json_content)
}

/// Validate content (auto-detecting format) against a JSON schema string.
pub fn validate_content_with_schema(
    content: &str,
    schema_content: &str,
) -> Result<(), ValidationError> {
    let validator = Validator::new(schema_content)?;
    validator.validate_content(content)
}

/// Validate a YAML file against a schema file.
pub fn validate_yaml_file_with_schema_file<P1: AsRef<Path>, P2: AsRef<Path>>(
    yaml_path: P1,
    schema_path: P2,
) -> Result<(), ValidationError> {
    let validator = Validator::from_file(schema_path)?;
    validator.validate_yaml_file(yaml_path)
}

/// Validate a file (auto-detecting format) against a schema file.
pub fn validate_file_with_schema_file<P1: AsRef<Path>, P2: AsRef<Path>>(
    file_path: P1,
    schema_path: P2,
) -> Result<(), ValidationError> {
    let validator = Validator::from_file(schema_path)?;
    validator.validate_file(file_path)
}

/// Validate a YAML file against a schema (file path or URL).
pub fn validate_yaml_file_with_schema_input<P: AsRef<Path>>(
    yaml_path: P,
    schema_input: &str,
) -> Result<(), ValidationError> {
    let validator = Validator::from_schema_input(schema_input)?;
    validator.validate_yaml_file(yaml_path)
}

/// Validate a file (auto-detecting format) against a schema (file path or URL).
pub fn validate_file_with_schema_input<P: AsRef<Path>>(
    file_path: P,
    schema_input: &str,
) -> Result<(), ValidationError> {
    let validator = Validator::from_schema_input(schema_input)?;
    validator.validate_file(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_validation() {
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
        let validator = Validator::new(schema).unwrap();

        // Valid cases
        assert!(validator.validate_yaml("name: Alice").is_ok());
        assert!(validator.validate_json(r#"{"name": "Bob"}"#).is_ok());
        assert!(validator.validate_content("name: Charlie").is_ok());
        assert!(validator.validate_content(r#"{"name": "David"}"#).is_ok());

        // Invalid cases
        assert!(validator.validate_yaml("name: 123").is_err());
        assert!(validator.validate_json(r#"{"name": 123}"#).is_err());
    }

    #[test]
    fn test_validator_reuse() {
        let schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0}
            },
            "required": ["name"]
        }"#;

        let validator = Validator::new(schema).unwrap();

        // Test multiple YAML validations
        assert!(validator.validate_yaml("name: Alice\nage: 30").is_ok());
        assert!(validator.validate_yaml("name: Bob").is_ok());
        assert!(validator.validate_yaml("age: 25").is_err()); // Missing required name

        // Test multiple JSON validations
        assert!(validator
            .validate_json(r#"{"name": "Alice", "age": 30}"#)
            .is_ok());
        assert!(validator.validate_json(r#"{"name": "Bob"}"#).is_ok());
        assert!(validator.validate_json(r#"{"age": 25}"#).is_err()); // Missing required name

        // Test content auto-detection
        assert!(validator.validate_content(r#"{"name": "Charlie"}"#).is_ok()); // JSON
        assert!(validator.validate_content("name: David").is_ok()); // YAML
    }

    #[test]
    fn test_json_validation() {
        let schema = r#"{"type": "object", "properties": {"count": {"type": "integer"}}}"#;
        let validator = Validator::new(schema).unwrap();

        // Valid JSON
        assert!(validator.validate_json(r#"{"count": 42}"#).is_ok());
        assert!(validator.validate_json("{}").is_ok());

        // Invalid JSON structure
        assert!(validator
            .validate_json(r#"{"count": "not a number"}"#)
            .is_err());

        // Malformed JSON
        assert!(validator.validate_json(r#"{"count": 42"#).is_err()); // Missing closing brace
    }

    #[test]
    fn test_content_auto_detection() {
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
        let validator = Validator::new(schema).unwrap();

        // JSON detection (starts with {)
        assert!(validator.validate_content(r#"{"name": "test"}"#).is_ok());

        // JSON array detection (starts with [)
        let array_schema = r#"{"type": "array", "items": {"type": "string"}}"#;
        let array_validator = Validator::new(array_schema).unwrap();
        assert!(array_validator
            .validate_content(r#"["item1", "item2"]"#)
            .is_ok());

        // YAML detection (doesn't start with { or [)
        assert!(validator.validate_content("name: test").is_ok());

        // Test with whitespace
        assert!(validator
            .validate_content("  \n  {\"name\": \"test\"}")
            .is_ok());
    }

    #[test]
    fn test_is_url() {
        assert!(is_url("https://example.com/schema.json"));
        assert!(is_url("http://example.com/schema.json"));
        assert!(!is_url("schema.json"));
        assert!(!is_url("/path/to/schema.json"));
        assert!(!is_url("file://schema.json"));
    }

    #[test]
    fn test_cache_filename_generation() {
        let url1 = "https://example.com/schema.json";
        let url2 = "https://example.com/other.json";

        let filename1 = get_cache_filename(url1);
        let filename2 = get_cache_filename(url2);

        // Should be different
        assert_ne!(filename1, filename2);

        // Should be consistent
        assert_eq!(filename1, get_cache_filename(url1));

        // Should end with .json
        assert!(filename1.ends_with(".json"));
        assert!(filename2.ends_with(".json"));
    }
}
