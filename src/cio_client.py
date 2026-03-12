"""
LLMHQ CIO Client
Formats market briefing and captures CIO decisions
"""

import json
import os
from datetime import datetime
from typing import Dict, Optional

class CIOClient:
    """Client for CIO decision workflow"""
    
    def __init__(self, log_dir: str = "logs"):
        self.log_dir = log_dir
        os.makedirs(log_dir, exist_ok=True)
    
    def format_briefing_dossier(
        self,
        ticker: Dict,
        orderbook: Dict,
        indicators: Dict,
        narrative: str,
        interval: str = "5m"
    ) -> Dict:
        """Format the three-layer briefing dossier"""
        
        return {
            "timestamp": datetime.utcnow().isoformat(),
            "interval": interval,
            "raw_snapshot": {
                "price": ticker.get("price"),
                "best_bid": orderbook.get("best_bid"),
                "best_ask": orderbook.get("best_ask"),
                "spread_bps": orderbook.get("spread_bps"),
                "volume_24h": ticker.get("volume"),
                "price_change_24h_pct": ticker.get("price_change_pct")
            },
            "derived_indicators": {
                "hma": indicators.get("hma"),
                "hma_slope": indicators.get("hma_slope"),
                "roc_3m": indicators.get("roc"),
                "rsi": indicators.get("rsi"),
                "volatility": indicators.get("volatility"),
                "obi": orderbook.get("obi"),
                "obi_normalized": orderbook.get("obi_normalized"),
                "bid_depth_1pct": orderbook.get("bid_depth_1pct"),
                "ask_depth_1pct": orderbook.get("ask_depth_1pct"),
                "regime": indicators.get("regime")
            },
            "semantic_narrative": narrative
        }
    
    def save_briefing(self, dossier: Dict, trade_id: str) -> str:
        """Save briefing to file for review"""
        filepath = os.path.join(self.log_dir, f"briefing_{trade_id}.json")
        with open(filepath, 'w') as f:
            json.dump(dossier, f, indent=2)
        return filepath
    
    def load_cio_prompt_template(self) -> str:
        """Load the CIO prompt template"""
        template_path = os.path.join("skills", "llmhq-trading-engine", "templates", "cio_prompt.md")
        if os.path.exists(template_path):
            with open(template_path, 'r') as f:
                return f.read()
        # Fallback template
        return self._default_cio_prompt()
    
    def _default_cio_prompt(self) -> str:
        return """You are the CIO for LLMHQ. Review the market briefing and issue a directional prediction.

Output format (JSON):
{
  "direction": "UP or DOWN",
  "confidence": 0-100,
  "regime": "trending|ranging|volatile_expansion|quiet_compression|manipulative",
  "lead_driver": "OBI|HMA|sentiment|whale_flow|other",
  "rationale": "brief explanation",
  "risk_flags": ["flag1"],
  "veto_applied": false,
  "veto_reason": ""
}

Briefing:
{BRIEFING}
"""
    
    def create_decision_record(
        self,
        briefing: Dict,
        cio_decision: Dict,
        execution_status: str,
        mode: str = "paper"
    ) -> Dict:
        """Create a complete decision record for logging"""
        
        return {
            "timestamp": briefing["timestamp"],
            "interval": briefing["interval"],
            "mode": mode,
            "raw_snapshot": briefing["raw_snapshot"],
            "derived_indicators": briefing["derived_indicators"],
            "semantic_narrative": briefing["semantic_narrative"],
            "cio_decision": cio_decision,
            "execution_status": execution_status,
            "logging_complete": True
        }
    
    def log_decision(self, record: Dict, trade_id: str) -> str:
        """Log the complete decision to file"""
        filepath = os.path.join(self.log_dir, f"decision_{trade_id}.json")
        with open(filepath, 'w') as f:
            json.dump(record, f, indent=2)
        return filepath

if __name__ == "__main__":
    # Test the CIO client
    client = CIOClient()
    
    sample_briefing = {
        "timestamp": datetime.utcnow().isoformat(),
        "interval": "5m",
        "raw_snapshot": {
            "price": 83900.50,
            "best_bid": 83899.00,
            "best_ask": 83902.00,
            "spread_bps": 3.6
        },
        "derived_indicators": {
            "hma_slope": 22.5,
            "obi": 0.65,
            "regime": "trending"
        },
        "semantic_narrative": "BTC at $83,900. Moderate upward momentum. Buy-side pressure elevated. Market in trending regime."
    }
    
    print("=== CIO Client Test ===")
    print(f"Briefing formatted for interval: {sample_briefing['interval']}")
    print(f"\nPrompt template loaded: {len(client.load_cio_prompt_template())} chars")
