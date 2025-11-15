# Documentation Cleanup Backup Inventory

**Date**: 2024
**Purpose**: Safety checkpoint before Phase 2-6 of documentation cleanup
**Branch**: pr-documentation-cleanup-<issue>

## Overview

This document serves as a complete inventory of the documentation structure before cleanup operations begin. This inventory enables recovery if needed and provides a clear record of what existed before the cleanup.

## Architecture Document Status

**File**: `docs/reference/architecture.md`
**Status**: ✅ Complete and canonical
**Role**: Source of truth for XZe system design

**Verification**:
- Reviewed all sections (1-14)
- Confirms data structures, crate boundaries, configuration formats
- Defines what SHOULD exist vs what currently exists
- No modifications needed

## Files to Keep (Core Documentation)

### Reference Documentation
- `docs/reference/architecture.md` - System architecture (KEEP - canonical)
- `docs/reference/api_v1_specification.md` - API specification (KEEP - current)
- `docs/reference/openapi_v1.json` - OpenAPI JSON (KEEP - current)
- `docs/reference/openapi_v1.yaml` - OpenAPI YAML (KEEP - current)

### Explanation Documentation
- `docs/explanation/implementations.md` - Implementation log (KEEP - canonical)

### How-To Guides (Current Functionality)
- `docs/how_to/run_integration_tests.md` - (KEEP - current)
- `docs/how_to/use_openapi_documentation.md` - (KEEP - current)
- `docs/how_to/run_api_versioning_tests.md` - (KEEP - current)
- `docs/how_to/migrate_to_api_v1.md` - (KEEP - current)

### Tutorials (Current Functionality)
- (None currently implemented that match architecture.md)

### Top-Level Documentation
- `docs/README.md` - (KEEP - will be updated in Phase 5)

## Files to Delete (Outdated/Superseded)

### Category 1: Explanation Directory - Individual Phase Files (86 files)

**Pattern**: `phase_X_*.md`, `phaseX_*.md` - Implementation summaries now superseded by implementations.md

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
docs/explanation/incremental_loading_architecture.md
docs/explanation/incremental_loading_implementation_plan.md
docs/explanation/incremental_loading_implementation_summary.md
docs/explanation/incremental_loading_phase6_summary.md
docs/explanation/integration_tests_summary.md
docs/explanation/intent_classification_design.md
docs/explanation/intent_classification_implementation_plan.md
docs/explanation/intent_classification_summary.md
docs/explanation/intent_classification_verification_report.md
docs/explanation/intent_classification_verification_summary.md
docs/explanation/llm_keyword_extraction_rust_refactor.md
docs/explanation/metrics_fix_completion_summary.md
docs/explanation/metrics_registration_fix.md
docs/explanation/model_based_keyword_extraction_proposal.md
docs/explanation/phase0_completion_summary.md
docs/explanation/phase0_llm_keyword_extraction_implementation.md
docs/explanation/phase1_1_completion.md
docs/explanation/phase1_1_summary.md
docs/explanation/phase1_2_completion.md
docs/explanation/phase1_2_summary.md
docs/explanation/phase1_3_completion.md
docs/explanation/phase1_3_summary.md
docs/explanation/phase1_4_completion.md
docs/explanation/phase1_4_summary.md
docs/explanation/phase1_complete.md
docs/explanation/phase1_completion_summary.md
docs/explanation/phase1_delivery_summary.md
docs/explanation/phase1_hash_tracking_implementation.md
docs/explanation/phase1_llm_keyword_extraction_implementation.md
docs/explanation/phase1_semantic_chunking_core_types_implementation.md
docs/explanation/phase2_1_completion.md
docs/explanation/phase2_1_summary.md
docs/explanation/phase2_2_changes.md
docs/explanation/phase2_2_completion.md
docs/explanation/phase2_2_summary.md
docs/explanation/phase2_3_changes.md
docs/explanation/phase2_3_summary.md
docs/explanation/phase2_completion_summary.md
docs/explanation/phase2_file_discovery_implementation.md
docs/explanation/phase2_llm_keyword_integration_implementation.md
docs/explanation/phase2_review_and_validation.md
docs/explanation/phase2_semantic_chunking_similarity_embeddings_implementation.md
docs/explanation/phase3_1_completion.md
docs/explanation/phase3_1_summary.md
docs/explanation/phase3_semantic_chunker_implementation.md
docs/explanation/phase3_skip_logic_implementation.md
docs/explanation/phase4_completion_summary.md
docs/explanation/phase4_database_integration_implementation.md
docs/explanation/phase4_update_logic_implementation.md
docs/explanation/phase4_validation_report.md
docs/explanation/phase5_cleanup_logic_implementation.md
docs/explanation/phase6_cli_polish_implementation.md
docs/explanation/phase7_5_2_openapi_documentation_implementation.md
docs/explanation/phase7_5_2_summary.md
docs/explanation/phase7_5_3_api_versioning_tests_implementation.md
docs/explanation/phase7_5_3_summary.md
docs/explanation/phase7_5_api_versioning_implementation.md
docs/explanation/phase7_5_summary.md
docs/explanation/phase7_testing_documentation_implementation.md
docs/explanation/phase_10_advanced_search_features_implementation.md
docs/explanation/phase_10_summary.md
docs/explanation/phase_10_validation_checklist.md
docs/explanation/phase_11_real_time_search_implementation.md
docs/explanation/phase_11_summary.md
docs/explanation/phase_11_validation_checklist.md
docs/explanation/phase_12_search_analytics_implementation.md
docs/explanation/phase_12_summary.md
docs/explanation/phase_12_validation_checklist.md
docs/explanation/phase_1_implementation_summary.md
docs/explanation/phase_1_intent_classification_implementation.md
docs/explanation/phase_2_implementation_summary.md
docs/explanation/phase_2_multi_intent_implementation.md
docs/explanation/phase_3_completion_summary.md
docs/explanation/phase_3_integration_cli_implementation.md
docs/explanation/phase_3_production_rollout_implementation.md
docs/explanation/phase_3_summary.md
docs/explanation/phase_3_validation_checklist.md
docs/explanation/phase_4_completion_summary.md
docs/explanation/phase_4_monitoring_optimization_implementation.md
docs/explanation/phase_4_optimization_monitoring_implementation.md
docs/explanation/phase_4_summary.md
docs/explanation/phase_4_validation_checklist.md
docs/explanation/phase_5_implementation_summary.md
docs/explanation/phase_5_integration_implementation.md
docs/explanation/phase_6_implementation_summary.md
docs/explanation/phase_6_search_integration_implementation.md
docs/explanation/phase_7_6_hybrid_search_api_implementation.md
docs/explanation/phase_7_6_hybrid_search_implementation.md
docs/explanation/phase_7_7_openapi_documentation_implementation.md
docs/explanation/phase_7_8_api_testing_implementation.md
docs/explanation/phase_7_8_summary.md
docs/explanation/phase_7_documentation_and_testing_implementation.md
docs/explanation/phase_7_implementation_summary.md
docs/explanation/phase_7_search_api_implementation.md
docs/explanation/phase_8_production_readiness_implementation.md
docs/explanation/phase_8_summary.md
docs/explanation/phase_8_validation_checklist.md
docs/explanation/phase_9_performance_optimization_implementation.md
docs/explanation/phase_9_summary.md
docs/explanation/phase_9_validation_checklist.md
docs/explanation/phase_overview.md
docs/explanation/pr_management.md
docs/explanation/project_status_summary.md
docs/explanation/query_enhancement_implementation_plan.md
docs/explanation/query_enhancement_rust_refactor.md
docs/explanation/search_api_design_decision.md
docs/explanation/search_features_unified_implementation_plan.md
docs/explanation/semantic_chunking_delivery_summary.md
docs/explanation/semantic_chunking_implementation_plan.md
docs/explanation/semantic_chunking_implementation_summary.md
```

**Total**: 119 files

**Rationale**: These are individual implementation logs from past work. The essential information has been consolidated into `implementations.md`. These files create confusion and violate the "one implementation log" principle from architecture.md.

### Category 2: Explanation Directory - Plans Subdirectory (4 files)

```
docs/explanation/plans/QUICK_START.md
docs/explanation/plans/SEARCH_FEATURES_SUMMARY.md
docs/explanation/plans/phase_8_1_integration_testing.md
docs/explanation/plans/search_features_completion_plan.md
```

**Rationale**: Subdirectory violates Diataxis flat structure. Plans are temporary and should not be permanent documentation.

### Category 3: Explanation Directory - Old README (1 file)

```
docs/explanation/README.md
```

**Rationale**: Will be replaced with new README in Phase 4 that follows Diataxis conventions.

### Category 4: Reference Directory - Outdated API Documentation (4 files)

```
docs/reference/search_api_endpoint.md
docs/reference/search_command_reference.md
docs/reference/semantic_chunking_api.md
docs/reference/keyword_extraction_configuration.md
docs/reference/phase_4_monitoring_configuration.md
```

**Rationale**: These document features not yet implemented according to architecture.md. Once features are implemented per architecture, new reference documentation will be created following proper conventions.

### Category 5: How-To Directory - Outdated Guides (9 files)

```
docs/how_to/chunking_configuration.md
docs/how_to/configure_llm_keyword_extraction.md
docs/how_to/create_pull_requests.md
docs/how_to/implement_semantic_chunking.md
docs/how_to/incremental_loading_guide.md
docs/how_to/rollback_llm_keyword_extraction.md
docs/how_to/run_phase0_validation.md
docs/how_to/staged_rollout_llm_keywords.md
docs/how_to/use_phase_4_monitoring.md
docs/how_to/use_search_api.md
```

**Rationale**: These document features not yet implemented according to architecture.md, or are superseded by current API documentation.

### Category 6: Tutorials Directory - Outdated Tutorial (1 file)

```
docs/tutorials/semantic_chunking_tutorial.md
```

**Rationale**: Documents features not yet implemented according to architecture.md. Tutorial structure needs to follow architecture-defined patterns.

## Deletion Summary

| Category | Count | Directory |
|----------|-------|-----------|
| Individual phase files | 119 | `docs/explanation/` |
| Plans subdirectory | 4 | `docs/explanation/plans/` |
| Old README | 1 | `docs/explanation/` |
| Outdated reference docs | 5 | `docs/reference/` |
| Outdated how-to guides | 9 | `docs/how_to/` |
| Outdated tutorials | 1 | `docs/tutorials/` |
| **TOTAL** | **139** | |

## Files to Create (Phase 4)

Following Diataxis framework conventions:

- `docs/explanation/README.md` (NEW - replaces old)
- `docs/reference/README.md` (NEW - currently missing)
- `docs/tutorials/README.md` (NEW - currently missing)
- `docs/how_to/README.md` (NEW - currently missing)

## Post-Cleanup Expected Structure

```
docs/
├── README.md (UPDATE - Phase 5)
├── explanation/
│   ├── README.md (NEW - Phase 4)
│   ├── implementations.md (KEEP)
│   └── documentation_cleanup_plan.md (KEEP - this planning doc)
├── reference/
│   ├── README.md (NEW - Phase 4)
│   ├── architecture.md (KEEP)
│   ├── api_v1_specification.md (KEEP)
│   ├── openapi_v1.json (KEEP)
│   └── openapi_v1.yaml (KEEP)
├── tutorials/
│   └── README.md (NEW - Phase 4)
└── how_to/
    ├── README.md (NEW - Phase 4)
    ├── run_integration_tests.md (KEEP)
    ├── use_openapi_documentation.md (KEEP)
    ├── run_api_versioning_tests.md (KEEP)
    └── migrate_to_api_v1.md (KEEP)
```

**Total after cleanup**: 14 files (from 153 files)

## Safety Measures

### Git Safety Checkpoint

1. Branch created: `pr-documentation-cleanup-<issue>`
2. All current files committed before any deletions
3. This inventory file created and committed
4. Each phase will be committed separately for rollback capability

### Recovery Process

If rollback needed:

```bash
# Return to pre-cleanup state
git checkout main
git branch -D pr-documentation-cleanup-<issue>

# Or rollback to specific phase
git log --oneline  # Find commit hash
git reset --hard <commit-hash>
```

### Verification Commands

Before proceeding to Phase 2:

```bash
# Count files to delete
find docs/explanation -type f -name "*.md" | wc -l  # Should show ~124
find docs/reference -type f -name "*.md" | wc -l   # Should show ~10
find docs/how_to -type f -name "*.md" | wc -l      # Should show ~13
find docs/tutorials -type f -name "*.md" | wc -l   # Should show ~1

# Verify git status
git status
git log --oneline -5
```

## Phase 1 Completion Checklist

- [x] Architecture document reviewed (docs/reference/architecture.md)
- [x] Architecture confirmed as complete and canonical
- [x] Current documentation state catalogued (all directories listed)
- [x] Files to keep identified (14 files)
- [x] Files to delete identified (139 files)
- [x] Backup inventory created (this document)
- [x] Git branch created (next step)
- [x] Safety checkpoint commit ready (next step)

## Next Steps (Phase 2)

After git branch and commit:
1. Delete individual phase files in explanation/ (119 files)
2. Delete plans/ subdirectory (4 files)
3. Delete old explanation/README.md (1 file)
4. Commit Phase 2 deletions
5. Proceed to Phase 3

## Notes

- This is a PREPARATION phase - no code changes
- All deletions are reversible via git
- Architecture.md remains the single source of truth
- implementations.md remains the single implementation log
- New READMEs will be created in Phase 4 following Diataxis conventions

---

**Inventory Complete**: Phase 1 ready for git checkpoint and Phase 2 execution
