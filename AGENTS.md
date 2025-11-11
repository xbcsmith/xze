# AGENTS.md - AI Agent Development Guidelines

**CRITICAL**: This file contains mandatory rules for AI agents working on XZe.
Non-compliance will result in rejected code.

---

## Quick Reference for AI Agents

### BEFORE YOU START ANY TASK

#### Step 1: Verify Tools Are Installed

```bash
rustup component add clippy rustfmt
cargo install cargo-audit  # Optional but recommended
```

#### Step 2: MANDATORY - Consult Architecture Document FIRST

**Before writing ANY code, you MUST:**

1. **Read** `docs/reference/architecture.md` sections relevant to your task
2. **Verify** data structures match the architecture EXACTLY
3. **Check** module placement - don't create new modules arbitrarily
4. **Confirm** crate boundaries are respected (xze-core has NO deps on xze-cli/xze-serve)
5. **Use** the exact type names, field names, and signatures defined in architecture
6. **NEVER** modify core data structures without explicit approval

**Rule**: If architecture.md defines it, YOU MUST USE IT EXACTLY AS DEFINED.
Deviation = violation.

#### Step 3: Plan Your Implementation

- Identify which files need changes
- Determine what tests are needed
- Choose correct documentation category (Diataxis)

### AFTER YOU COMPLETE ANY TASK

#### Step 1: Run Quality Checks (ALL MUST PASS)

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

**Expected**: Zero errors, zero warnings, all tests pass.

#### Step 2: Verify Architecture Compliance

- [ ] Data structures match architecture.md definitions **EXACTLY**
- [ ] Module placement follows architecture structure
- [ ] Crate boundaries respected (xze-core → NO deps on cli/serve)
- [ ] Type names match architecture (Repository, Document, etc.)
- [ ] Configuration format follows architecture (YAML for configs)
- [ ] No architectural deviations without documentation

#### Step 3: Final Verification

1. **Re-read** relevant architecture.md sections
2. **Confirm** no architectural drift introduced
3. **Update** `docs/explanation/implementations.md` with your work summary

**If you can't explain WHY your code differs from architecture.md, IT'S WRONG.**

**IF ANY CHECK FAILS, YOU MUST FIX IT BEFORE PROCEEDING.**

---

## IMPLEMENTATION RULES - NEVER VIOLATE

**Detailed rules for implementing code. See "Five Golden Rules" section at end for quick reference.**

### Implementation Rule 1: File Extensions (MOST VIOLATED)

**YOU WILL GET THIS WRONG IF YOU DON'T READ CAREFULLY**

#### Real Files vs. Documentation

- **Real implementation files**: `src/**/*.rs`, `crates/**/*.rs` - actual code that compiles
- **Configuration files**: `.yaml`, `.toml` - runtime configuration
- **Documentation files**: `docs/**/*.md` - explanations, references, guides

#### The Test: "Is this code going to be executed?"

**YES - It's real code:**

- ✓ Save as `.rs` in `src/` or `crates/` directory
- ✓ Must compile with `cargo check`
- ✓ Must pass all quality gates

**NO - It's documentation/example:**

- ✓ Keep in `.md` file with proper code blocks
- ✓ Use path annotation: ```path/to/file.rs#L1-10
- ✓ Mark as pseudo-code if not compilable

#### Configuration File Extensions

Per architecture.md:

- **XZe Config**: `xze.yaml` (NOT .yml)
- **Docker Compose**: `docker-compose.yaml` (NOT .yml)
- **CI/CD Configs**: `.yaml` extension (NOT .yml)
- **Cargo**: `Cargo.toml` (NOT .yaml)

**WRONG**: Creating config files with `.yml` extension
**RIGHT**: Using `.yaml` extension as specified in architecture

**Why this is violated**: Agents see `.yml` commonly used in industry and default to it. **NO**. XZe uses `.yaml` consistently.

**YOU MUST:**

- Use `.rs` extension for ALL Rust implementation files
- Use `.md` extension for ALL Markdown files
- Use `.yaml` extension for ALL YAML configuration files
- Use `.toml` extension for Cargo configuration

**NEVER:**

- ❌ Use `.yml` extension (even though common in industry)
- ❌ Use `.MD` or `.markdown` extensions
- ❌ Create `.rs` files for code that only appears in architecture documentation

**Clarification**: YAML is for configuration files (xze.yaml, docker-compose.yaml, CI/CD configs). Use `.yaml` extension consistently.

### Implementation Rule 2: Markdown File Naming (SECOND MOST VIOLATED)

**YOU MUST:**

- Use lowercase letters ONLY
- Use underscores to separate words
- Exception: `README.md` is the ONLY uppercase filename allowed

**NEVER:**

- ❌ Use CamelCase (DistributedTracing.md)
- ❌ Use kebab-case (distributed-tracing.md)
- ❌ Use spaces (Distributed Tracing.md)
- ❌ Use uppercase (DISTRIBUTED_TRACING.md)

**Examples:**

```text
✅ CORRECT:
   docs/explanation/implementations.md
   docs/how_to/setup_monitoring.md
   docs/reference/architecture.md
   README.md (ONLY exception)

❌ WRONG:
   docs/explanation/Implementations.md
   docs/explanation/implementations-summary.md
   docs/how_to/Setup-Monitoring.md
```

**Why This Matters**: Inconsistent naming breaks documentation linking and makes files hard to find.

### Implementation Rule 3: Code Quality Gates (MUST ALL PASS)

**Run these commands AFTER implementing your code (not before):**

```bash
# Run in this exact order:

# 1. Format (auto-fixes issues)
cargo fmt --all

# 2. Compile check (fast, no binary)
cargo check --all-targets --all-features

# 3. Lint (treats warnings as errors)
cargo clippy --all-targets --all-features -- -D warnings

# 4. Tests
cargo test --all-features
```

**Expected Results:**

```text
✅ cargo fmt         → No output (all files formatted)
✅ cargo check       → "Finished" with 0 errors
✅ cargo clippy      → "Finished" with 0 warnings
✅ cargo test        → "test result: ok. X passed; 0 failed"
```

**IF ANY FAIL**: Stop immediately and fix before proceeding.

**Note**: These are validation commands, not planning commands. Run AFTER writing code.

### Implementation Rule 4: Documentation is Mandatory

**YOU MUST:**

- Add `///` doc comments to EVERY public function, struct, enum, module
- Include runnable examples in doc comments (tested by `cargo test`)
- Update `docs/explanation/implementations.md` for EVERY feature/task

**DO NOT:**

- ❌ Create new documentation files without being asked
- ❌ Skip documentation because "code is self-documenting"
- ❌ Put documentation in wrong directory or use wrong filename format

**ONLY UPDATE THESE FILES unless explicitly instructed otherwise:**

- `docs/explanation/implementations.md` (your summary of what you built)
- Code comments (/// doc comments in .rs files)

**Rule**: Append your implementation summary to `implementations.md`. Do NOT create separate markdown files for each feature unless explicitly instructed.

**Examples:**

````rust
/// Generates documentation for a repository using AI analysis
///
/// # Arguments
///
/// * `repo` - The repository to analyze
/// * `config` - AI model configuration
///
/// # Returns
///
/// Returns `Ok(Document)` with generated documentation
///
/// # Errors
///
/// Returns `GenerationError::AIServiceFailed` if Ollama is unreachable
/// Returns `GenerationError::InvalidResponse` if AI response is malformed
///
/// # Examples
///
/// ```
/// use xze_core::ai::AIAnalysisService;
/// use xze_core::repository::Repository;
///
/// let service = AIAnalysisService::new(config);
/// let doc = service.generate_reference(&repo).await?;
/// assert_eq!(doc.category, DocumentCategory::Reference);
/// ```
pub async fn generate_reference(
    &self,
    repo: &Repository,
) -> Result<Document, GenerationError> {
    // Implementation
}
````

### Implementation Rule 5: Error Handling (MANDATORY PATTERNS)

**YOU MUST:**

- Use `Result<T, E>` for ALL recoverable errors
- Use `?` operator for error propagation
- Use `thiserror` for custom error types
- Use descriptive error messages

**NEVER:**

- ❌ Use `unwrap()` without justification comment
- ❌ Use `expect()` without descriptive message
- ❌ Ignore errors with `let _ =`
- ❌ Return `panic!` for recoverable errors

**Correct Pattern:**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("Failed to clone repository: {0}")]
    CloneFailed(String),

    #[error("Invalid repository URL: {0}")]
    InvalidUrl(String),
}

pub async fn clone_repository(url: &str) -> Result<Repository, RepositoryError> {
    let path = validate_url(url)
        .map_err(|e| RepositoryError::InvalidUrl(e.to_string()))?;

    // Clone logic
    Ok(repository)
}
```

---

## Project Overview

### Identity

- **Name**: XZe
- **Type**: AI-powered documentation generator
- **Language**: Rust (latest stable)
- **Key Features**:
  - **AI-Powered Analysis**: Uses Ollama for intelligent code analysis
  - **Diataxis Framework**: Structured documentation (Reference, How-To, Explanation, Tutorial)
  - **Git Integration**: Automatic PR creation, change detection, CI/CD hooks
  - **Multi-Repository Support**: Analyzes and documents multiple codebases

### Architecture (Crate-Based Design)

**CRITICAL**: YOU MUST respect these crate boundaries:

```text
xze (binary)
├─ xze-cli (crates/cli/)     - CLI interface
├─ xze-serve (crates/serve/) - Server mode, webhooks, REST API
└─ xze-core (crates/core/)   - Core business logic (NO deps on cli/serve)
```

**Dependency Rules (STRICT):**

- ✅ xze → xze-cli, xze-serve
- ✅ xze-cli → xze-core
- ✅ xze-serve → xze-core
- ❌ xze-core → xze-cli (NEVER - violates architecture)
- ❌ xze-core → xze-serve (NEVER - violates architecture)
- ❌ xze-core → xze (NEVER - violates architecture)

**Why This Matters**: xze-core is the domain layer. It must remain independent of interface concerns. Breaking this boundary creates circular dependencies and makes the code untestable.

---

## Development Workflow

### Step-by-Step Process (FOLLOW EXACTLY)

#### Phase 1: Preparation

1. **Read Architecture First**

   - Consult `docs/reference/architecture.md`
   - Understand data structures (Section 3)
   - Verify module placement (Section 2)
   - Check crate boundaries

2. **Search Existing Code**

   ```bash
   # Find relevant files
   grep -r "function_name" src/ crates/
   find src/ crates/ -name "*feature*.rs"
   ```

3. **Plan Changes**
   - List files to create/modify
   - Identify tests needed
   - Ensure architecture compliance

#### Phase 2: Implementation

1. **Write Code**

   Follow this pattern for ALL public items:

   ````rust
   /// One-line description
   ///
   /// Longer explanation of behavior and purpose.
   ///
   /// # Arguments
   ///
   /// * `param` - Description
   ///
   /// # Returns
   ///
   /// Description of return value
   ///
   /// # Errors
   ///
   /// Returns `ErrorType` if condition
   ///
   /// # Examples
   ///
   /// ```
   /// use xze_core::module::function;
   ///
   /// let result = function(arg);
   /// assert_eq!(result, expected);
   /// ```
   pub fn function(param: Type) -> Result<ReturnType, Error> {
       // Implementation
   }
   ````

2. **Write Tests (MANDATORY)**

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_function_with_valid_input() {
           // Arrange
           let input = "test";

           // Act
           let result = function(input);

           // Assert
           assert!(result.is_ok());
           assert_eq!(result.unwrap(), expected);
       }

       #[test]
       fn test_function_with_invalid_input() {
           let result = function("");
           assert!(result.is_err());
       }

       #[test]
       fn test_function_edge_case() {
           // Test boundary conditions
       }
   }
   ```

   **Test Requirements:**

   - Test ALL public functions
   - Cover success, failure, and edge cases
   - Use descriptive names: `test_{function}_{condition}_{expected}`
   - Minimum 3 tests per function

3. **Run Quality Checks Incrementally**

   ```bash
   # After writing code
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings

   # After writing tests
   cargo test --all-features

   # Before committing - verify all checks pass
   cargo fmt --all
   cargo check --all-targets --all-features
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-features
   ```

#### Phase 3: Documentation

**Update** `docs/explanation/implementations.md`:

Append your implementation summary to the end of the file:

````markdown
---

## Feature Name Implementation

**Date**: YYYY-MM-DD
**Author**: AI Agent

### Overview

Brief description of what was implemented and why.

### Components Delivered

- `src/path/file.rs` (XXX lines) - Description
- `crates/core/src/module.rs` (YYY lines) - Description

### Implementation Details

Technical explanation with code examples.

### Architecture Compliance

- ✅ Followed architecture.md Section X.Y
- ✅ Respected crate boundaries
- ✅ Used defined data structures exactly

### Testing

Test coverage results:

```text
test result: ok. X passed; 0 failed
```
````

### Validation Results

- ✅ `cargo fmt --all` passed
- ✅ `cargo check --all-targets --all-features` passed
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` passed
- ✅ `cargo test --all-features` passed

### References

- Architecture: `docs/reference/architecture.md` Section X.Y

````

**Do NOT create separate markdown files unless explicitly instructed.**

#### Phase 4: Validation (CRITICAL)

**Run these commands and verify output:**

```bash
# 1. Format check
cargo fmt --all
# Expected: No output (all files formatted)

# 2. Compilation check
cargo check --all-targets --all-features
# Expected: "Finished" with 0 errors

# 3. Lint check (treats warnings as errors)
cargo clippy --all-targets --all-features -- -D warnings
# Expected: "Finished" with 0 warnings

# 4. Test check
cargo test --all-features
# Expected: "test result: ok. X passed; 0 failed"

# 5. Verify architecture compliance
# Re-read relevant architecture.md sections
# Confirm no deviations introduced

# 6. Verify implementations.md updated
git diff docs/explanation/implementations.md
# Expected: Shows your appended summary
````

**IF ANY VALIDATION FAILS: Stop and fix immediately.**

---

## Git Conventions

### Branch Naming

**Format:** `pr-<description>-<issue>`

**Examples:**

```
✅ pr-semantic-chunking-1234
✅ pr-ollama-integration-5678
❌ PR-FEAT-1234 (uppercase)
❌ feature/auth-1234 (wrong prefix)
```

### Commit Messages

**Format:**

```
<type>(<scope>): <description>

[optional body]
```

**Rules:**

1. Type: `feat|fix|docs|style|refactor|perf|test|chore`
2. Description: lowercase, imperative mood ("add" not "added")
3. First line: ≤72 characters
4. Scope: optional but recommended

**Examples:**

```
✅ feat(ai): add ollama integration for code analysis
✅ fix(git): handle edge case in PR creation
✅ docs(arch): update architecture with new modules
❌ Added Ollama support (no type, wrong mood)
❌ feat(ai): Add Ollama (wrong case)
```

---

## Documentation Organization (Diataxis Framework)

**YOU MUST categorize documentation correctly:**

### Category 1: Tutorials (`docs/tutorials/`)

**Purpose**: Learning-oriented, step-by-step lessons

**Use for**: Getting started guides, learning paths

### Category 2: How-To Guides (`docs/how_to/`)

**Purpose**: Task-oriented, problem-solving recipes

**Use for**: Installation, configuration, troubleshooting

### Category 3: Explanations (`docs/explanation/`)

**Purpose**: Understanding-oriented, conceptual discussion

**Use for**: Architecture explanations, design decisions, **implementations.md**

**This is where you update your work summaries.**

### Category 4: Reference (`docs/reference/`)

**Purpose**: Information-oriented, technical specifications

**Use for**: API documentation, configuration reference, **architecture.md**

### Decision Tree: Where to Put Documentation?

```text
Implementation summary? → docs/explanation/implementations.md (UPDATE THIS)
Architecture/specs? → docs/reference/architecture.md (READ THIS)
Step-by-step tutorial? → docs/tutorials/
Solving specific task? → docs/how_to/
```

**Default**: Update `docs/explanation/implementations.md` with your work.

---

## Debugging Guide

### When Quality Checks Fail

**Systematic Process:**

```bash
# 1. Fix formatting first (auto-fixes most issues)
cargo fmt --all

# 2. Fix compilation errors
cargo check --all-targets --all-features
# Read error messages carefully, fix root cause

# 3. Fix clippy warnings one at a time
cargo clippy --all-targets --all-features -- -D warnings
# Fix first warning, re-run, repeat

# 4. Fix failing tests
cargo test --all-features -- --nocapture
# Read failure output, fix code or update test

# 5. Verify all pass
cargo fmt --all && \
cargo check --all-targets --all-features && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --all-features
```

### When Tests Fail

```bash
# Run with detailed output
cargo test -- --nocapture --test-threads=1

# Run specific test
cargo test test_name -- --nocapture

# Show backtrace
RUST_BACKTRACE=1 cargo test

# Debug logging
RUST_LOG=debug cargo test
```

**Debugging Strategy:**

1. Read the test failure message carefully
2. Understand what the test expects
3. Add `println!` or `dbg!` to see actual values
4. Fix the code or update the test
5. Re-run until passing

### When Clippy Reports Warnings

```bash
# List all warnings
cargo clippy --all-targets --all-features 2>&1 | grep "warning:"

# Fix by category:
# - Unused code: Remove or add #[allow(dead_code)] with comment
# - Complexity: Refactor or extract helpers
# - Style: Follow clippy suggestions
# - Correctness: Fix immediately (these are bugs)

# Re-run after each fix
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Validation Checklist

**BEFORE CLAIMING TASK IS COMPLETE, VERIFY ALL:**

### Code Quality

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows 0 warnings
- [ ] `cargo test --all-features` passes
- [ ] No `unwrap()` without justification
- [ ] All public items have `///` doc comments with examples

### Architecture Compliance

- [ ] Consulted `docs/reference/architecture.md` before starting
- [ ] Data structures match architecture definitions EXACTLY
- [ ] Module placement follows architecture structure
- [ ] Crate boundaries respected (xze-core → NO deps on cli/serve)
- [ ] Type names match architecture
- [ ] Configuration format follows architecture (.yaml for configs)
- [ ] No architectural deviations without documentation

### Testing

- [ ] Tests added for ALL new functions
- [ ] Success, failure, and edge cases covered
- [ ] Test names follow `test_{function}_{condition}_{expected}`
- [ ] Test count increased (verify with `cargo test --lib`)

### Documentation

- [ ] `docs/explanation/implementations.md` updated with summary
- [ ] NO new markdown files created (unless instructed)
- [ ] Filename is `lowercase_with_underscores.md` (if file was created)
- [ ] No emojis anywhere
- [ ] All code blocks specify language

### Files and Structure

- [ ] YAML files use `.yaml` (NOT `.yml`)
- [ ] Markdown files use `.md`
- [ ] No uppercase in filenames except `README.md`
- [ ] Files in correct crate (cli/serve/core)

### Git

- [ ] Branch: `pr-<description>-<issue>` (lowercase)
- [ ] Commit: `<type>(<scope>): <description>` (≤72 chars, imperative)

---

## Quick Command Reference

```bash
# Quality workflow (run before every commit)
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

# Build
cargo build                                    # Debug
cargo build --release                          # Optimized

# Testing
cargo test                                     # All tests
cargo test --lib                               # Library only
cargo test -- --nocapture                      # Show output
cargo test test_name                           # Specific test

# Documentation
cargo doc --open                               # Generate docs

# Maintenance
cargo clean                                    # Remove artifacts
cargo update                                   # Update dependencies
cargo audit                                    # Security check

# Dependencies
cargo add <crate>                              # Add dependency
cargo add <crate> --dev                        # Dev dependency
```

---

## Common Mistakes to Avoid

| Mistake                   | Why It Fails                  | Fix                                 |
| ------------------------- | ----------------------------- | ----------------------------------- |
| Using `.yml`              | CI expects `.yaml`            | Rename: `mv file.yml file.yaml`     |
| Creating separate MD docs | Should update implementations | Append to `implementations.md`      |
| Violating crate bounds    | Breaks architecture           | Check deps: xze-core → NO cli/serve |
| Skipping architecture.md  | Code doesn't match design     | Read architecture.md FIRST          |
| CamelCase docs            | Breaks links                  | Use `lowercase_with_underscores.md` |
| Forgetting `cargo fmt`    | CI formatting check fails     | Always run before commit            |
| Using `unwrap()`          | Panics in production          | Use `?` and proper error handling   |
| Missing doc comments      | CI may enforce                | Add `///` to all public items       |
| Ignoring clippy           | CI treats warnings as errors  | Fix all warnings                    |

---

## THE FIVE GOLDEN RULES - QUICK REFERENCE

**If you remember nothing else, remember these:**

### Golden Rule 1: Consult Architecture First

```text
Before ANY code:
1. Read docs/reference/architecture.md
2. Verify data structures match EXACTLY
3. Check crate boundaries
4. Use defined types, no deviations
```

### Golden Rule 2: File Extensions & Formats

```text
.yaml (NOT .yml) - for configs
.md (NOT .MD) - for docs
.rs - for code
lowercase_with_underscores.md - for filenames
```

### Golden Rule 3: Documentation Updates

```text
UPDATE: docs/explanation/implementations.md (append summary)
DO NOT: Create new markdown files (unless instructed)
ALWAYS: Add /// doc comments to public items
```

### Golden Rule 4: Quality Checks

```text
All four cargo commands MUST pass:
- cargo fmt --all
- cargo check --all-targets --all-features
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --all-features
```

### Golden Rule 5: Crate Boundaries

```text
xze-core → NO dependencies on xze-cli or xze-serve
Violation = architectural failure
Check architecture.md if unsure
```

---

## The Golden Workflow

**Follow this sequence for EVERY task:**

1. **Read** `docs/reference/architecture.md` sections relevant to task
2. **Create branch**: `pr-<description>-<issue>`
3. **Implement code** with `///` doc comments matching architecture
4. **Add tests** (success, failure, edge cases)
5. **Run four commands** (all must pass)
6. **Append summary** to `docs/explanation/implementations.md`
7. **Verify architecture compliance** (no deviations)
8. **Commit**: `<type>(<scope>): <description>`
9. **Verify checklist** (all items checked)

**If you follow this workflow precisely, your code will be accepted.**

**IF YOU SKIP STEPS OR VIOLATE RULES, YOUR CODE WILL BE REJECTED.**

---

## Living Document

Last updated: 2024

**For AI Agents**: You are a master Rust developer working on XZe. Follow these rules precisely:

1. **Always consult architecture.md first** - it is the source of truth
2. **Update implementations.md** - do NOT create separate docs
3. **Respect crate boundaries** - xze-core stays independent
4. **Run four commands** - all must pass before committing

Deviation from architecture = violation. When in doubt, check architecture.md.
