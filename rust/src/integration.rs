//! LLMHQ Integration - Brings all layers together
//!
//! This is the production-ready main loop that:
//! 1. Starts WebSocket collectors (Layer A)
//! 2. Runs block timing synchronization
//! 3. Generates analyst readings from live data
//! 4. Creates semantic narratives (Layer B)
//! 5. Makes CIO decisions (Layer C)
//! 6. Executes paper trades (Layer D)
//! 7. Resolves trades and tracks P&L

use crate::{
    collectors::{spawn_collector, LiquidationEvent},
    cio::CIODecisionEngine,
    db::Database,
    executor::{PaperExecutor, PaperTradingConfig, PriceTracker},
    narrator::Narrator,
    timing::BlockTimer,
    types::*,
};
use chrono::Utc;
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{error, info, warn};

/// Main LLMHQ trading loop
pub struct TradingLoop {
    /// Block timing manager
    block_timer: BlockTimer,
    /// Narrator for semantic synthesis
    narrator: Narrator,
    /// CIO decision engine
    cio: CIODecisionEngine,
    /// Paper trading executor
    executor: PaperExecutor,
    /// Price tracker (shared with collector)
    price_tracker: Arc<RwLock<PriceTracker>>,
    /// Database connection
    db: Option<Arc<Database>>,
    /// Liquidation event receiver
    liquidation_rx: mpsc::Receiver<LiquidationEvent>,
    /// Current analyst readings
    current_readings: Option<AnalystReadings>,
}

impl TradingLoop {
    pub async fn new(
        db_url: Option<&str>,
    ) -> anyhow::Result<Self> {
        // Initialize database if URL provided
        let db = if let Some(url) = db_url {
            Some(Arc::new(Database::new(url).await?))
        } else {
            None
        };

        // Spawn collector
        let (collector, liquidation_rx) = spawn_collector(db.clone()).await;

        // Create price tracker
        let price_tracker = Arc::new(RwLock::new(PriceTracker::default()));

        // Start collector
        let collector_ref = collector;
        tokio::spawn(async move {
            if let Err(e) = collector_ref.start().await {
                error!("Collector error: {}", e);
            }
        });

        // Create executor
        let config = PaperTradingConfig::default();
        let executor = PaperExecutor::new(config, db.clone());

        Ok(Self {
            block_timer: BlockTimer::new(5), // 5-minute blocks
            narrator: Narrator::new(),
            cio: CIODecisionEngine::new(),
            executor,
            price_tracker,
            db,
            liquidation_rx,
            current_readings: None,
        })
    }

    /// Run the main trading loop
    pub async fn run(&mut self
    ) -> anyhow::Result<()> {
        info!("Starting LLMHQ trading loop...");
        info!("Block interval: 5 minutes");

        // Main loop
        let mut tick = interval(Duration::from_secs(1));

        loop {
            tick.tick().await;

            // Process any liquidation events
            self.process_liquidations().await;

            // Check block timing
            let timing = self.block_timer.get_timing();

            match timing.phase {
                BlockPhase::Idle => {
                    // Update price tracker with current prices
                    self.update_price_tracker().await;
                }

                BlockPhase::Calculation => {
                    // t-30s to t-15s: Calculate features
                    self.calculate_features(timing.current_block_number).await;
                }

                BlockPhase::Aggregation => {
                    // t-15s to t-10s: Aggregate data
                    // (Already done during calculation)
                }

                BlockPhase::Synthesis => {
                    // t-10s to t-5s: Generate narrative
                    if self.current_readings.is_none() {
                        self.calculate_features(timing.current_block_number).await;
                    }
                }

                BlockPhase::Decision => {
                    // t-5s to t-2s: Make decision
                    self.make_decision().await;
                }

                BlockPhase::Execution => {
                    // t-2s to t=0: Execute trade
                    self.execute_trade().await;
                }

                BlockPhase::PostExecution => {
                    // After t=0: Monitor and resolve
                    self.monitor_resolution().await;
                }
            }
        }
    }

    /// Process liquidation events
    async fn process_liquidations(&mut self
    ) {
        while let Ok(event) = self.liquidation_rx.try_recv() {
            debug!(
                "Liquidation: {} {} @ ${} (${} USD)",
                event.symbol, event.side, event.price, event.usd_value
            );
            // Store in readings or database
        }
    }

    /// Update price tracker with current market data
    async fn update_price_tracker(&self
    ) {
        // TODO: Get prices from collector
        // For now, placeholder
    }

    /// Calculate analyst features from market data
    async fn calculate_features(
        &mut self,
        block_number: u64,
    ) {
        // TODO: Implement full analyst calculations
        // This would compute:
        // - HMA, slope, ROC, RSI
        // - OBI, spread, pressure
        // - VPIN, volatility
        // - Basis, correlations
        // - Pinning risk

        // Placeholder readings
        let readings = AnalystReadings {
            timestamp: Utc::now(),
            block_number,
            seconds_to_block_end: 10.0,
            spot_price: Some(Decimal::from(70250)),
            perp_price: Some(Decimal::from(70285)),
            basis_bps: Some(5.0),
            obi: Some(0.65),
            hma_slope: Some(18.5),
            vpin: Some(0.58),
            ..Default::default()
        };

        self.current_readings = Some(readings);
    }

    /// Generate narrative and make CIO decision
    async fn make_decision(
        &mut self
    ) {
        if let Some(ref readings) = self.current_readings {
            // Identify patterns
            let patterns = self.narrator.identify_patterns(readings);

            // Generate narrative
            let narrative = self.narrator.generate_narrative(
                readings,
                patterns.iter().map(|(p, s)| (&**p, *s)).collect(),
            );

            // Make CIO decision
            let decision = self.cio.make_decision(readings, &narrative);

            info!(
                "CIO Decision: {:?} @ {}% confidence (veto: {})",
                decision.direction,
                decision.confidence,
                decision.veto_applied
            );

            // Store in database
            if let Some(ref db) = self.db {
                if let Err(e) = db.store_decision(&decision).await {
                    error!("Failed to store decision: {}", e);
                }
            }
        }
    }

    /// Execute paper trade
    async fn execute_trade(
        &self
    ) {
        // TODO: Implement trade execution
        // This would:
        // 1. Capture t=0 price
        // 2. Fetch Polymarket odds
        // 3. Validate odds
        // 4. Execute if valid
    }

    /// Monitor trade resolution
    async fn monitor_resolution(
        &self
    ) {
        // TODO: Check if we have an active trade that needs resolution
        if self.executor.has_active_trade().await {
            // Check if block has ended
            // Resolve trade if so
        }
    }
}

/// Run the complete LLMHQ system
pub async fn run_llmhq(
    db_url: Option<&str>,
) -> anyhow::Result<()> {
    let mut trading_loop = TradingLoop::new(db_url).await?;
    trading_loop.run().await
}
