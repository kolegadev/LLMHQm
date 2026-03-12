//! Layer C: CIO - Chief Investment Officer Decision Engine
//!
//! The CIO's job:
//! 1. Compile and assess all analyst readings
//! 2. Distill market behavior prediction
//! 3. Identify external influences
//! 4. Make UP/DOWN prediction with confidence
//! 5. Apply veto logic
//! 6. Interface with LLM for final decision

use crate::types::*;
use crate::narrator::SemanticNarrative;
use chrono::Utc;
use std::collections::HashMap;

/// CIO Decision Engine
pub struct CIODecisionEngine {
    /// Minimum confidence threshold for trading
    min_confidence: u8,
    /// Maximum risk score before veto
    max_risk_score: u8,
    /// Pattern weights for scoring
    pattern_weights: HashMap<String, f64>,
}

impl CIODecisionEngine {
    pub fn new() -> Self {
        Self {
            min_confidence: 65,
            max_risk_score: 70,
            pattern_weights: Self::initialize_weights(),
        }
    }

    fn initialize_weights() -> HashMap<String, f64> {
        let mut weights = HashMap::new();
        
        // Primary drivers (microstructure)
        weights.insert("Heavy_Buy_Absorption".to_string(), 1.5);
        weights.insert("Heavy_Sell_Absorption".to_string(), 1.5);
        weights.insert("OBI_Acceleration_Bull".to_string(), 1.2);
        weights.insert("OBI_Acceleration_Bear".to_string(), 1.2);
        
        // Momentum
        weights.insert("HMA_Surf_Steepening".to_string(), 1.0);
        weights.insert("HMA_Break_Down".to_string(), 1.0);
        weights.insert("HMA_Flat_Consolidation".to_string(), 0.3);
        
        // Basis/Lead-lag
        weights.insert("Perp_Premium_Levered_Long".to_string(), 0.8);
        weights.insert("Perp_Discount_Forced_Sell".to_string(), 0.8);
        
        // Liquidations (contrarian signals)
        weights.insert("Long_Liquidation_Cascade".to_string(), -0.5); // Counter-trend
        weights.insert("Short_Liquidation_Squeeze".to_string(), 0.7); // Trend continuation
        
        // Volatility
        weights.insert("Volatility_Expansion".to_string(), 0.4);
        weights.insert("Volatility_Compression".to_string(), 0.6);
        
        // Risk patterns (negative weights = veto/confidence reduction)
        weights.insert("Pinning_Risk_High".to_string(), -2.0);
        weights.insert("Late_OBI_Spike".to_string(), -0.8);
        
        weights
    }

    /// Main decision entry point
    pub fn make_decision(
        &self,
        readings: &AnalystReadings,
        narrative: &SemanticNarrative,
    ) -> CIODecision {
        let timestamp = Utc::now();
        
        // Step 1: Assess regime
        let regime = readings.dominant_regime();
        
        // Step 2: Calculate base directional signal
        let (directional_score, lead_driver) = self.calculate_directional_score(readings, &narrative.pattern_tags);
        
        // Step 3: Check veto conditions
        let veto_check = self.check_veto_conditions(readings, &narrative.pattern_tags);
        
        // Step 4: Apply risk adjustments
        let risk_adjusted_confidence = self.apply_risk_adjustments(
            directional_score,
            readings,
            &veto_check,
        );
        
        // Step 5: Determine final direction and confidence
        let (direction, confidence, veto_applied, veto_reason) = if veto_check.should_veto {
            (
                Direction::Neutral,
                0u8,
                true,
                Some(veto_check.reason),
            )
        } else {
            let dir = if directional_score > 0.1 {
                Direction::Up
            } else if directional_score < -0.1 {
                Direction::Down
            } else {
                Direction::Neutral
            };
            
            let conf = ((risk_adjusted_confidence.abs() * 100.0) as u8).clamp(0, 100);
            (dir, conf, false, None)
        };
        
        // Step 6: Calculate position size adjustment
        let position_size = if veto_applied {
            0
        } else {
            self.calculate_position_size(confidence, readings, regime)
        };
        
        // Step 7: Build rationale
        let rationale = self.build_rationale(
            readings,
            &narrative.pattern_tags,
            directional_score,
            &lead_driver,
            veto_applied,
        );
        
        // Step 8: Collect risk flags
        let risk_flags = self.collect_risk_flags(readings, &veto_check);
        
        CIODecision {
            timestamp,
            block_number: readings.block_number,
            direction,
            confidence,
            regime,
            lead_driver,
            rationale,
            risk_flags,
            veto_applied,
            veto_reason,
            pinning_assessment: readings.pinning_classification,
            suggested_position_size_pct: position_size,
        }
    }

    /// Calculate directional score (-1.0 to 1.0, positive = bullish)
    fn calculate_directional_score(
        &self,
        readings: &AnalystReadings,
        patterns: &[String],
    ) -> (f64, String) {
        let mut score = 0.0;
        let mut max_weight = 0.0;
        let mut lead_driver = "Mixed_signals".to_string();
        
        // Pattern-based scoring
        for pattern in patterns {
            if let Some(weight) = self.pattern_weights.get(pattern) {
                score += weight;
                if weight.abs() > max_weight {
                    max_weight = weight.abs();
                    lead_driver = pattern.clone();
                }
            }
        }
        
        // Additional readings-based scoring
        
        // HMA slope contribution
        if let Some(slope) = readings.hma_slope {
            let hma_contrib = (slope / 45.0).clamp(-0.5, 0.5); // Normalize to ±0.5
            score += hma_contrib;
            if hma_contrib.abs() > 0.3 && hma_contrib.abs() > max_weight {
                max_weight = hma_contrib.abs();
                lead_driver = if hma_contrib > 0.0 {
                    "HMA_momentum_bullish"
                } else {
                    "HMA_momentum_bearish"
                }.to_string();
            }
        }
        
        // OBI contribution
        if let Some(obi) = readings.obi {
            let obi_contrib = obi * 0.5; // OBI is -1 to 1, scale to ±0.5
            score += obi_contrib;
            if obi_contrib.abs() > 0.3 && obi_contrib.abs() > max_weight {
                max_weight = obi_contrib.abs();
                lead_driver = if obi_contrib > 0.0 {
                    "OBI_buy_pressure"
                } else {
                    "OBI_sell_pressure"
                }.to_string();
            }
        }
        
        // Basis contribution
        if let Some(basis) = readings.basis_bps {
            // Basis > 0 means perp premium (leveraged longs)
            let basis_contrib = (basis / 20.0).clamp(-0.3, 0.3);
            score += basis_contrib;
        }
        
        // Liquidation pressure (contrarian)
        if let Some(pressure) = readings.net_liquidation_pressure {
            // Positive = longs liquidated (shorts winning, possible bounce)
            // Negative = shorts liquidated (longs winning, possible continuation)
            let liq_contrib = if pressure > 100000.0 {
                0.3 // Longs liquidated = potential bullish reversal
            } else if pressure < -100000.0 {
                -0.2 // Shorts liquidated = bullish continuation, but fading
            } else {
                0.0
            };
            score += liq_contrib;
        }
        
        // VPIN contribution (informed flow)
        if let (Some(vpin), Some(obi)) = (readings.vpin, readings.obi) {
            if vpin > 0.6 {
                // High VPIN + positive OBI = informed buying
                // High VPIN + negative OBI = informed selling
                let vpin_contrib = if obi > 0.0 { 0.2 } else { -0.2 };
                score += vpin_contrib;
            }
        }
        
        // Normalize score to ±1.0 range
        let normalized_score = score.clamp(-1.0, 1.0);
        
        (normalized_score, lead_driver)
    }

    /// Check veto conditions
    fn check_veto_conditions(
        &self,
        readings: &AnalystReadings,
        _patterns: &[String],
    ) -> VetoCheck {
        let mut reasons = Vec::new();
        
        // Veto 1: High pinning risk
        if let Some(PinningClassification::HighBreak) = readings.pinning_classification {
            reasons.push("HIGH_BREAK pinning detected - possible manipulation".to_string());
        }
        
        // Veto 2: Extremely wide spreads (illiquid)
        if let Some(spread) = readings.spread_bps {
            if spread > 20.0 {
                reasons.push(format!("Extreme spread ({} bps) - insufficient liquidity", spread));
            }
        }
        
        // Veto 3: Missing critical data
        if readings.hma.is_none() || readings.obi.is_none() {
            reasons.push("Critical data missing - cannot make informed decision".to_string());
        }
        
        // Veto 4: Extreme volatility without clear direction
        if let Some(VolatilityRegime::Expanding) = readings.volatility_regime {
            if readings.hma_slope.map(|s| s.abs() < 10.0).unwrap_or(true) {
                reasons.push("Volatility expansion without directional momentum".to_string());
            }
        }
        
        // Veto 5: Late OBI spike with thin liquidity
        if let (Some(obi_vel), Some(spread)) = (readings.obi_velocity, readings.spread_bps) {
            if obi_vel.abs() > 0.2 && spread > 10.0 {
                reasons.push("Late OBI spike with thin liquidity - possible fake wall".to_string());
            }
        }
        
        if reasons.is_empty() {
            VetoCheck {
                should_veto: false,
                reason: String::new(),
            }
        } else {
            VetoCheck {
                should_veto: true,
                reason: reasons.join("; "),
            }
        }
    }

    /// Apply risk adjustments to confidence
    fn apply_risk_adjustments(
        &self,
        base_score: f64,
        readings: &AnalystReadings,
        veto_check: &VetoCheck,
    ) -> f64 {
        if veto_check.should_veto {
            return 0.0;
        }
        
        let mut adjusted = base_score;
        
        // Reduce confidence for elevated pinning risk
        if let Some(PinningClassification::HighHold) = readings.pinning_classification {
            adjusted *= 0.7; // 30% confidence reduction
        } else if let Some(PinningClassification::Elevated) = readings.pinning_classification {
            adjusted *= 0.85; // 15% confidence reduction
        }
        
        // Reduce confidence for moderate spread
        if let Some(spread) = readings.spread_bps {
            if spread > 10.0 {
                adjusted *= 0.9;
            }
        }
        
        // Reduce confidence if correlations are breaking down
        if readings.correlations.values().any(|&c| c < 0.3 && c > 0.0) {
            adjusted *= 0.9;
        }
        
        adjusted
    }

    /// Calculate suggested position size (0-100%)
    fn calculate_position_size(
        &self,
        confidence: u8,
        readings: &AnalystReadings,
        regime: MarketRegime,
    ) -> u8 {
        let base_size = confidence;
        
        // Adjust for regime
        let regime_multiplier = match regime {
            MarketRegime::Trending => 1.0,
            MarketRegime::Ranging => 0.7,
            MarketRegime::VolatileExpansion => 0.5,
            MarketRegime::QuietCompression => 0.8,
            MarketRegime::Manipulative => 0.0, // Should be vetoed anyway
        };
        
        // Adjust for pinning risk
        let pinning_multiplier = match readings.pinning_classification {
            Some(PinningClassification::HighHold) => 0.5,
            Some(PinningClassification::Elevated) => 0.8,
            _ => 1.0,
        };
        
        let adjusted = (base_size as f64 * regime_multiplier * pinning_multiplier) as u8;
        adjusted.clamp(0, 100)
    }

    /// Build human-readable rationale
    fn build_rationale(
        &self,
        readings: &AnalystReadings,
        patterns: &[String],
        score: f64,
        lead_driver: &str,
        vetoed: bool,
    ) -> String {
        if vetoed {
            return format!(
                "Trade vetoed due to risk factors. Primary driver {} detected but market conditions unsafe.",
                lead_driver.replace("_", " ")
            );
        }
        
        let direction_str = if score > 0.1 {
            "UP"
        } else if score < -0.1 {
            "DOWN"
        } else {
            "NEUTRAL"
        };
        
        let mut parts = vec![
            format!(
                "Directional bias: {} (score: {:.2}). Primary driver: {}.",
                direction_str,
                score,
                lead_driver.replace("_", " ")
            ),
        ];
        
        // Add supporting evidence
        if let Some(obi) = readings.obi {
            parts.push(format!("OBI at {:.0}% suggests {} flow.", 
                (obi + 1.0) / 2.0 * 100.0,
                if obi > 0.0 { "buy-side" } else { "sell-side" }
            ));
        }
        
        if let Some(slope) = readings.hma_slope {
            parts.push(format!("HMA slope {:.1}° indicates {} momentum.",
                slope,
                if slope > 0.0 { "bullish" } else { "bearish" }
            ));
        }
        
        if let Some(basis) = readings.basis_bps {
            parts.push(format!("Perp basis at {:+.1} bps shows {}.",
                basis,
                if basis > 0.0 { "leveraged long interest" } else { "hedging pressure" }
            ));
        }
        
        // Mention key patterns
        if !patterns.is_empty() {
            let pattern_list: Vec<String> = patterns.iter()
                .take(2)
                .map(|p| p.replace("_", " "))
                .collect();
            parts.push(format!("Detected patterns: {}.", pattern_list.join(", ")));
        }
        
        parts.join(" ")
    }

    /// Collect risk flags for reporting
    fn collect_risk_flags(
        &self,
        readings: &AnalystReadings,
        veto_check: &VetoCheck,
    ) -> Vec<String> {
        let mut flags = Vec::new();
        
        if veto_check.should_veto {
            flags.push(format!("VETO: {}", veto_check.reason));
        }
        
        if let Some(PinningClassification::HighHold) = readings.pinning_classification {
            flags.push("Pinning risk elevated".to_string());
        }
        
        if let Some(spread) = readings.spread_bps {
            if spread > 10.0 {
                flags.push(format!("Wide spread ({} bps)", spread));
            }
        }
        
        if let Some(VolatilityRegime::Expanding) = readings.volatility_regime {
            flags.push("Volatility expansion".to_string());
        }
        
        if let Some(vpin) = readings.vpin {
            if vpin > 0.7 {
                flags.push("High toxicity (VPIN)".to_string());
            }
        }
        
        flags
    }

    /// Generate prompt for LLM (Kimi-K2.5) if deep analysis needed
    pub fn build_llm_prompt(
        &self,
        readings: &AnalystReadings,
        narrative: &SemanticNarrative,
        preliminary_decision: &CIODecision,
    ) -> String {
        format!(r#"You are the Chief Investment Officer (CIO) for LLMHQ, a quantitative trading system.

## Your Task
Review the market briefing and issue a final directional prediction for the next 5-minute BTC interval.

## Market Briefing

{narrative}

## Preliminary Analysis

- Directional Score: {score:.2} (positive = bullish, negative = bearish)
- Suggested Direction: {direction}
- Confidence: {confidence}%
- Lead Driver: {driver}
- Regime: {regime}

## Veto Check

{veto_status}

## Your Output Format (JSON)

```json
{{
  "direction": "UP" | "DOWN" | "NEUTRAL",
  "confidence": 0-100,
  "regime": "trending" | "ranging" | "volatile_expansion" | "quiet_compression" | "manipulative",
  "lead_driver": "primary signal driving the prediction",
  "rationale": "2-3 sentence explanation",
  "risk_flags": ["flag1", "flag2"],
  "veto_applied": true | false,
  "veto_reason": "if vetoed, explain why",
  "position_size_pct": 0-100
}}
```

## Veto Triggers

Override the preliminary decision and veto if you detect:
1. Contradictory evidence not captured in the risk flags
2. False pattern match (semantic narrative doesn't match raw data)
3. Market manipulation or spoofing signals
4. External context that invalidates the thesis (e.g., macro news)

## Decision Rules

1. If veto triggers apply → direction: NEUTRAL, confidence: 0
2. If confidence < 65 → direction: NEUTRAL (insufficient edge)
3. Provide specific rationale referencing the data
4. State uncertainty clearly when present

What is your decision?"#,
            narrative = narrative.narrative_md,
            score = preliminary_decision.confidence as f64 / 100.0,
            direction = format!("{:?}", preliminary_decision.direction),
            confidence = preliminary_decision.confidence,
            driver = preliminary_decision.lead_driver,
            regime = format!("{:?}", preliminary_decision.regime),
            veto_status = if preliminary_decision.veto_applied {
                format!("VETO APPLIED: {}", preliminary_decision.veto_reason.as_ref().unwrap())
            } else {
                "No veto conditions triggered".to_string()
            },
        )
    }
}

#[derive(Debug)]
struct VetoCheck {
    should_veto: bool,
    reason: String,
}

impl Default for CIODecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cio_creation() {
        let cio = CIODecisionEngine::new();
        assert!(cio.min_confidence > 0);
    }

    #[test]
    fn test_directional_scoring() {
        let cio = CIODecisionEngine::new();
        let readings = AnalystReadings {
            obi: Some(0.7),
            hma_slope: Some(25.0),
            ..Default::default()
        };
        
        let (score, driver) = cio.calculate_directional_score(&readings, &[]);
        assert!(score > 0.0);
        assert!(!driver.is_empty());
    }
}
