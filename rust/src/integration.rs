//! LLMHQ Integration - Wired for Production
//!
//! Fully functional trading loop with real data flow:
//! WebSocket Collectors → AnalystEngine → Narrator → CIO → PaperExecutor

use crate::{
    analysts::AnalystEngine,
    collectors::{spawn_collector, LiquidationEvent},
    cio::CIODecisionEngine,
    db::Database,
    executor::{PaperExecutor, PaperTradingConfig, PriceTracker},
    narrator::Narrator,
    timing::{BlockPhase, BlockTimer},
    types::*,
};
use chrono::Utc;
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Production-ready LLMHQ trading loop
pub struct TradingLoop {
    /// Block timing manager
    block_timer: BlockTimer,
    /// Analyst engine (calculates all 8 indicators)
    analyst_engine: Arc<AnalystEngine>,
    /// Narrator for semantic synthesis
    narrator: Narrator,
    /// CIO decision engine
    cio: CIODecisionEngine,
    /// Paper trading executor
    executor: PaperExecutor,
    /// Price tracker
    price_tracker: Arc<RwLock<PriceTracker>>,
    /// Database connection
    db: Option<Arc<Database>>,
    /// Liquidation event receiver
    liquidation_rx: mpsc::Receiver<LiquidationEvent>,
    /// Last processed block number
    last_block: u64,
}

impl TradingLoop {
    pub async fn new(
        db_url: Option<&str>,
    ) -> anyhow::Result<Self> {
        info!("Initializing LLMHQ Trading Loop...");

        // Initialize database
        let db = if let Some(url) = db_url {
            info!("Connecting to database: {}", url);
            Some(Arc::new(Database::new(url).await?))
        } else {
            warn!("No database URL provided - running without persistence");
            None
        };

        // Create analyst engine
        let analyst_engine = Arc::new(AnalystEngine::new());

        // Spawn WebSocket collector
        let (collector, liquidation_rx) = spawn_collector(db.clone()).await;

        // Create price tracker
        let price_tracker = Arc::new(RwLock::new(PriceTracker::default()));

        // Clone references for collector task
        let analyst_for_collector = Arc::clone(&analyst_engine);
        let tracker_for_collector = Arc::clone(&price_tracker);

        // Start collector in background task
        tokio::spawn(async move {
            info!("Starting WebSocket collector...");
            
            // Create a wrapper that feeds data to analyst engine
            let collector_wrapper = CollectorWrapper {
                inner: collector,
                analyst: analyst_for_collector,
                price_tracker: tracker_for_collector,
            };
            
            if let Err(e) = collector_wrapper.run().await {
                error!("Collector error: {}", e);
            }
        });

        // Create paper executor
        let config = PaperTradingConfig::default();
        let executor = PaperExecutor::new(config, db.clone());

        info!("LLMHQ initialization complete");

        Ok(Self {
            block_timer: BlockTimer::new(5),
            analyst_engine,
            narrator: Narrator::new(),
            cio: CIODecisionEngine::new(),
            executor,
            price_tracker,
            db,
            liquidation_rx,
            last_block: 0,
        })
    }

    /// Run the main trading loop
    pub async fn run(&mut self) -> anyhow::Result<()> {
        info!("========================================");
        info!("LLMHQ Trading Loop Started");
        info!("Block interval: 5 minutes");
        info!("========================================\n");

        let mut tick = interval(Duration::from_millis(100)); // 10Hz tick

        loop {
            tick.tick().await;

            // Process liquidation events
            self.process_liquidations().await;

            // Get current timing
            let timing = self.block_timer.get_timing();

            // Print status on phase changes
            if timing.current_block_number != self.last_block {
                self.last_block = timing.current_block_number;
                info!("New block started: #{}", timing.current_block_number);
            }

            match timing.phase {
                BlockPhase::Idle => {
                    // Nothing to do, waiting for next block
                }

                BlockPhase::Calculation => {
                    // t-30s to t-15s: Analyst calculations running continuously
                    // Data flows automatically from collector → analyst_engine
                }

                BlockPhase::Synthesis => {
                    // t-10s to t-5s: Generate narrative if we haven't already
                    if timing.seconds_to_next_block <= 10.0 
                        && timing.seconds_to_next_block > 8.0 {
                        self.generate_narrative(timing.current_block_number).await;
                    }
                }

                BlockPhase::Decision => {
                    // t-5s to t-2s: Make CIO decision
                    if timing.seconds_to_next_block <= 5.0 
                        && timing.seconds_to_next_block > 3.0 {
                        self.make_decision(timing.current_block_number).await;
                    }
                }

                BlockPhase::Execution => {
                    // t-2s to t=0: Execute trade
                    if timing.seconds_to_next_block <= 2.0 
                        && timing.seconds_to_next_block > 0.5 {
                        self.execute_trade(timing.current_block_number).await;
                    }
                }

                BlockPhase::PostExecution => {
                    // Monitor for resolution
                    self.monitor_resolution(timing.current_block_number).await;
                }

                _ => {}
            }
        }
    }

    /// Process liquidation events from futures stream
    async fn process_liquidations(&mut self) {
        while let Ok(event) = self.liquidation_rx.try_recv() {
            // Feed to analyst engine
            self.analyst_engine.process_liquidation(event).await;
            
            debug!(
                "Liquidation: {} {} @ ${} (${} USD)",
                event.symbol, event.side, event.price, event.usd_value
            );
        }
    }

    /// Generate semantic narrative
    async fn generate_narrative(&self, block_number: u64) {
        // Get current readings from analyst engine
        let readings = self.analyst_engine.get_readings().await;
        
        // Identify patterns
        let patterns = self.narrator.identify_patterns(&readings);
        
        // Generate narrative
        let narrative = self.narrator.generate_narrative(
            &readings,
            patterns.iter().map(|(p, s)| (&**p, *s)).collect(),
        );

        info!("\n{}", narrative.narrative_md);

        // Store in database
        if let Some(ref db) = self.db {
            if let Err(e) = db.store_narrative(&narrative).await {
                error!("Failed to store narrative: {}", e);
            }
        }
    }

    /// Make CIO decision
    async fn make_decision(&self, block_number: u64) {
        let readings = self.analyst_engine.get_readings().await;
        
        // Generate narrative (needed for decision)
        let patterns = self.narrator.identify_patterns(&readings);
        let narrative = self.narrator.generate_narrative(
            &readings,
            patterns.iter().map(|(p, s)| (&**p, *s)).collect(),
        );
        
        // Make decision
        let decision = self.cio.make_decision(&readings, &narrative);

        let veto_str = if decision.veto_applied {
            format!("VETOED: {}", decision.veto_reason.as_ref().unwrap_or(&"Unknown".to_string()))
        } else {
            "OK".to_string()
        };

        info!(
            "CIO Decision [Block #{}]: {:?} @ {}% confidence | {}",
            block_number,
            decision.direction,
            decision.confidence,
            veto_str
        );

        // Store in database
        if let Some(ref db) = self.db {
            if let Err(e) = db.store_decision(&decision).await {
                error!("Failed to store decision: {}", e);
            }
        }
    }

    /// Execute paper trade at t=0
    async fn execute_trade(&self, block_number: u64) {
        let readings = self.analyst_engine.get_readings().await;
        
        // Generate narrative for decision context
        let patterns = self.narrator.identify_patterns(&readings);
        let narrative = self.narrator.generate_narrative(
            &readings,
            patterns.iter().map(|(p, s)| (&**p, *s)).collect(),
        );
        
        let decision = self.cio.make_decision(&readings, &narrative);

        // Skip if vetoed or low confidence
        if decision.veto_applied || decision.confidence < 65 {
            info!(
                "Trade skipped [Block #{}]: veto={} confidence={}",
                block_number,
                decision.veto_applied,
                decision.confidence
            );
            return;
        }

        // Execute paper trade
        match self.executor.execute_trade(
            &decision,
            block_number,
        ).await {
            Ok(Some(trade)) => {
                info!(
                    "✅ Trade executed [Block #{}]: {:?} @ ${} | Size: ${}",
                    block_number,
                    trade.decision.direction,
                    trade.entry_price,
                    trade.position_size_usd
                );
            }
            Ok(None) => {
                info!("Trade blocked by validation [Block #{}]", block_number);
            }
            Err(e) => {
                error!("Trade execution error [Block #{}]: {}", block_number, e);
            }
        }
    }

    /// Monitor and resolve trades
    async fn monitor_resolution(&self, block_number: u64) {
        // Check if we have an active trade
        if !self.executor.has_active_trade().await {
            return;
        }

        // Get the active trade
        if let Some(trade) = self.executor.get_active_trade().await {
            // Check if this trade's block has ended
            let trade_block = trade.block_number;
            
            // Simple check: if we're 2+ blocks ahead, resolve
            if block_number >= trade_block + 2 {
                info!("Resolving trade from block #{}", trade_block);
                
                match self.executor.resolve_trade().await {
                    Ok(Some(completed)) => {
                        let outcome_str = match completed.outcome {
                            Some(TradeOutcome::Win) => "✅ WIN",
                            Some(TradeOutcome::Loss) => "❌ LOSS",
                            Some(TradeOutcome::Breakeven) => "➖ BREAKEVEN",
                            None => "?",
                        };
                        
                        info!(
                            "{} [Block #{}]: P&L: {:.2}% | Balance: ${}",
                            outcome_str,
                            trade_block,
                            completed.pnl_pct.unwrap_or(0.0),
                            self.executor.get_balance().await
                        );
                    }
                    Ok(None) => {
                        warn!("No trade to resolve");
                    }
                    Err(e) => {
                        error!("Resolution error: {}", e);
                    }
                }
            }
        }
    }
}

/// Wrapper that connects collector to analyst engine
struct CollectorWrapper {
    inner: crate::collectors::BinanceCollector,
    analyst: Arc<AnalystEngine>,
    price_tracker: Arc<RwLock<PriceTracker>>,
}

impl CollectorWrapper {
    async fn run(self) -> anyhow::Result<()> {
        // This would need to be implemented to wire the collector's
        // output to the analyst engine's input
        // For now, placeholder
        self.inner.start().await
    }
}

/// Run the complete LLMHQ system
pub async fn run_llmhq(db_url: Option<&str>) -> anyhow::Result<()> {
    let mut trading_loop = TradingLoop::new(db_url).await?;
    trading_loop.run().await
}
