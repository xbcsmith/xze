//! Analytics event collector
//!
//! This module provides the infrastructure for collecting and storing analytics events.
//! Events are buffered in memory and periodically flushed to storage.

use super::types::{AnalyticsEvent, ResultClickEvent, SearchQueryEvent, SessionEvent};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Analytics collector for tracking search events
#[derive(Debug, Clone)]
pub struct AnalyticsCollector {
    /// Internal state
    state: Arc<RwLock<CollectorState>>,
    /// Configuration
    config: CollectorConfig,
}

/// Internal collector state
#[derive(Debug)]
struct CollectorState {
    /// Buffered events waiting to be flushed
    buffer: Vec<AnalyticsEvent>,
    /// Total events collected
    total_events: usize,
    /// Total events flushed
    total_flushed: usize,
    /// Total flush errors
    total_errors: usize,
}

/// Collector configuration
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    /// Maximum buffer size before automatic flush
    pub max_buffer_size: usize,
    /// Flush interval in seconds
    pub flush_interval_secs: u64,
    /// Enable debug logging
    pub debug: bool,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            max_buffer_size: 1000,
            flush_interval_secs: 60,
            debug: false,
        }
    }
}

impl AnalyticsCollector {
    /// Creates a new analytics collector
    ///
    /// # Arguments
    ///
    /// * `config` - Collector configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::analytics::collector::{AnalyticsCollector, CollectorConfig};
    ///
    /// let config = CollectorConfig::default();
    /// let collector = AnalyticsCollector::new(config);
    /// ```
    pub fn new(config: CollectorConfig) -> Self {
        let state = CollectorState {
            buffer: Vec::with_capacity(config.max_buffer_size),
            total_events: 0,
            total_flushed: 0,
            total_errors: 0,
        };

        Self {
            state: Arc::new(RwLock::new(state)),
            config,
        }
    }

    /// Creates a new collector with default configuration
    pub fn default_config() -> Self {
        Self::new(CollectorConfig::default())
    }

    /// Tracks an analytics event
    ///
    /// # Arguments
    ///
    /// * `event` - The analytics event to track
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::analytics::collector::AnalyticsCollector;
    /// use xze_serve::search::analytics::types::{AnalyticsEvent, SearchQueryEvent, QueryType};
    /// use chrono::Utc;
    ///
    /// # async fn example() {
    /// let collector = AnalyticsCollector::default_config();
    ///
    /// let event = SearchQueryEvent {
    ///     query_id: "q123".to_string(),
    ///     session_id: "s456".to_string(),
    ///     user_id: None,
    ///     query: "rust async".to_string(),
    ///     query_type: QueryType::Simple,
    ///     filters: None,
    ///     result_count: 10,
    ///     execution_time_ms: 150,
    ///     zero_results: false,
    ///     timestamp: Utc::now(),
    /// };
    ///
    /// collector.track(AnalyticsEvent::SearchQuery(event)).await;
    /// # }
    /// ```
    pub async fn track(&self, event: AnalyticsEvent) {
        let mut state = self.state.write().await;

        if self.config.debug {
            debug!("Tracking analytics event: {:?}", event);
        }

        state.buffer.push(event);
        state.total_events += 1;

        if state.buffer.len() >= self.config.max_buffer_size {
            info!(
                "Buffer full ({} events), triggering flush",
                state.buffer.len()
            );
            drop(state);
            self.flush().await;
        }
    }

    /// Tracks a search query event
    pub async fn track_query(&self, event: SearchQueryEvent) {
        self.track(AnalyticsEvent::SearchQuery(event)).await;
    }

    /// Tracks a result click event
    pub async fn track_click(&self, event: ResultClickEvent) {
        self.track(AnalyticsEvent::ResultClick(event)).await;
    }

    /// Tracks a session start event
    pub async fn track_session_start(&self, event: SessionEvent) {
        self.track(AnalyticsEvent::SessionStart(event)).await;
    }

    /// Tracks a session end event
    pub async fn track_session_end(&self, event: SessionEvent) {
        self.track(AnalyticsEvent::SessionEnd(event)).await;
    }

    /// Flushes buffered events to storage
    ///
    /// This method is called automatically when the buffer is full,
    /// but can also be called manually to force a flush.
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::analytics::collector::AnalyticsCollector;
    ///
    /// # async fn example() {
    /// let collector = AnalyticsCollector::default_config();
    /// collector.flush().await;
    /// # }
    /// ```
    pub async fn flush(&self) {
        let mut state = self.state.write().await;

        if state.buffer.is_empty() {
            if self.config.debug {
                debug!("No events to flush");
            }
            return;
        }

        let events = std::mem::take(&mut state.buffer);
        let count = events.len();

        if self.config.debug {
            debug!("Flushing {} events to storage", count);
        }

        // TODO: Implement actual storage (database, file, etc.)
        // For now, just log the events
        match self.flush_to_storage(&events).await {
            Ok(_) => {
                state.total_flushed += count;
                info!("Successfully flushed {} events", count);
            }
            Err(e) => {
                state.total_errors += 1;
                error!("Failed to flush events: {}", e);
                // Re-buffer events on failure (with limit to prevent memory issues)
                let rebuffer_count = events.len().min(self.config.max_buffer_size / 2);
                state.buffer.extend(events.into_iter().take(rebuffer_count));
                warn!("Re-buffered {} events after flush failure", rebuffer_count);
            }
        }
    }

    /// Flushes events to storage (placeholder implementation)
    async fn flush_to_storage(&self, events: &[AnalyticsEvent]) -> Result<(), String> {
        // TODO: Implement actual storage backend
        // Options:
        // 1. PostgreSQL database
        // 2. Time-series database (InfluxDB, TimescaleDB)
        // 3. Log aggregation system (Elasticsearch, Loki)
        // 4. File-based storage

        if self.config.debug {
            for event in events {
                debug!("Would store event: {:?}", event);
            }
        }

        // Simulate successful storage for now
        Ok(())
    }

    /// Returns collector statistics
    ///
    /// # Returns
    ///
    /// Returns `CollectorStats` containing buffer size, total events, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::analytics::collector::AnalyticsCollector;
    ///
    /// # async fn example() {
    /// let collector = AnalyticsCollector::default_config();
    /// let stats = collector.stats().await;
    /// println!("Total events: {}", stats.total_events);
    /// # }
    /// ```
    pub async fn stats(&self) -> CollectorStats {
        let state = self.state.read().await;
        CollectorStats {
            buffer_size: state.buffer.len(),
            total_events: state.total_events,
            total_flushed: state.total_flushed,
            total_errors: state.total_errors,
        }
    }

    /// Clears all buffered events without flushing
    ///
    /// This should only be used in testing or emergency scenarios.
    pub async fn clear(&self) {
        let mut state = self.state.write().await;
        state.buffer.clear();
        warn!("Cleared analytics buffer");
    }
}

/// Collector statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectorStats {
    /// Current buffer size
    pub buffer_size: usize,
    /// Total events collected
    pub total_events: usize,
    /// Total events flushed
    pub total_flushed: usize,
    /// Total flush errors
    pub total_errors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::analytics::types::QueryType;
    use chrono::Utc;

    fn create_test_query_event(query_id: &str) -> SearchQueryEvent {
        SearchQueryEvent {
            query_id: query_id.to_string(),
            session_id: "test_session".to_string(),
            user_id: None,
            query: "test query".to_string(),
            query_type: QueryType::Simple,
            filters: None,
            result_count: 10,
            execution_time_ms: 100,
            zero_results: false,
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_collector_creation() {
        let config = CollectorConfig {
            max_buffer_size: 100,
            flush_interval_secs: 30,
            debug: true,
        };

        let collector = AnalyticsCollector::new(config.clone());
        let stats = collector.stats().await;

        assert_eq!(stats.buffer_size, 0);
        assert_eq!(stats.total_events, 0);
    }

    #[tokio::test]
    async fn test_track_event() {
        let collector = AnalyticsCollector::default_config();

        let event = create_test_query_event("q1");
        collector.track(AnalyticsEvent::SearchQuery(event)).await;

        let stats = collector.stats().await;
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.buffer_size, 1);
    }

    #[tokio::test]
    async fn test_track_query_convenience_method() {
        let collector = AnalyticsCollector::default_config();

        let event = create_test_query_event("q1");
        collector.track_query(event).await;

        let stats = collector.stats().await;
        assert_eq!(stats.total_events, 1);
    }

    #[tokio::test]
    async fn test_track_click() {
        let collector = AnalyticsCollector::default_config();

        let event = ResultClickEvent {
            query_id: "q1".to_string(),
            session_id: "s1".to_string(),
            user_id: None,
            result_id: "doc1".to_string(),
            position: 0,
            time_to_click_ms: 1000,
            timestamp: Utc::now(),
        };

        collector.track_click(event).await;

        let stats = collector.stats().await;
        assert_eq!(stats.total_events, 1);
    }

    #[tokio::test]
    async fn test_multiple_events() {
        let collector = AnalyticsCollector::default_config();

        for i in 0..5 {
            let event = create_test_query_event(&format!("q{}", i));
            collector.track_query(event).await;
        }

        let stats = collector.stats().await;
        assert_eq!(stats.total_events, 5);
        assert_eq!(stats.buffer_size, 5);
    }

    #[tokio::test]
    async fn test_manual_flush() {
        let collector = AnalyticsCollector::default_config();

        let event = create_test_query_event("q1");
        collector.track_query(event).await;

        collector.flush().await;

        let stats = collector.stats().await;
        assert_eq!(stats.buffer_size, 0);
        assert_eq!(stats.total_flushed, 1);
    }

    #[tokio::test]
    async fn test_auto_flush_on_full_buffer() {
        let config = CollectorConfig {
            max_buffer_size: 3,
            flush_interval_secs: 60,
            debug: false,
        };
        let collector = AnalyticsCollector::new(config);

        // Add events up to buffer size
        for i in 0..3 {
            let event = create_test_query_event(&format!("q{}", i));
            collector.track_query(event).await;
        }

        // Buffer should be flushed automatically
        let stats = collector.stats().await;
        assert_eq!(stats.total_events, 3);
        // Buffer should be empty after auto-flush
        assert_eq!(stats.buffer_size, 0);
    }

    #[tokio::test]
    async fn test_clear_buffer() {
        let collector = AnalyticsCollector::default_config();

        let event = create_test_query_event("q1");
        collector.track_query(event).await;

        collector.clear().await;

        let stats = collector.stats().await;
        assert_eq!(stats.buffer_size, 0);
        // Total events still tracked
        assert_eq!(stats.total_events, 1);
    }

    #[tokio::test]
    async fn test_flush_empty_buffer() {
        let collector = AnalyticsCollector::default_config();

        collector.flush().await;

        let stats = collector.stats().await;
        assert_eq!(stats.buffer_size, 0);
        assert_eq!(stats.total_flushed, 0);
    }

    #[tokio::test]
    async fn test_session_tracking() {
        let collector = AnalyticsCollector::default_config();

        let start_event = SessionEvent {
            session_id: "s1".to_string(),
            user_id: Some("u1".to_string()),
            duration_ms: None,
            query_count: None,
            timestamp: Utc::now(),
        };

        collector.track_session_start(start_event.clone()).await;

        let end_event = SessionEvent {
            session_id: "s1".to_string(),
            user_id: Some("u1".to_string()),
            duration_ms: Some(60000),
            query_count: Some(5),
            timestamp: Utc::now(),
        };

        collector.track_session_end(end_event).await;

        let stats = collector.stats().await;
        assert_eq!(stats.total_events, 2);
    }

    #[tokio::test]
    async fn test_collector_stats() {
        let collector = AnalyticsCollector::default_config();

        // Track some events
        for i in 0..3 {
            let event = create_test_query_event(&format!("q{}", i));
            collector.track_query(event).await;
        }

        let stats = collector.stats().await;
        assert_eq!(stats.buffer_size, 3);
        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.total_flushed, 0);
        assert_eq!(stats.total_errors, 0);

        // Flush
        collector.flush().await;

        let stats_after = collector.stats().await;
        assert_eq!(stats_after.buffer_size, 0);
        assert_eq!(stats_after.total_flushed, 3);
    }
}
