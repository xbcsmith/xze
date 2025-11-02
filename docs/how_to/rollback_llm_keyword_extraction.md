# Rollback LLM Keyword Extraction

This guide provides procedures for rolling back the LLM keyword extraction feature if issues arise in production.

## Quick Rollback Options

### Option 1: Disable via Environment Variable (Fastest)

This is the recommended approach for immediate rollback without code changes.

```bash
# Disable LLM extraction completely
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0

# OR disable the feature entirely
unset XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE
```

After setting the environment variable, restart the service:

```bash
# Systemd service
sudo systemctl restart xze-server

# Docker container
docker restart xze

# Kubernetes pod
kubectl rollout restart deployment/xze
```

**Recovery Time**: Less than 1 minute

**Impact**: New documents will use frequency-based extraction. Existing documents retain their keywords.

### Option 2: Staged Rollback (Gradual)

Reduce the rollout percentage incrementally to minimize impact:

```bash
# Reduce to 50%
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=50

# Wait 10 minutes, monitor metrics

# Reduce to 25%
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=25

# Wait 10 minutes, monitor metrics

# Disable completely
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0
```

**Recovery Time**: 30-60 minutes

**Impact**: Gradual reduction allows monitoring for specific issues.

### Option 3: Code Revert (Complete Rollback)

If environment variable rollback is insufficient, revert the code changes.

```bash
# Identify the commit to revert to
git log --oneline

# Revert to commit before Phase 3
git revert <commit-hash>

# Or checkout the previous stable tag
git checkout tags/v1.x.x

# Rebuild
cargo build --release

# Run tests
cargo test --all-features

# Deploy
make deploy
```

**Recovery Time**: 5-15 minutes

**Impact**: Complete removal of LLM extraction feature.

## Rollback Scenarios

### Scenario 1: High Error Rate

**Symptoms**:
- Error rate exceeds 5%
- Logs show repeated LLM failures
- Metrics show high failure count

**Action**:

```bash
# Immediate disable
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0
sudo systemctl restart xze-server

# Check logs for root cause
journalctl -u xze-server -n 100 | grep -i error

# Verify error rate drops
curl http://localhost:8080/metrics | grep keyword_extraction_error_rate
```

### Scenario 2: Performance Degradation

**Symptoms**:
- Average extraction time exceeds 2 seconds
- CPU usage increased by more than 50%
- Memory usage growing unbounded

**Action**:

```bash
# Reduce rollout to 10%
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=10
sudo systemctl restart xze-server

# Monitor for 15 minutes
watch -n 10 'curl -s http://localhost:8080/metrics | grep keyword_extraction'

# If still problematic, disable completely
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0
sudo systemctl restart xze-server
```

### Scenario 3: LLM Service Unavailable

**Symptoms**:
- Connection errors to Ollama
- Timeout errors in logs
- All LLM extractions failing

**Action**:

```bash
# Disable LLM extraction (automatic fallback to frequency-based)
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0

# Or rely on automatic fallback if enabled
# (fallback_on_error: true in config)

# Restart service
sudo systemctl restart xze-server

# Fix Ollama service
sudo systemctl restart ollama
```

### Scenario 4: Poor Quality Keywords

**Symptoms**:
- User reports of irrelevant keywords
- Search quality degraded
- Manual inspection shows hallucinated keywords

**Action**:

```bash
# Stage 1: Reduce to 25% while investigating
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=25
sudo systemctl restart xze-server

# Stage 2: Disable if issue confirmed
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0
sudo systemctl restart xze-server

# Investigate prompt engineering improvements
# Review sampled outputs for quality issues
```

## Data Rollback

### Reload Documents with Frequency-Based Keywords

If documents were loaded with LLM keywords and need to be regenerated:

```bash
# Disable LLM extraction
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0

# Option 1: Reload specific documents
xze load --force docs/specific_file.md

# Option 2: Reload entire documentation set
xze load --force --recursive docs/

# Option 3: Reload from database
xze db reload-keywords --method frequency
```

**Warning**: This will overwrite existing keywords. Ensure you have backups if needed.

### Database Backup and Restore

Before major rollbacks, ensure you have a recent backup:

```bash
# Create backup before rollback
xze db backup --output /backup/keywords_$(date +%Y%m%d_%H%M%S).sql

# If rollback causes data issues, restore from backup
xze db restore --input /backup/keywords_YYYYMMDD_HHMMSS.sql
```

## Verification After Rollback

### Check Service Health

```bash
# Verify service is running
systemctl status xze-server

# Check recent logs for errors
journalctl -u xze-server -n 50 --no-pager

# Test health endpoint
curl http://localhost:8080/health
```

### Verify Keyword Extraction Method

```bash
# Load a test document
xze load test_document.md

# Check extraction method in logs
journalctl -u xze-server | grep -i "extraction_method"

# Expected: "extraction_method": "frequency"
```

### Monitor Metrics

```bash
# Check error rate (should be near 0%)
curl -s http://localhost:8080/metrics | grep keyword_extraction_error_rate

# Check extraction method distribution
curl -s http://localhost:8080/metrics | grep keyword_extraction_method

# Check processing time
curl -s http://localhost:8080/metrics | grep keyword_extraction_duration
```

## Communication Template

When performing a rollback, communicate with stakeholders:

```text
Subject: LLM Keyword Extraction Rollback - [TIMESTAMP]

Status: ROLLBACK INITIATED

Issue: [Describe the problem: high error rate/performance/quality]

Action Taken:
- Disabled LLM keyword extraction at [TIME]
- Rolled back to frequency-based extraction
- Service restarted at [TIME]

Current Status:
- Service: HEALTHY
- Error Rate: [X%]
- Processing Time: [Y ms]

Impact:
- New documents will use frequency-based keywords
- Existing documents unchanged
- No data loss

Next Steps:
- Root cause analysis in progress
- Fix expected by [DATE/TIME]
- Will communicate re-enablement plan

Contact: [Your contact info]
```

## Preventing Future Rollbacks

### Pre-Rollout Checklist

Before enabling LLM extraction:

- [ ] All tests passing
- [ ] Benchmarks show acceptable performance
- [ ] Manual quality review completed
- [ ] Monitoring dashboards configured
- [ ] Rollback procedure documented and tested
- [ ] Team trained on rollback procedures
- [ ] Stakeholders informed of rollout schedule

### Gradual Rollout Strategy

Always use staged rollout:

1. Internal testing (0% - manual only)
2. Canary (10%)
3. Limited (25%)
4. Majority (50%)
5. Full (100%)

Wait at each stage and verify:
- Error rate < 1%
- Processing time < 2 seconds
- Quality metrics improved
- No user complaints

### Automated Rollback Triggers

Consider implementing automatic rollback if:

```yaml
triggers:
  - condition: error_rate > 5%
    duration: 5 minutes
    action: set_rollout_percentage(0)

  - condition: avg_processing_time_ms > 3000
    duration: 10 minutes
    action: set_rollout_percentage(25)

  - condition: cache_hit_rate < 20%
    duration: 15 minutes
    action: alert_team()
```

## References

- Configuration Guide: `docs/reference/keyword_extraction_configuration.md`
- Troubleshooting: `docs/how_to/troubleshoot_keyword_extraction.md`
- Architecture: `docs/explanations/keyword_extraction_architecture.md`
- Phase 3 Implementation: `docs/explanations/phase3_production_rollout_implementation.md`

## Support

If rollback procedures fail or issues persist:

1. Check logs: `journalctl -u xze-server -f`
2. Review metrics: `curl http://localhost:8080/metrics`
3. Contact team: [Team contact information]
4. Create incident: [Incident tracking URL]
