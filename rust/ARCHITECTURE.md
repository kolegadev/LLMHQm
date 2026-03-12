# LLMHQ Rust Architecture

## Project Structure

```
llmhq/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry point
│   ├── lib.rs                  # Library exports
│   ├── config.rs               # Configuration
│   ├── db/
│   │   ├── mod.rs              # Database module
│   │   ├── connection.rs       # Async connection pool
│   │   ├── models.rs           # Data models
│   │   └── migrations/         # SQL migrations
│   ├── collectors/
│   │   ├── mod.rs
│   │   ├── binance_spot.rs     # Spot WebSocket
│   │   ├── binance_futures.rs  # Perp + liquidations
│   │   └── aggregator.rs       # Multi-stream aggregation
│   ├── analysts/
│   │   ├── mod.rs
│   │   ├── tape_reader.rs      # OBI, spread, pressure
│   │   ├── momentum.rs         # HMA, slope, ROC, RSI
│   │   ├── microstructure.rs   # VPIN, volatility
│   │   ├── whale_watcher.rs    # Liquidations
│   │   ├── cross_exchange.rs   # Spot-perp basis
│   │   ├── correlation.rs      # Multi-asset correlation
│   │   ├── liquidity_map.rs    # Voids, walls
│   │   └── pinning_risk.rs     # Manipulation detection
│   ├── timing/
│   │   ├── mod.rs
│   │   └── block_timer.rs      # 5m interval sync
│   ├── narrator/
│   │   ├── mod.rs
│   │   ├── pattern_library.rs  # Chess-like pattern definitions
│   │   ├── semantic_builder.rs # Markdown narrative generator
│   │   └── templates/          # Pattern templates
│   ├── cio/
│   │   ├── mod.rs
│   │   ├── decision_engine.rs  # Assessment + prediction
│   │   ├── veto_logic.rs       # Veto rules
│   │   └── prompt_builder.rs   # CIO prompt construction
│   └── executor/
│       ├── mod.rs
│       └── paper_trade.rs      # Paper trading + logging
├── migrations/
│   └── 001_initial.sql
└── config/
    └── default.toml
```

## Stack

- **Runtime**: Tokio (async)
- **WebSocket**: tokio-tungstenite
- **Database**: sqlx (PostgreSQL/TimescaleDB) with connection pooling
- **Serialization**: serde + serde_json
- **HTTP**: reqwest
- **Logging**: tracing
- **Metrics**: metrics + metrics-exporter-prometheus

## Database Schema

```sql
-- Raw market data (TimescaleDB hypertable)
CREATE TABLE market_ticks (
    time TIMESTAMPTZ NOT NULL,
    source TEXT,
    symbol TEXT,
    price DECIMAL,
    quantity DECIMAL,
    is_buyer_maker BOOLEAN
);

-- Analyst readings
CREATE TABLE analyst_readings (
    time TIMESTAMPTZ NOT NULL,
    block_number BIGINT,
    analyst TEXT,
    readings JSONB
);

-- Narrative outputs
CREATE TABLE narratives (
    time TIMESTAMPTZ NOT NULL,
    block_number BIGINT,
    narrative_md TEXT,
    pattern_tags TEXT[]
);

-- CIO decisions
CREATE TABLE cio_decisions (
    time TIMESTAMPTZ NOT NULL,
    block_number BIGINT,
    direction TEXT, -- UP or DOWN
    confidence INTEGER,
    regime TEXT,
    lead_driver TEXT,
    rationale TEXT,
    veto_applied BOOLEAN,
    risk_flags TEXT[]
);

-- Paper trades
CREATE TABLE paper_trades (
    id SERIAL PRIMARY KEY,
    block_number BIGINT,
    decision_time TIMESTAMPTZ,
    entry_time TIMESTAMPTZ,
    direction TEXT,
    confidence INTEGER,
    entry_price DECIMAL,
    exit_price DECIMAL,
    outcome TEXT, -- WIN or LOSS
    pnl_pct DECIMAL
);
```

## Key Features

1. **Zero-allocation hot path**: Market data flows through without heap allocation
2. **Lock-free channels**: Cross-thread communication via tokio::sync::mpsc
3. **Parallel analysts**: Each analyst runs in own task, results merged
4. **Backpressure handling**: Slow consumers don't block fast producers
5. **Graceful degradation**: If one feed fails, others continue
