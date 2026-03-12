//! Async database layer with PostgreSQL/TimescaleDB

use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use crate::types::*;
use chrono::Utc;
use anyhow::Result;

/// Database connection manager
pub struct Database {
    pool: Pool<Postgres>,
}

impl Database {
    /// Create new database connection pool
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(std::time::Duration::from_secs(300))
            .connect(database_url)
            .await?;
        
        Ok(Self { pool })
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await?;
        Ok(())
    }

    /// Store analyst readings
    pub async fn store_readings(
        &self,
        readings: &AnalystReadings,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO analyst_readings (
                time, block_number, analyst, readings
            ) VALUES ($1, $2, 'all', $3)
            "#,
        )
        .bind(readings.timestamp)
        .bind(readings.block_number as i64)
        .bind(serde_json::to_value(readings)?)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Store semantic narrative
    pub async fn store_narrative(
        &self,
        narrative: &SemanticNarrative,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO narratives (
                time, block_number, narrative_md, pattern_tags
            ) VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(narrative.timestamp)
        .bind(narrative.block_number as i64)
        .bind(&narrative.narrative_md)
        .bind(&narrative.pattern_tags)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Store CIO decision
    pub async fn store_decision(
        &self,
        decision: &CIODecision,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO cio_decisions (
                time, block_number, direction, confidence, regime,
                lead_driver, rationale, risk_flags, veto_applied,
                veto_reason
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(decision.timestamp)
        .bind(decision.block_number as i64)
        .bind(format!("{:?}", decision.direction))
        .bind(decision.confidence as i32)
        .bind(format!("{:?}", decision.regime))
        .bind(&decision.lead_driver)
        .bind(&decision.rationale)
        .bind(&decision.risk_flags)
        .bind(decision.veto_applied)
        .bind(decision.veto_reason.as_ref())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Store paper trade
    pub async fn store_paper_trade(
        &self,
        trade: &PaperTrade,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO paper_trades (
                id, block_number, decision_time, entry_time,
                direction, confidence, entry_price
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(trade.id)
        .bind(trade.block_number as i64)
        .bind(trade.decision.timestamp)
        .bind(trade.entry_time)
        .bind(format!("{:?}", trade.decision.direction))
        .bind(trade.decision.confidence as i32)
        .bind(trade.entry_price)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Update trade outcome after resolution
    pub async fn update_trade_outcome(
        &self,
        trade_id: uuid::Uuid,
        exit_price: rust_decimal::Decimal,
        outcome: TradeOutcome,
        pnl_pct: f64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE paper_trades
            SET exit_price = $1, outcome = $2, pnl_pct = $3, exit_time = $4
            WHERE id = $5
            "#,
        )
        .bind(exit_price)
        .bind(format!("{:?}", outcome))
        .bind(pnl_pct)
        .bind(Utc::now())
        .bind(trade_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Get recent decisions for analysis
    pub async fn get_recent_decisions(
        &self,
        limit: i64,
    ) -> Result<Vec<CIODecision>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM cio_decisions
            ORDER BY time DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        // Convert rows to CIODecision structs
        // ... implementation details
        
        Ok(Vec::new()) // Placeholder
    }

    /// Get paper trading statistics
    pub async fn get_paper_stats(&self,
    ) -> Result<PaperTradingStats> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total,
                COUNT(CASE WHEN outcome = 'Win' THEN 1 END) as wins,
                COUNT(CASE WHEN outcome = 'Loss' THEN 1 END) as losses,
                AVG(CASE WHEN outcome = 'Win' THEN pnl_pct END) as avg_win,
                AVG(CASE WHEN outcome = 'Loss' THEN pnl_pct END) as avg_loss
            FROM paper_trades
            WHERE outcome IS NOT NULL
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        
        let total: i64 = row.try_get("total")?;
        let wins: i64 = row.try_get("wins")?;
        let losses: i64 = row.try_get("losses")?;
        
        Ok(PaperTradingStats {
            total_trades: total as u32,
            wins: wins as u32,
            losses: losses as u32,
            win_rate: if total > 0 {
                (wins as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            avg_win_pnl: row.try_get("avg_win").unwrap_or(0.0),
            avg_loss_pnl: row.try_get("avg_loss").unwrap_or(0.0),
        })
    }
}

/// Paper trading statistics
#[derive(Debug, Clone)]
pub struct PaperTradingStats {
    pub total_trades: u32,
    pub wins: u32,
    pub losses: u32,
    pub win_rate: f64,
    pub avg_win_pnl: f64,
    pub avg_loss_pnl: f64,
}

/// Database models module
pub mod models {
    use super::*;
    
    // Re-export types used in database operations
    pub use crate::types::*;
}
