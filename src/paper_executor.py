"""
LLMHQ Paper Executor
Simulates trade execution and logs paper trades
"""

import json
import os
import sqlite3
from datetime import datetime
from typing import Dict, Optional, List

class PaperExecutor:
    """Execute paper trades and maintain paper trading ledger"""
    
    def __init__(self, db_path: str = "db/paper_trades.db"):
        self.db_path = db_path
        os.makedirs(os.path.dirname(db_path), exist_ok=True)
        self._init_db()
    
    def _init_db(self):
        """Initialize SQLite database for paper trades"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS paper_trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                trade_id TEXT UNIQUE,
                timestamp TEXT,
                interval TEXT,
                direction TEXT,
                confidence INTEGER,
                entry_price REAL,
                regime TEXT,
                lead_driver TEXT,
                rationale TEXT,
                risk_flags TEXT,
                veto_applied INTEGER,
                execution_status TEXT,
                outcome TEXT,
                pnl_pct REAL,
                resolved INTEGER DEFAULT 0,
                resolved_at TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
        ''')
        
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS trade_decisions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                trade_id TEXT,
                raw_snapshot TEXT,
                derived_indicators TEXT,
                semantic_narrative TEXT,
                cio_decision TEXT,
                FOREIGN KEY (trade_id) REFERENCES paper_trades(trade_id)
            )
        ''')
        
        conn.commit()
        conn.close()
    
    def check_thresholds(self, confidence: int, min_confidence: int = 65) -> Dict:
        """Check if decision meets execution thresholds"""
        return {
            "confidence_sufficient": confidence >= min_confidence,
            "min_confidence": min_confidence,
            "actual_confidence": confidence,
            "passed": confidence >= min_confidence
        }
    
    def execute_paper_trade(
        self,
        trade_id: str,
        briefing: Dict,
        cio_decision: Dict,
        threshold_checks: Dict
    ) -> Dict:
        """Execute a paper trade (simulated)"""
        
        if not threshold_checks["passed"]:
            return {
                "status": "BLOCKED",
                "reason": f"Confidence {cio_decision['confidence']} below threshold {threshold_checks['min_confidence']}",
                "trade_id": trade_id
            }
        
        if cio_decision.get("veto_applied"):
            return {
                "status": "VETOED",
                "reason": cio_decision.get("veto_reason", "No reason given"),
                "trade_id": trade_id
            }
        
        # Simulate execution
        execution = {
            "status": "EXECUTED",
            "trade_id": trade_id,
            "timestamp": datetime.utcnow().isoformat(),
            "direction": cio_decision["direction"],
            "entry_price": briefing["raw_snapshot"]["price"],
            "confidence": cio_decision["confidence"],
            "regime": cio_decision["regime"],
            "simulated": True
        }
        
        # Log to database
        self._log_trade(trade_id, briefing, cio_decision, execution)
        
        return execution
    
    def _log_trade(
        self,
        trade_id: str,
        briefing: Dict,
        cio_decision: Dict,
        execution: Dict
    ):
        """Log trade to SQLite database"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        
        cursor.execute('''
            INSERT INTO paper_trades 
            (trade_id, timestamp, interval, direction, confidence, entry_price, 
             regime, lead_driver, rationale, risk_flags, veto_applied, execution_status)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ''', (
            trade_id,
            briefing["timestamp"],
            briefing["interval"],
            cio_decision["direction"],
            cio_decision["confidence"],
            briefing["raw_snapshot"]["price"],
            cio_decision["regime"],
            cio_decision["lead_driver"],
            cio_decision["rationale"],
            json.dumps(cio_decision.get("risk_flags", [])),
            1 if cio_decision.get("veto_applied") else 0,
            execution["status"]
        ))
        
        cursor.execute('''
            INSERT INTO trade_decisions 
            (trade_id, raw_snapshot, derived_indicators, semantic_narrative, cio_decision)
            VALUES (?, ?, ?, ?, ?)
        ''', (
            trade_id,
            json.dumps(briefing["raw_snapshot"]),
            json.dumps(briefing["derived_indicators"]),
            briefing["semantic_narrative"],
            json.dumps(cio_decision)
        ))
        
        conn.commit()
        conn.close()
    
    def get_open_trades(self) -> List[Dict]:
        """Get all unresolved paper trades"""
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()
        
        cursor.execute('''
            SELECT * FROM paper_trades 
            WHERE resolved = 0 AND execution_status = 'EXECUTED'
            ORDER BY timestamp DESC
        ''')
        
        trades = [dict(row) for row in cursor.fetchall()]
        conn.close()
        return trades
    
    def resolve_trade(
        self,
        trade_id: str,
        outcome: str,  # "WIN" or "LOSS"
        final_price: float
    ):
        """Resolve a paper trade with outcome"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        
        # Get entry price
        cursor.execute("SELECT entry_price, direction FROM paper_trades WHERE trade_id = ?", (trade_id,))
        row = cursor.fetchone()
        
        if row:
            entry_price, direction = row
            
            # Calculate P&L
            if direction == "UP":
                pnl_pct = ((final_price - entry_price) / entry_price) * 100
            else:  # DOWN
                pnl_pct = ((entry_price - final_price) / entry_price) * 100
            
            cursor.execute('''
                UPDATE paper_trades 
                SET resolved = 1, outcome = ?, pnl_pct = ?, resolved_at = ?
                WHERE trade_id = ?
            ''', (outcome, pnl_pct, datetime.utcnow().isoformat(), trade_id))
            
            conn.commit()
        
        conn.close()
    
    def get_stats(self) -> Dict:
        """Get paper trading statistics"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        
        # Total trades
        cursor.execute("SELECT COUNT(*) FROM paper_trades WHERE execution_status = 'EXECUTED'")
        total = cursor.fetchone()[0]
        
        # Wins/losses
        cursor.execute("SELECT COUNT(*), AVG(pnl_pct) FROM paper_trades WHERE outcome = 'WIN'")
        wins, win_pnl = cursor.fetchone()
        
        cursor.execute("SELECT COUNT(*), AVG(pnl_pct) FROM paper_trades WHERE outcome = 'LOSS'")
        losses, loss_pnl = cursor.fetchone()
        
        cursor.execute("SELECT COUNT(*) FROM paper_trades WHERE resolved = 0 AND execution_status = 'EXECUTED'")
        open_trades = cursor.fetchone()[0]
        
        conn.close()
        
        return {
            "total_trades": total,
            "wins": wins or 0,
            "losses": losses or 0,
            "open_trades": open_trades,
            "win_rate": (wins / (wins + losses) * 100) if (wins + losses) > 0 else 0,
            "avg_win_pnl": win_pnl or 0,
            "avg_loss_pnl": loss_pnl or 0
        }

if __name__ == "__main__":
    # Test paper executor
    executor = PaperExecutor()
    
    print("=== Paper Executor Test ===")
    
    # Test threshold check
    check = executor.check_thresholds(confidence=75, min_confidence=65)
    print(f"Threshold check (75 >= 65): {check['passed']}")
    
    # Show stats
    stats = executor.get_stats()
    print(f"\nPaper Trading Stats:")
    print(f"  Total trades: {stats['total_trades']}")
    print(f"  Win rate: {stats['win_rate']:.1f}%")
