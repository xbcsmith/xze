//! WebSocket handler for real-time search
//!
//! This module implements the WebSocket connection handler for real-time search
//! functionality, including connection management, heartbeat mechanism, and
//! message routing.

use super::types::{ClientMessage, ServerMessage};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// WebSocket connection timeout (30 seconds)
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// WebSocket connection handler
///
/// Manages a single WebSocket connection, including heartbeat mechanism,
/// message routing, and connection lifecycle.
///
/// # Examples
///
/// ```rust,no_run
/// use axum::extract::ws::WebSocket;
/// use xze_serve::search::websocket::handler::WebSocketHandler;
/// use xze_serve::search::websocket::connection::ConnectionRegistry;
/// use std::sync::Arc;
///
/// # async fn example(socket: WebSocket) {
/// let registry = Arc::new(ConnectionRegistry::new());
/// let handler = WebSocketHandler::new(socket, registry);
/// handler.run().await;
/// # }
/// ```
pub struct WebSocketHandler {
    /// The WebSocket connection
    socket: WebSocket,
    /// Connection registry for managing subscriptions
    registry: Arc<super::connection::ConnectionRegistry>,
    /// Unique connection ID
    connection_id: Uuid,
    /// Last time we received a message from the client
    last_heartbeat: Instant,
}

impl WebSocketHandler {
    /// Create a new WebSocket handler
    ///
    /// # Arguments
    ///
    /// * `socket` - The WebSocket connection
    /// * `registry` - Connection registry for managing subscriptions
    ///
    /// # Returns
    ///
    /// Returns a new WebSocket handler instance
    pub fn new(socket: WebSocket, registry: Arc<super::connection::ConnectionRegistry>) -> Self {
        let connection_id = Uuid::new_v4();
        info!("New WebSocket connection: {}", connection_id);

        Self {
            socket,
            registry,
            connection_id,
            last_heartbeat: Instant::now(),
        }
    }

    /// Run the WebSocket handler
    ///
    /// This is the main event loop that processes incoming messages,
    /// sends outgoing messages, and maintains the heartbeat.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use axum::extract::ws::WebSocket;
    /// use xze_serve::search::websocket::handler::WebSocketHandler;
    /// use xze_serve::search::websocket::connection::ConnectionRegistry;
    /// use std::sync::Arc;
    ///
    /// # async fn example(socket: WebSocket) {
    /// let registry = Arc::new(ConnectionRegistry::new());
    /// let handler = WebSocketHandler::new(socket, registry);
    /// handler.run().await;
    /// # }
    /// ```
    pub async fn run(self) {
        // Extract fields before splitting socket
        let connection_id = self.connection_id;
        let registry = self.registry;
        let mut last_heartbeat = self.last_heartbeat;

        // Create channels for sending/receiving messages
        let (tx, mut rx) = mpsc::channel::<ServerMessage>(100);

        // Register connection
        registry.register(connection_id, tx.clone()).await;

        // Split socket into sender and receiver
        let (mut sender, mut receiver) = self.socket.split();

        // Create heartbeat timer
        let mut heartbeat_interval = interval(HEARTBEAT_INTERVAL);

        loop {
            tokio::select! {
                // Handle incoming messages from client
                msg = receiver.next() => {
                    match msg {
                        Some(Ok(msg)) => {
                            if !Self::handle_message(msg, &tx, &mut last_heartbeat, &registry, connection_id).await {
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            info!("WebSocket connection closed by client");
                            break;
                        }
                    }
                }

                // Handle outgoing messages to client
                msg = rx.recv() => {
                    match msg {
                        Some(msg) => {
                            if let Err(e) = Self::send_message(&mut sender, msg).await {
                                error!("Failed to send message: {}", e);
                                break;
                            }
                        }
                        None => {
                            info!("Message channel closed");
                            break;
                        }
                    }
                }

                // Handle heartbeat
                _ = heartbeat_interval.tick() => {
                    if last_heartbeat.elapsed() > CLIENT_TIMEOUT {
                        warn!("Client timeout, closing connection");
                        break;
                    }

                    // Send ping
                    if let Err(e) = Self::send_message(&mut sender, ServerMessage::Pong).await {
                        error!("Failed to send ping: {}", e);
                        break;
                    }
                }
            }
        }

        // Cleanup on disconnect
        registry.unregister(connection_id).await;
        info!("WebSocket connection closed: {}", connection_id);
    }

    /// Handle incoming client message
    ///
    /// # Arguments
    ///
    /// * `msg` - The WebSocket message
    /// * `tx` - Channel for sending responses
    /// * `last_heartbeat` - Last heartbeat timestamp
    /// * `registry` - Connection registry
    /// * `connection_id` - Connection ID
    ///
    /// # Returns
    ///
    /// Returns `true` if the connection should continue, `false` to close
    async fn handle_message(
        msg: Message,
        tx: &mpsc::Sender<ServerMessage>,
        last_heartbeat: &mut Instant,
        registry: &Arc<super::connection::ConnectionRegistry>,
        connection_id: Uuid,
    ) -> bool {
        *last_heartbeat = Instant::now();

        match msg {
            Message::Text(text) => {
                debug!("Received text message: {}", text);
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        Self::handle_client_command(client_msg, tx, registry, connection_id).await;
                    }
                    Err(e) => {
                        error!("Failed to parse message: {}", e);
                        let _ = tx
                            .send(ServerMessage::Error {
                                message: format!("Invalid message format: {}", e),
                            })
                            .await;
                    }
                }
            }
            Message::Binary(data) => {
                debug!("Received binary message: {} bytes", data.len());
                match serde_json::from_slice::<ClientMessage>(&data) {
                    Ok(client_msg) => {
                        Self::handle_client_command(client_msg, tx, registry, connection_id).await;
                    }
                    Err(e) => {
                        error!("Failed to parse binary message: {}", e);
                        let _ = tx
                            .send(ServerMessage::Error {
                                message: format!("Invalid message format: {}", e),
                            })
                            .await;
                    }
                }
            }
            Message::Ping(_) => {
                debug!("Received ping");
                // Axum handles pong automatically
            }
            Message::Pong(_) => {
                debug!("Received pong");
            }
            Message::Close(_) => {
                info!("Received close message");
                return false;
            }
        }

        true
    }

    /// Handle client command
    ///
    /// # Arguments
    ///
    /// * `msg` - The client message
    /// * `tx` - Channel for sending responses
    /// * `registry` - Connection registry
    /// * `connection_id` - Connection ID
    async fn handle_client_command(
        msg: ClientMessage,
        tx: &mpsc::Sender<ServerMessage>,
        registry: &Arc<super::connection::ConnectionRegistry>,
        connection_id: Uuid,
    ) {
        match msg {
            ClientMessage::Ping => {
                debug!("Ping from client");
                let _ = tx.send(ServerMessage::Pong).await;
            }
            ClientMessage::StreamingSearch { request_id, query } => {
                debug!("Streaming search request: {}", request_id);
                Self::handle_streaming_search(request_id, query, tx).await;
            }
            ClientMessage::Subscribe {
                subscription_id,
                filters,
            } => {
                debug!("Subscribe request: {}", subscription_id);
                Self::handle_subscribe(subscription_id, filters, tx, registry, connection_id).await;
            }
            ClientMessage::Unsubscribe { subscription_id } => {
                debug!("Unsubscribe request: {}", subscription_id);
                Self::handle_unsubscribe(subscription_id, tx, registry, connection_id).await;
            }
            ClientMessage::CancelSearch { request_id } => {
                debug!("Cancel search request: {}", request_id);
                // TODO: Implement search cancellation
                let _ = tx
                    .send(ServerMessage::SearchComplete {
                        request_id,
                        total_results: 0,
                    })
                    .await;
            }
        }
    }

    /// Handle streaming search request
    async fn handle_streaming_search(
        request_id: Uuid,
        _query: Box<crate::search::AdvancedSearchRequest>,
        tx: &mpsc::Sender<ServerMessage>,
    ) {
        // TODO: Implement actual search logic
        // For now, send empty result
        let _ = tx
            .send(ServerMessage::SearchComplete {
                request_id,
                total_results: 0,
            })
            .await;
    }

    /// Handle subscribe request
    async fn handle_subscribe(
        subscription_id: Uuid,
        filters: super::types::SubscriptionFilters,
        tx: &mpsc::Sender<ServerMessage>,
        registry: &Arc<super::connection::ConnectionRegistry>,
        connection_id: Uuid,
    ) {
        registry
            .add_subscription(connection_id, subscription_id, filters)
            .await;

        let _ = tx.send(ServerMessage::Subscribed { subscription_id }).await;
    }

    /// Handle unsubscribe request
    async fn handle_unsubscribe(
        subscription_id: Uuid,
        tx: &mpsc::Sender<ServerMessage>,
        registry: &Arc<super::connection::ConnectionRegistry>,
        connection_id: Uuid,
    ) {
        registry
            .remove_subscription(connection_id, subscription_id)
            .await;

        let _ = tx
            .send(ServerMessage::Unsubscribed { subscription_id })
            .await;
    }

    /// Send a message to the client
    async fn send_message(
        sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
        msg: ServerMessage,
    ) -> Result<(), axum::Error> {
        let json = serde_json::to_string(&msg).map_err(|e| {
            axum::Error::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize message: {}", e),
            ))
        })?;

        sender.send(Message::Text(json)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_interval() {
        assert_eq!(HEARTBEAT_INTERVAL, Duration::from_secs(10));
    }

    #[test]
    fn test_client_timeout() {
        assert_eq!(CLIENT_TIMEOUT, Duration::from_secs(30));
        assert!(CLIENT_TIMEOUT > HEARTBEAT_INTERVAL);
    }
}
