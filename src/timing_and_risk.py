"""
LLMHQ Block Timer and Pinning Risk Calculator
Implements missing components from data audit
"""

import time
import math
from datetime import datetime, timezone
from typing import Dict, Optional, Tuple
from collections import deque
from dataclasses import dataclass

@dataclass
class BlockTiming:
    """Current block timing state"""
    seconds_to_next_block: float
    phase: str  # idle, t-30_to_t-15, t-15_to_t-10, t-10_to_t-5, t-5_to_t-2, execution, post_execution
    next_block_timestamp: float
    current_block_number: int
    
class BlockTimer:
    """
    Manages 5-minute block timing for Polymarket-style intervals
    
    Timeline:
    - t-30s to t-15s: Parallel feature calculation
    - t-15s to t-10s: Data aggregation
    - t-10s to t-5s: Semantic synthesis
    - t-5s to t-2s: CIO decision window
    - t-2s to t=0: Execution preparation
    - t=0: Trade execution
    """
    
    def __init__(self, interval_minutes: int = 5):
        self.interval_seconds = interval_minutes * 60
        self.decision_phases = {
            "t-30_to_t-15": (30, 15, "PARALLEL_CALCULATION"),
            "t-15_to_t-10": (15, 10, "AGGREGATION"),
            "t-10_to_t-5": (10, 5, "SEMANTIC_SYNTHESIS"),
            "t-5_to_t-2": (5, 2, "CIO_DECISION"),
            "t-2_to_t-0": (2, 0, "EXECUTION_PREP"),
        }
        
    def get_next_block_time(self) -> float:
        """Get Unix timestamp of next 5m block start"""
        now = time.time()
        interval = self.interval_seconds
        return ((now // interval) + 1) * interval
    
    def get_current_block_time(self) -> float:
        """Get Unix timestamp of current block start"""
        now = time.time()
        interval = self.interval_seconds
        return (now // interval) * interval
    
    def get_block_number(self) -> int:
        """Get current block number (for logging)"""
        return int(self.get_current_block_time() / self.interval_seconds)
    
    def get_timing(self) -> BlockTiming:
        """Get current block timing state"""
        next_block = self.get_next_block_time()
        seconds_to_next = next_block - time.time()
        current_block = self.get_current_block_time()
        
        # Determine phase
        if seconds_to_next > 30:
            phase = "idle"
        elif seconds_to_next > 15:
            phase = "t-30_to_t-15"
        elif seconds_to_next > 10:
            phase = "t-15_to_t-10"
        elif seconds_to_next > 5:
            phase = "t-10_to_t-5"
        elif seconds_to_next > 2:
            phase = "t-5_to_t-2"
        elif seconds_to_next > 0:
            phase = "t-2_to_t-0"
        else:
            phase = "post_execution"
            
        return BlockTiming(
            seconds_to_next_block=seconds_to_next,
            phase=phase,
            next_block_timestamp=next_block,
            current_block_number=self.get_block_number()
        )
    
    def should_calculate(self) -> bool:
        """Check if we should be calculating features (t-30s onwards)"""
        timing = self.get_timing()
        return timing.seconds_to_next_block <= 30
    
    def should_decide(self) -> bool:
        """Check if we're in CIO decision window (t-5s to t-2s)"""
        timing = self.get_timing()
        return 2 < timing.seconds_to_next_block <= 5
    
    def should_execute(self) -> bool:
        """Check if we should execute (t-2s to t=0)"""
        timing = self.get_timing()
        return 0 < timing.seconds_to_next_block <= 2
    
    def get_phase_description(self, phase: str) -> str:
        """Get human-readable phase description"""
        descriptions = {
            "idle": "Waiting for next block...",
            "t-30_to_t-15": "PHASE 1: Parallel feature calculation (Rust layer)",
            "t-15_to_t-10": "PHASE 2: Data aggregation",
            "t-10_to_t-5": "PHASE 3: Semantic synthesis (narrative generation)",
            "t-5_to_t-2": "PHASE 4: CIO decision window (LLM evaluation)",
            "t-2_to_t-0": "PHASE 5: Execution preparation",
            "post_execution": "Block started - monitoring outcome"
        }
        return descriptions.get(phase, "Unknown phase")
    
    def format_countdown(self, seconds: float) -> str:
        """Format seconds as MM:SS countdown"""
        mins = int(seconds // 60)
        secs = int(seconds % 60)
        return f"{mins:02d}:{secs:02d}"
    
    def print_status(self):
        """Print current timing status"""
        timing = self.get_timing()
        countdown = self.format_countdown(timing.seconds_to_next_block)
        desc = self.get_phase_description(timing.phase)
        
        print(f"\n⏱️  BLOCK TIMING")
        print(f"   Next block in: {countdown}")
        print(f"   Phase: {timing.phase}")
        print(f"   Action: {desc}")
        print(f"   Block #{timing.current_block_number}")


class PinningRiskCalculator:
    """
    Detects block-end pinning and manipulation
    
    Pinning types:
    - HIGH_BREAK: Pin breaking, high volatility - VETO trade
    - HIGH_HOLD: Pin holding steady - may be genuine or trapped
    - LOW: No manipulation detected
    """
    
    def __init__(self):
        self.obi_history: deque = deque(maxlen=50)
        self.spread_history: deque = deque(maxlen=50)
        self.volatility_history: deque = deque(maxlen=50)
        
    def update(self, obi: float, spread_bps: float, volatility: float):
        """Update with latest market data"""
        self.obi_history.append(obi)
        self.spread_history.append(spread_bps)
        self.volatility_history.append(volatility)
    
    def calculate_pinning_risk(
        self,
        obi: float,
        obi_velocity: float,
        spread_bps: float,
        volatility: float,
        seconds_to_block_end: float
    ) -> Dict:
        """
        Calculate pinning risk score and classification
        
        Risk factors:
        1. OBI velocity spike (>0.2 in final seconds)
        2. High OBI (>0.7) with high spread
        3. Volatility expansion during final 10s
        4. Thin order book (wide spreads)
        """
        risk_score = 0
        risk_factors = []
        
        # Factor 1: OBI velocity spike in final seconds
        if seconds_to_block_end < 10 and abs(obi_velocity) > 0.15:
            risk_score += 30
            risk_factors.append("high_obi_velocity_final_10s")
        
        # Factor 2: Extreme OBI with wide spread
        obi_norm = (obi + 1) / 2  # 0 to 1
        if obi_norm > 0.75 and spread_bps > 10:
            risk_score += 25
            risk_factors.append("extreme_obi_with_wide_spread")
        elif obi_norm < 0.25 and spread_bps > 10:
            risk_score += 25
            risk_factors.append("extreme_obi_sell_with_wide_spread")
        
        # Factor 3: Volatility expansion
        if len(self.volatility_history) >= 10:
            recent_vol = sum(list(self.volatility_history)[-5:]) / 5
            prior_vol = sum(list(self.volatility_history)[-10:-5]) / 5
            if recent_vol > prior_vol * 1.5:
                risk_score += 20
                risk_factors.append("volatility_expansion")
        
        # Factor 4: Thin liquidity (wide spreads)
        if spread_bps > 15:
            risk_score += 15
            risk_factors.append("thin_liquidity")
        
        # Classify risk
        if risk_score >= 70:
            classification = "HIGH_BREAK"
            recommendation = "VETO"
            confidence_reduction = 100  # Full veto
        elif risk_score >= 40:
            classification = "HIGH_HOLD"
            recommendation = "REDUCE_SIZE"
            confidence_reduction = 30
        elif risk_score >= 20:
            classification = "ELEVATED"
            recommendation = "CAUTION"
            confidence_reduction = 15
        else:
            classification = "LOW"
            recommendation = "PROCEED"
            confidence_reduction = 0
        
        return {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "risk_score": risk_score,
            "classification": classification,
            "recommendation": recommendation,
            "confidence_reduction": confidence_reduction,
            "risk_factors": risk_factors,
            "inputs": {
                "obi": obi,
                "obi_velocity": obi_velocity,
                "spread_bps": spread_bps,
                "volatility": volatility,
                "seconds_to_block_end": seconds_to_block_end
            }
        }
    
    def is_veto_recommended(self, risk_assessment: Dict) -> bool:
        """Check if veto is recommended"""
        return risk_assessment["recommendation"] == "VETO"


# =============================================================================
# INTEGRATION EXAMPLE
# =============================================================================

class ThesisAlignedOrchestrator:
    """
    Full orchestration with block timing and pinning detection
    """
    
    def __init__(self):
        self.block_timer = BlockTimer(interval_minutes=5)
        self.pinning_calc = PinningRiskCalculator()
        
    def run_decision_cycle(self, market_data: Dict) -> Dict:
        """
        Single decision cycle aligned with thesis timing
        """
        timing = self.block_timer.get_timing()
        
        # Update pinning calculator
        self.pinning_calc.update(
            obi=market_data.get("obi", 0),
            spread_bps=market_data.get("spread_bps", 0),
            volatility=market_data.get("volatility", 0)
        )
        
        # Calculate pinning risk
        pinning = self.pinning_calc.calculate_pinning_risk(
            obi=market_data.get("obi", 0),
            obi_velocity=market_data.get("obi_velocity", 0),
            spread_bps=market_data.get("spread_bps", 0),
            volatility=market_data.get("volatility", 0),
            seconds_to_block_end=timing.seconds_to_next_block
        )
        
        # Determine action based on phase
        action = {
            "timing": timing,
            "pinning_risk": pinning,
            "should_trade": False,
            "veto_applied": False
        }
        
        if timing.phase == "t-5_to_t-2":
            # CIO decision window
            if pinning["recommendation"] == "VETO":
                action["should_trade"] = False
                action["veto_applied"] = True
                action["veto_reason"] = f"Pinning risk: {pinning['classification']}"
            else:
                action["should_trade"] = True
                action["confidence_adjustment"] = -pinning["confidence_reduction"]
                
        return action


if __name__ == "__main__":
    # Test BlockTimer
    print("="*60)
    print("Testing BlockTimer")
    print("="*60)
    
    timer = BlockTimer(interval_minutes=5)
    
    for _ in range(3):
        timer.print_status()
        time.sleep(2)
    
    # Test PinningRiskCalculator
    print("\n" + "="*60)
    print("Testing PinningRiskCalculator")
    print("="*60)
    
    pinning = PinningRiskCalculator()
    
    # Simulate high risk scenario
    pinning.update(obi=0.8, spread_bps=12, volatility=2.5)
    risk = pinning.calculate_pinning_risk(
        obi=0.8,
        obi_velocity=0.25,
        spread_bps=12,
        volatility=2.5,
        seconds_to_block_end=5
    )
    
    print(f"\nHigh Risk Scenario:")
    print(f"   Risk Score: {risk['risk_score']}")
    print(f"   Classification: {risk['classification']}")
    print(f"   Recommendation: {risk['recommendation']}")
    print(f"   Factors: {risk['risk_factors']}")
    
    # Simulate low risk scenario
    risk2 = pinning.calculate_pinning_risk(
        obi=0.3,
        obi_velocity=0.05,
        spread_bps=3,
        volatility=0.8,
        seconds_to_block_end=8
    )
    
    print(f"\nLow Risk Scenario:")
    print(f"   Risk Score: {risk2['risk_score']}")
    print(f"   Classification: {risk2['classification']}")
    print(f"   Recommendation: {risk2['recommendation']}")
