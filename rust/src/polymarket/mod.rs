//! Polymarket API Integration
//!
//! Uses Gamma API for market data and odds
//! Docs: https://docs.polymarket.com/

use crate::executor::PolymarketOdds;
use chrono::Utc;
use serde::Deserialize;
use tracing::{info, debug, error};

/// Polymarket Gamma API client
pub struct PolymarketClient {
    client: reqwest::Client,
    base_url: String,
}

impl PolymarketClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://gamma-api.polymarket.com".to_string(),
        }
    }
    
    /// Fetch market data by ID
    pub async fn get_market(
        &self,
        market_id: &str
    ) -> anyhow::Result<MarketData> {
        let url = format!("{}/markets/{}", self.base_url, market_id);
        
        debug!("Fetching Polymarket market: {}", market_id);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Polymarket API error: {}", response.status());
        }
        
        let market: MarketData = response.json().await?;
        
        info!("Fetched market: {} | Active: {}", 
            market.question, 
            market.active
        );
        
        Ok(market)
    }
    
    /// Get current odds for a market
    pub async fn get_current_odds(
        &self,
        market_id: &str
    ) -> anyhow::Result<PolymarketOdds> {
        let market = self.get_market(market_id).await?;
        
        // Extract YES token price from outcome prices
        let yes_price = market.outcomes
            .iter()
            .find(|o| o.name == "Yes")
            .map(|o| o.price)
            .unwrap_or(0.5);
        
        let no_price = market.outcomes
            .iter()
            .find(|o| o.name == "No")
            .map(|o| o.price)
            .unwrap_or(0.5);
        
        let spread = (yes_price - no_price).abs();
        
        Ok(PolymarketOdds {
            timestamp: Utc::now(),
            yes_price,
            no_price,
            spread,
            volume_24h: market.volume_24hr,
        })
    }
    
    /// Search for active BTC prediction markets
    pub async fn search_btc_markets(
        &self
    ) -> anyhow::Result<Vec<MarketSummary>> {
        let url = format!("{}/markets?active=true&archived=false", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Polymarket API error: {}", response.status());
        }
        
        let markets: Vec<MarketSummary> = response.json().await?;
        
        // Filter for BTC-related markets
        let btc_markets: Vec<MarketSummary> = markets
            .into_iter()
            .filter(|m| {
                let q = m.question.to_lowercase();
                q.contains("bitcoin") || q.contains("btc")
            })
            .collect();
        
        info!("Found {} active BTC markets", btc_markets.len());
        
        Ok(btc_markets)
    }
}

impl Default for PolymarketClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Market data from Gamma API
#[derive(Debug, Deserialize)]
pub struct MarketData {
    pub id: String,
    pub question: String,
    pub description: String,
    pub active: bool,
    pub closed: bool,
    pub outcomes: Vec<Outcome>,
    pub volume_24hr: f64,
    pub liquidity: f64,
    pub end_date: String,
    pub resolution_source: Option<String>,
}

/// Market outcome (Yes/No)
#[derive(Debug, Deserialize)]
pub struct Outcome {
    pub name: String,
    pub price: f64,
    pub winner: Option<bool>,
}

/// Summary for market search results
#[derive(Debug, Deserialize)]
pub struct MarketSummary {
    pub id: String,
    pub question: String,
    pub slug: String,
    pub volume_24hr: f64,
    pub liquidity: f64,
    pub end_date: String,
}

/// Find the BTC 5-minute or 15-minute market
pub fn find_btc_interval_market(
    markets: &[MarketSummary],
    interval_minutes: u64,
) -> Option<&MarketSummary> {
    let interval_str = format!("{}-minute", interval_minutes);
    let alt_str = format!("{} min", interval_minutes);
    
    markets.iter().find(|m| {
        let q = m.question.to_lowercase();
        (q.contains(&interval_str) || q.contains(&alt_str)) &&
        q.contains("bitcoin")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_market_parsing() {
        let json = r#"{
            "id": "test-market-123",
            "question": "Will Bitcoin be above $70k at 12:00?",
            "description": "5-minute prediction market",
            "active": true,
            "closed": false,
            "outcomes": [
                {"name": "Yes", "price": 0.52, "winner": null},
                {"name": "No", "price": 0.48, "winner": null}
            ],
            "volume_24hr": 1500000.0,
            "liquidity": 500000.0,
            "end_date": "2024-03-13T12:00:00Z",
            "resolution_source": null
        }"#;
        
        let market: MarketData = serde_json::from_str(json).unwrap();
        assert_eq!(market.question, "Will Bitcoin be above $70k at 12:00?");
        assert_eq!(market.outcomes.len(), 2);
        
        let yes = market.outcomes.iter().find(|o| o.name == "Yes").unwrap();
        assert_eq!(yes.price, 0.52);
    }
    
    #[test]
    fn test_find_btc_market() {
        let markets = vec![
            MarketSummary {
                id: "btc-5m-123".to_string(),
                question: "Bitcoin 5-minute UP?".to_string(),
                slug: "btc-5m".to_string(),
                volume_24hr: 1000000.0,
                liquidity: 500000.0,
                end_date: "2024-03-13T12:00:00Z".to_string(),
            },
            MarketSummary {
                id: "btc-15m-456".to_string(),
                question: "Bitcoin 15-minute UP?".to_string(),
                slug: "btc-15m".to_string(),
                volume_24hr: 2000000.0,
                liquidity: 800000.0,
                end_date: "2024-03-13T12:15:00Z".to_string(),
            },
        ];
        
        let market_5m = find_btc_interval_market(&markets, 5);
        assert!(market_5m.is_some());
        assert_eq!(market_5m.unwrap().id, "btc-5m-123");
        
        let market_15m = find_btc_interval_market(&markets, 15);
        assert!(market_15m.is_some());
        assert_eq!(market_15m.unwrap().id, "btc-15m-456");
    }
}
