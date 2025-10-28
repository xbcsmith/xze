//! Classify command implementation for intent classification

use clap::Args;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Arc;
use xze_core::{
    ai::{client::OllamaClient, ClassifierConfig, DiataxisIntent, IntentClassifier},
    Result, XzeError,
};

use crate::commands::CliCommand;

/// Classify a query to determine its documentation intent
#[derive(Debug, Args)]
pub struct ClassifyCommand {
    /// Query to classify
    #[arg(value_name = "QUERY")]
    pub query: String,

    /// Enable verbose output with reasoning
    #[arg(short, long)]
    pub verbose: bool,

    /// Output format (text or json)
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Enable multi-intent detection
    #[arg(short, long)]
    pub multi_intent: bool,

    /// Ollama server URL
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: String,

    /// Model to use for classification
    #[arg(long, default_value = "llama3.2")]
    pub model: String,

    /// Confidence threshold (0.0-1.0)
    #[arg(short = 't', long, default_value = "0.6")]
    pub threshold: f64,

    /// Temperature for LLM generation (0.0-1.0)
    #[arg(long, default_value = "0.1")]
    pub temperature: f64,

    /// Disable caching
    #[arg(long)]
    pub no_cache: bool,
}

impl CliCommand for ClassifyCommand {
    async fn execute(&self) -> Result<()> {
        // Build classifier configuration
        let config = ClassifierConfig {
            model: self.model.clone(),
            temperature: self.temperature as f32,
            confidence_threshold: self.threshold as f32,
            enable_multi_intent: self.multi_intent,
            cache_size: if self.no_cache { 0 } else { 100 },
            cache_ttl_seconds: if self.no_cache { 0 } else { 3600 },
            enable_metrics: false,
        };

        // Create Ollama client
        let client = Arc::new(OllamaClient::new(self.ollama_url.clone()));

        // Create classifier
        let classifier = IntentClassifier::new(config, client);

        // Classify query
        let result = classifier.classify(&self.query).await?;

        // Output results
        match self.format.as_str() {
            "json" => {
                self.output_json(&result)?;
            }
            _ => {
                self.output_text(&result)?;
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "classify"
    }

    fn validate(&self) -> Result<()> {
        if self.query.trim().is_empty() {
            return Err(XzeError::validation("Query cannot be empty"));
        }

        if !(0.0..=1.0).contains(&self.threshold) {
            return Err(XzeError::validation(
                "Confidence threshold must be between 0.0 and 1.0",
            ));
        }

        if !(0.0..=1.0).contains(&self.temperature) {
            return Err(XzeError::validation(
                "Temperature must be between 0.0 and 1.0",
            ));
        }

        if self.format != "text" && self.format != "json" {
            return Err(XzeError::validation("Format must be 'text' or 'json'"));
        }

        Ok(())
    }
}

impl ClassifyCommand {
    /// Output results in JSON format
    fn output_json(&self, result: &xze_core::ai::ClassificationResult) -> Result<()> {
        let output = ClassificationOutput {
            primary_intent: result.primary_intent.to_string(),
            confidence: result.confidence.value(),
            confidence_level: format!("{:?}", result.confidence.level()),
            secondary_intents: result
                .secondary_intents
                .iter()
                .map(|(intent, conf)| SecondaryIntent {
                    intent: intent.to_string(),
                    confidence: conf.value(),
                })
                .collect(),
            reasoning: if self.verbose {
                Some(result.reasoning.clone())
            } else {
                None
            },
            metadata: if self.verbose {
                Some(MetadataOutput {
                    cached: result.metadata.cached,
                    duration_ms: result.metadata.duration_ms,
                    model: result.metadata.model.clone(),
                    timestamp: result.metadata.timestamp.to_rfc3339(),
                })
            } else {
                None
            },
        };

        let json = serde_json::to_string_pretty(&output)
            .map_err(|e| XzeError::filesystem(format!("Failed to serialize JSON: {}", e)))?;

        println!("{}", json);
        Ok(())
    }

    /// Output results in human-readable text format
    fn output_text(&self, result: &xze_core::ai::ClassificationResult) -> Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();

        writeln!(handle, "\nIntent Classification Results")
            .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;
        writeln!(handle, "==============================")
            .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;

        writeln!(
            handle,
            "\nPrimary Intent: {}",
            intent_to_display_string(&result.primary_intent)
        )
        .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;

        writeln!(
            handle,
            "Confidence:     {:.1}% ({})",
            result.confidence.value() * 100.0,
            confidence_indicator(&result.confidence)
        )
        .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;

        if !result.secondary_intents.is_empty() {
            writeln!(handle, "\nSecondary Intents:")
                .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;

            for (intent, confidence) in &result.secondary_intents {
                writeln!(
                    handle,
                    "  - {} ({})",
                    intent_to_display_string(intent),
                    confidence_indicator(confidence)
                )
                .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;

                writeln!(
                    handle,
                    "    Confidence: {:.1}% - {:?}",
                    confidence.value() * 100.0,
                    confidence.level()
                )
                .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;
            }
        }

        if self.verbose {
            writeln!(handle)
                .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;
            writeln!(handle, "Reasoning:")
                .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;
            writeln!(handle, "  {}", result.reasoning)
                .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;
        }

        if self.verbose {
            writeln!(handle)
                .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;
            writeln!(handle, "Metadata:")
                .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;
            writeln!(
                handle,
                "  Cached: {}",
                if result.metadata.cached { "yes" } else { "no" }
            )
            .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;
            writeln!(handle, "  Duration: {}ms", result.metadata.duration_ms)
                .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;
            writeln!(handle, "  Model: {}", result.metadata.model)
                .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;
            writeln!(handle, "  Timestamp: {}", result.metadata.timestamp)
                .map_err(|e| XzeError::filesystem(format!("Failed to write output: {}", e)))?;
        }

        Ok(())
    }
}

/// Convert DiataxisIntent to display string with description
fn intent_to_display_string(intent: &DiataxisIntent) -> String {
    match intent {
        DiataxisIntent::Tutorial => "Tutorial (learning-oriented)",
        DiataxisIntent::HowTo => "How-To (task-oriented)",
        DiataxisIntent::Reference => "Reference (information-oriented)",
        DiataxisIntent::Explanation => "Explanation (understanding-oriented)",
    }
    .to_string()
}

/// Generate confidence indicator symbol
fn confidence_indicator(confidence: &xze_core::ai::Confidence) -> String {
    match confidence.level() {
        xze_core::ai::ConfidenceLevel::High => "High",
        xze_core::ai::ConfidenceLevel::Medium => "Medium",
        xze_core::ai::ConfidenceLevel::Low => "Low",
    }
    .to_string()
}

/// JSON output structure
#[derive(Debug, Serialize, Deserialize)]
struct ClassificationOutput {
    primary_intent: String,
    confidence: f32,
    confidence_level: String,
    secondary_intents: Vec<SecondaryIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<MetadataOutput>,
}

/// Secondary intent output
#[derive(Debug, Serialize, Deserialize)]
struct SecondaryIntent {
    intent: String,
    confidence: f32,
}

/// Metadata output
#[derive(Debug, Serialize, Deserialize)]
struct MetadataOutput {
    cached: bool,
    duration_ms: u64,
    model: String,
    timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_command_validation() {
        let cmd = ClassifyCommand {
            query: "How do I install XZe?".to_string(),
            verbose: false,
            format: "text".to_string(),
            multi_intent: false,
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            threshold: 0.6,
            temperature: 0.1,
            no_cache: false,
        };

        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_classify_command_empty_query() {
        let cmd = ClassifyCommand {
            query: "   ".to_string(),
            verbose: false,
            format: "text".to_string(),
            multi_intent: false,
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            threshold: 0.6,
            temperature: 0.1,
            no_cache: false,
        };

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_classify_command_invalid_threshold() {
        let cmd = ClassifyCommand {
            query: "test query".to_string(),
            verbose: false,
            format: "text".to_string(),
            multi_intent: false,
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            threshold: 1.5,
            temperature: 0.1,
            no_cache: false,
        };

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_classify_command_invalid_temperature() {
        let cmd = ClassifyCommand {
            query: "test query".to_string(),
            verbose: false,
            format: "text".to_string(),
            multi_intent: false,
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            threshold: 0.6,
            temperature: -0.1,
            no_cache: false,
        };

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_classify_command_invalid_format() {
        let cmd = ClassifyCommand {
            query: "test query".to_string(),
            verbose: false,
            format: "xml".to_string(),
            multi_intent: false,
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            threshold: 0.6,
            temperature: 0.1,
            no_cache: false,
        };

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_classify_command_name() {
        let cmd = ClassifyCommand {
            query: "test".to_string(),
            verbose: false,
            format: "text".to_string(),
            multi_intent: false,
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            threshold: 0.6,
            temperature: 0.1,
            no_cache: false,
        };

        assert_eq!(cmd.name(), "classify");
    }

    #[test]
    fn test_intent_to_display_string() {
        assert!(intent_to_display_string(&DiataxisIntent::Tutorial).contains("Tutorial"));
        assert!(intent_to_display_string(&DiataxisIntent::HowTo).contains("How-To"));
        assert!(intent_to_display_string(&DiataxisIntent::Reference).contains("Reference"));
        assert!(intent_to_display_string(&DiataxisIntent::Explanation).contains("Explanation"));
    }

    #[test]
    fn test_json_output_serialization() {
        let output = ClassificationOutput {
            primary_intent: "Tutorial".to_string(),
            confidence: 0.95,
            confidence_level: "VeryHigh".to_string(),
            secondary_intents: vec![SecondaryIntent {
                intent: "HowTo".to_string(),
                confidence: 0.75,
            }],
            reasoning: Some("This is a tutorial query".to_string()),
            metadata: Some(MetadataOutput {
                cached: false,
                duration_ms: 123,
                model: "llama2".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            }),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
    }
}
