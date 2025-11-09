# CLI Argument Conflict Fix

## Overview

This document describes the fix for a CLI argument short option conflict between the global `config` argument and the `search` command's `category` argument.

## Problem Description

### Issue

When running the `xze search` command, the application panicked with:

```
thread 'main' panicked at /Users/bsmith/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/clap_builder-4.5.48/src/builder/debug_asserts.rs:112:17:
Command search: Short option names must be unique for each argument, but '-c' is in use by both 'category' and 'config'
```

### Root Cause

Two arguments were configured to use the same short option `-c`:

1. **Global `config` argument** (in `src/main.rs`):
   ```rust
   /// Configuration file path
   #[arg(short, long, global = true)]
   config: Option<PathBuf>,
   ```

2. **Search command `category` argument** (in `crates/cli/src/commands/search.rs`):
   ```rust
   /// Filter by document category
   #[arg(short = 'c', long)]
   pub category: Option<String>,
   ```

Clap (the CLI parsing library) enforces unique short options across all arguments, including global arguments that apply to all subcommands.

## Solution

### Approach

Removed the short option from the `category` argument in the `search` command, leaving only the long form `--category`.

**Rationale**:
- The global `config` argument is more commonly associated with `-c` in CLI conventions
- The `config` argument is used across all commands (global), making it a higher priority
- The `category` argument can be accessed via `--category` without loss of functionality
- Most users will likely use tab completion for longer argument names anyway

### Implementation

**File**: `crates/cli/src/commands/search.rs`

**Before**:
```rust
/// Filter by document category
///
/// Only search within documents of the specified category.
/// Categories follow the Diataxis framework: tutorial, how_to,
/// explanation, reference.
#[arg(short = 'c', long)]
pub category: Option<String>,
```

**After**:
```rust
/// Filter by document category
///
/// Only search within documents of the specified category.
/// Categories follow the Diataxis framework: tutorial, how_to,
/// explanation, reference.
#[arg(long)]
pub category: Option<String>,
```

## Impact

### Breaking Changes

**Minor**: Users who were using `-c` for category filtering in the search command will need to update their scripts to use `--category` instead.

**Example**:
```bash
# Before (would have worked if not for the conflict)
xze search "query" -c tutorial

# After (required)
xze search "query" --category tutorial
```

### Non-Breaking

- The long form `--category` was always available and continues to work
- All other functionality remains unchanged
- Global `-c` for config continues to work across all commands

## Usage Examples

### Search Command

```bash
# Basic search (no category filter)
xze search "how to configure logging"

# Search with category filter (long form only)
xze search "installation steps" --category tutorial

# Using global config argument (short form works)
xze -c config.yaml search "deployment"

# Both together
xze -c config.yaml search "API usage" --category reference
```

### Help Output

The search command help now correctly shows:

```
Options:
  -n, --max-results <MAX_RESULTS>
          Maximum number of results to return
          [default: 10]

  -s, --min-similarity <MIN_SIMILARITY>
          Minimum similarity threshold (0.0 to 1.0)
          [default: 0.0]

      --category <CATEGORY>
          Filter by document category
          Categories follow the Diataxis framework: tutorial, how_to,
          explanation, reference.
```

Global options available to all commands:

```
Options:
  -v, --verbose
          Enable verbose logging

  -c, --config <CONFIG>
          Configuration file path

  -o, --output <OUTPUT>
          Output format (json, yaml, pretty)
          [default: pretty]
```

## Verification

### Build Verification

```bash
# Compile successfully
cargo build --release
# Output: Finished `release` profile [optimized]

# Check help works without panic
./target/release/xze search --help
# Output: Shows help text with --category option
```

### Quality Gates

All quality gates pass:

```bash
✅ cargo fmt --all
✅ cargo check --all-targets --all-features
✅ cargo clippy --all-targets --all-features -- -D warnings
```

### Runtime Verification

```bash
# Global config short option works
./target/release/xze -c config.yaml search "test"

# Category long option works
./target/release/xze search "test" --category tutorial --database-url postgresql://localhost/xze

# Both together
./target/release/xze -c config.yaml search "test" --category tutorial
```

## Alternative Solutions Considered

### Option 1: Change Global Config Short Option (Rejected)

**Approach**: Change `-c` to `-f` (for file) for the global config argument

**Pros**:
- Allows category to keep `-c`

**Cons**:
- Breaking change for existing users of global config
- `-c` is a standard convention for config files in many CLI tools
- Config is used more frequently than category filtering

### Option 2: Use Different Short Option for Category (Rejected)

**Approach**: Use `-t` (type) or `-k` (kategory) for category

**Pros**:
- Provides a short option for category

**Cons**:
- `-t` is commonly associated with "type" in different contexts
- `-k` is non-intuitive
- Adding more short options increases cognitive load

### Option 3: Remove Short Option from Category (Chosen)

**Approach**: Use only long form `--category`

**Pros**:
- Simple and clean solution
- No confusion with standard `-c` for config
- Long form is self-documenting
- Tab completion makes long form easy to use

**Cons**:
- Slightly more typing for users who don't use tab completion

## Best Practices for Future Development

### Avoiding Short Option Conflicts

1. **Check global arguments first**: Before adding short options to subcommand arguments, verify they don't conflict with global options

2. **Document short options**: Keep a reference list of used short options:
   - `-c` - config (global)
   - `-v` - verbose (global)
   - `-o` - output (global)
   - `-n` - max-results (search)
   - `-s` - min-similarity (search)
   - `-h` - help (clap default)

3. **Test with `--help`**: Always test help output to verify argument parsing:
   ```bash
   cargo run -- subcommand --help
   ```

4. **Prefer long forms for optional features**: Reserve short options for frequently used, core functionality

5. **Use clap's debug assertions**: Run in debug mode during development to catch conflicts early:
   ```bash
   cargo run --features clap/debug
   ```

## Related Documentation

- Main CLI: `src/main.rs`
- Search command: `crates/cli/src/commands/search.rs`
- Clap documentation: https://docs.rs/clap/latest/clap/

## Conclusion

The CLI argument conflict has been resolved by removing the short option from the `category` argument in the `search` command. This fix prioritizes the global `config` argument's use of `-c` while maintaining full functionality through the long form `--category` option.

**Status**: ✅ RESOLVED
**Impact**: Minor (long form `--category` required)
**Quality Gates**: All passing
**Production Ready**: YES
