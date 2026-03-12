# LLMHQ Layer D: Paper Trading Executor - Complete ✅

## Overview

Layer D executes paper trades based on CIO decisions, with strict validation against Polymarket odds at t=0.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        LAYER D                                   │
│                 Paper Trading Executor                           │
│                                                                  │
│  ┌─────────────────┐    ┌─────────────────┐                     │
│  │ Price Tracker   │    │ Polymarket API  │                     │
│  │ (Binance WS)    │    │ (Gamma API)     │                     │
│  └────────┬────────┘    └────────┬────────┘                     │
│           │                      │                               │
│           ▼                      ▼                               │
│  ┌─────────────────────────────────────────┐                    │
│  │         PaperExecutor                   │                    │
│  │  ┌─────────────────────────────────┐   │                    │
│  │  │ 1. Capture t=0 price (Binance)  │   │                    │
│  │  │ 2. Fetch Polymarket odds        │   │                    │
│  │  │ 3. Validate odds vs direction   │   │                    │
│  │  │    - YES: odds >= 0.505         │   │                    │
│  │  │    - NO:  odds <= 0.495         │   │                    │
│  │  │ 4. Veto if contradiction        │   │                    │
│  │  │ 5. Execute paper trade          │   │                    │
│  │  │ 6. Monitor to t=300/t=900       │   │                    │
│  │  │ 7. Resolve + P&L calc           │   │                    │
│  │  └─────────────────────────────────┘   │                    │
│  └─────────────────────────────────────────┘                    │
│           │                                                      │
│           ▼                                                      │
│  ┌─────────────────────────────────────────┐                    │
│  │        TimescaleDB                      │                    │
│  │  - paper_trades table                   │                    │
│  │  - Trade lifecycle tracking             │                    │
│  └─────────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────┘
```

## Key Components

### 1. PriceTracker
Captures prices from Binance WebSocket:
- **t=0 reference**: Price at block start (trade entry)
- **Resolution**: Price at t=300 (5m) or t=900 (15m)

```rust
pub struct PriceTracker {
    current_price: Option<Decimal>,     // Live from WebSocket
    t0_price: Option<Decimal>,          // Captured at block start
    resolution_price: Option<Decimal>,  // Captured at block end
}
```

### 2. OddsValidator
Ensures Polymarket odds align with predicted direction:

| Prediction | Required Odds | Example |
|------------|---------------|---------|
| UP (YES) | >= 0.505 | YES at 0.52 ✅ |
| UP (YES) | < 0.505 | YES at 0.49 ❌ VETO |
| DOWN (NO) | <= 0.495 | YES at 0.48 ✅ |
| DOWN (NO) | > 0.495 | YES at 0.52 ❌ VETO |

**Why this matters**: If we predict UP but Polymarket shows YES at 0.49, the market disagrees with our prediction. Trading against the market odds reduces expected value.

### 3. PolymarketClient
Fetches real-time odds from Gamma API:
```rust
pub async fn get_current_odds(
    &self,
    market_id: &str
) -> Result<PolymarketOdds>
```

API endpoint: `https://gamma-api.polymarket.com/markets/{id}`

### 4. PaperExecutor
Manages full trade lifecycle:
- Balance tracking
- Position sizing
- Trade execution
- P&L calculation
- Database persistence

## Configuration

```rust
PaperTradingConfig {
    block_duration_secs: 300,      // 5 minutes (or 900 for 15m)
    yes_odds_threshold: 0.505,     // Minimum YES odds for UP bet
    no_odds_threshold: 0.495,      // Maximum YES odds for DOWN bet
    initial_balance: $10,000,      // Starting paper money
    max_position_pct: 95,          // Max % of balance per trade
    validate_odds: true,           // Enable odds validation
    polymarket_market_id: "...",   // Target market ID
}
```

## Trade Execution Flow

### Phase 1: t=0 (Block Start)
```
1. BlockTimer triggers execution window (t-2s to t=0)
2. Capture Binance BTC price → t0_price
3. Fetch Polymarket odds via Gamma API
4. Validate odds match direction
5. If valid: Execute paper trade
6. Store: entry_price, odds, position_size
```

### Phase 2: Monitoring (t=0 to t=300/900)
```
- Active trade tracked in memory
- WebSocket continues updating current_price
- No action until block end
```

### Phase 3: Resolution (t=300 or t=900)
```
1. BlockTimer triggers resolution
2. Capture Binance BTC price → resolution_price
3. Calculate outcome:
   - UP prediction: WIN if price_up, LOSS if price_down
   - DOWN prediction: WIN if price_down, LOSS if price_up
4. Calculate P&L %
5. Update balance
6. Store outcome in database
```

## Database Schema

```sql
CREATE TABLE paper_trades (
    id UUID PRIMARY KEY,
    block_number BIGINT NOT NULL,
    decision_time TIMESTAMPTZ,
    entry_time TIMESTAMPTZ,
    direction TEXT CHECK (direction IN ('UP', 'DOWN')),
    confidence INTEGER,
    entry_price DECIMAL(18,8),
    exit_price DECIMAL(18,8),
    exit_time TIMESTAMPTZ,
    outcome TEXT CHECK (outcome IN ('WIN', 'LOSS', 'BREAKEVEN')),
    pnl_pct DECIMAL(8,4),
    polymarket_odds JSONB,  -- YES price at entry
    narrative_md TEXT
);
```

## Example Trade Lifecycle

### Block #5911234

**t=0 (Decision)**
```
CIO Decision: UP @ 78% confidence
Lead Driver: Heavy_Buy_Absorption

Binance t=0 Price: $70,250
Polymarket YES Odds: 0.52 (passes >= 0.505 threshold)

→ Trade APPROVED
→ Position Size: $7,800 (78% of $10k balance)
→ Entry: YES at $0.52 implied probability
```

**t=300 (Resolution)**
```
Binance Resolution Price: $70,485
Price Change: +0.33%

→ OUTCOME: WIN
→ P&L: +0.33% (simplified, actual uses position sizing)
→ New Balance: $10,025.74
```

## Odds Validation Scenarios

### Scenario 1: Aligned Odds ✅
```
Prediction: UP
Polymarket YES: 0.52
Threshold: 0.505
Result: VALID - Execute trade
```

### Scenario 2: Contradictory Odds ❌
```
Prediction: UP  
Polymarket YES: 0.49
Threshold: 0.505
Result: VETO - "YES odds 0.490 below threshold 0.505"
```

### Scenario 3: Market Disagreement on DOWN ❌
```
Prediction: DOWN
Polymarket YES: 0.55
Threshold: 0.495
Result: VETO - "YES odds 0.550 above threshold 0.495"
```

## Risk Controls

1. **Odds Validation**: Can't trade against market-implied probabilities
2. **Spread Check**: Wide spreads (>5%) indicate illiquidity → potential veto
3. **Stale Price Check**: t=0 price must be recent (<2s old)
4. **Max Position**: No single trade >95% of balance

## Future Enhancements

### Layer E: Retrospective Analysis
- Ghost-trade analysis (what happened in first 60s)
- Pattern library refinement
- Odds prediction accuracy tracking
- PIN success rate statistics

### Layer F: Policy Learning (RLAIF)
- Automated threshold tuning
- Pattern weight optimization
- Regime-specific rule adjustments

## Notes

### HIGH_BREAK Pinning Strategy Review
Current implementation vetoes HIGH_BREAK pinning. However, data shows certain conditions yield 75-80% PIN success. Future iteration should:
- Track PIN success rate per regime
- When probability >70%, switch to "piggyback" strategy
- Bet WITH the manipulation, not against it

### Multi-Market Support
Current implementation targets single market. Future:
- Support multiple concurrent markets
- Cross-market arbitrage detection
- Portfolio-level risk management
