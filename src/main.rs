//! XZe - Pipeline Documentation Tool
//!
//! XZe is a tool that uses open source models from Ollama to analyze service
//! source code and documentation in git repositories and creates comprehensive
//! documentation following the Diátaxis Documentation Framework.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{error, info};
use xze_core::{Result, XzeError};

#[derive(Parser)]
#[command(name = "xze")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "XZe Pipeline Documentation Tool - AI-powered documentation generation")]
#[command(long_about = r#"
XZe is a pipeline documentation tool that uses AI to analyze your source code
and generate comprehensive documentation following the Diátaxis framework.

The tool can operate in multiple modes:
- Local: Analyze specific repository paths
- Auto: Use configuration file to manage multiple repositories
- Server: Run as a web service with REST API

Supported languages: Rust, Go, Python, JavaScript, TypeScript, Java, and more.
"#)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Output format (json, yaml, pretty)
    #[arg(short, long, default_value = "pretty", global = true)]
    output: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Load documents into knowledge base
    Load(xze_cli::LoadArgs),

    /// Analyze repositories and generate documentation
    Analyze {
        /// Repository paths to analyze (local mode)
        #[arg(short, long)]
        repos: Vec<PathBuf>,

        /// Use configuration file for repository list (auto mode)
        #[arg(short, long)]
        auto: bool,

        /// Programming language override
        #[arg(short, long)]
        language: Option<String>,

        /// Output directory for generated documentation
        #[arg(short = 'o', long)]
        output_dir: Option<PathBuf>,

        /// Dry run - don't write files
        #[arg(long)]
        dry_run: bool,
    },

    /// Start XZe as a web server
    Serve {
        /// Server host address
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Server port
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Ollama server URL
        #[arg(long, default_value = "http://localhost:11434")]
        ollama_url: String,
    },

    /// Initialize XZe configuration
    Init {
        /// Configuration file path
        #[arg(short, long, default_value = "xze-config.yaml")]
        config: PathBuf,

        /// Interactive mode for configuration
        #[arg(short, long)]
        interactive: bool,
    },

    /// Validate configuration file
    Validate {
        /// Configuration file to validate
        config: PathBuf,
    },

    /// Show version information
    Version,

    /// Show health status
    Health,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    let log_format = if cli.output == "json" {
        "json"
    } else {
        "pretty"
    };
    xze_core::init_logging_with_config(log_level, log_format)?;

    info!("Starting XZe v{}", env!("CARGO_PKG_VERSION"));

    // Handle commands
    match cli.command {
        Some(Commands::Load(ref args)) => {
            handle_load(args, &cli).await?;
        }

        Some(Commands::Analyze {
            ref repos,
            auto,
            ref language,
            ref output_dir,
            dry_run,
        }) => {
            handle_analyze(
                repos.clone(),
                auto,
                language.clone(),
                output_dir.clone(),
                dry_run,
                &cli,
            )
            .await?;
        }

        Some(Commands::Serve {
            ref host,
            port,
            ref ollama_url,
        }) => {
            handle_serve(host.clone(), port, ollama_url.clone(), &cli).await?;
        }

        Some(Commands::Init {
            ref config,
            interactive,
        }) => {
            handle_init(config.clone(), interactive, &cli).await?;
        }

        Some(Commands::Validate { ref config }) => {
            handle_validate(config.clone(), &cli).await?;
        }

        Some(Commands::Version) => {
            handle_version(&cli).await?;
        }

        Some(Commands::Health) => {
            handle_health(&cli).await?;
        }

        None => {
            // No subcommand provided - show help or run in demo mode
            handle_default(&cli).await?;
        }
    }

    info!("XZe completed successfully");
    Ok(())
}

async fn handle_load(args: &xze_cli::LoadArgs, _cli: &Cli) -> Result<()> {
    info!("Executing load command");

    // Validate and execute the load command
    xze_cli::execute_command(args.clone()).await?;

    Ok(())
}

async fn handle_analyze(
    repos: Vec<PathBuf>,
    auto: bool,
    language: Option<String>,
    _output_dir: Option<PathBuf>,
    dry_run: bool,
    cli: &Cli,
) -> Result<()> {
    info!("Running analysis mode");

    if auto {
        info!("Using auto mode with configuration file");
        let config_path = cli
            .config
            .as_ref()
            .ok_or_else(|| XzeError::validation("Configuration file required for auto mode"))?;

        let config = xze_core::XzeConfig::from_file(config_path)?;
        config.validate()?;

        info!(
            "Found {} repositories in configuration",
            config.repositories.len()
        );

        // TODO: Implement auto mode analysis
        println!(
            "Auto mode analysis would process {} configured repositories",
            config.repositories.len()
        );
    } else if !repos.is_empty() {
        info!("Using local mode with {} repositories", repos.len());

        for repo_path in &repos {
            info!("Analyzing repository: {:?}", repo_path);

            if !repo_path.exists() {
                error!("Repository path does not exist: {:?}", repo_path);
                continue;
            }

            // Use xze-core to analyze the repository
            let (detected_lang, analyzer) =
                xze_core::repository::analyzer::AnalyzerFactory::auto_detect_analyzer(repo_path)?;

            let actual_language = if let Some(lang_str) = &language {
                xze_core::types::ProgrammingLanguage::from(lang_str.as_str())
            } else {
                detected_lang
            };

            info!("Using language: {}", actual_language);

            let structure = analyzer.analyze(repo_path)?;

            if dry_run {
                info!(
                    "Dry run - would analyze repository with {} items",
                    structure.item_count()
                );
            } else {
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
                    _ => {
                        print_analysis_results(&structure, repo_path);
                    }
                }
            }
        }
    } else {
        return Err(XzeError::validation("No repositories specified. Use --repos for local mode or --auto for configuration mode"));
    }

    Ok(())
}

async fn handle_serve(host: String, port: u16, ollama_url: String, cli: &Cli) -> Result<()> {
    info!("Starting XZe server on {}:{}", host, port);
    info!("Using Ollama at: {}", ollama_url);

    // Load configuration if provided
    let config = if let Some(config_path) = &cli.config {
        xze_core::XzeConfig::from_file(config_path)?
    } else {
        xze_core::XzeConfig::default()
    };

    // TODO: Start the actual server using xze-serve
    println!("🚀 XZe Server would start on http://{}:{}", host, port);
    println!("📡 Ollama endpoint: {}", ollama_url);
    println!(
        "⚙️  Configuration: {} repositories",
        config.repositories.len()
    );
    println!("\nPress Ctrl+C to stop the server");

    // For now, just simulate server running
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    println!("Server simulation complete");

    Ok(())
}

async fn handle_init(config_path: PathBuf, interactive: bool, _cli: &Cli) -> Result<()> {
    info!("Initializing XZe configuration at {:?}", config_path);

    if config_path.exists() {
        return Err(XzeError::validation(format!(
            "Configuration file already exists: {:?}. Remove it first or use a different path.",
            config_path
        )));
    }

    let config = xze_core::XzeConfig::default();

    if interactive {
        println!("🎯 XZe Configuration Setup");
        println!("=========================");
        println!();

        // TODO: Implement interactive configuration
        println!("Interactive configuration not yet implemented.");
        println!("Creating default configuration file...");
    }

    // Save the configuration
    config.to_file(&config_path)?;

    println!("✅ Configuration initialized at {:?}", config_path);
    println!();
    println!("Next steps:");
    println!("1. Edit the configuration file to add your repositories");
    println!(
        "2. Run 'xze validate {:?}' to check your configuration",
        config_path
    );
    println!(
        "3. Run 'xze analyze --auto --config {:?}' to analyze your repositories",
        config_path
    );

    Ok(())
}

async fn handle_validate(config_path: PathBuf, cli: &Cli) -> Result<()> {
    info!("Validating configuration at {:?}", config_path);

    if !config_path.exists() {
        return Err(XzeError::not_found(format!(
            "Configuration file not found: {:?}",
            config_path
        )));
    }

    let config = xze_core::XzeConfig::from_file(&config_path)?;

    // Validate configuration
    match config.validate() {
        Ok(()) => match cli.output.as_str() {
            "json" => {
                let result = serde_json::json!({
                    "valid": true,
                    "message": "Configuration is valid",
                    "repositories": config.repositories.len()
                });
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            _ => {
                println!("✅ Configuration is valid");
                println!("📁 Found {} repositories", config.repositories.len());
                println!("🤖 Ollama URL: {}", config.ollama.url);
                println!("🧠 Primary model: {}", config.ollama.models.primary);
            }
        },
        Err(e) => {
            match cli.output.as_str() {
                "json" => {
                    let result = serde_json::json!({
                        "valid": false,
                        "error": e.to_string()
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                _ => {
                    println!("❌ Configuration is invalid: {}", e);
                }
            }
            return Err(e);
        }
    }

    Ok(())
}

async fn handle_version(_cli: &Cli) -> Result<()> {
    println!("{}", xze_core::version_info());
    println!();
    println!("Build info:");
    println!(
        "  Commit: {}",
        option_env!("VERGEN_GIT_SHA").unwrap_or("unknown")
    );
    println!(
        "  Build date: {}",
        option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown")
    );
    println!(
        "  Rust version: {}",
        option_env!("VERGEN_RUSTC_SEMVER").unwrap_or("unknown")
    );
    Ok(())
}

async fn handle_health(_cli: &Cli) -> Result<()> {
    info!("Running health check");

    match xze_core::health_check() {
        Ok(()) => {
            println!("✅ XZe is healthy");
            println!("  ✓ Environment checks passed");
            println!("  ✓ File system access OK");
            println!("  ✓ Core library loaded");
        }
        Err(e) => {
            println!("❌ Health check failed: {}", e);
            return Err(XzeError::validation(format!("Health check failed: {}", e)));
        }
    }

    Ok(())
}

async fn handle_default(_cli: &Cli) -> Result<()> {
    println!("🚀 Welcome to XZe - Pipeline Documentation Tool");
    println!();
    println!("XZe helps you generate comprehensive documentation for your");
    println!("software projects using AI analysis and the Diátaxis framework.");
    println!();
    println!("Quick start:");
    println!("  xze init                    # Create configuration file");
    println!("  xze analyze --repos ./repo  # Analyze a local repository");
    println!("  xze serve                   # Start web server");
    println!("  xze --help                  # Show all options");
    println!();
    println!("For more information, visit: https://github.com/xbcsmith/xze");

    Ok(())
}

fn print_analysis_results(structure: &xze_core::CodeStructure, repo_path: &PathBuf) {
    println!();
    println!("📊 Analysis Results for {:?}", repo_path);
    println!("{:=<50}", "");
    println!();

    if structure.is_empty() {
        println!("No code structure found in the repository.");
        return;
    }

    println!("📈 Summary:");
    println!("  Total items: {}", structure.item_count());
    println!("  Modules: {}", structure.modules.len());
    println!("  Functions: {}", structure.functions.len());
    println!("  Types: {}", structure.types.len());
    println!("  Config files: {}", structure.configs.len());
    println!();

    if !structure.functions.is_empty() {
        let public_functions = structure.public_functions();
        println!("⚡ Functions:");
        println!("  Total: {}", structure.functions.len());
        println!("  Public: {}", public_functions.len());
        println!(
            "  Private: {}",
            structure.functions.len() - public_functions.len()
        );

        if !public_functions.is_empty() {
            println!("  Public functions:");
            for func in public_functions.iter().take(5) {
                let async_marker = if func.is_async { " (async)" } else { "" };
                let doc_marker = if func.documentation.is_some() {
                    " 📝"
                } else {
                    ""
                };
                println!("    • {}{}{}", func.name, async_marker, doc_marker);
            }
            if public_functions.len() > 5 {
                println!("    ... and {} more", public_functions.len() - 5);
            }
        }
        println!();
    }

    if !structure.types.is_empty() {
        println!("🏗️  Types:");
        for type_def in structure.types.iter().take(10) {
            let visibility = match type_def.visibility {
                xze_core::repository::Visibility::Public => "pub",
                xze_core::repository::Visibility::Private => "prv",
                xze_core::repository::Visibility::Protected => "prt",
            };
            let doc_marker = if type_def.documentation.is_some() {
                " 📝"
            } else {
                ""
            };
            println!(
                "  {} {:?} {}{}",
                visibility, type_def.kind, type_def.name, doc_marker
            );
        }
        if structure.types.len() > 10 {
            println!("  ... and {} more", structure.types.len() - 10);
        }
        println!();
    }

    if !structure.configs.is_empty() {
        println!("⚙️  Configuration Files:");
        for config in &structure.configs {
            println!(
                "  {:?} ({})",
                config.path.file_name().unwrap_or_default(),
                format!("{:?}", config.format).to_lowercase()
            );
        }
    }
}
