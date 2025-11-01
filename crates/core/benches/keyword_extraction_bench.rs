//! Benchmarks for keyword extraction performance
//!
//! These benchmarks measure the performance of different keyword extraction strategies:
//! - Frequency-based extraction (fallback method)
//! - Cache hit performance
//! - Batch processing performance
//!
//! Run with: cargo bench --package xze-core --bench keyword_extraction_bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use xze_core::keyword_extractor::{KeywordExtractor, KeywordExtractorConfig};

/// Sample documentation content of varying sizes
const SMALL_DOC: &str = r#"
# Getting Started with Rust

Rust is a systems programming language that runs blazingly fast,
prevents segfaults, and guarantees thread safety. This guide will
help you get started with Rust programming.
"#;

const MEDIUM_DOC: &str = r#"
# Async Programming in Rust

Rust's async programming model is based on the Future trait. This allows
for efficient concurrent programming without the overhead of traditional
threads. The tokio runtime provides a complete async ecosystem.

## Key Concepts

- Futures: Lazy computations that can be awaited
- Async/await syntax: Makes async code look synchronous
- Runtime: Executes async tasks (like tokio)
- Pin: Ensures futures don't move in memory

## Example Usage

Using async functions with tokio is straightforward. You mark functions
with the async keyword and use .await to wait for operations to complete.
The runtime handles scheduling and execution of concurrent tasks efficiently.

## Best Practices

Always use structured concurrency patterns. Avoid blocking operations in
async contexts. Use channels for communication between tasks. Handle errors
properly with Result types throughout your async code.
"#;

const LARGE_DOC: &str = r#"
# Comprehensive Guide to Cargo and Rust Project Management

Cargo is Rust's build system and package manager. It handles building your
code, downloading dependencies, building those dependencies, and much more.

## Project Structure

A typical Rust project managed by Cargo has the following structure:
- Cargo.toml: Project manifest with metadata and dependencies
- Cargo.lock: Exact versions of dependencies (generated)
- src/: Source code directory
- target/: Build artifacts directory

## Dependency Management

Cargo makes it easy to manage dependencies. You can specify dependencies
in your Cargo.toml file using semantic versioning. Cargo will download
and compile all dependencies automatically.

### Types of Dependencies

- Regular dependencies: Used in your library or binary
- Dev dependencies: Only used during development and testing
- Build dependencies: Used in build scripts

## Building and Running

Cargo provides several commands for building and running your project:
- cargo build: Compile the project
- cargo run: Compile and run the project
- cargo test: Run tests
- cargo bench: Run benchmarks
- cargo doc: Generate documentation

## Workspaces

For larger projects, Cargo supports workspaces. A workspace is a collection
of packages that share the same Cargo.lock file and output directory. This
allows you to manage multiple related packages together efficiently.

## Publishing to crates.io

Cargo makes it easy to publish your library to crates.io, Rust's package
registry. You need to package your crate, ensure it builds cleanly, and
then publish it with cargo publish.

## Best Practices

- Use semantic versioning for your crate versions
- Write comprehensive documentation with examples
- Include tests for all public APIs
- Keep dependencies up to date with cargo update
- Use cargo clippy to catch common mistakes
- Format code with cargo fmt for consistency

## Advanced Features

Cargo supports many advanced features like custom build scripts, feature
flags, platform-specific dependencies, and more. These allow you to create
flexible and portable Rust projects that work across different platforms.
"#;

fn bench_frequency_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("frequency_extraction");

    let config = KeywordExtractorConfig {
        enable_fallback: true,
        ..Default::default()
    };
    let extractor = KeywordExtractor::new(config).unwrap();

    for (name, content) in [
        ("small", SMALL_DOC),
        ("medium", MEDIUM_DOC),
        ("large", LARGE_DOC),
    ] {
        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), content, |b, content| {
            b.iter(|| {
                extractor.extract_with_frequency(black_box(content)).unwrap()
            });
        });
    }

    group.finish();
}

fn bench_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = KeywordExtractorConfig {
        cache_size: 1000,
        enable_fallback: true,
        ..Default::default()
    };
    let extractor = KeywordExtractor::new(config).unwrap();

    // Pre-populate cache
    rt.block_on(async {
        let _ = extractor.extract(MEDIUM_DOC).await;
    });

    group.bench_function("cache_hit", |b| {
        b.to_async(&rt).iter(|| async {
            extractor.extract(black_box(MEDIUM_DOC)).await.unwrap()
        });
    });

    group.bench_function("cache_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let unique_content = format!("{} {}", MEDIUM_DOC, rand::random::<u64>());
            extractor.extract(black_box(&unique_content)).await.unwrap()
        });
    });

    group.finish();
}

fn bench_batch_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_processing");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = KeywordExtractorConfig {
        enable_fallback: true,
        ..Default::default()
    };
    let extractor = KeywordExtractor::new(config).unwrap();

    let documents: Vec<&str> = vec![SMALL_DOC, MEDIUM_DOC, LARGE_DOC];

    for size in [1, 3, 5, 10] {
        let batch: Vec<&str> = (0..size)
            .map(|i| documents[i % documents.len()])
            .collect();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}docs", size)),
            &batch,
            |b, batch| {
                b.to_async(&rt).iter(|| async {
                    extractor.extract_batch(black_box(batch)).await
                });
            },
        );
    }

    group.finish();
}

fn bench_tokenization(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokenization");

    let config = KeywordExtractorConfig::default();
    let extractor = KeywordExtractor::new(config).unwrap();

    for (name, content) in [
        ("small", SMALL_DOC),
        ("medium", MEDIUM_DOC),
        ("large", LARGE_DOC),
    ] {
        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), content, |b, content| {
            b.iter(|| {
                extractor.tokenize(black_box(content))
            });
        });
    }

    group.finish();
}

fn bench_cache_key_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_key_generation");

    let config = KeywordExtractorConfig::default();
    let extractor = KeywordExtractor::new(config).unwrap();

    for (name, content) in [
        ("small", SMALL_DOC),
        ("medium", MEDIUM_DOC),
        ("large", LARGE_DOC),
    ] {
        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), content, |b, content| {
            b.iter(|| {
                extractor.generate_cache_key(black_box(content))
            });
        });
    }

    group.finish();
}

fn bench_keyword_cleaning(c: &mut Criterion) {
    let mut group = c.benchmark_group("keyword_cleaning");

    let config = KeywordExtractorConfig::default();
    let extractor = KeywordExtractor::new(config).unwrap();

    let keywords_small: Vec<String> = vec![
        "rust".to_string(),
        "Rust".to_string(),
        "cargo".to_string(),
        "async".to_string(),
        "  tokio  ".to_string(),
    ];

    let keywords_large: Vec<String> = (0..100)
        .map(|i| format!("keyword_{}", i))
        .collect();

    group.bench_function("small_5_keywords", |b| {
        b.iter(|| {
            extractor.clean_keywords(black_box(keywords_small.clone()), 10)
        });
    });

    group.bench_function("large_100_keywords", |b| {
        b.iter(|| {
            extractor.clean_keywords(black_box(keywords_large.clone()), 20)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_frequency_extraction,
    bench_cache_operations,
    bench_batch_processing,
    bench_tokenization,
    bench_cache_key_generation,
    bench_keyword_cleaning,
);

criterion_main!(benches);
