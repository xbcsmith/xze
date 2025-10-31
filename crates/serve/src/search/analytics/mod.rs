//! Search analytics module
//!
//! This module provides analytics tracking and reporting for search functionality,
//! including usage statistics and quality metrics.
//!
//! # Features
//!
//! - Event tracking (queries, clicks, sessions)
//! - Usage analytics (popular queries, zero-result tracking)
//! - Quality metrics (CTR, relevance scores)
//! - Analytics reporting endpoints
//!
//! # Architecture
//!
//! The analytics module consists of:
//! - `types`: Event and metric data structures
//! - `collector`: Event collection and buffering
//! - `aggregator`: Metrics computation from events
//! - `handlers`: HTTP API handlers
//!
//! # Usage
//!
//! ```rust,no_run
//! use axum::Router;
//! use xze_serve::search::analytics::{analytics_routes, AnalyticsState};
//! use std::sync::Arc;
//!
//! # async fn example() {
//! let analytics_state = Arc::new(AnalyticsState::new());
//! let app = Router::new().nest("/api/search/analytics", analytics_routes(analytics_state));
//! # }
//! ```
//!
//! # Endpoints
//!
//! ## POST /track
//!
//! Track an analytics event:
//!
//! ```text
//! POST /api/search/analytics/track
//! Content-Type: application/json
//!
//! {
//!   "event": {
//!     "type": "search_query",
//!     "query_id": "q123",
//!     "session_id": "s456",
//!     "query": "rust async",
//!     "query_type": "simple",
//!     "result_count": 42,
//!     "execution_time_ms": 150,
//!     "zero_results": false,
//!     "timestamp": "2024-01-01T00:00:00Z"
//!   }
//! }
//! ```
//!
//! ## POST /reports
//!
//! Generate an analytics report:
//!
//! ```text
//! POST /api/search/analytics/reports
//! Content-Type: application/json
//!
//! {
//!   "period": "last_24_hours",
//!   "include_usage": true,
//!   "include_quality": true,
//!   "max_popular_queries": 10
//! }
//! ```
//!
//! ## GET /stats
//!
//! Get collector statistics:
//!
//! ```text
//! GET /api/search/analytics/stats
//! ```
//!
//! ## POST /flush
//!
//! Manually flush buffered events:
//!
//! ```text
//! POST /api/search/analytics/flush
//! ```

pub mod aggregator;
pub mod collector;
pub mod handlers;
pub mod types;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub use aggregator::{AggregatorConfig, AnalyticsAggregator};
pub use collector::{AnalyticsCollector, CollectorConfig, CollectorStats};
pub use handlers::{AnalyticsError, AnalyticsState};
pub use types::{
    AnalyticsEvent, AnalyticsReportRequest, AnalyticsReportResponse, FilterSummary, PopularQuery,
    QualityMetrics, QueryType, ResultClickEvent, SearchQueryEvent, SessionEvent, TimePeriod,
    TrackEventRequest, TrackEventResponse, UsageAnalytics,
};

/// Creates analytics routes
///
/// # Arguments
///
/// * `state` - Shared analytics state
///
/// # Returns
///
/// Returns a router with analytics endpoints mounted
///
/// # Examples
///
/// ```rust,no_run
/// use axum::Router;
/// use xze_serve::search::analytics::{analytics_routes, AnalyticsState};
/// use std::sync::Arc;
///
/// # async fn example() {
/// let analytics_state = Arc::new(AnalyticsState::new());
/// let routes = analytics_routes(analytics_state);
/// let app = Router::new().nest("/api/search/analytics", routes);
/// # }
/// ```
pub fn analytics_routes(state: Arc<AnalyticsState>) -> Router {
    Router::new()
        .route("/track", post(handlers::handle_track_event))
        .route("/reports", post(handlers::handle_analytics_report))
        .route("/stats", get(handlers::handle_collector_stats))
        .route("/flush", post(handlers::handle_flush))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify main types are exported
        let _state = AnalyticsState::new();
        let _collector = AnalyticsCollector::default_config();
        let _aggregator = AnalyticsAggregator::default_config();
    }

    #[test]
    fn test_analytics_routes_creation() {
        let state = Arc::new(AnalyticsState::new());
        let _routes = analytics_routes(state);
        // Router creation succeeds if no panic
    }
}
