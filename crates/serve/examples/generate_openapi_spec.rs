//! Generate OpenAPI specification files
//!
//! This example generates the OpenAPI v3 specification in both JSON and YAML formats.
//! The specifications are written to the docs/reference/ directory.
//!
//! # Usage
//!
//! ```bash
//! cargo run -p xze-serve --features openapi --example generate_openapi_spec
//! ```
//!
//! This will generate:
//! - docs/reference/openapi_v1.json
//! - docs/reference/openapi_v1.yaml

#[cfg(feature = "openapi")]
use xze_serve::api::v1::openapi::{get_openapi_json, get_openapi_yaml};

#[cfg(feature = "openapi")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::Path;

    println!("Generating OpenAPI specification files...");

    // Ensure docs/reference directory exists
    let docs_dir = Path::new("docs/reference");
    fs::create_dir_all(docs_dir)?;

    // Generate JSON specification
    let json_spec = get_openapi_json()?;
    let json_path = docs_dir.join("openapi_v1.json");
    fs::write(&json_path, json_spec)?;
    println!("✓ Generated: {}", json_path.display());

    // Generate YAML specification
    let yaml_spec = get_openapi_yaml()?;
    let yaml_path = docs_dir.join("openapi_v1.yaml");
    fs::write(&yaml_path, yaml_spec)?;
    println!("✓ Generated: {}", yaml_path.display());

    println!("\nOpenAPI specification files generated successfully!");
    println!("\nYou can:");
    println!("  - View JSON spec: {}", json_path.display());
    println!("  - View YAML spec: {}", yaml_path.display());
    println!(
        "  - Access Swagger UI at: http://localhost:3000/api/v1/docs (when server is running)"
    );

    Ok(())
}

#[cfg(not(feature = "openapi"))]
fn main() {
    eprintln!("Error: This example requires the 'openapi' feature to be enabled.");
    eprintln!(
        "Run with: cargo run -p xze-serve --features openapi --example generate_openapi_spec"
    );
    std::process::exit(1);
}
