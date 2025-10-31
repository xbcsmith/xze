//! Analytics API handlers
//!
//! This module provides HTTP handlers for analytics tracking and reporting endpoints.

use super::aggregator::AnalyticsAggregator;
use super::collector::AnalyticsCollector;
use super::types::{
    AnalyticsReportRequest, AnalyticsReportResponse, TrackEventRequest, TrackEventResponse,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, info};

/// Shared analytics state
#[derive(Debug, Clone)]
pub struct AnalyticsState {
    /// Event collector
    pub collector: AnalyticsCollector,
    /// Metrics aggregator
    pub aggregator: AnalyticsAggregator,
}

impl AnalyticsState {
    /// Creates a new analytics state with default configuration
    pub fn new() -> Self {
        Self {
            collector: AnalyticsCollector::default_config(),
            aggregator: AnalyticsAggregator::default_config(),
        }
    }

    /// Creates a new analytics state with custom configuration
    pub fn with_config(collector: AnalyticsCollector, aggregator: AnalyticsAggregator) -> Self {
        Self {
            collector,
            aggregator,
        }
    }
}

impl Default for AnalyticsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Handles tracking of analytics events
///
/// # Endpoint
///
/// `POST /api/search/analytics/track`
///
/// # Request Body
///
/// ```json
/// {
///   "event": {
///     "type": "search_query",
///     "query_id": "q123",
///     "session_id": "s456",
///     "user_id": "u789",
///     "query": "rust async",
///     "query_type": "simple",
///     "filters": null,
///     "result_count": 42,
///     "execution_time_ms": 150,
///     "zero_results": false,
///     "timestamp": "2024-01-01T00:00:00Z"
///   }
/// }
/// ```
///
/// # Response
///
/// ```json
/// {
///   "success": true,
///   "error": null
/// }
/// ```
///
/// # Errors
///
/// Returns `400 Bad Request` if the event is invalid.
/// Returns `500 Internal Server Error` if tracking fails.
pub async fn handle_track_event(
    State(state): State<Arc<AnalyticsState>>,
    Json(request): Json<TrackEventRequest>,
) -> Result<Json<TrackEventResponse>, AnalyticsError> {
    debug!("Tracking analytics event: {:?}", request.event);

    // Track the event
    state.collector.track(request.event).await;

    info!("Analytics event tracked successfully");

    Ok(Json(TrackEventResponse {
        success: true,
        error: None,
    }))
}

/// Handles analytics report generation
///
/// # Endpoint
///
/// `POST /api/search/analytics/reports`
///
/// # Request Body
///
/// ```json
/// {
///   "period": "last_24_hours",
///   "start_date": null,
///   "end_date": null,
///   "include_usage": true,
///   "include_quality": true,
///   "max_popular_queries": 10
/// }
/// ```
///
/// # Response
///
/// ```json
/// {
///   "usage": {
///     "period": "last_24_hours",
///     "total_queries": 1000,
///     "total_sessions": 250,
///     "total_users": 100,
///     "zero_result_queries": 50,
///     "avg_results_per_query": 15.5,
///     "avg_execution_time_ms": 125.0,
///     "popular_queries": [
///       {
///         "query": "rust",
///         "count": 100,
///         "avg_results": 20.0,
///         "click_through_rate": 0.75
///       }
///     ],
///     "query_type_distribution": {
///       "simple": 700,
///       "advanced": 300
///     },
///     "zero_result_examples": ["obscure query"]
///   },
///   "quality": {
///     "period": "last_24_hours",
///     "click_through_rate": 0.65,
///     "avg_click_position": 2.3,
///     "avg_time_to_click_ms": 3500.0,
///     "queries_with_clicks_pct": 0.70,
///     "avg_session_duration_ms": 180000.0,
///     "avg_queries_per_session": 3.5,
///     "relevance_score": 0.82
///   },
///   "generated_at": "2024-01-01T00:00:00Z"
/// }
/// ```
///
/// # Errors
///
/// Returns `400 Bad Request` if the request is invalid.
/// Returns `500 Internal Server Error` if report generation fails.
pub async fn handle_analytics_report(
    State(state): State<Arc<AnalyticsState>>,
    Json(request): Json<AnalyticsReportRequest>,
) -> Result<Json<AnalyticsReportResponse>, AnalyticsError> {
    debug!("Generating analytics report: {:?}", request);

    // TODO: Fetch events from storage
    // For now, we'll return empty analytics as a placeholder
    let events = vec![];

    // Filter events by period
    let filtered_events = state.aggregator.filter_by_period(
        &events,
        request.period,
        request.start_date,
        request.end_date,
    );

    info!("Filtered {} events for report", filtered_events.len());

    // Aggregate usage analytics
    let usage = if request.include_usage {
        Some(
            state
                .aggregator
                .aggregate_usage(&filtered_events, request.period),
        )
    } else {
        None
    };

    // Aggregate quality metrics
    let quality = if request.include_quality {
        Some(
            state
                .aggregator
                .aggregate_quality(&filtered_events, request.period),
        )
    } else {
        None
    };

    let response = AnalyticsReportResponse {
        usage,
        quality,
        generated_at: Utc::now(),
    };

    info!("Analytics report generated successfully");

    Ok(Json(response))
}

/// Handles collector statistics endpoint
///
/// # Endpoint
///
/// `GET /api/search/analytics/stats`
///
/// # Response
///
/// ```json
/// {
///   "buffer_size": 42,
///   "total_events": 1000,
///   "total_flushed": 950,
///   "total_errors": 2
/// }
/// ```
pub async fn handle_collector_stats(
    State(state): State<Arc<AnalyticsState>>,
) -> Result<Json<serde_json::Value>, AnalyticsError> {
    debug!("Fetching collector statistics");

    let stats = state.collector.stats().await;

    Ok(Json(json!({
        "buffer_size": stats.buffer_size,
        "total_events": stats.total_events,
        "total_flushed": stats.total_flushed,
        "total_errors": stats.total_errors,
    })))
}

/// Handles manual flush endpoint
///
/// # Endpoint
///
/// `POST /api/search/analytics/flush`
///
/// # Response
///
/// ```json
/// {
///   "success": true,
///   "message": "Flush completed"
/// }
/// ```
pub async fn handle_flush(
    State(state): State<Arc<AnalyticsState>>,
) -> Result<Json<serde_json::Value>, AnalyticsError> {
    info!("Manual flush requested");

    state.collector.flush().await;

    Ok(Json(json!({
        "success": true,
        "message": "Flush completed",
    })))
}

/// Analytics error type
#[derive(Debug)]
pub enum AnalyticsError {
    /// Invalid request
    InvalidRequest(String),
    /// Internal error
    InternalError(String),
}

impl IntoResponse for AnalyticsError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AnalyticsError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AnalyticsError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::analytics::types::{
        AnalyticsEvent, QueryType, SearchQueryEvent, TimePeriod,
    };
    use axum::extract::State;
    use chrono::Utc;

    fn create_test_state() -> Arc<AnalyticsState> {
        Arc::new(AnalyticsState::new())
    }

    fn create_test_query_event() -> AnalyticsEvent {
        AnalyticsEvent::SearchQuery(SearchQueryEvent {
            query_id: "q123".to_string(),
            session_id: "s456".to_string(),
            user_id: Some("u789".to_string()),
            query: "rust async".to_string(),
            query_type: QueryType::Simple,
            filters: None,
            result_count: 10,
            execution_time_ms: 150,
            zero_results: false,
            timestamp: Utc::now(),
        })
    }

    #[tokio::test]
    async fn test_handle_track_event_success() {
        let state = create_test_state();
        let request = TrackEventRequest {
            event: create_test_query_event(),
        };

        let result = handle_track_event(State(state.clone()), Json(request)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert!(response.success);
        assert!(response.error.is_none());

        // Verify event was tracked
        let stats = state.collector.stats().await;
        assert_eq!(stats.total_events, 1);
    }

    #[tokio::test]
    async fn test_handle_analytics_report_empty() {
        let state = create_test_state();
        let request = AnalyticsReportRequest {
            period: TimePeriod::Last24Hours,
            start_date: None,
            end_date: None,
            include_usage: true,
            include_quality: true,
            max_popular_queries: Some(10),
        };

        let result = handle_analytics_report(State(state), Json(request)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert!(response.usage.is_some());
        assert!(response.quality.is_some());
    }

    #[tokio::test]
    async fn test_handle_analytics_report_usage_only() {
        let state = create_test_state();
        let request = AnalyticsReportRequest {
            period: TimePeriod::Last7Days,
            start_date: None,
            end_date: None,
            include_usage: true,
            include_quality: false,
            max_popular_queries: Some(5),
        };

        let result = handle_analytics_report(State(state), Json(request)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert!(response.usage.is_some());
        assert!(response.quality.is_none());
    }

    #[tokio::test]
    async fn test_handle_analytics_report_quality_only() {
        let state = create_test_state();
        let request = AnalyticsReportRequest {
            period: TimePeriod::Last30Days,
            start_date: None,
            end_date: None,
            include_usage: false,
            include_quality: true,
            max_popular_queries: None,
        };

        let result = handle_analytics_report(State(state), Json(request)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert!(response.usage.is_none());
        assert!(response.quality.is_some());
    }

    #[tokio::test]
    async fn test_handle_collector_stats() {
        let state = create_test_state();

        // Track some events first
        let event = create_test_query_event();
        state.collector.track(event).await;

        let result = handle_collector_stats(State(state)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert!(response.get("total_events").is_some());
        assert_eq!(response["total_events"], 1);
    }

    #[tokio::test]
    async fn test_handle_flush() {
        let state = create_test_state();

        // Track an event
        let event = create_test_query_event();
        state.collector.track(event).await;

        // Flush
        let result = handle_flush(State(state.clone())).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response["success"], true);

        // Verify buffer is cleared
        let stats = state.collector.stats().await;
        assert_eq!(stats.buffer_size, 0);
    }

    #[tokio::test]
    async fn test_analytics_state_creation() {
        let state = AnalyticsState::new();
        assert!(format!("{:?}", state).contains("AnalyticsCollector"));
    }

    #[tokio::test]
    async fn test_analytics_state_default() {
        let state = AnalyticsState::default();
        assert!(format!("{:?}", state).contains("AnalyticsAggregator"));
    }

    #[test]
    fn test_analytics_error_invalid_request() {
        let error = AnalyticsError::InvalidRequest("test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_analytics_error_internal_error() {
        let error = AnalyticsError::InternalError("test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
