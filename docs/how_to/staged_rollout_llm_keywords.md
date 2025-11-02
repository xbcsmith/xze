# Staged Rollout Plan for LLM Keyword Extraction

This document provides a detailed plan for rolling out LLM-based keyword extraction to production in a controlled, gradual manner.

## Overview

The staged rollout minimizes risk by progressively increasing the percentage of documents using LLM extraction while continuously monitoring performance, quality, and stability.

## Rollout Stages

### Stage 0: Pre-Rollout Preparation

**Duration**: 1-2 days

**Objective**: Ensure all prerequisites are met before starting rollout.

#### Checklist

- [ ] All Phase 3 code merged and deployed to staging
- [ ] All tests passing (unit, integration, benchmarks)
- [ ] Monitoring dashboards configured and accessible
- [ ] Rollback procedures documented and tested
- [ ] Team trained on monitoring and rollback
- [ ] Stakeholders informed of rollout schedule
- [ ] LLM service (Ollama) healthy and scaled appropriately
- [ ] Database backups created
- [ ] Alerts configured for error thresholds

#### Configuration

```bash
# Staging environment
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0
export XZE_KEYWORD_EXTRACTION_AB_TEST=false
export XZE_KEYWORD_EXTRACTION_METRICS=true
```

#### Validation

```bash
# Deploy to staging
make deploy-staging

# Run smoke tests
cargo test --all-features

# Verify metrics endpoint
curl http://staging.xze:8080/metrics | grep keyword_extraction

# Load test documents manually
xze load --verbose test_documents/
```

**Decision Gate**: All prerequisites met, monitoring functional, team ready.

### Stage 1: Internal Testing (0% Rollout)

**Duration**: 1-2 days

**Objective**: Manual testing with controlled documents to verify functionality.

#### Configuration

```bash
# Production environment - disabled but ready
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0
export XZE_KEYWORD_EXTRACTION_AB_TEST=false
export XZE_KEYWORD_EXTRACTION_METRICS=true
```

#### Actions

1. Deploy to production with feature disabled
2. Manually test with internal documents
3. Verify monitoring and metrics collection
4. Test rollback procedure in production (dry run)
5. Review logs for any unexpected behavior

#### Testing Commands

```bash
# Load internal test documents
xze load --method llm internal_docs/sample_*.md

# Verify keywords generated
xze query "test keyword extraction"

# Check metrics
curl http://localhost:8080/metrics | jq '.keyword_extraction'
```

#### Success Metrics

- Zero errors during manual testing
- Metrics correctly reported
- Keywords generated successfully
- Monitoring dashboards showing data
- Rollback procedure works as expected

**Decision Gate**: Manual testing successful, no errors, team confident in stability.

### Stage 2: Canary Rollout (10%)

**Duration**: 2-3 days

**Objective**: Enable for 10% of documents to detect issues with minimal impact.

#### Configuration

```bash
# Enable for 10% of documents
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=10
export XZE_KEYWORD_EXTRACTION_AB_TEST=false
export XZE_KEYWORD_EXTRACTION_METRICS=true

# Restart service
sudo systemctl restart xze-server
```

#### Monitoring

Monitor every 4 hours for the first day, then daily:

```bash
# Error rate (target: < 1%)
curl -s http://localhost:8080/metrics | jq '.keyword_extraction.error_rate'

# Average processing time (target: < 2000ms)
curl -s http://localhost:8080/metrics | jq '.keyword_extraction.avg_extraction_time_ms'

# Cache hit rate (target: > 20%)
curl -s http://localhost:8080/metrics | jq '.keyword_extraction.cache_hit_rate'

# Method distribution
curl -s http://localhost:8080/metrics | jq '.keyword_extraction.method_breakdown'
```

#### Quality Check

Sample 20 documents enriched with LLM keywords:

```bash
# Export recent LLM-enriched documents
xze export --method llm --limit 20 --output canary_sample.json

# Manual review for quality
cat canary_sample.json | jq '.[] | {file: .file, keywords: .keywords}'
```

Review for:
- Relevance to document content
- No hallucinated keywords
- Appropriate keyword count (10-15)
- Better quality than frequency-based

#### Rollback Triggers

Automatically rollback if:
- Error rate > 5% for more than 5 minutes
- Average processing time > 3 seconds for more than 10 minutes
- More than 10 user complaints about search quality
- LLM service becomes unavailable

**Decision Gate**: Error rate < 1%, performance acceptable, no quality issues, team confident.

### Stage 3: Limited Rollout (25%)

**Duration**: 3-5 days

**Objective**: Expand to 25% to gather more performance data and user feedback.

#### Configuration

```bash
# Enable for 25% of documents
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=25

# Restart service
sudo systemctl restart xze-server
```

#### Enhanced Monitoring

Monitor daily, with deeper analysis:

```bash
# Daily metrics snapshot
date >> metrics_log.txt
curl -s http://localhost:8080/metrics | jq '.keyword_extraction' >> metrics_log.txt

# Performance trends
xze metrics analyze --days 7 --output performance_report.json

# Error analysis
xze logs analyze --level error --filter keyword_extraction
```

#### User Feedback Collection

If applicable, collect user feedback:
- Search result quality surveys
- Support ticket analysis
- Direct user interviews (sample of 5-10 users)

#### Quality Sampling

Sample 50 documents:

```bash
# Export larger sample
xze export --method llm --limit 50 --output limited_sample.json

# Compare with frequency-based
xze export --method frequency --limit 50 --output frequency_sample.json

# Run comparison analysis
xze analyze compare-keywords \
  --llm limited_sample.json \
  --frequency frequency_sample.json \
  --output comparison_report.json
```

#### Key Metrics to Track

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Error Rate | < 1% | TBD | |
| Avg Processing Time | < 2000ms | TBD | |
| Cache Hit Rate | > 20% | TBD | |
| Search Quality Score | > Baseline | TBD | |
| User Satisfaction | > 80% | TBD | |

**Decision Gate**: All metrics within target, user feedback positive, search quality improved by 10%+.

### Stage 4: Majority Rollout with A/B Test (50%)

**Duration**: 5-7 days

**Objective**: Run formal A/B test to statistically validate LLM extraction improves outcomes.

#### Configuration

```bash
# Enable 50% with A/B testing
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=50
export XZE_KEYWORD_EXTRACTION_AB_TEST=true

# Restart service
sudo systemctl restart xze-server
```

#### A/B Test Design

- **Control Group** (50%): Frequency-based extraction
- **Treatment Group** (50%): LLM-based extraction
- **Duration**: Minimum 7 days for statistical significance
- **Sample Size**: All documents loaded during period

#### Metrics Collection

```bash
# Export A/B test assignments
xze ab-test export --output ab_test_assignments.json

# Collect metrics per group
xze ab-test analyze --output ab_test_results.json
```

#### Statistical Analysis

Compare groups on:

**Performance Metrics**:
- Processing time per document
- Cache hit rate
- Error rate

**Quality Metrics**:
- Average keywords per document
- Keyword relevance score (if available)
- Search result click-through rate
- User satisfaction ratings

**Statistical Tests**:
- Two-sample t-test for continuous metrics
- Chi-square test for categorical metrics
- Significance level: p < 0.05

```python
# Example analysis script
import json
import scipy.stats as stats

with open('ab_test_results.json') as f:
    data = json.load(f)

control = data['control']['search_quality_scores']
treatment = data['treatment']['search_quality_scores']

t_stat, p_value = stats.ttest_ind(control, treatment)
print(f"T-statistic: {t_stat}, P-value: {p_value}")

if p_value < 0.05 and treatment.mean() > control.mean():
    print("LLM extraction shows statistically significant improvement")
```

#### Success Criteria for A/B Test

- Search quality improved by 15%+ (p < 0.05)
- Processing time increase acceptable (< 2x)
- Error rate remains < 1%
- User satisfaction increased or unchanged

**Decision Gate**: A/B test shows statistically significant improvement (p < 0.05), no significant negative impacts.

### Stage 5: Full Rollout (100%)

**Duration**: Ongoing

**Objective**: Enable LLM extraction for all documents, declare production stable.

#### Configuration

```bash
# Enable for 100% of documents
export XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=100
export XZE_KEYWORD_EXTRACTION_AB_TEST=false
export XZE_KEYWORD_EXTRACTION_METRICS=true

# Restart service
sudo systemctl restart xze-server
```

#### Monitoring Period

Monitor closely for 7 days after full rollout:

**Daily**:
- Error rate trends
- Processing time trends
- Cache performance
- User feedback

**Weekly**:
- Quality sampling (50 documents)
- Performance review meeting
- Incident review
- Optimization opportunities

#### Post-Rollout Tasks

- [ ] Document lessons learned
- [ ] Update runbooks and procedures
- [ ] Schedule prompt optimization review
- [ ] Plan Phase 4 optimizations
- [ ] Celebrate success with team

#### Declare Production Stable

After 7 days at 100% with:
- Error rate < 1%
- Performance within targets
- No critical incidents
- Positive user feedback

**Decision Gate**: Stable for 7 days, all metrics within targets, declare production stable.

## Monitoring Dashboard

Essential metrics to display:

### Real-Time Metrics

```yaml
dashboard:
  - panel: Rollout Status
    metric: current_rollout_percentage
    type: gauge
    thresholds: [0, 25, 50, 100]

  - panel: Error Rate
    metric: keyword_extraction_error_rate
    type: time_series
    alert_threshold: 1%

  - panel: Processing Time
    metric: keyword_extraction_avg_time_ms
    type: time_series
    alert_threshold: 2000ms

  - panel: Cache Hit Rate
    metric: keyword_extraction_cache_hit_rate
    type: time_series
    target: 20%

  - panel: Method Distribution
    metric: keyword_extraction_method_breakdown
    type: pie_chart
```

### Historical Trends

```yaml
trends:
  - Total documents enriched
  - LLM vs frequency usage over time
  - Error rate trend (7 days)
  - Performance trend (7 days)
  - Cache efficiency trend
```

## Rollback Decision Matrix

| Condition | Severity | Action | Timeline |
|-----------|----------|--------|----------|
| Error rate > 5% | Critical | Immediate rollback to 0% | < 5 min |
| Error rate 3-5% | High | Rollback to previous stage | < 15 min |
| Avg time > 3s | High | Rollback to previous stage | < 15 min |
| Avg time 2-3s | Medium | Investigate, reduce by 50% | < 1 hour |
| Cache hit < 10% | Medium | Investigate, continue monitoring | < 1 day |
| User complaints > 5 | Medium | Quality review, pause rollout | < 1 day |

## Communication Plan

### Internal Team

**Daily during rollout** (Stages 2-4):
- Metrics summary email
- Slack update with key numbers
- Any incidents or anomalies

**Weekly**:
- Rollout progress meeting
- Decision on proceeding to next stage
- Review of user feedback

### Stakeholders

**Before rollout**:
- Rollout plan and timeline
- Expected benefits and risks
- Monitoring approach

**At each stage gate**:
- Progress update
- Key metrics achieved
- Decision to proceed or hold

**After completion**:
- Final results and outcomes
- Lessons learned
- Next steps (Phase 4)

### Users (if applicable)

**Stage 2 (Canary)**:
- "We're testing improved keyword extraction on a small subset of documents"

**Stage 4 (A/B Test)**:
- "We're evaluating new keyword extraction technology. You may notice improved search results."

**Stage 5 (Full Rollout)**:
- "We've launched improved keyword extraction. Search results should be more relevant."

## Emergency Contacts

```yaml
contacts:
  - role: Technical Lead
    name: [Name]
    phone: [Phone]
    email: [Email]

  - role: DevOps Engineer
    name: [Name]
    phone: [Phone]
    email: [Email]

  - role: Product Owner
    name: [Name]
    phone: [Phone]
    email: [Email]
```

## Post-Rollout Review

After declaring production stable, conduct a review:

### What Went Well

Document successes and effective practices.

### What Could Be Improved

Identify challenges and areas for improvement.

### Metrics Summary

| Metric | Stage 2 | Stage 3 | Stage 4 | Stage 5 |
|--------|---------|---------|---------|---------|
| Error Rate | | | | |
| Avg Time (ms) | | | | |
| Cache Hit Rate | | | | |
| Search Quality | | | | |

### Lessons Learned

Document key learnings for future rollouts.

### Action Items

Create tasks for Phase 4 optimizations.

## References

- Configuration Guide: `docs/reference/keyword_extraction_configuration.md`
- Rollback Procedures: `docs/how_to/rollback_llm_keyword_extraction.md`
- Troubleshooting: `docs/how_to/troubleshoot_keyword_extraction.md`
- Phase 3 Implementation: `docs/explanations/phase3_production_rollout_implementation.md`
