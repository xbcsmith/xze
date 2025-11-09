# Diataxis Directory Naming Convention

## Overview

This document explains the decision to use singular directory names for the Diataxis framework categories in XZe's documentation structure.

## Background

The Diataxis framework organizes documentation into four categories:

1. **Tutorial** - Learning-oriented guides
2. **How-To** - Task-oriented recipes
3. **Reference** - Information-oriented specifications
4. **Explanation** - Understanding-oriented discussion

## Directory Naming Decision

We use **singular** directory names for all Diataxis categories:

```text
docs/
├── tutorial/      # Not "tutorials"
├── how_to/        # Task-oriented (compound name)
├── reference/     # Not "references"
└── explanation/   # Not "explanations"
```

### Rationale

1. **Framework Consistency**: The Diataxis framework refers to these as singular category names (Tutorial, Explanation, Reference, How-To)

2. **Type Classification**: Each directory represents a single category/type of documentation, not a collection of plural items

3. **Parallel Structure**: Using singular names creates consistency across all four categories rather than mixing singular and plural

4. **Code Mapping**: Rust enums for categories use singular forms:
   ```rust
   pub enum DiátaxisCategory {
       Tutorial,
       HowTo,
       Reference,
       Explanation,
   }
   ```

## Implementation

### Changed Files

The following files were updated to reflect the singular naming:

**Rust Source Files:**
- `crates/core/src/documentation/crossref.rs`
- `crates/core/src/documentation/generator.rs`
- `crates/core/src/documentation/index.rs`

**Documentation Files:**
- `README.md` - Project structure diagram
- `docs/README.md` - Documentation navigation and structure
- `docs/explanation/README.md` - Category landing page
- `docs/explanation/plans/SEARCH_FEATURES_SUMMARY.md`
- `docs/how_to/configure_llm_keyword_extraction.md`
- `docs/how_to/create_pull_requests.md`
- `docs/reference/keyword_extraction_configuration.md`
- `docs/reference/search_api_endpoint.md`

**Directory Structure:**
- Renamed `docs/explanations/` → `docs/explanation/`

### Code Changes

Directory path mapping functions now return singular names:

```rust
fn get_category_dir(&self, category: &DiátaxisCategory) -> &'static str {
    match category {
        DiátaxisCategory::Tutorial => "tutorial",      // Was "tutorials"
        DiátaxisCategory::HowTo => "how_to",
        DiátaxisCategory::Reference => "reference",    // Was "references"
        DiátaxisCategory::Explanation => "explanation", // Was "explanations"
    }
}
```

Cross-reference links updated to use singular paths:

```rust
links.push_str("- [Explanation](../explanation/) - Understand concepts\n");
```

## Migration Notes

### Backward Compatibility

This is a **breaking change** for:
- Existing documentation links pointing to `docs/explanations/`
- Build scripts that reference old directory names
- IDE bookmarks or documentation indexes

### Migration Steps

1. Update all internal documentation links
2. Update external references (README, wiki, etc.)
3. Notify team members to update local bookmarks
4. Update CI/CD scripts if they reference doc paths
5. Search codebase for hardcoded paths: `grep -r "explanations" .`

## Validation

All changes were validated with:

```bash
# Format code
cargo fmt --all

# Check compilation
cargo check --all-targets --all-features

# Lint with zero warnings
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
cargo test --all-features
```

**Results**: All checks passed with zero errors and zero warnings.

## References

- [Diataxis Framework](https://diataxis.fr/) - Official documentation framework
- [AGENTS.md](../../AGENTS.md) - Project development guidelines
- [docs/README.md](../README.md) - Documentation structure overview

## Decision Log

**Date**: 2024
**Decision**: Use singular directory names for Diataxis categories
**Rationale**: Framework consistency, type classification semantics
**Impact**: Breaking change requiring link updates
**Status**: ✅ Implemented and validated
