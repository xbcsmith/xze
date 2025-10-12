//! XZe Core - Main binary for testing core functionality

use std::path::PathBuf;
use xze_core::{
    config::XzeConfig,
    repository::{RepositoryManager, analyzer::AnalyzerFactory},
    types::{RepositoryId, ProgrammingLanguage},
    Result, XzeError,
};
use clap::Parser;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "xze-core")]
#[command(about = "XZe Core - Pipeline Documentation Tool Core Library")]
struct Cli {
    /// Repository path to analyze
    #[arg(short, long)]
    repo_path: Option<PathBuf>,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Output format (json, yaml, pretty)
    #[arg(short, long, default_value = "pretty")]
    output: String,

    /// Language to use for analysis (auto-detect if not specified)
    #[arg(short, long)]
    language: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    xze_core::init_logging_with_config(log_level, "pretty")?;

    info!("Starting XZe Core v{}", xze_core::VERSION);

    // Health check
    if let Err(e) = xze_core::health_check() {
        error!("Health check failed: {}", e);
        return Err(XzeError::validation(format!("Health check failed: {}", e)));
    }

    // Load or create configuration
    let config = if let Some(config_path) = cli.config {
        info!("Loading configuration from {:?}", config_path);
        XzeConfig::from_file(config_path)?
    } else {
        info!("Using default configuration");
        XzeConfig::default()
    };

    // Validate configuration
    config.validate()?;

    // Create repository manager
    let cache_dir = std::env::temp_dir().join("xze-cache");
    let repo_manager = RepositoryManager::new(cache_dir, config.clone())?;

    if let Some(repo_path) = cli.repo_path {
        // Analyze a local repository
        analyze_local_repository(&repo_path, &cli).await?;
    } else if !config.repositories.is_empty() {
        // Analyze configured repositories
        analyze_configured_repositories(&repo_manager, &cli).await?;
    } else {
        // Demo mode - show capabilities
        run_demo_mode().await?;
    }

    info!("XZe Core completed successfully");
    Ok(())
}

async fn analyze_local_repository(repo_path: &PathBuf, cli: &Cli) -> Result<()> {
    info!("Analyzing local repository: {:?}", repo_path);

    if !repo_path.exists() {
        return Err(XzeError::not_found(format!("Repository path does not exist: {:?}", repo_path)));
    }

    // Detect or use specified language
    let language = if let Some(lang_str) = &cli.language {
        ProgrammingLanguage::from(lang_str.as_str())
    } else {
        // Auto-detect language
        let (detected_lang, _) = AnalyzerFactory::auto_detect_analyzer(repo_path)?;
        info!("Detected language: {}", detected_lang);
        detected_lang
    };

    // Create analyzer
    let analyzer = AnalyzerFactory::create_analyzer(&language);

    // Analyze repository structure
    let structure = analyzer.analyze(repo_path)?;

    // Output results
    match cli.output.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&structure)?;
            println!("{}", json);
        }
        "yaml" => {
            let yaml = serde_yaml::to_string(&structure)?;
            println!("{}", yaml);
        }
        "pretty" | _ => {
            print_structure_pretty(&structure);
        }
    }

    Ok(())
}

async fn analyze_configured_repositories(repo_manager: &RepositoryManager, cli: &Cli) -> Result<()> {
    info!("Analyzing configured repositories");

    let stats = repo_manager.get_stats().await;
    info!("Repository manager stats: {} repositories cached", stats.total_repositories);

    // This would iterate through configured repositories and analyze them
    // For now, just show the stats
    match cli.output.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&stats)?;
            println!("{}", json);
        }
        "yaml" => {
            let yaml = serde_yaml::to_string(&stats)?;
            println!("{}", yaml);
        }
        "pretty" | _ => {
            println!("Repository Manager Statistics:");
            println!("  Total repositories: {}", stats.total_repositories);
            println!("  Total files: {}", stats.total_files);
            println!("  Total lines of code: {}", stats.total_lines_of_code);
            println!("  Cache size: {} MB", stats.cache_size_mb);
            println!("  Language distribution:");
            for (lang, count) in &stats.language_distribution {
                println!("    {}: {}", lang, count);
            }
        }
    }

    Ok(())
}

async fn run_demo_mode() -> Result<()> {
    info!("Running in demo mode");

    println!("🚀 XZe Core Demo Mode");
    println!("=====================");
    println!();
    println!("XZe is a pipeline documentation tool that uses AI to analyze");
    println!("your source code and generate comprehensive documentation");
    println!("following the Diátaxis framework.");
    println!();
    println!("Features:");
    println!("  ✓ Multi-language source code analysis");
    println!("  ✓ AI-powered documentation generation");
    println!("  ✓ Git integration for automated updates");
    println!("  ✓ Diátaxis framework compliance");
    println!("  ✓ REST API and CLI interfaces");
    println!();
    println!("Supported Languages:");

    let languages = vec![
        ProgrammingLanguage::Rust,
        ProgrammingLanguage::Go,
        ProgrammingLanguage::Python,
        ProgrammingLanguage::JavaScript,
        ProgrammingLanguage::TypeScript,
        ProgrammingLanguage::Java,
    ];

    for lang in languages {
        let analyzer = AnalyzerFactory::create_analyzer(&lang);
        let extensions = analyzer.supported_extensions().join(", ");
        println!("  • {} ({})", lang, extensions);
    }

    println!();
    println!("Usage Examples:");
    println!("  # Analyze a local repository");
    println!("  xze-core --repo-path /path/to/repo");
    println!();
    println!("  # Use configuration file");
    println!("  xze-core --config xze-config.yaml");
    println!();
    println!("  # Specify language explicitly");
    println!("  xze-core --repo-path /path/to/repo --language rust");
    println!();
    println!("For more information, visit: https://github.com/xbcsmith/xze");

    Ok(())
}

fn print_structure_pretty(structure: &xze_core::repository::CodeStructure) {
    println!("📊 Code Structure Analysis");
    println!("==========================");
    println!();

    if structure.is_empty() {
        println!("No code structure found.");
        return;
    }

    println!("Summary:");
    println!("  Total items: {}", structure.item_count());
    println!("  Modules: {}", structure.modules.len());
    println!("  Functions: {}", structure.functions.len());
    println!("  Types: {}", structure.types.len());
    println!("  Config files: {}", structure.configs.len());
    println!();

    if !structure.modules.is_empty() {
        println!("📁 Modules ({}):", structure.modules.len());
        for module in &structure.modules {
            let visibility_icon = match module.visibility {
                xze_core::repository::Visibility::Public => "🌐",
                xze_core::repository::Visibility::Private => "🔒",
                xze_core::repository::Visibility::Protected => "🛡️",
            };
            println!("  {} {} {}", visibility_icon, module.name,
                     if module.documentation.is_some() { "📝" } else { "" });
        }
        println!();
    }

    if !structure.functions.is_empty() {
        println!("⚡ Functions ({}):", structure.functions.len());
        let public_functions = structure.public_functions();
        println!("  Public functions: {}", public_functions.len());

        for func in public_functions.iter().take(10) { // Show first 10
            let async_icon = if func.is_async { "🔄" } else { "" };
            let doc_icon = if func.documentation.is_some() { "📝" } else { "" };
            println!("  🌐 {} {} {}", func.name, async_icon, doc_icon);
        }

        if public_functions.len() > 10 {
            println!("  ... and {} more", public_functions.len() - 10);
        }
        println!();
    }

    if !structure.types.is_empty() {
        println!("🏗️  Types ({}):", structure.types.len());
        for type_def in structure.types.iter().take(10) { // Show first 10
            let visibility_icon = match type_def.visibility {
                xze_core::repository::Visibility::Public => "🌐",
                xze_core::repository::Visibility::Private => "🔒",
                xze_core::repository::Visibility::Protected => "🛡️",
            };
            let kind_icon = match type_def.kind {
                xze_core::repository::TypeKind::Struct => "📦",
                xze_core::repository::TypeKind::Enum => "🔢",
                xze_core::repository::TypeKind::Trait => "🎯",
                xze_core::repository::TypeKind::Interface => "🔌",
                xze_core::repository::TypeKind::Class => "🏛️",
            };
            let doc_icon = if type_def.documentation.is_some() { "📝" } else { "" };
            println!("  {} {} {} {} {}", visibility_icon, kind_icon, type_def.name, doc_icon,
                     if !type_def.fields.is_empty() {
                         format!("({} fields)", type_def.fields.len())
                     } else {
                         String::new()
                     });
        }

        if structure.types.len() > 10 {
            println!("  ... and {} more", structure.types.len() - 10);
        }
        println!();
    }

    if !structure.configs.is_empty() {
        println!("⚙️  Configuration Files ({}):", structure.configs.len());
        for config in &structure.configs {
            let format_icon = match config.format {
                xze_core::repository::ConfigFormat::Yaml => "📄",
                xze_core::repository::ConfigFormat::Toml => "📋",
                xze_core::repository::ConfigFormat::Json => "📊",
                xze_core::repository::ConfigFormat::Env => "🔧",
            };
            println!("  {} {} ({:?})", format_icon,
                     config.path.file_name().unwrap_or_default().to_string_lossy(),
                     config.format);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test that CLI can be parsed with basic arguments
        let cli = Cli::try_parse_from(&["xze-core", "--repo-path", "/tmp/test"]).unwrap();
        assert_eq!(cli.repo_path, Some(PathBuf::from("/tmp/test")));
        assert_eq!(cli.output, "pretty");
    }

    #[tokio::test]
    async fn test_demo_mode() {
        // Test that demo mode runs without errors
        let result = run_demo_mode().await;
        assert!(result.is_ok());
    }
}
