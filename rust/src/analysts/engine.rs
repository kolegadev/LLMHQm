//! Layer A: Real-Time Analyst Calculations
//!
//! Computes all 8 analyst indicators from live market data:
//! 1. Tape Reader - OBI, spread, pressure
//! 2. Momentum Engine - HMA, slope, ROC, RSI  
//! 3. Microstructure - VPIN, volatility
//! 4. Whale Watcher - Liquidation tracking
//! 5. Cross-Exchange - Spot-perp basis
//! 6. Correlation - Multi-asset correlation
//! 7. Liquidity Map - Voids, walls
//! 8. Pinning Risk - Manipulation detection

use crate::types::*;
use crate::collectors::LiquidationEvent;
use rust_decimal::Decimal;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Window sizes for calculations
const HMA_PERIOD: usize = 14;
const RSI_PERIOD: usize = 14;
const VPIN_BUCKETS: usize = 50;
const VOLATILITY_WINDOW: usize = 20;
const CORRELATION_WINDOW: usize = 100;

/// Real-time analyst engine
pub struct AnalystEngine {
    /// Price history for HMA/RSI (circular buffer)
    price_history: Arc<RwLock<VecDeque<Decimal>>>,
    /// HMA history for slope calculation
    hma_history: Arc<RwLock<VecDeque<Decimal>>,
    /// Trade history for VPIN (buy/sell volume)
    trade_history: Arc<RwLock<VecDeque<TradeInfo>>>,
    /// Liquidation tracking
    liquidations: Arc<RwLock<VecDeque<LiquidationEvent>>>,
    /// Multi-asset prices for correlation
    multi_asset_prices: Arc<RwLock<HashMap<String, VecDeque<Decimal>>㸾,
    /// Order book state
    order_book: Arc<RwLock<OrderBookState>>,
    /// Current readings (updated atomically)
    current_readings: Arc<RwLock<AnalystReadings>>,
}

#[derive(Debug, Clone, Copy)]
struct TradeInfo {
    price: Decimal,
    quantity: Decimal,
    is_buy: bool, // true = buyer is taker (aggressive buy)
    timestamp: i64,
}

#[derive(Debug, Clone, Default)]
struct OrderBookState {
    bids: Vec<PriceLevel>,
    asks: Vec<PriceLevel>,
    last_update: i64,
}

impl AnalystEngine {
    pub fn new() -> Self {
        Self {
            price_history: Arc::new(RwLock::new(VecDeque::with_capacity(200))),
            hma_history: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            trade_history: Arc::new(RwLock::new(VecDeque::with_capacity(500))),
            liquidations: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            multi_asset_prices: Arc::new(RwLock::new(HashMap::new())),
            order_book: Arc::new(RwLock::new(OrderBookState::default())),
            current_readings: Arc::new(RwLock::new(AnalystReadings::default())),
        }
    }

    /// Process new price tick
    pub async fn process_price_tick(
        &self,
        asset: &str,
        price: Decimal,
        timestamp: i64,
    ) {
        // Store in price history
        {
            let mut history = self.price_history.write().await;
            history.push_back(price);
            if history.len() > 200 {
                history.pop_front();
            }
        }

        // Store in multi-asset prices for correlation
        {
            let mut prices = self.multi_asset_prices.write().await;
            let asset_history = prices.entry(asset.to_string()).or_insert_with(|| {
                VecDeque::with_capacity(CORRELATION_WINDOW)
            });
            asset_history.push_back(price);
            if asset_history.len() > CORRELATION_WINDOW {
                asset_history.pop_front();
            }
        }

        // Recalculate indicators
        self.recalculate_all(timestamp).await;
    }

    /// Process trade tick for VPIN
    pub async fn process_trade_tick(
        &self,
        price: Decimal,
        quantity: Decimal,
        is_buy: bool,
        timestamp: i64,
    ) {
        let trade = TradeInfo {
            price,
            quantity,
            is_buy,
            timestamp,
        };

        {
            let mut history = self.trade_history.write().await;
            history.push_back(trade);
            if history.len() > 500 {
                history.pop_front();
            }
        }
    }

    /// Process liquidation event
    pub async fn process_liquidation(
        &self,
        event: LiquidationEvent,
    ) {
        let mut liqs = self.liquidations.write().await;
        liqs.push_back(event);
        if liqs.len() > 100 {
            liqs.pop_front();
        }
    }

    /// Update order book
    pub async fn update_order_book(
        &self,
        bids: Vec<PriceLevel>,
        asks: Vec<PriceLevel>,
    ) {
        let mut ob = self.order_book.write().await;
        ob.bids = bids;
        ob.asks = asks;
        ob.last_update = chrono::Utc::now().timestamp();
    }

    /// Recalculate all indicators
    async fn recalculate_all(
        &self,
        timestamp: i64,
    ) {
        let mut readings = AnalystReadings {
            timestamp: chrono::DateTime::from_timestamp(timestamp, 0)
                .unwrap_or_else(|| chrono::Utc::now()),
            ..Default::default()
        };

        // Calculate momentum indicators
        self.calculate_momentum(&mut readings).await;

        // Calculate tape reader indicators
        self.calculate_tape_reader(&mut readings).await;

        // Calculate microstructure
        self.calculate_microstructure(&mut readings).await;

        // Calculate liquidations
        self.calculate_liquidations(&mut readings).await;

        // Calculate correlations
        self.calculate_correlations(&mut readings).await;

        // Calculate liquidity map
        self.calculate_liquidity_map(&mut readings).await;

        // Calculate pinning risk
        self.calculate_pinning_risk(&mut readings).await;

        // Update current readings
        {
            let mut current = self.current_readings.write().await;
            *current = readings;
        }
    }

    /// Calculate momentum indicators (HMA, slope, ROC, RSI)
    async fn calculate_momentum(
        &self,
        readings: &mut AnalystReadings,
    ) {
        let history = self.price_history.read().await;
        
        if history.len() < HMA_PERIOD {
            return;
        }

        let prices: Vec<Decimal> = history.iter().copied().collect();
        
        // Calculate HMA
        if let Some(hma) = Self::calculate_hma(&prices, HMA_PERIOD) {
            readings.hma = Some(hma);

            // Store in HMA history for slope
            {
                let mut hma_hist = self.hma_history.write().await;
                hma_hist.push_back(hma);
                if hma_hist.len() > 100 {
                    hma_hist.pop_front();
                }
            }

            // Calculate HMA slope
            let hma_hist = self.hma_history.read().await;
            if hma_hist.len() >= 2 {
                let prev_hma = hma_hist[hma_hist.len() - 2];
                if prev_hma > Decimal::ZERO {
                    let change = (hma - prev_hma) / prev_hma;
                    // Convert to approximate degrees
                    let slope = change * Decimal::from(4500);
                    readings.hma_slope = Some(slope.clamp(
                        Decimal::from(-90),
                        Decimal::from(90)
                    ).try_into().unwrap_or(0.0));

                    // Classify trend
                    readings.hma_trend = Some(if slope > Decimal::from(15) {
                        Trend::Up
                    } else if slope < Decimal::from(-15) {
                        Trend::Down
                    } else {
                        Trend::Flat
                    });
                }
            }
        }

        // Calculate ROC (3-period)
        if prices.len() >= 4 {
            let current = prices[prices.len() - 1];
            let previous = prices[prices.len() - 4];
            if previous > Decimal::ZERO {
                let roc = ((current - previous) / previous) * Decimal::from(100);
                readings.roc_3m = Some(roc.try_into().unwrap_or(0.0));
            }
        }

        // Calculate RSI
        if let Some(rsi) = Self::calculate_rsi(&prices, RSI_PERIOD) {
            readings.rsi = Some(rsi);
        }
    }

    /// Hull Moving Average calculation
    fn calculate_hma(
        prices: &[Decimal],
        period: usize,
    ) -> Option<Decimal> {
        if prices.len() < period {
            return None;
        }

        let half_period = period / 2;
        let sqrt_period = (period as f64).sqrt() as usize;

        // WMA of half period
        let wma_half = Self::calculate_wma(
            &prices[prices.len() - half_period..],
            half_period,
        )?;

        // WMA of full period
        let wma_full = Self::calculate_wma(
            &prices[prices.len() - period..],
            period,
        )?;

        // Raw HMA = 2 * WMA_half - WMA_full
        let raw_hma = wma_half * Decimal::from(2) - wma_full;

        Some(raw_hma)
    }

    /// Weighted Moving Average
    fn calculate_wma(
        prices: &[Decimal],
        period: usize,
    ) -> Option<Decimal> {
        if prices.len() < period {
            return None;
        }

        let mut weighted_sum = Decimal::ZERO;
        let mut weight_sum = Decimal::ZERO;

        for (i, &price) in prices.iter().enumerate() {
            let weight = Decimal::from(i + 1);
            weighted_sum += price * weight;
            weight_sum += weight;
        }

        if weight_sum > Decimal::ZERO {
            Some(weighted_sum / weight_sum)
        } else {
            None
        }
    }

    /// RSI calculation
    fn calculate_rsi(
        prices: &[Decimal],
        period: usize,
    ) -> Option<f64> {
        if prices.len() <= period {
            return None;
        }

        let mut gains = 0.0f64;
        let mut losses = 0.0f64;

        for i in 1..=period {
            let current: f64 = prices[prices.len() - i].try_into().ok()?;
            let previous: f64 = prices[prices.len() - i - 1].try_into().ok()?;
            
            let change = current - previous;
            if change > 0.0 {
                gains += change;
            } else {
                losses += change.abs();
            }
        }

        let avg_gain = gains / period as f64;
        let avg_loss = losses / period as f64;

        if avg_loss == 0.0 {
            return Some(100.0);
        }

        let rs = avg_gain / avg_loss;
        let rsi = 100.0 - (100.0 / (1.0 + rs));
        
        Some(rsi)
    }

    /// Calculate tape reader indicators (OBI, spread)
    async fn calculate_tape_reader(
        &self,
        readings: &mut AnalystReadings,
    ) {
        let ob = self.order_book.read().await;
        
        if ob.bids.is_empty() || ob.asks.is_empty() {
            return;
        }

        // Calculate OBI (Order Book Imbalance)
        let bid_depth: Decimal = ob.bids.iter().take(10).map(|l| l.quantity).sum();
        let ask_depth: Decimal = ob.asks.iter().take(10).map(|l| l.quantity).sum();
        
        let total_depth = bid_depth + ask_depth;
        if total_depth > Decimal::ZERO {
            let obi = (bid_depth - ask_depth) / total_depth;
            readings.obi = Some(obi.try_into().unwrap_or(0.0));
            readings.obi_normalized = Some(
                ((obi + Decimal::ONE) / Decimal::from(2)).try_into().unwrap_or(0.5)
            );

            // Classify pressure
            readings.pressure = Some(if obi > Decimal::from(0.6) {
                Pressure::StrongBuy
            } else if obi > Decimal::from(0.2) {
                Pressure::Buy
            } else if obi < Decimal::from(-0.6) {
                Pressure::StrongSell
            } else if obi < Decimal::from(-0.2) {
                Pressure::Sell
            } else {
                Pressure::Neutral
            });
        }

        // Calculate spread
        let best_bid = ob.bids[0].price;
        let best_ask = ob.asks[0].price;
        let mid = (best_bid + best_ask) / Decimal::from(2);
        
        if mid > Decimal::ZERO {
            let spread = (best_ask - best_bid) / mid * Decimal::from(10000);
            readings.spread_bps = Some(spread.try_into().unwrap_or(0.0));
        }
    }

    /// Calculate microstructure (VPIN, volatility)
    async fn calculate_microstructure(
        &self,
        readings: &mut AnalystReadings,
    ) {
        let trades = self.trade_history.read().await;
        
        if trades.len() < 20 {
            return;
        }

        // Simplified VPIN - buy/sell volume imbalance
        let mut buy_volume = Decimal::ZERO;
        let mut sell_volume = Decimal::ZERO;
        
        for trade in trades.iter() {
            if trade.is_buy {
                buy_volume += trade.quantity;
            } else {
                sell_volume += trade.quantity;
            }
        }
        
        let total_volume = buy_volume + sell_volume;
        if total_volume > Decimal::ZERO {
            let vpin = ((buy_volume - sell_volume).abs() / total_volume)
                .try_into()
                .unwrap_or(0.0);
            readings.vpin = Some(vpin);
            
            readings.toxicity = Some(if vpin > 0.7 {
                Toxicity::Elevated
            } else {
                Toxicity::Normal
            });
        }

        // Calculate realized volatility
        if trades.len() >= VOLATILITY_WINDOW {
            let returns: Vec<f64> = trades
                .windows(2)
                .map(|w| {
                    let p1: f64 = w[0].price.try_into().unwrap_or(0.0);
                    let p2: f64 = w[1].price.try_into().unwrap_or(0.0);
                    if p1 > 0.0 {
                        (p2 - p1) / p1
                    } else {
                        0.0
                    }
                })
                .collect();

            if !returns.is_empty() {
                let mean = returns.iter().sum::<f64>() / returns.len() as f64;
                let variance = returns
                    .iter()
                    .map(|r| (r - mean).powi(2))
                    .sum::<f64>() / returns.len() as f64;
                let volatility = variance.sqrt() * 100.0;
                
                readings.volatility = Some(volatility);

                // Classify regime
                readings.volatility_regime = Some(if volatility > 2.0 {
                    VolatilityRegime::Expanding
                } else if volatility < 0.5 {
                    VolatilityRegime::Compressing
                } else {
                    VolatilityRegime::Normal
                });
            }
        }
    }

    /// Calculate liquidation stats
    async fn calculate_liquidations(
        &self,
        readings: &mut AnalystReadings,
    ) {
        let liqs = self.liquidations.read().await;
        
        let now = chrono::Utc::now().timestamp();
        let one_minute_ago = now - 60;
        
        let (long_liq, short_liq): (Decimal, Decimal) = liqs
            .iter()
            .filter(|l| l.timestamp.timestamp() > one_minute_ago)
            .fold((Decimal::ZERO, Decimal::ZERO), |(long, short), liq| {
                if liq.side == "SELL" {
                    // SELL liquidation = longs getting liquidated
                    (long + liq.usd_value, short)
                } else {
                    // BUY liquidation = shorts getting liquidated
                    (long, short + liq.usd_value)
                }
            });

        readings.long_liquidations_1m = Some(long_liq.try_into().unwrap_or(0.0));
        readings.short_liquidations_1m = Some(short_liq.try_into().unwrap_or(0.0));
        readings.net_liquidation_pressure = Some(
            (long_liq - short_liq).try_into().unwrap_or(0.0)
        );
    }

    /// Calculate multi-asset correlations
    async fn calculate_correlations(
        &self,
        readings: &mut AnalystReadings,
    ) {
        let prices = self.multi_asset_prices.read().await;
        
        // Get BTC history
        let btc_history = match prices.get("BTC") {
            Some(h) if h.len() >= 20 => h.iter().copied().collect::<Vec<_>>(),
            _ => return,
        };

        // Calculate correlation with each asset
        for (asset, history) in prices.iter() {
            if asset == "BTC" || history.len() < 20 {
                continue;
            }

            let asset_history: Vec<Decimal> = history.iter().copied().collect();
            
            if let Some(corr) = Self::calculate_correlation(
                &btc_history,
                &asset_history,
            ) {
                readings.correlations.insert(asset.clone(), corr);
            }
        }
    }

    /// Pearson correlation coefficient
    fn calculate_correlation(
        x: &[Decimal],
        y: &[Decimal],
    ) -> Option<f64> {
        let n = x.len().min(y.len());
        if n < 10 {
            return None;
        }

        let x: Vec<f64> = x.iter().take(n)
            .filter_map(|d| (*d).try_into().ok())
            .collect();
        let y: Vec<f64> = y.iter().take(n)
            .filter_map(|d| (*d).try_into().ok())
            .collect();

        if x.len() != y.len() || x.len() < 10 {
            return None;
        }

        let mean_x = x.iter().sum::<f64>() / x.len() as f64;
        let mean_y = y.iter().sum::<f64>() / y.len() as f64;

        let numerator: f64 = x.iter().zip(y.iter())
            .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
            .sum();

        let sum_sq_x: f64 = x.iter()
            .map(|xi| (xi - mean_x).powi(2))
            .sum();
        let sum_sq_y: f64 = y.iter()
            .map(|yi| (yi - mean_y).powi(2))
            .sum();

        let denominator = (sum_sq_x * sum_sq_y).sqrt();

        if denominator == 0.0 {
            return Some(0.0);
        }

        Some(numerator / denominator)
    }

    /// Calculate liquidity map (voids, walls)
    async fn calculate_liquidity_map(
        &self,
        readings: &mut AnalystReadings,
    ) {
        let ob = self.order_book.read().await;
        
        if ob.bids.len() < 2 || ob.asks.len() < 2 {
            return;
        }

        // Find liquidity voids (gaps in order book)
        let mut void_above = None;
        let mut void_below = None;

        // Check ask side for voids above current price
        for i in 0..ob.asks.len() - 1 {
            let current = ob.asks[i].price;
            let next = ob.asks[i + 1].price;
            let gap = (next - current) / current;
            
            if gap > Decimal::from(0.001) { // 0.1% gap
                void_above = Some(gap * Decimal::from(100));
                break;
            }
        }

        // Check bid side for voids below current price
        for i in 0..ob.bids.len() - 1 {
            let current = ob.bids[i].price;
            let next = ob.bids[i + 1].price;
            let gap = (current - next) / current;
            
            if gap > Decimal::from(0.001) {
                void_below = Some(gap * Decimal::from(100));
                break;
            }
        }

        readings.liquidity_void_above_pct = void_above
            .and_then(|v| v.try_into().ok());
        readings.liquidity_void_below_pct = void_below
            .and_then(|v| v.try_into().ok());
    }

    /// Calculate pinning risk
    async fn calculate_pinning_risk(
        &self,
        readings: &mut AnalystReadings,
    ) {
        // Get OBI velocity if we have history
        let obi_current = readings.obi.unwrap_or(0.0);
        let spread = readings.spread_bps.unwrap_or(0.0);
        let volatility = readings.volatility.unwrap_or(0.0);

        let mut risk_score = 0u8;

        // High OBI with wide spread
        if obi_current.abs() > 0.7 && spread > 10.0 {
            risk_score += 40;
        }

        // Extreme volatility
        if volatility > 2.0 {
            risk_score += 20;
        }

        // Wide spread alone
        if spread > 15.0 {
            risk_score += 20;
        }

        readings.pinning_risk_score = Some(risk_score);
        readings.pinning_classification = Some(match risk_score {
            70..=100 => PinningClassification::HighBreak,
            40..=69 => PinningClassification::HighHold,
            20..=39 => PinningClassification::Elevated,
            _ => PinningClassification::Low,
        });
    }

    /// Get current readings (for other components to read)
    pub async fn get_readings(&self
    ) -> AnalystReadings {
        self.current_readings.read().await.clone()
    }
}

impl Default for AnalystEngine {
    fn default() -> Self {
        Self::new()
    }
}
