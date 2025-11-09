# API v1 Reference Specification

## Overview

This document provides the complete reference specification for XZe API v1. All v1 endpoints are accessible under the `/api/v1` path prefix.

## Base URL

```
http://localhost:3000/api/v1
```

Replace `localhost:3000` with your actual server address and port.

## Authentication

Authentication mechanisms will be documented in Phase 8. Currently, API endpoints are accessible without authentication in development mode.

## Common Headers

### Request Headers

- `Content-Type: application/json` - Required for POST requests
- `Accept-Version: v1` - Optional, specifies requested API version

### Response Headers

All responses include:

- `API-Version: v1` - Active API version
- `Content-Type: application/json` - Response format

## Endpoints

### Health Check

Check the service health status.

**Endpoint:** `GET /api/v1/health`

**Parameters:** None

**Response:** `200 OK`

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

**Status Field Values:**
- `healthy` - Service is operational
- `degraded` - Service is operational but experiencing issues
- `unhealthy` - Service is not operational

**Example:**

```bash
curl http://localhost:3000/api/v1/health
```

---

### Version Information

Get version and build information.

**Endpoint:** `GET /api/v1/version`

**Parameters:** None

**Response:** `200 OK`

```json
{
  "version": "0.1.0",
  "api_version": "v1",
  "build_info": {
    "commit": "abc123def456...",
    "build_date": "2024-01-15T08:00:00Z"
  }
}
```

**Example:**

```bash
curl http://localhost:3000/api/v1/version
```

---

### Analyze Repository

Queue a repository for AI-powered documentation analysis.

**Endpoint:** `POST /api/v1/analyze`

**Request Body:**

```json
{
  "repository_url": "https://github.com/user/repo",
  "branch": "main",
  "language": "rust"
}
```

**Parameters:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `repository_url` | string | Yes | Git repository URL (https or git protocol) |
| `branch` | string | No | Branch to analyze (defaults to main/master) |
| `language` | string | No | Primary language (auto-detected if omitted) |

**Response:** `200 OK`

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "queued",
  "message": "Analysis queued for repository: https://github.com/user/repo"
}
```

**Status Values:**
- `queued` - Job is queued for processing
- `processing` - Job is currently being processed
- `completed` - Job completed successfully
- `failed` - Job failed with errors

**Example:**

```bash
curl -X POST http://localhost:3000/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "repository_url": "https://github.com/rust-lang/rust",
    "branch": "master",
    "language": "rust"
  }'
```

---

### List Repositories

List all analyzed repositories with pagination.

**Endpoint:** `GET /api/v1/repositories`

**Query Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `page` | integer | No | 1 | Page number (1-indexed) |
| `per_page` | integer | No | 10 | Items per page (max 100) |

**Response:** `200 OK`

```json
{
  "repositories": [
    {
      "id": "repo-123",
      "name": "rust",
      "url": "https://github.com/rust-lang/rust",
      "language": "rust",
      "last_analyzed": "2024-01-15T10:00:00Z"
    }
  ],
  "total": 42,
  "page": 1,
  "per_page": 10
}
```

**Example:**

```bash
curl http://localhost:3000/api/v1/repositories?page=1&per_page=20
```

---

### Get Repository

Get detailed information about a specific repository.

**Endpoint:** `GET /api/v1/repositories/:id`

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Repository ID |

**Response:** `200 OK` or `404 Not Found`

```json
{
  "id": "repo-123",
  "name": "rust",
  "url": "https://github.com/rust-lang/rust",
  "language": "rust",
  "last_analyzed": "2024-01-15T10:00:00Z"
}
```

**Example:**

```bash
curl http://localhost:3000/api/v1/repositories/repo-123
```

---

### Analyze Repository by ID

Re-analyze an existing repository.

**Endpoint:** `POST /api/v1/repositories/:id/analyze`

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Repository ID |

**Request Body:** None

**Response:** `200 OK` or `404 Not Found`

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "queued",
  "message": "Analysis queued for repository ID: repo-123"
}
```

**Example:**

```bash
curl -X POST http://localhost:3000/api/v1/repositories/repo-123/analyze
```

---

### List Documentation

List all generated documentation.

**Endpoint:** `GET /api/v1/documentation`

**Query Parameters:** None (pagination to be added in future version)

**Response:** `200 OK`

```json
{
  "documents": [
    {
      "id": "doc-456",
      "title": "Getting Started with Rust",
      "category": "tutorial",
      "repository_id": "repo-123",
      "created_at": "2024-01-15T10:00:00Z",
      "updated_at": "2024-01-15T11:00:00Z"
    }
  ],
  "total": 15
}
```

**Category Values:**
- `tutorial` - Step-by-step learning guides
- `how-to` - Task-oriented problem-solving guides
- `reference` - Technical reference material
- `explanation` - Conceptual explanations

**Example:**

```bash
curl http://localhost:3000/api/v1/documentation
```

---

### Get Documentation

Get a specific documentation item.

**Endpoint:** `GET /api/v1/documentation/:id`

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Documentation ID |

**Response:** `200 OK` or `404 Not Found`

```json
{
  "id": "doc-456",
  "title": "Getting Started with Rust",
  "category": "tutorial",
  "repository_id": "repo-123",
  "created_at": "2024-01-15T10:00:00Z",
  "updated_at": "2024-01-15T11:00:00Z"
}
```

**Example:**

```bash
curl http://localhost:3000/api/v1/documentation/doc-456
```

---

### Search Documentation

Semantic search across all documentation.

**Endpoint:** `GET /api/v1/search`

**Query Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `q` | string | Yes | - | Search query |
| `max_results` | integer | No | 10 | Maximum results to return |
| `min_similarity` | float | No | 0.7 | Minimum similarity score (0.0-1.0) |
| `category` | string | No | - | Filter by category |

**Response:** `200 OK`

```json
{
  "query": "getting started",
  "results": [
    {
      "id": "chunk-789",
      "source_file": "README.md",
      "content": "Getting started with XZe is easy...",
      "similarity": 0.92,
      "chunk_index": 0,
      "total_chunks": 5,
      "title": "Introduction",
      "category": "tutorial",
      "sentence_range": [0, 10],
      "avg_chunk_similarity": 0.87
    }
  ],
  "total_results": 3,
  "config": {
    "max_results": 10,
    "min_similarity": 0.7,
    "category_filter": null
  }
}
```

**Example:**

```bash
curl "http://localhost:3000/api/v1/search?q=installation&max_results=5&min_similarity=0.8"
```

---

## Error Responses

All error responses follow a consistent format:

```json
{
  "error": "Error Type",
  "message": "Human-readable error description",
  "details": "Optional additional details"
}
```

### Common Status Codes

| Code | Meaning | Description |
|------|---------|-------------|
| 200 | OK | Request succeeded |
| 400 | Bad Request | Invalid request parameters or invalid API version |
| 404 | Not Found | Resource not found |
| 500 | Internal Server Error | Server encountered an error |

### Error Examples

**Invalid API Version:**

```http
GET /api/v1/health
Accept-Version: v99
```

Response: `400 Bad Request`

```json
{
  "error": "Invalid API version",
  "message": "Requested version 'v99' is not supported. Supported versions: v1",
  "supported_versions": ["v1"]
}
```

**Resource Not Found:**

```http
GET /api/v1/repositories/invalid-id
```

Response: `404 Not Found`

---

## Version Negotiation

Clients can request a specific API version using the `Accept-Version` header:

```http
GET /api/v1/health
Accept-Version: v1
```

Supported version formats:
- `v1`
- `1`
- `1.0`

The server responds with the active version in the `API-Version` header:

```http
HTTP/1.1 200 OK
API-Version: v1
Content-Type: application/json
```

If the `Accept-Version` header is omitted, the server uses the version specified in the URL path.

---

## Rate Limiting

Rate limiting will be implemented in Phase 8. Current development builds have no rate limits.

---

## Pagination

Endpoints that return lists support pagination:

- `page` - Page number (1-indexed)
- `per_page` - Items per page (default 10, max 100)

Response includes pagination metadata:

```json
{
  "items": [...],
  "total": 100,
  "page": 2,
  "per_page": 10
}
```

Calculate total pages: `Math.ceil(total / per_page)`

---

## Data Types

### Timestamp Format

All timestamps use ISO 8601 format in UTC:

```
2024-01-15T10:30:00Z
```

### UUID Format

IDs use UUID v4 format:

```
550e8400-e29b-41d4-a716-446655440000
```

### Similarity Score

Floating point number between 0.0 and 1.0:
- `1.0` - Perfect match
- `0.0` - No similarity

---

## OpenAPI Specification

Interactive API documentation will be available at `/api/v1/docs` in Phase 7.5.2 (OpenAPI implementation).

---

## Changelog

### v1 (Current)

- Initial API v1 release
- All endpoints under `/api/v1` prefix
- Version negotiation via `Accept-Version` header
- Deprecation of legacy root-level endpoints

---

## Migration from Legacy API

Legacy endpoints at root level (e.g., `/health`) are deprecated and will be removed on March 1, 2025.

See: `docs/how_to/migrate_to_api_v1.md` for migration guide.

---

## Support

- GitHub Issues: Report bugs or request features
- Documentation: `docs/` directory
- Implementation Details: `docs/explanation/phase7_5_api_versioning_implementation.md`
