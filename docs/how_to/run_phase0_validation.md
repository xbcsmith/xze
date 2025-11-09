# How To Run Phase 0 Validation

## Overview

This guide explains how to execute Phase 0 (Research and Validation) of the LLM-based keyword extraction feature.

**Prerequisites**:
- Rust toolchain installed
- Ollama running locally with llama3.2:3b model
- XZe repository cloned

## Step 1: Setup Ollama

Install and start Ollama with the required model:

```bash
# Install Ollama (if not already installed)
# Visit https://ollama.ai for installation instructions

# Pull the model
ollama pull llama3.2:3b

# Start Ollama server (in separate terminal)
ollama serve
```

Verify Ollama is running:

```bash
curl http://localhost:11434/api/version
```

Expected output: `{"version":"0.x.x"}`

## Step 2: Build the Examples

```bash
cd xze

# Format code
cargo fmt --all

# Build examples
cargo build --examples

# Verify examples compile
cargo check --examples
```

## Step 3: Run the Prototype LLM Extractor

Extract keywords from documentation using LLM:

```bash
cargo run --example prototype_llm_extractor -- \
    --input docs/explanation/ \
    --output llm_keywords.json \
    --model llama3.2:3b
```

**Expected Output**:

```
INFO  LLM Keyword Extraction Prototype
INFO  ================================
INFO  Input directory: "docs/explanation/"
INFO  Output file: "llm_keywords.json"
INFO  Model: llama3.2:3b
INFO
INFO  Processing: implement_llm_keyword_extraction.md
INFO  ✓ implement_llm_keyword_extraction.md (1250 ms)
INFO  Processing: query_enhancement_implementation_plan.md
INFO  ✓ query_enhancement_implementation_plan.md (1380 ms)
INFO
INFO  Extraction Summary
INFO  ==================
INFO  Total documents: 5
INFO  Successful: 5
INFO  Failed: 0
INFO  Success rate: 100.0%
INFO  Average extraction time: 1250 ms
INFO  Total time: 6250 ms
INFO
INFO  Results saved to: "llm_keywords.json"
```

## Step 4: Review Extraction Results

Examine the extracted keywords:

```bash
# View summary
cat llm_keywords.json | jq '.summary'

# View first result
cat llm_keywords.json | jq '.results[0]'

# Count successful extractions
cat llm_keywords.json | jq '.summary.successful'
```

**Expected Summary**:

```json
{
  "total_documents": 5,
  "successful": 5,
  "failed": 0,
  "avg_extraction_time_ms": 1250,
  "total_time_ms": 6250
}
```

## Step 5: Measure Search Quality

Run baseline search quality measurement:

```bash
cargo run --example measure_search_quality -- \
    --queries test_queries.json \
    --output baseline_metrics.json \
    --top-k 5
```

**Expected Output**:

```
INFO  Search Quality Measurement Tool
INFO  ================================
INFO  Queries file: "test_queries.json"
INFO  Output file: "baseline_metrics.json"
INFO  Top K: 5
INFO
INFO  Loaded 50 test queries
INFO
INFO  Metrics Summary
INFO  ===============
INFO  Total queries: 50
INFO  Avg Precision@5: 0.720
INFO  Avg Recall@5: 0.680
INFO  Mean Reciprocal Rank: 0.810
INFO  Zero-result rate: 4.0%
INFO  Queries with results: 48
INFO  Avg results per query: 4.8
INFO
INFO  Metrics saved to: "baseline_metrics.json"
```

## Step 6: Review Search Metrics

```bash
# View metrics summary
cat baseline_metrics.json | jq '.summary'

# View metrics for specific query
cat baseline_metrics.json | jq '.query_metrics[0]'

# Check zero-result queries
cat baseline_metrics.json | jq '[.query_metrics[] | select(.has_zero_results == true)]'
```

## Step 7: Run Quality Gates

Ensure all code quality checks pass:

```bash
# Format code
cargo fmt --all

# Check compilation
cargo check --all-targets --all-features

# Lint with zero warnings
cargo clippy --examples -- -D warnings

# Run tests
cargo test --example prototype_llm_extractor
cargo test --example measure_search_quality
```

**All checks must pass with zero errors and zero warnings.**

## Step 8: Manual Quality Review

Review a sample of extracted keywords for quality:

1. Open `llm_keywords.json`
2. Compare extracted keywords against source documents
3. Verify categorization (keywords, phrases, acronyms, tools, commands)
4. Check for missing important terms
5. Check for hallucinated or irrelevant terms

**Quality Assessment Criteria**:
- Are important technical terms captured?
- Are domain-specific phrases identified?
- Are acronyms correctly expanded?
- Are tool names and commands recognized?
- Is there minimal noise (irrelevant terms)?

## Troubleshooting

### Ollama Connection Error

**Problem**: `Failed to send request to Ollama`

**Solution**:
```bash
# Check Ollama is running
curl http://localhost:11434/api/version

# Restart Ollama
killall ollama
ollama serve
```

### Model Not Found

**Problem**: `model not found`

**Solution**:
```bash
# Pull the model
ollama pull llama3.2:3b

# List available models
ollama list
```

### Compilation Errors

**Problem**: `cargo build` fails

**Solution**:
```bash
# Update dependencies
cargo update

# Clean build
cargo clean
cargo build --examples
```

### Test Failures

**Problem**: Tests fail

**Solution**:
```bash
# Run tests with detailed output
cargo test --example prototype_llm_extractor -- --nocapture

# Check for missing dependencies
cargo check --examples
```

## Success Criteria

Phase 0 validation is successful if:

- [ ] LLM extraction succeeds on 95%+ of test documents
- [ ] Average extraction time < 3 seconds per document
- [ ] Extracted keywords are relevant and comprehensive
- [ ] JSON output validates correctly
- [ ] All quality gates pass
- [ ] Baseline metrics established for comparison

## Next Steps

After successful Phase 0 validation:

1. **Quality Comparison** (Task 0.3):
   - Run on 100 representative documents
   - Compare against frequency-based extraction
   - Calculate improvement metrics

2. **Decision Gate** (Task 0.4):
   - Review results with stakeholders
   - Obtain approval for Phase 1 if criteria met

3. **Phase 1 Planning**:
   - Create feature branch: `pr-llm-keyword-core-XZE-2002`
   - Plan core implementation sprint
   - Schedule team capacity

## Additional Commands

### Custom Model Testing

Test with different Ollama models:

```bash
cargo run --example prototype_llm_extractor -- \
    --input docs/ \
    --output results_mistral.json \
    --model mistral:7b
```

### Large Document Set

Process all documentation:

```bash
cargo run --example prototype_llm_extractor -- \
    --input docs/ \
    --output all_keywords.json \
    --max-content-length 3000
```

### Custom Top-K Metrics

Measure different top-K values:

```bash
cargo run --example measure_search_quality -- \
    --queries test_queries.json \
    --output metrics_k10.json \
    --top-k 10
```

## Documentation References

- Implementation plan: `docs/explanation/implement_llm_keyword_extraction.md`
- Phase 0 summary: `docs/explanation/phase0_llm_keyword_extraction_implementation.md`
- Project guidelines: `AGENTS.md`
- Test queries: `test_queries.json`

## Support

For issues or questions:

1. Check AGENTS.md for development guidelines
2. Review implementation plan for detailed specifications
3. Examine example code for usage patterns
4. Run quality gates to identify specific errors
