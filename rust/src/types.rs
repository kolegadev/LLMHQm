//! Core types and data structures for LLMHQ

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Market tick from exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTick {
    pub timestamp: DateTime<Utc>,
    pub source: Exchange,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub side: TradeSide,
    pub is_liquidation: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Exchange {
    BinanceSpot,
    BinanceFutures,
    Bitmex,
    Deribit,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

/// Order book snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub last_update_id: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: Decimal,
    pub quantity: Decimal,
}

/// Candle/OHLCV data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub trades: u64,
}

/// Analyst readings - unified output from all analysts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalystReadings {
    pub timestamp: DateTime<Utc>,
    pub block_number: u64,
    pub seconds_to_block_end: f64,
    
    // Tape Reader
    pub obi: Option<f64>,
    pub obi_normalized: Option<f64>,
    pub obi_velocity: Option<f64>,
    pub spread_bps: Option<f64>,
    pub pressure: Option<Pressure>,
    
    // Momentum
    pub hma: Option<Decimal>,
    pub hma_slope: Option<f64>,
    pub roc_3m: Option<f64>,
    pub rsi: Option<f64>,
    pub hma_trend: Option<Trend>,
    
    // Microstructure
    pub vpin: Option<f64>,
    pub volatility: Option<f64>,
    pub volatility_regime: Option<VolatilityRegime>,
    pub toxicity: Option<Toxicity>,
    
    // Whale/Liquidations
    pub long_liquidations_1m: Option<f64>,
    pub short_liquidations_1m: Option<f64>,
    pub net_liquidation_pressure: Option<f64>,
    
    // Cross-Exchange
    pub spot_price: Option<Decimal>,
    pub perp_price: Option<Decimal>,
    pub basis_bps: Option<f64>,
    pub perp_bias: Option<PerpBias>,
    
    // Correlation
    pub correlations: HashMap<String, f64>,
    
    // Liquidity Map
    pub liquidity_void_above_pct: Option<f64>,
    pub liquidity_void_below_pct: Option<f64>,
    pub near_bid_wall: Option<bool>,
    pub near_ask_wall: Option<bool>,
    
    // Pinning Risk
    pub pinning_risk_score: Option<u8>,
    pub pinning_classification: Option<PinningClassification>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Pressure {
    StrongBuy,
    Buy,
    Neutral,
    Sell,
    StrongSell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Trend {
    Up,
    Down,
    Flat,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VolatilityRegime {
    Expanding,
    Normal,
    Compressing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Toxicity {
    Elevated,
    Normal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PerpBias {
    StrongPremium,
    Premium,
    Neutral,
    Discount,
    StrongDiscount,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PinningClassification {
    HighBreak,    // Veto - manipulation breaking
    HighHold,     // Caution - possible trap
    Elevated,     // Reduced confidence
    Low,          // Proceed normally
}

impl AnalystReadings {
    /// Get the dominant regime based on readings
    pub fn dominant_regime(&self) -> MarketRegime {
        use MarketRegime::*;
        
        // Check for manipulation first
        if let Some(PinningClassification::HighBreak) = self.pinning_classification {
            return Manipulative;
        }
        
        // Check volatility
        if let Some(VolatilityRegime::Expanding) = self.volatility_regime {
            return VolatileExpansion;
        }
        
        // Check trend
        if let Some(Trend::Up) = self.hma_trend {
            if let Some(Pressure::Buy | Pressure::StrongBuy) = self.pressure {
                return Trending;
            }
        }
        
        // Check compression
        if let Some(VolatilityRegime::Compressing) = self.volatility_regime {
            return QuietCompression;
        }
        
        // Default
        Ranging
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MarketRegime {
    Trending,
    Ranging,
    VolatileExpansion,
    QuietCompression,
    Manipulative,
}

/// Block timing state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTiming {
    pub current_block_number: u64,
    pub next_block_timestamp: DateTime<Utc>,
    pub seconds_to_next_block: f64,
    pub phase: BlockPhase,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BlockPhase {
    Idle,
    Calculation,      // t-30 to t-15
    Aggregation,      // t-15 to t-10
    Synthesis,        // t-10 to t-5
    Decision,         // t-5 to t-2
    Execution,        // t-2 to t=0
    PostExecution,
}

impl BlockPhase {
    pub fn description(&self) -> &'static str {
        match self {
            BlockPhase::Idle => "Waiting for next block",
            BlockPhase::Calculation => "Phase 1: Parallel feature calculation",
            BlockPhase::Aggregation => "Phase 2: Data aggregation",
            BlockPhase::Synthesis => "Phase 3: Semantic synthesis",
            BlockPhase::Decision => "Phase 4: CIO decision window",
            BlockPhase::Execution => "Phase 5: Execution preparation",
            BlockPhase::PostExecution => "Block complete - monitoring outcome",
        }
    }
}

/// Semantic narrative output (Layer B)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticNarrative {
    pub timestamp: DateTime<Utc>,
    pub block_number: u64,
    pub narrative_md: String,
    pub pattern_tags: Vec<String>,
    pub confidence: f64,
}

/// CIO Decision output (Layer C)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CIODecision {
    pub timestamp: DateTime<Utc>,
    pub block_number: u64,
    pub direction: Direction,
    pub confidence: u8, // 0-100
    pub regime: MarketRegime,
    pub lead_driver: String,
    pub rationale: String,
    pub risk_flags: Vec<String>,
    pub veto_applied: bool,
    pub veto_reason: Option<String>,
    pub pinning_assessment: Option<PinningClassification>,
    pub suggested_position_size_pct: u8, // Adjusted for risk
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Neutral,
}

/// Paper trade record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperTrade {
    pub id: uuid::Uuid,
    pub block_number: u64,
    pub decision: CIODecision,
    pub entry_price: Decimal,
    pub entry_time: DateTime<Utc>,
    pub exit_price: Option<Decimal>,
    pub exit_time: Option<DateTime<Utc>>,
    pub outcome: Option<TradeOutcome>,
    pub pnl_pct: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TradeOutcome {
    Win,
    Loss,
    Breakeven,
}

/// Risk assessment for veto logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub risk_score: u8,
    pub classification: PinningClassification,
    pub factors: Vec<String>,
    pub recommendation: RiskRecommendation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RiskRecommendation {
    Veto,
    ReduceSize,
    Caution,
    Proceed,
}
