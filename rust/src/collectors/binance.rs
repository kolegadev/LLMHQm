//! Layer A: Real-Time WebSocket Collectors
//!
//! Collects market data from Binance (Spot + Futures)
//! - Spot: BTC, ETH, XRP, SOL, MATIC prices
//! - Futures: Liquidations (@forceOrder), Mark prices (@markPrice)

use crate::types::*;
use crate::db::Database;
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// Binance Spot WebSocket URL
const BINANCE_SPOT_WS: &str = "wss://stream.binance.com:9443/ws";

/// Binance Futures WebSocket URL  
const BINANCE_FUTURES_WS: &str = "wss://fstream.binance.com/ws";

/// Multi-stream WebSocket collector
pub struct BinanceCollector {
    /// Spot price updates
    spot_prices: Arc<RwLock<std::collections::HashMap<String, Decimal>>>,
    /// Perp price updates
    perp_prices: Arc<RwLock<std::collections::HashMap<String, Decimal>>>,
    /// Liquidation events
    liquidation_tx: mpsc::Sender<LiquidationEvent>,
    /// Database for persistence
    db: Option<Arc<Database>>,
    /// Running flag
    running: Arc<RwLock<bool>>,
}

/// Liquidation event from @forceOrder stream
#[derive(Debug, Clone)]
pub struct LiquidationEvent {
    pub timestamp: chrono::DateTime<Utc>,
    pub symbol: String,
    pub side: String, // "BUY" = short liquidated, "SELL" = long liquidated
    pub price: Decimal,
    pub quantity: Decimal,
    pub usd_value: Decimal,
}

impl BinanceCollector {
    pub fn new(
        liquidation_tx: mpsc::Sender<LiquidationEvent>,
        db: Option<Arc<Database>>,
    ) -> Self {
        Self {
            spot_prices: Arc::new(RwLock::new(std::collections::HashMap::new())),
            perp_prices: Arc::new(RwLock::new(std::collections::HashMap::new())),
            liquidation_tx,
            db,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start all WebSocket connections
    pub async fn start(&self,
    ) -> anyhow::Result<()> {
        let mut running = self.running.write().await;
        *running = true;
        drop(running);

        info!("Starting Binance WebSocket collectors...");

        // Start spot streams (BTC, ETH, XRP, SOL, MATIC)
        let spot_handle = self.start_spot_streams();

        // Start futures streams (liquidations + mark prices)
        let futures_handle = self.start_futures_streams();

        // Wait for both
        tokio::try_join!(spot_handle, futures_handle)?;

        Ok(())
    }

    /// Start spot price streams
    async fn start_spot_streams(&self
    ) -> anyhow::Result<()> {
        let streams = vec![
            "btcusdt@trade",
            "ethusdt@trade", 
            "xrpusdt@trade",
            "solusdt@trade",
            "maticusdt@trade",
        ];

        let stream_path = streams.join("/");
        let url = format!("{}/{}", BINANCE_SPOT_WS, stream_path);

        info!("Connecting to Binance Spot streams: {}", streams.join(", "));

        let (ws_stream, _) = connect_async(&url).await?;
        let (mut write, mut read) = ws_stream.split();

        let spot_prices = Arc::clone(&self.spot_prices);
        let running = Arc::clone(&self.running);
        let db = self.db.clone();

        tokio::spawn(async move {
            while *running.read().await {
                match read.next().await {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = Self::handle_spot_message(
                            &text,
                            &spot_prices,
                            db.as_ref(),
                        ).await {
                            debug!("Error handling spot message: {}", e);
                        }
                    }
                    Some(Ok(Message::Ping(_))) => {
                        let _ = write.send(Message::Pong(vec![])).await;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        warn!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Handle spot trade message
    async fn handle_spot_message(
        text: &str,
        spot_prices: &Arc<RwLock<std::collections::HashMap<String, Decimal>>>,
        _db: Option&Arc<Database>>,
    ) -> anyhow::Result<()> {
        #[derive(Deserialize)]
        struct TradeMsg {
            #[serde(rename = "s")]
            symbol: String,
            #[serde(rename = "p")]
            price: String,
            #[serde(rename = "q")]
            quantity: String,
            #[serde(rename = "T")]
            trade_time: i64,
            #[serde(rename = "m")]
            is_buyer_maker: bool,
        }

        let msg: TradeMsg = serde_json::from_str(text)?;
        let price = Decimal::from_str_exact(&msg.price)?;
        
        // Extract asset name (e.g., "BTCUSDT" -> "BTC")
        let asset = msg.symbol
            .trim_end_matches("USDT")
            .to_uppercase();

        // Update price
        {
            let mut prices = spot_prices.write().await;
            prices.insert(asset.clone(), price);
        }

        debug!("Spot update: {} @ ${}", asset, price);

        // TODO: Store in database if needed
        // if let Some(db) = db {
        //     let tick = MarketTick { ... };
        //     db.store_tick(&tick).await?;
        // }

        Ok(())
    }

    /// Start futures streams (liquidations + mark prices)
    async fn start_futures_streams(&self
    ) -> anyhow::Result<()> {
        // Liquidations
        let liq_handle = self.start_liquidation_stream();
        
        // Mark prices
        let mark_handle = self.start_mark_price_stream();

        tokio::try_join!(liq_handle, mark_handle)?;

        Ok(())
    }

    /// Start liquidation stream (@forceOrder)
    async fn start_liquidation_stream(&self
    ) -> anyhow::Result<()> {
        let url = format!("{}/btcusdt@forceOrder", BINANCE_FUTURES_WS);
        
        info!("Connecting to Binance Futures liquidation stream");

        let (ws_stream, _) = connect_async(&url).await?;
        let (mut write, mut read) = ws_stream.split();

        let liquidation_tx = self.liquidation_tx.clone();
        let running = Arc::clone(&self.running);

        tokio::spawn(async move {
            while *running.read().await {
                match read.next().await {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = Self::handle_liquidation_message(
                            &text,
                            &liquidation_tx,
                        ).await {
                            debug!("Error handling liquidation: {}", e);
                        }
                    }
                    Some(Ok(Message::Ping(_))) => {
                        let _ = write.send(Message::Pong(vec![])).await;
                    }
                    Some(Err(e)) => {
                        error!("Liquidation WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        warn!("Liquidation stream ended");
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Handle liquidation message
    async fn handle_liquidation_message(
        text: &str,
        tx: &mpsc::Sender<LiquidationEvent>,
    ) -> anyhow::Result<()> {
        #[derive(Deserialize)]
        struct ForceOrderMsg {
            #[serde(rename = "E")]
            event_time: i64,
            #[serde(rename = "o")]
            order: OrderDetails,
        }

        #[derive(Deserialize)]
        struct OrderDetails {
            #[serde(rename = "s")]
            symbol: String,
            #[serde(rename = "S")]
            side: String, // "BUY" or "SELL"
            #[serde(rename = "p")]
            price: String,
            #[serde(rename = "q")]
            quantity: String,
        }

        let msg: ForceOrderMsg = serde_json::from_str(text)?;
        let price = Decimal::from_str_exact(&msg.order.price)?;
        let quantity = Decimal::from_str_exact(&msg.order.quantity)?;
        let usd_value = price * quantity;

        let event = LiquidationEvent {
            timestamp: DateTime::from_timestamp(msg.event_time / 1000, 0)
                .unwrap_or_else(|| Utc::now()),
            symbol: msg.order.symbol,
            side: msg.order.side.clone(),
            price,
            quantity,
            usd_value,
        };

        // Log significant liquidations (>$10k)
        if usd_value >= Decimal::from(10000) {
            let liq_type = if msg.order.side == "BUY" {
                "SHORT LIQUIDATION"
            } else {
                "LONG LIQUIDATION"
            };
            
            info!(
                "🚨 {}: {} {} @ ${} (${} USD)",
                liq_type,
                msg.order.symbol,
                quantity,
                price,
                usd_value
            );
        }

        // Send to channel
        if let Err(e) = tx.send(event).await {
            warn!("Failed to send liquidation event: {}", e);
        }

        Ok(())
    }

    /// Start mark price stream (@markPrice)
    async fn start_mark_price_stream(&self
    ) -> anyhow::Result<()> {
        let url = format!("{}/btcusdt@markPrice@1s", BINANCE_FUTURES_WS);
        
        info!("Connecting to Binance Futures mark price stream");

        let (ws_stream, _) = connect_async(&url).await?;
        let (mut write, mut read) = ws_stream.split();

        let perp_prices = Arc::clone(&self.perp_prices);
        let running = Arc::clone(&self.running);

        tokio::spawn(async move {
            while *running.read().await {
                match read.next().await {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = Self::handle_mark_price_message(
                            &text,
                            &perp_prices,
                        ).await {
                            debug!("Error handling mark price: {}", e);
                        }
                    }
                    Some(Ok(Message::Ping(_))) => {
                        let _ = write.send(Message::Pong(vec![])).await;
                    }
                    Some(Err(e)) => {
                        error!("Mark price WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        warn!("Mark price stream ended");
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Handle mark price message
    async fn handle_mark_price_message(
        text: &str,
        perp_prices: &Arc<RwLock<std::collections::HashMap<String, Decimal>>>,
    ) -> anyhow::Result<()> {
        #[derive(Deserialize)]
        struct MarkPriceMsg {
            #[serde(rename = "s")]
            symbol: String,
            #[serde(rename = "p")]
            mark_price: String,
            #[serde(rename = "i")]
            index_price: String,
            #[serde(rename = "r")]
            funding_rate: String,
        }

        let msg: MarkPriceMsg = serde_json::from_str(text)?;
        let mark_price = Decimal::from_str_exact(&msg.mark_price)?;

        // Extract asset name
        let asset = msg.symbol
            .trim_end_matches("USDT")
            .to_uppercase();

        // Update price
        {
            let mut prices = perp_prices.write().await;
            prices.insert(asset, mark_price);
        }

        debug!("Perp mark price: {} @ ${}", msg.symbol, mark_price);

        Ok(())
    }

    /// Get current spot price for an asset
    pub async fn get_spot_price(
        &self,
        asset: &str
    ) -> Option<Decimal> {
        let prices = self.spot_prices.read().await;
        prices.get(asset).copied()
    }

    /// Get current perp price for an asset
    pub async fn get_perp_price(
        &self,
        asset: &str
    ) -> Option<Decimal> {
        let prices = self.perp_prices.read().await;
        prices.get(asset).copied()
    }

    /// Stop all collectors
    pub async fn stop(&self
    ) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Stopping Binance collectors...");
    }
}

/// Spawn the collector and return handles
pub async fn spawn_collector(
    db: Option<Arc<Database>>,
) -> (BinanceCollector, mpsc::Receiver<LiquidationEvent>) {
    let (tx, rx) = mpsc::channel<LiquidationEvent>(100);
    let collector = BinanceCollector::new(tx, db);
    (collector, rx)
}
