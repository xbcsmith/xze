//! WebSocket support for real-time search
//!
//! This module provides WebSocket-based real-time search functionality,
//! including streaming search results, live document updates, and subscription
//! management.
//!
//! # Features
//!
//! - **Streaming Search**: Progressive delivery of search results as they are found
//! - **Live Updates**: Real-time notifications when documents change
//! - **Subscriptions**: Filter-based subscriptions to document changes
//! - **Connection Management**: Automatic connection lifecycle and heartbeat
//!
//! # Architecture
//!
//! The WebSocket module is organized into:
//! - `types`: Message types and data structures
//! - `handler`: WebSocket connection handler
//! - `connection`: Connection registry and subscription management
//! - `streaming`: Streaming search execution
//!
//! # Usage
//!
//! ```rust,no_run
//! use axum::Router;
//! use xze_serve::search::websocket::websocket_routes;
//! use xze_serve::search::websocket::connection::ConnectionRegistry;
//! use std::sync::Arc;
//!
//! # async fn example() {
//! let registry = Arc::new(ConnectionRegistry::new());
//! let app = Router::new().nest("/api", websocket_routes(registry));
//! # }
//! ```
//!
//! # WebSocket Protocol
//!
//! ## Client-to-Server Messages
//!
//! ### Streaming Search
//!
//! ```json
//! {
//!   "type": "streaming_search",
//!   "request_id": "550e8400-e29b-41d4-a716-446655440000",
//!   "query": {
//!     "query": "rust async programming",
//!     "filters": {
//!       "categories": ["tutorial"]
//!     }
//!   }
//! }
//! ```
//!
//! ### Subscribe to Updates
//!
//! ```json
//! {
//!   "type": "subscribe",
//!   "subscription_id": "550e8400-e29b-41d4-a716-446655440001",
//!   "filters": {
//!     "categories": ["tutorial", "reference"],
//!     "repositories": ["xze"]
//!   }
//! }
//! ```
//!
//! ### Unsubscribe
//!
//! ```json
//! {
//!   "type": "unsubscribe",
//!   "subscription_id": "550e8400-e29b-41d4-a716-446655440001"
//! }
//! ```
//!
//! ### Cancel Search
//!
//! ```json
//! {
//!   "type": "cancel_search",
//!   "request_id": "550e8400-e29b-41d4-a716-446655440000"
//! }
//! ```
//!
//! ### Ping
//!
//! ```json
//! {
//!   "type": "ping"
//! }
//! ```
//!
//! ## Server-to-Client Messages
//!
//! ### Search Batch
//!
//! ```json
//! {
//!   "type": "search_batch",
//!   "request_id": "550e8400-e29b-41d4-a716-446655440000",
//!   "results": [...],
//!   "has_more": true,
//!   "total": 100
//! }
//! ```
//!
//! ### Search Complete
//!
//! ```json
//! {
//!   "type": "search_complete",
//!   "request_id": "550e8400-e29b-41d4-a716-446655440000",
//!   "total_results": 42
//! }
//! ```
//!
//! ### Document Update
//!
//! ```json
//! {
//!   "type": "document_update",
//!   "subscription_ids": ["550e8400-e29b-41d4-a716-446655440001"],
//!   "event": {
//!     "event_type": "created",
//!     "document_id": "550e8400-e29b-41d4-a716-446655440002",
//!     "title": "New Document",
//!     "category": "tutorial",
//!     "repository": "xze"
//!   }
//! }
//! ```
//!
//! ### Subscribed
//!
//! ```json
//! {
//!   "type": "subscribed",
//!   "subscription_id": "550e8400-e29b-41d4-a716-446655440001"
//! }
//! ```
//!
//! ### Pong
//!
//! ```json
//! {
//!   "type": "pong"
//! }
//! ```

pub mod connection;
pub mod handler;
pub mod streaming;
pub mod types;

use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::Response,
    routing::get,
    Router,
};
use std::sync::Arc;

/// Create WebSocket routes
///
/// # Arguments
///
/// * `registry` - Connection registry for managing subscriptions
///
/// # Returns
///
/// Returns an Axum router with WebSocket endpoint
///
/// # Examples
///
/// ```rust,no_run
/// use axum::Router;
/// use xze_serve::search::websocket::{websocket_routes, connection::ConnectionRegistry};
/// use std::sync::Arc;
///
/// # async fn example() {
/// let registry = Arc::new(ConnectionRegistry::new());
/// let app = Router::new().nest("/api", websocket_routes(registry));
/// # }
/// ```
pub fn websocket_routes(registry: Arc<ConnectionRegistry>) -> Router {
    Router::new()
        .route("/ws/search", get(websocket_handler))
        .with_state(registry)
}

/// WebSocket upgrade handler
///
/// # Arguments
///
/// * `ws` - WebSocket upgrade request
/// * `registry` - Connection registry
///
/// # Returns
///
/// Returns a response that upgrades the connection to WebSocket
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(registry): State<Arc<ConnectionRegistry>>,
) -> Response {
    ws.on_upgrade(|socket| async move {
        let handler = WebSocketHandler::new(socket, registry);
        handler.run().await;
    })
}

// Re-export commonly used types
pub use connection::ConnectionRegistry;
pub use handler::WebSocketHandler;
pub use streaming::{StreamingConfig, StreamingSearchHandler};
pub use types::{
    ClientMessage, DocumentChanges, DocumentUpdateEvent, ServerMessage, SubscriptionFilters,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify that main types are exported
        let _config = StreamingConfig::default();
        let _registry = ConnectionRegistry::new();
    }

    #[test]
    fn test_websocket_routes_creation() {
        let registry = Arc::new(ConnectionRegistry::new());
        let _routes = websocket_routes(registry);
    }
}
