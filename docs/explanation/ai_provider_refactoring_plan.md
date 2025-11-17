# AI Provider Abstraction Refactoring - Implementation Plan

## Overview

This plan outlines a phased approach to refactor XZe's AI integration from Ollama-only to a multi-provider abstraction layer supporting OpenAI, Anthropic, GitHub Copilot, and Ollama. The refactoring will maintain backward compatibility while introducing a clean provider abstraction that aligns with XZe's API-first, event-driven architecture.

**Goal**: Transform `xze-core/src/ai` from Ollama-specific implementation to a provider-agnostic system that supports multiple AI providers through a unified interface.

**Timeline**: 6-8 weeks (assuming 1 developer, part-time)

**Lines of Code**: ~6,350 LOC implementation + ~2,200 LOC tests = ~8,550 LOC total

## Current State Analysis

### Existing Infrastructure

**Current AI Integration** (`crates/core/src/ai/`):
- `client.rs` - `OllamaClient` implementation (Ollama-specific)
- `intent_classifier.rs` - Uses `OllamaClient` directly
- `context.rs` - AI context management
- `confidence.rs` - Response confidence scoring
- `health.rs` - Ollama health checks
- `prompts.rs` - Prompt templates
- `validator.rs` - Response validation
- `metrics.rs` - AI performance metrics

**Dependencies**:
- CLI commands (`crates/cli/src/commands/`) reference `OllamaClient` directly
- `AIAnalysisService` in core expects Ollama API format
- Configuration hardcoded to Ollama endpoints (`http://localhost:11434`)
- Environment variables: `OLLAMA_HOST`, `OLLAMA_MODEL`

**Architecture Context**:
- `xze-core` must remain independent of `xze-cli` and `xze-serve`
- Provider abstraction belongs in `xze-core/src/ai/providers/`
- Configuration should support multiple providers via `xze.yaml`

### Identified Issues

1. **Tight Coupling**: `OllamaClient` is directly instantiated throughout codebase
2. **No Provider Abstraction**: Cannot switch between AI providers
3. **Hardcoded URLs**: Ollama-specific endpoints embedded in code
4. **Limited Functionality**: Missing streaming support for other providers
5. **No Tool Call Abstraction**: Different providers have incompatible tool calling formats
6. **Configuration Inflexibility**: Single provider configuration model
7. **Testing Challenges**: Cannot mock or test against different providers

## Implementation Phases

### Phase 1: Core Provider Abstractions (Foundation)

**Objective**: Define provider-agnostic interfaces and types without breaking existing Ollama functionality.

**Estimated Effort**: 1 week | ~700 LOC implementation + ~200 LOC tests

#### Task 1.1: Define Provider Trait

Create base provider trait in `crates/core/src/ai/providers/base.rs`:

**Key Components**:
- `Provider` trait with `complete()` and `get_metadata()` methods
- `ProviderMetadata` struct for provider capabilities
- Default implementations for optional features

**Files to Create**:
- `crates/core/src/ai/providers/base.rs` - Core provider trait (~100 lines)
- `crates/core/src/ai/providers/mod.rs` - Module declarations (~50 lines)

**Architecture Alignment**:
- Trait uses `async_trait` for async methods
- Returns `Result<T, XzeError>` to match existing error handling
- No dependencies on interface layers

#### Task 1.2: Define Unified Types

Create provider-agnostic message and response types in `crates/core/src/ai/providers/types.rs`:

**Key Structures**:
- `Message` - Unified message format (role, content, metadata)
- `Tool` - Unified tool definition (name, description, JSON schema)
- `ToolCall` - Unified tool call representation
- `CompletionResponse` - Unified response format
- `Usage` - Token usage statistics
- `ModelConfig` - Model configuration (temperature, max_tokens, etc.)

**Files to Create**:
- `crates/core/src/ai/providers/types.rs` - Unified types (~200 lines)

**Design Decisions**:
- Types support all provider features (superset approach)
- Serialization with `serde` for configuration persistence
- Builder pattern for complex types

#### Task 1.3: Define Provider Errors

Create provider-specific error types in `crates/core/src/ai/providers/errors.rs`:

**Error Categories**:
- `ProviderError::Authentication` - Invalid credentials
- `ProviderError::RateLimit { retry_after: Duration }` - Rate limiting
- `ProviderError::ContextLength { max: usize, actual: usize }` - Input too long
- `ProviderError::ServerError { status: u16 }` - 5xx errors
- `ProviderError::RequestError { message: String }` - 4xx errors
- `ProviderError::Network { source: reqwest::Error }` - Connection issues
- `ProviderError::NotImplemented { feature: String }` - Unsupported features

**Files to Create**:
- `crates/core/src/ai/providers/errors.rs` - Error types (~100 lines)

**Integration**:
- Map to existing `XzeError` variants
- Implement `From<ProviderError>` for `XzeError`

#### Task 1.4: Define Provider Configuration

Create configuration structure in `crates/core/src/ai/providers/config.rs`:

**Configuration Structure**:
```yaml
ai:
  provider: "ollama"  # or "openai", "anthropic", "copilot"
  model: "qwen3"
  fast_model: "qwen3:instruct"  # optional, for quick operations

  # Provider-specific settings
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://api.openai.com"
    timeout: 600

  anthropic:
    api_key: "${ANTHROPIC_API_KEY}"
    base_url: "https://api.anthropic.com"

  ollama:
    base_url: "http://localhost:11434"
    timeout: 300
```

**Files to Create**:
- `crates/core/src/ai/providers/config.rs` - Configuration types (~150 lines)

**Features**:
- Environment variable substitution
- Validation on load
- Per-provider configuration sections

#### Task 1.5: Testing Requirements

**Unit Tests**:
- Message/Tool/Response serialization/deserialization
- Error type conversions to `XzeError`
- Configuration loading from YAML and environment variables
- Builder pattern tests for complex types

**Files to Create**:
- `crates/core/src/ai/providers/base.rs` - Trait tests (~50 lines)
- `crates/core/src/ai/providers/types.rs` - Type tests (~100 lines)
- `crates/core/src/ai/providers/errors.rs` - Error tests (~50 lines)

#### Task 1.6: Deliverables

- [ ] `crates/core/src/ai/providers/base.rs` - Provider trait
- [ ] `crates/core/src/ai/providers/types.rs` - Unified types
- [ ] `crates/core/src/ai/providers/errors.rs` - Error types
- [ ] `crates/core/src/ai/providers/config.rs` - Configuration
- [ ] `crates/core/src/ai/providers/mod.rs` - Module structure
- [ ] Unit tests with >80% coverage
- [ ] Updated `docs/reference/architecture.md` with provider abstraction section

#### Task 1.7: Success Criteria

- [ ] Provider trait compiles and passes clippy
- [ ] All types serialize/deserialize correctly
- [ ] Error types map to existing `XzeError` variants
- [ ] Configuration loads from YAML and environment
- [ ] `cargo test --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Test coverage >80%
- [ ] No breaking changes to existing Ollama functionality

### Phase 2: HTTP Client and Retry Infrastructure

**Objective**: Build reusable HTTP client with authentication, retry logic, and request formatting.

**Estimated Effort**: 1.5 weeks | ~1,250 LOC implementation + ~400 LOC tests

#### Task 2.1: Implement Generic API Client

Create HTTP client abstraction in `crates/core/src/ai/providers/api_client.rs`:

**Key Components**:
- `ApiClient` struct with configurable authentication
- `AuthMethod` enum (Bearer, ApiKey, OAuth, None)
- Connection pooling and timeout configuration
- Request/response logging (debug mode only)
- Custom header support

**Files to Create**:
- `crates/core/src/ai/providers/api_client.rs` - HTTP client (~300 lines)

**Features**:
- Built on `reqwest::Client` (already in dependencies)
- Async methods using `tokio`
- Structured logging with `tracing`
- Header injection for provider-specific requirements

#### Task 2.2: Implement Retry Logic

Create retry mechanism in `crates/core/src/ai/providers/retry.rs`:

**Retry Strategy**:
- Exponential backoff (configurable multiplier)
- Maximum retry attempts (default: 3)
- Retry on specific status codes: 429, 500, 502, 503, 504
- Respect `Retry-After` header for rate limits
- Jitter to prevent thundering herd

**Files to Create**:
- `crates/core/src/ai/providers/retry.rs` - Retry logic (~150 lines)

**Configuration**:
```yaml
ai:
  retry:
    max_attempts: 3
    initial_delay_ms: 1000
    max_delay_ms: 60000
    backoff_multiplier: 2.0
```

#### Task 2.3: Implement Request Formatters

Create provider-specific request/response formatters:

**OpenAI Format** (`crates/core/src/ai/providers/formats/openai.rs`):
- Convert unified `Message` to OpenAI chat format
- System prompt as first message
- Tool calls in separate `tool_calls` field
- Tool results as `tool` role messages

**Anthropic Format** (`crates/core/src/ai/providers/formats/anthropic.rs`):
- Convert unified `Message` to Anthropic format
- System prompt in separate `system` field
- Tool calls in content array as `tool_use` blocks
- Tool results as user messages with `tool_result` content

**Files to Create**:
- `crates/core/src/ai/providers/formats/mod.rs` - Format module (~50 lines)
- `crates/core/src/ai/providers/formats/openai.rs` - OpenAI format (~200 lines)
- `crates/core/src/ai/providers/formats/anthropic.rs` - Anthropic format (~200 lines)

**Design Pattern**:
- Traits for request/response conversion
- Bidirectional mapping (to/from unified types)
- Validation during conversion

#### Task 2.4: Testing Requirements

**Unit Tests**:
- API client authentication header construction
- Retry logic with mock failures (succeed after N retries)
- Request formatting for each provider (compare against known good JSON)
- Response parsing for each provider
- Error handling (timeouts, network errors)

**Integration Tests**:
- Mock HTTP server using `wiremock` crate
- Test complete request/response cycle
- Test retry behavior with server errors

**Files to Create**:
- `crates/core/src/ai/providers/api_client.rs` - Client tests (~150 lines)
- `crates/core/src/ai/providers/retry.rs` - Retry tests (~100 lines)
- `crates/core/src/ai/providers/formats/openai.rs` - Format tests (~75 lines)
- `crates/core/src/ai/providers/formats/anthropic.rs` - Format tests (~75 lines)

#### Task 2.5: Deliverables

- [ ] `crates/core/src/ai/providers/api_client.rs` - Generic HTTP client
- [ ] `crates/core/src/ai/providers/retry.rs` - Retry logic
- [ ] `crates/core/src/ai/providers/formats/openai.rs` - OpenAI formatter
- [ ] `crates/core/src/ai/providers/formats/anthropic.rs` - Anthropic formatter
- [ ] Unit tests with >80% coverage
- [ ] Integration tests with mock HTTP server

#### Task 2.6: Success Criteria

- [ ] API client handles all authentication methods
- [ ] Retry logic respects backoff and max attempts
- [ ] Request formatters produce valid provider JSON
- [ ] Response parsers handle all provider response formats
- [ ] Mock server tests verify end-to-end behavior
- [ ] `cargo test --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Test coverage >80%

### Phase 3: Provider Implementations

**Objective**: Implement each provider using the abstractions, starting with Ollama adapter to maintain compatibility.

**Estimated Effort**: 2 weeks | ~2,100 LOC implementation + ~600 LOC tests

#### Task 3.1: Adapt Existing Ollama Client

Refactor `crates/core/src/ai/client.rs` to implement `Provider` trait:

**Refactoring Strategy**:
1. Keep existing `OllamaClient` intact for backward compatibility
2. Create `OllamaProvider` wrapper that implements `Provider` trait
3. Add conversion methods between old and new types
4. Update `client.rs` to re-export `OllamaProvider` as `OllamaClient` (alias)

**Files to Modify**:
- `crates/core/src/ai/client.rs` - Add `Provider` implementation (~100 lines added)

**Files to Create**:
- `crates/core/src/ai/providers/ollama.rs` - Ollama provider (~250 lines)

**Migration Path**:
```rust
// Old code (still works)
let client = OllamaClient::new("http://localhost:11434".into());

// New code (Provider trait)
let provider: Box<dyn Provider> = Box::new(OllamaProvider::new(config));
```

#### Task 3.2: Implement OpenAI Provider

Create OpenAI provider in `crates/core/src/ai/providers/openai.rs`:

**OpenAI Specifics**:
- Base URL: `https://api.openai.com/v1/chat/completions`
- Authentication: `Authorization: Bearer <api_key>`
- Default model: `gpt-4o`
- Fast model: `gpt-4o-mini`
- Context limit: 128K tokens
- Native function calling

**Environment Variables**:
- `OPENAI_API_KEY` (required)
- `OPENAI_HOST` (optional, custom endpoint)
- `OPENAI_ORGANIZATION` (optional)
- `OPENAI_TIMEOUT` (optional, default: 600)

**Files to Create**:
- `crates/core/src/ai/providers/openai.rs` - OpenAI provider (~300 lines)

**Key Methods**:
- `new(config)` - Constructor with configuration
- `complete(messages, tools)` - Standard completion
- `complete_fast(messages, tools)` - Using fast model
- `get_metadata()` - Provider capabilities

#### Task 3.3: Implement Anthropic Provider

Create Anthropic provider in `crates/core/src/ai/providers/anthropic.rs`:

**Anthropic Specifics**:
- Base URL: `https://api.anthropic.com/v1/messages`
- Authentication: `x-api-key: <api_key>`
- Required header: `anthropic-version: 2023-06-01`
- Default model: `claude-sonnet-4-0`
- Fast model: `claude-3-7-sonnet-latest`
- Context limit: 200K tokens
- Tool calls in content array

**Environment Variables**:
- `ANTHROPIC_API_KEY` (required)
- `ANTHROPIC_HOST` (optional)
- `ANTHROPIC_TIMEOUT` (optional, default: 600)

**Files to Create**:
- `crates/core/src/ai/providers/anthropic.rs` - Anthropic provider (~350 lines)

**Special Handling**:
- System prompt extraction (separate field, not message)
- Tool call parsing from content blocks
- Tool result formatting (user message with `tool_result`)
- Total tokens calculation (input + output)

#### Task 3.4: Implement GitHub Copilot Provider

Create Copilot provider in `crates/core/src/ai/providers/copilot.rs`:

**Copilot Specifics**:
- Base URL: `https://api.githubcopilot.com/chat/completions`
- Authentication: OAuth device flow (complex)
- OpenAI-compatible API format
- Requires GitHub account access

**OAuth Flow**:
1. Request device code from GitHub OAuth
2. Display user code and verification URL
3. Poll for access token
4. Store token securely for reuse

**Environment Variables**:
- `GITHUB_TOKEN` (optional, for OAuth)
- `COPILOT_API_KEY` (alternative auth)

**Files to Create**:
- `crates/core/src/ai/providers/copilot.rs` - Copilot provider (~400 lines)
- `crates/core/src/ai/providers/oauth.rs` - OAuth utilities (~200 lines)

**Note**: Can reuse OpenAI request/response formatters

#### Task 3.5: Testing Requirements

**Unit Tests**:
- Provider instantiation from configuration
- Request formatting for each provider
- Response parsing for each provider
- Tool call extraction and formatting
- Error mapping (all HTTP status codes)
- Usage statistics extraction

**Integration Tests**:
- Mock HTTP server responses for each provider
- End-to-end completion requests
- Error scenarios (auth failures, rate limits, timeouts)
- Tool calling workflows

**Files to Create**:
- `crates/core/src/ai/providers/ollama.rs` - Ollama tests (~100 lines)
- `crates/core/src/ai/providers/openai.rs` - OpenAI tests (~150 lines)
- `crates/core/src/ai/providers/anthropic.rs` - Anthropic tests (~150 lines)
- `crates/core/src/ai/providers/copilot.rs` - Copilot tests (~150 lines)
- `crates/core/src/ai/providers/oauth.rs` - OAuth tests (~50 lines)

#### Task 3.6: Deliverables

- [ ] `crates/core/src/ai/providers/ollama.rs` - Ollama provider (adapted)
- [ ] `crates/core/src/ai/providers/openai.rs` - OpenAI provider
- [ ] `crates/core/src/ai/providers/anthropic.rs` - Anthropic provider
- [ ] `crates/core/src/ai/providers/copilot.rs` - Copilot provider
- [ ] `crates/core/src/ai/providers/oauth.rs` - OAuth device flow
- [ ] Backward compatibility maintained for `OllamaClient`
- [ ] Unit tests with >80% coverage per provider
- [ ] Integration tests with mock responses

#### Task 3.7: Success Criteria

- [ ] All four providers implement `Provider` trait
- [ ] Ollama provider maintains backward compatibility
- [ ] OpenAI provider handles standard and custom endpoints
- [ ] Anthropic provider correctly handles system prompts and tool calls
- [ ] Copilot provider completes OAuth flow (manual testing)
- [ ] All providers parse tool calls correctly
- [ ] Error mapping works for all HTTP status codes
- [ ] `cargo test --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Test coverage >80%
- [ ] Existing functionality unaffected (run integration tests)

### Phase 4: Streaming Support

**Objective**: Add streaming response support for real-time output across all providers.

**Estimated Effort**: 1.5 weeks | ~950 LOC implementation + ~300 LOC tests

#### Task 4.1: Define Streaming Interface

Extend `Provider` trait with streaming methods:

**Streaming Types**:
- `ResponseChunk` - Incremental content delta
- `ToolCallDelta` - Partial tool call (JSON may be split)
- `StreamingProvider` trait extension

**Files to Create**:
- `crates/core/src/ai/providers/streaming.rs` - Streaming types (~100 lines)

**Files to Modify**:
- `crates/core/src/ai/providers/base.rs` - Add streaming methods (~50 lines)

#### Task 4.2: Implement SSE Parser

Create Server-Sent Events parser for OpenAI/Anthropic/Copilot:

**SSE Format**:
```
data: {"id":"1","choices":[{"delta":{"content":"Hello"}}]}

data: {"id":"1","choices":[{"delta":{"content":" world"}}]}

data: [DONE]
```

**Parser Features**:
- Line-by-line parsing
- JSON deserialization per event
- Handle `[DONE]` sentinel
- Error recovery (skip malformed chunks)
- Chunk accumulation for tool calls

**Files to Create**:
- `crates/core/src/ai/providers/sse_parser.rs` - SSE parsing (~200 lines)

#### Task 4.3: Implement Streaming for Each Provider

Add streaming support to each provider implementation:

**OpenAI Streaming**:
- Set `"stream": true` in request
- Parse SSE response
- Accumulate JSON fragments for tool calls

**Anthropic Streaming**:
- Set `"stream": true` in request
- Parse SSE with different event types
- Handle `content_block_start`, `content_block_delta`, `content_block_stop`

**Copilot Streaming**:
- Same as OpenAI (compatible format)

**Ollama Streaming**:
- Set `"stream": true` in request
- Parse newline-delimited JSON (not SSE)
- Each line is complete JSON object

**Files to Modify**:
- `crates/core/src/ai/providers/openai.rs` - Add `stream()` (~100 lines)
- `crates/core/src/ai/providers/anthropic.rs` - Add `stream()` (~150 lines)
- `crates/core/src/ai/providers/copilot.rs` - Add `stream()` (~100 lines)
- `crates/core/src/ai/providers/ollama.rs` - Add `stream()` (~100 lines)

#### Task 4.4: Testing Requirements

**Unit Tests**:
- SSE parsing with various chunk patterns
- Delta accumulation (ensure content merges correctly)
- Tool call streaming (JSON split across chunks)
- `[DONE]` marker handling
- Newline-delimited JSON parsing (Ollama)

**Integration Tests**:
- Mock streaming responses
- End-to-end streaming request
- Stream interruption handling
- Error in stream recovery

**Files to Create**:
- `crates/core/src/ai/providers/sse_parser.rs` - SSE tests (~100 lines)
- `crates/core/src/ai/providers/streaming.rs` - Streaming tests (~200 lines)

#### Task 4.5: Deliverables

- [ ] `crates/core/src/ai/providers/streaming.rs` - Streaming types
- [ ] `crates/core/src/ai/providers/sse_parser.rs` - SSE parser
- [ ] Streaming methods in all four providers
- [ ] Unit tests with >80% coverage
- [ ] Integration tests with mock streams

#### Task 4.6: Success Criteria

- [ ] SSE parser handles all event formats
- [ ] Streaming works for all providers
- [ ] Deltas accumulate correctly
- [ ] Tool calls parse correctly from streamed JSON
- [ ] Stream errors handled gracefully
- [ ] `cargo test --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Test coverage >80%

### Phase 5: Provider Factory and Configuration Integration

**Objective**: Create factory pattern for provider instantiation and integrate with XZe configuration system.

**Estimated Effort**: 1 week | ~700 LOC implementation + ~200 LOC tests

#### Task 5.1: Implement Provider Factory

Create factory for dynamic provider instantiation:

**Factory Pattern**:
- `ProviderFactory::create(config)` returns `Box<dyn Provider>`
- Provider registration at module initialization
- Metadata for discovery (supported features, models, config keys)
- Validation of configuration before provider creation

**Files to Create**:
- `crates/core/src/ai/providers/factory.rs` - Provider factory (~200 lines)
- `crates/core/src/ai/providers/registry.rs` - Provider registry (~100 lines)

**Usage Pattern**:
```rust
// From configuration
let config = ProviderConfig::from_yaml("xze.yaml")?;
let provider = ProviderFactory::create(&config)?;

// Dynamic selection
let provider = ProviderFactory::create_from_name("openai", openai_config)?;
```

#### Task 5.2: Integrate with XZe Configuration

Update `crates/core/src/config.rs` to support provider configuration:

**Configuration Schema** (`xze.yaml`):
```yaml
ai:
  provider: "openai"  # Active provider
  default_model: "gpt-4o"
  fast_model: "gpt-4o-mini"

  retry:
    max_attempts: 3
    initial_delay_ms: 1000

  providers:
    openai:
      api_key: "${OPENAI_API_KEY}"
      base_url: "https://api.openai.com"
      timeout: 600

    anthropic:
      api_key: "${ANTHROPIC_API_KEY}"
      base_url: "https://api.anthropic.com"

    ollama:
      base_url: "http://localhost:11434"
      timeout: 300
```

**Files to Modify**:
- `crates/core/src/config.rs` - Add provider configuration (~100 lines)

**Files to Create**:
- `crates/core/src/ai/providers/metadata.rs` - Provider metadata (~200 lines)

#### Task 5.3: Update AIAnalysisService

Refactor `AIAnalysisService` to use provider abstraction:

**Current**: Directly uses `OllamaClient`
**Target**: Uses `Box<dyn Provider>` injected via constructor

**Files to Modify**:
- `crates/core/src/ai/mod.rs` - Update service to use Provider trait (~50 lines changed)
- Create `crates/core/src/ai/service.rs` if not exists

**Migration Strategy**:
1. Add provider parameter to service constructor
2. Replace direct `OllamaClient` calls with `Provider` trait methods
3. Maintain backward compatibility with default Ollama provider

#### Task 5.4: Testing Requirements

**Unit Tests**:
- Factory creates correct provider from configuration
- Metadata retrieval for all providers
- Configuration validation (missing keys, invalid values)
- Provider not found error handling
- Environment variable substitution

**Integration Tests**:
- Load configuration from YAML file
- Create provider and execute completion
- Switch between providers at runtime

**Files to Create**:
- `crates/core/src/ai/providers/factory.rs` - Factory tests (~100 lines)
- `crates/core/src/ai/providers/registry.rs` - Registry tests (~50 lines)
- `crates/core/src/ai/providers/metadata.rs` - Metadata tests (~50 lines)

#### Task 5.5: Deliverables

- [ ] `crates/core/src/ai/providers/factory.rs` - Provider factory
- [ ] `crates/core/src/ai/providers/registry.rs` - Provider registry
- [ ] `crates/core/src/ai/providers/metadata.rs` - Provider metadata
- [ ] Updated `crates/core/src/config.rs` with provider configuration
- [ ] Refactored `AIAnalysisService` to use provider abstraction
- [ ] Example `xze.yaml` with all provider configurations
- [ ] Unit tests with >80% coverage

#### Task 5.6: Success Criteria

- [ ] Factory creates all four providers from configuration
- [ ] Metadata is complete for all providers
- [ ] Invalid provider names return descriptive errors
- [ ] Missing configuration keys detected and reported
- [ ] `AIAnalysisService` works with any provider
- [ ] Backward compatibility maintained (Ollama as default)
- [ ] `cargo test --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Test coverage >80%
- [ ] Configuration loads from `xze.yaml` successfully

### Phase 6: CLI and Integration Updates (Final Integration)

**Objective**: Update CLI commands and documentation to support multiple providers.

**Estimated Effort**: 1 week | ~400 LOC modifications + ~300 LOC tests

#### Task 6.1: Update CLI Commands

Update CLI commands to support provider selection:

**Commands to Update**:
- `crates/cli/src/commands/chunk.rs` - Add provider flag
- `crates/cli/src/commands/classify.rs` - Add provider flag
- `crates/cli/src/commands/search.rs` - Add provider flag
- Any other commands using `OllamaClient`

**New CLI Flags**:
```bash
xze classify --query "How do I..." --provider openai --model gpt-4o
xze chunk --input docs/ --provider anthropic
xze search --query "semantic" --provider ollama
```

**Files to Modify**:
- `crates/cli/src/commands/*.rs` - Add provider arguments (~50 lines per file)

**Backward Compatibility**:
- Default to Ollama if `--provider` not specified
- Support legacy `--ollama-url` flag for backward compatibility

#### Task 6.2: Add Provider Management Commands

Add CLI commands for provider management:

**New Commands**:
```bash
xze provider list                    # List available providers
xze provider info <name>             # Show provider details
xze provider test <name>             # Test provider connection
xze provider set-default <name>      # Set default provider
```

**Files to Create**:
- `crates/cli/src/commands/provider.rs` - Provider management commands (~200 lines)

#### Task 6.3: Update Documentation

Update all documentation to reflect multi-provider support:

**Documentation Updates**:
- `docs/reference/architecture.md` - Add provider abstraction section
- `docs/how_to/configure_ai_providers.md` - NEW: Provider configuration guide
- `docs/how_to/switch_ai_providers.md` - NEW: Switching providers
- `README.md` - Update AI provider section
- `docs/explanation/implementations.md` - Append implementation summary

**Files to Create**:
- `docs/how_to/configure_ai_providers.md` - Configuration guide (~300 lines)
- `docs/how_to/switch_ai_providers.md` - Switching guide (~200 lines)

**Files to Modify**:
- `docs/reference/architecture.md` - Add Section 3.6: AI Provider Abstraction (~500 lines)
- `README.md` - Update AI integration section (~100 lines)
- `docs/explanation/implementations.md` - Append summary (~200 lines)

#### Task 6.4: Testing Requirements

**Integration Tests**:
- End-to-end CLI commands with different providers
- Configuration loading from `xze.yaml`
- Provider switching at runtime
- Error handling (invalid provider, missing API key)

**Manual Testing Checklist**:
- [ ] Run `xze classify` with OpenAI
- [ ] Run `xze classify` with Anthropic
- [ ] Run `xze classify` with Ollama (default)
- [ ] Run `xze provider list`
- [ ] Run `xze provider test openai`
- [ ] Switch provider via configuration
- [ ] Verify streaming output works

**Files to Create**:
- `crates/cli/tests/integration/provider_commands.rs` - Integration tests (~300 lines)

#### Task 6.5: Deliverables

- [ ] Updated CLI commands with provider flags
- [ ] New `provider` management commands
- [ ] Updated `docs/reference/architecture.md`
- [ ] New how-to guides for provider configuration
- [ ] Updated `README.md`
- [ ] Implementation summary in `docs/explanation/implementations.md`
- [ ] Integration tests for all provider scenarios

#### Task 6.6: Success Criteria

- [ ] All CLI commands support `--provider` flag
- [ ] Provider management commands work correctly
- [ ] Documentation complete and accurate
- [ ] Manual testing checklist completed
- [ ] `cargo test --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Integration tests pass with mock providers
- [ ] Backward compatibility verified (existing scripts work)

## Migration Strategy

### Backward Compatibility Plan

**Phase 1-3**: Zero Breaking Changes
- Existing `OllamaClient` usage continues to work
- New provider abstraction added alongside

**Phase 4-5**: Optional Migration
- `AIAnalysisService` uses provider abstraction internally
- Externally, behavior unchanged (defaults to Ollama)

**Phase 6**: Full Migration Available
- CLI commands support explicit provider selection
- Configuration system supports multiple providers
- Legacy flags still work (deprecated warnings)

**Future (Post-Phase 6)**: Deprecation
- Mark `OllamaClient` as deprecated (not removal)
- Encourage migration to provider abstraction
- Maintain backward compatibility indefinitely

### Rollback Plan

Each phase is independently testable and can be rolled back:

1. **Phase 1-2**: Abstract types, no behavioral changes - safe to merge
2. **Phase 3**: Provider implementations behind feature flags - can be disabled
3. **Phase 4**: Streaming is additive - can be skipped if issues arise
4. **Phase 5**: Factory pattern is internal - no API changes
5. **Phase 6**: CLI updates are backward compatible - can revert flags

**Rollback Strategy**:
- Each phase in separate PR with comprehensive tests
- Feature flags for new providers (`--features openai,anthropic,copilot`)
- Keep `OllamaClient` as default and fallback

## Open Questions

### Technical Decisions

1. **Streaming vs. Async Iterators**: Use `futures::Stream` or custom iterator pattern?
   - **Recommendation**: `futures::Stream` for ecosystem compatibility

2. **Provider Plugin System**: Should providers be dynamically loadable?
   - **Recommendation**: No, compile-time linking sufficient for now

3. **Caching Layer**: Should responses be cached across providers?
   - **Recommendation**: Phase 7 (future work), not critical path

4. **Cost Tracking**: Track API usage and costs per provider?
   - **Recommendation**: Phase 7 (future work), metadata exists for this

5. **Model Aliases**: Support custom model aliases across providers?
   - **Recommendation**: Yes, in configuration system

### Configuration Questions

1. **Multiple Providers Simultaneously**: Should XZe support multiple active providers?
   - **Recommendation**: Yes, with primary/fallback pattern

2. **Provider-Specific Features**: How to expose provider-unique features?
   - **Recommendation**: Provider metadata + optional trait methods

3. **API Key Storage**: Use environment variables or secure keychain?
   - **Recommendation**: Both, environment variables for containers, keychain for local

## Implementation Estimates

### Effort Breakdown

| Phase | Development | Testing | Documentation | Total |
|-------|-------------|---------|---------------|-------|
| Phase 1: Core Abstractions | 3 days | 1 day | 1 day | 1 week |
| Phase 2: HTTP Client | 4 days | 2 days | 1 day | 1.5 weeks |
| Phase 3: Provider Implementations | 6 days | 3 days | 1 day | 2 weeks |
| Phase 4: Streaming Support | 4 days | 2 days | 1 day | 1.5 weeks |
| Phase 5: Factory & Config | 3 days | 1 day | 1 day | 1 week |
| Phase 6: CLI & Integration | 3 days | 2 days | 2 days | 1 week |
| **Total** | **23 days** | **11 days** | **7 days** | **8 weeks** |

### Lines of Code

| Phase | Implementation | Tests | Total |
|-------|----------------|-------|-------|
| Phase 1 | 700 | 200 | 900 |
| Phase 2 | 1,250 | 400 | 1,650 |
| Phase 3 | 2,100 | 600 | 2,700 |
| Phase 4 | 950 | 300 | 1,250 |
| Phase 5 | 700 | 200 | 900 |
| Phase 6 | 400 | 300 | 700 |
| **Total** | **6,100** | **2,000** | **8,100** |

*Note: Documentation not counted in LOC estimates*

## Risk Assessment

### High Risk Items

1. **OAuth Flow for Copilot**: Complex authentication, requires manual testing
   - **Mitigation**: Implement last, provide clear setup documentation

2. **Anthropic Tool Call Format**: Significantly different from OpenAI
   - **Mitigation**: Comprehensive format tests, reference implementations

3. **Streaming Implementation**: Complex state management for chunked responses
   - **Mitigation**: Extensive unit tests, graceful error handling

### Medium Risk Items

4. **Breaking Changes**: Accidentally breaking existing Ollama functionality
   - **Mitigation**: Comprehensive regression tests, feature flags

5. **Configuration Complexity**: Supporting multiple providers increases config surface
   - **Mitigation**: Validation, clear error messages, examples

### Low Risk Items

6. **Performance Overhead**: Abstraction layer may add latency
   - **Mitigation**: Benchmark tests, optimize hot paths

7. **Provider-Specific Edge Cases**: Unique behaviors per provider
   - **Mitigation**: Provider-specific test suites, documentation

## Testing Strategy

### Unit Test Requirements

**Coverage Target**: >80% per module

**Required Tests**:
- Type serialization/deserialization
- Error type conversions
- Request/response formatting for each provider
- Retry logic with various failure patterns
- Configuration loading and validation
- Provider metadata accuracy

### Integration Test Requirements

**Test Scenarios**:
- Mock HTTP server for each provider
- Complete request/response cycles
- Error handling (auth failures, rate limits, timeouts)
- Streaming responses with chunk accumulation
- Tool calling workflows end-to-end

### Manual Test Plan

**Pre-Release Checklist**:
- [ ] OpenAI completion with API key
- [ ] Anthropic completion with API key
- [ ] Ollama completion (local)
- [ ] Copilot OAuth flow (manual)
- [ ] Streaming output from each provider
- [ ] Tool calling with each provider
- [ ] Provider switching via configuration
- [ ] CLI commands with different providers
- [ ] Error scenarios (invalid keys, rate limits)
- [ ] Backward compatibility (existing scripts)

## Security Considerations

### API Key Management

**Requirements**:
- Never log API keys (even in debug mode)
- Support environment variable substitution
- Warn if API keys in plaintext configuration
- Recommend secure keychain for local development

**Implementation**:
- Redact API keys in error messages
- Use `SecretString` type for sensitive data
- Validate key format before storing

### Network Security

**Requirements**:
- HTTPS for all external providers (except localhost Ollama)
- Certificate validation enabled
- Timeout configuration to prevent hangs
- Rate limiting to prevent abuse

### Data Privacy

**Requirements**:
- Log prompts only in debug mode
- Sanitize sensitive data in logs
- Clear documentation on data sent to providers
- Support for on-premises/local providers (Ollama)

## Documentation Requirements

### API Documentation

**Required Sections**:
- Provider trait documentation with examples
- Configuration schema reference
- Error handling guide
- Migration guide from `OllamaClient`

### How-To Guides

**Required Guides**:
- Configure AI providers
- Switch between providers
- Set up OpenAI integration
- Set up Anthropic integration
- Set up GitHub Copilot
- Troubleshoot provider issues
- Implement custom provider (advanced)

### Reference Documentation

**Required References**:
- Provider comparison matrix
- Model capabilities by provider
- Configuration options per provider
- Error codes and meanings
- API endpoint specifications

## Success Metrics

### Phase Completion Criteria

**Each Phase Must Achieve**:
- [ ] All deliverables completed
- [ ] Test coverage >80%
- [ ] `cargo test --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Documentation updated
- [ ] Code review completed
- [ ] No regressions in existing functionality

### Project Success Criteria

**Overall Project Success**:
- [ ] All 6 phases completed
- [ ] All 4 providers implemented and tested
- [ ] CLI supports provider selection
- [ ] Configuration system supports multiple providers
- [ ] Backward compatibility maintained
- [ ] Documentation complete and accurate
- [ ] Integration tests pass
- [ ] Manual testing checklist completed
- [ ] Performance benchmarks acceptable (<10% overhead)

## References

### Internal Documentation

- `docs/explanation/provider_abstraction_implementation_plan.md` - Full technical plan
- `docs/explanation/provider_abstraction_quick_reference.md` - Quick reference
- `docs/reference/architecture.md` - XZe architecture
- `AGENTS.md` - Development guidelines

### External API Documentation

- OpenAI API: https://platform.openai.com/docs/api-reference
- Anthropic API: https://docs.anthropic.com/en/api
- GitHub Copilot: https://docs.github.com/en/copilot
- Ollama API: https://github.com/ollama/ollama/blob/main/docs/api.md

### Implementation Examples

- OpenAI Rust SDK: https://github.com/64bit/async-openai
- Anthropic Rust SDK: https://github.com/anthropics/anthropic-sdk-rust (unofficial)
- Reqwest Examples: https://github.com/seanmonstar/reqwest/tree/master/examples

---

**Last Updated**: 2025-01-20
**Author**: AI Planning Agent
**Status**: Ready for Review
