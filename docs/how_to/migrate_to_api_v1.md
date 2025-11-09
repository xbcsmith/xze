# How to Migrate to API v1

## Overview

This guide explains how to migrate from legacy XZe API endpoints to the new v1 API. The v1 API provides a stable, versioned interface that will be maintained going forward, while legacy endpoints will be removed on March 1, 2025.

## Why Migrate?

- Legacy endpoints will be removed on March 1, 2025
- v1 API provides versioning guarantees and stability
- v1 API will receive new features and improvements
- Better documentation and OpenAPI specification
- Industry-standard deprecation handling

## Migration Checklist

- [ ] Update all endpoint URLs to use `/api/v1` prefix
- [ ] Test all API calls in development environment
- [ ] Update any hardcoded URLs in configuration
- [ ] Update client libraries or SDKs
- [ ] Monitor for deprecation headers in production
- [ ] Deploy updated application before March 1, 2025

## Endpoint Mapping

### Health Check

**Legacy (Deprecated):**
```http
GET /health
```

**V1 (Current):**
```http
GET /api/v1/health
```

**Response:** Identical structure in both versions

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### Version Information

**Legacy (Deprecated):**
```http
GET /version
```

**V1 (Current):**
```http
GET /api/v1/version
```

**Response:**
```json
{
  "version": "0.1.0",
  "api_version": "v1",
  "build_info": {
    "commit": "abc123...",
    "build_date": "2024-01-15T08:00:00Z"
  }
}
```

### Repository Analysis

**Legacy (Deprecated):**
```http
POST /analyze
Content-Type: application/json

{
  "repository_url": "https://github.com/user/repo",
  "branch": "main",
  "language": "rust"
}
```

**V1 (Current):**
```http
POST /api/v1/analyze
Content-Type: application/json

{
  "repository_url": "https://github.com/user/repo",
  "branch": "main",
  "language": "rust"
}
```

**Response:**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "queued",
  "message": "Analysis queued for repository: https://github.com/user/repo"
}
```

### List Repositories

**Legacy (Deprecated):**
```http
GET /repositories?page=1&per_page=10
```

**V1 (Current):**
```http
GET /api/v1/repositories?page=1&per_page=10
```

**Response:**
```json
{
  "repositories": [],
  "total": 0,
  "page": 1,
  "per_page": 10
}
```

### Get Repository

**Legacy (Deprecated):**
```http
GET /repositories/:id
```

**V1 (Current):**
```http
GET /api/v1/repositories/:id
```

### Analyze Repository by ID

**Legacy (Deprecated):**
```http
POST /repositories/:id/analyze
```

**V1 (Current):**
```http
POST /api/v1/repositories/:id/analyze
```

### List Documentation

**Legacy (Deprecated):**
```http
GET /documentation
```

**V1 (Current):**
```http
GET /api/v1/documentation
```

**Response:**
```json
{
  "documents": [],
  "total": 0
}
```

### Get Documentation

**Legacy (Deprecated):**
```http
GET /documentation/:id
```

**V1 (Current):**
```http
GET /api/v1/documentation/:id
```

### Search Documentation

**Legacy (Deprecated):**
```http
GET /search?q=getting+started&max_results=10
```

**V1 (Current):**
```http
GET /api/v1/search?q=getting+started&max_results=10
```

## Code Examples

### Python (requests)

**Before (Legacy):**
```python
import requests

# Legacy endpoint
response = requests.get("http://api.example.com/health")
print(response.json())
```

**After (V1):**
```python
import requests

# V1 endpoint
response = requests.get("http://api.example.com/api/v1/health")

# Optional: Check API version
api_version = response.headers.get("API-Version")
print(f"Using API version: {api_version}")

print(response.json())
```

### JavaScript (fetch)

**Before (Legacy):**
```javascript
// Legacy endpoint
fetch('http://api.example.com/analyze', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    repository_url: 'https://github.com/user/repo',
    branch: 'main'
  })
})
.then(response => response.json())
.then(data => console.log(data));
```

**After (V1):**
```javascript
// V1 endpoint
fetch('http://api.example.com/api/v1/analyze', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Accept-Version': 'v1'  // Optional but recommended
  },
  body: JSON.stringify({
    repository_url: 'https://github.com/user/repo',
    branch: 'main'
  })
})
.then(response => {
  // Check API version from response header
  const apiVersion = response.headers.get('API-Version');
  console.log('API Version:', apiVersion);
  return response.json();
})
.then(data => console.log(data));
```

### Rust (reqwest)

**Before (Legacy):**
```rust
use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    // Legacy endpoint
    let response = client
        .post("http://api.example.com/analyze")
        .json(&json!({
            "repository_url": "https://github.com/user/repo",
            "branch": "main"
        }))
        .send()
        .await?;

    let data = response.json::<serde_json::Value>().await?;
    println!("{}", data);

    Ok(())
}
```

**After (V1):**
```rust
use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    // V1 endpoint
    let response = client
        .post("http://api.example.com/api/v1/analyze")
        .header("Accept-Version", "v1")  // Optional
        .json(&json!({
            "repository_url": "https://github.com/user/repo",
            "branch": "main"
        }))
        .send()
        .await?;

    // Check API version from response header
    if let Some(api_version) = response.headers().get("api-version") {
        println!("API Version: {:?}", api_version);
    }

    let data = response.json::<serde_json::Value>().await?;
    println!("{}", data);

    Ok(())
}
```

### cURL

**Before (Legacy):**
```bash
# Health check
curl http://api.example.com/health

# Analyze repository
curl -X POST http://api.example.com/analyze \
  -H "Content-Type: application/json" \
  -d '{"repository_url":"https://github.com/user/repo","branch":"main"}'
```

**After (V1):**
```bash
# Health check with version header
curl -v http://api.example.com/api/v1/health \
  -H "Accept-Version: v1"

# Analyze repository
curl -X POST http://api.example.com/api/v1/analyze \
  -H "Content-Type: application/json" \
  -H "Accept-Version: v1" \
  -d '{"repository_url":"https://github.com/user/repo","branch":"main"}'
```

## Configuration Updates

### Environment Variables

If you use environment variables for API base URLs:

**Before:**
```bash
API_BASE_URL=http://api.example.com
```

**After:**
```bash
API_BASE_URL=http://api.example.com/api/v1
```

### Application Configuration Files

**config.yaml (Before):**
```yaml
api:
  base_url: http://api.example.com
  endpoints:
    health: /health
    analyze: /analyze
```

**config.yaml (After):**
```yaml
api:
  base_url: http://api.example.com/api/v1
  endpoints:
    health: /health
    analyze: /analyze
```

Or keep base URL the same and update endpoints:

```yaml
api:
  base_url: http://api.example.com
  endpoints:
    health: /api/v1/health
    analyze: /api/v1/analyze
```

## Version Negotiation (Optional)

The v1 API supports version negotiation via the `Accept-Version` header:

```http
GET /api/v1/health
Accept-Version: v1
```

Supported version formats:
- `v1`
- `1`
- `1.0`

The server responds with the active API version:

```http
HTTP/1.1 200 OK
API-Version: v1
Content-Type: application/json
```

If you request an unsupported version:

```http
GET /api/v1/health
Accept-Version: v99
```

You receive an error:

```http
HTTP/1.1 400 Bad Request
Content-Type: application/json

{
  "error": "Invalid API version",
  "message": "Requested version 'v99' is not supported. Supported versions: v1",
  "supported_versions": ["v1"]
}
```

## Detecting Deprecation Warnings

Legacy endpoints include deprecation headers. Monitor these in production:

**Python Example:**
```python
import requests

response = requests.get("http://api.example.com/health")

# Check for deprecation
if 'deprecation' in response.headers:
    print("WARNING: This endpoint is deprecated")
    print(f"Sunset date: {response.headers.get('sunset')}")
    print(f"Migrate to: {response.headers.get('link')}")
    print(f"Details: {response.headers.get('warning')}")
```

**JavaScript Example:**
```javascript
fetch('http://api.example.com/health')
  .then(response => {
    if (response.headers.get('deprecation')) {
      console.warn('WARNING: This endpoint is deprecated');
      console.warn('Sunset:', response.headers.get('sunset'));
      console.warn('Link:', response.headers.get('link'));
    }
    return response.json();
  });
```

## Testing Your Migration

### 1. Verify Endpoints Work

Test each updated endpoint in your development environment:

```bash
# Test health
curl http://localhost:3000/api/v1/health

# Test version
curl http://localhost:3000/api/v1/version

# Test with invalid version
curl -H "Accept-Version: v99" http://localhost:3000/api/v1/health
```

### 2. Compare Responses

Verify that v1 responses match legacy responses:

```bash
# Legacy response
curl http://localhost:3000/health > legacy.json

# V1 response
curl http://localhost:3000/api/v1/health > v1.json

# Compare (should be identical except for headers)
diff <(jq -S . legacy.json) <(jq -S . v1.json)
```

### 3. Check Headers

Verify API-Version header is present:

```bash
curl -I http://localhost:3000/api/v1/health | grep -i api-version
```

### 4. Load Testing

Run load tests against v1 endpoints to ensure performance:

```bash
# Using Apache Bench
ab -n 1000 -c 10 http://localhost:3000/api/v1/health

# Using wrk
wrk -t4 -c100 -d30s http://localhost:3000/api/v1/health
```

## Rollback Plan

If you encounter issues after migration:

1. Revert to legacy endpoints temporarily
2. Legacy endpoints remain functional until March 1, 2025
3. Report issues to the XZe team
4. Re-test and redeploy when ready

## Common Migration Issues

### Issue: 404 Not Found

**Symptom:** Receiving 404 errors on v1 endpoints

**Solution:** Ensure `/api/v1` prefix is included in all URLs

```bash
# Wrong
curl http://api.example.com/health

# Correct
curl http://api.example.com/api/v1/health
```

### Issue: Invalid API Version Error

**Symptom:** Receiving 400 Bad Request with "Invalid API version" message

**Solution:** Remove or update the `Accept-Version` header

```bash
# Wrong
curl -H "Accept-Version: v2" http://api.example.com/api/v1/health

# Correct
curl -H "Accept-Version: v1" http://api.example.com/api/v1/health

# Or omit the header entirely
curl http://api.example.com/api/v1/health
```

### Issue: Deprecation Warnings in Logs

**Symptom:** Seeing deprecation warnings in application logs

**Solution:** Complete migration to v1 endpoints to eliminate warnings

## Timeline and Deadlines

- **Now - March 1, 2025**: Migration period
  - Both legacy and v1 endpoints are functional
  - Legacy endpoints return deprecation headers
  - Complete your migration during this period

- **March 1, 2025**: Sunset date
  - Legacy endpoints will be removed
  - Only v1 endpoints will be available
  - Applications not migrated will break

## Support and Resources

- API Documentation: `/api/v1/docs` (coming in Phase 7.5.2)
- Implementation Details: `docs/explanation/phase7_5_api_versioning_implementation.md`
- GitHub Issues: Report problems or ask questions
- Migration Guide: This document

## FAQ

### Do I need to change my authentication?

No, authentication mechanisms remain unchanged.

### Will my API keys still work?

Yes, existing API keys work with both legacy and v1 endpoints.

### Are there any breaking changes in v1?

No, v1 maintains the same response structures as legacy endpoints. The only change is the URL prefix.

### Can I use both legacy and v1 endpoints during migration?

Yes, you can migrate endpoints incrementally. Both work during the migration period.

### What if I miss the deadline?

Applications using legacy endpoints will break after March 1, 2025. We strongly recommend migrating before this date.

### Will there be a v2 API?

When v2 is needed, it will be introduced as `/api/v2` while v1 remains available, following the same migration pattern.

## Quick Reference

```text
Legacy Endpoint              -> V1 Endpoint
---------------------------------------------------------
GET  /health                 -> GET  /api/v1/health
GET  /version                -> GET  /api/v1/version
POST /analyze                -> POST /api/v1/analyze
GET  /repositories           -> GET  /api/v1/repositories
GET  /repositories/:id       -> GET  /api/v1/repositories/:id
POST /repositories/:id/analyze -> POST /api/v1/repositories/:id/analyze
GET  /documentation          -> GET  /api/v1/documentation
GET  /documentation/:id      -> GET  /api/v1/documentation/:id
GET  /search                 -> GET  /api/v1/search
```

## Conclusion

Migrating to API v1 is straightforward:

1. Add `/api/v1` prefix to all endpoint URLs
2. Test in development
3. Deploy before March 1, 2025

The v1 API provides a stable foundation for future development while maintaining backward compatibility during the migration period. If you have questions or encounter issues, please refer to the support resources listed above.
