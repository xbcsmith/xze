# How to Use OpenAPI Documentation

## Overview

This guide explains how to use the XZe API v1 OpenAPI documentation for exploring, testing, and integrating with the API.

## Prerequisites

- XZe server running with OpenAPI feature enabled
- Web browser for Swagger UI
- curl or HTTP client for API testing (optional)

## Accessing the Documentation

### Interactive Swagger UI

The easiest way to explore the API is through the interactive Swagger UI:

1. Start the XZe server with OpenAPI enabled:

   ```bash
   cargo run --features openapi
   ```

2. Open your browser and navigate to:

   ```text
   http://localhost:3000/api/v1/docs
   ```

3. You should see the Swagger UI interface with all available endpoints

### Static Specification Files

Pre-generated OpenAPI specifications are available in the repository:

- JSON format: `docs/reference/openapi_v1.json`
- YAML format: `docs/reference/openapi_v1.yaml`

View them directly:

```bash
# View YAML specification
cat docs/reference/openapi_v1.yaml

# View JSON specification
cat docs/reference/openapi_v1.json | jq .
```

### Download Specification from Running Server

Download the spec from a running server:

```bash
# Download JSON specification
curl http://localhost:3000/api/v1/docs/openapi.json > openapi.json

# Download and format with jq
curl http://localhost:3000/api/v1/docs/openapi.json | jq . > openapi_pretty.json
```

## Using Swagger UI

### Exploring Endpoints

1. **Browse by Tag**: Endpoints are organized by category (health, system, analysis, repositories, documentation, search)

2. **Expand an Endpoint**: Click on any endpoint to see:
   - HTTP method and path
   - Description and summary
   - Parameters and request body schema
   - Response codes and schemas
   - Example responses

3. **View Schemas**: Click on "Schemas" at the bottom to see all data structures

### Testing Endpoints

#### Basic GET Request

1. Navigate to `GET /api/v1/health`
2. Click "Try it out"
3. Click "Execute"
4. View the response in the "Responses" section

#### POST Request with Body

1. Navigate to `POST /api/v1/analyze`
2. Click "Try it out"
3. Edit the request body in the JSON editor:

   ```json
   {
     "repository_url": "https://github.com/xbcsmith/xze",
     "branch": "main",
     "language": "rust"
   }
   ```

4. Click "Execute"
5. View the response with job ID

#### Query Parameters

1. Navigate to `GET /api/v1/repositories`
2. Click "Try it out"
3. Fill in query parameters:
   - page: 1
   - per_page: 10
4. Click "Execute"
5. View the paginated results

#### Search with Filters

1. Navigate to `GET /api/v1/search`
2. Click "Try it out"
3. Fill in query parameters:
   - q: "authentication guide"
   - max_results: 5
   - min_similarity: 0.7
   - category: "how-to"
4. Click "Execute"
5. View the semantic search results

### Understanding Responses

Each response includes:

- **Status Code**: HTTP status code (200, 400, 404, 500, etc.)
- **Headers**: Response headers including deprecation warnings
- **Body**: JSON response with the data structure
- **Schema**: Link to the response type definition

## Generating API Clients

### Using OpenAPI Generator

Generate a client library in your preferred language:

```bash
# Install OpenAPI Generator
npm install -g @openapitools/openapi-generator-cli

# Generate Rust client
openapi-generator-cli generate \
  -i docs/reference/openapi_v1.yaml \
  -g rust \
  -o client/rust/

# Generate Python client
openapi-generator-cli generate \
  -i docs/reference/openapi_v1.yaml \
  -g python \
  -o client/python/

# Generate TypeScript client
openapi-generator-cli generate \
  -i docs/reference/openapi_v1.yaml \
  -g typescript-axios \
  -o client/typescript/

# Generate Go client
openapi-generator-cli generate \
  -i docs/reference/openapi_v1.yaml \
  -g go \
  -o client/go/
```

### Using Swagger Codegen

Alternative client generation tool:

```bash
# Install Swagger Codegen
brew install swagger-codegen  # macOS
# or download from https://swagger.io/tools/swagger-codegen/

# Generate Java client
swagger-codegen generate \
  -i docs/reference/openapi_v1.yaml \
  -l java \
  -o client/java/

# Generate C# client
swagger-codegen generate \
  -i docs/reference/openapi_v1.yaml \
  -l csharp \
  -o client/csharp/
```

### Manual Client Creation

Use the OpenAPI spec as a reference when writing custom clients:

```python
import requests

class XzeClient:
    def __init__(self, base_url="http://localhost:3000"):
        self.base_url = base_url
        self.api_version = "v1"
    
    def health_check(self):
        """Check service health - GET /api/v1/health"""
        response = requests.get(f"{self.base_url}/api/{self.api_version}/health")
        return response.json()
    
    def analyze_repository(self, repo_url, branch=None, language=None):
        """Analyze repository - POST /api/v1/analyze"""
        payload = {
            "repository_url": repo_url,
            "branch": branch,
            "language": language
        }
        response = requests.post(
            f"{self.base_url}/api/{self.api_version}/analyze",
            json=payload
        )
        return response.json()
    
    def search(self, query, max_results=10, min_similarity=0.0, category=None):
        """Semantic search - GET /api/v1/search"""
        params = {
            "q": query,
            "max_results": max_results,
            "min_similarity": min_similarity,
        }
        if category:
            params["category"] = category
        
        response = requests.get(
            f"{self.base_url}/api/{self.api_version}/search",
            params=params
        )
        return response.json()

# Usage
client = XzeClient()
health = client.health_check()
print(f"Status: {health['status']}")

results = client.search("how to implement authentication", max_results=5)
print(f"Found {results['total_results']} results")
```

## Validating API Contracts

### Testing Against Specification

Use tools to validate your implementation matches the spec:

```bash
# Install Prism (API mocking and validation)
npm install -g @stoplight/prism-cli

# Validate server responses against spec
prism validate docs/reference/openapi_v1.yaml

# Mock the API for testing
prism mock docs/reference/openapi_v1.yaml
```

### Contract Testing

Ensure your client works with the API spec:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_health_response_matches_schema() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "1.0.0".to_string(),
            timestamp: chrono::Utc::now(),
        };
        
        // Serialize to JSON
        let json = serde_json::to_string(&response).unwrap();
        
        // Deserialize back
        let parsed: HealthResponse = serde_json::from_str(&json).unwrap();
        
        // Verify fields match schema requirements
        assert_eq!(response.status, parsed.status);
        assert_eq!(response.version, parsed.version);
    }
}
```

## Reading the Specification

### Understanding Paths

Each path in the specification describes an endpoint:

```yaml
paths:
  /api/v1/health:
    get:
      tags:
        - health
      summary: Health check endpoint
      responses:
        '200':
          description: Service is healthy
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/HealthResponse'
```

Key elements:
- **Path**: The URL path
- **Method**: HTTP method (get, post, put, delete)
- **Tags**: Organizational categories
- **Responses**: Possible response codes and schemas

### Understanding Schemas

Schemas define data structures:

```yaml
components:
  schemas:
    HealthResponse:
      type: object
      required:
        - status
        - version
        - timestamp
      properties:
        status:
          type: string
          example: healthy
        version:
          type: string
          example: 1.0.0
        timestamp:
          type: string
          format: date-time
```

Key elements:
- **type**: Data type (object, array, string, number, boolean)
- **required**: Required fields
- **properties**: Field definitions
- **example**: Example values

### Understanding Parameters

Query parameters are documented with types and constraints:

```yaml
parameters:
  - name: q
    in: query
    required: true
    schema:
      type: string
    description: Search query text
  - name: max_results
    in: query
    required: false
    schema:
      type: integer
      default: 10
    description: Maximum number of results
```

## Troubleshooting

### Swagger UI Not Loading

1. Verify server is running with OpenAPI feature:

   ```bash
   cargo run --features openapi
   ```

2. Check server logs for errors

3. Verify you're accessing the correct URL: `http://localhost:3000/api/v1/docs`

### Specification File Not Found

Regenerate specification files:

```bash
cargo run -p xze-serve --features openapi --example generate_openapi_spec
```

### Client Generation Fails

1. Verify OpenAPI specification is valid:

   ```bash
   npx @apidevtools/swagger-cli validate docs/reference/openapi_v1.yaml
   ```

2. Check OpenAPI generator version compatibility

3. Try alternative generators (openapi-generator vs swagger-codegen)

### API Response Doesn't Match Schema

1. Check API version in request URL
2. Verify you're using the correct specification version
3. Check for deprecation headers indicating legacy endpoints
4. Review server logs for errors

## Best Practices

### For API Consumers

1. **Always use versioned endpoints**: Use `/api/v1/` prefix
2. **Check deprecation headers**: Migrate away from deprecated endpoints
3. **Validate inputs**: Use schema definitions to validate requests before sending
4. **Handle all response codes**: Implement error handling for 400, 404, 500 responses
5. **Use examples**: Start with the examples in Swagger UI
6. **Generate typed clients**: Use code generation for type safety

### For API Developers

1. **Keep spec in sync**: Regenerate specs after API changes
2. **Add examples**: Include realistic examples for all endpoints
3. **Document errors**: Specify all possible error responses
4. **Use semantic versioning**: Version the API appropriately
5. **Test against spec**: Validate implementation matches specification

## Next Steps

- Read the API v1 specification: `docs/reference/api_v1_specification.md`
- Follow the migration guide: `docs/how_to/migrate_to_api_v1.md`
- Explore semantic search: Try the `/api/v1/search` endpoint
- Generate a client: Use openapi-generator for your language
- Set up contract testing: Validate your client against the spec

## Additional Resources

- OpenAPI Specification: https://swagger.io/specification/
- Swagger UI Documentation: https://swagger.io/tools/swagger-ui/
- OpenAPI Generator: https://openapi-generator.tech/
- Prism (API mocking): https://stoplight.io/open-source/prism
- Swagger Editor: https://editor.swagger.io/