# XZe Pipeline Documentation Tool - Architectural Design Plan

## 1. System Architecture Overview

### 1.1 High-Level Architecture

XZe follows a modular, layered architecture designed for flexibility across multiple deployment modes (CLI, Server, VSCode Agent). The system is built in Rust for performance, safety, and excellent async capabilities.

```
┌─────────────────────────────────────────────────────────────┐
│                     Interface Layer                          │
│  ┌──────────┐  ┌──────────┐  ┌────────────────────┐        │
│  │   CLI    │  │  Server  │  │  VSCode Extension  │        │
│  └──────────┘  └──────────┘  └────────────────────┘        │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                    Orchestration Layer                       │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         Documentation Pipeline Controller             │  │
│  │  - Mode Management (Local/Auto)                       │  │
│  │  - Workflow Coordination                              │  │
│  │  - Event Processing                                   │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                     Service Layer                            │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Repository   │  │ AI Analysis  │  │  Documentation  │  │
│  │ Manager      │  │ Service      │  │  Generator      │  │
│  └──────────────┘  └──────────────┘  └─────────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Git          │  │ Change       │  │  PR Manager     │  │
│  │ Operations   │  │ Detector     │  │                 │  │
│  └──────────────┘  └──────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                    Infrastructure Layer                      │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Ollama       │  │ File System  │  │  Logging        │  │
│  │ Client       │  │ Abstraction  │  │  (JSON)         │  │
│  └──────────────┘  └──────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 Core Design Principles

- **Modularity**: Each component is independently testable and replaceable
- **Async-First**: Leverage Tokio for concurrent repository analysis
- **Configuration-Driven**: YAML-based configuration for repositories and models
- **Stateless Processing**: Each documentation generation is independent
- **Observable**: Structured JSON logging for all operations

## 2. Component Design

### 2.1 Interface Layer

#### 2.1.1 CLI Module (`xze-cli`)

**Responsibilities**:
- Parse command-line arguments
- Validate input parameters
- Invoke orchestration layer
- Display progress and results

**Key Interfaces**:
```rust
pub struct CliConfig {
    mode: OperationMode,
    repositories: Vec<PathBuf>,
    config_file: Option<PathBuf>,
    output_repo: PathBuf,
    dry_run: bool,
}

pub enum OperationMode {
    Local { paths: Vec<PathBuf> },
    Auto { config: PathBuf },
}
```

**Commands**:
- `xze analyze <repo-paths>` - Analyze repositories in local mode
- `xze watch <config.yaml>` - Monitor repositories in auto mode
- `xze generate <repo-path>` - Generate documentation for single repository
- `xze validate <doc-repo>` - Validate existing documentation

#### 2.1.2 Server Module (`xze-server`)

**Responsibilities**:
- RESTful API endpoint exposure
- WebSocket support for real-time updates
- Authentication/Authorization (future)
- Request queuing and rate limiting

**API Endpoints**:
```
POST   /api/v1/analyze          - Trigger analysis
GET    /api/v1/status/:job_id   - Check job status
GET    /api/v1/repositories     - List tracked repositories
POST   /api/v1/repositories     - Add repository to watch list
DELETE /api/v1/repositories/:id - Remove repository
WS     /api/v1/stream           - Real-time event stream
```

**Technology Stack**:
- Framework: `axum` (async web framework)
- Serialization: `serde_json`
- WebSocket: `tokio-tungstenite`

#### 2.1.3 VSCode Extension Module (`xze-vscode`)

**Responsibilities**:
- Integrate with VSCode workspace
- Provide commands via command palette
- Display documentation generation progress
- Copilot integration for documentation suggestions

**Components**:
- TypeScript extension host
- Rust binary (xze-cli) invocation via Node.js child process
- Language Server Protocol (LSP) for documentation awareness

### 2.2 Orchestration Layer

#### 2.2.1 Documentation Pipeline Controller

**Responsibilities**:
- Coordinate end-to-end documentation workflow
- Manage state transitions
- Handle error recovery
- Dispatch work to service layer

**Workflow State Machine**:
```
┌──────────────┐
│   Idle       │
└──────┬───────┘
       │
       ↓
┌──────────────┐     ┌──────────────┐
│  Cloning     │────>│   Analyzing  │
└──────────────┘     └──────┬───────┘
                            │
                            ↓
                     ┌──────────────┐
                     │  Generating  │
                     └──────┬───────┘
                            │
                            ↓
                     ┌──────────────┐
                     │  Reviewing   │
                     └──────┬───────┘
                            │
                            ↓
                     ┌──────────────┐
                     │ Creating PR  │
                     └──────┬───────┘
                            │
                            ↓
                     ┌──────────────┐
                     │  Completed   │
                     └──────────────┘
```

**Key Structure**:
```rust
pub struct PipelineController {
    repo_manager: Arc<RepositoryManager>,
    ai_service: Arc<AIAnalysisService>,
    doc_generator: Arc<DocumentationGenerator>,
    git_ops: Arc<GitOperations>,
    logger: Arc<StructuredLogger>,
}

pub struct PipelineJob {
    id: Uuid,
    source_repo: Repository,
    target_repo: Repository,
    state: JobState,
    metadata: JobMetadata,
}
```

### 2.3 Service Layer

#### 2.3.1 Repository Manager

**Responsibilities**:
- Abstract repository access (local/remote)
- Cache repository metadata
- Track repository changes
- Index code structure

**Key Features**:
```rust
pub struct RepositoryManager {
    cache_dir: PathBuf,
    repositories: HashMap<RepoId, Repository>,
}

pub struct Repository {
    id: RepoId,
    url: Option<String>,
    local_path: PathBuf,
    language: ProgrammingLanguage,
    structure: CodeStructure,
}

pub struct CodeStructure {
    modules: Vec<Module>,
    functions: Vec<Function>,
    types: Vec<TypeDefinition>,
    configs: Vec<ConfigFile>,
}
```

**Capabilities**:
- Parse source code using `tree-sitter`
- Extract configuration from YAML/TOML/JSON
- Identify dependencies and relationships
- Detect documentation files

#### 2.3.2 AI Analysis Service

**Responsibilities**:
- Interface with Ollama API
- Manage model selection and fallback
- Context window management
- Prompt engineering for documentation tasks

**Model Strategy**:
```rust
pub struct AIAnalysisService {
    ollama_client: OllamaClient,
    model_config: ModelConfig,
    prompt_templates: PromptTemplateLibrary,
}

pub struct ModelConfig {
    primary_model: String,      // e.g., "codellama:13b"
    fallback_models: Vec<String>,
    context_window: usize,
    temperature: f32,
}
```

**Analysis Capabilities**:
- Code understanding and summarization
- Architecture pattern detection
- API endpoint extraction
- Configuration semantics
- Existing documentation quality assessment

**Prompt Templates** (Diátaxis-aligned):
- Reference generation: Extract function signatures, parameters, return types
- Tutorial generation: Identify common workflows and user journeys
- How-to generation: Extract procedural tasks from code patterns
- Explanation generation: Understand architectural decisions and trade-offs

#### 2.3.3 Documentation Generator

**Responsibilities**:
- Generate Markdown documentation
- Organize content according to Diátaxis framework
- Apply documentation templates
- Maintain cross-references

**Diátaxis Structure**:
```
pipeline-documentation/
├── tutorials/
│   ├── getting-started.md
│   └── service-name/
│       └── first-deployment.md
├── how-to-guides/
│   └── service-name/
│       ├── configure-auth.md
│       └── scale-horizontally.md
├── reference/
│   └── service-name/
│       ├── api.md
│       ├── configuration.md
│       └── cli.md
└── explanation/
    ├── architecture.md
    └── service-name/
        └── design-decisions.md
```

**Generator Modules**:
```rust
pub trait DocumentationGenerator {
    async fn generate_reference(&self, repo: &Repository) -> Result<Vec<Document>>;
    async fn generate_howto(&self, repo: &Repository) -> Result<Vec<Document>>;
    async fn generate_tutorial(&self, repo: &Repository) -> Result<Vec<Document>>;
    async fn generate_explanation(&self, repo: &Repository) -> Result<Vec<Document>>;
}

pub struct Document {
    category: DiátaxisCategory,
    title: String,
    content: String,
    metadata: DocumentMetadata,
}
```

#### 2.3.4 Change Detector

**Responsibilities**:
- Monitor git repositories for changes
- Determine if changes require documentation updates
- Prioritize documentation work

**Detection Strategy**:
```rust
pub struct ChangeDetector {
    watcher: Option<FileWatcher>,
    git_poller: Option<GitPoller>,
}

pub struct ChangeAnalysis {
    changed_files: Vec<PathBuf>,
    change_significance: Significance,
    affected_docs: Vec<DocumentPath>,
    recommended_actions: Vec<Action>,
}

pub enum Significance {
    Major,      // API changes, new features
    Minor,      // Bug fixes, config changes
    Patch,      // Typos, formatting
    None,       // Non-code changes
}
```

**Change Rules**:
- API signature changes → Update Reference + How-to
- New endpoints → Create Reference + How-to
- Configuration changes → Update Reference
- Major refactoring → Update Explanation
- New features → Create Tutorial + Reference

#### 2.3.5 Git Operations

**Responsibilities**:
- Clone repositories
- Create branches
- Commit changes
- Push to remote
- Manage authentication

**Implementation**:
```rust
pub struct GitOperations {
    credentials: CredentialStore,
}

impl GitOperations {
    pub async fn clone(&self, url: &str, dest: &Path) -> Result<()>;
    pub async fn create_branch(&self, repo: &Path, name: &str) -> Result<()>;
    pub async fn commit(&self, repo: &Path, message: &str) -> Result<Commit>;
    pub async fn push(&self, repo: &Path, branch: &str) -> Result<()>;
    pub async fn create_pr(&self, repo: &Repository, pr: PullRequest) -> Result<PrId>;
}
```

**Git Library**: `git2-rs` for libgit2 bindings

#### 2.3.6 PR Manager

**Responsibilities**:
- Create pull requests
- Format PR descriptions
- Link to source changes
- Tag reviewers (configurable)

**PR Template**:
```markdown
## Documentation Update

### Source Changes
- Repository: {source_repo}
- Commit Range: {from_commit}..{to_commit}
- Changed Files: {file_list}

### Documentation Changes
- **Reference**: Updated API documentation
- **How-to**: Added new configuration guide
- **Tutorial**: Updated getting started guide

### AI Analysis Summary
{ai_summary}

### Review Checklist
- [ ] Technical accuracy verified
- [ ] Code examples tested
- [ ] Links validated
- [ ] Diátaxis structure maintained

---
Generated by XZe Documentation Tool
```

### 2.4 Infrastructure Layer

#### 2.4.1 Ollama Client

**Responsibilities**:
- HTTP client for Ollama API
- Request/response serialization
- Connection pooling
- Error handling and retry logic

**Implementation**:
```rust
pub struct OllamaClient {
    base_url: String,
    http_client: reqwest::Client,
}

pub struct GenerateRequest {
    model: String,
    prompt: String,
    context: Vec<i32>,
    options: GenerateOptions,
}
```

**Error Handling**:
- Model not found → Fallback to alternative model
- Context too large → Chunk and summarize
- Connection failure → Retry with exponential backoff

#### 2.4.2 File System Abstraction

**Responsibilities**:
- Unified interface for file operations
- Support for virtual file systems (testing)
- Path normalization
- Atomic writes

```rust
pub trait FileSystem: Send + Sync {
    async fn read(&self, path: &Path) -> Result<Vec<u8>>;
    async fn write(&self, path: &Path, content: &[u8]) -> Result<()>;
    async fn exists(&self, path: &Path) -> bool;
    async fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
}
```

#### 2.4.3 Structured Logger

**Responsibilities**:
- JSON-formatted logging
- Contextual log enrichment
- Performance metrics
- Error tracking

**Log Schema**:
```json
{
  "timestamp": "2025-10-12T14:30:00Z",
  "level": "info",
  "message": "Documentation generated",
  "context": {
    "job_id": "uuid",
    "repository": "service-name",
    "category": "reference",
    "duration_ms": 1234
  }
}
```

**Implementation**: `tracing` crate with JSON subscriber

## 3. Data Models

### 3.1 Configuration Schema

**Repository Configuration** (`config.yaml`):
```yaml
version: "1.0"
documentation_repo:
  url: "https://github.com/org/pipeline-documentation"
  branch: "main"

repositories:
  - name: "auth-service"
    url: "https://github.com/org/auth-service"
    language: "rust"
    watch_branches: ["main", "develop"]
    
  - name: "api-gateway"
    url: "https://github.com/org/api-gateway"
    language: "go"
    watch_branches: ["main"]

ollama:
  url: "http://localhost:11434"
  models:
    primary: "codellama:13b"
    fallback: ["mistral:7b"]
  
generation:
  temperature: 0.3
  max_tokens: 4096
  
pr:
  auto_assign_reviewers: ["tech-lead", "docs-team"]
  labels: ["documentation", "automated"]
```

### 3.2 Internal Data Structures

```rust
// Repository metadata
pub struct RepositoryMetadata {
    pub name: String,
    pub language: ProgrammingLanguage,
    pub last_commit: String,
    pub last_analyzed: DateTime<Utc>,
    pub doc_coverage: f32,
}

// Documentation analysis
pub struct DocumentationAnalysis {
    pub exists: bool,
    pub up_to_date: bool,
    pub completeness_score: f32,
    pub missing_sections: Vec<DiátaxisCategory>,
    pub outdated_sections: Vec<String>,
}

// Job tracking
pub struct JobMetadata {
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration: Option<Duration>,
    pub status: JobStatus,
}
```

## 4. Workflow Algorithms

### 4.1 Local Mode Workflow

```
1. Parse command-line arguments
2. For each repository path:
   a. Load and analyze source code
   b. Load existing documentation from pipeline-documentation
   c. Analyze documentation quality and coverage
   d. Generate missing documentation
   e. Update outdated documentation
   f. Create git branch in pipeline-documentation
   g. Commit changes
   h. Create pull request
3. Display summary report
```

### 4.2 Auto Mode Workflow

```
1. Load configuration from YAML
2. Clone all repositories to cache directory
3. Initialize file watchers / git pollers
4. Main event loop:
   a. Detect changes in monitored repositories
   b. Analyze change significance
   c. If significant:
      i. Re-analyze affected repository
      ii. Generate/update documentation
      iii. Create branch and PR
   d. Wait for next event
```

### 4.3 Documentation Generation Algorithm

```
For each repository:
  1. Extract code structure (tree-sitter)
  2. Generate Reference documentation:
     - API endpoints → API reference
     - Configuration → Config reference
     - CLI commands → CLI reference
     
  3. Generate How-to guides:
     - Common patterns in code → How-to tasks
     - Configuration examples → Setup guides
     
  4. Generate Tutorials:
     - Main use cases → Step-by-step tutorials
     - Integration patterns → Integration guides
     
  5. Generate Explanations:
     - Architecture patterns → Design explanations
     - Trade-offs in code → Decision rationale
     
  6. Cross-reference all documents
  7. Validate generated markdown
  8. Write to pipeline-documentation structure
```

## 5. Technology Stack

### 5.1 Core Dependencies

| Component | Library | Purpose |
|-----------|---------|---------|
| Async Runtime | `tokio` | Async task execution |
| HTTP Client | `reqwest` | Ollama API communication |
| CLI Parsing | `clap` | Command-line interface |
| Web Framework | `axum` | REST API server |
| Git Operations | `git2` | Git integration |
| Code Parsing | `tree-sitter` | Source code parsing |
| Serialization | `serde`, `serde_json` | Data serialization |
| Logging | `tracing`, `tracing-subscriber` | Structured logging |
| Error Handling | `anyhow`, `thiserror` | Error management |
| Testing | `tokio-test`, `mockall` | Unit/integration testing |

### 5.2 Language-Specific Parsers

- Rust: `tree-sitter-rust`
- Python: `tree-sitter-python`
- Go: `tree-sitter-go`
- JavaScript/TypeScript: `tree-sitter-javascript`, `tree-sitter-typescript`
- Java: `tree-sitter-java`

## 6. Deployment Architecture

### 6.1 CLI Deployment

**Distribution**:
- GitHub Releases (binary artifacts)
- Cargo install (`cargo install xze`)
- Homebrew (macOS)
- APT/YUM repositories (Linux)

**System Requirements**:
- Ollama running locally or accessible via network
- Git installed
- Network access to git repositories

### 6.2 Server Deployment

#### Docker Deployment

**Dockerfile**:
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin xze-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y git ca-certificates
COPY --from=builder /app/target/release/xze-server /usr/local/bin/
EXPOSE 8080
CMD ["xze-server"]
```

**Docker Compose**:
```yaml
version: '3.8'
services:
  xze-server:
    build: .
    ports:
      - "8080:8080"
    environment:
      - OLLAMA_URL=http://ollama:11434
      - RUST_LOG=info
    volumes:
      - ./config.yaml:/etc/xze/config.yaml
      - xze-cache:/var/cache/xze
    depends_on:
      - ollama
      
  ollama:
    image: ollama/ollama:latest
    ports:
      - "11434:11434"
    volumes:
      - ollama-models:/root/.ollama

volumes:
  xze-cache:
  ollama-models:
```

#### Kubernetes Deployment

**Key Resources**:
- Deployment: xze-server (horizontal scaling)
- Service: LoadBalancer for API access
- ConfigMap: Configuration file
- Secret: Git credentials
- PersistentVolumeClaim: Cache storage

**Scaling Considerations**:
- Stateless design allows horizontal scaling
- Job queue (Redis) for work distribution
- Shared cache volume or S3-compatible storage

## 7. Security Considerations

### 7.1 Authentication & Authorization

- Git credentials stored securely (Kubernetes secrets, environment variables)
- API authentication via JWT tokens (server mode)
- Role-based access control for PR creation

### 7.2 Code Execution Safety

- Sandboxed code parsing (tree-sitter is safe)
- No arbitrary code execution
- Input validation on all user-provided paths

### 7.3 Secrets Management

- Never log sensitive data (tokens, credentials)
- Scrub secrets from AI prompts
- Support for external secret managers (Vault, AWS Secrets Manager)

## 8. Monitoring & Observability

### 8.1 Metrics

- Jobs processed per hour
- Documentation generation latency
- Ollama API response times
- Cache hit rates
- PR creation success rates

### 8.2 Logging

All operations logged as JSON with fields:
- `job_id`: Unique identifier
- `repository`: Repository name
- `operation`: Type of operation
- `duration_ms`: Operation duration
- `status`: Success/failure
- `error`: Error details if failed

### 8.3 Health Checks

- `/health` endpoint (server mode)
- Ollama connectivity check
- Git repository accessibility
- Disk space availability

## 9. Testing Strategy

### 9.1 Unit Tests

- Mock file system for repository operations
- Mock Ollama client responses
- Test documentation generation logic
- Test change detection algorithms

### 9.2 Integration Tests

- Test full pipeline with sample repositories
- Verify PR creation end-to-end
- Test different repository languages
- Validate Diátaxis structure compliance

### 9.3 End-to-End Tests

- Deploy in test environment
- Trigger analysis on real repositories
- Verify documentation quality manually
- Performance benchmarking

## 10. Future Enhancements

### 10.1 Phase 2 Features

- Documentation versioning and diff viewer
- Interactive documentation review UI
- Multi-language documentation generation
- Custom documentation templates
- Diagram generation (architecture, sequence)

### 10.2 Advanced AI Features

- Self-improving prompts based on feedback
- Fine-tuned models for specific domains
- Code example validation and testing
- Automated documentation testing

### 10.3 Enterprise Features

- SSO integration
- Audit logging
- Compliance reporting
- SLA tracking for documentation freshness

## 11. Project Structure

```
xze/
├── Cargo.toml
├── crates/
│   ├── xze-core/              # Core business logic
│   │   ├── src/
│   │   │   ├── pipeline/
│   │   │   ├── repository/
│   │   │   ├── ai/
│   │   │   └── documentation/
│   │   └── Cargo.toml
│   ├── xze-cli/               # CLI interface
│   ├── xze-server/            # Server interface
│   ├── xze-vscode/            # VSCode extension (Rust side)
│   └── xze-infra/             # Infrastructure layer
├── vscode-extension/          # VSCode extension (TypeScript)
├── docs/                      # Project documentation
├── examples/                  # Example configurations
├── tests/                     # Integration tests
└── docker/                    # Docker files
```

## Conclusion

This architectural design provides a solid foundation for building XZe as a robust, scalable, and maintainable documentation automation tool. The modular design allows for incremental development and testing of components while maintaining clear separation of concerns.

The system's async-first approach enables efficient concurrent processing of multiple repositories, while the stateless design facilitates horizontal scaling in server mode. The use of established Rust libraries and frameworks ensures reliability and community support.

Key success factors include:
- Well-defined component boundaries for parallel development
- Comprehensive testing strategy at multiple levels
- Observable operations through structured logging
- Flexible deployment options (CLI, Server, VSCode)
- Security-conscious design with credential management
- Extensible architecture for future enhancements