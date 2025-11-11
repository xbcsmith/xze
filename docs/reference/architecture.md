# XZe Architecture Reference

## 1. System Architecture Overview

### 1.1 Introduction

XZe is an AI-powered documentation generation tool written in Rust that uses open source models from Ollama to analyze source code repositories and automatically generate comprehensive documentation following the Diátaxis framework. The system is designed with an **API-first, event-driven architecture** that enables scalable, automated documentation workflows.

### 1.2 High-Level Architecture

XZe follows a layered, modular architecture with the REST API as the primary interface. The system is built in Rust for performance, safety, and excellent async capabilities.

```
┌─────────────────────────────────────────────────────────────┐
│                     Interface Layer                          │
│  ┌──────────┐  ┌──────────┐  ┌────────────────────┐        │
│  │   CLI    │  │   SDK    │  │  External Systems  │        │
│  │ (Client) │  │ (Dual)   │  │  (Webhooks/Kafka)  │        │
│  └────┬─────┘  └────┬─────┘  └─────────┬──────────┘        │
│       │             │                   │                    │
│       └─────────────┴───────────────────┘                    │
│                           ↓                                  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              REST API (xze-serve)                     │  │
│  │  - Versioned Endpoints (/api/v1/*)                    │  │
│  │  - OpenAPI Documentation                              │  │
│  │  - Webhook Receivers                                  │  │
│  │  - Event Consumers (Kafka)                            │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                    Orchestration Layer                       │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         Documentation Pipeline Controller             │  │
│  │  - Event Processing                                   │  │
│  │  - Job Orchestration                                  │  │
│  │  - Workflow Coordination                              │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                     Service Layer (xze-core)                 │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Repository   │  │ AI Analysis  │  │  Documentation  │  │
│  │ Manager      │  │ Service      │  │  Generator      │  │
│  └──────────────┘  └──────────────┘  └─────────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Git          │  │ Change       │  │  PR Manager     │  │
│  │ Operations   │  │ Analyzer     │  │                 │  │
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

### 1.3 Core Design Principles

**API-First Architecture**

- REST API is the primary interface for all operations
- All functionality exposed through versioned HTTP endpoints
- CLI and SDK are clients of the API
- Enables integration with any HTTP-capable system

**Event-Driven Processing**

- Repository changes trigger documentation updates via events
- Supports multiple event sources: webhooks, message queues
- Asynchronous, non-blocking event processing
- Scalable job queue for concurrent analysis

**Modularity**

- Each component is independently testable and replaceable
- Clear crate boundaries with strict dependency rules
- Service layer (xze-core) has no interface dependencies
- Infrastructure components are abstracted behind traits

**Async-First**

- Leverage Tokio for concurrent repository analysis
- Non-blocking I/O for all network operations
- Efficient resource utilization for multi-repository processing

**Configuration-Driven**

- YAML-based configuration for repositories and models
- Environment variables for deployment-specific settings
- Runtime configuration via API endpoints

**Observable**

- Structured JSON logging for all operations
- Health check endpoints for monitoring
- Metrics export for observability platforms

## 2. Component Design

### 2.1 API Layer (xze-serve)

**Crate**: `xze-serve`

**Responsibilities**:

- Primary interface for all XZe operations
- REST API endpoint exposure with OpenAPI documentation
- Webhook receivers for GitHub/GitLab events
- Kafka event consumer for repository changes
- Authentication and authorization
- Request validation and rate limiting
- Job queue management

**Technology Stack**:

- Framework: `axum` (async web framework)
- OpenAPI: `utoipa` (compile-time OpenAPI generation)
- Serialization: `serde`, `serde_json`
- Event streaming: Kafka client for Redpanda

**API Endpoints** (Version: v1):

```
Authentication & Health:
  GET    /api/v1/health              - Health check
  GET    /api/v1/openapi.json        - OpenAPI specification
  GET    /api/v1/version             - Service version

Job Management:
  POST   /api/v1/jobs                - Create analysis job
  GET    /api/v1/jobs                - List jobs
  GET    /api/v1/jobs/:id            - Get job status
  DELETE /api/v1/jobs/:id            - Cancel job

Repository Management:
  GET    /api/v1/repositories        - List tracked repositories
  POST   /api/v1/repositories        - Add repository to tracking
  GET    /api/v1/repositories/:id    - Get repository details
  DELETE /api/v1/repositories/:id    - Remove repository
  PUT    /api/v1/repositories/:id    - Update repository config

Documentation:
  GET    /api/v1/docs/:repo_id       - Get generated documentation
  GET    /api/v1/docs/:repo_id/status - Documentation status

Event Receivers:
  POST   /api/v1/webhooks/github     - GitHub webhook receiver
  POST   /api/v1/webhooks/gitlab     - GitLab webhook receiver
```

**OpenAPI Generation**:

```rust
use utoipa::{OpenApi, ToSchema};
use axum::{Router, Json};

#[derive(OpenApi)]
#[openapi(
    paths(
        create_job,
        get_job_status,
        list_repositories,
    ),
    components(
        schemas(JobRequest, JobResponse, Repository)
    ),
    tags(
        (name = "xze", description = "XZe Documentation API")
    )
)]
pub struct ApiDoc;

// Serve OpenAPI spec at /api/v1/openapi.json
pub fn openapi_route() -> Router {
    Router::new()
        .route("/api/v1/openapi.json", get(|| async {
            Json(ApiDoc::openapi())
        }))
}
```

**Webhook Handler Structure**:

```rust
pub struct WebhookHandler {
    job_queue: Arc<JobQueue>,
    signature_validator: SignatureValidator,
}

impl WebhookHandler {
    pub async fn handle_github_webhook(
        &self,
        payload: GitHubWebhookPayload,
        signature: String,
    ) -> Result<JobId, WebhookError> {
        // Validate signature
        self.signature_validator.verify_github(&payload, &signature)?;

        // Extract repository and change info
        let repo_info = self.extract_repo_info(&payload)?;

        // Queue analysis job
        let job_id = self.job_queue.enqueue(repo_info).await?;

        Ok(job_id)
    }
}
```

**Kafka Event Consumer**:

```rust
pub struct KafkaEventConsumer {
    consumer: StreamConsumer,
    job_queue: Arc<JobQueue>,
}

impl KafkaEventConsumer {
    pub async fn consume_repository_events(&self) -> Result<()> {
        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    let event: RepositoryChangeEvent =
                        serde_json::from_slice(message.payload())?;

                    self.job_queue.enqueue(event.into()).await?;
                    self.consumer.commit_message(&message, CommitMode::Async)?;
                }
                Err(e) => {
                    error!("Kafka consumer error: {}", e);
                }
            }
        }
    }
}
```

### 2.2 SDK Layer (xze-sdk)

**Crate**: `xze-sdk`

**Responsibilities**:

- Provide Rust library interface to XZe functionality
- Dual API support: Client API (HTTP) and Direct API (embedded)
- Enable programmatic access for integrations
- Support for ad-hoc analysis without running server

**Dual Interface Design**:

```rust
pub mod client {
    //! HTTP client for xze-serve REST API

    pub struct XzeClient {
        base_url: String,
        http_client: reqwest::Client,
        api_key: Option<String>,
    }

    impl XzeClient {
        pub fn new(base_url: impl Into<String>) -> Self {
            Self {
                base_url: base_url.into(),
                http_client: reqwest::Client::new(),
                api_key: None,
            }
        }

        pub async fn create_analysis_job(
            &self,
            request: AnalysisRequest,
        ) -> Result<JobId, ClientError> {
            let response = self.http_client
                .post(&format!("{}/api/v1/jobs", self.base_url))
                .json(&request)
                .send()
                .await?;

            Ok(response.json().await?)
        }

        pub async fn get_job_status(
            &self,
            job_id: JobId,
        ) -> Result<JobStatus, ClientError> {
            // HTTP GET to /api/v1/jobs/:id
        }
    }
}

pub mod direct {
    //! Direct access to xze-core for ad-hoc analysis
    //! Enables standalone usage without xze-serve running

    use xze_core::pipeline::PipelineController;

    pub struct XzeDirect {
        controller: PipelineController,
    }

    impl XzeDirect {
        pub async fn new(config: DirectConfig) -> Result<Self, DirectError> {
            let controller = PipelineController::new(config.into()).await?;
            Ok(Self { controller })
        }

        pub async fn analyze_repository(
            &self,
            repo_path: &Path,
        ) -> Result<AnalysisResult, DirectError> {
            self.controller.analyze_local_repository(repo_path).await
        }

        pub async fn generate_documentation(
            &self,
            repo: &Repository,
            category: DiátaxisCategory,
        ) -> Result<Document, DirectError> {
            self.controller.generate_documentation(repo, category).await
        }
    }
}
```

**Usage Examples**:

```rust
// Using HTTP Client API
use xze_sdk::client::XzeClient;

let client = XzeClient::new("http://localhost:8080");
let job_id = client.create_analysis_job(request).await?;
let status = client.get_job_status(job_id).await?;

// Using Direct API (ad-hoc, no server needed)
use xze_sdk::direct::{XzeDirect, DirectConfig};

let xze = XzeDirect::new(DirectConfig {
    ollama_url: "http://localhost:11434".into(),
    ..Default::default()
}).await?;

let result = xze.analyze_repository(Path::new("/path/to/repo")).await?;
```

### 2.3 CLI Layer (xze-cli)

**Crate**: `xze-cli`

**Responsibilities**:

- Command-line interface for XZe operations
- Pure API client (uses xze-sdk client module)
- User-friendly command and output formatting
- Configuration file management

**Architecture**:

- CLI does NOT perform analysis directly
- CLI does NOT have local/standalone mode
- All operations call xze-serve REST API via xze-sdk client

**Key Structure**:

```rust
use xze_sdk::client::XzeClient;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xze")]
#[command(about = "AI-powered documentation generation")]
pub struct Cli {
    #[arg(long, env = "XZE_API_URL", default_value = "http://localhost:8080")]
    api_url: String,

    #[arg(long, env = "XZE_API_KEY")]
    api_key: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Trigger documentation analysis job
    Analyze {
        /// Repository URL or ID
        repository: String,
    },

    /// Check job status
    Status {
        /// Job ID
        job_id: String,
    },

    /// List tracked repositories
    Repos {
        #[arg(long)]
        format: Option<OutputFormat>,
    },

    /// Add repository to tracking
    Add {
        /// Repository URL
        url: String,

        #[arg(long)]
        language: Option<String>,
    },
}
```

**Commands**:

```bash
# Trigger analysis via API
xze analyze https://github.com/org/repo

# Check job status
xze status job-id-12345

# List tracked repositories
xze repos --format json

# Add repository to tracking
xze add https://github.com/org/new-repo --language rust

# Health check
xze health
```

**Note**: For ad-hoc local analysis without running xze-serve, users should use the xze-sdk direct API in their own Rust programs.

### 2.4 Orchestration Layer

**Location**: `xze-core` crate

#### 2.4.1 Documentation Pipeline Controller

**Responsibilities**:

- Coordinate end-to-end documentation workflow
- Manage state transitions for analysis jobs
- Handle error recovery and retry logic
- Dispatch work to service layer components

**Workflow State Machine**:

```
┌──────────────┐
│   Queued     │
└──────┬───────┘
       │
       ↓
┌──────────────┐
│   Cloning    │
└──────┬───────┘
       │
       ↓
┌──────────────┐     ┌──────────────┐
│  Analyzing   │────>│  Generating  │
└──────────────┘     └──────┬───────┘
                            │
                            ↓
                     ┌──────────────┐
                     │  Committing  │
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
                            │
                            ↓ (on error)
                     ┌──────────────┐
                     │    Failed    │
                     └──────────────┘
```

**Key Structures**:

```rust
pub struct PipelineController {
    repo_manager: Arc<RepositoryManager>,
    ai_service: Arc<AIAnalysisService>,
    doc_generator: Arc<DocumentationGenerator>,
    git_ops: Arc<GitOperations>,
    pr_manager: Arc<PrManager>,
    logger: Arc<StructuredLogger>,
}

pub struct PipelineJob {
    pub id: Ulid,
    pub source_repo: Repository,
    pub target_repo: Repository,
    pub state: JobState,
    pub metadata: JobMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum JobState {
    Queued,
    Cloning,
    Analyzing,
    Generating,
    Committing,
    CreatingPr,
    Completed { pr_url: String },
    Failed { error: String },
}
```

**Implementation**:

```rust
impl PipelineController {
    pub async fn process_job(&self, job: PipelineJob) -> Result<PipelineJob> {
        let mut job = job;

        // Clone repository
        job.state = JobState::Cloning;
        let repo = self.repo_manager.clone_or_update(&job.source_repo).await?;

        // Analyze code structure
        job.state = JobState::Analyzing;
        let analysis = self.ai_service.analyze_repository(&repo).await?;

        // Generate documentation
        job.state = JobState::Generating;
        let docs = self.doc_generator.generate_all(&repo, &analysis).await?;

        // Commit to documentation repo
        job.state = JobState::Committing;
        let branch = self.git_ops.create_doc_branch(&job.target_repo).await?;
        self.git_ops.commit_documents(&branch, &docs).await?;

        // Create pull request
        job.state = JobState::CreatingPr;
        let pr_url = self.pr_manager.create_pr(&branch, &docs).await?;

        job.state = JobState::Completed { pr_url };
        Ok(job)
    }
}
```

### 2.5 Service Layer (xze-core)

**Crate**: `xze-core`

**Design Principle**: xze-core contains domain logic ONLY. No dependencies on interface layers (xze-serve, xze-cli, xze-sdk). This enables reusability and testing.

#### 2.5.1 Repository Manager

**Module**: `xze_core::repository`

**Responsibilities**:

- Abstract repository access (local/remote)
- Cache repository metadata
- Track repository state
- Index code structure using tree-sitter

**Key Structures**:

```rust
pub struct RepositoryManager {
    cache_dir: PathBuf,
    repositories: Arc<RwLock<HashMap<RepoId, Repository>>>,
    fs: Arc<dyn FileSystem>,
}

pub struct Repository {
    pub id: RepoId,
    pub url: Option<String>,
    pub local_path: PathBuf,
    pub language: ProgrammingLanguage,
    pub structure: CodeStructure,
    pub metadata: RepositoryMetadata,
}

pub struct CodeStructure {
    pub modules: Vec<Module>,
    pub functions: Vec<Function>,
    pub types: Vec<TypeDefinition>,
    pub configs: Vec<ConfigFile>,
}

pub struct RepositoryMetadata {
    pub name: String,
    pub language: ProgrammingLanguage,
    pub last_commit: String,
    pub last_analyzed: DateTime<Utc>,
    pub doc_coverage: f32,
}
```

**Capabilities**:

- Parse source code using tree-sitter
- Extract configuration from YAML/TOML/JSON files
- Identify dependencies and relationships
- Detect existing documentation files
- Calculate documentation coverage metrics

#### 2.5.2 AI Analysis Service

**Module**: `xze_core::ai`

**Responsibilities**:

- Interface with Ollama API
- Manage model selection and fallback
- Context window management
- Prompt engineering for documentation tasks

**Key Structures**:

```rust
pub struct AIAnalysisService {
    ollama_client: Arc<OllamaClient>,
    model_config: ModelConfig,
    prompt_templates: PromptTemplateLibrary,
}

pub struct ModelConfig {
    pub primary_model: String,      // e.g., "codellama:13b"
    pub fallback_models: Vec<String>,
    pub context_window: usize,
    pub temperature: f32,
}

pub struct AnalysisResult {
    pub summary: String,
    pub architecture_patterns: Vec<Pattern>,
    pub api_endpoints: Vec<ApiEndpoint>,
    pub configurations: Vec<ConfigEntry>,
    pub quality_assessment: QualityScore,
}
```

**Analysis Capabilities**:

- Code understanding and summarization
- Architecture pattern detection
- API endpoint extraction from code
- Configuration semantics analysis
- Existing documentation quality assessment

**Prompt Templates** (Diátaxis-aligned):

```rust
pub struct PromptTemplateLibrary {
    reference_generation: Template,
    tutorial_generation: Template,
    howto_generation: Template,
    explanation_generation: Template,
}

// Example prompt for reference generation
const REFERENCE_PROMPT: &str = r#"
Analyze the following source code and generate technical reference documentation.
Focus on:
- Function signatures, parameters, and return types
- API endpoints and their specifications
- Configuration options and defaults
- Data structures and their fields

Code:
{code}

Generate markdown documentation following this structure:
## Function: {name}
Description: {description}
Parameters: {params}
Returns: {return_type}
Example: {example}
"#;
```

#### 2.5.3 Documentation Generator

**Module**: `xze_core::documentation`

**Responsibilities**:

- Generate Markdown documentation from analysis results
- Organize content according to Diátaxis framework
- Apply documentation templates
- Maintain cross-references between documents

**Diátaxis Structure**:

```
pipeline-documentation/
├── tutorials/
│   ├── quickstart.md
│   └── service-name/
│       └── first-deployment.md
├── how_to/
│   └── service-name/
│       ├── configure_auth.md
│       └── scale_horizontally.md
├── reference/
│   ├── architecture.md
│   └── service-name/
│       ├── api.md
│       ├── configuration.md
│       └── cli.md
└── explanation/
    ├── implementations.md
    └── service-name/
        └── design_decisions.md
```

**Key Structures**:

```rust
pub trait DocumentationGenerator: Send + Sync {
    async fn generate_reference(
        &self,
        repo: &Repository,
        analysis: &AnalysisResult,
    ) -> Result<Vec<Document>>;

    async fn generate_howto(
        &self,
        repo: &Repository,
        analysis: &AnalysisResult,
    ) -> Result<Vec<Document>>;

    async fn generate_tutorial(
        &self,
        repo: &Repository,
        analysis: &AnalysisResult,
    ) -> Result<Vec<Document>>;

    async fn generate_explanation(
        &self,
        repo: &Repository,
        analysis: &AnalysisResult,
    ) -> Result<Vec<Document>>;
}

pub struct Document {
    pub category: DiátaxisCategory,
    pub title: String,
    pub content: String,
    pub metadata: DocumentMetadata,
    pub path: PathBuf,
}

pub enum DiátaxisCategory {
    Tutorial,
    HowTo,
    Reference,
    Explanation,
}
```

#### 2.5.4 Change Analyzer

**Module**: `xze_core::changes`

**Responsibilities**:

- Analyze git commit changes
- Determine if changes require documentation updates
- Prioritize documentation work based on change significance
- Identify affected documentation sections

**Note**: This replaces the old `ChangeDetector` with file watching. The new `ChangeAnalyzer` receives change information from events (webhooks/Kafka) and analyzes their impact.

**Key Structures**:

```rust
pub struct ChangeAnalyzer {
    config: AnalyzerConfig,
}

pub struct ChangeAnalysis {
    pub changed_files: Vec<PathBuf>,
    pub change_significance: Significance,
    pub affected_docs: Vec<DocumentPath>,
    pub recommended_actions: Vec<Action>,
}

pub enum Significance {
    Major,      // API changes, new features
    Minor,      // Bug fixes, config changes
    Patch,      // Typos, formatting
    None,       // Non-code changes (CI, docs)
}

pub enum Action {
    CreateReference { topic: String },
    UpdateReference { path: PathBuf },
    CreateHowTo { topic: String },
    UpdateHowTo { path: PathBuf },
    CreateTutorial { topic: String },
    UpdateExplanation { path: PathBuf },
}
```

**Change Analysis Rules**:

```rust
impl ChangeAnalyzer {
    pub async fn analyze_commit(
        &self,
        commit: &Commit,
    ) -> Result<ChangeAnalysis> {
        let changed_files = self.get_changed_files(commit).await?;
        let significance = self.determine_significance(&changed_files);
        let affected_docs = self.find_affected_docs(&changed_files).await?;
        let actions = self.recommend_actions(&changed_files, &significance);

        Ok(ChangeAnalysis {
            changed_files,
            change_significance: significance,
            affected_docs,
            recommended_actions: actions,
        })
    }

    fn determine_significance(&self, files: &[PathBuf]) -> Significance {
        // API signature changes → Major
        // New endpoints → Major
        // Configuration changes → Minor
        // Bug fixes → Minor
        // Documentation/comments → Patch
        // CI/build files → None
    }
}
```

#### 2.5.5 Git Operations

**Module**: `xze_core::git`

**Responsibilities**:

- Clone repositories
- Create branches
- Commit changes
- Push to remote
- Manage authentication credentials

**Key Structures**:

```rust
pub struct GitOperations {
    credentials: Arc<CredentialStore>,
}

impl GitOperations {
    pub async fn clone(
        &self,
        url: &str,
        dest: &Path,
    ) -> Result<Repository, GitError> {
        // Clone using git2
    }

    pub async fn create_branch(
        &self,
        repo: &Path,
        name: &str,
    ) -> Result<Branch, GitError> {
        // Create branch from main/master
    }

    pub async fn commit(
        &self,
        repo: &Path,
        message: &str,
        files: &[PathBuf],
    ) -> Result<Commit, GitError> {
        // Stage and commit files
    }

    pub async fn push(
        &self,
        repo: &Path,
        branch: &str,
    ) -> Result<(), GitError> {
        // Push to remote
    }
}
```

**Git Library**: `git2` (libgit2 Rust bindings)

#### 2.5.6 PR Manager

**Module**: `xze_core::pr`

**Responsibilities**:

- Create pull requests via GitHub/GitLab APIs
- Format PR descriptions with analysis summary
- Link to source changes
- Tag reviewers based on configuration

**Key Structures**:

```rust
pub struct PrManager {
    github_client: Option<GitHubClient>,
    gitlab_client: Option<GitLabClient>,
}

pub struct PullRequest {
    pub title: String,
    pub body: String,
    pub base_branch: String,
    pub head_branch: String,
    pub labels: Vec<String>,
    pub reviewers: Vec<String>,
}
```

**PR Template**:

```markdown
## Documentation Update

### Source Changes

- **Repository**: {source_repo}
- **Commit Range**: {from_commit}..{to_commit}
- **Changed Files**: {file_list}

### Documentation Changes

- **Reference**: {reference_changes}
- **How-To**: {howto_changes}
- **Tutorial**: {tutorial_changes}
- **Explanation**: {explanation_changes}

### AI Analysis Summary

{ai_summary}

### Review Checklist

- [ ] Technical accuracy verified
- [ ] Code examples tested
- [ ] Links validated
- [ ] Diátaxis structure maintained
- [ ] Formatting follows style guide

---

Generated by XZe Documentation Tool v{version}
Job ID: {job_id}
```

### 2.6 Infrastructure Layer

**Location**: `xze-core` crate

#### 2.6.1 Ollama Client

**Module**: `xze_core::infrastructure::ollama`

**Responsibilities**:

- HTTP client for Ollama API
- Request/response serialization
- Connection pooling
- Error handling and retry logic

**Key Structures**:

```rust
pub struct OllamaClient {
    base_url: String,
    http_client: reqwest::Client,
}

pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    pub context: Vec<i32>,
    pub options: GenerateOptions,
}

pub struct GenerateResponse {
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub response: String,
    pub done: bool,
    pub context: Vec<i32>,
}
```

**Error Handling Strategy**:

```rust
impl OllamaClient {
    pub async fn generate(
        &self,
        request: GenerateRequest,
    ) -> Result<GenerateResponse, OllamaError> {
        // Model not found → Fallback to alternative model
        // Context too large → Chunk and summarize
        // Connection failure → Retry with exponential backoff
    }
}
```

#### 2.6.2 File System Abstraction

**Module**: `xze_core::infrastructure::fs`

**Responsibilities**:

- Unified interface for file operations
- Support for virtual file systems (testing)
- Path normalization
- Atomic writes

**Key Trait**:

```rust
pub trait FileSystem: Send + Sync {
    async fn read(&self, path: &Path) -> Result<Vec<u8>, FsError>;
    async fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError>;
    async fn exists(&self, path: &Path) -> bool;
    async fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>, FsError>;
    async fn create_dir_all(&self, path: &Path) -> Result<(), FsError>;
    async fn remove_file(&self, path: &Path) -> Result<(), FsError>;
}

// Real implementation
pub struct StdFileSystem;

// Mock implementation for testing
pub struct MockFileSystem {
    files: Arc<RwLock<HashMap<PathBuf, Vec<u8>>>>,
}
```

#### 2.6.3 Structured Logger

**Module**: `xze_core::infrastructure::logging`

**Responsibilities**:

- JSON-formatted logging
- Contextual log enrichment
- Performance metrics
- Error tracking

**Log Schema**:

```json
{
  "timestamp": "2025-01-07T18:12:07.982682Z",
  "level": "info",
  "message": "Documentation generated",
  "context": {
    "job_id": "01JG8X9YZ5N2QWE8R7TPMKFHVB",
    "repository": "auth-service",
    "category": "reference",
    "duration_ms": 1234,
    "model": "codellama:13b"
  }
}
```

**Implementation**: `tracing` crate with JSON subscriber

```rust
use tracing::{info, error, warn};
use tracing_subscriber::fmt::format::JsonFields;

pub fn init_logger() {
    tracing_subscriber::fmt()
        .json()
        .with_current_span(false)
        .with_target(false)
        .init();
}

// Usage in code
info!(
    job_id = %job.id,
    repository = %job.source_repo.name,
    duration_ms = elapsed.as_millis(),
    "Documentation generation completed"
);
```

## 3. Data Models

### 3.1 Configuration Schema

**Application Configuration** (`xze.yaml`):

```yaml
version: "1.0"

# Documentation repository where generated docs are committed
documentation_repo:
  url: "https://github.com/org/pipeline-documentation"
  branch: "main"

# Repositories to track for documentation generation
repositories:
  - name: "auth-service"
    url: "https://github.com/org/auth-service"
    language: "rust"
    enabled: true

  - name: "api-gateway"
    url: "https://github.com/org/api-gateway"
    language: "go"
    enabled: true

# Ollama configuration
ollama:
  url: "http://localhost:11434"
  models:
    primary: "codellama:13b"
    fallback: ["mistral:7b", "llama2:7b"]

# AI generation settings
generation:
  temperature: 0.3
  max_tokens: 4096
  timeout_seconds: 300

# Pull request configuration
pr:
  auto_assign_reviewers: ["tech-lead", "docs-team"]
  labels: ["documentation", "automated"]
  template_path: "./templates/pr_template.md"

# Event sources
events:
  webhooks:
    enabled: true
    github:
      enabled: true
      secret: "${GITHUB_WEBHOOK_SECRET}"
    gitlab:
      enabled: true
      secret: "${GITLAB_WEBHOOK_SECRET}"

  kafka:
    enabled: true
    brokers: ["localhost:9092"]
    topics:
      - "repository-changes"
      - "deployment-events"
    consumer_group: "xze-documentation"

# API server configuration
server:
  host: "0.0.0.0"
  port: 8080
  max_connections: 100
  request_timeout_seconds: 30
```

**Rust Configuration Struct**:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub version: String,
    pub documentation_repo: DocumentationRepo,
    pub repositories: Vec<RepositoryConfig>,
    pub ollama: OllamaConfig,
    pub generation: GenerationConfig,
    pub pr: PrConfig,
    pub events: EventConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DocumentationRepo {
    pub url: String,
    pub branch: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepositoryConfig {
    pub name: String,
    pub url: String,
    pub language: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventConfig {
    pub webhooks: WebhookConfig,
    pub kafka: KafkaConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KafkaConfig {
    pub enabled: bool,
    pub brokers: Vec<String>,
    pub topics: Vec<String>,
    pub consumer_group: String,
}
```

### 3.2 API Request/Response Models

**Job Creation**:

```rust
use utoipa::ToSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateJobRequest {
    /// Repository URL or ID to analyze
    pub repository: String,

    /// Optional: specific commit SHA to analyze
    pub commit: Option<String>,

    /// Optional: specific documentation categories to generate
    pub categories: Option<Vec<DiátaxisCategory>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JobResponse {
    /// Unique job identifier (ULID)
    pub id: String,

    /// Current job state
    pub state: String,

    /// Repository being analyzed
    pub repository: String,

    /// Job creation timestamp (RFC-3339)
    pub created_at: String,

    /// Job last update timestamp (RFC-3339)
    pub updated_at: String,

    /// Error message if failed
    pub error: Option<String>,

    /// Pull request URL if completed
    pub pr_url: Option<String>,
}
```

**Repository Management**:

```rust
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RepositoryResponse {
    pub id: String,
    pub name: String,
    pub url: String,
    pub language: String,
    pub enabled: bool,
    pub last_analyzed: Option<String>,
    pub doc_coverage: f32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddRepositoryRequest {
    pub url: String,
    pub language: Option<String>,
    pub enabled: bool,
}
```

**Webhook Payloads**:

```rust
#[derive(Debug, Deserialize)]
pub struct GitHubWebhookPayload {
    pub action: String,
    pub repository: GitHubRepository,
    pub commits: Vec<GitHubCommit>,
    pub pusher: GitHubUser,
}

#[derive(Debug, Deserialize)]
pub struct GitLabWebhookPayload {
    pub object_kind: String,
    pub project: GitLabProject,
    pub commits: Vec<GitLabCommit>,
    pub user: GitLabUser,
}
```

### 3.3 Internal Data Structures

```rust
pub struct DocumentationAnalysis {
    pub exists: bool,
    pub up_to_date: bool,
    pub completeness_score: f32,
    pub missing_sections: Vec<DiátaxisCategory>,
    pub outdated_sections: Vec<String>,
}

pub struct JobMetadata {
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration: Option<Duration>,
    pub status: JobStatus,
}

pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

## 4. Event-Driven Architecture

### 4.1 Event Sources

XZe supports two primary event sources for triggering documentation generation:

**1. Webhooks** (GitHub/GitLab)

- Push model: git hosting service sends HTTP POST on repository events
- Real-time notification of code changes
- Requires XZe server to be publicly accessible or use webhook relay

**2. Kafka Message Queue** (Redpanda)

- Pull model: XZe consumes messages from Kafka topics
- Decoupled architecture with message buffering
- Better for high-volume or complex event routing

### 4.2 Event Processing Flow

```
┌─────────────────┐
│  GitHub/GitLab  │
│    Webhook      │
└────────┬────────┘
         │
         ↓ HTTP POST
┌─────────────────────────────────────┐
│  xze-serve                          │
│  POST /api/v1/webhooks/github       │
│  - Validate signature               │
│  - Extract repository info          │
│  - Queue job                        │
└────────┬────────────────────────────┘
         │
         ↓
┌────────────────────────────────────┐
│  Job Queue (in-memory/Redis)       │
│  - Job ID: ULID                    │
│  - Repository: auth-service        │
│  - Commit: abc123                  │
└────────┬───────────────────────────┘
         │
         ↓
┌────────────────────────────────────┐
│  Pipeline Controller               │
│  - Process job                     │
│  - Analyze → Generate → PR         │
└────────┬───────────────────────────┘
         │
         ↓
┌────────────────────────────────────┐
│  Completed                         │
│  - PR created in docs repo         │
│  - Job status updated              │
└────────────────────────────────────┘
```

**Alternative: Kafka Flow**

```
┌─────────────────┐
│  CI/CD System   │
│  Custom Service │
└────────┬────────┘
         │
         ↓ Publish Message
┌─────────────────────────────────────┐
│  Redpanda Kafka                     │
│  Topic: repository-changes          │
│  Message: { repo, commit, ... }     │
└────────┬────────────────────────────┘
         │
         ↓ Consumer Poll
┌─────────────────────────────────────┐
│  xze-serve (Kafka Consumer)         │
│  - Deserialize event                │
│  - Queue job                        │
└────────┬────────────────────────────┘
         │
         ↓
    (same as webhook flow)
```

### 4.3 Event Message Schema

**Kafka Message Format**:

```json
{
  "event_id": "01JG8X9YZ5N2QWE8R7TPMKFHVB",
  "timestamp": "2025-01-07T18:12:07.982682Z",
  "event_type": "repository.push",
  "repository": {
    "name": "auth-service",
    "url": "https://github.com/org/auth-service",
    "language": "rust"
  },
  "commit": {
    "sha": "abc123def456",
    "author": "developer@example.com",
    "message": "Add new authentication endpoint",
    "files_changed": ["src/auth/mod.rs", "src/api/endpoints.rs"]
  },
  "metadata": {
    "branch": "main",
    "triggeredBy": "ci-pipeline"
  }
}
```

**Rust Event Structure**:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryChangeEvent {
    pub event_id: Ulid,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub repository: EventRepository,
    pub commit: EventCommit,
    pub metadata: HashMap<String, String>,
}

impl From<RepositoryChangeEvent> for PipelineJob {
    fn from(event: RepositoryChangeEvent) -> Self {
        PipelineJob {
            id: Ulid::new(),
            source_repo: event.repository.into(),
            // ... map other fields
        }
    }
}
```

### 4.4 Event Handler Implementation

```rust
pub struct EventHandler {
    job_queue: Arc<JobQueue>,
    change_analyzer: Arc<ChangeAnalyzer>,
    config: Arc<Config>,
}

impl EventHandler {
    pub async fn handle_repository_event(
        &self,
        event: RepositoryChangeEvent,
    ) -> Result<JobId, EventError> {
        // Check if repository is tracked
        if !self.is_tracked_repository(&event.repository.name) {
            return Err(EventError::RepositoryNotTracked);
        }

        // Analyze change significance
        let analysis = self.change_analyzer
            .analyze_commit(&event.commit)
            .await?;

        // Skip if changes don't require documentation update
        if analysis.change_significance == Significance::None {
            info!("Skipping documentation update for insignificant changes");
            return Err(EventError::InsignificantChange);
        }

        // Queue documentation generation job
        let job = PipelineJob::from_event(event, analysis);
        let job_id = self.job_queue.enqueue(job).await?;

        info!("Queued documentation job: {}", job_id);
        Ok(job_id)
    }
}
```

## 5. API Specification

### 5.1 Endpoint Reference

**Base URL**: `http://localhost:8080` (configurable)

**API Version**: `v1`

**All endpoints are prefixed with**: `/api/v1`

#### Health & Metadata

```
GET /api/v1/health
Response: 200 OK
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600
}
```

```
GET /api/v1/version
Response: 200 OK
{
  "version": "0.1.0",
  "commit": "abc123",
  "build_date": "2025-01-07T18:12:07Z"
}
```

```
GET /api/v1/openapi.json
Response: 200 OK
{
  "openapi": "3.0.0",
  "info": { ... },
  "paths": { ... }
}
```

#### Job Management

```
POST /api/v1/jobs
Request Body:
{
  "repository": "https://github.com/org/auth-service",
  "commit": "abc123",  // optional
  "categories": ["reference", "howto"]  // optional
}

Response: 201 Created
{
  "id": "01JG8X9YZ5N2QWE8R7TPMKFHVB",
  "state": "queued",
  "repository": "auth-service",
  "created_at": "2025-01-07T18:12:07.982682Z",
  "updated_at": "2025-01-07T18:12:07.982682Z"
}
```

```
GET /api/v1/jobs/:id
Response: 200 OK
{
  "id": "01JG8X9YZ5N2QWE8R7TPMKFHVB",
  "state": "completed",
  "repository": "auth-service",
  "created_at": "2025-01-07T18:12:07.982682Z",
  "updated_at": "2025-01-07T18:15:23.123456Z",
  "pr_url": "https://github.com/org/docs/pull/42"
}
```

```
GET /api/v1/jobs?state=running&limit=10
Response: 200 OK
{
  "jobs": [ ... ],
  "total": 3,
  "page": 1,
  "per_page": 10
}
```

```
DELETE /api/v1/jobs/:id
Response: 204 No Content
```

#### Repository Management

```
GET /api/v1/repositories
Response: 200 OK
{
  "repositories": [
    {
      "id": "01JG8X9YZ5N2QWE8R7TPMKFHVB",
      "name": "auth-service",
      "url": "https://github.com/org/auth-service",
      "language": "rust",
      "enabled": true,
      "last_analyzed": "2025-01-07T18:12:07.982682Z",
      "doc_coverage": 0.85
    }
  ]
}
```

```
POST /api/v1/repositories
Request Body:
{
  "url": "https://github.com/org/new-service",
  "language": "go",
  "enabled": true
}

Response: 201 Created
{
  "id": "01JG8X9YZ5N2QWE8R7TPMKFHVB",
  "name": "new-service",
  ...
}
```

```
GET /api/v1/repositories/:id
Response: 200 OK
{ ... }
```

```
PUT /api/v1/repositories/:id
Request Body:
{
  "enabled": false
}

Response: 200 OK
{ ... }
```

```
DELETE /api/v1/repositories/:id
Response: 204 No Content
```

#### Webhooks

```
POST /api/v1/webhooks/github
Headers:
  X-GitHub-Event: push
  X-Hub-Signature-256: sha256=...

Request Body: (GitHub webhook payload)

Response: 202 Accepted
{
  "job_id": "01JG8X9YZ5N2QWE8R7TPMKFHVB",
  "message": "Job queued for processing"
}
```

```
POST /api/v1/webhooks/gitlab
Headers:
  X-Gitlab-Event: Push Hook
  X-Gitlab-Token: ...

Request Body: (GitLab webhook payload)

Response: 202 Accepted
{
  "job_id": "01JG8X9YZ5N2QWE8R7TPMKFHVB",
  "message": "Job queued for processing"
}
```

### 5.2 OpenAPI Generation

**Technology**: `utoipa` crate for compile-time OpenAPI schema generation

**Implementation**:

```rust
use utoipa::{OpenApi, ToSchema};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "XZe Documentation API",
        version = "1.0.0",
        description = "AI-powered documentation generation service",
        contact(
            name = "XZe Team",
            email = "team@example.com"
        ),
        license(
            name = "MIT"
        )
    ),
    paths(
        // Job endpoints
        crate::handlers::jobs::create_job,
        crate::handlers::jobs::get_job,
        crate::handlers::jobs::list_jobs,
        crate::handlers::jobs::cancel_job,

        // Repository endpoints
        crate::handlers::repos::list_repositories,
        crate::handlers::repos::add_repository,
        crate::handlers::repos::get_repository,
        crate::handlers::repos::update_repository,
        crate::handlers::repos::delete_repository,

        // Webhook endpoints
        crate::handlers::webhooks::github_webhook,
        crate::handlers::webhooks::gitlab_webhook,
    ),
    components(
        schemas(
            CreateJobRequest,
            JobResponse,
            RepositoryResponse,
            AddRepositoryRequest,
            DiátaxisCategory,
            JobState,
        )
    ),
    tags(
        (name = "jobs", description = "Job management endpoints"),
        (name = "repositories", description = "Repository management endpoints"),
        (name = "webhooks", description = "Webhook receivers"),
        (name = "health", description = "Health and metadata endpoints"),
    )
)]
pub struct ApiDoc;

// Serve OpenAPI spec
pub async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
```

**Example OpenAPI Annotation**:

```rust
use utoipa::ToSchema;
use axum::{Json, extract::Path};

/// Create a new documentation generation job
#[utoipa::path(
    post,
    path = "/api/v1/jobs",
    request_body = CreateJobRequest,
    responses(
        (status = 201, description = "Job created successfully", body = JobResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "jobs"
)]
pub async fn create_job(
    Json(request): Json<CreateJobRequest>,
) -> Result<Json<JobResponse>, ApiError> {
    // Implementation
}
```

### 5.3 API Versioning Strategy

**Current Version**: `v1`

**Versioning Approach**: URL path versioning

**Breaking Changes**: New major version (v2, v3, etc.)

**Non-Breaking Changes**: Same version with additions

**Deprecation Policy**:

1. Announce deprecation in release notes
2. Support deprecated endpoints for minimum 6 months
3. Return `Deprecated` header on deprecated endpoints
4. Provide migration guide to new version

**Example**: When v2 is released, both `/api/v1/*` and `/api/v2/*` are supported simultaneously.

## 6. Workflow Algorithms

### 6.1 Event-Driven Workflow (Primary)

```
1. Event arrives (webhook or Kafka message)
2. Validate event source and signature
3. Extract repository and commit information
4. Check if repository is tracked in configuration
5. Analyze change significance:
   a. If significance == None → Skip processing
   b. If significance >= Patch → Continue
6. Queue documentation generation job
7. Return job ID to caller (202 Accepted)

Background Job Processing:
8. Dequeue next job from queue
9. Clone/update source repository
10. Analyze code structure with tree-sitter
11. Run AI analysis on changed files
12. Generate documentation for affected categories:
    a. Reference: API changes, new functions
    b. How-To: New configuration patterns
    c. Tutorial: Major new features
    d. Explanation: Architecture changes
13. Clone documentation repository
14. Create feature branch (e.g., "docs/update-auth-service-abc123")
15. Write generated documentation to appropriate paths
16. Commit changes with descriptive message
17. Push branch to remote
18. Create pull request with analysis summary
19. Update job state to "completed" with PR URL
20. Send notification (optional)
```

### 6.2 API-Triggered Workflow

```
1. Client sends POST /api/v1/jobs with repository info
2. Validate request body
3. Check if repository exists in tracking
4. Create job with "queued" state
5. Return job ID immediately (201 Created)
6. Queue job for background processing
7. Follow same background processing flow as event-driven

Client can poll job status:
- GET /api/v1/jobs/:id
- Returns current state and progress
```

### 6.3 SDK Direct Analysis Workflow

```
For ad-hoc analysis without xze-serve running:

1. User creates XzeDirect instance with config
2. User calls analyze_repository(path) or analyze_repository_url(url)
3. SDK clones repository (if URL) or uses local path
4. SDK analyzes code structure directly using xze-core
5. SDK runs AI analysis via Ollama
6. SDK generates documentation files
7. SDK returns analysis results and documents
8. User decides what to do with documents:
   - Write to local filesystem
   - Commit to git manually
   - Process further programmatically

Note: No job queue, no API server needed
```

### 6.4 Documentation Generation Algorithm

```
For each repository:
  1. Extract code structure using tree-sitter:
     - Parse source files by language
     - Identify modules, functions, types, configs

  2. Generate Reference documentation:
     - API endpoints → API reference (reference/service-name/api.md)
     - CLI commands → CLI reference (reference/service-name/cli.md)
     - Configuration → Config reference (reference/service-name/configuration.md)

  3. Generate How-To guides:
     - Common patterns in code → Task guides (how_to/service-name/*.md)
     - Configuration examples → Setup guides
     - Deployment patterns → Deployment guides

  4. Generate Tutorials:
     - Main use cases → Step-by-step tutorials (tutorials/service-name/*.md)
     - Quickstart from main() → Getting started guide

  5. Generate Explanations:
     - Architecture patterns → Design docs (explanation/service-name/*.md)
     - Trade-offs in code → Decision rationale
     - Complex algorithms → Conceptual explanations

  6. Cross-reference all documents:
     - Link references to how-tos
     - Link tutorials to references
     - Link explanations to implementations

  7. Validate generated markdown:
     - Check links are valid
     - Verify code blocks have language tags
     - Ensure Diátaxis structure compliance

  8. Write to documentation repository structure
```

## 7. Technology Stack

### 7.1 Core Dependencies

| Component          | Library                         | Purpose                      |
| ------------------ | ------------------------------- | ---------------------------- |
| Async Runtime      | `tokio`                         | Async task execution         |
| HTTP Client        | `reqwest`                       | Ollama & webhook HTTP client |
| Web Framework      | `axum`                          | REST API server              |
| OpenAPI Generation | `utoipa`                        | Compile-time OpenAPI schemas |
| Git Operations     | `git2`                          | Git integration              |
| Code Parsing       | `tree-sitter`                   | Source code parsing          |
| Serialization      | `serde`, `serde_json`           | Data serialization           |
| Logging            | `tracing`, `tracing-subscriber` | Structured logging           |
| Error Handling     | `anyhow`, `thiserror`           | Error management             |
| Kafka Client       | `rdkafka`                       | Redpanda/Kafka integration   |
| Testing            | `tokio-test`, `mockall`         | Unit/integration testing     |
| CLI Parsing        | `clap`                          | Command-line interface       |
| Unique IDs         | `ulid`                          | ULID generation              |
| Date/Time          | `chrono`                        | RFC-3339 timestamps          |

### 7.2 Language-Specific Parsers

Tree-sitter grammars for code analysis:

- Rust: `tree-sitter-rust`
- Python: `tree-sitter-python`
- Go: `tree-sitter-go`
- JavaScript/TypeScript: `tree-sitter-javascript`, `tree-sitter-typescript`
- Java: `tree-sitter-java`
- Additional languages can be added as plugins

### 7.3 Ollama Models

Recommended models for code analysis:

- **Primary**: `codellama:13b` (best for code understanding)
- **Fallback**: `mistral:7b` (faster, good general purpose)
- **Alternative**: `llama2:7b` (if others unavailable)

Model selection configurable via `xze.yaml`

## 8. Deployment Architecture

### 8.1 Docker Deployment

**Dockerfile** (`xze-serve`):

```dockerfile
FROM rust:1.90 as builder
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
RUN cargo build --release --bin xze-serve

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/xze-serve /usr/local/bin/

# Create non-root user
RUN useradd -m -u 1000 xze
USER xze

EXPOSE 8080
CMD ["xze-serve"]
```

**Docker Compose** (`docker-compose.yaml`):

```yaml
version: "3.8"

services:
  xze-server:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
      - XZE_CONFIG_PATH=/etc/xze/xze.yaml
      - OLLAMA_URL=http://ollama:11434
      - KAFKA_BROKERS=redpanda:9092
    volumes:
      - ./xze.yaml:/etc/xze/xze.yaml:ro
      - xze-cache:/var/cache/xze
      - git-credentials:/home/xze/.git-credentials:ro
    depends_on:
      - ollama
      - redpanda
    restart: unless-stopped

  ollama:
    image: ollama/ollama:latest
    ports:
      - "11434:11434"
    volumes:
      - ollama-models:/root/.ollama
    restart: unless-stopped

  redpanda:
    image: docker.redpanda.com/redpandadata/redpanda:latest
    command:
      - redpanda
      - start
      - --smp
      - "1"
      - --memory
      - "1G"
      - --overprovisioned
      - --kafka-addr
      - PLAINTEXT://0.0.0.0:9092
      - --advertise-kafka-addr
      - PLAINTEXT://redpanda:9092
    ports:
      - "9092:9092"
      - "9644:9644"
    volumes:
      - redpanda-data:/var/lib/redpanda/data
    restart: unless-stopped

volumes:
  xze-cache:
  ollama-models:
  redpanda-data:
```

**Usage**:

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f xze-server

# Stop services
docker-compose down

# Pull latest models
docker-compose exec ollama ollama pull codellama:13b
```

### 8.2 Kubernetes Deployment

**Namespace**:

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: xze-system
```

**Deployment**:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: xze-server
  namespace: xze-system
spec:
  replicas: 3
  selector:
    matchLabels:
      app: xze-server
  template:
    metadata:
      labels:
        app: xze-server
    spec:
      containers:
        - name: xze-server
          image: xze-server:latest
          ports:
            - containerPort: 8080
              name: http
          env:
            - name: RUST_LOG
              value: "info"
            - name: XZE_CONFIG_PATH
              value: "/etc/xze/xze.yaml"
            - name: OLLAMA_URL
              value: "http://ollama-service:11434"
            - name: KAFKA_BROKERS
              value: "redpanda-service:9092"
          volumeMounts:
            - name: config
              mountPath: /etc/xze
            - name: cache
              mountPath: /var/cache/xze
          livenessProbe:
            httpGet:
              path: /api/v1/health
              port: http
            initialDelaySeconds: 10
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /api/v1/health
              port: http
            initialDelaySeconds: 5
            periodSeconds: 10
          resources:
            requests:
              cpu: "500m"
              memory: "512Mi"
            limits:
              cpu: "2000m"
              memory: "2Gi"
      volumes:
        - name: config
          configMap:
            name: xze-config
        - name: cache
          persistentVolumeClaim:
            claimName: xze-cache
```

**Service**:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: xze-service
  namespace: xze-system
spec:
  type: LoadBalancer
  selector:
    app: xze-server
  ports:
    - port: 80
      targetPort: 8080
      name: http
```

**ConfigMap**:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: xze-config
  namespace: xze-system
data:
  xze.yaml: |
    version: "1.0"
    documentation_repo:
      url: "https://github.com/org/pipeline-documentation"
      branch: "main"
    ollama:
      url: "http://ollama-service:11434"
    events:
      kafka:
        enabled: true
        brokers: ["redpanda-service:9092"]
        topics: ["repository-changes"]
```

**PersistentVolumeClaim**:

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: xze-cache
  namespace: xze-system
spec:
  accessModes:
    - ReadWriteMany
  resources:
    requests:
      storage: 10Gi
```

**Horizontal Pod Autoscaler**:

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: xze-server-hpa
  namespace: xze-system
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: xze-server
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

### 8.3 Scaling Considerations

**Stateless Design**: XZe server is stateless, enabling horizontal scaling

**Job Queue**: Use Redis or database-backed queue for distributed job processing

**Shared Cache**: Use S3-compatible storage or shared PersistentVolume for repository cache

**Load Balancing**: Use Kubernetes Service or external load balancer

**Auto-Scaling**: Configure HPA based on CPU/memory or custom metrics (queue depth)

## 9. Security Considerations

### 9.1 Authentication & Authorization

**Git Credentials**:

- Stored in Kubernetes Secrets or environment variables
- Never logged or exposed in API responses
- Support for SSH keys and personal access tokens

**API Authentication**:

- JWT tokens for API access (future enhancement)
- API keys for programmatic access
- Role-based access control (RBAC)

**Webhook Security**:

- Signature verification for GitHub webhooks (HMAC-SHA256)
- Token verification for GitLab webhooks
- IP allowlisting for webhook sources

### 9.2 Code Execution Safety

**No Arbitrary Code Execution**:

- Tree-sitter parsing is safe (no code execution)
- AI prompts are templated and sanitized
- No `eval()` or dynamic code execution

**Input Validation**:

- All API inputs validated against schemas
- Path traversal prevention in file operations
- Repository URL validation

**Sandboxing**:

- Consider containerized code analysis for additional safety
- Resource limits on AI model inference

### 9.3 Secrets Management

**Best Practices**:

- Never log sensitive data (tokens, credentials, API keys)
- Scrub secrets from AI prompts before sending to Ollama
- Use environment variables or external secret managers
- Support for HashiCorp Vault, AWS Secrets Manager

**Secret Rotation**:

- Support for credential rotation without service restart
- Monitor for expired credentials

## 10. Monitoring & Observability

### 10.1 Metrics

**Job Metrics**:

- `xze_jobs_total` - Total jobs processed (counter)
- `xze_jobs_duration_seconds` - Job processing duration (histogram)
- `xze_jobs_failed_total` - Failed jobs (counter)
- `xze_jobs_queued` - Jobs in queue (gauge)

**API Metrics**:

- `xze_http_requests_total` - HTTP requests (counter)
- `xze_http_request_duration_seconds` - Request latency (histogram)
- `xze_http_errors_total` - HTTP errors by status code (counter)

**Ollama Metrics**:

- `xze_ollama_requests_total` - Ollama API calls (counter)
- `xze_ollama_duration_seconds` - Ollama response time (histogram)
- `xze_ollama_errors_total` - Ollama errors (counter)

**Repository Metrics**:

- `xze_repositories_tracked` - Number of tracked repositories (gauge)
- `xze_doc_coverage` - Documentation coverage by repository (gauge)

**Export Format**: Prometheus metrics at `/metrics` endpoint

### 10.2 Logging

All operations logged as structured JSON with fields:

```json
{
  "timestamp": "2025-01-07T18:12:07.982682Z",
  "level": "info",
  "target": "xze_serve::handlers::jobs",
  "message": "Job processing completed",
  "job_id": "01JG8X9YZ5N2QWE8R7TPMKFHVB",
  "repository": "auth-service",
  "operation": "generate_documentation",
  "duration_ms": 45231,
  "status": "completed",
  "pr_url": "https://github.com/org/docs/pull/42"
}
```

**Log Levels**:

- `ERROR`: Unrecoverable errors, failed jobs
- `WARN`: Recoverable errors, retries, degraded performance
- `INFO`: Job lifecycle events, API requests
- `DEBUG`: Detailed operation info (disabled in production)
- `TRACE`: Very detailed debugging (disabled in production)

### 10.3 Health Checks

**Liveness Probe** (`/api/v1/health`):

- Returns 200 if service is running
- Checks if HTTP server is responsive

**Readiness Probe** (`/api/v1/health`):

- Returns 200 if service is ready to accept traffic
- Checks:
  - Ollama connectivity
  - Kafka connectivity (if enabled)
  - Database connectivity (if used for job queue)

**Response**:

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "checks": {
    "ollama": "ok",
    "kafka": "ok"
  }
}
```

## 11. Testing Strategy

### 11.1 Unit Tests

**Scope**: Individual functions and components in isolation

**Coverage**: Target >80% code coverage

**Examples**:

- Repository metadata parsing
- Change significance analysis
- Prompt template rendering
- Configuration validation
- API request/response serialization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_analyzer_determines_major_significance() {
        let analyzer = ChangeAnalyzer::new(Config::default());
        let files = vec![PathBuf::from("src/api/endpoints.rs")];

        let significance = analyzer.determine_significance(&files);

        assert_eq!(significance, Significance::Major);
    }
}
```

### 11.2 Integration Tests

**Scope**: Component interactions with real dependencies

**Examples**:

- Full pipeline with sample repositories
- Ollama integration with real models
- Git operations with test repositories
- Kafka producer/consumer integration

```rust
#[tokio::test]
async fn test_full_documentation_pipeline() {
    let config = load_test_config();
    let controller = PipelineController::new(config).await.unwrap();

    let repo = Repository::from_path("tests/fixtures/sample-repo");
    let job = PipelineJob::new(repo);

    let result = controller.process_job(job).await;

    assert!(result.is_ok());
    assert!(matches!(result.unwrap().state, JobState::Completed { .. }));
}
```

### 11.3 End-to-End Tests

**Scope**: Full system with real external services

**Setup**:

- Deploy XZe in test environment
- Configure with test GitHub/GitLab repository
- Set up test documentation repository

**Test Cases**:

1. Trigger webhook from test repository push
2. Verify job is created and processed
3. Verify documentation is generated correctly
4. Verify PR is created in documentation repository
5. Verify Diátaxis structure compliance

**Performance Testing**:

- Load testing with multiple concurrent jobs
- Stress testing with large repositories
- Latency testing for API endpoints

## 12. Future Enhancements

### 12.1 Phase 2 Features

**Documentation Quality**:

- Documentation versioning and diff viewer
- A/B testing for generated documentation
- Feedback loop for documentation quality

**User Experience**:

- Interactive documentation review UI
- Web dashboard for job monitoring
- Real-time progress updates via WebSocket

**Content Generation**:

- Diagram generation (architecture, sequence, ERD)
- Multi-language documentation generation
- Custom documentation templates per repository

### 12.2 Advanced AI Features

**Model Improvements**:

- Fine-tuned models for specific domains
- Self-improving prompts based on feedback
- Ensemble model approach for higher quality

**Code Understanding**:

- Automated code example validation and testing
- Test generation from documentation
- Documentation-driven development support

**Quality Assurance**:

- Automated documentation testing
- Consistency checks across documentation
- Style guide enforcement

### 12.3 Enterprise Features

**Security & Compliance**:

- SSO integration (SAML, OAuth)
- Audit logging for compliance
- Data residency controls

**Management**:

- Multi-tenancy support
- Organization-level configuration
- Team management and permissions

**Operations**:

- SLA tracking for documentation freshness
- Advanced analytics and reporting
- Cost tracking and optimization

## 13. Project Structure

```
xze/
├── Cargo.toml                    # Workspace configuration
├── Cargo.lock
├── README.md
├── LICENSE
├── .gitignore
├── xze.yaml                      # Example configuration
│
├── crates/
│   ├── xze-core/                 # Core business logic (NO interface deps)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pipeline/         # Pipeline controller
│   │       ├── repository/       # Repository management
│   │       ├── ai/               # AI analysis service
│   │       ├── documentation/    # Documentation generation
│   │       ├── changes/          # Change analyzer
│   │       ├── git/              # Git operations
│   │       ├── pr/               # PR manager
│   │       └── infrastructure/   # Ollama, FS, logging
│   │
│   ├── xze-serve/                # REST API server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── api/              # API routes and handlers
│   │       ├── webhooks/         # Webhook receivers
│   │       ├── kafka/            # Kafka event consumer
│   │       └── openapi.rs        # OpenAPI schema
│   │
│   ├── xze-sdk/                  # Rust SDK library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client/           # HTTP API client
│   │       └── direct/           # Direct xze-core access
│   │
│   └── xze-cli/                  # CLI interface (API client)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           └── commands/         # CLI command handlers
│
├── docs/
│   ├── tutorials/
│   │   └── quickstart.md
│   ├── how_to/
│   │   ├── setup_development.md
│   │   └── configure_webhooks.md
│   ├── reference/
│   │   ├── architecture.md       # THIS FILE (canonical architecture)
│   │   ├── api.md                # API reference (generated from OpenAPI)
│   │   └── configuration.md      # Configuration reference
│   └── explanation/
│       ├── implementations.md    # Implementation log
│       └── design_decisions.md   # Design rationale
│
├── tests/
│   ├── integration/              # Integration tests
│   └── fixtures/                 # Test data and sample repos
│
├── examples/
│   ├── xze.yaml                  # Example configuration
│   └── custom_pipeline.rs        # SDK usage example
│
├── docker/
│   ├── Dockerfile                # Production Dockerfile
│   └── docker-compose.yaml       # Local development stack
│
└── k8s/
    ├── namespace.yaml
    ├── deployment.yaml
    ├── service.yaml
    ├── configmap.yaml
    └── hpa.yaml
```

## 14. Conclusion

This architecture document defines XZe as an **API-first, event-driven documentation generation system** built in Rust. The design prioritizes:

**API-First Architecture**:

- REST API (`xze-serve`) as the primary interface
- CLI (`xze-cli`) and SDK (`xze-sdk`) as API clients
- OpenAPI documentation with `utoipa`
- Versioned endpoints for stability

**Event-Driven Processing**:

- Webhook support for GitHub/GitLab
- Kafka/Redpanda integration for event streaming
- Asynchronous job processing
- Scalable, non-blocking architecture

**Modularity & Maintainability**:

- Clear crate boundaries with strict dependency rules
- xze-core contains domain logic only (no interface dependencies)
- Infrastructure abstraction for testability
- Well-defined data models and interfaces

**Production-Ready**:

- Comprehensive security considerations
- Observable operations (structured logging, metrics, health checks)
- Deployment configurations (Docker, Kubernetes)
- Horizontal scaling support

**Key Success Factors**:

- Well-defined component boundaries enable parallel development
- Comprehensive testing strategy at multiple levels
- Observable operations through structured logging and metrics
- Flexible deployment options (Docker, Kubernetes)
- Security-conscious design with credential management
- Extensible architecture for future enhancements

**Next Steps**:

1. Implement xze-core components (repository, AI, documentation)
2. Build REST API server (xze-serve) with OpenAPI
3. Implement event handlers (webhooks, Kafka)
4. Create SDK with dual interface (client + direct)
5. Build CLI as API client
6. Add comprehensive tests at all levels
7. Deploy and iterate based on real-world usage

This architecture provides a solid foundation for building XZe as a robust, scalable, and maintainable documentation automation tool that integrates seamlessly into modern development workflows.
