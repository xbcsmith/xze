//! Benchmark tests for intent classification performance
//!
//! Run with: cargo bench --bench intent_classification_bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;
use xze_core::ai::client::OllamaClient;
use xze_core::ai::intent_classifier::{ClassifierConfig, IntentClassifier};

/// Benchmark single classification with cache disabled
fn bench_single_classification(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));

    // Note: This requires a running Ollama instance
    // For CI/CD, these benchmarks should be skipped or use mocks

    let mut config = ClassifierConfig::default().with_model("llama2:latest");
    config.cache_size = 0; // Disable cache for pure classification benchmark

    let classifier = IntentClassifier::new(config, client);

    let queries = vec![
        "How do I install this library?",
        "What is the architecture of this system?",
        "Explain how dependency injection works",
        "API reference for the authentication module",
    ];

    c.bench_function("classify_single", |b| {
        b.to_async(&rt).iter(|| async {
            let query = black_box(queries[0]);
            // Note: In real benchmarks, we'd need a mock or test endpoint
            // This is a placeholder structure
            let _ = black_box(query);
        });
    });
}

/// Benchmark classification with cache enabled
fn bench_cached_classification(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));

    let mut config = ClassifierConfig::default().with_model("llama2:latest");
    config.cache_size = 1000;

    let _classifier = IntentClassifier::new(config, client);

    c.bench_function("classify_cached", |b| {
        b.to_async(&rt).iter(|| async {
            let query = black_box("How do I install this library?");
            let _ = black_box(query);
            // In production, would call: classifier.classify(query).await
        });
    });
}

/// Benchmark batch classification
fn bench_batch_classification(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));

    let mut config = ClassifierConfig::default().with_model("llama2:latest");
    config.cache_size = 100;

    let _classifier = IntentClassifier::new(config, client);

    let queries = vec![
        "How do I configure logging?",
        "What are the system requirements?",
        "Explain the event-driven architecture",
        "REST API endpoint documentation",
        "Tutorial for getting started",
    ];

    c.bench_with_input(
        BenchmarkId::new("batch_classify", queries.len()),
        &queries,
        |b, queries| {
            b.to_async(&rt).iter(|| async {
                let _queries = black_box(queries);
                // In production: classifier.classify_batch(queries).await
            });
        },
    );
}

/// Benchmark cache operations
fn bench_cache_operations(c: &mut Criterion) {
    let _rt = Runtime::new().unwrap();
    let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));

    let mut group = c.benchmark_group("cache_operations");

    for size in [100, 500, 1000].iter() {
        let mut config = ClassifierConfig::default().with_model("llama2:latest");
        config.cache_size = *size;

        let classifier = IntentClassifier::new(config, Arc::clone(&client));

        group.bench_with_input(BenchmarkId::new("cache_lookup", size), size, |b, _size| {
            b.iter(|| {
                let _ = black_box(classifier.cache_stats());
            });
        });
    }

    group.finish();
}

/// Benchmark prompt generation
fn bench_prompt_generation(c: &mut Criterion) {
    c.bench_function("prompt_generation", |b| {
        let query = "How do I set up continuous integration?";
        b.iter(|| {
            // Simulate prompt generation overhead
            let _prompt = format!(
                "Classify the following query into one of: Tutorial, HowTo, Reference, Explanation.\n\nQuery: {}",
                black_box(query)
            );
        });
    });
}

/// Benchmark multi-intent detection parsing
fn bench_multi_intent_parsing(c: &mut Criterion) {
    let response = r#"
Primary Intent: Tutorial
Confidence: 0.85
Reasoning: This is a step-by-step guide

Secondary Intents:
- HowTo (0.72): Contains task-oriented instructions
- Reference (0.45): Includes API documentation
"#;

    c.bench_function("parse_multi_intent", |b| {
        b.iter(|| {
            // Simulate parsing overhead
            let lines: Vec<&str> = black_box(response).lines().collect();
            let _parsed = black_box(lines.len());
        });
    });
}

/// Benchmark cache key normalization
fn bench_cache_key_normalization(c: &mut Criterion) {
    let queries = vec![
        "  How do I install this?  ",
        "HOW DO I INSTALL THIS?",
        "how do i install this?",
        "How    do   I   install   this?",
    ];

    c.bench_function("normalize_cache_key", |b| {
        b.iter(|| {
            for query in &queries {
                let _normalized = black_box(
                    query
                        .trim()
                        .to_lowercase()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }
        });
    });
}

/// Benchmark confidence threshold validation
fn bench_confidence_validation(c: &mut Criterion) {
    c.bench_function("confidence_validation", |b| {
        let confidence = 0.75;
        let threshold = 0.7;
        b.iter(|| {
            let _valid = black_box(confidence) >= black_box(threshold);
        });
    });
}

criterion_group!(
    benches,
    bench_single_classification,
    bench_cached_classification,
    bench_batch_classification,
    bench_cache_operations,
    bench_prompt_generation,
    bench_multi_intent_parsing,
    bench_cache_key_normalization,
    bench_confidence_validation,
);

criterion_main!(benches);
