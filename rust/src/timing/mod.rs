//! Block timing synchronization for 5-minute intervals

use crate::types::*;
use chrono::{DateTime, Duration, Utc};
use std::time::{SystemTime, UNIX_EPOCH};

/// Manages 5-minute block timing for Polymarket-style intervals
pub struct BlockTimer {
    interval_seconds: i64,
}

impl BlockTimer {
    pub fn new(interval_minutes: i64) -> Self {
        Self {
            interval_seconds: interval_minutes * 60,
        }
    }

    /// Get Unix timestamp of next block start
    pub fn get_next_block_time(&self) -> f64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        let interval = self.interval_seconds as f64;
        ((now / interval).floor() + 1.0) * interval
    }

    /// Get Unix timestamp of current block start
    pub fn get_current_block_time(&self) -> f64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        let interval = self.interval_seconds as f64;
        (now / interval).floor() * interval
    }

    /// Get current block number
    pub fn get_block_number(&self) -> u64 {
        (self.get_current_block_time() / self.interval_seconds as f64) as u64
    }

    /// Get current timing state
    pub fn get_timing(&self) -> BlockTiming {
        let next_block = self.get_next_block_time();
        let seconds_to_next = next_block - SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        
        let phase = self.determine_phase(seconds_to_next);
        
        BlockTiming {
            current_block_number: self.get_block_number(),
            next_block_timestamp: DateTime::from_timestamp(next_block as i64, 0)
                .unwrap_or_else(|| Utc::now()),
            seconds_to_next_block: seconds_to_next,
            phase,
        }
    }

    fn determine_phase(&self,
        seconds_to_next: f64
    ) -> BlockPhase {
        if seconds_to_next > 30.0 {
            BlockPhase::Idle
        } else if seconds_to_next > 15.0 {
            BlockPhase::Calculation
        } else if seconds_to_next > 10.0 {
            BlockPhase::Aggregation
        } else if seconds_to_next > 5.0 {
            BlockPhase::Synthesis
        } else if seconds_to_next > 2.0 {
            BlockPhase::Decision
        } else if seconds_to_next > 0.0 {
            BlockPhase::Execution
        } else {
            BlockPhase::PostExecution
        }
    }

    /// Check if we should be calculating features (t-30s onwards)
    pub fn should_calculate(&self) -> bool {
        self.get_timing().seconds_to_next_block <= 30.0
    }

    /// Check if we're in CIO decision window (t-5s to t-2s)
    pub fn should_decide(&self) -> bool {
        let timing = self.get_timing();
        let secs = timing.seconds_to_next_block;
        secs > 2.0 && secs <= 5.0
    }

    /// Check if we should execute (t-2s to t=0)
    pub fn should_execute(&self) -> bool {
        let timing = self.get_timing();
        let secs = timing.seconds_to_next_block;
        secs > 0.0 && secs <= 2.0
    }

    /// Format seconds as MM:SS
    pub fn format_countdown(&self,
        seconds: f64
    ) -> String {
        let mins = (seconds / 60.0) as i64;
        let secs = (seconds % 60.0) as i64;
        format!("{:02d}:{:02d}", mins, secs)
    }

    /// Print current timing status
    pub fn print_status(&self) {
        let timing = self.get_timing();
        let countdown = self.format_countdown(timing.seconds_to_next_block);
        
        println!("\n⏱️  BLOCK TIMING");
        println!("   Next block in: {}", countdown);
        println!("   Phase: {:?}", timing.phase);
        println!("   Action: {}", timing.phase.description());
        println!("   Block #{}", timing.current_block_number);
    }
}

impl Default for BlockTimer {
    fn default() -> Self {
        Self::new(5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_timer_creation() {
        let timer = BlockTimer::new(5);
        let timing = timer.get_timing();
        assert!(timing.seconds_to_next_block > 0.0);
        assert!(timing.seconds_to_next_block <= 300.0);
    }

    #[test]
    fn test_phase_determination() {
        let timer = BlockTimer::new(5);
        
        assert!(matches!(timer.determine_phase(35.0), BlockPhase::Idle));
        assert!(matches!(timer.determine_phase(25.0), BlockPhase::Calculation));
        assert!(matches!(timer.determine_phase(12.0), BlockPhase::Aggregation));
        assert!(matches!(timer.determine_phase(7.0), BlockPhase::Synthesis));
        assert!(matches!(timer.determine_phase(3.0), BlockPhase::Decision));
        assert!(matches!(timer.determine_phase(1.0), BlockPhase::Execution));
        assert!(matches!(timer.determine_phase(-1.0), BlockPhase::PostExecution));
    }
}
