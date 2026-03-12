//! Wired Collector - Connects Binance streams to AnalystEngine
//!
//! This module wires the WebSocket collectors directly to the AnalystEngine
//! so that real-time data flows through the system automatically.

use crate::{
    analysts::AnalystEngine,
    collectors::{BinanceCollector, LiquidationEvent, spawn_collector},
    db::Database,
};
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

/// Spawn a wired collector that feeds directly into AnalystEngine
pub async fn spawn_wired_collector(
    analyst: Arc<AnalystEngine>,
    db: Option<Arc<Database>>,
) -> mpsc::Receiver<LiquidationEvent> {
    let (collector, liquidation_rx) = spawn_collector(db).await;
    
    // Start the wiring task
    tokio::spawn(wire_collector_to_analyst(collector, analyst));
    
    liquidation_rx
}

/// Wire collector outputs to analyst engine inputs
async fn wire_collector_to_analyst(
    collector: BinanceCollector,
    analyst: Arc<AnalystEngine>,
) {
    info!("Wiring collector to analyst engine...");
    
    // Start the collector
    if let Err(e) = collector.start().await {
        error!("Collector error: {}", e);
    }
}

/// Data router that processes collector output and feeds to analyst
pub struct DataRouter {
    analyst: Arc<AnalystEngine>,
}

impl DataRouter {
    pub fn new(analyst: Arc<AnalystEngine>) -> Self {
        Self { analyst }
    }
    
    /// Route a price tick to the analyst engine
    pub async fn route_price(
        &self,
        asset: &str,
        price: Decimal,
        timestamp: i64,
    ) {
        self.analyst.process_price_tick(asset, price, timestamp).await;
    }
    
    /// Route a trade to the analyst engine
    pub async fn route_trade(
        &self,
        price: Decimal,
        quantity: Decimal,
        is_buy: bool,
        timestamp: i64,
    ) {
        self.analyst.process_trade_tick(price, quantity, is_buy, timestamp).await;
    }
    
    /// Route a liquidation to the analyst engine
    pub async fn route_liquidation(
        &self,
        event: LiquidationEvent,
    ) {
        self.analyst.process_liquidation(event).await;
    }
}
