# WebSocket Real-Time Search

This module provides WebSocket-based real-time search functionality for XZe,
enabling streaming search results and live document update notifications.

## Features

- **Streaming Search**: Progressive delivery of search results as they are found
- **Live Updates**: Real-time notifications when documents change
- **Subscriptions**: Filter-based subscriptions to document changes
- **Connection Management**: Automatic connection lifecycle and heartbeat
- **Type-Safe Protocol**: Strongly-typed message protocol with serde

## Architecture

The WebSocket module is organized into:

- `types.rs` - Message types and data structures
- `handler.rs` - WebSocket connection handler with heartbeat
- `connection.rs` - Connection registry and subscription management
- `streaming.rs` - Streaming search execution with batching
- `mod.rs` - Module entry point and route configuration

## WebSocket Endpoint

```
WS /api/ws/search
```

## Message Protocol

### Client-to-Server Messages

#### Streaming Search

Start a progressive search that delivers results in batches:

```json
{
  "type": "streaming_search",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "query": {
    "query": "rust async programming",
    "filters": {
      "categories": ["tutorial", "reference"],
      "similarity": { "min": 0.7 }
    },
    "options": {
      "max_results": 50,
      "include_snippets": true
    }
  }
}
```

#### Subscribe to Document Updates

Subscribe to receive notifications when documents matching filters change:

```json
{
  "type": "subscribe",
  "subscription_id": "550e8400-e29b-41d4-a716-446655440001",
  "filters": {
    "categories": ["tutorial"],
    "repositories": ["xze"],
    "tags": ["rust", "async"]
  }
}
```

#### Unsubscribe

Cancel a subscription:

```json
{
  "type": "unsubscribe",
  "subscription_id": "550e8400-e29b-41d4-a716-446655440001"
}
```

#### Cancel Search

Cancel an in-progress streaming search:

```json
{
  "type": "cancel_search",
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### Ping

Keep-alive message:

```json
{
  "type": "ping"
}
```

### Server-to-Client Messages

#### Search Batch

A batch of search results:

```json
{
  "type": "search_batch",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "results": [
    {
      "id": "doc-123",
      "title": "Async Programming in Rust",
      "snippet": "Learn about async/await...",
      "category": "tutorial",
      "similarity": 0.95
    }
  ],
  "has_more": true,
  "total": 42
}
```

#### Search Complete

Search has finished:

```json
{
  "type": "search_complete",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "total_results": 42
}
```

#### Search Error

Search failed:

```json
{
  "type": "search_error",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "error": "Invalid query syntax"
}
```

#### Document Update

A document matching subscriptions changed:

```json
{
  "type": "document_update",
  "subscription_ids": ["550e8400-e29b-41d4-a716-446655440001"],
  "event": {
    "event_type": "updated",
    "document_id": "doc-456",
    "changes": {
      "title": "Updated Title",
      "content": "New content...",
      "category": "reference"
    }
  }
}
```

#### Subscribed

Subscription confirmed:

```json
{
  "type": "subscribed",
  "subscription_id": "550e8400-e29b-41d4-a716-446655440001"
}
```

#### Unsubscribed

Unsubscribe confirmed:

```json
{
  "type": "unsubscribed",
  "subscription_id": "550e8400-e29b-41d4-a716-446655440001"
}
```

#### Pong

Response to ping:

```json
{
  "type": "pong"
}
```

#### Error

General error message:

```json
{
  "type": "error",
  "message": "Invalid message format"
}
```

## Usage Examples

### JavaScript Client

```javascript
const ws = new WebSocket('ws://localhost:3000/api/ws/search');

ws.onopen = () => {
    console.log('Connected');

    // Start streaming search
    ws.send(JSON.stringify({
        type: 'streaming_search',
        request_id: crypto.randomUUID(),
        query: {
            query: 'rust async programming',
            filters: {
                categories: ['tutorial']
            }
        }
    }));

    // Subscribe to updates
    ws.send(JSON.stringify({
        type: 'subscribe',
        subscription_id: crypto.randomUUID(),
        filters: {
            categories: ['tutorial', 'reference']
        }
    }));
};

ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);

    switch (msg.type) {
        case 'search_batch':
            console.log('Received batch:', msg.results.length, 'results');
            displayResults(msg.results);
            if (!msg.has_more) {
                console.log('Search complete, total:', msg.total);
            }
            break;

        case 'search_complete':
            console.log('Search finished:', msg.total_results, 'results');
            break;

        case 'document_update':
            console.log('Document updated:', msg.event);
            updateDocumentInUI(msg.event);
            break;

        case 'subscribed':
            console.log('Subscription active:', msg.subscription_id);
            break;

        case 'error':
            console.error('Error:', msg.message);
            break;
    }
};

ws.onerror = (error) => {
    console.error('WebSocket error:', error);
};

ws.onclose = () => {
    console.log('Disconnected');
};
```

### Rust Server Integration

```rust
use axum::Router;
use xze_serve::search::websocket::{websocket_routes, ConnectionRegistry};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Create connection registry
    let registry = Arc::new(ConnectionRegistry::new());

    // Create router with WebSocket routes
    let app = Router::new()
        .nest("/api", websocket_routes(registry.clone()));

    // Run server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
```

### Broadcasting Updates

```rust
use xze_serve::search::websocket::types::{DocumentUpdateEvent, ConnectionRegistry};
use std::sync::Arc;
use uuid::Uuid;

async fn broadcast_document_created(
    registry: Arc<ConnectionRegistry>,
    doc_id: Uuid,
    title: String,
    category: String,
) {
    let event = DocumentUpdateEvent::Created {
        document_id: doc_id,
        title,
        category,
        repository: Some("xze".to_string()),
    };

    registry.broadcast_update(event).await;
}
```

## Configuration

### Streaming Configuration

Control batch size and timing:

```rust
use xze_serve::search::websocket::streaming::StreamingConfig;

let config = StreamingConfig::new(
    20,    // batch_size: send 20 results per batch
    1000,  // batch_timeout_ms: max 1s wait for partial batch
);
```

Defaults:
- Batch size: 10 results
- Batch timeout: 500ms
- Max batch size: 100 results (enforced)

### Connection Configuration

Heartbeat and timeout settings are constants in `handler.rs`:

```rust
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);
```

## Performance Characteristics

### Memory Usage

- Per-connection overhead: ~100 bytes
- Message channel capacity: 100 items per connection
- Subscriptions stored as HashMap entries
- No unbounded buffers

### Network Traffic

- Default batch size (10) balances latency and bandwidth
- Heartbeat every 10 seconds (minimal overhead)
- WebSocket compression can be enabled
- Binary protocol option available (currently JSON)

### Concurrency

- `RwLock` for read-heavy connection registry
- Non-blocking message sends with `try_send`
- `tokio::select!` for efficient I/O multiplexing
- No blocking operations in message handlers

## Security Considerations

### Current Implementation

- 30-second timeout for inactive clients
- Automatic cleanup on disconnect
- Validated message parsing with error handling
- No unbounded resource allocation

### Production Requirements

Before deploying to production, implement:

1. **Authentication**: Token-based connection authentication
2. **Authorization**: Filter results based on user permissions
3. **Rate Limiting**: Per-connection message rate limits
4. **Encryption**: Use WSS (WebSocket Secure) only
5. **Monitoring**: Connection metrics and alerting

## Testing

The module includes comprehensive unit tests:

```bash
# Run all tests
cargo test -p xze-serve --lib

# Run only WebSocket tests
cargo test -p xze-serve --lib websocket
```

Test coverage:
- Message serialization/deserialization (9 tests)
- Connection registry operations (5 tests)
- Streaming handler logic (5 tests)
- Handler lifecycle (2 tests)
- Module integration (3 tests)

Total: 24 tests

## Monitoring

### Connection Metrics

Get current connection statistics:

```rust
let registry = Arc::new(ConnectionRegistry::new());

// Number of active connections
let conn_count = registry.connection_count().await;

// Total subscriptions across all connections
let sub_count = registry.subscription_count().await;
```

### Logging

The module uses `tracing` for structured logging:

```rust
use tracing::Level;
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .init();
```

Log events include:
- Connection lifecycle (open, close, timeout)
- Message routing and handling
- Subscription management
- Broadcast operations
- Error conditions

## Future Enhancements

### Planned Features

1. **Search Execution Integration**
   - Connect to xze-core search engine
   - Implement actual progressive result delivery
   - Support search cancellation

2. **Document Change Detection**
   - File system watcher integration
   - Git repository change detection
   - Automatic update broadcasting

3. **Advanced Subscriptions**
   - Wildcard pattern matching
   - Complex filter expressions
   - Multiple subscriptions per connection

4. **Performance Optimizations**
   - Binary message protocol option
   - Result compression
   - Connection pooling
   - Distributed registry for horizontal scaling

5. **Production Features**
   - JWT authentication middleware
   - Fine-grained authorization
   - Rate limiting per user/connection
   - Metrics and distributed tracing
   - Health checks and monitoring endpoints

## Troubleshooting

### Connection Issues

**Problem**: Connection closes immediately

**Solution**: Check that WebSocket upgrade is successful. Ensure client sends
proper WebSocket upgrade headers.

### Message Parsing Errors

**Problem**: Receiving error messages about invalid format

**Solution**: Verify JSON message structure matches the schema. All messages
must have a `type` field.

### No Updates Received

**Problem**: Subscribed but not receiving document updates

**Solution**: Verify filters match the documents. Check that
`broadcast_update()` is being called when documents change.

### Timeout Issues

**Problem**: Connections timing out frequently

**Solution**: Ensure client responds to pings or sends periodic messages.
Adjust timeout constants if needed for your use case.

## References

- Implementation: `docs/explanations/phase_11_real_time_search_implementation.md`
- Search Plan: `docs/explanations/search_features_unified_implementation_plan.md`
- API Documentation: Run `cargo doc --open`
