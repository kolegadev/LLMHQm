//! Layer B: Narrator - Semantic Synthesis Engine
//! 
//! The Narrator's job is to:
//! 1. Spot patterns in the data (like chess strategies/move patterns)
//! 2. Label them in semantic terms
//! 3. Generate token-efficient Markdown narratives

use crate::types::*;
use chrono::Utc;
use std::collections::HashMap;

/// Pattern definition - like a chess opening or tactical motif
#[derive(Debug, Clone)]
pub struct MarketPattern {
    pub name: String,
    pub description: String,
    pub conditions: Vec<PatternCondition>,
    pub confidence_weight: f64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum PatternCondition {
    ObiAbove(f64),
    ObiBelow(f64),
    ObiVelocityAbove(f64),
    HmaSlopeAbove(f64),
    HmaSlopeBelow(f64),
    VpinAbove(f64),
    VolatilityRegime(VolatilityRegime),
    BasisAbove(f64),
    BasisBelow(f64),
    LiquidationPressureAbove(f64),
    PinningRisk(PinningClassification),
    SpreadAbove(f64),
}

/// The Narrator engine - spots patterns and generates semantic narratives
pub struct Narrator {
    patterns: Vec<MarketPattern>,
    pattern_history: Vec<(String, f64)>, // (pattern_name, confidence)
}

impl Narrator {
    pub fn new() -> Self {
        let patterns = Self::initialize_patterns();
        Self {
            patterns,
            pattern_history: Vec::new(),
        }
    }

    /// Initialize the pattern library - like chess openings database
    fn initialize_patterns() -> Vec<MarketPattern> {
        vec![
            // =================================================================
            // MOMENTUM PATTERNS
            // =================================================================
            MarketPattern {
                name: "HMA_Surf_Steepening".to_string(),
                description: "Price surfing HMA with accelerating momentum".to_string(),
                conditions: vec![
                    PatternCondition::HmaSlopeAbove(15.0),
                ],
                confidence_weight: 0.15,
                tags: vec!["momentum".to_string(), "continuation".to_string()],
            },
            MarketPattern {
                name: "HMA_Flat_Consolidation".to_string(),
                description: "Price grinding sideways, momentum neutral".to_string(),
                conditions: vec![
                    PatternCondition::HmaSlopeAbove(-5.0),
                    PatternCondition::HmaSlopeBelow(5.0),
                ],
                confidence_weight: 0.05,
                tags: vec!["consolidation".to_string(), "mean_reversion_setup".to_string()],
            },
            MarketPattern {
                name: "HMA_Break_Down".to_string(),
                description: "Sharp downward momentum, HMA slope steepening bearish".to_string(),
                conditions: vec![
                    PatternCondition::HmaSlopeBelow(-20.0),
                ],
                confidence_weight: 0.15,
                tags: vec!["momentum".to_string(), "breakdown".to_string()],
            },

            // =================================================================
            // ORDER FLOW PATTERNS
            // =================================================================
            MarketPattern {
                name: "Heavy_Buy_Absorption".to_string(),
                description: "Aggressive buying absorbing sell pressure".to_string(),
                conditions: vec![
                    PatternCondition::ObiAbove(0.6),
                    PatternCondition::VpinAbove(0.6),
                ],
                confidence_weight: 0.20,
                tags: vec!["order_flow".to_string(), "informed_buying".to_string()],
            },
            MarketPattern {
                name: "Heavy_Sell_Absorption".to_string(),
                description: "Aggressive selling hitting bid, distribution".to_string(),
                conditions: vec![
                    PatternCondition::ObiBelow(-0.6),
                    PatternCondition::VpinAbove(0.6),
                ],
                confidence_weight: 0.20,
                tags: vec!["order_flow".to_string(), "informed_selling".to_string()],
            },
            MarketPattern {
                name: "OBI_Acceleration_Bull".to_string(),
                description: "Buy-side pressure building rapidly".to_string(),
                conditions: vec![
                    PatternCondition::ObiVelocityAbove(0.1),
                    PatternCondition::ObiAbove(0.5),
                ],
                confidence_weight: 0.12,
                tags: vec!["momentum".to_string(), "flow_acceleration".to_string()],
            },
            MarketPattern {
                name: "OBI_Acceleration_Bear".to_string(),
                description: "Sell-side pressure building rapidly".to_string(),
                conditions: vec![
                    PatternCondition::ObiVelocityAbove(0.1), // Magnitude, check sign in matching
                    PatternCondition::ObiBelow(-0.5),
                ],
                confidence_weight: 0.12,
                tags: vec!["momentum".to_string(), "flow_acceleration".to_string()],
            },

            // =================================================================
            // BASIS/LEAD-LAG PATTERNS
            // =================================================================
            MarketPattern {
                name: "Perp_Premium_Levered_Long".to_string(),
                description: "Perp trading at premium, leveraged longs driving price".to_string(),
                conditions: vec![
                    PatternCondition::BasisAbove(5.0),
                ],
                confidence_weight: 0.10,
                tags: vec!["lead_lag".to_string(), "leverage".to_string()],
            },
            MarketPattern {
                name: "Perp_Discount_Forced_Sell".to_string(),
                description: "Perp at discount, possible forced selling or hedging".to_string(),
                conditions: vec![
                    PatternCondition::BasisBelow(-5.0),
                ],
                confidence_weight: 0.10,
                tags: vec!["lead_lag".to_string(), "forced_selling".to_string()],
            },

            // =================================================================
            // LIQUIDATION PATTERNS
            // =================================================================
            MarketPattern {
                name: "Long_Liquidation_Cascade".to_string(),
                description: "Significant long liquidations, potential reversal setup".to_string(),
                conditions: vec![
                    PatternCondition::LiquidationPressureAbove(100000.0), // $100k+
                ],
                confidence_weight: 0.08,
                tags: vec!["liquidation".to_string(), "cascade".to_string(), "contrarian".to_string()],
            },
            MarketPattern {
                name: "Short_Liquidation_Squeeze".to_string(),
                description: "Shorts being squeezed, continuation likely".to_string(),
                conditions: vec![
                    PatternCondition::LiquidationPressureAbove(-100000.0), // Negative = shorts
                ],
                confidence_weight: 0.08,
                tags: vec!["liquidation".to_string(), "squeeze".to_string()],
            },

            // =================================================================
            // VOLATILITY PATTERNS
            // =================================================================
            MarketPattern {
                name: "Volatility_Expansion".to_string(),
                description: "Market entering high volatility regime".to_string(),
                conditions: vec![
                    PatternCondition::VolatilityRegime(VolatilityRegime::Expanding),
                ],
                confidence_weight: 0.06,
                tags: vec!["volatility".to_string(), "breakout".to_string()],
            },
            MarketPattern {
                name: "Volatility_Compression".to_string(),
                description: "Market coiling, volatility compression before expansion".to_string(),
                conditions: vec![
                    PatternCondition::VolatilityRegime(VolatilityRegime::Compressing),
                ],
                confidence_weight: 0.08,
                tags: vec!["volatility".to_string(), "compression".to_string(), "setup".to_string()],
            },

            // =================================================================
            // MANIPULATION/PINNING PATTERNS
            // =================================================================
            MarketPattern {
                name: "Pinning_Risk_High".to_string(),
                description: "Possible end-of-block manipulation, wide spreads, thin liquidity".to_string(),
                conditions: vec![
                    PatternCondition::PinningRisk(PinningClassification::HighBreak),
                ],
                confidence_weight: -0.50, // Negative = veto
                tags: vec!["manipulation".to_string(), "pinning".to_string(), "veto".to_string()],
            },
            MarketPattern {
                name: "Late_OBI_Spike".to_string(),
                description: "Late OBI movement, possible fake wall".to_string(),
                conditions: vec![
                    PatternCondition::ObiVelocityAbove(0.2),
                    PatternCondition::SpreadAbove(10.0),
                ],
                confidence_weight: -0.20,
                tags: vec!["manipulation".to_string(), "fake_wall".to_string()],
            },
        ]
    }

    /// Match current readings against pattern library
    pub fn identify_patterns(&self, readings: &AnalystReadings) -> Vec<(&MarketPattern, f64)> {
        let mut matches = Vec::new();

        for pattern in &self.patterns {
            let match_score = self.calculate_pattern_match(pattern, readings);
            if match_score > 0.5 {
                matches.push((pattern, match_score));
            }
        }

        // Sort by match score descending
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        matches
    }

    fn calculate_pattern_match(&self,
        pattern: &MarketPattern,
        readings: &AnalystReadings
    ) -> f64 {
        let mut total_conditions = pattern.conditions.len() as f64;
        let mut matched_conditions = 0.0;

        for condition in &pattern.conditions {
            let matched = match condition {
                PatternCondition::ObiAbove(threshold) => {
                    readings.obi.map(|v| v > *threshold).unwrap_or(false)
                }
                PatternCondition::ObiBelow(threshold) => {
                    readings.obi.map(|v| v < *threshold).unwrap_or(false)
                }
                PatternCondition::ObiVelocityAbove(threshold) => {
                    readings.obi_velocity.map(|v| v.abs() > *threshold).unwrap_or(false)
                }
                PatternCondition::HmaSlopeAbove(threshold) => {
                    readings.hma_slope.map(|v| v > *threshold).unwrap_or(false)
                }
                PatternCondition::HmaSlopeBelow(threshold) => {
                    readings.hma_slope.map(|v| v < *threshold).unwrap_or(false)
                }
                PatternCondition::VpinAbove(threshold) => {
                    readings.vpin.map(|v| v > *threshold).unwrap_or(false)
                }
                PatternCondition::VolatilityRegime(regime) => {
                    readings.volatility_regime.map(|v| &v == regime).unwrap_or(false)
                }
                PatternCondition::BasisAbove(threshold) => {
                    readings.basis_bps.map(|v| v > *threshold).unwrap_or(false)
                }
                PatternCondition::BasisBelow(threshold) => {
                    readings.basis_bps.map(|v| v < *threshold).unwrap_or(false)
                }
                PatternCondition::LiquidationPressureAbove(threshold) => {
                    readings.net_liquidation_pressure
                        .map(|v| v > threshold.abs() || v < -threshold.abs())
                        .unwrap_or(false)
                }
                PatternCondition::PinningRisk(classification) => {
                    readings.pinning_classification.map(|v| &v == classification).unwrap_or(false)
                }
                PatternCondition::SpreadAbove(threshold) => {
                    readings.spread_bps.map(|v| v > *threshold).unwrap_or(false)
                }
            };

            if matched {
                matched_conditions += 1.0;
            }
        }

        if total_conditions > 0.0 {
            matched_conditions / total_conditions
        } else {
            0.0
        }
    }

    /// Generate semantic narrative in Markdown (token-efficient for LLM)
    pub fn generate_narrative(
        &self,
        readings: &AnalystReadings,
        patterns: Vec<(&MarketPattern, f64)>,
    ) -> SemanticNarrative {
        let mut narrative = String::new();
        let mut pattern_tags = Vec::new();

        // Header
        narrative.push_str(&format!(
            "## Market Briefing: Block #{}\n\n",
            readings.block_number
        ));

        // Price context
        if let Some(price) = readings.spot_price {
            narrative.push_str(&format!("**Price**: \u0026#36;{}\n\n", price));
        }

        // Regime classification
        let regime = readings.dominant_regime();
        narrative.push_str(&format!("**Regime**: {}\n\n", Self::format_regime(regime)));

        // Primary patterns (top 3)
        if !patterns.is_empty() {
            narrative.push_str("### Primary Patterns\n\n");
            for (pattern, score) in patterns.iter().take(3) {
                pattern_tags.push(pattern.name.clone());
                narrative.push_str(&format!(
                    "- **{}** (match: {:.0}%): {}\n",
                    pattern.name,
                    score * 100.0,
                    pattern.description
                ));
            }
            narrative.push('\n');
        }

        // Technical details (concise)
        narrative.push_str("### Technical Snapshot\n\n");
        
        // Momentum
        if let (Some(slope), Some(hma)) = (readings.hma_slope, readings.hma) {
            narrative.push_str(&format!(
                "- **Momentum**: HMA at \u0026#36;{:.2}, slope {:.1}° ({})\n",
                hma,
                slope,
                match readings.hma_trend {
                    Some(Trend::Up) => "accelerating",
                    Some(Trend::Down) => "decelerating",
                    _ => "flat",
                }
            ));
        }

        // Order flow
        if let (Some(obi), Some(velocity)) = (readings.obi_normalized, readings.obi_velocity) {
            let obi_pct = obi * 100.0;
            let flow_direction = if velocity > 0.0 { "building" } else { "fading" };
            narrative.push_str(&format!(
                "- **Order Flow**: {:.0}% buy-side dominance, {} pressure\n",
                obi_pct,
                flow_direction
            ));
        }

        // Basis
        if let Some(basis) = readings.basis_bps {
            let bias = match readings.perp_bias {
                Some(PerpBias::StrongPremium | PerpBias::Premium) => "leveraged longs active",
                Some(PerpBias::StrongDiscount | PerpBias::Discount) => "hedging/short pressure",
                _ => "neutral",
            };
            narrative.push_str(&format!(
                "- **Basis**: {:+.1} bps ({})\n",
                basis,
                bias
            ));
        }

        // Liquidations
        if let Some(net_pressure) = readings.net_liquidation_pressure {
            if net_pressure.abs() > 50000.0 {
                let side = if net_pressure > 0.0 { "longs" } else { "shorts" };
                narrative.push_str(&format!(
                    "- **Liquidations**: ${:,.0f} {} liquidated (1m)\n",
                    net_pressure.abs(),
                    side
                ));
            }
        }

        // VPIN/Toxicity
        if let (Some(vpin), Some(tox)) = (readings.vpin, readings.toxicity) {
            if vpin > 0.6 {
                narrative.push_str(&format!(
                    "- **Toxicity**: VPIN {:.2} ({} informed flow)\n",
                    vpin,
                    match tox {
                        Toxicity::Elevated => "elevated",
                        Toxicity::Normal => "normal",
                    }
                ));
            }
        }

        // Risk warnings
        if let Some(PinningClassification::HighBreak | PinningClassification::HighHold) = 
            readings.pinning_classification {
            narrative.push_str("\n### ⚠️ Risk Warnings\n\n");
            if let Some(score) = readings.pinning_risk_score {
                narrative.push_str(&format!(
                    "- **Pinning Risk**: Score {}/100 - Possible manipulation\n",
                    score
                ));
            }
        }

        // Summary sentence
        narrative.push_str("\n### Summary\n\n");
        narrative.push_str(&self.generate_summary_sentence(readings, &patterns));

        // Calculate narrative confidence
        let confidence = if patterns.is_empty() {
            0.5
        } else {
            patterns.iter().map(|(_, s)| s).sum::<f64>() / patterns.len() as f64
        };

        SemanticNarrative {
            timestamp: Utc::now(),
            block_number: readings.block_number,
            narrative_md: narrative,
            pattern_tags,
            confidence,
        }
    }

    fn format_regime(regime: MarketRegime) -> &'static str {
        match regime {
            MarketRegime::Trending => "Trending",
            MarketRegime::Ranging => "Mean-Reverting / Ranging",
            MarketRegime::VolatileExpansion => "Volatile Expansion",
            MarketRegime::QuietCompression => "Quiet Compression (setup)",
            MarketRegime::Manipulative => "⚠️ Manipulative / Pinning",
        }
    }

    fn generate_summary_sentence(
        &self,
        readings: &AnalystReadings,
        patterns: &[(&MarketPattern, f64)],
    ) -> String {
        let mut parts = Vec::new();

        // Directional bias from momentum
        if let Some(Trend::Up) = readings.hma_trend {
            parts.push("Upward momentum".to_string());
        } else if let Some(Trend::Down) = readings.hma_trend {
            parts.push("Downward pressure".to_string());
        }

        // Order flow
        if let Some(Pressure::StrongBuy | Pressure::Buy) = readings.pressure {
            parts.push("buy-side absorption".to_string());
        } else if let Some(Pressure::StrongSell | Pressure::Sell) = readings.pressure {
            parts.push("sell-side pressure".to_string());
        }

        // Leverage/basis
        if let Some(PerpBias::StrongPremium | PerpBias::Premium) = readings.perp_bias {
            parts.push("leveraged long interest".to_string());
        }

        // Volatility
        if let Some(VolatilityRegime::Expanding) = readings.volatility_regime {
            parts.push("volatility expansion".to_string());
        } else if let Some(VolatilityRegime::Compressing) = readings.volatility_regime {
            parts.push("compression setup".to_string());
        }

        // Top pattern
        if let Some((pattern, _)) = patterns.first() {
            parts.push(format!("suggests {}", pattern.name.to_lowercase().replace("_", " ")));
        }

        if parts.is_empty() {
            "Mixed signals, no clear directional bias.".to_string()
        } else {
            format!("{}.", parts.join(", "))
        }
    }
}

impl Default for Narrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_narrator_creation() {
        let narrator = Narrator::new();
        assert!(!narrator.patterns.is_empty());
    }

    #[test]
    fn test_pattern_matching() {
        let narrator = Narrator::new();
        let readings = AnalystReadings {
            obi: Some(0.7),
            obi_normalized: Some(0.85),
            vpin: Some(0.65),
            ..Default::default()
        };

        let patterns = narrator.identify_patterns(&readings);
        assert!(!patterns.is_empty());
    }
}
