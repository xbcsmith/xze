//! Performance benchmarks for search API
//!
//! These benchmarks measure the performance of various search operations
//! including GET requests, POST requests, filtering, and aggregations.
//!
//! Run with: cargo bench --bench search_benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use xze_serve::search::{
    AdvancedSearchRequest, AggregationRequest, SearchFilters, SearchOptions, SimilarityRange,
};

/// Benchmark basic search request validation
fn benchmark_request_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_validation");

    // Simple request
    let simple_request = AdvancedSearchRequest {
        query: "rust async programming".to_string(),
        filters: None,
        options: None,
        aggregations: None,
    };

    group.bench_function("simple_request", |b| {
        b.iter(|| {
            let request = black_box(&simple_request);
            request.validate()
        });
    });

    // Complex request with filters
    let complex_request = AdvancedSearchRequest {
        query: "documentation testing".to_string(),
        filters: Some(SearchFilters {
            categories: Some(vec![
                "tutorial".to_string(),
                "how-to".to_string(),
                "reference".to_string(),
            ]),
            similarity: Some(SimilarityRange {
                min: Some(0.6),
                max: Some(0.95),
            }),
            date_range: None,
            tags: Some(vec!["rust".to_string(), "async".to_string()]),
            repositories: Some(vec!["xze".to_string()]),
        }),
        options: Some(SearchOptions {
            max_results: Some(50),
            offset: Some(10),
            include_snippets: Some(true),
            highlight_terms: Some(true),
            group_by: Some("category".to_string()),
        }),
        aggregations: Some(AggregationRequest {
            by_category: Some(true),
            by_similarity_range: Some(true),
            by_date: Some(false),
        }),
    };

    group.bench_function("complex_request", |b| {
        b.iter(|| {
            let request = black_box(&complex_request);
            request.validate()
        });
    });

    group.finish();
}

/// Benchmark search request serialization
fn benchmark_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    let request = AdvancedSearchRequest {
        query: "comprehensive test query".to_string(),
        filters: Some(SearchFilters {
            categories: Some(vec![
                "tutorial".to_string(),
                "how-to".to_string(),
                "reference".to_string(),
                "explanation".to_string(),
            ]),
            similarity: Some(SimilarityRange {
                min: Some(0.5),
                max: Some(0.9),
            }),
            date_range: None,
            tags: Some(vec![
                "rust".to_string(),
                "documentation".to_string(),
                "testing".to_string(),
            ]),
            repositories: Some(vec!["xze-core".to_string(), "xze-serve".to_string()]),
        }),
        options: Some(SearchOptions {
            max_results: Some(100),
            offset: Some(0),
            include_snippets: Some(true),
            highlight_terms: Some(true),
            group_by: Some("category".to_string()),
        }),
        aggregations: Some(AggregationRequest {
            by_category: Some(true),
            by_similarity_range: Some(true),
            by_date: Some(true),
        }),
    };

    group.bench_function("to_json", |b| {
        b.iter(|| {
            let request = black_box(&request);
            serde_json::to_string(request).unwrap()
        });
    });

    let json = serde_json::to_string(&request).unwrap();

    group.bench_function("from_json", |b| {
        b.iter(|| {
            let json = black_box(&json);
            serde_json::from_str::<AdvancedSearchRequest>(json).unwrap()
        });
    });

    group.bench_function("roundtrip", |b| {
        b.iter(|| {
            let request = black_box(&request);
            let json = serde_json::to_string(request).unwrap();
            serde_json::from_str::<AdvancedSearchRequest>(&json).unwrap()
        });
    });

    group.finish();
}

/// Benchmark filter validation with varying complexity
fn benchmark_filter_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_validation");

    // Single category filter
    let single_category = SearchFilters {
        categories: Some(vec!["tutorial".to_string()]),
        similarity: None,
        date_range: None,
        tags: None,
        repositories: None,
    };

    group.bench_function("single_category", |b| {
        b.iter(|| {
            let filter = black_box(&single_category);
            filter.validate()
        });
    });

    // Multiple categories
    let multiple_categories = SearchFilters {
        categories: Some(vec![
            "tutorial".to_string(),
            "how-to".to_string(),
            "reference".to_string(),
            "explanation".to_string(),
        ]),
        similarity: None,
        date_range: None,
        tags: None,
        repositories: None,
    };

    group.bench_function("multiple_categories", |b| {
        b.iter(|| {
            let filter = black_box(&multiple_categories);
            filter.validate()
        });
    });

    // All filters
    let all_filters = SearchFilters {
        categories: Some(vec![
            "tutorial".to_string(),
            "how-to".to_string(),
            "reference".to_string(),
        ]),
        similarity: Some(SimilarityRange {
            min: Some(0.6),
            max: Some(0.95),
        }),
        date_range: None,
        tags: Some(vec![
            "rust".to_string(),
            "async".to_string(),
            "documentation".to_string(),
        ]),
        repositories: Some(vec!["xze".to_string(), "xze-core".to_string()]),
    };

    group.bench_function("all_filters", |b| {
        b.iter(|| {
            let filter = black_box(&all_filters);
            filter.validate()
        });
    });

    group.finish();
}

/// Benchmark similarity range validation
fn benchmark_similarity_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("similarity_validation");

    let valid_range = SimilarityRange {
        min: Some(0.5),
        max: Some(0.9),
    };

    group.bench_function("valid_range", |b| {
        b.iter(|| {
            let range = black_box(&valid_range);
            range.validate()
        });
    });

    let min_only = SimilarityRange {
        min: Some(0.7),
        max: None,
    };

    group.bench_function("min_only", |b| {
        b.iter(|| {
            let range = black_box(&min_only);
            range.validate()
        });
    });

    let max_only = SimilarityRange {
        min: None,
        max: Some(0.8),
    };

    group.bench_function("max_only", |b| {
        b.iter(|| {
            let range = black_box(&max_only);
            range.validate()
        });
    });

    let neither = SimilarityRange {
        min: None,
        max: None,
    };

    group.bench_function("neither", |b| {
        b.iter(|| {
            let range = black_box(&neither);
            range.validate()
        });
    });

    group.finish();
}

/// Benchmark search options validation
fn benchmark_options_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("options_validation");

    // Default options
    let default_options = SearchOptions::default();

    group.bench_function("default", |b| {
        b.iter(|| {
            let options = black_box(&default_options);
            options.validate()
        });
    });

    // Custom options
    let custom_options = SearchOptions {
        max_results: Some(50),
        offset: Some(10),
        include_snippets: Some(true),
        highlight_terms: Some(true),
        group_by: Some("category".to_string()),
    };

    group.bench_function("custom", |b| {
        b.iter(|| {
            let options = black_box(&custom_options);
            options.validate()
        });
    });

    // Maximum options
    let max_options = SearchOptions {
        max_results: Some(100),
        offset: Some(1000),
        include_snippets: Some(true),
        highlight_terms: Some(true),
        group_by: Some("similarity".to_string()),
    };

    group.bench_function("maximum", |b| {
        b.iter(|| {
            let options = black_box(&max_options);
            options.validate()
        });
    });

    group.finish();
}

/// Benchmark search request creation with varying sizes
fn benchmark_request_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_creation");

    for size in [1, 5, 10, 20].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let categories: Vec<String> =
                    (0..size).map(|i| format!("category_{}", i)).collect();

                let request = AdvancedSearchRequest {
                    query: "test query".to_string(),
                    filters: Some(SearchFilters {
                        categories: Some(categories),
                        similarity: Some(SimilarityRange {
                            min: Some(0.5),
                            max: Some(0.9),
                        }),
                        date_range: None,
                        tags: None,
                        repositories: None,
                    }),
                    options: Some(SearchOptions::default()),
                    aggregations: None,
                };

                black_box(request)
            });
        });
    }

    group.finish();
}

/// Benchmark query string parsing and validation
fn benchmark_query_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_parsing");

    let short_query = "rust";
    let medium_query = "rust async programming tutorial";
    let long_query = "comprehensive rust async programming tutorial with examples and documentation for beginners";

    group.bench_function("short_query", |b| {
        b.iter(|| {
            let query = black_box(short_query);
            !query.trim().is_empty()
        });
    });

    group.bench_function("medium_query", |b| {
        b.iter(|| {
            let query = black_box(medium_query);
            !query.trim().is_empty()
        });
    });

    group.bench_function("long_query", |b| {
        b.iter(|| {
            let query = black_box(long_query);
            !query.trim().is_empty()
        });
    });

    group.finish();
}

/// Benchmark category validation
fn benchmark_category_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("category_validation");

    let valid_categories = vec!["tutorial", "how-to", "reference", "explanation"];

    let invalid_categories = vec!["invalid1", "invalid2", "invalid3", "invalid4"];

    group.bench_function("valid_categories", |b| {
        b.iter(|| {
            for category in black_box(&valid_categories) {
                let lower = category.to_lowercase();
                matches!(
                    lower.as_str(),
                    "tutorial" | "how-to" | "reference" | "explanation"
                );
            }
        });
    });

    group.bench_function("invalid_categories", |b| {
        b.iter(|| {
            for category in black_box(&invalid_categories) {
                let lower = category.to_lowercase();
                matches!(
                    lower.as_str(),
                    "tutorial" | "how-to" | "reference" | "explanation"
                );
            }
        });
    });

    group.finish();
}

/// Benchmark complete request validation pipeline
fn benchmark_full_validation_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_validation");

    let simple_request = AdvancedSearchRequest {
        query: "test".to_string(),
        filters: None,
        options: None,
        aggregations: None,
    };

    group.bench_function("simple", |b| {
        b.iter(|| {
            let request = black_box(&simple_request);
            request.validate().is_ok()
        });
    });

    let moderate_request = AdvancedSearchRequest {
        query: "rust documentation".to_string(),
        filters: Some(SearchFilters {
            categories: Some(vec!["tutorial".to_string(), "reference".to_string()]),
            similarity: Some(SimilarityRange {
                min: Some(0.7),
                max: None,
            }),
            date_range: None,
            tags: None,
            repositories: None,
        }),
        options: Some(SearchOptions {
            max_results: Some(20),
            ..Default::default()
        }),
        aggregations: None,
    };

    group.bench_function("moderate", |b| {
        b.iter(|| {
            let request = black_box(&moderate_request);
            request.validate().is_ok()
        });
    });

    let complex_request = AdvancedSearchRequest {
        query: "comprehensive search query".to_string(),
        filters: Some(SearchFilters {
            categories: Some(vec![
                "tutorial".to_string(),
                "how-to".to_string(),
                "reference".to_string(),
                "explanation".to_string(),
            ]),
            similarity: Some(SimilarityRange {
                min: Some(0.6),
                max: Some(0.95),
            }),
            date_range: None,
            tags: Some(vec![
                "rust".to_string(),
                "async".to_string(),
                "documentation".to_string(),
            ]),
            repositories: Some(vec!["xze".to_string(), "xze-core".to_string()]),
        }),
        options: Some(SearchOptions {
            max_results: Some(100),
            offset: Some(50),
            include_snippets: Some(true),
            highlight_terms: Some(true),
            group_by: Some("category".to_string()),
        }),
        aggregations: Some(AggregationRequest {
            by_category: Some(true),
            by_similarity_range: Some(true),
            by_date: Some(true),
        }),
    };

    group.bench_function("complex", |b| {
        b.iter(|| {
            let request = black_box(&complex_request);
            request.validate().is_ok()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_request_validation,
    benchmark_serialization,
    benchmark_filter_validation,
    benchmark_similarity_validation,
    benchmark_options_validation,
    benchmark_request_creation,
    benchmark_query_parsing,
    benchmark_category_validation,
    benchmark_full_validation_pipeline,
);

criterion_main!(benches);
