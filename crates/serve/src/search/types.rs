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
}
