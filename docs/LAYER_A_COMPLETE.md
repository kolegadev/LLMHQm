# LLMHQ Layer A - User Configuration Complete ✅

## Your Choices Implemented

### Q1: Whale Watcher - Binance @forceOrder ✅
- **Data Source**: Binance Futures WebSocket (@forceOrder)
- **Cost**: FREE
- **Data**: Real-time liquidations
- **Tracks**: 
  - Long liquidations (USD value)
  - Short liquidations (USD value)
  - Net liquidation pressure
  - Individual liquidation events

### Q2: Cross-Exchange - Binance Spot + Perp ✅
- **Data Source**: Binance Spot + Futures WebSocket
- **Cost**: FREE
- **Data**: 
  - Spot price (BTCUSDT)
  - Perp mark price (BTCUSDT)
  - Index price
  - Funding rate
  - Basis (spot-perp spread in bps)

### Q3: Correlation - Multi-Asset Stream ✅
- **Data Source**: Binance Multi-Stream WebSocket
- **Cost**: FREE
- **Assets**: BTC, ETH, XRP, SOL, MATIC (Polygon)
- **Calculates**: Pearson correlation vs BTC in real-time

---

## Architecture Overview

```
Layer A - Real-Time Sensory Array
├── Spot Streams (wss://stream.binance.com:9443/ws)
│   ├── btcusdt@trade
│   ├── ethusdt@trade
│   ├── xrpusdt@trade
│   ├── solusdt@trade
│   └── maticusdt@trade
├── Futures Streams (wss://fstream.binance.com/ws)
│   ├── btcusdt@forceOrder (liquidations)
│   └── btcusdt@markPrice@1s (perp data)
└── Calculators
    ├── Tape Reader (OBI from order book)
    ├── Momentum Engine (HMA, RSI)
    ├── Microstructure (VPIN, volatility)
    ├── Whale Watcher (liquidations)
    ├── Cross-Exchange (basis)
    ├── Correlation (BTC vs alts)
    └── Liquidity Map (voids, walls)
```

---

## Running Layer A

### Quick Test (REST API - no WebSocket)
```bash
cd /root/.openclaw/workspace/LLMHQm-work/src
python3 collector.py
```

### Run Enhanced Layer A (All 8 Analysts)
```bash
cd /root/.openclaw/workspace/LLMHQm-work/src
python3 layer_a_enhanced.py
```

You'll see output like:
```
======================================================================
[Layer A Summary] 05:45:30 UTC
======================================================================

💰 PRICES:
   BTC: $70,270.75
   ETH: $3,850.20
   XRP: $0.62
   SOL: $145.30
   MATIC: $0.85

⚡ BASIS:
   Spot: $70,270.75
   Perp: $70,295.50
   Spread: +3.5 bps

🚨 LIQUIDATIONS (1m):
   Longs liquidated: $125,000
   Shorts liquidated: $45,000
   Net pressure: -$80,000 (shorts being liquidated = bullish)

🔗 CORRELATIONS (vs BTC):
   ETH: +0.85
   SOL: +0.72
   XRP: +0.45
   MATIC: +0.68
```

---

## Next Step: Paper Trading Setup

Now that Layer A is complete, we need to build:

1. **Layer B**: Semantic Synthesis (convert numbers to narrative)
2. **Layer C**: CIO Decision Core (LLM makes prediction)
3. **Layer D**: Paper Execution (simulated trades with logging)
4. **Layer E**: Retrospective Loop (ghost-trade analysis)

### Paper Trading Requirements

For Polymarket BTC UP/DOWN 5M:
- Need to track 5-minute intervals
- Make decision at t=0 (block start)
- Predict UP or DOWN for next 5 minutes
- Log prediction and outcome
- Calculate P&L

### Data Feeds Needed

| Data | Source | Status |
|------|--------|--------|
| BTC Price | Binance | ✅ Ready |
| 5m Interval Timing | Internal | ⏳ Need to build |
| Polymarket Resolution | Manual/API | ⏳ Need to check |

---

## Polymarket Integration Options

### Option A: Manual Paper Trading
- You manually check Polymarket
- System provides prediction
- You log outcome after 5 minutes
- Simplest to start

### Option B: API Integration
- Polymarket has a GraphQL API
- Requires wallet connection
- Can fetch market data programmatically
- More complex but fully automated

**Which approach do you prefer for paper trading?**

A) Manual - System predicts, you check/confirm outcome
B) API - Full automation (requires Polymarket API setup)

---

## File Reference

| File | Purpose |
|------|---------|
| `src/layer_a_enhanced.py` | Main enhanced collector |
| `src/layer_a_collector.py` | Base 5-analyst collector |
| `src/analysts_external.py` | External data analysts |
| `src/collector.py` | REST API collector |
| `src/features.py` | Indicator calculations |
| `src/cio_client.py` | CIO workflow client |
| `src/paper_executor.py` | Paper trading execution |
