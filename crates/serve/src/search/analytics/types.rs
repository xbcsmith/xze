//! Analytics types for search usage and quality metrics
//!
//! This module defines the data structures for tracking search analytics,
//! including user queries, interactions, and quality metrics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Analytics event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnalyticsEvent {
    /// A search query was executed
    SearchQuery(SearchQueryEvent),
    /// A search result was clicked
    ResultClick(ResultClickEvent),
    /// A search session started
    SessionStart(SessionEvent),
    /// A search session ended
    SessionEnd(SessionEvent),
}

/// Search query event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchQueryEvent {
    /// Unique query ID
    pub query_id: String,
    /// Session ID
    pub session_id: String,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Search query text
    pub query: String,
    /// Query type (simple or advanced)
    pub query_type: QueryType,
    /// Filters applied
    pub filters: Option<FilterSummary>,
    /// Number of results returned
    pub result_count: usize,
    /// Query execution time in milliseconds
    pub execution_time_ms: u64,
    /// Whether this was a zero-result query
    pub zero_results: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Result click event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResultClickEvent {
    /// Query ID this click relates to
    pub query_id: String,
    /// Session ID
    pub session_id: String,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Result ID that was clicked
    pub result_id: String,
    /// Position of result in search results (0-based)
    pub position: usize,
    /// Time from query to click in milliseconds
    pub time_to_click_ms: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Session event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionEvent {
    /// Session ID
    pub session_id: String,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Session duration in milliseconds (for SessionEnd)
    pub duration_ms: Option<u64>,
    /// Number of queries in session (for SessionEnd)
    pub query_count: Option<usize>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Query type classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    /// Simple GET-based search
    Simple,
    /// Advanced POST-based search
    Advanced,
    /// Multi-field search
    MultiField,
    /// Boolean search
    Boolean,
    /// Semantic similarity search
    Semantic,
}

/// Summary of filters applied to a search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterSummary {
    /// Categories filtered
    pub categories: Option<Vec<String>>,
    /// Similarity threshold applied
    pub similarity_min: Option<f32>,
    /// Date range applied
    pub date_range: bool,
    /// Tags filtered
    pub tags: Option<Vec<String>>,
    /// Repositories filtered
    pub repositories: Option<Vec<String>>,
}

/// Aggregated usage analytics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageAnalytics {
    /// Time period for these analytics
    pub period: TimePeriod,
    /// Total number of queries
    pub total_queries: usize,
    /// Total number of unique sessions
    pub total_sessions: usize,
    /// Total number of unique users
    pub total_users: usize,
    /// Number of zero-result queries
    pub zero_result_queries: usize,
    /// Average results per query
    pub avg_results_per_query: f64,
    /// Average query execution time in milliseconds
    pub avg_execution_time_ms: f64,
    /// Most popular queries
    pub popular_queries: Vec<PopularQuery>,
    /// Query type distribution
    pub query_type_distribution: HashMap<QueryType, usize>,
    /// Zero-result query examples
    pub zero_result_examples: Vec<String>,
}

/// Quality metrics for search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QualityMetrics {
    /// Time period for these metrics
    pub period: TimePeriod,
    /// Click-through rate (clicks / queries)
    pub click_through_rate: f64,
    /// Average position of clicked results
    pub avg_click_position: f64,
    /// Average time to first click in milliseconds
    pub avg_time_to_click_ms: f64,
    /// Percentage of queries with at least one click
    pub queries_with_clicks_pct: f64,
    /// Average session duration in milliseconds
    pub avg_session_duration_ms: f64,
    /// Average queries per session
    pub avg_queries_per_session: f64,
    /// Relevance score (0.0 - 1.0)
    pub relevance_score: f64,
}

/// Popular query entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PopularQuery {
    /// Query text
    pub query: String,
    /// Number of times executed
    pub count: usize,
    /// Average result count
    pub avg_results: f64,
    /// Click-through rate for this query
    pub click_through_rate: f64,
}

/// Time period for analytics
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimePeriod {
    /// Last hour
    LastHour,
    /// Last 24 hours
    Last24Hours,
    /// Last 7 days
    Last7Days,
    /// Last 30 days
    Last30Days,
    /// Custom date range
    Custom,
}

/// Analytics report request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyticsReportRequest {
    /// Time period for the report
    pub period: TimePeriod,
    /// Custom start date (if period is Custom)
    pub start_date: Option<DateTime<Utc>>,
    /// Custom end date (if period is Custom)
    pub end_date: Option<DateTime<Utc>>,
    /// Include usage analytics
    pub include_usage: bool,
    /// Include quality metrics
    pub include_quality: bool,
    /// Maximum number of popular queries to return
    pub max_popular_queries: Option<usize>,
}

/// Analytics report response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyticsReportResponse {
    /// Usage analytics (if requested)
    pub usage: Option<UsageAnalytics>,
    /// Quality metrics (if requested)
    pub quality: Option<QualityMetrics>,
    /// Report generation timestamp
    pub generated_at: DateTime<Utc>,
}

/// Track event request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackEventRequest {
    /// Analytics event to track
    pub event: AnalyticsEvent,
}

/// Track event response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackEventResponse {
    /// Whether tracking succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_event_serialization() {
        let event = SearchQueryEvent {
            query_id: "q123".to_string(),
            session_id: "s456".to_string(),
            user_id: Some("u789".to_string()),
            query: "rust async".to_string(),
            query_type: QueryType::Simple,
            filters: None,
            result_count: 42,
            execution_time_ms: 150,
            zero_results: false,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: SearchQueryEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_result_click_event_serialization() {
        let event = ResultClickEvent {
            query_id: "q123".to_string(),
            session_id: "s456".to_string(),
            user_id: None,
            result_id: "doc789".to_string(),
            position: 3,
            time_to_click_ms: 2500,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ResultClickEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_analytics_event_enum_serialization() {
        let query_event = SearchQueryEvent {
            query_id: "q1".to_string(),
            session_id: "s1".to_string(),
            user_id: None,
            query: "test".to_string(),
            query_type: QueryType::Advanced,
            filters: Some(FilterSummary {
                categories: Some(vec!["tutorial".to_string()]),
                similarity_min: Some(0.7),
                date_range: false,
                tags: None,
                repositories: None,
            }),
            result_count: 10,
            execution_time_ms: 100,
            zero_results: false,
            timestamp: Utc::now(),
        };

        let event = AnalyticsEvent::SearchQuery(query_event);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"search_query"#));

        let deserialized: AnalyticsEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, AnalyticsEvent::SearchQuery(_)));
    }

    #[test]
    fn test_usage_analytics_structure() {
        let analytics = UsageAnalytics {
            period: TimePeriod::Last24Hours,
            total_queries: 1000,
            total_sessions: 250,
            total_users: 100,
            zero_result_queries: 50,
            avg_results_per_query: 15.5,
            avg_execution_time_ms: 125.0,
            popular_queries: vec![PopularQuery {
                query: "rust".to_string(),
                count: 100,
                avg_results: 20.0,
                click_through_rate: 0.75,
            }],
            query_type_distribution: HashMap::from([
                (QueryType::Simple, 700),
                (QueryType::Advanced, 300),
            ]),
            zero_result_examples: vec!["obscure query".to_string()],
        };

        assert_eq!(analytics.total_queries, 1000);
        assert_eq!(analytics.zero_result_queries, 50);
    }

    #[test]
    fn test_quality_metrics_structure() {
        let metrics = QualityMetrics {
            period: TimePeriod::Last7Days,
            click_through_rate: 0.65,
            avg_click_position: 2.3,
            avg_time_to_click_ms: 3500.0,
            queries_with_clicks_pct: 0.70,
            avg_session_duration_ms: 180000.0,
            avg_queries_per_session: 3.5,
            relevance_score: 0.82,
        };

        assert!(metrics.click_through_rate > 0.0 && metrics.click_through_rate <= 1.0);
        assert!(metrics.relevance_score >= 0.0 && metrics.relevance_score <= 1.0);
    }

    #[test]
    fn test_analytics_report_request() {
        let request = AnalyticsReportRequest {
            period: TimePeriod::Last30Days,
            start_date: None,
            end_date: None,
            include_usage: true,
            include_quality: true,
            max_popular_queries: Some(10),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: AnalyticsReportRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_filter_summary() {
        let summary = FilterSummary {
            categories: Some(vec!["tutorial".to_string(), "reference".to_string()]),
            similarity_min: Some(0.8),
            date_range: true,
            tags: Some(vec!["rust".to_string()]),
            repositories: None,
        };

        assert_eq!(summary.categories.as_ref().unwrap().len(), 2);
        assert_eq!(summary.similarity_min, Some(0.8));
        assert!(summary.date_range);
    }

    #[test]
    fn test_query_type_enum() {
        let types = vec![
            QueryType::Simple,
            QueryType::Advanced,
            QueryType::MultiField,
            QueryType::Boolean,
            QueryType::Semantic,
        ];

        for query_type in types {
            let json = serde_json::to_string(&query_type).unwrap();
            let deserialized: QueryType = serde_json::from_str(&json).unwrap();
            assert_eq!(query_type, deserialized);
        }
    }

    #[test]
    fn test_time_period_enum() {
        let periods = vec![
            TimePeriod::LastHour,
            TimePeriod::Last24Hours,
            TimePeriod::Last7Days,
            TimePeriod::Last30Days,
            TimePeriod::Custom,
        ];

        for period in periods {
            let json = serde_json::to_string(&period).unwrap();
            let deserialized: TimePeriod = serde_json::from_str(&json).unwrap();
            assert_eq!(period, deserialized);
        }
    }
}
