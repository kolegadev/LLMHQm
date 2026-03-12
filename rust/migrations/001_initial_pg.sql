-- Initial database schema for LLMHQ (PostgreSQL without TimescaleDB)
-- Works with standard PostgreSQL - TimescaleDB is optional enhancement

-- =============================================================================
-- RAW MARKET DATA
-- =============================================================================

CREATE TABLE market_ticks (
    id SERIAL PRIMARY KEY,
    time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source TEXT NOT NULL,
    symbol TEXT NOT NULL,
    price DECIMAL(18, 8) NOT NULL,
    quantity DECIMAL(18, 8) NOT NULL,
    is_buyer_maker BOOLEAN NOT NULL,
    is_liquidation BOOLEAN DEFAULT FALSE
);

CREATE INDEX idx_market_ticks_time ON market_ticks (time DESC);
CREATE INDEX idx_market_ticks_symbol_time ON market_ticks (symbol, time DESC);
CREATE INDEX idx_market_ticks_liquidations ON market_ticks (is_liquidation, time DESC) WHERE is_liquidation = TRUE;

-- Order book snapshots
CREATE TABLE order_book_snapshots (
    id SERIAL PRIMARY KEY,
    time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source TEXT NOT NULL,
    symbol TEXT NOT NULL,
    bids JSONB NOT NULL,
    asks JSONB NOT NULL,
    last_update_id BIGINT NOT NULL
);

CREATE INDEX idx_ob_snapshots_time ON order_book_snapshots (time DESC);
CREATE INDEX idx_ob_snapshots_symbol_time ON order_book_snapshots (symbol, time DESC);

-- =============================================================================
-- ANALYST READINGS
-- =============================================================================

CREATE TABLE analyst_readings (
    id SERIAL PRIMARY KEY,
    time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    block_number BIGINT NOT NULL,
    analyst TEXT NOT NULL,
    readings JSONB NOT NULL
);

CREATE INDEX idx_readings_time ON analyst_readings (time DESC);
CREATE INDEX idx_readings_block ON analyst_readings (block_number);
CREATE INDEX idx_readings_analyst ON analyst_readings (analyst, time DESC);

-- =============================================================================
-- NARRATIVES (LAYER B OUTPUT)
-- =============================================================================

CREATE TABLE narratives (
    id SERIAL PRIMARY KEY,
    time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    block_number BIGINT NOT NULL,
    narrative_md TEXT NOT NULL,
    pattern_tags TEXT[] DEFAULT '{}',
    confidence DECIMAL(3, 2) NOT NULL
);

CREATE INDEX idx_narratives_time ON narratives (time DESC);
CREATE INDEX idx_narratives_block ON narratives (block_number);

-- =============================================================================
-- CIO DECISIONS (LAYER C OUTPUT)
-- =============================================================================

CREATE TABLE cio_decisions (
    id SERIAL PRIMARY KEY,
    time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
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

CREATE INDEX idx_decisions_time ON cio_decisions (time DESC);
CREATE INDEX idx_decisions_block ON cio_decisions (block_number);
CREATE INDEX idx_decisions_direction ON cio_decisions (direction, time DESC);
CREATE INDEX idx_decisions_veto ON cio_decisions (veto_applied, time DESC);

-- =============================================================================
-- PAPER TRADES (LAYER D)
-- =============================================================================

CREATE TABLE paper_trades (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    block_number BIGINT NOT NULL,
    decision_time TIMESTAMPTZ,
    entry_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    direction TEXT NOT NULL CHECK (direction IN ('UP', 'DOWN')),
    confidence INTEGER NOT NULL,
    entry_price DECIMAL(18, 8) NOT NULL,
    exit_price DECIMAL(18, 8),
    exit_time TIMESTAMPTZ,
    outcome TEXT CHECK (outcome IN ('WIN', 'LOSS', 'BREAKEVEN')),
    pnl_pct DECIMAL(8, 4),
    polymarket_yes_odds DECIMAL(4, 3),
    narrative_md TEXT,
    decision_json JSONB
);

CREATE INDEX idx_paper_trades_time ON paper_trades (entry_time DESC);
CREATE INDEX idx_paper_trades_block ON paper_trades (block_number);
CREATE INDEX idx_paper_trades_outcome ON paper_trades (outcome, exit_time DESC) WHERE outcome IS NOT NULL;
CREATE INDEX idx_paper_trades_open ON paper_trades (entry_time) WHERE outcome IS NULL;

-- =============================================================================
-- PERFORMANCE METRICS
-- =============================================================================

CREATE TABLE performance_metrics (
    id SERIAL PRIMARY KEY,
    time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
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

CREATE INDEX idx_metrics_time ON performance_metrics (time DESC);

-- =============================================================================
-- GHOST TRADE ANALYSIS (LAYER E)
-- =============================================================================

CREATE TABLE ghost_trade_analysis (
    id SERIAL PRIMARY KEY,
    time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    trade_id UUID NOT NULL REFERENCES paper_trades(id),
    classification TEXT NOT NULL CHECK (classification IN ('success', 'failure', 'mixed')),
    likely_cause TEXT NOT NULL,
    lesson_learned TEXT NOT NULL,
    regime_adjustment TEXT,
    warning_rule TEXT
);

CREATE INDEX idx_ghost_time ON ghost_trade_analysis (time DESC);
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

CREATE INDEX idx_policy_status ON policy_updates (review_status, created_at DESC);

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

-- Daily summary view
CREATE VIEW daily_summary AS
SELECT 
    DATE_TRUNC('day', time) as date,
    COUNT(DISTINCT block_number) as blocks_analyzed,
    COUNT(CASE WHEN direction = 'UP' THEN 1 END) as up_predictions,
    COUNT(CASE WHEN direction = 'DOWN' THEN 1 END) as down_predictions,
    COUNT(CASE WHEN veto_applied THEN 1 END) as vetoes,
    AVG(confidence) as avg_confidence
FROM cio_decisions
GROUP BY DATE_TRUNC('day', time)
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
    p_exit_price DECIMAL
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

-- Get current balance (paper trading)
CREATE OR REPLACE FUNCTION get_paper_balance(
    p_initial_balance DECIMAL DEFAULT 10000
) RETURNS DECIMAL AS $$
DECLARE
    v_total_pnl DECIMAL;
BEGIN
    SELECT COALESCE(SUM(pnl_pct * entry_price / 100), 0)
    INTO v_total_pnl
    FROM paper_trades
    WHERE outcome IS NOT NULL;
    
    RETURN p_initial_balance + v_total_pnl;
END;
$$ LANGUAGE plpgsql;

-- Grant permissions to llmhq user
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO llmhq;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO llmhq;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO llmhq;
