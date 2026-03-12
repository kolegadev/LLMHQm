-- Initial database schema for LLMHQ
-- Designed for PostgreSQL with TimescaleDB extension

-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- =============================================================================
-- RAW MARKET DATA
-- =============================================================================

-- Market ticks (hypertable)
CREATE TABLE market_ticks (
    time TIMESTAMPTZ NOT NULL,
    source TEXT NOT NULL,
    symbol TEXT NOT NULL,
    price DECIMAL(18, 8) NOT NULL,
    quantity DECIMAL(18, 8) NOT NULL,
    is_buyer_maker BOOLEAN NOT NULL,
    is_liquidation BOOLEAN DEFAULT FALSE
);

-- Convert to hypertable
SELECT create_hypertable('market_ticks', 'time', chunk_time_interval => INTERVAL '1 hour');

-- Index for common queries
CREATE INDEX idx_market_ticks_symbol_time ON market_ticks (symbol, time DESC);
CREATE INDEX idx_market_ticks_liquidations ON market_ticks (is_liquidation, time DESC) WHERE is_liquidation = TRUE;

-- Order book snapshots
CREATE TABLE order_book_snapshots (
    time TIMESTAMPTZ NOT NULL,
    source TEXT NOT NULL,
    symbol TEXT NOT NULL,
    bids JSONB NOT NULL,
    asks JSONB NOT NULL,
    last_update_id BIGINT NOT NULL
);

SELECT create_hypertable('order_book_snapshots', 'time', chunk_time_interval => INTERVAL '1 hour');

CREATE INDEX idx_ob_snapshots_symbol_time ON order_book_snapshots (symbol, time DESC);

-- =============================================================================
-- ANALYST READINGS
-- =============================================================================

CREATE TABLE analyst_readings (
    time TIMESTAMPTZ NOT NULL,
    block_number BIGINT NOT NULL,
    analyst TEXT NOT NULL,
    readings JSONB NOT NULL
);

SELECT create_hypertable('analyst_readings', 'time', chunk_time_interval => INTERVAL '1 day');

CREATE INDEX idx_readings_block ON analyst_readings (block_number);
CREATE INDEX idx_readings_analyst ON analyst_readings (analyst, time DESC);

-- =============================================================================
-- NARRATIVES (LAYER B OUTPUT)
-- =============================================================================

CREATE TABLE narratives (
    time TIMESTAMPTZ NOT NULL,
    block_number BIGINT NOT NULL,
    narrative_md TEXT NOT NULL,
    pattern_tags TEXT[] DEFAULT '{}',
    confidence DECIMAL(3, 2) NOT NULL
);

SELECT create_hypertable('narratives', 'time', chunk_time_interval => INTERVAL '1 day');

CREATE INDEX idx_narratives_block ON narratives (block_number);
CREATE INDEX idx_narratives_tags ON narratives USING GIN (pattern_tags);

-- =============================================================================
-- CIO DECISIONS (LAYER C OUTPUT)
-- =============================================================================

CREATE TABLE cio_decisions (
    time TIMESTAMPTZ NOT NULL,
    block_number BIGINT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('UP', 'DOWN', 'NEUTRAL')),
    confidence INTEGER NOT NULL CHECK (confidence >= 0 AND confidence <= 100),
    regime TEXT NOT NULL,
    lead_driver TEXT NOT NULL,
    rationale TEXT NOT NULL,
    risk_flags TEXT[] DEFAULT '{}',
    veto_applied BOOLEAN DEFAULT FALSE,
    veto_reason TEXT,
    llm_prompt TEXT,
    llm_response TEXT
);

SELECT create_hypertable('cio_decisions', 'time', chunk_time_interval => INTERVAL '1 day');

CREATE INDEX idx_decisions_block ON cio_decisions (block_number);
CREATE INDEX idx_decisions_direction ON cio_decisions (direction, time DESC);
CREATE INDEX idx_decisions_veto ON cio_decisions (veto_applied, time DESC);

-- =============================================================================
-- PAPER TRADES (LAYER D)
-- =============================================================================

CREATE TABLE paper_trades (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    block_number BIGINT NOT NULL,
    decision_time TIMESTAMPTZ NOT NULL,
    entry_time TIMESTAMPTZ NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('UP', 'DOWN')),
    confidence INTEGER NOT NULL,
    entry_price DECIMAL(18, 8) NOT NULL,
    exit_price DECIMAL(18, 8),
    exit_time TIMESTAMPTZ,
    outcome TEXT CHECK (outcome IN ('WIN', 'LOSS', 'BREAKEVEN')),
    pnl_pct DECIMAL(8, 4),
    narrative_md TEXT,
    decision_json JSONB
);

CREATE INDEX idx_paper_trades_block ON paper_trades (block_number);
CREATE INDEX idx_paper_trades_outcome ON paper_trades (outcome, exit_time DESC) WHERE outcome IS NOT NULL;
CREATE INDEX idx_paper_trades_open ON paper_trades (entry_time) WHERE outcome IS NULL;

-- =============================================================================
-- PERFORMANCE METRICS
-- =============================================================================

CREATE TABLE performance_metrics (
    time TIMESTAMPTZ NOT NULL,
    window_hours INTEGER NOT NULL,
    total_trades INTEGER NOT NULL,
    win_count INTEGER NOT NULL,
    loss_count INTEGER NOT NULL,
    win_rate DECIMAL(5, 2) NOT NULL,
    avg_pnl DECIMAL(8, 4),
    avg_win_pnl DECIMAL(8, 4),
    avg_loss_pnl DECIMAL(8, 4),
    sharpe_ratio DECIMAL(6, 2),
    max_drawdown DECIMAL(8, 4)
);

SELECT create_hypertable('performance_metrics', 'time', chunk_time_interval => INTERVAL '1 day');

-- =============================================================================
-- GHOST TRADE ANALYSIS (LAYER E)
-- =============================================================================

CREATE TABLE ghost_trade_analysis (
    time TIMESTAMPTZ NOT NULL,
    trade_id UUID NOT NULL REFERENCES paper_trades(id),
    classification TEXT NOT NULL CHECK (classification IN ('success', 'failure', 'mixed')),
    likely_cause TEXT NOT NULL,
    lesson_learned TEXT NOT NULL,
    regime_adjustment TEXT,
    warning_rule TEXT
);

SELECT create_hypertable('ghost_trade_analysis', 'time', chunk_time_interval => INTERVAL '7 days');

CREATE INDEX idx_ghost_trade_id ON ghost_trade_analysis (trade_id);

-- =============================================================================
-- POLICY UPDATES
-- =============================================================================

CREATE TABLE policy_updates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    issue_summary TEXT NOT NULL,
    evidence JSONB NOT NULL,
    current_behavior TEXT NOT NULL,
    proposed_change TEXT NOT NULL,
    expected_benefit TEXT NOT NULL,
    rollback_note TEXT,
    review_status TEXT DEFAULT 'proposed' CHECK (review_status IN ('proposed', 'approved', 'rejected', 'deployed')),
    deployed_at TIMESTAMPTZ
);

-- =============================================================================
-- VIEWS
-- =============================================================================

-- Open positions view
CREATE VIEW open_paper_trades AS
SELECT * FROM paper_trades
WHERE outcome IS NULL;

-- Recent performance view
CREATE VIEW recent_performance AS
SELECT 
    DATE_TRUNC('day', exit_time) as date,
    COUNT(*) as trades,
    COUNT(CASE WHEN outcome = 'WIN' THEN 1 END) as wins,
    COUNT(CASE WHEN outcome = 'LOSS' THEN 1 END) as losses,
    AVG(pnl_pct) as avg_pnl,
    SUM(CASE WHEN outcome = 'WIN' THEN pnl_pct ELSE 0 END) as total_win_pnl,
    SUM(CASE WHEN outcome = 'LOSS' THEN pnl_pct ELSE 0 END) as total_loss_pnl
FROM paper_trades
WHERE outcome IS NOT NULL
GROUP BY DATE_TRUNC('day', exit_time)
ORDER BY date DESC;

-- =============================================================================
-- FUNCTIONS
-- =============================================================================

-- Calculate win rate over period
CREATE OR REPLACE FUNCTION calculate_win_rate(
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ
) RETURNS DECIMAL(5, 2) AS $$
DECLARE
    total INTEGER;
    wins INTEGER;
BEGIN
    SELECT COUNT(*), COUNT(CASE WHEN outcome = 'WIN' THEN 1 END)
    INTO total, wins
    FROM paper_trades
    WHERE exit_time BETWEEN start_time AND end_time
    AND outcome IS NOT NULL;
    
    IF total = 0 THEN
        RETURN 0.0;
    END IF;
    
    RETURN (wins::DECIMAL / total::DECIMAL) * 100.0;
END;
$$ LANGUAGE plpgsql;

-- Update trade outcome (call when block resolves)
CREATE OR REPLACE FUNCTION resolve_trade(
    p_trade_id UUID,
    p_exit_price DECIMAL,
    p_block_close_price DECIMAL
) RETURNS VOID AS $$
DECLARE
    v_direction TEXT;
    v_entry_price DECIMAL;
    v_pnl_pct DECIMAL;
    v_outcome TEXT;
BEGIN
    -- Get trade details
    SELECT direction, entry_price
    INTO v_direction, v_entry_price
    FROM paper_trades
    WHERE id = p_trade_id;
    
    -- Calculate P&L
    IF v_direction = 'UP' THEN
        v_pnl_pct := ((p_exit_price - v_entry_price) / v_entry_price) * 100;
    ELSE
        v_pnl_pct := ((v_entry_price - p_exit_price) / v_entry_price) * 100;
    END IF;
    
    -- Determine outcome
    IF v_pnl_pct > 0 THEN
        v_outcome := 'WIN';
    ELSIF v_pnl_pct < 0 THEN
        v_outcome := 'LOSS';
    ELSE
        v_outcome := 'BREAKEVEN';
    END IF;
    
    -- Update trade
    UPDATE paper_trades
    SET exit_price = p_exit_price,
        exit_time = NOW(),
        outcome = v_outcome,
        pnl_pct = v_pnl_pct
    WHERE id = p_trade_id;
END;
$$ LANGUAGE plpgsql;
