# LLMHQ Layer A Configuration Guide

## Status: 5/8 Analysts Fully Operational (FREE)

### ✅ Fully Implemented (Binance WebSocket - FREE)

| Analyst | Data Source | Status | Description |
|---------|-------------|--------|-------------|
| **Tape Reader** | Binance WebSocket | ✅ Active | OBI, spread, pressure |
| **Momentum Engine** | Binance WebSocket | ✅ Active | HMA, slope, ROC, RSI |
| **Microstructure** | Binance WebSocket | ✅ Active | VPIN, volatility |
| **Liquidity Map** | Binance WebSocket | ✅ Active | Voids, walls |
| **Social Sentiment** | Alternative.me | ✅ Active | Fear & Greed Index |

### ⏳ Needs Your Configuration

| Analyst | Options | Recommended | Cost |
|---------|---------|-------------|------|
| **Whale Watcher** | Coinglass / Glassnode / Binance @forceOrder | **Coinglass** | Free tier |
| **Cross-Exchange** | Binance Spot+Perp / CryptoCompare | **Binance** | Free |
| **Correlation** | Binance Multi-Stream / Yahoo Finance | **Binance** | Free |

---

## Data Source Details

### 1. WHALE WATCHER - Choose One:

**Option A: Coinglass** ⭐ RECOMMENDED
- URL: https://coinglass.com/
- Free tier: Available with limits
- Data: Liquidations, exchange flows, funding rates
- API: https://open-api.coinglass.com/

**Option B: Binance @forceOrder Stream** (FREE, simpler)
- Already using Binance WebSocket
- Just add `@forceOrder` stream
- Data: Real-time liquidations only
- No exchange flow data

**Option C: Glassnode**
- URL: https://glassnode.com/
- Free tier: 30 API calls/day
- Data: Exchange inflows/outflows, on-chain metrics
- API Key required

### 2. CROSS-EXCHANGE MONITOR - Choose One:

**Option A: Binance Spot + Perp** ⭐ RECOMMENDED
- Subscribe to both streams simultaneously
- BTCUSDT (spot) + BTCUSDT_PERP (perp)
- Calculate basis in real-time
- Completely FREE

**Option B: CryptoCompare**
- URL: https://min-api.cryptocompare.com/
- Free tier: Limited calls
- Data: Prices across multiple exchanges

### 3. CORRELATION CHECKER - Choose One:

**Option A: Binance Multi-Stream** ⭐ RECOMMENDED
- Subscribe to BTCUSDT, ETHUSDT, SOLUSDT
- Calculate correlation in real-time
- Completely FREE

**Option B: Yahoo Finance (yfinance)**
- pip install yfinance
- Data: Crypto + traditional markets
- Good for BTC-SPX, BTC-DXY correlations

---

## Quick Start

### Test Basic Collector (REST API)
```bash
cd /root/.openclaw/workspace/LLMHQm-work/src
python3 collector.py
```

### Run Live WebSocket Collection
```bash
cd /root/.openclaw/workspace/LLMHQm-work/src
python3 layer_a_collector.py
```

### Run Complete 8-Analyst Array
```bash
cd /root/.openclaw/workspace/LLMHQm-work/src
python3 layer_a_complete.py
```

---

## Next Steps

To complete the setup, I need you to choose:

1. **Whale Watcher**: Which service should I configure?
2. **Cross-Exchange**: Do you want perp data (requires Binance Futures)?
3. **Correlation**: Which assets to track besides BTC? (ETH, SOL, or others?)

Once you decide, I'll:
- Implement the chosen data source integrations
- Add API key management (if needed)
- Create the full orchestration layer
- Set up the paper trading loop
