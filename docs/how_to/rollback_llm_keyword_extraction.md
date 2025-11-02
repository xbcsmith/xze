# Rollback Procedures for LLM Keyword Extraction

This document provides step-by-step procedures for rolling back the LLM-based
keyword extraction feature in case of issues.

## Overview

XZe provides multiple rollback mechanisms with different recovery times:

| Method               | Recovery Time | Data Loss | Use Case           |
| -------------------- | ------------- | --------- | ------------------ |
| Environment Variable | < 1 minute    | None      | Quick disable      |
| Configuration Change | < 5 minutes   | None      | Planned rollback   |
| Code Rollback        | < 30 minutes  | None      | Critical issues    |
| Data Reload          | 1-24 hours    | None      | Corrupted keywords |

## Quick Rollback (Immediate)

### Method 1: Environment Variable Disable

The fastest way to disable LLM extraction without restarting services.

**Steps**:

```bash
# Disable LLM extraction immediately
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0

# Verify the change
echo $KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE
```

**Effect**: New document processing will use frequency-based extraction.
Existing cached keywords remain unchanged.

**Recovery Time**: Immediate (next document processed)

**Rollback This Change**:

```bash
# Re-enable LLM extraction
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=100
```

### Method 2: Service Restart with Disabled Feature

For containerized deployments where environment changes require restart.

**Docker**:

```bash
# Stop the service
docker-compose stop xze

# Update environment
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0

# Restart with new config
docker-compose up -d xze
```

**Kubernetes**:

```bash
# Update ConfigMap
kubectl patch configmap xze-keyword-config -n xze \
  -p '{"data":{"KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE":"0"}}'

# Restart pods to pick up config
kubectl rollout restart deployment/xze -n xze

# Verify rollout
kubectl rollout status deployment/xze -n xze
```

**Recovery Time**: 1-5 minutes

## Staged Rollback

### Gradual Reduction

If you need to reduce LLM usage gradually:

```bash
# Current: 100% LLM
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=100

# Step 1: Reduce to 50%
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=50
# Monitor for 1 hour

# Step 2: Reduce to 25%
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=25
# Monitor for 1 hour

# Step 3: Reduce to 10%
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=10
# Monitor for 1 hour

# Step 4: Disable completely
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0
```

**Use Case**: Non-critical issues, gradual migration back to baseline

## Configuration Rollback

### Programmatic Configuration Change

If configuration is managed in code:

**Before** (with LLM):

```rust
let config = KeywordExtractorConfig {
    rollout_percentage: 100,
    ab_test_enabled: true,
    metrics_enabled: true,
    ..Default::default()
};
```

**After** (rollback):

```rust
let config = KeywordExtractorConfig {
    rollout_percentage: 0,
    ab_test_enabled: false,
    metrics_enabled: true, // Keep metrics for monitoring
    ..Default::default()
};
```

**Steps**:

1. Update configuration file
2. Rebuild application: `cargo build --release`
3. Deploy updated binary
4. Restart services
5. Verify with metrics endpoint

**Recovery Time**: 5-15 minutes

## Code Rollback

### Git Revert

If the feature code is causing issues:

**Steps**:

```bash
# Find the commit that introduced the feature
git log --oneline --grep="keyword extraction"

# Option 1: Revert specific commits
git revert <commit-hash>

# Option 2: Revert merge commit
git revert -m 1 <merge-commit-hash>

# Rebuild
cargo build --release

# Run tests
cargo test --all-features

# Deploy
make deploy
```

**Recovery Time**: 15-30 minutes

### Checkout Previous Tag

Rollback to a known good version:

```bash
# List recent tags
git tag -l | tail -5

# Checkout previous version
git checkout tags/v0.4.0

# Create a new branch from this tag
git checkout -b rollback-keyword-extraction

# Rebuild and test
cargo build --release
cargo test --all-features

# Deploy
make deploy
```

**Recovery Time**: 20-30 minutes

## Data Rollback

### Clear Keyword Cache

If cached keywords are problematic:

```rust
use xze_core::keyword_extractor::{KeywordExtractor, KeywordExtractorConfig};

#[tokio::main]
async fn main() -> xze_core::Result<()> {
    let config = KeywordExtractorConfig::default();
    let extractor = KeywordExtractor::new(config)?;

    // Clear all cached keywords
    extractor.clear_cache().await;
    println!("Keyword cache cleared");

    Ok(())
}
```

**Effect**: Forces re-extraction for all future requests

**Recovery Time**: Immediate

### Reload Documents

If persisted keywords need regeneration:

**Steps**:

1. Disable LLM extraction:

```bash
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0
```

2. Clear existing keywords (if stored in database):

```sql
-- PostgreSQL example
UPDATE documents SET keywords = NULL;
-- Or delete and reload
DELETE FROM documents;
```

3. Reload documents:

```bash
# Using XZe CLI
cargo run --bin xze -- load --enhanced data/

# Or programmatically
cargo run --release --example reload_documents
```

**Recovery Time**: 1-24 hours (depends on corpus size)

**Data Loss**: None (keywords are regenerated)

## Rollback Decision Tree

```text
Is there an active incident?
├─ YES (Critical)
│  └─ Use Quick Rollback (Method 1 or 2)
│     └─ Set ROLLOUT_PERCENTAGE=0
└─ NO (Planned)
   ├─ Is it a feature issue?
   │  ├─ YES → Use Code Rollback
   │  └─ NO → Continue
   └─ Is it a data issue?
      ├─ YES → Use Data Rollback
      └─ NO → Use Staged Rollback
```

## Verification After Rollback

### Check Configuration

```bash
# Verify environment variables
echo "Rollout: $KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE"
echo "A/B Test: $KEYWORD_EXTRACTION_AB_TEST"
echo "Metrics: $KEYWORD_EXTRACTION_METRICS"
```

### Check Metrics

```rust
// Verify no LLM extractions are happening
let metrics = extractor.get_metrics().await;
assert_eq!(metrics.llm_extractions, 0);
assert!(metrics.frequency_extractions > 0);
```

### Check Logs

```bash
# Check for extraction method
grep "extraction method" /var/log/xze/app.log | tail -20

# Verify frequency method is used
# Should see: extraction_method: "frequency"
```

### Test Extraction

```rust
let keywords = extractor.extract("test content").await?;
assert_eq!(keywords.extraction_method, "frequency");
```

## Communication Templates

### Internal Team Notification

```text
Subject: LLM Keyword Extraction Rollback - [TIMESTAMP]

Team,

We are rolling back LLM keyword extraction due to [REASON].

Action: Set KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0
Status: In Progress / Complete
Impact: Documents will use frequency-based extraction
Duration: [EXPECTED DURATION]
Next Steps: [INVESTIGATION PLAN]

Metrics before rollback:
- LLM extraction rate: [X]%
- Error rate: [Y]%
- Avg extraction time: [Z]ms

Contact [NAME] with questions.
```

### Stakeholder Update

```text
Subject: Temporary Change to Keyword Extraction

Hi [STAKEHOLDER],

We've temporarily disabled the LLM-based keyword extraction feature
due to [HIGH-LEVEL REASON].

Impact: Minimal - keyword extraction continues using our proven
frequency-based method.

Timeline: We expect to re-enable the feature within [TIMEFRAME]
after addressing [ISSUE].

No action required on your part.

Best regards,
[NAME]
```

## Post-Rollback Actions

### Immediate (< 1 hour)

1. **Verify rollback success**:

   - Check metrics show 0% LLM extraction
   - Verify frequency extraction is working
   - Confirm no error rate increase

2. **Monitor system**:

   - Watch error logs
   - Check performance metrics
   - Verify user reports

3. **Document incident**:
   - Record time of rollback
   - Document reason
   - Note who performed rollback

### Short-term (1-24 hours)

1. **Root cause analysis**:

   - Investigate what caused the rollback
   - Collect relevant logs and metrics
   - Identify contributing factors

2. **Create fix plan**:

   - Document required changes
   - Estimate fix timeline
   - Plan testing approach

3. **Communicate status**:
   - Update team on findings
   - Provide ETA for fix
   - Set expectations for re-rollout

### Long-term (1-7 days)

1. **Implement fix**:

   - Make necessary code changes
   - Add tests for failure scenario
   - Update documentation

2. **Test thoroughly**:

   - Run full test suite
   - Perform integration tests
   - Conduct manual verification

3. **Plan re-rollout**:
   - Schedule staged rollout
   - Define monitoring plan
   - Prepare rollback procedure (again)

## Preventing Future Rollbacks

### Monitoring

Set up alerts before issues require rollback:

```yaml
# Example alert configuration
alerts:
  - name: HighKeywordExtractionErrors
    condition: error_rate > 5%
    action: Alert on-call engineer

  - name: SlowKeywordExtraction
    condition: avg_time > 5000ms
    action: Alert on-call engineer

  - name: HighLLMFallbackRate
    condition: fallback_rate > 20%
    action: Reduce rollout percentage
```

### Gradual Rollout

Always use staged rollout:

```bash
# Week 1: Canary
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=10

# Week 2: Limited
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=25

# Week 3: A/B Test
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=50

# Week 4: Full rollout
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=100
```

### Automated Rollback

Implement circuit breaker pattern:

```rust
// Future implementation example
if error_rate > THRESHOLD {
    warn!("Error rate {} exceeds threshold {}, disabling LLM extraction",
          error_rate, THRESHOLD);
    config.rollout_percentage = 0;
}
```

## Rollback Checklist

Use this checklist for any rollback:

- [ ] Identify rollback method (quick/staged/code/data)
- [ ] Notify team of rollback intention
- [ ] Execute rollback procedure
- [ ] Verify rollback success (check metrics)
- [ ] Monitor system for 1 hour post-rollback
- [ ] Document rollback in incident log
- [ ] Communicate status to stakeholders
- [ ] Begin root cause analysis
- [ ] Create fix plan with timeline
- [ ] Schedule re-rollout when ready

## Emergency Contacts

Maintain a list of who can execute rollbacks:

```text
Primary: [NAME] - [PHONE] - [EMAIL]
Secondary: [NAME] - [PHONE] - [EMAIL]
Manager: [NAME] - [PHONE] - [EMAIL]
On-call: [ROTATION] - [PAGER]
```

## See Also

- [Configuration Reference](../reference/keyword_extraction_configuration.md)
- [How to Configure LLM Keyword Extraction](configure_llm_keyword_extraction.md)
- [Troubleshooting Guide](troubleshoot_keyword_extraction.md)
- [Incident Response Playbook](../operations/incident_response.md) (if exists)
