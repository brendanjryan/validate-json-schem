use clap::{Arg, Command};
use std::process;
use validate_json_schema::{clear_schema_cache, validate_file_with_schema_input, ValidationError};

fn main() {
    let matches = Command::new("validate-json-schema")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Your Name <your.email@example.com>")
        .about("Validates YAML and JSON files against JSON schemas")
        .long_about(
            "A fast, ergonomic tool for validating YAML and JSON files against JSON schemas.\n\
             Supports both local schema files and remote schema URLs with automatic caching.\n\
             Automatically detects file format based on extension and content.",
        )
        .arg(
            Arg::new("file")
                .help("The YAML or JSON file to validate")
                .long_help("Path to the YAML or JSON file to validate. Format is auto-detected.")
                .required(false)
                .index(1)
                .value_name("FILE"),
        )
        .arg(
            Arg::new("schema")
                .help("The JSON schema file path or URL")
                .long_help(
                    "Path to a local JSON schema file or URL to a remote schema.\n\
                     Remote schemas are automatically cached for faster subsequent validations.",
                )
                .required(false)
                .index(2)
                .value_name("SCHEMA"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .long_help("Show detailed information about the validation process")
                .action(clap::ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("clear-cache")
                .about("Clear the schema cache")
                .long_about("Remove all cached remote schemas from the local cache directory"),
        )
        .get_matches();

    // Handle subcommands
    if matches.subcommand_matches("clear-cache").is_some() {
        handle_clear_cache();
        return;
    }

    // Handle main validation command
    let file_path = matches.get_one::<String>("file");
    let schema_input = matches.get_one::<String>("schema");

    match (file_path, schema_input) {
        (Some(file), Some(schema)) => {
            let verbose = matches.get_flag("verbose");
            handle_validation(file, schema, verbose);
        }
        _ => {
            eprintln!("Error: Both FILE and SCHEMA arguments are required for validation");
            eprintln!("Usage: validate-json-schema <FILE> <SCHEMA>");
            eprintln!("       validate-json-schema clear-cache");
            eprintln!("Try 'validate-json-schema --help' for more information.");
            process::exit(1);
        }
    }
}

fn handle_clear_cache() {
    match clear_schema_cache() {
        Ok(()) => {
            println!("Schema cache cleared successfully");
        }
        Err(e) => {
            eprintln!("Error clearing cache: {}", e);
            process::exit(1);
        }
    }
}

fn handle_validation(file_path: &str, schema_input: &str, verbose: bool) {
    if verbose {
        print_verbose_info(file_path, schema_input);
    }

    match validate_file_with_schema_input(file_path, schema_input) {
        Ok(()) => {
            if verbose {
                println!("Validation successful!");
            } else {
                println!("Valid");
            }
        }
        Err(ValidationError::ValidationFailed(msg)) => {
            eprintln!("Validation failed: {}", msg);
            process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn print_verbose_info(file_path: &str, schema_input: &str) {
    // Schema source info
    if schema_input.starts_with("http://") || schema_input.starts_with("https://") {
        println!("Using remote schema: {}", schema_input);
    } else {
        println!("Using local schema: {}", schema_input);
    }

    println!("Validating file: {}", file_path);

    // File type detection
    let file_type = detect_file_type(file_path);
    println!("File type: {}", file_type);
}

fn detect_file_type(file_path: &str) -> &'static str {
    if file_path.ends_with(".json") {
        "JSON"
    } else if file_path.ends_with(".yaml") || file_path.ends_with(".yml") {
        "YAML"
    } else {
        "Auto-detected"
    }
}
