//! Analytics metrics aggregator
//!
//! This module provides functionality for aggregating analytics events into
//! usage statistics and quality metrics.

use super::types::{
    AnalyticsEvent, PopularQuery, QualityMetrics, QueryType, TimePeriod, UsageAnalytics,
};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use tracing::{debug, info};

/// Analytics aggregator for computing metrics from events
#[derive(Debug, Clone)]
pub struct AnalyticsAggregator {
    /// Configuration
    config: AggregatorConfig,
}

/// Aggregator configuration
#[derive(Debug, Clone)]
pub struct AggregatorConfig {
    /// Maximum number of popular queries to track
    pub max_popular_queries: usize,
    /// Maximum number of zero-result examples to include
    pub max_zero_result_examples: usize,
    /// Enable debug logging
    pub debug: bool,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            max_popular_queries: 10,
            max_zero_result_examples: 20,
            debug: false,
        }
    }
}

impl AnalyticsAggregator {
    /// Creates a new analytics aggregator
    ///
    /// # Arguments
    ///
    /// * `config` - Aggregator configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::analytics::aggregator::{AnalyticsAggregator, AggregatorConfig};
    ///
    /// let config = AggregatorConfig::default();
    /// let aggregator = AnalyticsAggregator::new(config);
    /// ```
    pub fn new(config: AggregatorConfig) -> Self {
        Self { config }
    }

    /// Creates an aggregator with default configuration
    pub fn default_config() -> Self {
        Self::new(AggregatorConfig::default())
    }

    /// Aggregates usage analytics from events
    ///
    /// # Arguments
    ///
    /// * `events` - Analytics events to aggregate
    /// * `period` - Time period for the analytics
    ///
    /// # Returns
    ///
    /// Returns `UsageAnalytics` with aggregated statistics
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::analytics::aggregator::AnalyticsAggregator;
    /// use xze_serve::search::analytics::types::{AnalyticsEvent, TimePeriod};
    ///
    /// let aggregator = AnalyticsAggregator::default_config();
    /// let events: Vec<AnalyticsEvent> = vec![];
    /// let usage = aggregator.aggregate_usage(&events, TimePeriod::Last24Hours);
    /// ```
    pub fn aggregate_usage(&self, events: &[AnalyticsEvent], period: TimePeriod) -> UsageAnalytics {
        if self.config.debug {
            debug!(
                "Aggregating usage analytics for {:?} period from {} events",
                period,
                events.len()
            );
        }

        let mut query_count = 0;
        let mut zero_result_count = 0;
        let mut total_results = 0;
        let mut total_execution_time = 0;
        let mut sessions = HashMap::new();
        let mut users = HashMap::new();
        let mut query_counts: HashMap<String, QueryStats> = HashMap::new();
        let mut query_type_counts: HashMap<QueryType, usize> = HashMap::new();
        let mut zero_result_examples = Vec::new();

        for event in events {
            match event {
                AnalyticsEvent::SearchQuery(query) => {
                    query_count += 1;
                    total_results += query.result_count;
                    total_execution_time += query.execution_time_ms;

                    if query.zero_results {
                        zero_result_count += 1;
                        if zero_result_examples.len() < self.config.max_zero_result_examples {
                            zero_result_examples.push(query.query.clone());
                        }
                    }

                    // Track query popularity
                    let stats =
                        query_counts
                            .entry(query.query.clone())
                            .or_insert_with(|| QueryStats {
                                count: 0,
                                total_results: 0,
                                clicks: 0,
                            });
                    stats.count += 1;
                    stats.total_results += query.result_count;

                    // Track query type distribution
                    *query_type_counts.entry(query.query_type).or_insert(0) += 1;

                    // Track sessions and users
                    sessions.insert(query.session_id.clone(), ());
                    if let Some(user_id) = &query.user_id {
                        users.insert(user_id.clone(), ());
                    }
                }
                AnalyticsEvent::ResultClick(click) => {
                    // Track clicks for popular queries (need to match by session/query_id)
                    // This is simplified - in production you'd track query->click relationships
                    if self.config.debug {
                        debug!("Processing click event for query_id: {}", click.query_id);
                    }
                }
                AnalyticsEvent::SessionStart(_) | AnalyticsEvent::SessionEnd(_) => {
                    // Session events processed separately
                }
            }
        }

        // Calculate averages
        let avg_results_per_query = if query_count > 0 {
            total_results as f64 / query_count as f64
        } else {
            0.0
        };

        let avg_execution_time_ms = if query_count > 0 {
            total_execution_time as f64 / query_count as f64
        } else {
            0.0
        };

        // Build popular queries list
        let mut popular_queries: Vec<PopularQuery> = query_counts
            .into_iter()
            .map(|(query, stats)| PopularQuery {
                query,
                count: stats.count,
                avg_results: if stats.count > 0 {
                    stats.total_results as f64 / stats.count as f64
                } else {
                    0.0
                },
                click_through_rate: if stats.count > 0 {
                    stats.clicks as f64 / stats.count as f64
                } else {
                    0.0
                },
            })
            .collect();

        // Sort by count descending and take top N
        popular_queries.sort_by(|a, b| b.count.cmp(&a.count));
        popular_queries.truncate(self.config.max_popular_queries);

        if self.config.debug {
            info!(
                "Aggregated {} queries, {} sessions, {} users",
                query_count,
                sessions.len(),
                users.len()
            );
        }

        UsageAnalytics {
            period,
            total_queries: query_count,
            total_sessions: sessions.len(),
            total_users: users.len(),
            zero_result_queries: zero_result_count,
            avg_results_per_query,
            avg_execution_time_ms,
            popular_queries,
            query_type_distribution: query_type_counts,
            zero_result_examples,
        }
    }

    /// Aggregates quality metrics from events
    ///
    /// # Arguments
    ///
    /// * `events` - Analytics events to aggregate
    /// * `period` - Time period for the metrics
    ///
    /// # Returns
    ///
    /// Returns `QualityMetrics` with aggregated quality statistics
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::analytics::aggregator::AnalyticsAggregator;
    /// use xze_serve::search::analytics::types::{AnalyticsEvent, TimePeriod};
    ///
    /// let aggregator = AnalyticsAggregator::default_config();
    /// let events: Vec<AnalyticsEvent> = vec![];
    /// let quality = aggregator.aggregate_quality(&events, TimePeriod::Last7Days);
    /// ```
    pub fn aggregate_quality(
        &self,
        events: &[AnalyticsEvent],
        period: TimePeriod,
    ) -> QualityMetrics {
        if self.config.debug {
            debug!(
                "Aggregating quality metrics for {:?} period from {} events",
                period,
                events.len()
            );
        }

        let mut query_count = 0;
        let mut click_count = 0;
        let mut queries_with_clicks = HashMap::new();
        let mut total_click_position = 0;
        let mut total_time_to_click = 0;
        let mut session_durations = Vec::new();
        let mut session_query_counts: HashMap<String, usize> = HashMap::new();

        for event in events {
            match event {
                AnalyticsEvent::SearchQuery(query) => {
                    query_count += 1;
                    *session_query_counts
                        .entry(query.session_id.clone())
                        .or_insert(0) += 1;
                }
                AnalyticsEvent::ResultClick(click) => {
                    click_count += 1;
                    total_click_position += click.position;
                    total_time_to_click += click.time_to_click_ms;
                    queries_with_clicks.insert(click.query_id.clone(), ());
                }
                AnalyticsEvent::SessionEnd(session) => {
                    if let Some(duration) = session.duration_ms {
                        session_durations.push(duration);
                    }
                }
                AnalyticsEvent::SessionStart(_) => {}
            }
        }

        // Calculate click-through rate
        let click_through_rate = if query_count > 0 {
            click_count as f64 / query_count as f64
        } else {
            0.0
        };

        // Calculate average click position
        let avg_click_position = if click_count > 0 {
            total_click_position as f64 / click_count as f64
        } else {
            0.0
        };

        // Calculate average time to click
        let avg_time_to_click_ms = if click_count > 0 {
            total_time_to_click as f64 / click_count as f64
        } else {
            0.0
        };

        // Calculate queries with clicks percentage
        let queries_with_clicks_pct = if query_count > 0 {
            queries_with_clicks.len() as f64 / query_count as f64
        } else {
            0.0
        };

        // Calculate average session duration
        let avg_session_duration_ms = if !session_durations.is_empty() {
            session_durations.iter().sum::<u64>() as f64 / session_durations.len() as f64
        } else {
            0.0
        };

        // Calculate average queries per session
        let avg_queries_per_session = if !session_query_counts.is_empty() {
            session_query_counts.values().sum::<usize>() as f64 / session_query_counts.len() as f64
        } else {
            0.0
        };

        // Calculate relevance score (composite metric)
        let relevance_score = self.calculate_relevance_score(
            click_through_rate,
            avg_click_position,
            queries_with_clicks_pct,
        );

        if self.config.debug {
            info!(
                "Quality metrics: CTR={:.2}, avg_position={:.2}, relevance={:.2}",
                click_through_rate, avg_click_position, relevance_score
            );
        }

        QualityMetrics {
            period,
            click_through_rate,
            avg_click_position,
            avg_time_to_click_ms,
            queries_with_clicks_pct,
            avg_session_duration_ms,
            avg_queries_per_session,
            relevance_score,
        }
    }

    /// Filters events by time period
    ///
    /// # Arguments
    ///
    /// * `events` - All analytics events
    /// * `period` - Time period to filter by
    /// * `custom_start` - Custom start date (for Custom period)
    /// * `custom_end` - Custom end date (for Custom period)
    ///
    /// # Returns
    ///
    /// Returns filtered events within the specified period
    pub fn filter_by_period(
        &self,
        events: &[AnalyticsEvent],
        period: TimePeriod,
        custom_start: Option<DateTime<Utc>>,
        custom_end: Option<DateTime<Utc>>,
    ) -> Vec<AnalyticsEvent> {
        let now = Utc::now();
        let (start_time, end_time) = match period {
            TimePeriod::LastHour => (now - Duration::hours(1), now),
            TimePeriod::Last24Hours => (now - Duration::hours(24), now),
            TimePeriod::Last7Days => (now - Duration::days(7), now),
            TimePeriod::Last30Days => (now - Duration::days(30), now),
            TimePeriod::Custom => {
                let start = custom_start.unwrap_or(now - Duration::days(30));
                let end = custom_end.unwrap_or(now);
                (start, end)
            }
        };

        events
            .iter()
            .filter(|event| {
                let timestamp = match event {
                    AnalyticsEvent::SearchQuery(e) => e.timestamp,
                    AnalyticsEvent::ResultClick(e) => e.timestamp,
                    AnalyticsEvent::SessionStart(e) => e.timestamp,
                    AnalyticsEvent::SessionEnd(e) => e.timestamp,
                };
                timestamp >= start_time && timestamp <= end_time
            })
            .cloned()
            .collect()
    }

    /// Calculates a composite relevance score
    ///
    /// The relevance score is a weighted combination of:
    /// - Click-through rate (40%)
    /// - Inverse of average click position (30%)
    /// - Queries with clicks percentage (30%)
    ///
    /// Returns a value between 0.0 and 1.0
    fn calculate_relevance_score(
        &self,
        click_through_rate: f64,
        avg_click_position: f64,
        queries_with_clicks_pct: f64,
    ) -> f64 {
        // Normalize click position (lower is better)
        // Assume position 0-9 is reasonable range
        let position_score = if avg_click_position > 0.0 {
            (1.0 - (avg_click_position / 10.0).min(1.0)).max(0.0)
        } else {
            1.0
        };

        // Weighted average
        let score =
            (click_through_rate * 0.4) + (position_score * 0.3) + (queries_with_clicks_pct * 0.3);

        score.clamp(0.0, 1.0)
    }
}

/// Internal query statistics
#[derive(Debug, Clone)]
struct QueryStats {
    count: usize,
    total_results: usize,
    clicks: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::analytics::types::{QueryType, ResultClickEvent, SearchQueryEvent};

    fn create_test_query_event(
        query: &str,
        result_count: usize,
        zero_results: bool,
    ) -> AnalyticsEvent {
        AnalyticsEvent::SearchQuery(SearchQueryEvent {
            query_id: format!("q_{}", query),
            session_id: "s1".to_string(),
            user_id: Some("u1".to_string()),
            query: query.to_string(),
            query_type: QueryType::Simple,
            filters: None,
            result_count,
            execution_time_ms: 100,
            zero_results,
            timestamp: Utc::now(),
        })
    }

    fn create_test_click_event(
        query_id: &str,
        position: usize,
        time_to_click: u64,
    ) -> AnalyticsEvent {
        AnalyticsEvent::ResultClick(ResultClickEvent {
            query_id: query_id.to_string(),
            session_id: "s1".to_string(),
            user_id: Some("u1".to_string()),
            result_id: "doc1".to_string(),
            position,
            time_to_click_ms: time_to_click,
            timestamp: Utc::now(),
        })
    }

    #[test]
    fn test_aggregator_creation() {
        let config = AggregatorConfig::default();
        let aggregator = AnalyticsAggregator::new(config);
        assert!(format!("{:?}", aggregator).contains("AggregatorConfig"));
    }

    #[test]
    fn test_aggregate_usage_empty() {
        let aggregator = AnalyticsAggregator::default_config();
        let events = vec![];
        let usage = aggregator.aggregate_usage(&events, TimePeriod::Last24Hours);

        assert_eq!(usage.total_queries, 0);
        assert_eq!(usage.total_sessions, 0);
        assert_eq!(usage.zero_result_queries, 0);
    }

    #[test]
    fn test_aggregate_usage_basic() {
        let aggregator = AnalyticsAggregator::default_config();
        let events = vec![
            create_test_query_event("rust", 10, false),
            create_test_query_event("golang", 15, false),
            create_test_query_event("obscure", 0, true),
        ];

        let usage = aggregator.aggregate_usage(&events, TimePeriod::Last24Hours);

        assert_eq!(usage.total_queries, 3);
        assert_eq!(usage.zero_result_queries, 1);
        assert_eq!(usage.avg_results_per_query, 25.0 / 3.0);
        assert!(usage.zero_result_examples.contains(&"obscure".to_string()));
    }

    #[test]
    fn test_aggregate_usage_popular_queries() {
        let aggregator = AnalyticsAggregator::default_config();
        let events = vec![
            create_test_query_event("rust", 10, false),
            create_test_query_event("rust", 12, false),
            create_test_query_event("rust", 11, false),
            create_test_query_event("golang", 5, false),
        ];

        let usage = aggregator.aggregate_usage(&events, TimePeriod::Last24Hours);

        assert_eq!(usage.total_queries, 4);
        assert_eq!(usage.popular_queries.len(), 2);

        let top_query = &usage.popular_queries[0];
        assert_eq!(top_query.query, "rust");
        assert_eq!(top_query.count, 3);
        assert_eq!(top_query.avg_results, 11.0);
    }

    #[test]
    fn test_aggregate_quality_empty() {
        let aggregator = AnalyticsAggregator::default_config();
        let events = vec![];
        let quality = aggregator.aggregate_quality(&events, TimePeriod::Last7Days);

        assert_eq!(quality.click_through_rate, 0.0);
        assert_eq!(quality.avg_click_position, 0.0);
    }

    #[test]
    fn test_aggregate_quality_with_clicks() {
        let aggregator = AnalyticsAggregator::default_config();
        let events = vec![
            create_test_query_event("rust", 10, false),
            create_test_query_event("golang", 15, false),
            create_test_click_event("q_rust", 0, 1000),
            create_test_click_event("q_rust", 2, 2000),
        ];

        let quality = aggregator.aggregate_quality(&events, TimePeriod::Last7Days);

        assert_eq!(quality.click_through_rate, 2.0 / 2.0); // 2 clicks / 2 queries
        assert_eq!(quality.avg_click_position, 1.0); // (0 + 2) / 2
        assert_eq!(quality.avg_time_to_click_ms, 1500.0); // (1000 + 2000) / 2
    }

    #[test]
    fn test_aggregate_quality_relevance_score() {
        let aggregator = AnalyticsAggregator::default_config();

        // High quality scenario: high CTR, low position, high queries with clicks
        let score1 = aggregator.calculate_relevance_score(0.8, 0.5, 0.9);
        assert!(score1 > 0.7);

        // Low quality scenario: low CTR, high position, low queries with clicks
        let score2 = aggregator.calculate_relevance_score(0.2, 8.0, 0.3);
        assert!(score2 < 0.4);
    }

    #[test]
    fn test_filter_by_period_last_hour() {
        let aggregator = AnalyticsAggregator::default_config();
        let _now = Utc::now();

        let events = vec![create_test_query_event("recent", 10, false)];

        let filtered = aggregator.filter_by_period(&events, TimePeriod::LastHour, None, None);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_by_period_custom() {
        let aggregator = AnalyticsAggregator::default_config();
        let now = Utc::now();
        let start = now - Duration::days(2);
        let end = now + Duration::hours(1); // Add buffer to account for timing

        let events = vec![create_test_query_event("test", 10, false)];

        let filtered =
            aggregator.filter_by_period(&events, TimePeriod::Custom, Some(start), Some(end));
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_zero_result_examples_limit() {
        let config = AggregatorConfig {
            max_zero_result_examples: 2,
            ..Default::default()
        };
        let aggregator = AnalyticsAggregator::new(config);

        let events = vec![
            create_test_query_event("query1", 0, true),
            create_test_query_event("query2", 0, true),
            create_test_query_event("query3", 0, true),
        ];

        let usage = aggregator.aggregate_usage(&events, TimePeriod::Last24Hours);
        assert_eq!(usage.zero_result_examples.len(), 2);
    }

    #[test]
    fn test_popular_queries_limit() {
        let config = AggregatorConfig {
            max_popular_queries: 2,
            ..Default::default()
        };
        let aggregator = AnalyticsAggregator::new(config);

        let events = vec![
            create_test_query_event("a", 10, false),
            create_test_query_event("b", 10, false),
            create_test_query_event("c", 10, false),
            create_test_query_event("a", 10, false),
        ];

        let usage = aggregator.aggregate_usage(&events, TimePeriod::Last24Hours);
        assert_eq!(usage.popular_queries.len(), 2);
    }
}
