# Documentation Cleanup Implementation Plan

## Overview

Clean up and align all documentation in `docs/explanation/` and `docs/reference/` with the new API-first, event-driven architecture defined in `docs/reference/architecture.md`. Remove all outdated files describing deprecated features (search APIs, semantic chunking, intent classification, incremental loading) that are not part of the current XZe system.

## Current State Analysis

### Existing Infrastructure

**docs/explanation/**:
- 121 markdown files describing deprecated features
- 1 subdirectory `plans/` with old implementation plans
- 1 `README.md` describing old system focus
- 1 `implementations.md` (KEEP - per AGENTS.md requirement)

**docs/reference/**:
- 1 `architecture.md` (KEEP - canonical architecture)
- 8 outdated files for search/chunking APIs not in new architecture
- Old OpenAPI specs for deprecated features

**docs/tutorials/** and **docs/how_to/**:
- Status unknown, needs verification for README.md presence

### Identified Issues

1. **Massive documentation debt**: 130+ files describing features not in new architecture
2. **Architecture mismatch**: Current docs describe search system; new architecture is documentation generation system
3. **Missing README.md files**: Diátaxis directories lack explanatory README.md files per AGENTS.md requirements
4. **No migration needed**: Project not yet in use, can do clean slate removal

## Implementation Phases

### Phase 1: Backup and Preparation

#### Task 1.1: Verify Current Architecture

**Actions**:
- Confirm `docs/reference/architecture.md` is the canonical source of truth
- Review architecture sections to identify what docs SHOULD exist (none currently implemented)
- Verify `docs/explanation/implementations.md` is the only implementation log

**Success Criteria**:
- Architecture document reviewed and confirmed as complete
- List of files to keep vs delete is finalized

#### Task 1.2: Create Safety Checkpoint

**Actions**:
- Create git branch: `pr-documentation-cleanup-<issue>`
- Commit current state before deletions
- Document deletion plan in this file

**Success Criteria**:
- Branch created
- Current state committed
- Ready to proceed with deletions

#### Task 1.3: Deliverables

- Git branch created
- Deletion list finalized
- Checkpoint commit created

#### Task 1.4: Success Criteria

- ✅ Architecture confirmed as complete
- ✅ Files to keep identified (2 files)
- ✅ Files to delete identified (130+ files)
- ✅ Git safety checkpoint created

### Phase 2: Delete Outdated Explanation Files

#### Task 2.1: Delete Individual Phase Files

**Actions**:
- Delete all `phase*.md` files in `docs/explanation/` (70+ files)
- Delete all implementation summaries (`*_implementation.md`, `*_summary.md`)
- Delete all design documents (`*_design.md`, `*_plan.md`)

**Files to DELETE** (examples):
```
docs/explanation/agents_md_update.md
docs/explanation/ai_analysis_service.md
docs/explanation/cli_argument_conflict_fix.md
docs/explanation/clippy_fixes_phase2.md
docs/explanation/diataxis_directory_naming.md
docs/explanation/git_integration.md
docs/explanation/git_operations.md
docs/explanation/implement_llm_keyword_extraction.md
docs/explanation/implementation_roadmap.md
docs/explanation/incremental_loading_*.md
docs/explanation/intent_classification_*.md
docs/explanation/integration_tests_summary.md
docs/explanation/llm_keyword_extraction_*.md
docs/explanation/metrics_*.md
docs/explanation/model_based_keyword_extraction_proposal.md
docs/explanation/phase*.md (all phase files)
docs/explanation/pr_management.md
docs/explanation/project_status_summary.md
docs/explanation/query_enhancement_*.md
docs/explanation/search_*.md
docs/explanation/semantic_chunking_*.md
```

**Files to KEEP**:
```
docs/explanation/implementations.md
```

**Success Criteria**:
- All outdated explanation files deleted
- Only `implementations.md` remains

#### Task 2.2: Delete Plans Subdirectory

**Actions**:
- Delete entire `docs/explanation/plans/` directory and contents

**Success Criteria**:
- `plans/` directory completely removed

#### Task 2.3: Delete Old README

**Actions**:
- Delete `docs/explanation/README.md` (will be replaced in Phase 4)

**Success Criteria**:
- Old README removed
- Directory contains only `implementations.md`

#### Task 2.4: Deliverables

- `docs/explanation/` contains only `implementations.md`
- 121+ files deleted
- `plans/` subdirectory removed

#### Task 2.5: Success Criteria

- ✅ All phase files deleted
- ✅ All implementation summaries deleted
- ✅ All design documents deleted
- ✅ `plans/` directory removed
- ✅ Old README.md deleted
- ✅ Only `implementations.md` remains

### Phase 3: Delete Outdated Reference Files

#### Task 3.1: Delete Search/Chunking API Documentation

**Actions**:
- Delete `docs/reference/api_v1_specification.md`
- Delete `docs/reference/search_api_endpoint.md`
- Delete `docs/reference/search_command_reference.md`
- Delete `docs/reference/semantic_chunking_api.md`

**Success Criteria**:
- All search-related API docs removed

#### Task 3.2: Delete Configuration Files

**Actions**:
- Delete `docs/reference/keyword_extraction_configuration.md`
- Delete `docs/reference/phase_4_monitoring_configuration.md`

**Success Criteria**:
- All outdated configuration references removed

#### Task 3.3: Delete Old OpenAPI Specs

**Actions**:
- Delete `docs/reference/openapi_v1.json`
- Delete `docs/reference/openapi_v1.yaml`

**Rationale**: These describe search API endpoints not in new architecture. New OpenAPI spec will be generated from `xze-serve` using `utoipa` crate.

**Success Criteria**:
- Old OpenAPI files removed

#### Task 3.4: Deliverables

- `docs/reference/` contains only `architecture.md`
- 8 outdated reference files deleted

#### Task 3.5: Success Criteria

- ✅ All search API docs deleted
- ✅ All outdated config docs deleted
- ✅ Old OpenAPI specs deleted
- ✅ Only `architecture.md` remains in `docs/reference/`

### Phase 4: Create New README Files

#### Task 4.1: Create docs/explanation/README.md

**Actions**:
- Create new `docs/explanation/README.md`
- Explain Diátaxis "Explanation" category
- Document that this directory contains:
  - `implementations.md` - chronological implementation log (per AGENTS.md)
  - Future design decision documents
  - Future architectural explanations
- Align with new architecture focus: AI-powered documentation generation

**Content Template**:
```markdown
# Explanation

This directory contains understanding-oriented documentation for the XZe project following the Diátaxis framework.

## What is Explanation?

Explanation documents provide context, background, and reasoning behind the XZe system. They help you understand:

- Why architectural decisions were made
- How components fit together conceptually
- Trade-offs and alternatives considered
- Design patterns and principles used

## Contents

### Implementation Log

- [implementations.md](implementations.md) - Chronological log of all implementations (REQUIRED: all agents append here)

### Future Content

As XZe development progresses, this directory will contain:

- Architecture decision records (ADRs)
- Design rationale for major features
- Conceptual explanations of the AI-powered documentation system
- Event-driven architecture explanations
- API-first design principles

## Current Architecture

XZe is an AI-powered documentation generation tool with:

- **API-First Design**: REST API (xze-serve) as primary interface
- **Event-Driven**: Webhooks (GitHub/GitLab) and Kafka (Redpanda) trigger documentation updates
- **AI-Powered**: Uses Ollama models to analyze code and generate Diátaxis-structured documentation
- **Modular Crates**: xze-core (domain logic), xze-serve (API), xze-sdk (library), xze-cli (client)

See [architecture.md](../reference/architecture.md) for complete technical specification.

## Contributing

When adding explanation documents:

- Focus on "why" and "what" rather than "how"
- Provide context and background
- Discuss alternatives and trade-offs
- Link to related reference, tutorials, and how-to docs
- Follow lowercase_with_underscores.md naming convention
- Append implementation summaries to `implementations.md`
```

**Success Criteria**:
- New README.md created and follows Diátaxis principles
- References new architecture correctly
- Explains `implementations.md` requirement

#### Task 4.2: Create docs/reference/README.md

**Actions**:
- Create new `docs/reference/README.md`
- Explain Diátaxis "Reference" category
- Document current contents: `architecture.md`
- List future reference documentation (API, configuration, CLI)

**Content Template**:
```markdown
# Reference

This directory contains technical reference documentation for the XZe project following the Diátaxis framework.

## What is Reference?

Reference documentation provides precise, technical specifications. It is:

- Information-oriented
- Accurate and up-to-date
- Structured for lookup, not learning
- Comprehensive in coverage

## Contents

### Architecture

- [architecture.md](architecture.md) - **Canonical architecture specification** (SOURCE OF TRUTH)

### Future Content

As XZe is implemented, this directory will contain:

- **API Reference**: REST API endpoint specifications (auto-generated from OpenAPI)
- **Configuration Reference**: Complete xze.yaml configuration options
- **CLI Reference**: Command-line interface documentation
- **Data Models**: Request/response schemas, internal data structures
- **Error Codes**: Complete error reference with remediation steps

## Current System

XZe is an API-first, event-driven documentation generation system. See [architecture.md](architecture.md) for:

- System overview and design principles
- Component design (API, SDK, CLI, core services)
- Data models and configuration schemas
- Event-driven architecture (webhooks, Kafka)
- API specifications and versioning
- Deployment architecture (Docker, Kubernetes)
- Security, monitoring, testing strategies

## API Versioning

All XZe APIs use version prefix: `/api/v1/*`

OpenAPI specification will be available at: `GET /api/v1/openapi.json`

## Contributing

When adding reference documents:

- Maintain technical accuracy
- Use precise language and specifications
- Include code examples with expected outputs
- Follow lowercase_with_underscores.md naming convention
- Update this README when adding new reference docs
```

**Success Criteria**:
- New README.md created
- Correctly identifies `architecture.md` as canonical source
- Lists future reference documentation aligned with architecture

#### Task 4.3: Verify/Create docs/tutorials/README.md

**Actions**:
- Check if `docs/tutorials/README.md` exists
- If exists: review and update to align with new architecture
- If missing: create new README.md explaining tutorial category

**Content Template** (if creating):
```markdown
# Tutorials

This directory contains learning-oriented tutorials for the XZe project following the Diátaxis framework.

## What are Tutorials?

Tutorials are lessons that guide you through learning XZe step-by-step. They are:

- Learning-oriented
- Hands-on and practical
- Safe to follow (show the happy path)
- Provide a sense of achievement

## Future Content

As XZe is implemented, this directory will contain:

- **Getting Started**: First-time setup and basic usage
- **Your First Documentation Generation**: Analyze a repository and generate docs
- **Working with Events**: Set up webhooks and Kafka integration
- **Deploying XZe**: Deploy with Docker and Kubernetes
- **Using the SDK**: Build custom integrations with xze-sdk

## Prerequisites

Before starting tutorials, ensure you have:

- Rust toolchain installed (rustup, cargo)
- Docker and Docker Compose (for local development)
- Ollama running locally or accessible remotely
- Git installed

## Contributing

When adding tutorials:

- Focus on learning, not reference
- Show the complete path from start to finish
- Keep steps clear and achievable
- Test tutorials on a fresh environment
- Follow lowercase_with_underscores.md naming convention
```

**Success Criteria**:
- README.md exists in `docs/tutorials/`
- Content aligns with new architecture

#### Task 4.4: Verify/Create docs/how_to/README.md

**Actions**:
- Check if `docs/how_to/README.md` exists
- If exists: review and update to align with new architecture
- If missing: create new README.md explaining how-to category

**Content Template** (if creating):
```markdown
# How-To Guides

This directory contains task-oriented how-to guides for the XZe project following the Diátaxis framework.

## What are How-To Guides?

How-to guides are recipes for solving specific problems. They are:

- Goal-oriented
- Focused on results, not explanation
- Assume you understand basics
- Solve real-world problems

## Future Content

As XZe is implemented, this directory will contain:

- **Configure Webhooks**: Set up GitHub/GitLab webhooks for XZe
- **Set Up Kafka**: Configure Redpanda/Kafka event streaming
- **Customize AI Models**: Switch Ollama models and tune generation
- **Deploy to Kubernetes**: Production deployment configuration
- **Monitor XZe**: Set up metrics and logging
- **Troubleshoot Common Issues**: Debug common problems

## Prerequisites

How-to guides assume you have:

- Completed at least one tutorial
- Basic understanding of XZe architecture
- Development environment set up

## Contributing

When adding how-to guides:

- Start with a clear goal
- List prerequisites
- Provide step-by-step instructions
- Show expected results
- Follow lowercase_with_underscores.md naming convention
```

**Success Criteria**:
- README.md exists in `docs/how_to/`
- Content aligns with new architecture

#### Task 4.5: Deliverables

- `docs/explanation/README.md` created
- `docs/reference/README.md` created
- `docs/tutorials/README.md` verified or created
- `docs/how_to/README.md` verified or created

#### Task 4.6: Success Criteria

- ✅ All four Diátaxis category directories have README.md
- ✅ All READMEs explain their purpose per Diátaxis framework
- ✅ All READMEs align with new architecture
- ✅ Diátaxis framework only explained in main `docs/README.md`

### Phase 5: Update Main Documentation README

#### Task 5.1: Update docs/README.md

**Actions**:
- Update `docs/README.md` to reference Diátaxis framework
- Link to category READMEs
- Explain overall documentation structure
- Link to `architecture.md` as starting point

**Success Criteria**:
- Main README updated
- Diátaxis framework explained with external link
- Clear navigation to all documentation categories

#### Task 5.2: Deliverables

- Updated `docs/README.md`

#### Task 5.3: Success Criteria

- ✅ Main README explains Diátaxis framework
- ✅ Links to all category READMEs
- ✅ Points to `architecture.md` as canonical spec

### Phase 6: Verification and Commit

#### Task 6.1: Verify Cleanup

**Actions**:
- Confirm only these files remain:
  - `docs/explanation/README.md`
  - `docs/explanation/implementations.md`
  - `docs/reference/README.md`
  - `docs/reference/architecture.md`
  - `docs/tutorials/README.md`
  - `docs/how_to/README.md`
  - `docs/README.md`
- Run `cargo fmt --all`
- Run `cargo check --all-targets --all-features`

**Success Criteria**:
- File count verified (7 files total in docs/)
- No compilation errors
- No formatting issues

#### Task 6.2: Update implementations.md

**Actions**:
- Append cleanup summary to `docs/explanation/implementations.md`
- Document what was deleted and why
- Record new README.md files created

**Success Criteria**:
- Cleanup documented in implementations.md
- Follows template format from AGENTS.md

#### Task 6.3: Create Pull Request

**Actions**:
- Commit all changes with message: `docs: clean up outdated documentation files`
- Push branch
- Create PR with:
  - Title: "Documentation Cleanup - Remove Outdated Files"
  - Description summarizing deletions and new structure
  - Link to this plan document

**Success Criteria**:
- All changes committed
- PR created
- Ready for review

#### Task 6.4: Deliverables

- Clean documentation structure
- Updated implementations.md
- Pull request created

#### Task 6.5: Success Criteria

- ✅ Only 7 files remain in docs/ directories
- ✅ All README.md files created and accurate
- ✅ Cleanup documented in implementations.md
- ✅ PR created for review

## Validation Checklist

### Pre-Implementation

- [ ] New architecture.md reviewed and confirmed as complete
- [ ] Files to keep vs delete identified
- [ ] Git branch created: `pr-documentation-cleanup-<issue>`
- [ ] Safety checkpoint commit created

### Post-Implementation

- [ ] All 121+ explanation files deleted (except implementations.md)
- [ ] `docs/explanation/plans/` directory deleted
- [ ] All 8 outdated reference files deleted (except architecture.md)
- [ ] `docs/explanation/README.md` created
- [ ] `docs/reference/README.md` created
- [ ] `docs/tutorials/README.md` verified or created
- [ ] `docs/how_to/README.md` verified or created
- [ ] `docs/README.md` updated with Diátaxis explanation
- [ ] Only 7 files remain in docs/ structure
- [ ] Cleanup documented in implementations.md
- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] Pull request created

## Summary

This cleanup removes 130+ outdated documentation files describing deprecated features (search APIs, semantic chunking, intent classification, incremental loading) that are not part of the new API-first, event-driven XZe architecture. The result is a clean slate with only:

- `architecture.md` - canonical architecture specification
- `implementations.md` - implementation log per AGENTS.md
- README.md files explaining each Diátaxis category

New documentation will be created as features are implemented following the architecture defined in `docs/reference/architecture.md`.
