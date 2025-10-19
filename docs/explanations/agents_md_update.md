# AGENTS.md Update - Project Name Change

## Overview

This document explains the updates made to the `AGENTS.md` file to reflect the
project name change from XZepr to XZe, including corrections to the project
description and architecture documentation.

## Changes Made

### Project Name Updates

All references to "XZepr" have been updated to "XZe" throughout the document:

- Main project name in the introduction
- Code examples and import statements
- Branch naming examples
- Commit message examples
- JIRA ticket prefixes

### Project Description Updates

Updated the project description to accurately reflect XZe's purpose:

**Before:**
- Type: High-performance event tracking server
- Key Features: Real-time streaming (Redpanda), Authentication, RBAC, Observability

**After:**
- Type: AI-powered documentation generator
- Key Features: AI-powered code analysis (Ollama), Git integration, Diataxis
  documentation framework, automated documentation generation

### Architecture Updates

Completely revised the architecture section to match XZe's actual crate-based
structure:

**Old Architecture (Event Tracking Server):**
```text
API Layer → Application Layer → Domain Layer
Auth Layer ← Infrastructure Layer
```

**New Architecture (Documentation Generator):**
```text
xze (Binary Crate)
├── xze-cli (CLI Interface)
│   └── xze-core (Core Logic)
└── xze-serve (Server Mode)
    └── xze-core (Core Logic)
```

### Code Example Updates

Updated all code examples to use the correct module names:

**Import Statements:**
- Changed: `use xzepr::math::factorial;`
- To: `use xze::math::factorial;`

**Module References:**
- Changed: `use xzepr::module::function;`
- To: `use xze::module::function;`

**Package References:**
- Changed: `use xzepr::module::Feature;`
- To: `use xze::module::Feature;`

### Branch Naming Examples

Updated branch naming examples:

**Before:**
```text
pr-cpipe-1234
pr-xzepr-5678
```

**After:**
```text
pr-cpipe-1234
pr-xze-5678
```

### Commit Message Examples

Updated JIRA ticket prefixes in commit examples:

**Before:**
```text
docs(readme): update installation instructions (XZEPR-9012)
feat(tracing): add distributed tracing support (XZEPR-4567)
```

**After:**
```text
docs(readme): update installation instructions (XZE-9012)
feat(tracing): add distributed tracing support (XZE-4567)
```

## Crate Dependencies

Updated the dependency rules to reflect the actual project structure:

### Allowed Dependencies

- xze → xze-cli, xze-serve
- xze-cli → xze-core
- xze-serve → xze-core

### Prohibited Dependencies

- xze-core → xze-cli (NEVER)
- xze-core → xze-serve (NEVER)
- xze-core → xze (NEVER)

**Rationale**: The core library must remain independent and reusable. Only
higher-level crates should depend on core, never the reverse.

## Crate Structure

### xze (Binary Crate)

Main entry point and CLI orchestration.

### xze-cli (crates/cli/)

Command-line interface and user interaction layer.

### xze-serve (crates/serve/)

Server mode with webhook support and REST API.

### xze-core (crates/core/)

Core business logic containing:
- AI analysis services (Ollama integration)
- Git operations and repository management
- Code parsers (Rust, Python, Go, etc.)
- Documentation generation (Diataxis framework)
- Pipeline controller and job management
- Change detection and repository watchers
- Pull request management (GitHub, GitLab)

## Verification

All changes were verified to ensure:

1. No references to "XZepr" or "xzepr" remain in the file
2. All "XZe" references are correctly capitalized
3. Code examples use valid module paths
4. Architecture diagram matches actual project structure
5. Dependencies follow the established rules
6. Build system remains unaffected (`cargo check` passes)

## Impact

These changes ensure that:

- AI agents receive accurate information about the project
- Code examples are valid and can be copied directly
- Architecture guidelines match the actual implementation
- New contributors understand the correct project structure
- Documentation generation follows established patterns

## Related Documentation

- Project Overview: `README.md`
- Implementation Roadmap: `docs/explanations/implementation_roadmap.md`
- Phase 2.2 Completion: `docs/explanations/phase2_2_completion.md`
- Phase 2.3 Summary: `docs/explanations/phase2_3_summary.md`
- Clippy Fixes: `docs/explanations/clippy_fixes_phase2.md`

## Notes for Future Updates

When updating AGENTS.md in the future:

1. Ensure all project references are consistent
2. Verify code examples compile and run
3. Keep architecture diagrams in sync with actual structure
4. Update dependency rules when adding new crates
5. Maintain the Diataxis documentation structure
6. Follow the file naming conventions (lowercase with underscores)
7. Run markdownlint to catch formatting issues
