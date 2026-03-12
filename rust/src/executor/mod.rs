//! Layer D: Paper Trading Executor
//!
//! Responsibilities:
//! 1. Capture t=0 reference price from Binance WebSocket
//! 2. Fetch Polymarket Gamma API odds at t=0
//! 3. Validate odds match trade direction (YES >= 0.505, NO <= 0.495)
//! 4. Execute paper trade if approved
//! 5. Monitor resolution at t=300 (5m) or t=900 (15m)
//! 6. Calculate P&L and store outcome

use crate::types::*;
use crate::db::Database;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use tracing::{info, warn, error};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for paper trading
#[derive(Debug, Clone)]
pub struct PaperTradingConfig {
    /// Block interval in seconds (300 for 5m, 900 for 15m)
    pub block_duration_secs: i64,
    /// Minimum odds threshold for YES bet (0.0-1.0)
    pub yes_odds_threshold: f64,
    /// Maximum odds threshold for NO bet (0.0-1.0)
    pub no_odds_threshold: f64,
    /// Initial paper trading balance
    pub initial_balance: Decimal,
    /// Max position size as % of balance
    pub max_position_pct: u8,
    /// Enable odds validation
    pub validate_odds: bool,
    /// Market ID on Polymarket (for Gamma API)
    pub polymarket_market_id: String,
}

impl Default for PaperTradingConfig {
    fn default() -> Self {
        Self {
            block_duration_secs: 300, // 5 minutes default
            yes_odds_threshold: 0.505,
            no_odds_threshold: 0.495,
            initial_balance: Decimal::from(10000), // $10k paper money
            max_position_pct: 95,
            validate_odds: true,
            polymarket_market_id: String::new(), // Set per market
        }
    }
}

/// Price tracker for t=0 reference and resolution
#[derive(Debug, Default)]
pub struct PriceTracker {
    /// Current BTC price from Binance WebSocket
    pub current_price: Option<Decimal>,
    /// t=0 reference price (captured at block start)
    pub t0_price: Option<Decimal>,
    /// Resolution price (captured at block end)
    pub resolution_price: Option<Decimal>,
    /// Last update timestamp
    pub last_update: Option<DateTime<Utc>>,
}

impl PriceTracker {
    /// Update current price from WebSocket
    pub fn update_price(&mut self,
        price: Decimal
    ) {
        self.current_price = Some(price);
        self.last_update = Some(Utc::now());
    }
    
    /// Capture t=0 reference price
    pub fn capture_t0(&mut self) -> Option<Decimal> {
        if let Some(price) = self.current_price {
            self.t0_price = Some(price);
            info!("Captured t=0 reference price: ${}", price);
            Some(price)
        } else {
            warn!("No current price available for t=0 capture");
            None
        }
    }
    
    /// Capture resolution price at block end
    pub fn capture_resolution(&mut self) -> Option<Decimal> {
        if let Some(price) = self.current_price {
            self.resolution_price = Some(price);
            info!("Captured resolution price: ${}", price);
            Some(price)
        } else {
            warn!("No current price available for resolution capture");
            None
        }
    }
    
    /// Check if t=0 price is stale
    pub fn is_t0_stale(&self,
        max_age_secs: i64
    ) -> bool {
        match self.last_update {
            Some(last) => {
                let age = Utc::now() - last;
                age > Duration::seconds(max_age_secs)
            }
            None => true,
        }
    }
}

/// Polymarket odds from Gamma API
#[derive(Debug, Clone)]
pub struct PolymarketOdds {
    /// Timestamp of odds fetch
    pub timestamp: DateTime<Utc>,
    /// YES token price (0.0-1.0)
    pub yes_price: f64,
    /// NO token price (0.0-1.0)
    pub no_price: f64,
    /// Spread between YES and NO
    pub spread: f64,
    /// Volume in last hour
    pub volume_24h: f64,
}

impl PolymarketOdds {
    /// Calculate implied probability
    pub fn implied_yes_probability(&self
    ) -> f64 {
        self.yes_price
    }
    
    /// Check if odds are valid for trading
    pub fn is_valid(&self
    ) -> bool {
        // Check prices are in valid range
        if self.yes_price < 0.01 || self.yes_price > 0.99 {
            return false;
        }
        // Check spread isn't too wide (indicates illiquidity)
        if self.spread > 0.05 {
            return false;
        }
        true
    }
}

/// Odds validator - ensures odds match trade direction
pub struct OddsValidator {
    config: PaperTradingConfig,
}

impl OddsValidator {
    pub fn new(config: PaperTradingConfig) -> Self {
        Self { config }
    }
    
    /// Validate odds match intended trade direction
    /// 
    /// Logic:
    /// - UP prediction: Buy YES tokens → YES odds must be ≥ 0.505
    /// - DOWN prediction: Buy NO tokens → NO odds must be ≥ 0.505 (i.e., YES ≤ 0.495)
    /// 
    /// Returns: (is_valid, reason)
    pub fn validate(
        &self,
        direction: Direction,
        odds: &PolymarketOdds,
    ) -> (bool, Option<String>) {
        if !self.config.validate_odds {
            return (true, None);
        }
        
        if !odds.is_valid() {
            return (false, Some(format!(
                "Invalid odds: YES={:.3}, spread={:.3}",
                odds.yes_price, odds.spread
            )));
        }
        
        match direction {
            Direction::Up => {
                // For UP bet, we buy YES tokens → need YES odds >= threshold
                if odds.yes_price >= self.config.yes_odds_threshold {
                    (true, None)
                } else {
                    (false, Some(format!(
                        "YES odds {:.3} below threshold {:.3}. Contradicts UP prediction.",
                        odds.yes_price, self.config.yes_odds_threshold
                    )))
                }
            }
            Direction::Down => {
                // For DOWN bet, we buy NO tokens → need NO odds >= threshold
                // NO odds = 1 - YES odds (approximately)
                let no_price = odds.no_price;
                if no_price >= self.config.yes_odds_threshold { // Using same threshold for NO
                    (true, None)
                } else {
                    (false, Some(format!(
                        "NO odds {:.3} below threshold {:.3}. Contradicts DOWN prediction.",
                        no_price, self.config.yes_odds_threshold
                    )))
                }
            }
            Direction::Neutral => {
                (false, Some("Neutral direction - no trade".to_string()))
            }
        }
    }
}

/// Active paper trade
#[derive(Debug, Clone)]
pub struct ActiveTrade {
    pub id: uuid::Uuid,
    pub block_number: u64,
    pub decision: CIODecision,
    pub entry_price: Decimal,
    pub entry_time: DateTime<Utc>,
    pub position_size_usd: Decimal,
    pub polymarket_odds: PolymarketOdds,
    pub expected_resolution_time: DateTime<Utc>,
}

/// Paper trading executor
pub struct PaperExecutor {
    config: PaperTradingConfig,
    price_tracker: Arc<RwLock<PriceTracker>>,
    odds_validator: OddsValidator,
    active_trade: Arc<RwLock<Option<ActiveTrade>>>,
    balance: Arc<RwLock<Decimal>>,
    database: Option<Arc<Database>>,
}

impl PaperExecutor {
    pub fn new(
        config: PaperTradingConfig,
        database: Option<Arc<Database>>,
    ) -> Self {
        let initial_balance = config.initial_balance;
        
        Self {
            config: config.clone(),
            price_tracker: Arc::new(RwLock::new(PriceTracker::default())),
            odds_validator: OddsValidator::new(config),
            active_trade: Arc::new(RwLock::new(None)),
            balance: Arc::new(RwLock::new(initial_balance)),
            database,
        }
    }
    
    /// Get price tracker reference (for WebSocket updates)
    pub fn price_tracker(&self
    ) -> Arc<RwLock<PriceTracker>> {
        Arc::clone(&self.price_tracker)
    }
    
    /// Fetch Polymarket odds from Gamma API
    pub async fn fetch_polymarket_odds(
        &self,
    ) -> anyhow::Result<PolymarketOdds> {
        // Note: This is a placeholder. Actual implementation would call:
        // GET https://gamma-api.polymarket.com/markets/{market_id}
        // or use the Polymarket CLOB API
        
        // For now, simulate with reqwest call structure
        let url = format!(
            "https://gamma-api.polymarket.com/markets/{}",
            self.config.polymarket_market_id
        );
        
        // TODO: Implement actual API call
        // let client = reqwest::Client::new();
        // let response = client.get(&url).send().await?;
        // let data: serde_json::Value = response.json().await?;
        
        // Placeholder return
        Ok(PolymarketOdds {
            timestamp: Utc::now(),
            yes_price: 0.52, // Simulated
            no_price: 0.48,
            spread: 0.04,
            volume_24h: 1000000.0,
        })
    }
    
    /// Execute paper trade at t=0
    pub async fn execute_trade(
        &self,
        decision: &CIODecision,
        block_number: u64,
    ) -> anyhow::Result<Option<ActiveTrade>> {
        // Step 1: Capture t=0 price
        let entry_price = {
            let mut tracker = self.price_tracker.write().await;
            match tracker.capture_t0() {
                Some(price) => price,
                None => {
                    error!("Failed to capture t=0 price");
                    return Ok(None);
                }
            }
        };
        
        // Step 2: Fetch Polymarket odds
        let odds = match self.fetch_polymarket_odds().await {
            Ok(o) => o,
            Err(e) => {
                error!("Failed to fetch Polymarket odds: {}", e);
                return Ok(None);
            }
        };
        
        info!(
            "Fetched Polymarket odds: YES={:.3}, NO={:.3}, spread={:.3}",
            odds.yes_price, odds.no_price, odds.spread
        );
        
        // Step 3: Validate odds match trade direction
        let (odds_valid, veto_reason) = self.odds_validator.validate(
            decision.direction,
            &odds,
        );
        
        if !odds_valid {
            warn!(
                "Trade vetoed due to odds validation: {}",
                veto_reason.unwrap_or_default()
            );
            return Ok(None);
        }
        
        // Step 4: Calculate position size
        let position_size = self.calculate_position_size(decision).await;
        
        // Step 5: Create active trade
        let trade = ActiveTrade {
            id: uuid::Uuid::new_v4(),
            block_number,
            decision: decision.clone(),
            entry_price,
            entry_time: Utc::now(),
            position_size_usd: position_size,
            polymarket_odds: odds,
            expected_resolution_time: Utc::now() 
                + Duration::seconds(self.config.block_duration_secs),
        };
        
        // Step 6: Store active trade
        {
            let mut active = self.active_trade.write().await;
            *active = Some(trade.clone());
        }
        
        // Step 7: Log to database
        if let Some(ref db) = self.database {
            let paper_trade = PaperTrade {
                id: trade.id,
                block_number: trade.block_number,
                decision: trade.decision.clone(),
                entry_price: trade.entry_price,
                entry_time: trade.entry_time,
                exit_price: None,
                exit_time: None,
                outcome: None,
                pnl_pct: None,
            };
            
            if let Err(e) = db.store_paper_trade(&paper_trade).await {
                error!("Failed to store paper trade: {}", e);
            }
        }
        
        info!(
            "Paper trade executed: {} @ ${} (size: ${}, odds: YES={:.3})",
            match trade.decision.direction {
                Direction::Up => "YES",
                Direction::Down => "NO",
                Direction::Neutral => "NEUTRAL",
            },
            trade.entry_price,
            trade.position_size_usd,
            trade.polymarket_odds.yes_price
        );
        
        Ok(Some(trade))
    }
    
    /// Calculate position size based on confidence and balance
    async fn calculate_position_size(
        &self,
        decision: &CIODecision
    ) -> Decimal {
        let balance = *self.balance.read().await;
        
        // Base size on confidence
        let confidence_pct = Decimal::from(decision.suggested_position_size_pct) 
            / Decimal::from(100);
        
        // Cap at max position %
        let max_position_pct = Decimal::from(self.config.max_position_pct) 
            / Decimal::from(100);
        
        let position_pct = confidence_pct.min(max_position_pct);
        
        balance * position_pct
    }
    
    /// Resolve active trade at block end
    pub async fn resolve_trade(
        &self
    ) -> anyhow::Result<Option<PaperTrade>> {
        // Step 1: Get active trade
        let trade = {
            let active = self.active_trade.read().await;
            match active.as_ref() {
                Some(t) => t.clone(),
                None => return Ok(None),
            }
        };
        
        // Step 2: Capture resolution price
        let exit_price = {
            let mut tracker = self.price_tracker.write().await;
            match tracker.capture_resolution() {
                Some(price) => price,
                None => {
                    error!("Failed to capture resolution price");
                    return Ok(None);
                }
            }
        };
        
        // Step 3: Calculate outcome
        let (outcome, pnl_pct) = self.calculate_outcome(
            &trade,
            exit_price,
        );
        
        // Step 4: Update balance
        {
            let mut balance = self.balance.write().await;
            let pnl_decimal = Decimal::try_from(pnl_pct / 100.0).unwrap_or_default();
            let pnl_amount = trade.position_size_usd * pnl_decimal;
            *balance += pnl_amount;
        }
        
        // Step 5: Clear active trade
        {
            let mut active = self.active_trade.write().await;
            *active = None;
        }
        
        // Step 6: Update database
        if let Some(ref db) = self.database {
            if let Err(e) = db.update_trade_outcome(
                trade.id,
                exit_price,
                outcome,
                pnl_pct,
            ).await {
                error!("Failed to update trade outcome: {}", e);
            }
        }
        
        info!(
            "Trade resolved: {:?} | Entry: ${} | Exit: ${} | P&L: {:.2}% | Outcome: {:?}",
            trade.decision.direction,
            trade.entry_price,
            exit_price,
            pnl_pct,
            outcome
        );
        
        // Return completed trade
        Ok(Some(PaperTrade {
            id: trade.id,
            block_number: trade.block_number,
            decision: trade.decision,
            entry_price: trade.entry_price,
            entry_time: trade.entry_time,
            exit_price: Some(exit_price),
            exit_time: Some(Utc::now()),
            outcome: Some(outcome),
            pnl_pct: Some(pnl_pct),
        }))
    }
    
    /// Calculate trade outcome and P&L
    fn calculate_outcome(
        &self,
        trade: &ActiveTrade,
        exit_price: Decimal,
    ) -> (TradeOutcome, f64) {
        let entry_f64: f64 = trade.entry_price.try_into().unwrap_or(0.0);
        let exit_f64: f64 = exit_price.try_into().unwrap_or(0.0);
        
        if entry_f64 == 0.0 {
            return (TradeOutcome::Breakeven, 0.0);
        }
        
        let price_change_pct = match trade.decision.direction {
            Direction::Up => {
                // Long position - profit if price goes up
                ((exit_f64 - entry_f64) / entry_f64) * 100.0
            }
            Direction::Down => {
                // Short position - profit if price goes down
                ((entry_f64 - exit_f64) / entry_f64) * 100.0
            }
            Direction::Neutral => 0.0,
        };
        
        let outcome = if price_change_pct > 0.0 {
            TradeOutcome::Win
        } else if price_change_pct < 0.0 {
            TradeOutcome::Loss
        } else {
            TradeOutcome::Breakeven
        };
        
        (outcome, price_change_pct)
    }
    
    /// Get current balance
    pub async fn get_balance(&self
    ) -> Decimal {
        *self.balance.read().await
    }
    
    /// Check if there's an active trade
    pub async fn has_active_trade(&self
    ) -> bool {
        self.active_trade.read().await.is_some()
    }
    
    /// Get active trade info
    pub async fn get_active_trade(&self
    ) -> Option<ActiveTrade> {
        self.active_trade.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_odds_validator_yes() {
        let config = PaperTradingConfig {
            yes_odds_threshold: 0.505,
            no_odds_threshold: 0.495,
            ..Default::default()
        };
        
        let validator = OddsValidator::new(config);
        
        // Valid YES bet
        let good_odds = PolymarketOdds {
            timestamp: Utc::now(),
            yes_price: 0.52,
            no_price: 0.48,
            spread: 0.04,
            volume_24h: 1000000.0,
        };
        let (valid, _) = validator.validate(Direction::Up, &good_odds);
        assert!(valid);
        
        // Invalid YES bet (odds too low)
        let bad_odds = PolymarketOdds {
            timestamp: Utc::now(),
            yes_price: 0.49,
            no_price: 0.51,
            spread: 0.02,
            volume_24h: 1000000.0,
        };
        let (valid, reason) = validator.validate(Direction::Up, &bad_odds);
        assert!(!valid);
        assert!(reason.unwrap().contains("below threshold"));
    }
    
    #[test]
    fn test_odds_validator_no() {
        let config = PaperTradingConfig {
            yes_odds_threshold: 0.505,
            ..Default::default()
        };
        
        let validator = OddsValidator::new(config);
        
        // Valid NO bet (NO price high enough)
        // YES at 0.48 means NO at ~0.52, which is >= 0.505 ✓
        let good_odds = PolymarketOdds {
            timestamp: Utc::now(),
            yes_price: 0.48,
            no_price: 0.52,
            spread: 0.04,
            volume_24h: 1000000.0,
        };
        let (valid, _) = validator.validate(Direction::Down, &good_odds);
        assert!(valid);
        
        // Invalid NO bet (NO price too low)
        // YES at 0.52 means NO at ~0.48, which is < 0.505 ✗
        let bad_odds = PolymarketOdds {
            timestamp: Utc::now(),
            yes_price: 0.52,
            no_price: 0.48,
            spread: 0.04,
            volume_24h: 1000000.0,
        };
        let (valid, reason) = validator.validate(Direction::Down, &bad_odds);
        assert!(!valid);
        assert!(reason.unwrap().contains("NO odds"));
        assert!(reason.unwrap().contains("below threshold"));
    }
}
