"""
LLMHQ Paper Trading System - Data Collector
Fetches BTC price and orderbook data from Binance
"""

import requests
import json
import time
from datetime import datetime
from typing import Dict, Optional

class BinanceCollector:
    def __init__(self):
        self.base_url = "https://api.binance.com"
        self.price_url = f"{self.base_url}/api/v3/ticker/24hr"
        self.depth_url = f"{self.base_url}/api/v3/depth"
        self.klines_url = f"{self.base_url}/api/v3/klines"
        
    def get_ticker(self, symbol: str = "BTCUSDT") -> Optional[Dict]:
        """Get 24hr ticker data"""
        try:
            resp = requests.get(self.price_url, params={"symbol": symbol}, timeout=5)
            resp.raise_for_status()
            data = resp.json()
            return {
                "symbol": data["symbol"],
                "price": float(data["lastPrice"]),
                "bid": float(data["bidPrice"]),
                "ask": float(data["askPrice"]),
                "volume": float(data["volume"]),
                "quote_volume": float(data["quoteVolume"]),
                "price_change_pct": float(data["priceChangePercent"]),
                "weighted_avg_price": float(data["weightedAvgPrice"]),
                "timestamp": datetime.utcnow().isoformat()
            }
        except Exception as e:
            print(f"Error fetching ticker: {e}")
            return None
    
    def get_orderbook(self, symbol: str = "BTCUSDT", limit: int = 100) -> Optional[Dict]:
        """Get order book depth"""
        try:
            resp = requests.get(self.depth_url, params={"symbol": symbol, "limit": limit}, timeout=5)
            resp.raise_for_status()
            data = resp.json()
            
            bids = [[float(p), float(q)] for p, q in data["bids"]]
            asks = [[float(p), float(q)] for p, q in data["asks"]]
            
            # Calculate depth within 1% of mid price
            mid = (bids[0][0] + asks[0][0]) / 2
            pct_range = 0.01
            
            bid_depth = sum(q for p, q in bids if p >= mid * (1 - pct_range))
            ask_depth = sum(q for p, q in asks if p <= mid * (1 + pct_range))
            
            # Calculate OBI (Order Book Imbalance)
            total_depth = bid_depth + ask_depth
            obi = (bid_depth - ask_depth) / total_depth if total_depth > 0 else 0
            
            return {
                "mid_price": mid,
                "spread": asks[0][0] - bids[0][0],
                "spread_bps": ((asks[0][0] - bids[0][0]) / mid) * 10000,
                "bid_depth_1pct": bid_depth,
                "ask_depth_1pct": ask_depth,
                "obi": obi,
                "obi_normalized": (obi + 1) / 2,  # 0 to 1 scale
                "best_bid": bids[0][0],
                "best_ask": asks[0][0],
                "bid_ask_ratio": bid_depth / ask_depth if ask_depth > 0 else 1.0,
                "timestamp": datetime.utcnow().isoformat()
            }
        except Exception as e:
            print(f"Error fetching orderbook: {e}")
            return None
    
    def get_recent_klines(self, symbol: str = "BTCUSDT", interval: str = "1m", limit: int = 30) -> Optional[list]:
        """Get recent candlestick data"""
        try:
            resp = requests.get(
                self.klines_url, 
                params={"symbol": symbol, "interval": interval, "limit": limit},
                timeout=5
            )
            resp.raise_for_status()
            data = resp.json()
            
            candles = []
            for candle in data:
                candles.append({
                    "timestamp": candle[0],
                    "open": float(candle[1]),
                    "high": float(candle[2]),
                    "low": float(candle[3]),
                    "close": float(candle[4]),
                    "volume": float(candle[5]),
                    "quote_volume": float(candle[7])
                })
            return candles
        except Exception as e:
            print(f"Error fetching klines: {e}")
            return None

if __name__ == "__main__":
    collector = BinanceCollector()
    
    print("=== LLMHQ Data Collector Test ===")
    
    ticker = collector.get_ticker()
    if ticker:
        print(f"\nTicker: BTC @ ${ticker['price']:,.2f}")
        print(f"24h Change: {ticker['price_change_pct']:.2f}%")
    
    orderbook = collector.get_orderbook()
    if orderbook:
        print(f"\nOrderbook:")
        print(f"  Mid: ${orderbook['mid_price']:,.2f}")
        print(f"  Spread: {orderbook['spread_bps']:.2f} bps")
        print(f"  OBI: {orderbook['obi']:.3f} (normalized: {orderbook['obi_normalized']:.3f})")
        print(f"  Bid/Ask Ratio: {orderbook['bid_ask_ratio']:.2f}")
    
    klines = collector.get_recent_klines(limit=5)
    if klines:
        print(f"\nRecent 1m candles: {len(klines)}")
        print(f"  Latest close: ${klines[-1]['close']:,.2f}")
