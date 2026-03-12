---
name: llmhq-trading-engine
description: Design, review, and operate the LLMHQ BTC interval prediction workflow for Polymarket-style 5m and 15m markets using a hybrid Rust plus LLM architecture. Use this skill for market-state briefing, prediction review, post-mortem analysis, and controlled project runbook tasks. Do not use it for broad autonomous trading or ungated live execution.
user-invocable: true
disable-model-invocation: true
metadata: {"openclaw":{"requires":{"bins":["bash","date","jq","python3"],"env":["LLMHQ_MODE"]},"platforms":["linux","darwin"]}}
---

# LLMHQ Trading Engine

## Purpose

This skill is the operating runbook for **LLMHQ**, a hybrid quant-intelligence system for ultra-short-horizon BTC interval prediction.

The system is built around a strict separation of responsibilities:

- **Deterministic utilities** perform fast collection, feature calculation, timing checks, and logging.
- The **LLM acts as the CIO** and interprets structured evidence instead of raw market noise.
- A **retrospective coach loop** reviews outcomes, identifies failure modes, and proposes controlled policy improvements.

The primary objective is to predict whether the next **5-minute** or **15-minute** BTC interval will resolve **UP** or **DOWN**, with enough confidence to justify a decision made at or just before the interval boundary (`t=0`).

This skill is intended to support:

- market-state briefing generation,
- structured prediction review,
- dry-run and paper-trade workflows,
- post-mortem and ghost-trade analysis,
- policy review and controlled self-tuning,
- project setup and implementation guidance for the LLMHQ codebase.

This skill must **not** be used as a general-purpose autonomous trading agent.

## Safety stance

This skill is intentionally **manual-only**.

- It is **user-invocable** so it can be run explicitly.
- It is **not model-invocable** because live trading and trade-like actions are high-risk.
- Read-only analysis, dry-run execution, and post-mortem review are the default operating modes.
- Any live execution path must stay tightly gated behind explicit configuration and operator confirmation.
- Secrets must come from environment variables or config files, never from prompt text.
- Prompt or policy mutation must never be pushed directly into production without review.

If the requested task would place a real trade, change production execution settings, or bypass thresholds or guardrails, stop and require explicit user confirmation plus a clearly enabled live mode.

## When to use this skill

Use this skill when the user wants to:

- review or refine the LLMHQ architecture,
- generate a structured BTC market-state briefing,
- run a prediction workflow for a 5m or 15m interval,
- inspect the CIO decision payload,
- perform ghost-trade or post-mortem analysis,
- review regime-specific failures,
- propose controlled policy updates,
- package or operate the project as an OpenClaw-compatible workflow.

## When not to use this skill

Do not use this skill when the user wants:

- unrestricted automated live trading,
- portfolio management across many assets,
- silent policy mutation without audit trail,
- broad financial advice detached from the LLMHQ system,
- hidden write actions or secret handling in plain text.

## Core mental model

The system should always preserve the following mental model:

### 1. Analysts

Deterministic modules and narrow utilities generate observations.

Core analyst roles include:

- **Tape Reader**: interpret L2 depth and order-book pressure.
- **Momentum Engine**: compute short-horizon momentum features and divergence clues.
- **Whale Watcher**: detect large exchange inflows, liquidations, and other high-impact flow.
- **Social Sentiment Filter**: extract high-signal market sentiment from curated sources.
- **Microstructure Engine**: compute VPIN, imbalance velocity, liquidity voids, and related features.
- **Cross-Exchange Lead-Lag Monitor**: check whether spot, perp, or related venues are leading the move.
- **Liquidity Map / Void Detector**: identify thin zones, stop clusters, and air pockets.
- **Velocity Vector / Delta**: identify acceleration, exhaustion, grind, or spike behavior.
- **Correlation Checker**: identify confirmation or contradiction from related assets or venues.

### 2. CIO

The LLM is the **CIO**, not the calculator.

The CIO must:

- review the raw snapshot,
- review derived indicators,
- review the semantic narrative,
- verify whether the story matches the evidence,
- assess regime,
- apply veto logic,
- issue final direction and confidence,
- record rationale and risk flags.

### 3. Coach

The retrospective layer is the **Coach**.

The Coach must:

- compare forecast vs outcome,
- identify useful and misleading signals,
- detect regime-specific failures,
- classify ghost-trade success or failure,
- propose controlled policy improvements.

## Operating modes

The skill must respect `LLMHQ_MODE`.

### Allowed values

- `design` — architecture, project planning, spec refinement, runbook drafting.
- `analyze` — read-only market briefing and prediction support.
- `paper` — dry-run / simulated execution with logging.
- `review` — post-mortem, ghost-trade, and policy review.
- `live` — controlled live execution mode. Treat as high risk and require explicit user confirmation before any live-trade action.

If `LLMHQ_MODE` is missing, assume `design` unless the user explicitly requests analysis or review.

If `LLMHQ_MODE=live`, do **not** place or stage a live trade unless all guardrails in this skill are satisfied.

## Expected project layout

Prefer a project layout similar to:

```text
skills/
  llmhq-trading-engine/
    SKILL.md
    templates/
      cio_prompt.md
      postmortem_prompt.md
      policy_review_prompt.md
    examples/
      market_briefing.json
      decision_output.json
      ghost_trade_review.json
project/
  src/
    collector/
    features/
    microstructure/
    narrator/
    cio_client/
    executor/
    logger/
    postmortem/
    regime_classifier/
    policy_manager/
  config/
  scripts/
  logs/
```

## System architecture

Treat the workflow as:

**Observer → Synthesiser → CIO → Execution → Retrospective**

### Layer A: Real-time sensory array

Collect and normalize:

- OHLCV,
- spot and perp prices,
- L2 order book data,
- order-book imbalance,
- OBI velocity,
- liquidation events,
- whale transfer events,
- cross-exchange spread deltas,
- selected sentiment inputs.

### Layer B: Semantic synthesis

Convert raw and derived metrics into compact, consistent operational language.

Examples:

- Instead of `HMA14 slope = +12`, say: `Price is surfing the 14-period HMA and the slope is steepening with no obvious exhaustion.`
- Instead of `OBI = 0.80`, say: `Buy-side absorption is dominant and the sell book is unusually thin.`
- Instead of `VPIN = 0.82`, say: `Market toxicity is elevated and liquidity providers appear to be pulling back under informed pressure.`

The semantic layer must be:

- concise,
- consistent,
- auditable,
- suitable for logging,
- stable across repeated runs.

### Layer C: CIO decision core

The CIO receives three layers of evidence:

1. **Raw snapshot**
2. **Derived indicators**
3. **Semantic narrative**

The CIO must output a structured result with:

```json
{
  "direction": "UP or DOWN",
  "confidence": 0,
  "regime": "trending | ranging | volatile_expansion | quiet_compression | manipulative",
  "lead_driver": "OBI | VPIN | HMA | sentiment | whale_flow | cross_exchange | other",
  "rationale": "short explanation",
  "risk_flags": ["warnings"],
  "veto_applied": false,
  "veto_reason": ""
}
```

### Layer D: execution layer

The execution layer must remain deterministic.

It must:

- enforce timing discipline,
- reject stale decisions,
- verify confidence threshold,
- verify market/feed health,
- verify logging availability or fallback,
- support dry-run mode,
- measure latency,
- store the final pre-trade state.

### Layer E: retrospective loop

The retrospective layer must:

- store outcome,
- run ghost-trade analysis,
- review regime performance,
- generate lessons learned,
- propose controlled policy updates,
- version any policy or prompt change.

## Timing model

Preserve the timing discipline below.

### Recommended cycle

- `t-30s` to `t-15s`: compute features in parallel.
- `t-15s` to `t-10s`: aggregate raw snapshot and derived indicators.
- `t-10s` to `t-5s`: generate semantic market narrative.
- `t-5s` to `t-2s`: CIO reviews the dossier and issues direction plus confidence.
- `t-2s` to `t=0`: deterministic execution layer validates and stages the action.

The heavy cognitive work must happen **before** `t=0`. The system is trading the stored kinetic energy and structure of the prior block, not the new block's already-developed price action.

## Regime logic

The skill must preserve explicit **regime-based thresholding**.

Minimum regimes:

- `trending`
- `ranging`
- `volatile_expansion`
- `quiet_compression`
- `manipulative`

Interpret signals differently by regime.

Examples:

- In **trending** conditions, continuation signals may deserve more weight.
- In **ranging** conditions, mean-reversion and late fake-outs may deserve more weight.
- In **volatile_expansion**, microstructure and toxicity can outrank slower momentum tools.
- In **manipulative** conditions, late OBI spikes may reduce confidence rather than increase it.

Never apply one fixed weighting scheme to all regimes.

## Default workflow

Follow this workflow unless the user explicitly requests a narrower subtask.

### Step 1: classify request type

Map the request to one of:

- `design`
- `market_briefing`
- `prediction_review`
- `paper_execution`
- `ghost_trade_review`
- `policy_review`
- `live_execution_request`

### Step 2: determine allowed mode

Check `LLMHQ_MODE` and enforce the matching boundary.

- In `design`, only produce specifications, prompts, project structure, and runbooks.
- In `analyze`, produce read-only briefings and CIO decisions.
- In `paper`, allow simulated execution only.
- In `review`, allow post-mortem and policy analysis.
- In `live`, still require explicit confirmation and all guardrails before any live action.

### Step 3: build the briefing dossier

When producing or reviewing a prediction, organize inputs into:

#### Raw snapshot

Include items such as:

- timestamp,
- interval type,
- last price,
- best bid / ask,
- depth summary,
- perp premium,
- recent spread delta,
- liquidation markers.

#### Derived indicators

Include items such as:

- OBI,
- OBI velocity,
- HMA and HMA slope,
- VPIN or equivalent order-flow toxicity,
- volatility state,
- liquidity void presence,
- acceleration or deceleration,
- cross-exchange lead-lag clues,
- whale or sentiment alerts.

#### Semantic narrative

Produce a concise operational summary describing the likely structure of the next block.

### Step 4: CIO review

When acting as the CIO, always do all of the following:

1. verify whether the raw data supports the semantic narrative,
2. identify contradictions,
3. classify regime,
4. decide whether the setup implies continuation, reversion, or liquidity-hunt behavior,
5. apply veto logic,
6. emit the structured output format.

### Step 5: execution or non-execution path

#### If mode is `design`, `analyze`, or `review`

Do not place trades.

Return:

- the structured decision,
- rationale,
- risk flags,
- recommended next checks,
- logging fields that should be stored.

#### If mode is `paper`

Simulate execution only.

Return:

- planned entry time,
- intended direction,
- confidence,
- threshold check result,
- staleness check result,
- simulated latency notes,
- required log record.

#### If mode is `live`

A live path is only eligible if **all** of the following are true:

- the user explicitly requests a live run,
- a confidence threshold is configured,
- the decision is timely and non-stale,
- feeds are healthy,
- transaction path is healthy,
- durable logging is available,
- no hard veto is active,
- user confirmation is explicit in the current conversation.

If any requirement fails, stop and explain why the live path is blocked.

## Guardrails for live requests

For any request that could cause a real trade, enforce these rules:

1. Never infer permission for live trading from earlier context alone.
2. Never bypass confidence thresholds.
3. Never proceed if the decision is stale.
4. Never continue if market/feed health is unknown.
5. Never continue if logging is unavailable and no durable fallback exists.
6. Never hide uncertainty.
7. Never silently rewrite the active policy or prompt.
8. Never expose secrets in output.

## Ghost-trade analysis workflow

Use this workflow after a paper or live decision has enough post-entry data.

### Questions to answer

- Did the market initially confirm the thesis?
- Did it reverse immediately?
- Was the OBI signal genuine or a fake wall?
- Did a whale or liquidation event appear after the decision?
- Was HMA lagging during volatility expansion?
- Did end-of-block pinning create a false continuation signal?

### Required output

Return:

```json
{
  "classification": "success | failure | mixed",
  "likely_cause": "text",
  "lesson_learned": "text",
  "regime_adjustment": "text",
  "warning_rule": "optional text"
}
```

## Policy review workflow

When asked to review recent runs, evaluate:

- weak indicators,
- false positives,
- regime-specific failure patterns,
- over-weighted and under-weighted drivers,
- prompt or policy improvements.

Do **not** directly mutate production logic.

Instead, produce a **versioned proposal** containing:

- issue summary,
- evidence,
- proposed weighting or rule change,
- expected benefit,
- rollback note.

## Logging requirements

Every prediction or simulated/live decision should store at least:

- timestamp,
- interval type,
- market regime,
- raw snapshot hash or reference,
- semantic narrative,
- prediction,
- confidence,
- lead driver,
- veto notes,
- outcome,
- execution latency,
- slippage or timing details,
- retrospective lesson if generated.

The exact semantic narrative sent to the CIO is a first-class artifact and must be stored.

## Output requirements

Prefer structured, audit-friendly output.

### For market briefing and prediction tasks

Return this shape:

```json
{
  "request_type": "market_briefing | prediction_review",
  "interval": "5m | 15m",
  "raw_snapshot": {},
  "derived_indicators": {},
  "semantic_narrative": "",
  "cio_decision": {
    "direction": "UP or DOWN",
    "confidence": 0,
    "regime": "",
    "lead_driver": "",
    "rationale": "",
    "risk_flags": [],
    "veto_applied": false,
    "veto_reason": ""
  },
  "execution_status": "not_applicable | simulated | blocked | ready_for_manual_confirmation",
  "logging_notes": {}
}
```

### For design and implementation tasks

Return:

- the recommended module or file structure,
- required interfaces,
- assumptions,
- unresolved dependencies,
- test or validation notes.

## Implementation guidance

Prefer a Rust-first implementation for:

- WebSocket streaming,
- concurrent feature calculation,
- buffer management,
- precise timing,
- deterministic execution,
- database logging,
- outcome reconciliation.

Recommended internal modules:

- `collector`
- `features`
- `microstructure`
- `narrator`
- `cio_client`
- `executor`
- `logger`
- `postmortem`
- `regime_classifier`
- `policy_manager`

## Practical command pattern

If the repository already provides a project binary or wrapper scripts, prefer those.

Suggested interface pattern:

```bash
./scripts/llmhq briefing --interval 5m
./scripts/llmhq predict --interval 15m --dry-run
./scripts/llmhq postmortem --trade-id <id>
./scripts/llmhq policy-review --window 24h
```

If the project is not yet implemented, stay in `design` mode and produce interface specs rather than inventing fake execution results.

## Stop conditions

Stop and explain the block if any of these apply:

- required inputs are missing,
- mode does not permit the requested action,
- confidence threshold is undefined for execution,
- decision timing is stale,
- market/feed health is unknown,
- logging path is unavailable,
- live execution is requested without explicit current confirmation,
- the request would require silent policy mutation.

## Success criteria

Treat the workflow as successful when it can:

- generate a reproducible market-state briefing,
- produce an auditable `UP` or `DOWN` forecast for 5m and 15m windows,
- explain the lead driver and confidence,
- log decisions and outcomes reliably,
- identify recurring failure modes,
- propose sensible policy improvements,
- stay safely packaged behind OpenClaw guardrails.

## Style rules for this skill

- Be operational, not poetic.
- Prefer concise technical language.
- Distinguish facts from inference.
- State uncertainty clearly.
- Preserve auditability.
- Prefer read-only and dry-run behavior unless the user explicitly requests otherwise and the mode permits it.
