//! Search API types and request/response structures
//!
//! This module defines all the types used for the hybrid GET/POST search API,
//! including request payloads, response structures, filters, options, and
//! aggregation types.
//!
//! # Architecture
//!
//! The search API supports two modes:
//! - GET `/search?q=query&limit=10` - Simple search with query parameters
//! - POST `/search/advanced` - Advanced search with JSON body
//!
//! # Examples
//!
//! ```rust
//! use xze_serve::search::types::{AdvancedSearchRequest, SearchFilters, SearchOptions};
//!
//! let request = AdvancedSearchRequest {
//!     query: "rust documentation".to_string(),
//!     filters: Some(SearchFilters {
//!         categories: Some(vec!["tutorial".to_string()]),
//!         similarity: None,
//!         date_range: None,
//!         tags: None,
//!         repositories: None,
//!     }),
//!     options: Some(SearchOptions {
//!         max_results: Some(50),
//!         offset: Some(0),
//!         include_snippets: Some(true),
//!         highlight_terms: Some(true),
//!         group_by: None,
//!     }),
//!     aggregations: None,
//! };
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "openapi")]
#[allow(unused_imports)]
use serde_json::json;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Errors that can occur during search operations
#[derive(Error, Debug)]
pub enum SearchError {
    /// Invalid query provided
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Invalid filter parameters
    #[error("Invalid filter: {0}")]
    InvalidFilter(String),

    /// Invalid search options
    #[error("Invalid options: {0}")]
    InvalidOptions(String),

    /// Database error during search
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Internal search error
    #[error("Internal search error: {0}")]
    InternalError(String),
}

/// Result type for search operations
pub type Result<T> = std::result::Result<T, SearchError>;

/// Advanced search request structure for POST endpoint
///
/// This structure supports complex queries with filters, options, and aggregations.
///
/// # Examples
///
/// ```rust
/// use xze_serve::search::types::AdvancedSearchRequest;
///
/// let request = AdvancedSearchRequest {
///     query: "rust async".to_string(),
///     filters: None,
///     options: None,
///     aggregations: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "query": "rust async programming",
    "filters": {
        "categories": ["tutorial", "reference"],
        "similarity": {"min": 0.7}
    },
    "options": {
        "max_results": 20,
        "include_snippets": true
    }
})))]
pub struct AdvancedSearchRequest {
    /// The search query string
    pub query: String,

    /// Optional filters to narrow results
    pub filters: Option<SearchFilters>,

    /// Optional search options (pagination, snippets, etc.)
    pub options: Option<SearchOptions>,

    /// Optional aggregations to compute
    pub aggregations: Option<AggregationRequest>,
}

impl AdvancedSearchRequest {
    /// Validates the search request
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidQuery` if query is empty
    /// Returns `SearchError::InvalidFilter` if filters are invalid
    /// Returns `SearchError::InvalidOptions` if options are invalid
    pub fn validate(&self) -> Result<()> {
        // Validate query
        if self.query.trim().is_empty() {
            return Err(SearchError::InvalidQuery(
                "Query cannot be empty".to_string(),
            ));
        }

        // Validate filters
        if let Some(ref filters) = self.filters {
            filters.validate()?;
        }

        // Validate options
        if let Some(ref options) = self.options {
            options.validate()?;
        }

        Ok(())
    }
}

/// Search filters to narrow down results
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchFilters {
    /// Filter by documentation categories (tutorial, how-to, explanation, reference)
    pub categories: Option<Vec<String>>,

    /// Filter by similarity score range
    pub similarity: Option<SimilarityRange>,

    /// Filter by date range
    pub date_range: Option<DateRange>,

    /// Filter by tags
    pub tags: Option<Vec<String>>,

    /// Filter by repository names
    pub repositories: Option<Vec<String>>,
}

impl SearchFilters {
    /// Validates the search filters
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidFilter` if any filter is invalid
    pub fn validate(&self) -> Result<()> {
        // Validate categories
        if let Some(ref categories) = self.categories {
            if categories.is_empty() {
                return Err(SearchError::InvalidFilter(
                    "Categories list cannot be empty".to_string(),
                ));
            }
            for category in categories {
                if !is_valid_category(category) {
                    return Err(SearchError::InvalidFilter(format!(
                        "Invalid category: {}",
                        category
                    )));
                }
            }
        }

        // Validate similarity range
        if let Some(ref similarity) = self.similarity {
            similarity.validate()?;
        }

        // Validate date range
        if let Some(ref date_range) = self.date_range {
            date_range.validate()?;
        }

        Ok(())
    }
}

/// Search options for controlling result format and pagination
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchOptions {
    /// Maximum number of results to return (default: 20, max: 100)
    pub max_results: Option<usize>,

    /// Offset for pagination (default: 0)
    pub offset: Option<usize>,

    /// Include content snippets in results (default: true)
    pub include_snippets: Option<bool>,

    /// Highlight search terms in snippets (default: false)
    pub highlight_terms: Option<bool>,

    /// Group results by field (e.g., "repository", "category")
    pub group_by: Option<String>,
}

impl SearchOptions {
    /// Validates the search options
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidOptions` if any option is invalid
    pub fn validate(&self) -> Result<()> {
        // Validate max_results
        if let Some(max_results) = self.max_results {
            if max_results == 0 {
                return Err(SearchError::InvalidOptions(
                    "max_results must be greater than 0".to_string(),
                ));
            }
            if max_results > 100 {
                return Err(SearchError::InvalidOptions(
                    "max_results cannot exceed 100".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Gets the max_results value or default
    pub fn get_max_results(&self) -> usize {
        self.max_results.unwrap_or(20)
    }

    /// Gets the offset value or default
    pub fn get_offset(&self) -> usize {
        self.offset.unwrap_or(0)
    }

    /// Gets whether to include snippets
    pub fn get_include_snippets(&self) -> bool {
        self.include_snippets.unwrap_or(true)
    }

    /// Gets whether to highlight terms
    pub fn get_highlight_terms(&self) -> bool {
        self.highlight_terms.unwrap_or(false)
    }
}

/// Aggregation request for computing statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AggregationRequest {
    /// Aggregate results by category
    pub by_category: Option<bool>,

    /// Aggregate results by similarity range
    pub by_similarity_range: Option<bool>,

    /// Aggregate results by date
    pub by_date: Option<bool>,
}

/// Similarity score range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SimilarityRange {
    /// Minimum similarity score (0.0 to 1.0)
    pub min: Option<f32>,

    /// Maximum similarity score (0.0 to 1.0)
    pub max: Option<f32>,
}

impl SimilarityRange {
    /// Validates the similarity range
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidFilter` if range is invalid
    pub fn validate(&self) -> Result<()> {
        if let Some(min) = self.min {
            if !(0.0..=1.0).contains(&min) {
                return Err(SearchError::InvalidFilter(
                    "Similarity min must be between 0.0 and 1.0".to_string(),
                ));
            }
        }

        if let Some(max) = self.max {
            if !(0.0..=1.0).contains(&max) {
                return Err(SearchError::InvalidFilter(
                    "Similarity max must be between 0.0 and 1.0".to_string(),
                ));
            }
        }

        if let (Some(min), Some(max)) = (self.min, self.max) {
            if min > max {
                return Err(SearchError::InvalidFilter(
                    "Similarity min cannot be greater than max".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Date range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct DateRange {
    /// Start date (inclusive)
    pub start: Option<DateTime<Utc>>,

    /// End date (inclusive)
    pub end: Option<DateTime<Utc>>,
}

impl DateRange {
    /// Validates the date range
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidFilter` if range is invalid
    pub fn validate(&self) -> Result<()> {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            if start > end {
                return Err(SearchError::InvalidFilter(
                    "Start date cannot be after end date".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Search response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchResponse {
    /// The original query
    pub query: String,

    /// The search results
    pub results: Vec<SearchResult>,

    /// Total number of results (before pagination)
    pub total_results: usize,

    /// Aggregations (if requested)
    pub aggregations: Option<AggregationResponse>,

    /// Pagination information
    pub pagination: PaginationInfo,
}

/// Individual search result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchResult {
    /// Unique identifier
    pub id: String,

    /// Document title
    pub title: String,

    /// Full content
    pub content: String,

    /// Content snippet (if requested)
    pub snippet: Option<String>,

    /// Documentation category
    pub category: String,

    /// Similarity score (0.0 to 1.0)
    pub similarity: Option<f32>,

    /// Repository name
    pub repository: String,

    /// File path within repository
    pub path: String,

    /// Tags associated with the document
    pub tags: Vec<String>,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Aggregation response structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AggregationResponse {
    /// Results grouped by category
    pub by_category: Option<Vec<CategoryCount>>,

    /// Results grouped by similarity range
    pub by_similarity_range: Option<Vec<SimilarityRangeCount>>,

    /// Results grouped by date
    pub by_date: Option<Vec<DateCount>>,
}

/// Category count for aggregations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CategoryCount {
    /// Category name
    pub category: String,

    /// Number of results in this category
    pub count: usize,
}

/// Similarity range count for aggregations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SimilarityRangeCount {
    /// Similarity range
    pub range: String,

    /// Number of results in this range
    pub count: usize,
}

/// Date count for aggregations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct DateCount {
    /// Date period (e.g., "2024-01")
    pub period: String,

    /// Number of results in this period
    pub count: usize,
}

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PaginationInfo {
    /// Current offset
    pub offset: usize,

    /// Number of results per page
    pub limit: usize,

    /// Total number of results
    pub total: usize,

    /// Whether there are more results
    pub has_more: bool,
}

impl PaginationInfo {
    /// Creates new pagination info
    pub fn new(offset: usize, limit: usize, total: usize) -> Self {
        let has_more = offset + limit < total;
        Self {
            offset,
            limit,
            total,
            has_more,
        }
    }
}

/// Validates if a category is valid according to Diataxis framework
fn is_valid_category(category: &str) -> bool {
    matches!(
        category.to_lowercase().as_str(),
        "tutorial" | "how-to" | "howto" | "explanation" | "reference"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_search_request_validation_success() {
        let request = AdvancedSearchRequest {
            query: "test query".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_advanced_search_request_validation_empty_query() {
        let request = AdvancedSearchRequest {
            query: "   ".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_similarity_range_validation() {
        let valid_range = SimilarityRange {
            min: Some(0.5),
            max: Some(0.9),
        };
        assert!(valid_range.validate().is_ok());

        let invalid_range = SimilarityRange {
            min: Some(0.9),
            max: Some(0.5),
        };
        assert!(invalid_range.validate().is_err());

        let out_of_bounds = SimilarityRange {
            min: Some(-0.1),
            max: Some(1.5),
        };
        assert!(out_of_bounds.validate().is_err());
    }

    #[test]
    fn test_search_options_max_results_range() {
        let valid_options = SearchOptions {
            max_results: Some(50),
            ..Default::default()
        };
        assert!(valid_options.validate().is_ok());

        let too_large = SearchOptions {
            max_results: Some(200),
            ..Default::default()
        };
        assert!(too_large.validate().is_err());

        let zero = SearchOptions {
            max_results: Some(0),
            ..Default::default()
        };
        assert!(zero.validate().is_err());
    }

    #[test]
    fn test_search_options_defaults() {
        let options = SearchOptions::default();
        assert_eq!(options.get_max_results(), 20);
        assert_eq!(options.get_offset(), 0);
        assert!(options.get_include_snippets());
        assert!(!options.get_highlight_terms());
    }

    #[test]
    fn test_date_range_validation() {
        let now = Utc::now();
        let past = now - chrono::Duration::days(7);

        let valid_range = DateRange {
            start: Some(past),
            end: Some(now),
        };
        assert!(valid_range.validate().is_ok());

        let invalid_range = DateRange {
            start: Some(now),
            end: Some(past),
        };
        assert!(invalid_range.validate().is_err());
    }

    #[test]
    fn test_is_valid_category() {
        assert!(is_valid_category("tutorial"));
        assert!(is_valid_category("Tutorial"));
        assert!(is_valid_category("how-to"));
        assert!(is_valid_category("explanation"));
        assert!(is_valid_category("reference"));
        assert!(!is_valid_category("invalid"));
    }

    #[test]
    fn test_search_filters_validation() {
        let valid_filters = SearchFilters {
            categories: Some(vec!["tutorial".to_string(), "reference".to_string()]),
            ..Default::default()
        };
        assert!(valid_filters.validate().is_ok());

        let invalid_categories = SearchFilters {
            categories: Some(vec!["invalid_category".to_string()]),
            ..Default::default()
        };
        assert!(invalid_categories.validate().is_err());

        let empty_categories = SearchFilters {
            categories: Some(vec![]),
            ..Default::default()
        };
        assert!(empty_categories.validate().is_err());
    }

    #[test]
    fn test_pagination_info() {
        let pagination = PaginationInfo::new(0, 20, 100);
        assert_eq!(pagination.offset, 0);
        assert_eq!(pagination.limit, 20);
        assert_eq!(pagination.total, 100);
        assert!(pagination.has_more);

        let last_page = PaginationInfo::new(90, 20, 100);
        assert!(!last_page.has_more);
    }

    #[test]
    fn test_advanced_search_request_serialization() {
        let request = AdvancedSearchRequest {
            query: "test query".to_string(),
            filters: Some(SearchFilters {
                categories: Some(vec!["tutorial".to_string()]),
                similarity: Some(SimilarityRange {
                    min: Some(0.7),
                    max: Some(0.9),
                }),
                date_range: None,
                tags: Some(vec!["rust".to_string(), "async".to_string()]),
                repositories: Some(vec!["xze".to_string()]),
            }),
            options: Some(SearchOptions {
                max_results: Some(50),
                offset: Some(10),
                include_snippets: Some(true),
                highlight_terms: Some(false),
                group_by: Some("category".to_string()),
            }),
            aggregations: Some(AggregationRequest {
                by_category: Some(true),
                by_similarity_range: Some(false),
                by_date: Some(true),
            }),
        };

        // Test serialization
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test query"));
        assert!(json.contains("tutorial"));
        assert!(json.contains("rust"));
        assert!(json.contains("async"));

        // Test deserialization
        let deserialized: AdvancedSearchRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.query, "test query");
        assert!(deserialized.filters.is_some());
        assert!(deserialized.options.is_some());
        assert!(deserialized.aggregations.is_some());

        // Verify filter details
        let filters = deserialized.filters.unwrap();
        assert_eq!(filters.categories.as_ref().unwrap().len(), 1);
        assert_eq!(filters.tags.as_ref().unwrap().len(), 2);
        assert!(filters.similarity.is_some());
    }

    #[test]
    fn test_search_filters_serialization_roundtrip() {
        let filters = SearchFilters {
            categories: Some(vec![
                "tutorial".to_string(),
                "how-to".to_string(),
                "reference".to_string(),
            ]),
            similarity: Some(SimilarityRange {
                min: Some(0.5),
                max: Some(0.95),
            }),
            date_range: None,
            tags: Some(vec!["documentation".to_string()]),
            repositories: Some(vec!["xze-core".to_string(), "xze-serve".to_string()]),
        };

        let json = serde_json::to_string(&filters).unwrap();
        let deserialized: SearchFilters = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.categories.as_ref().unwrap().len(),
            filters.categories.as_ref().unwrap().len()
        );
        assert_eq!(
            deserialized.tags.as_ref().unwrap(),
            filters.tags.as_ref().unwrap()
        );
        assert_eq!(
            deserialized.repositories.as_ref().unwrap().len(),
            filters.repositories.as_ref().unwrap().len()
        );
    }

    #[test]
    fn test_search_options_validation_offset() {
        // Test valid offset
        let valid_offset = SearchOptions {
            offset: Some(100),
            ..Default::default()
        };
        assert!(valid_offset.validate().is_ok());

        // Test large offset
        let large_offset = SearchOptions {
            offset: Some(1000),
            ..Default::default()
        };
        assert!(large_offset.validate().is_ok());
    }

    #[test]
    fn test_similarity_range_edge_cases() {
        // Both min and max at boundaries
        let boundary_range = SimilarityRange {
            min: Some(0.0),
            max: Some(1.0),
        };
        assert!(boundary_range.validate().is_ok());

        // Only min specified
        let min_only = SimilarityRange {
            min: Some(0.5),
            max: None,
        };
        assert!(min_only.validate().is_ok());

        // Only max specified
        let max_only = SimilarityRange {
            min: None,
            max: Some(0.8),
        };
        assert!(max_only.validate().is_ok());

        // Neither specified
        let neither = SimilarityRange {
            min: None,
            max: None,
        };
        assert!(neither.validate().is_ok());

        // Equal min and max
        let equal = SimilarityRange {
            min: Some(0.7),
            max: Some(0.7),
        };
        assert!(equal.validate().is_ok());

        // Min slightly greater than max
        let invalid = SimilarityRange {
            min: Some(0.7001),
            max: Some(0.7),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_search_filters_multiple_invalid_categories() {
        let filters = SearchFilters {
            categories: Some(vec![
                "tutorial".to_string(),
                "invalid1".to_string(),
                "reference".to_string(),
                "invalid2".to_string(),
            ]),
            ..Default::default()
        };

        assert!(filters.validate().is_err());
    }

    #[test]
    fn test_search_filters_case_insensitive_categories() {
        let filters = SearchFilters {
            categories: Some(vec![
                "Tutorial".to_string(),
                "HOW-TO".to_string(),
                "Reference".to_string(),
                "EXPLANATION".to_string(),
            ]),
            ..Default::default()
        };

        assert!(filters.validate().is_ok());
    }

    #[test]
    fn test_search_options_group_by_values() {
        let by_category = SearchOptions {
            group_by: Some("category".to_string()),
            ..Default::default()
        };
        assert!(by_category.validate().is_ok());

        let by_repository = SearchOptions {
            group_by: Some("repository".to_string()),
            ..Default::default()
        };
        assert!(by_repository.validate().is_ok());

        let by_similarity = SearchOptions {
            group_by: Some("similarity".to_string()),
            ..Default::default()
        };
        assert!(by_similarity.validate().is_ok());
    }

    #[test]
    fn test_pagination_info_boundary_conditions() {
        // First page
        let first_page = PaginationInfo::new(0, 10, 100);
        assert_eq!(first_page.offset, 0);
        assert!(first_page.has_more);

        // Exact last page
        let exact_last = PaginationInfo::new(90, 10, 100);
        assert!(!exact_last.has_more);

        // Beyond last page
        let beyond = PaginationInfo::new(100, 10, 100);
        assert!(!beyond.has_more);

        // Single result
        let single = PaginationInfo::new(0, 10, 1);
        assert!(!single.has_more);

        // No results
        let empty = PaginationInfo::new(0, 10, 0);
        assert!(!empty.has_more);
    }

    #[test]
    fn test_advanced_search_request_with_all_filters() {
        let now = Utc::now();
        let past = now - chrono::Duration::days(30);

        let request = AdvancedSearchRequest {
            query: "comprehensive test".to_string(),
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
                date_range: Some(DateRange {
                    start: Some(past),
                    end: Some(now),
                }),
                tags: Some(vec![
                    "rust".to_string(),
                    "documentation".to_string(),
                    "testing".to_string(),
                ]),
                repositories: Some(vec!["xze".to_string(), "xze-core".to_string()]),
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

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_search_response_structure() {
        let response = SearchResponse {
            query: "test".to_string(),
            results: vec![],
            total_results: 0,
            aggregations: None,
            pagination: PaginationInfo::new(0, 20, 0),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"query\":\"test\""));
        assert!(json.contains("\"results\":[]"));
        assert!(json.contains("\"total_results\":0"));
    }

    #[test]
    fn test_search_result_complete_structure() {
        let result = SearchResult {
            id: "test-id-123".to_string(),
            title: "Test Document".to_string(),
            content: "Test content here".to_string(),
            snippet: Some("Test...".to_string()),
            category: "tutorial".to_string(),
            similarity: Some(0.85),
            repository: "xze".to_string(),
            path: "docs/test.md".to_string(),
            tags: vec!["rust".to_string()],
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test-id-123"));
        assert!(json.contains("Test Document"));
        assert!(json.contains("0.85"));

        let deserialized: SearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-id-123");
        assert_eq!(deserialized.similarity, Some(0.85));
    }

    #[test]
    fn test_aggregation_response_serialization() {
        let aggregations = AggregationResponse {
            by_category: Some(vec![
                CategoryCount {
                    category: "tutorial".to_string(),
                    count: 15,
                },
                CategoryCount {
                    category: "reference".to_string(),
                    count: 23,
                },
            ]),
            by_similarity_range: Some(vec![
                SimilarityRangeCount {
                    range: "0.7-0.8".to_string(),
                    count: 12,
                },
                SimilarityRangeCount {
                    range: "0.8-0.9".to_string(),
                    count: 8,
                },
            ]),
            by_date: None,
        };

        let json = serde_json::to_string(&aggregations).unwrap();
        assert!(json.contains("tutorial"));
        assert!(json.contains("15"));
        assert!(json.contains("23"));
        assert!(json.contains("0.7-0.8"));

        let deserialized: AggregationResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.by_category.as_ref().unwrap().len(), 2);
        assert_eq!(deserialized.by_similarity_range.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_date_range_edge_cases() {
        let now = Utc::now();

        // Same start and end
        let same = DateRange {
            start: Some(now),
            end: Some(now),
        };
        assert!(same.validate().is_ok());

        // Only start
        let start_only = DateRange {
            start: Some(now),
            end: None,
        };
        assert!(start_only.validate().is_ok());

        // Only end
        let end_only = DateRange {
            start: None,
            end: Some(now),
        };
        assert!(end_only.validate().is_ok());

        // Neither
        let neither = DateRange {
            start: None,
            end: None,
        };
        assert!(neither.validate().is_ok());
    }

    #[test]
    fn test_search_options_max_results_boundary() {
        let max_allowed = SearchOptions {
            max_results: Some(100),
            ..Default::default()
        };
        assert!(max_allowed.validate().is_ok());

        let over_max = SearchOptions {
            max_results: Some(101),
            ..Default::default()
        };
        assert!(over_max.validate().is_err());

        let min_allowed = SearchOptions {
            max_results: Some(1),
            ..Default::default()
        };
        assert!(min_allowed.validate().is_ok());
    }

    #[test]
    fn test_empty_query_variations() {
        let empty_string = AdvancedSearchRequest {
            query: "".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(empty_string.validate().is_err());

        let whitespace_only = AdvancedSearchRequest {
            query: "     ".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(whitespace_only.validate().is_err());

        let tabs_and_newlines = AdvancedSearchRequest {
            query: "\t\n\r".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(tabs_and_newlines.validate().is_err());

        let valid_with_whitespace = AdvancedSearchRequest {
            query: "  valid query  ".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(valid_with_whitespace.validate().is_ok());
    }
}
