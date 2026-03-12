"""
LLMHQ Additional Analysts - External Data Required
These analysts need external APIs - free options provided
"""

from typing import Dict, Optional, List
from collections import deque
from dataclasses import dataclass
from datetime import datetime
import requests

# =============================================================================
# ANALYST 4: WHALE WATCHER
# =============================================================================

class WhaleWatcherCalculator:
    """
    Analyst 4: Whale Watcher
    Detects large exchange inflows, liquidations, whale movements
    
    DATA SOURCE OPTIONS (choose one):
    
    OPTION A: CryptoQuant (API Key required)
    - https://cryptoquant.com/api
    - Free tier: Limited requests/day
    - Covers: Exchange flows, fund flows, miner flows
    
    OPTION B: Glassnode (API Key required)
    - https://docs.glassnode.com/
    - Free tier: 30 API calls/day
    - Covers: Exchange balances, inflows, outflows
    
    OPTION C: Whale Alert (Twitter/X API + Web scraping)
    - https://whale-alert.io/
    - Free tier: Limited, but good for alerts
    - Covers: Large transaction alerts
    
    OPTION D: Coinglass (Free API - RECOMMENDED for liquidations)
    - https://coinglass.com/
    - Free tier available
    - Covers: Liquidation data, exchange flows
    
    OPTION E: Binance Liquidation Streams (FREE - via WebSocket)
    - {}@forceOrder stream
    - Real-time liquidation data
    """
    
    def __init__(self, data_source: str = "coinglass"):
        self.data_source = data_source
        self.alerts: deque = deque(maxlen=100)
        self.exchange_flows: Dict = {}
        
    def fetch_liquidations(self) -> Dict:
        """
        Fetch liquidation data
        
        FREE: Use Binance @forceOrder WebSocket or Coinglass API
        """
        # Placeholder - implement based on chosen data source
        return {
            "status": "not_implemented",
            "data_source": self.data_source,
            "liquidations_24h": None,
            "long_liquidations": None,
            "short_liquidations": None
        }
    
    def fetch_exchange_flows(self) -> Dict:
        """
        Fetch exchange inflow/outflow data
        
        FREE: Glassnode (30 calls/day) or CryptoQuant (limited)
        """
        return {
            "status": "not_implemented",
            "data_source": self.data_source,
            "netflow": None,
            "inflows": None,
            "outflows": None
        }
    
    def check_whale_transactions(self) -> List[Dict]:
        """
        Check for large whale transactions
        
        FREE: Whale Alert API or blockchain explorers
        """
        return []
    
    def calculate(self) -> Dict:
        """Generate whale watcher signal"""
        return {
            "timestamp": datetime.utcnow().isoformat(),
            "data_source": self.data_source,
            "alert_level": "none",  # none, low, medium, high
            "alerts": [],
            "net_exchange_flow": None,
            "liquidation_pressure": None,
            "signal": "neutral",
            "note": "Configure data source to enable whale detection"
        }


# =============================================================================
# ANALYST 5: SOCIAL SENTIMENT FILTER
# =============================================================================

class SocialSentimentCalculator:
    """
    Analyst 5: Social Sentiment Filter
    Extracts market sentiment from social sources
    
    DATA SOURCE OPTIONS (choose one):
    
    OPTION A: Alternative.me Fear & Greed Index (FREE - RECOMMENDED)
    - https://alternative.me/crypto/fear-and-greed-index/
    - Completely free, no API key
    - JSON endpoint: https://api.alternative.me/fng/?limit=1
    
    OPTION B: LunarCRUSH (API Key required)
    - https://lunarcrush.com/
    - Free tier: Limited metrics
    - Covers: Social volume, sentiment, influencers
    
    OPTION C: Santiment (API Key required)
    - https://santiment.net/
    - Free tier: Limited metrics
    - Covers: On-chain + social sentiment
    
    OPTION D: Twitter/X API v2 (Paid mostly, limited free)
    - https://developer.twitter.com/
    - Basic tier has limits
    
    OPTION E: CryptoPanic (FREE)
    - https://cryptopanic.com/developers/api/
    - Free tier available
    - News sentiment aggregator
    """
    
    def __init__(self, data_source: str = "fear_greed"):
        self.data_source = data_source
        self.sentiment_history: deque = deque(maxlen=50)
        
    def fetch_fear_greed_index(self) -> Optional[Dict]:
        """
        Fetch Fear & Greed Index from Alternative.me
        
        FREE - No API key required
        """
        try:
            resp = requests.get("https://api.alternative.me/fng/?limit=1", timeout=5)
            resp.raise_for_status()
            data = resp.json()
            
            if data.get("data"):
                item = data["data"][0]
                return {
                    "value": int(item["value"]),
                    "classification": item["value_classification"],
                    "timestamp": int(item["timestamp"])
                }
        except Exception as e:
            print(f"[Sentiment] Error fetching fear/greed: {e}")
        
        return None
    
    def calculate(self) -> Dict:
        """Generate sentiment reading"""
        
        if self.data_source == "fear_greed":
            fg = self.fetch_fear_greed_index()
            
            if fg:
                self.sentiment_history.append(fg["value"])
                
                # Classify sentiment
                value = fg["value"]
                if value >= 75:
                    sentiment = "extreme_greed"
                    signal = "caution_long"
                elif value >= 55:
                    sentiment = "greed"
                    signal = "neutral"
                elif value >= 45:
                    sentiment = "neutral"
                    signal = "neutral"
                elif value >= 25:
                    sentiment = "fear"
                    signal = "opportunity"
                else:
                    sentiment = "extreme_fear"
                    signal = "strong_opportunity"
                
                return {
                    "timestamp": datetime.utcnow().isoformat(),
                    "data_source": "alternative.me (fear/greed)",
                    "fear_greed_value": value,
                    "classification": fg["classification"],
                    "sentiment": sentiment,
                    "signal": signal,
                    "note": f"Market is showing {fg['classification']}"
                }
        
        return {
            "timestamp": datetime.utcnow().isoformat(),
            "data_source": self.data_source,
            "sentiment": "neutral",
            "signal": "neutral",
            "note": "Configure data source to enable sentiment analysis"
        }


# =============================================================================
# ANALYST 6: CROSS-EXCHANGE LEAD-LAG MONITOR
# =============================================================================

class CrossExchangeCalculator:
    """
    Analyst 6: Cross-Exchange Lead-Lag Monitor
    Compares spot vs perp vs other venues
    
    DATA SOURCE OPTIONS:
    
    OPTION A: Multiple Binance Streams (FREE - RECOMMENDED)
    - BTCUSDT (spot) @ Binance
    - BTCUSDT_PERP (perp) @ Binance Futures
    - Compare in real-time
    
    OPTION B: CoinGecko API (FREE tier)
    - https://www.coingecko.com/en/api
    - Free tier: Limited calls
    - Covers: Prices across exchanges
    
    OPTION C: CoinMarketCap API (API Key required)
    - https://coinmarketcap.com/api/
    - Free tier: Limited calls
    
    OPTION D: CryptoCompare API (FREE tier)
    - https://min-api.cryptocompare.com/
    - Free tier available
    """
    
    def __init__(self):
        self.spot_price: Optional[float] = None
        self.perp_price: Optional[float] = None
        self.price_history: deque = deque(maxlen=100)
        
    def update_spot(self, price: float):
        """Update spot price"""
        self.spot_price = price
        
    def update_perp(self, price: float):
        """Update perp price"""
        self.perp_price = price
        
    def calculate(self) -> Dict:
        """Calculate lead-lag metrics"""
        
        if self.spot_price and self.perp_price:
            basis = self.perp_price - self.spot_price
            basis_bps = (basis / self.spot_price) * 10000
            
            # Classify basis
            if basis_bps > 10:
                perp_bias = "strong_premium"
                signal = "perp_leading_bullish"
            elif basis_bps > 5:
                perp_bias = "premium"
                signal = "perp_bullish"
            elif basis_bps < -10:
                perp_bias = "strong_discount"
                signal = "perp_leading_bearish"
            elif basis_bps < -5:
                perp_bias = "discount"
                signal = "perp_bearish"
            else:
                perp_bias = "neutral"
                signal = "aligned"
            
            return {
                "timestamp": datetime.utcnow().isoformat(),
                "spot_price": self.spot_price,
                "perp_price": self.perp_price,
                "basis_usd": round(basis, 2),
                "basis_bps": round(basis_bps, 2),
                "perp_bias": perp_bias,
                "signal": signal,
                "note": f"Perp trading at {basis_bps:+.1f} bps to spot"
            }
        
        return {
            "timestamp": datetime.utcnow().isoformat(),
            "status": "waiting_for_data",
            "note": "Waiting for both spot and perp price feeds"
        }


# =============================================================================
# ANALYST 7: LIQUIDITY MAP / VOID DETECTOR
# =============================================================================

class LiquidityMapCalculator:
    """
    Analyst 7: Liquidity Map / Void Detector
    Identifies stop clusters, thin zones, air pockets
    
    DATA SOURCE:
    - Uses Binance order book data (FREE via WebSocket)
    - No external API required
    """
    
    def __init__(self, lookback_levels: int = 20):
        self.lookback_levels = lookback_levels
        self.order_book_snapshots: deque = deque(maxlen=50)
        
    def update_orderbook(self, bids: List, asks: List, current_price: float):
        """Process order book snapshot"""
        self.order_book_snapshots.append({
            "timestamp": datetime.utcnow().isoformat(),
            "bids": bids[:self.lookback_levels],
            "asks": asks[:self.lookback_levels],
            "price": current_price
        })
        
    def find_liquidity_voids(self, bids: List, asks: List, current_price: float) -> Dict:
        """
        Find air pockets in the order book
        """
        voids_above = []
        voids_below = []
        
        # Analyze ask side (resistance)
        if len(asks) >= 2:
            for i in range(len(asks) - 1):
                current = asks[i]
                next_level = asks[i + 1]
                
                price_gap = next_level[0] - current[0]
                pct_gap = (price_gap / current[0]) * 100
                
                # If gap > 0.1%, it's a potential void
                if pct_gap > 0.1:
                    voids_above.append({
                        "from": current[0],
                        "to": next_level[0],
                        "gap_pct": round(pct_gap, 4),
                        "distance_from_price": round(((current[0] - current_price) / current_price) * 100, 4)
                    })
        
        # Analyze bid side (support)
        if len(bids) >= 2:
            for i in range(len(bids) - 1):
                current = bids[i]
                next_level = bids[i + 1]
                
                price_gap = current[0] - next_level[0]
                pct_gap = (price_gap / current[0]) * 100
                
                if pct_gap > 0.1:
                    voids_below.append({
                        "from": current[0],
                        "to": next_level[0],
                        "gap_pct": round(pct_gap, 4),
                        "distance_from_price": round(((current_price - current[0]) / current_price) * 100, 4)
                    })
        
        return {
            "voids_above": voids_above[:3],  # Top 3
            "voids_below": voids_below[:3],
            "closest_resistance_void": voids_above[0] if voids_above else None,
            "closest_support_void": voids_below[0] if voids_below else None
        }
    
    def calculate(self, bids: List, asks: List, current_price: float) -> Dict:
        """Generate liquidity map reading"""
        
        voids = self.find_liquidity_voids(bids, asks, current_price)
        
        # Calculate wall strength
        bid_wall = max(bids, key=lambda x: x[1]) if bids else [0, 0]
        ask_wall = max(asks, key=lambda x: x[1]) if asks else [0, 0]
        
        # Determine if we're near a wall
        near_bid_wall = abs(current_price - bid_wall[0]) / current_price < 0.001
        near_ask_wall = abs(current_price - ask_wall[0]) / current_price < 0.001
        
        return {
            "timestamp": datetime.utcnow().isoformat(),
            "current_price": current_price,
            "bid_wall": {"price": bid_wall[0], "size": bid_wall[1]},
            "ask_wall": {"price": ask_wall[0], "size": ask_wall[1]},
            "near_bid_wall": near_bid_wall,
            "near_ask_wall": near_ask_wall,
            "voids": voids,
            "signal": "approaching_support" if near_bid_wall else "approaching_resistance" if near_ask_wall else "mid_range"
        }


# =============================================================================
# ANALYST 8: CORRELATION CHECKER
# =============================================================================

class CorrelationCalculator:
    """
    Analyst 8: Correlation Checker
    Checks BTC correlation with ETH, SOL, indices
    
    DATA SOURCE OPTIONS:
    
    OPTION A: Binance Multi-Stream (FREE - RECOMMENDED)
    - Subscribe to BTCUSDT, ETHUSDT, SOLUSDT simultaneously
    - Calculate correlation in real-time
    
    OPTION B: Yahoo Finance API (FREE via yfinance)
    - For SPX, NDX, DXY correlations
    - pip install yfinance
    
    OPTION C: CoinGecko (FREE tier)
    - Market data for multiple assets
    """
    
    def __init__(self, correlation_assets: List[str] = None):
        self.correlation_assets = correlation_assets or ["ETH", "SOL"]
        self.price_histories: Dict[str, deque] = {
            "BTC": deque(maxlen=100)
        }
        for asset in self.correlation_assets:
            self.price_histories[asset] = deque(maxlen=100)
            
    def update_price(self, asset: str, price: float):
        """Update price for an asset"""
        if asset in self.price_histories:
            self.price_histories[asset].append(price)
            
    def calculate_correlation(self, asset1: str, asset2: str) -> Optional[float]:
        """Calculate Pearson correlation between two assets"""
        
        hist1 = list(self.price_histories[asset1])
        hist2 = list(self.price_histories[asset2])
        
        if len(hist1) < 20 or len(hist2) < 20:
            return None
        
        # Ensure same length
        min_len = min(len(hist1), len(hist2))
        hist1 = hist1[-min_len:]
        hist2 = hist2[-min_len:]
        
        # Calculate returns
        ret1 = [(hist1[i] - hist1[i-1]) / hist1[i-1] for i in range(1, len(hist1))]
        ret2 = [(hist2[i] - hist2[i-1]) / hist2[i-1] for i in range(1, len(hist2))]
        
        if len(ret1) < 10:
            return None
        
        # Pearson correlation
        n = len(ret1)
        sum1 = sum(ret1)
        sum2 = sum(ret2)
        sum1_sq = sum(x**2 for x in ret1)
        sum2_sq = sum(x**2 for x in ret2)
        psum = sum(ret1[i] * ret2[i] for i in range(n))
        
        num = psum - (sum1 * sum2 / n)
        den = ((sum1_sq - sum1**2 / n) * (sum2_sq - sum2**2 / n)) ** 0.5
        
        if den == 0:
            return 0
        
        return num / den
    
    def calculate(self) -> Dict:
        """Generate correlation reading"""
        
        btc_len = len(self.price_histories["BTC"])
        
        if btc_len < 20:
            return {
                "timestamp": datetime.utcnow().isoformat(),
                "status": "collecting_data",
                "btc_price_history": btc_len,
                "note": f"Need 20+ samples, have {btc_len}"
            }
        
        correlations = {}
        for asset in self.correlation_assets:
            corr = self.calculate_correlation("BTC", asset)
            if corr is not None:
                correlations[asset] = round(corr, 3)
        
        # Determine regime based on correlation
        if correlations:
            avg_corr = sum(correlations.values()) / len(correlations)
            
            if avg_corr > 0.8:
                regime = "high_correlation"
                signal = "crypto_moving_together"
            elif avg_corr < 0.3:
                regime = "divergent"
                signal = "btc_leading_or_isolated"
            else:
                regime = "normal_correlation"
                signal = "mixed_signals"
        else:
            regime = "unknown"
            signal = "insufficient_data"
        
        return {
            "timestamp": datetime.utcnow().isoformat(),
            "correlations": correlations,
            "average_correlation": round(sum(correlations.values()) / len(correlations), 3) if correlations else None,
            "regime": regime,
            "signal": signal,
            "note": f"BTC correlation with alts: {correlations}"
        }


# =============================================================================
# DATA SOURCE CONFIGURATION HELPER
# =============================================================================

def print_data_source_options():
    """Print all data source options for user selection"""
    
    print("""
╔══════════════════════════════════════════════════════════════════════════╗
║                    LLMHQ DATA SOURCE SELECTION                           ║
║                         Free Options Available                           ║
╠══════════════════════════════════════════════════════════════════════════╣

📊 ANALYST 1: TAPE READER
   Status: ✅ IMPLEMENTED (Binance WebSocket - FREE)
   Data: Order book depth, OBI, spread

📈 ANALYST 2: MOMENTUM ENGINE  
   Status: ✅ IMPLEMENTED (Binance WebSocket - FREE)
   Data: HMA, slope, ROC, RSI

🔬 ANALYST 3: MICROSTRUCTURE ENGINE
   Status: ✅ IMPLEMENTED (Binance WebSocket - FREE)
   Data: VPIN, volatility, toxicity

🐋 ANALYST 4: WHALE WATCHER
   Status: ⏳ NEEDS CONFIGURATION
   Options:
     A) Coinglass (Free tier) - Liquidations + Exchange flows ⭐ RECOMMENDED
     B) Binance @forceOrder (FREE) - Real-time liquidations only
     C) Glassnode (30 calls/day free) - Exchange flows
     D) CryptoQuant (Limited free) - Exchange flows

😊 ANALYST 5: SOCIAL SENTIMENT FILTER
   Status: ✅ IMPLEMENTED (Alternative.me - FREE)
   Data: Fear & Greed Index
   Alternative: LunarCRUSH, Santiment (API key required)

⚡ ANALYST 6: CROSS-EXCHANGE MONITOR
   Status: ⏳ NEEDS CONFIGURATION
   Options:
     A) Binance Spot + Perp (FREE) - Same exchange ⭐ RECOMMENDED
     B) CryptoCompare (Free tier) - Multi-exchange

🗺️  ANALYST 7: LIQUIDITY MAP
   Status: ✅ IMPLEMENTED (Binance WebSocket - FREE)
   Data: Order book voids, walls

🔗 ANALYST 8: CORRELATION CHECKER
   Status: ⏳ NEEDS CONFIGURATION
   Options:
     A) Binance Multi-Stream (FREE) - BTC/ETH/SOL ⭐ RECOMMENDED
     B) Yahoo Finance (FREE) - Crypto + TradFi correlations

╚══════════════════════════════════════════════════════════════════════════╝
""")


if __name__ == "__main__":
    print_data_source_options()
