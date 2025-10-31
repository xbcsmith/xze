//! Connection registry for managing WebSocket connections
//!
//! This module provides a registry for tracking active WebSocket connections,
//! managing subscriptions, and broadcasting updates to connected clients.

use super::types::{DocumentUpdateEvent, ServerMessage, SubscriptionFilters};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Connection information
#[derive(Debug)]
struct ConnectionInfo {
    /// Channel for sending messages to the client
    sender: mpsc::Sender<ServerMessage>,
    /// Active subscriptions for this connection
    subscriptions: HashMap<Uuid, SubscriptionFilters>,
}

/// Registry for managing WebSocket connections
///
/// The registry tracks all active WebSocket connections and their subscriptions,
/// and provides methods for broadcasting updates to subscribed clients.
///
/// # Examples
///
/// ```rust
/// use xze_serve::search::websocket::connection::ConnectionRegistry;
/// use std::sync::Arc;
///
/// # async fn example() {
/// let registry = Arc::new(ConnectionRegistry::new());
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ConnectionRegistry {
    /// Map of connection ID to connection info
    connections: Arc<RwLock<HashMap<Uuid, ConnectionInfo>>>,
}

impl ConnectionRegistry {
    /// Create a new connection registry
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xze_serve::search::websocket::connection::ConnectionRegistry;
    ///
    /// let registry = ConnectionRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new WebSocket connection
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Unique identifier for the connection
    /// * `sender` - Channel for sending messages to the client
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use xze_serve::search::websocket::connection::ConnectionRegistry;
    /// use xze_serve::search::websocket::types::ServerMessage;
    /// use tokio::sync::mpsc;
    /// use uuid::Uuid;
    ///
    /// # async fn example() {
    /// let registry = ConnectionRegistry::new();
    /// let (tx, _rx) = mpsc::channel::<ServerMessage>(100);
    /// let connection_id = Uuid::new_v4();
    ///
    /// registry.register(connection_id, tx).await;
    /// # }
    /// ```
    pub async fn register(&self, connection_id: Uuid, sender: mpsc::Sender<ServerMessage>) {
        let mut connections = self.connections.write().await;
        connections.insert(
            connection_id,
            ConnectionInfo {
                sender,
                subscriptions: HashMap::new(),
            },
        );
        info!("Registered connection: {}", connection_id);
    }

    /// Unregister a WebSocket connection
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Connection identifier to remove
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use xze_serve::search::websocket::connection::ConnectionRegistry;
    /// use uuid::Uuid;
    ///
    /// # async fn example() {
    /// let registry = ConnectionRegistry::new();
    /// let connection_id = Uuid::new_v4();
    ///
    /// registry.unregister(connection_id).await;
    /// # }
    /// ```
    pub async fn unregister(&self, connection_id: Uuid) {
        let mut connections = self.connections.write().await;
        connections.remove(&connection_id);
        info!("Unregistered connection: {}", connection_id);
    }

    /// Add a subscription for a connection
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Connection identifier
    /// * `subscription_id` - Unique subscription identifier
    /// * `filters` - Filters for the subscription
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use xze_serve::search::websocket::connection::ConnectionRegistry;
    /// use xze_serve::search::websocket::types::SubscriptionFilters;
    /// use uuid::Uuid;
    ///
    /// # async fn example() {
    /// let registry = ConnectionRegistry::new();
    /// let connection_id = Uuid::new_v4();
    /// let subscription_id = Uuid::new_v4();
    /// let filters = SubscriptionFilters {
    ///     categories: Some(vec!["tutorial".to_string()]),
    ///     repositories: None,
    ///     tags: None,
    ///     document_ids: None,
    /// };
    ///
    /// registry.add_subscription(connection_id, subscription_id, filters).await;
    /// # }
    /// ```
    pub async fn add_subscription(
        &self,
        connection_id: Uuid,
        subscription_id: Uuid,
        filters: SubscriptionFilters,
    ) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(&connection_id) {
            conn.subscriptions.insert(subscription_id, filters);
            debug!(
                "Added subscription {} for connection {}",
                subscription_id, connection_id
            );
        } else {
            warn!(
                "Attempted to add subscription for unknown connection: {}",
                connection_id
            );
        }
    }

    /// Remove a subscription for a connection
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Connection identifier
    /// * `subscription_id` - Subscription identifier to remove
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use xze_serve::search::websocket::connection::ConnectionRegistry;
    /// use uuid::Uuid;
    ///
    /// # async fn example() {
    /// let registry = ConnectionRegistry::new();
    /// let connection_id = Uuid::new_v4();
    /// let subscription_id = Uuid::new_v4();
    ///
    /// registry.remove_subscription(connection_id, subscription_id).await;
    /// # }
    /// ```
    pub async fn remove_subscription(&self, connection_id: Uuid, subscription_id: Uuid) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(&connection_id) {
            conn.subscriptions.remove(&subscription_id);
            debug!(
                "Removed subscription {} for connection {}",
                subscription_id, connection_id
            );
        }
    }

    /// Broadcast a document update event to all matching subscriptions
    ///
    /// # Arguments
    ///
    /// * `event` - The document update event to broadcast
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use xze_serve::search::websocket::connection::ConnectionRegistry;
    /// use xze_serve::search::websocket::types::DocumentUpdateEvent;
    /// use uuid::Uuid;
    ///
    /// # async fn example() {
    /// let registry = ConnectionRegistry::new();
    /// let event = DocumentUpdateEvent::Created {
    ///     document_id: Uuid::new_v4(),
    ///     title: "New Document".to_string(),
    ///     category: "tutorial".to_string(),
    ///     repository: None,
    /// };
    ///
    /// registry.broadcast_update(event).await;
    /// # }
    /// ```
    pub async fn broadcast_update(&self, event: DocumentUpdateEvent) {
        let connections = self.connections.read().await;

        debug!("Broadcasting update to {} connections", connections.len());

        for (connection_id, conn) in connections.iter() {
            let mut matching_subscriptions = Vec::new();

            // Find all subscriptions that match this event
            for (subscription_id, filters) in conn.subscriptions.iter() {
                if filters.matches(&event) {
                    matching_subscriptions.push(*subscription_id);
                }
            }

            // Send update if any subscriptions matched
            if !matching_subscriptions.is_empty() {
                let msg = ServerMessage::DocumentUpdate {
                    subscription_ids: matching_subscriptions,
                    event: event.clone(),
                };

                if let Err(e) = conn.sender.try_send(msg) {
                    error!(
                        "Failed to send update to connection {}: {}",
                        connection_id, e
                    );
                }
            }
        }
    }

    /// Get the current number of active connections
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use xze_serve::search::websocket::connection::ConnectionRegistry;
    ///
    /// # async fn example() {
    /// let registry = ConnectionRegistry::new();
    /// let count = registry.connection_count().await;
    /// assert_eq!(count, 0);
    /// # }
    /// ```
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Get the total number of active subscriptions across all connections
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use xze_serve::search::websocket::connection::ConnectionRegistry;
    ///
    /// # async fn example() {
    /// let registry = ConnectionRegistry::new();
    /// let count = registry.subscription_count().await;
    /// assert_eq!(count, 0);
    /// # }
    /// ```
    pub async fn subscription_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections
            .values()
            .map(|conn| conn.subscriptions.len())
            .sum()
    }
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_unregister() {
        let registry = ConnectionRegistry::new();
        let (tx, _rx) = mpsc::channel(100);
        let connection_id = Uuid::new_v4();

        // Register connection
        registry.register(connection_id, tx).await;
        assert_eq!(registry.connection_count().await, 1);

        // Unregister connection
        registry.unregister(connection_id).await;
        assert_eq!(registry.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_add_remove_subscription() {
        let registry = ConnectionRegistry::new();
        let (tx, _rx) = mpsc::channel(100);
        let connection_id = Uuid::new_v4();
        let subscription_id = Uuid::new_v4();

        // Register connection
        registry.register(connection_id, tx).await;

        // Add subscription
        let filters = SubscriptionFilters {
            categories: Some(vec!["tutorial".to_string()]),
            repositories: None,
            tags: None,
            document_ids: None,
        };
        registry
            .add_subscription(connection_id, subscription_id, filters)
            .await;
        assert_eq!(registry.subscription_count().await, 1);

        // Remove subscription
        registry
            .remove_subscription(connection_id, subscription_id)
            .await;
        assert_eq!(registry.subscription_count().await, 0);
    }

    #[tokio::test]
    async fn test_broadcast_update_matching() {
        let registry = ConnectionRegistry::new();
        let (tx, mut rx) = mpsc::channel(100);
        let connection_id = Uuid::new_v4();
        let subscription_id = Uuid::new_v4();

        // Register connection with subscription
        registry.register(connection_id, tx).await;
        let filters = SubscriptionFilters {
            categories: Some(vec!["tutorial".to_string()]),
            repositories: None,
            tags: None,
            document_ids: None,
        };
        registry
            .add_subscription(connection_id, subscription_id, filters)
            .await;

        // Broadcast matching event
        let event = DocumentUpdateEvent::Created {
            document_id: Uuid::new_v4(),
            title: "Test".to_string(),
            category: "tutorial".to_string(),
            repository: None,
        };
        registry.broadcast_update(event.clone()).await;

        // Check that message was received
        let msg = rx.try_recv().unwrap();
        match msg {
            ServerMessage::DocumentUpdate {
                subscription_ids,
                event: received_event,
            } => {
                assert_eq!(subscription_ids, vec![subscription_id]);
                assert_eq!(received_event, event);
            }
            _ => panic!("Expected DocumentUpdate message"),
        }
    }

    #[tokio::test]
    async fn test_broadcast_update_no_match() {
        let registry = ConnectionRegistry::new();
        let (tx, mut rx) = mpsc::channel(100);
        let connection_id = Uuid::new_v4();
        let subscription_id = Uuid::new_v4();

        // Register connection with subscription for different category
        registry.register(connection_id, tx).await;
        let filters = SubscriptionFilters {
            categories: Some(vec!["reference".to_string()]),
            repositories: None,
            tags: None,
            document_ids: None,
        };
        registry
            .add_subscription(connection_id, subscription_id, filters)
            .await;

        // Broadcast non-matching event
        let event = DocumentUpdateEvent::Created {
            document_id: Uuid::new_v4(),
            title: "Test".to_string(),
            category: "tutorial".to_string(),
            repository: None,
        };
        registry.broadcast_update(event).await;

        // Check that no message was received
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_multiple_connections() {
        let registry = ConnectionRegistry::new();

        let (tx1, _rx1) = mpsc::channel(100);
        let (tx2, _rx2) = mpsc::channel(100);

        let conn1 = Uuid::new_v4();
        let conn2 = Uuid::new_v4();

        registry.register(conn1, tx1).await;
        registry.register(conn2, tx2).await;

        assert_eq!(registry.connection_count().await, 2);

        registry.unregister(conn1).await;
        assert_eq!(registry.connection_count().await, 1);

        registry.unregister(conn2).await;
        assert_eq!(registry.connection_count().await, 0);
    }
}
