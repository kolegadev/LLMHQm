//! LLMHQ - LLM Heuristic Quant Trading System
//! 
//! A hybrid Rust + LLM quantitative trading system for ultra-short-horizon
//! BTC interval prediction on Polymarket-style markets.

pub mod types;
pub mod db;
pub mod narrator;
pub mod cio;
pub mod timing;

// Re-export main types
pub use types::*;
pub use narrator::Narrator;
pub use cio::CIODecisionEngine;
pub use timing::BlockTimer;

use tracing::{info, warn, error};

/// Main LLMHQ engine that orchestrates all layers
pub struct LLMHQEngine {
    /// Block timing manager
    pub timer: BlockTimer,
    /// Narrator for semantic synthesis
    pub narrator: Narrator,
    /// CIO decision engine
    pub cio: CIODecisionEngine,
    /// Current analyst readings
    pub current_readings: Option<AnalystReadings>,
    /// Current narrative
    pub current_narrative: Option<SemanticNarrative>,
    /// Current decision
    pub current_decision: Option<CIODecision>,
}

impl LLMHQEngine {
    pub fn new() -> Self {
        Self {
            timer: BlockTimer::new(5), // 5-minute blocks
            narrator: Narrator::new(),
            cio: CIODecisionEngine::new(),
            current_readings: None,
            current_narrative: None,
            current_decision: None,
        }
    }

    /// Process a complete cycle: readings → narrative → decision
    pub fn process_cycle(&mut self,
        readings: AnalystReadings,
    ) -> Option<CIODecision> {
        // Step 1: Identify patterns
        let patterns = self.narrator.identify_patterns(&readings);
        
        // Step 2: Generate semantic narrative
        let narrative = self.narrator.generate_narrative(&readings,
            patterns.iter().map(|(p, s)| (&**p, *s)).collect()
        );
        
        // Step 3: Make CIO decision
        let decision = self.cio.make_decision(&readings,
            &narrative,
        );
        
        // Store for reference
        self.current_readings = Some(readings);
        self.current_narrative = Some(narrative);
        self.current_decision = Some(decision.clone());
        
        // Only return decision if we should trade
        if decision.veto_applied || decision.confidence < 65 {
            info!("Decision filtered: veto={}, confidence={}",
                decision.veto_applied,
                decision.confidence
            );
            None
        } else {
            Some(decision)
        }
    }

    /// Check if we're in the decision window
    pub fn should_decide(&self) -> bool {
        self.timer.should_decide()
    }

    /// Get current block timing
    pub fn get_timing(&self) -> timing::BlockTiming {
        self.timer.get_timing()
    }

    /// Print current status
    pub fn print_status(&self) {
        self.timer.print_status();
        
        if let Some(ref decision) = self.current_decision {
            println!("\n📊 LAST DECISION:");
            println!("   Direction: {:?}", decision.direction);
            println!("   Confidence: {}%", decision.confidence);
            println!("   Lead Driver: {}", decision.lead_driver);
            println!("   Veto: {}", if decision.veto_applied { "YES" } else { "NO" });
        }
    }
}

impl Default for LLMHQEngine {
    fn default() -> Self {
        Self::new()
    }
}
