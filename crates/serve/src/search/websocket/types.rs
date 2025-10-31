//! WebSocket message types for real-time search
//!
//! This module defines the message types and data structures used for
//! WebSocket-based real-time search functionality.

use crate::search::{AdvancedSearchRequest, SearchResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Client-to-server WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Start a streaming search
    StreamingSearch {
        /// Unique request ID for tracking
        request_id: Uuid,
        /// Search query
        query: Box<AdvancedSearchRequest>,
    },

    /// Subscribe to document updates matching filters
    Subscribe {
        /// Subscription ID
        subscription_id: Uuid,
        /// Filters for documents to watch
        filters: SubscriptionFilters,
    },

    /// Unsubscribe from document updates
    Unsubscribe {
        /// Subscription ID to cancel
        subscription_id: Uuid,
    },

    /// Cancel an ongoing streaming search
    CancelSearch {
        /// Request ID to cancel
        request_id: Uuid,
    },

    /// Ping message to keep connection alive
    Ping,
}

/// Server-to-client WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Streaming search result batch
    SearchBatch {
        /// Request ID this batch belongs to
        request_id: Uuid,
        /// Results in this batch
        results: Vec<SearchResult>,
        /// Whether more results are coming
        has_more: bool,
        /// Total results found (if known)
        total: Option<usize>,
    },

    /// Streaming search completed
    SearchComplete {
        /// Request ID that completed
        request_id: Uuid,
        /// Total results delivered
        total_results: usize,
    },

    /// Streaming search error
    SearchError {
        /// Request ID that failed
        request_id: Uuid,
        /// Error message
        error: String,
    },

    /// Document update notification
    DocumentUpdate {
        /// Subscription ID(s) that match this update
        subscription_ids: Vec<Uuid>,
        /// Update event
        event: DocumentUpdateEvent,
    },

    /// Subscription confirmation
    Subscribed {
        /// Subscription ID
        subscription_id: Uuid,
    },

    /// Unsubscribe confirmation
    Unsubscribed {
        /// Subscription ID
        subscription_id: Uuid,
    },

    /// Subscription error
    SubscriptionError {
        /// Subscription ID that failed
        subscription_id: Uuid,
        /// Error message
        error: String,
    },

    /// Pong response to ping
    Pong,

    /// Error message
    Error {
        /// Error message
        message: String,
    },
}

/// Document update event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum DocumentUpdateEvent {
    /// Document was created
    Created {
        /// Document ID
        document_id: Uuid,
        /// Document title
        title: String,
        /// Document category
        category: String,
        /// Repository
        repository: Option<String>,
    },

    /// Document was updated
    Updated {
        /// Document ID
        document_id: Uuid,
        /// Updated fields
        changes: DocumentChanges,
    },

    /// Document was deleted
    Deleted {
        /// Document ID
        document_id: Uuid,
    },
}

/// Document change details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentChanges {
    /// Title changed
    pub title: Option<String>,
    /// Content changed
    pub content: Option<String>,
    /// Category changed
    pub category: Option<String>,
    /// Tags changed
    pub tags: Option<Vec<String>>,
}

/// Filters for document update subscriptions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubscriptionFilters {
    /// Filter by categories
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,

    /// Filter by repositories
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repositories: Option<Vec<String>>,

    /// Filter by tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Filter by document IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ids: Option<Vec<Uuid>>,
}

impl SubscriptionFilters {
    /// Check if an update event matches the filters
    ///
    /// # Arguments
    ///
    /// * `event` - The document update event to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the event matches the filters
    ///
    /// # Examples
    ///
    /// ```
    /// use uuid::Uuid;
    /// use xze_serve::search::websocket::types::{SubscriptionFilters, DocumentUpdateEvent};
    ///
    /// let filters = SubscriptionFilters {
    ///     categories: Some(vec!["tutorial".to_string()]),
    ///     repositories: None,
    ///     tags: None,
    ///     document_ids: None,
    /// };
    ///
    /// let event = DocumentUpdateEvent::Created {
    ///     document_id: Uuid::new_v4(),
    ///     title: "Test".to_string(),
    ///     category: "tutorial".to_string(),
    ///     repository: None,
    /// };
    ///
    /// assert!(filters.matches(&event));
    /// ```
    pub fn matches(&self, event: &DocumentUpdateEvent) -> bool {
        match event {
            DocumentUpdateEvent::Created {
                document_id,
                category,
                repository,
                ..
            } => {
                self.matches_category(category)
                    && self.matches_repository(repository.as_deref())
                    && self.matches_document_id(document_id)
            }
            DocumentUpdateEvent::Updated {
                document_id,
                changes,
            } => {
                self.matches_document_id(document_id)
                    && (changes.category.is_none()
                        || self.matches_category(changes.category.as_ref().unwrap()))
            }
            DocumentUpdateEvent::Deleted { document_id } => self.matches_document_id(document_id),
        }
    }

    fn matches_category(&self, category: &str) -> bool {
        self.categories
            .as_ref()
            .map(|cats| cats.iter().any(|c| c == category))
            .unwrap_or(true)
    }

    fn matches_repository(&self, repository: Option<&str>) -> bool {
        self.repositories
            .as_ref()
            .map(|repos| {
                repository
                    .map(|r| repos.iter().any(|repo| repo == r))
                    .unwrap_or(false)
            })
            .unwrap_or(true)
    }

    fn matches_document_id(&self, document_id: &Uuid) -> bool {
        self.document_ids
            .as_ref()
            .map(|ids| ids.contains(document_id))
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::Ping;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"ping"}"#);

        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_streaming_search_message() {
        let request_id = Uuid::new_v4();
        let msg = ClientMessage::StreamingSearch {
            request_id,
            query: Box::new(AdvancedSearchRequest {
                query: "test".to_string(),
                multi_match: None,
                bool_query: None,
                filters: None,
                options: None,
                aggregations: None,
            }),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"pong"}"#);

        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_search_batch_message() {
        let request_id = Uuid::new_v4();
        let msg = ServerMessage::SearchBatch {
            request_id,
            results: vec![],
            has_more: true,
            total: Some(100),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_subscription_filters_matches_category() {
        let filters = SubscriptionFilters {
            categories: Some(vec!["tutorial".to_string()]),
            repositories: None,
            tags: None,
            document_ids: None,
        };

        let event = DocumentUpdateEvent::Created {
            document_id: Uuid::new_v4(),
            title: "Test".to_string(),
            category: "tutorial".to_string(),
            repository: None,
        };

        assert!(filters.matches(&event));
    }

    #[test]
    fn test_subscription_filters_no_match_category() {
        let filters = SubscriptionFilters {
            categories: Some(vec!["reference".to_string()]),
            repositories: None,
            tags: None,
            document_ids: None,
        };

        let event = DocumentUpdateEvent::Created {
            document_id: Uuid::new_v4(),
            title: "Test".to_string(),
            category: "tutorial".to_string(),
            repository: None,
        };

        assert!(!filters.matches(&event));
    }

    #[test]
    fn test_subscription_filters_matches_document_id() {
        let doc_id = Uuid::new_v4();
        let filters = SubscriptionFilters {
            categories: None,
            repositories: None,
            tags: None,
            document_ids: Some(vec![doc_id]),
        };

        let event = DocumentUpdateEvent::Deleted {
            document_id: doc_id,
        };

        assert!(filters.matches(&event));
    }

    #[test]
    fn test_subscription_filters_empty_matches_all() {
        let filters = SubscriptionFilters {
            categories: None,
            repositories: None,
            tags: None,
            document_ids: None,
        };

        let event = DocumentUpdateEvent::Created {
            document_id: Uuid::new_v4(),
            title: "Test".to_string(),
            category: "tutorial".to_string(),
            repository: None,
        };

        assert!(filters.matches(&event));
    }

    #[test]
    fn test_document_update_event_created() {
        let event = DocumentUpdateEvent::Created {
            document_id: Uuid::new_v4(),
            title: "New Document".to_string(),
            category: "tutorial".to_string(),
            repository: Some("repo1".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: DocumentUpdateEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, event);
    }

    #[test]
    fn test_document_changes_serialization() {
        let changes = DocumentChanges {
            title: Some("Updated Title".to_string()),
            content: None,
            category: Some("reference".to_string()),
            tags: Some(vec!["rust".to_string(), "async".to_string()]),
        };

        let json = serde_json::to_string(&changes).unwrap();
        let deserialized: DocumentChanges = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, changes);
    }
}
