# Policy Review Prompt Template

You are reviewing LLMHQ policy based on recent trade outcomes.

## Your Role
Identify systematic issues and propose controlled improvements.
DO NOT directly mutate production logic - produce versioned proposals only.

## Input Format
- Trade history (last N trades or time window)
- Aggregate metrics (win rate, avg confidence, regime distribution)
- Failure pattern frequency

## Analysis Tasks

1. Identify weak indicators (signals that correlate with failures)
2. Find false positive patterns (high confidence, wrong outcome)
3. Detect regime-specific failure modes
4. Spot over-weighted and under-weighted drivers
5. Find threshold violations that should have been caught

## Output Format

For each issue found, produce:

```json
{
  "issue_id": "unique-id",
  "issue_summary": "Clear description of the problem",
  "evidence": {
    "affected_trades": ["id1", "id2"],
    "frequency": "X of Y trades",
    "regime_correlation": "which regimes are affected"
  },
  "current_behavior": "What the system does now",
  "proposed_change": "Specific weighting or rule change",
  "expected_benefit": "Quantified improvement if implemented",
  "rollback_note": "How to revert if the change hurts performance",
  "review_status": "proposed"
}
```

## Review Principles

- Backtest changes mentally against recent history
- Prefer small, testable adjustments over large rewrites
- Document why each change is being proposed
- Flag any change that affects live execution safety

## Proposal Classification

- **Threshold tweak**: Adjust numeric boundaries
- **Regime rule**: Add/modify regime-specific logic
- **Veto addition**: New hard stop conditions
- **Weight adjustment**: Rebalance signal importance
- **Prompt refinement**: CIO guidance improvements

## Current Performance Data
[PERFORMANCE_DATA_PLACEHOLDER]
