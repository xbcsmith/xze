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

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// The search query string (simple query mode)
    pub query: String,

    /// Optional multi-field search configuration
    pub multi_match: Option<MultiMatchQuery>,

    /// Optional boolean query with must/should/must_not operators
    pub bool_query: Option<BoolQuery>,

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
        // Validate that at least one query type is provided
        let has_simple_query = !self.query.trim().is_empty();
        let has_multi_match = self.multi_match.is_some();
        let has_bool_query = self.bool_query.is_some();

        if !has_simple_query && !has_multi_match && !has_bool_query {
            return Err(SearchError::InvalidQuery(
                "At least one query type (query, multi_match, or bool_query) must be provided"
                    .to_string(),
            ));
        }

        // Validate multi_match if present
        if let Some(ref multi_match) = self.multi_match {
            multi_match.validate()?;
        }

        // Validate bool_query if present
        if let Some(ref bool_query) = self.bool_query {
            bool_query.validate()?;
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

/// Multi-field search query configuration
///
/// Allows searching across multiple fields with optional field boosting.
///
/// # Examples
///
/// ```rust
/// use xze_serve::search::types::{MultiMatchQuery, FieldConfig};
///
/// let multi_match = MultiMatchQuery {
///     query: "rust async".to_string(),
///     fields: vec![
///         FieldConfig { name: "title".to_string(), boost: Some(2.0) },
///         FieldConfig { name: "content".to_string(), boost: Some(1.0) },
///         FieldConfig { name: "tags".to_string(), boost: Some(1.5) },
///     ],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct MultiMatchQuery {
    /// The query text to search for
    pub query: String,

    /// Fields to search with optional boosting
    pub fields: Vec<FieldConfig>,
}

impl MultiMatchQuery {
    /// Validates the multi-match query
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidQuery` if query is empty or fields are empty
    pub fn validate(&self) -> Result<()> {
        if self.query.trim().is_empty() {
            return Err(SearchError::InvalidQuery(
                "Multi-match query cannot be empty".to_string(),
            ));
        }

        if self.fields.is_empty() {
            return Err(SearchError::InvalidQuery(
                "Multi-match must specify at least one field".to_string(),
            ));
        }

        for field in &self.fields {
            field.validate()?;
        }

        Ok(())
    }
}

/// Field configuration for multi-field search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct FieldConfig {
    /// Field name (e.g., "title", "content", "tags")
    pub name: String,

    /// Optional boost factor (default 1.0)
    /// Higher values give more weight to this field
    pub boost: Option<f32>,
}

impl FieldConfig {
    /// Validates the field configuration
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidQuery` if field name is invalid or boost is invalid
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(SearchError::InvalidQuery(
                "Field name cannot be empty".to_string(),
            ));
        }

        // Validate field name is one of the supported fields
        let valid_fields = ["title", "content", "tags", "path", "repository"];
        if !valid_fields.contains(&self.name.as_str()) {
            return Err(SearchError::InvalidQuery(format!(
                "Invalid field name '{}'. Must be one of: {}",
                self.name,
                valid_fields.join(", ")
            )));
        }

        // Validate boost if present
        if let Some(boost) = self.boost {
            if boost <= 0.0 || !boost.is_finite() {
                return Err(SearchError::InvalidQuery(format!(
                    "Field boost must be positive and finite, got: {}",
                    boost
                )));
            }
        }

        Ok(())
    }

    /// Gets the boost value, defaulting to 1.0 if not specified
    pub fn get_boost(&self) -> f32 {
        self.boost.unwrap_or(1.0)
    }
}

/// Boolean query with must, should, and must_not clauses
///
/// # Examples
///
/// ```rust
/// use xze_serve::search::types::{BoolQuery, QueryClause};
///
/// let bool_query = BoolQuery {
///     must: Some(vec![
///         QueryClause::Match { field: "content".to_string(), query: "rust".to_string() },
///     ]),
///     should: Some(vec![
///         QueryClause::Match { field: "tags".to_string(), query: "async".to_string() },
///     ]),
///     must_not: Some(vec![
///         QueryClause::Match { field: "content".to_string(), query: "deprecated".to_string() },
///     ]),
///     minimum_should_match: Some(1),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct BoolQuery {
    /// Clauses that must match (AND)
    pub must: Option<Vec<QueryClause>>,

    /// Clauses that should match (OR, affects scoring)
    pub should: Option<Vec<QueryClause>>,

    /// Clauses that must not match (NOT)
    pub must_not: Option<Vec<QueryClause>>,

    /// Minimum number of should clauses that must match
    pub minimum_should_match: Option<usize>,
}

impl BoolQuery {
    /// Validates the boolean query
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidQuery` if all clauses are empty or invalid
    pub fn validate(&self) -> Result<()> {
        let has_must = self.must.as_ref().is_some_and(|v| !v.is_empty());
        let has_should = self.should.as_ref().is_some_and(|v| !v.is_empty());
        let has_must_not = self.must_not.as_ref().is_some_and(|v| !v.is_empty());

        if !has_must && !has_should && !has_must_not {
            return Err(SearchError::InvalidQuery(
                "Boolean query must have at least one clause (must, should, or must_not)"
                    .to_string(),
            ));
        }

        // Validate all clauses
        if let Some(ref must) = self.must {
            for clause in must {
                clause.validate()?;
            }
        }

        if let Some(ref should) = self.should {
            for clause in should {
                clause.validate()?;
            }

            // Validate minimum_should_match
            if let Some(min) = self.minimum_should_match {
                if min > should.len() {
                    return Err(SearchError::InvalidQuery(format!(
                        "minimum_should_match ({}) cannot exceed number of should clauses ({})",
                        min,
                        should.len()
                    )));
                }
            }
        }

        if let Some(ref must_not) = self.must_not {
            for clause in must_not {
                clause.validate()?;
            }
        }

        Ok(())
    }
}

/// Individual query clause for boolean queries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueryClause {
    /// Match query on a specific field
    Match {
        /// Field name
        field: String,
        /// Query text
        query: String,
    },
    /// Term query for exact matching
    Term {
        /// Field name
        field: String,
        /// Exact value to match
        value: String,
    },
    /// Range query for numeric or date fields
    Range {
        /// Field name
        field: String,
        /// Minimum value (inclusive)
        gte: Option<f32>,
        /// Maximum value (inclusive)
        lte: Option<f32>,
    },
}

impl QueryClause {
    /// Validates the query clause
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidQuery` if clause parameters are invalid
    pub fn validate(&self) -> Result<()> {
        match self {
            QueryClause::Match { field, query } => {
                if field.trim().is_empty() {
                    return Err(SearchError::InvalidQuery(
                        "Match clause field cannot be empty".to_string(),
                    ));
                }
                if query.trim().is_empty() {
                    return Err(SearchError::InvalidQuery(
                        "Match clause query cannot be empty".to_string(),
                    ));
                }
            }
            QueryClause::Term { field, value } => {
                if field.trim().is_empty() {
                    return Err(SearchError::InvalidQuery(
                        "Term clause field cannot be empty".to_string(),
                    ));
                }
                if value.trim().is_empty() {
                    return Err(SearchError::InvalidQuery(
                        "Term clause value cannot be empty".to_string(),
                    ));
                }
            }
            QueryClause::Range { field, gte, lte } => {
                if field.trim().is_empty() {
                    return Err(SearchError::InvalidQuery(
                        "Range clause field cannot be empty".to_string(),
                    ));
                }
                if gte.is_none() && lte.is_none() {
                    return Err(SearchError::InvalidQuery(
                        "Range clause must specify at least one of gte or lte".to_string(),
                    ));
                }
                if let (Some(min), Some(max)) = (gte, lte) {
                    if min > max {
                        return Err(SearchError::InvalidQuery(format!(
                            "Range clause gte ({}) cannot exceed lte ({})",
                            min, max
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Search filters to narrow down results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CategoryCount {
    /// Category name
    pub category: String,

    /// Number of results in this category
    pub count: usize,
}

/// Similarity range count for aggregations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SimilarityRangeCount {
    /// Similarity range
    pub range: String,

    /// Number of results in this range
    pub count: usize,
}

/// Date count for aggregations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct DateCount {
    /// Date period (e.g., "2024-01")
    pub period: String,

    /// Number of results in this period
    pub count: usize,
}

/// Pagination information supporting both offset and cursor-based pagination
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PaginationInfo {
    /// Current offset (for offset-based pagination)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,

    /// Number of results per page
    pub limit: usize,

    /// Total number of results (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,

    /// Whether there are more results
    pub has_more: bool,

    /// Cursor for next page (cursor-based pagination)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,

    /// Cursor for previous page (cursor-based pagination)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
}

impl PaginationInfo {
    /// Creates new offset-based pagination info
    pub fn new(offset: usize, limit: usize, total: usize) -> Self {
        let has_more = offset + limit < total;
        Self {
            offset: Some(offset),
            limit,
            total: Some(total),
            has_more,
            cursor: None,
            prev_cursor: None,
        }
    }

    /// Creates new cursor-based pagination info
    ///
    /// # Arguments
    ///
    /// * `limit` - Number of results per page
    /// * `has_more` - Whether there are more results
    /// * `cursor` - Cursor for the next page
    /// * `prev_cursor` - Cursor for the previous page
    ///
    /// # Returns
    ///
    /// Returns a new PaginationInfo instance for cursor-based pagination
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::types::PaginationInfo;
    ///
    /// let pagination = PaginationInfo::cursor_based(
    ///     20,
    ///     true,
    ///     Some("next_cursor_token".to_string()),
    ///     None,
    /// );
    /// ```
    pub fn cursor_based(
        limit: usize,
        has_more: bool,
        cursor: Option<String>,
        prev_cursor: Option<String>,
    ) -> Self {
        Self {
            offset: None,
            limit,
            total: None,
            has_more,
            cursor,
            prev_cursor,
        }
    }
}

/// Cursor for pagination
///
/// Encodes the position in a result set for cursor-based pagination.
/// This is more efficient than offset-based pagination for large datasets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PaginationCursor {
    /// Last seen ID
    pub last_id: i64,

    /// Last seen timestamp (for stable sorting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_timestamp: Option<chrono::DateTime<chrono::Utc>>,

    /// Direction (forward or backward)
    #[serde(default)]
    pub forward: bool,
}

impl PaginationCursor {
    /// Creates a new pagination cursor
    ///
    /// # Arguments
    ///
    /// * `last_id` - The ID of the last item in the current page
    /// * `last_timestamp` - Optional timestamp for stable sorting
    /// * `forward` - Whether this cursor is for forward pagination
    ///
    /// # Returns
    ///
    /// Returns a new PaginationCursor instance
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::types::PaginationCursor;
    /// use chrono::Utc;
    ///
    /// let cursor = PaginationCursor::new(12345, Some(Utc::now()), true);
    /// ```
    pub fn new(
        last_id: i64,
        last_timestamp: Option<chrono::DateTime<chrono::Utc>>,
        forward: bool,
    ) -> Self {
        Self {
            last_id,
            last_timestamp,
            forward,
        }
    }

    /// Encodes the cursor as a base64 string
    ///
    /// # Returns
    ///
    /// Returns a base64-encoded cursor string
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::types::PaginationCursor;
    ///
    /// let cursor = PaginationCursor::new(12345, None, true);
    /// let encoded = cursor.encode().unwrap();
    /// ```
    pub fn encode(&self) -> Result<String> {
        let json = serde_json::to_string(self)
            .map_err(|e| SearchError::InternalError(format!("Failed to encode cursor: {}", e)))?;
        Ok(STANDARD.encode(json))
    }

    /// Decodes a cursor from a base64 string
    ///
    /// # Arguments
    ///
    /// * `encoded` - Base64-encoded cursor string
    ///
    /// # Returns
    ///
    /// Returns the decoded PaginationCursor
    ///
    /// # Errors
    ///
    /// Returns an error if decoding or deserialization fails
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::types::PaginationCursor;
    ///
    /// let cursor = PaginationCursor::new(12345, None, true);
    /// let encoded = cursor.encode().unwrap();
    /// let decoded = PaginationCursor::decode(&encoded).unwrap();
    /// ```
    pub fn decode(encoded: &str) -> Result<Self> {
        let json = STANDARD
            .decode(encoded)
            .map_err(|e| SearchError::InvalidOptions(format!("Invalid cursor: {}", e)))?;
        let cursor = serde_json::from_slice(&json)
            .map_err(|e| SearchError::InvalidOptions(format!("Invalid cursor format: {}", e)))?;
        Ok(cursor)
    }
}

/// Validates if a category is valid according to Diataxis framework
fn is_valid_category(category: &str) -> bool {
    matches!(
        category.to_lowercase().as_str(),
        "tutorial" | "explanation" | "reference" | "how-to"
    )
}

/// Saved search configuration
///
/// Allows users to save frequently used search queries for quick access.
///
/// # Examples
///
/// ```rust
/// use xze_serve::search::types::{SavedSearch, AdvancedSearchRequest};
///
/// let saved = SavedSearch {
///     id: Some(1),
///     user_id: "user123".to_string(),
///     name: "Rust Async Docs".to_string(),
///     description: Some("Find async/await documentation".to_string()),
///     search_request: AdvancedSearchRequest {
///         query: "rust async".to_string(),
///         multi_match: None,
///         bool_query: None,
///         filters: None,
///         options: None,
///         aggregations: None,
///     },
///     created_at: chrono::Utc::now(),
///     updated_at: chrono::Utc::now(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SavedSearch {
    /// Unique identifier (assigned by database)
    pub id: Option<i32>,

    /// User who owns this saved search
    pub user_id: String,

    /// Human-readable name for the search
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// The search request configuration
    pub search_request: AdvancedSearchRequest,

    /// When the search was created
    pub created_at: DateTime<Utc>,

    /// When the search was last updated
    pub updated_at: DateTime<Utc>,
}

impl SavedSearch {
    /// Validates the saved search
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidQuery` if name is empty or search_request is invalid
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(SearchError::InvalidQuery(
                "Saved search name cannot be empty".to_string(),
            ));
        }

        if self.user_id.trim().is_empty() {
            return Err(SearchError::InvalidQuery(
                "User ID cannot be empty".to_string(),
            ));
        }

        // Validate the embedded search request
        self.search_request.validate()?;

        Ok(())
    }
}

/// Request to create a new saved search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateSavedSearchRequest {
    /// Human-readable name for the search
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// The search request configuration to save
    pub search_request: AdvancedSearchRequest,
}

impl CreateSavedSearchRequest {
    /// Validates the create request
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidQuery` if validation fails
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(SearchError::InvalidQuery(
                "Saved search name cannot be empty".to_string(),
            ));
        }

        self.search_request.validate()?;

        Ok(())
    }
}

/// Request to update an existing saved search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct UpdateSavedSearchRequest {
    /// Optional new name
    pub name: Option<String>,

    /// Optional new description
    pub description: Option<String>,

    /// Optional new search request configuration
    pub search_request: Option<AdvancedSearchRequest>,
}

impl UpdateSavedSearchRequest {
    /// Validates the update request
    ///
    /// # Errors
    ///
    /// Returns `SearchError::InvalidQuery` if validation fails
    pub fn validate(&self) -> Result<()> {
        if let Some(ref name) = self.name {
            if name.trim().is_empty() {
                return Err(SearchError::InvalidQuery(
                    "Saved search name cannot be empty".to_string(),
                ));
            }
        }

        if let Some(ref search_request) = self.search_request {
            search_request.validate()?;
        }

        Ok(())
    }
}

/// Response containing a list of saved searches
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SavedSearchListResponse {
    /// List of saved searches
    pub searches: Vec<SavedSearch>,

    /// Total count of saved searches for the user
    pub total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_match_query_validation() {
        let multi_match = MultiMatchQuery {
            query: "rust async".to_string(),
            fields: vec![
                FieldConfig {
                    name: "title".to_string(),
                    boost: Some(2.0),
                },
                FieldConfig {
                    name: "content".to_string(),
                    boost: Some(1.0),
                },
            ],
        };
        assert!(multi_match.validate().is_ok());

        // Empty query should fail
        let invalid_multi_match = MultiMatchQuery {
            query: "".to_string(),
            fields: vec![FieldConfig {
                name: "title".to_string(),
                boost: None,
            }],
        };
        assert!(invalid_multi_match.validate().is_err());

        // Empty fields should fail
        let invalid_multi_match = MultiMatchQuery {
            query: "test".to_string(),
            fields: vec![],
        };
        assert!(invalid_multi_match.validate().is_err());
    }

    #[test]
    fn test_field_config_validation() {
        let valid_field = FieldConfig {
            name: "content".to_string(),
            boost: Some(1.5),
        };
        assert!(valid_field.validate().is_ok());
        assert_eq!(valid_field.get_boost(), 1.5);

        // Invalid field name
        let invalid_field = FieldConfig {
            name: "invalid_field".to_string(),
            boost: Some(1.0),
        };
        assert!(invalid_field.validate().is_err());

        // Invalid boost
        let invalid_boost = FieldConfig {
            name: "title".to_string(),
            boost: Some(-1.0),
        };
        assert!(invalid_boost.validate().is_err());

        // Zero boost
        let zero_boost = FieldConfig {
            name: "title".to_string(),
            boost: Some(0.0),
        };
        assert!(zero_boost.validate().is_err());

        // Default boost
        let default_boost = FieldConfig {
            name: "title".to_string(),
            boost: None,
        };
        assert_eq!(default_boost.get_boost(), 1.0);
    }

    #[test]
    fn test_bool_query_validation() {
        let bool_query = BoolQuery {
            must: Some(vec![QueryClause::Match {
                field: "content".to_string(),
                query: "rust".to_string(),
            }]),
            should: Some(vec![QueryClause::Match {
                field: "tags".to_string(),
                query: "async".to_string(),
            }]),
            must_not: Some(vec![QueryClause::Match {
                field: "content".to_string(),
                query: "deprecated".to_string(),
            }]),
            minimum_should_match: Some(1),
        };
        assert!(bool_query.validate().is_ok());

        // Empty bool query should fail
        let empty_bool = BoolQuery {
            must: None,
            should: None,
            must_not: None,
            minimum_should_match: None,
        };
        assert!(empty_bool.validate().is_err());

        // Invalid minimum_should_match
        let invalid_min = BoolQuery {
            must: None,
            should: Some(vec![QueryClause::Match {
                field: "content".to_string(),
                query: "test".to_string(),
            }]),
            must_not: None,
            minimum_should_match: Some(5),
        };
        assert!(invalid_min.validate().is_err());
    }

    #[test]
    fn test_query_clause_validation() {
        // Valid match clause
        let match_clause = QueryClause::Match {
            field: "content".to_string(),
            query: "rust".to_string(),
        };
        assert!(match_clause.validate().is_ok());

        // Empty field
        let empty_field = QueryClause::Match {
            field: "".to_string(),
            query: "rust".to_string(),
        };
        assert!(empty_field.validate().is_err());

        // Empty query
        let empty_query = QueryClause::Match {
            field: "content".to_string(),
            query: "".to_string(),
        };
        assert!(empty_query.validate().is_err());

        // Valid term clause
        let term_clause = QueryClause::Term {
            field: "category".to_string(),
            value: "tutorial".to_string(),
        };
        assert!(term_clause.validate().is_ok());

        // Valid range clause
        let range_clause = QueryClause::Range {
            field: "similarity".to_string(),
            gte: Some(0.5),
            lte: Some(1.0),
        };
        assert!(range_clause.validate().is_ok());

        // Invalid range (gte > lte)
        let invalid_range = QueryClause::Range {
            field: "similarity".to_string(),
            gte: Some(1.0),
            lte: Some(0.5),
        };
        assert!(invalid_range.validate().is_err());

        // Range with no bounds
        let no_bounds = QueryClause::Range {
            field: "similarity".to_string(),
            gte: None,
            lte: None,
        };
        assert!(no_bounds.validate().is_err());
    }

    #[test]
    fn test_advanced_search_request_with_multi_match() {
        let request = AdvancedSearchRequest {
            query: "".to_string(),
            multi_match: Some(MultiMatchQuery {
                query: "rust async".to_string(),
                fields: vec![FieldConfig {
                    name: "title".to_string(),
                    boost: Some(2.0),
                }],
            }),
            bool_query: None,
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_advanced_search_request_with_bool_query() {
        let request = AdvancedSearchRequest {
            query: "".to_string(),
            multi_match: None,
            bool_query: Some(BoolQuery {
                must: Some(vec![QueryClause::Match {
                    field: "content".to_string(),
                    query: "rust".to_string(),
                }]),
                should: None,
                must_not: None,
                minimum_should_match: None,
            }),
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_advanced_search_request_no_query() {
        let request = AdvancedSearchRequest {
            query: "".to_string(),
            multi_match: None,
            bool_query: None,
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_advanced_search_request_validation_success() {
        let request = AdvancedSearchRequest {
            query: "test query".to_string(),
            multi_match: None,
            bool_query: None,
            filters: None,
            options: None,
            aggregations: None,
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_advanced_search_request_validation_empty_query() {
        let request = AdvancedSearchRequest {
            query: "".to_string(),
            multi_match: None,
            bool_query: None,
            filters: None,
            options: None,
            aggregations: None,
        };
        let result = request.validate();
        assert!(result.is_err());
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
        assert_eq!(pagination.offset, Some(0));
        assert_eq!(pagination.limit, 20);
        assert_eq!(pagination.total, Some(100));
        assert!(pagination.has_more);

        let last_page = PaginationInfo::new(90, 20, 100);
        assert!(!last_page.has_more);
    }

    #[test]
    fn test_advanced_search_request_serialization() {
        let request = AdvancedSearchRequest {
            query: "test query".to_string(),
            multi_match: None,
            bool_query: None,
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
        assert_eq!(first_page.offset, Some(0));
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
            multi_match: None,
            bool_query: None,
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
            multi_match: None,
            bool_query: None,
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(empty_string.validate().is_err());

        let whitespace_only = AdvancedSearchRequest {
            query: "   ".to_string(),
            multi_match: None,
            bool_query: None,
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(whitespace_only.validate().is_err());

        let tabs_and_newlines = AdvancedSearchRequest {
            query: "\t\n\r".to_string(),
            multi_match: None,
            bool_query: None,
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(tabs_and_newlines.validate().is_err());

        let valid_with_whitespace = AdvancedSearchRequest {
            query: "  valid query  ".to_string(),
            multi_match: None,
            bool_query: None,
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(valid_with_whitespace.validate().is_ok());
    }

    #[test]
    fn test_pagination_cursor_encode_decode() {
        let cursor = PaginationCursor::new(12345, None, true);
        let encoded = cursor.encode().unwrap();
        let decoded = PaginationCursor::decode(&encoded).unwrap();

        assert_eq!(decoded.last_id, 12345);
        assert_eq!(decoded.forward, true);
        assert!(decoded.last_timestamp.is_none());
    }

    #[test]
    fn test_pagination_cursor_with_timestamp() {
        let now = Utc::now();
        let cursor = PaginationCursor::new(67890, Some(now), false);
        let encoded = cursor.encode().unwrap();
        let decoded = PaginationCursor::decode(&encoded).unwrap();

        assert_eq!(decoded.last_id, 67890);
        assert_eq!(decoded.forward, false);
        assert!(decoded.last_timestamp.is_some());
        assert_eq!(decoded.last_timestamp.unwrap().timestamp(), now.timestamp());
    }

    #[test]
    fn test_pagination_cursor_invalid_decode() {
        let result = PaginationCursor::decode("invalid_base64!");
        assert!(result.is_err());

        let result = PaginationCursor::decode("YWJjZGVm"); // Valid base64 but invalid JSON
        assert!(result.is_err());
    }

    #[test]
    fn test_pagination_info_cursor_based() {
        let pagination = PaginationInfo::cursor_based(
            20,
            true,
            Some("next_cursor".to_string()),
            Some("prev_cursor".to_string()),
        );

        assert!(pagination.offset.is_none());
        assert_eq!(pagination.limit, 20);
        assert!(pagination.total.is_none());
        assert_eq!(pagination.has_more, true);
        assert_eq!(pagination.cursor.unwrap(), "next_cursor");
        assert_eq!(pagination.prev_cursor.unwrap(), "prev_cursor");
    }

    #[test]
    fn test_pagination_info_offset_based() {
        let pagination = PaginationInfo::new(10, 20, 100);

        assert_eq!(pagination.offset, Some(10));
        assert_eq!(pagination.limit, 20);
        assert_eq!(pagination.total, Some(100));
        assert_eq!(pagination.has_more, true);
        assert!(pagination.cursor.is_none());
        assert!(pagination.prev_cursor.is_none());
    }

    #[test]
    fn test_pagination_info_serialization() {
        let cursor_based =
            PaginationInfo::cursor_based(25, true, Some("cursor123".to_string()), None);

        let json = serde_json::to_string(&cursor_based).unwrap();
        assert!(json.contains("\"limit\":25"));
        assert!(json.contains("\"has_more\":true"));
        assert!(json.contains("cursor123"));
        assert!(!json.contains("offset"));
        assert!(!json.contains("total"));

        let deserialized: PaginationInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.limit, 25);
        assert!(deserialized.has_more);
        assert!(deserialized.cursor.is_some());
    }

    #[test]
    fn test_pagination_cursor_forward_backward() {
        let forward = PaginationCursor::new(100, None, true);
        assert!(forward.forward);

        let backward = PaginationCursor::new(100, None, false);
        assert!(!backward.forward);
    }

    #[test]
    fn test_pagination_info_no_more_results() {
        let no_more = PaginationInfo::cursor_based(20, false, None, Some("prev".to_string()));

        assert_eq!(no_more.has_more, false);
        assert!(no_more.cursor.is_none());
        assert!(no_more.prev_cursor.is_some());
    }

    #[test]
    fn test_pagination_offset_compatibility() {
        // Old style offset pagination should still work
        let old_style = PaginationInfo::new(0, 10, 50);
        assert!(old_style.offset.is_some());
        assert_eq!(old_style.offset, Some(0));
        assert_eq!(old_style.total, Some(50));
    }

    #[test]
    fn test_saved_search_validation() {
        let saved = SavedSearch {
            id: Some(1),
            user_id: "user123".to_string(),
            name: "Rust Async".to_string(),
            description: Some("Async docs".to_string()),
            search_request: AdvancedSearchRequest {
                query: "rust async".to_string(),
                multi_match: None,
                bool_query: None,
                filters: None,
                options: None,
                aggregations: None,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(saved.validate().is_ok());

        // Empty name should fail
        let invalid_name = SavedSearch {
            id: Some(1),
            user_id: "user123".to_string(),
            name: "".to_string(),
            description: None,
            search_request: AdvancedSearchRequest {
                query: "test".to_string(),
                multi_match: None,
                bool_query: None,
                filters: None,
                options: None,
                aggregations: None,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(invalid_name.validate().is_err());

        // Empty user_id should fail
        let invalid_user = SavedSearch {
            id: Some(1),
            user_id: "".to_string(),
            name: "Test".to_string(),
            description: None,
            search_request: AdvancedSearchRequest {
                query: "test".to_string(),
                multi_match: None,
                bool_query: None,
                filters: None,
                options: None,
                aggregations: None,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(invalid_user.validate().is_err());
    }

    #[test]
    fn test_create_saved_search_request_validation() {
        let create_request = CreateSavedSearchRequest {
            name: "My Search".to_string(),
            description: Some("Test search".to_string()),
            search_request: AdvancedSearchRequest {
                query: "rust".to_string(),
                multi_match: None,
                bool_query: None,
                filters: None,
                options: None,
                aggregations: None,
            },
        };
        assert!(create_request.validate().is_ok());

        // Empty name should fail
        let invalid = CreateSavedSearchRequest {
            name: "".to_string(),
            description: None,
            search_request: AdvancedSearchRequest {
                query: "test".to_string(),
                multi_match: None,
                bool_query: None,
                filters: None,
                options: None,
                aggregations: None,
            },
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_update_saved_search_request_validation() {
        let update_request = UpdateSavedSearchRequest {
            name: Some("New Name".to_string()),
            description: Some("Updated description".to_string()),
            search_request: None,
        };
        assert!(update_request.validate().is_ok());

        // Empty name should fail
        let invalid = UpdateSavedSearchRequest {
            name: Some("".to_string()),
            description: None,
            search_request: None,
        };
        assert!(invalid.validate().is_err());

        // None values should be valid
        let all_none = UpdateSavedSearchRequest {
            name: None,
            description: None,
            search_request: None,
        };
        assert!(all_none.validate().is_ok());
    }

    #[test]
    fn test_saved_search_list_response() {
        let response = SavedSearchListResponse {
            searches: vec![],
            total: 0,
        };
        assert_eq!(response.searches.len(), 0);
        assert_eq!(response.total, 0);

        let response_with_items = SavedSearchListResponse {
            searches: vec![SavedSearch {
                id: Some(1),
                user_id: "user123".to_string(),
                name: "Test".to_string(),
                description: None,
                search_request: AdvancedSearchRequest {
                    query: "rust".to_string(),
                    multi_match: None,
                    bool_query: None,
                    filters: None,
                    options: None,
                    aggregations: None,
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }],
            total: 1,
        };
        assert_eq!(response_with_items.searches.len(), 1);
        assert_eq!(response_with_items.total, 1);
    }

    #[test]
    fn test_saved_search_serialization() {
        let saved = SavedSearch {
            id: Some(1),
            user_id: "user123".to_string(),
            name: "Test Search".to_string(),
            description: Some("A test".to_string()),
            search_request: AdvancedSearchRequest {
                query: "rust async".to_string(),
                multi_match: None,
                bool_query: None,
                filters: None,
                options: None,
                aggregations: None,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&saved).unwrap();
        let deserialized: SavedSearch = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, saved.id);
        assert_eq!(deserialized.user_id, saved.user_id);
        assert_eq!(deserialized.name, saved.name);
        assert_eq!(deserialized.description, saved.description);
    }
}
