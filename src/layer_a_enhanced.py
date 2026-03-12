"""
LLMHQ Layer A - Enhanced Configuration
Implements user's choices:
- Q1: Binance @forceOrder for liquidations (FREE)
- Q2: Binance Spot + Perp (BTCUSDT + BTCUSDT_PERP)
- Q3: Multi-stream correlation (ETH, XRP, SOL, MATIC)
"""

import asyncio
import json
import websockets
from datetime import datetime, timezone
from typing import Dict, Optional, List, Callable
from dataclasses import dataclass, field
from collections import deque
import sys
sys.path.insert(0, '/root/.openclaw/workspace/LLMHQm-work/src')

@dataclass
class LiquidationData:
    """Liquidation event from @forceOrder stream"""
    timestamp: float
    symbol: str
    side: str  # BUY (short liquidated) or SELL (long liquidated)
    price: float
    quantity: float
    usd_value: float

@dataclass  
class PerpData:
    """Perpetual futures data"""
    timestamp: float
    price: float
    funding_rate: float
    mark_price: float
    index_price: float

@dataclass
class CorrelationAssetData:
    """Price data for correlation tracking"""
    symbol: str
    timestamp: float
    price: float
    price_change_24h: float

class EnhancedBinanceCollector:
    """
    Enhanced collector with:
    - Spot trades (@trade)
    - Liquidations (@forceOrder) 
    - Perp data (@markPrice)
    - Multi-asset correlation streams
    """
    
    BINANCE_SPOT_WS = "wss://stream.binance.com:9443/ws"
    BINANCE_FUTURES_WS = "wss://fstream.binance.com/ws"
    
    def __init__(self):
        # Data storage
        self.spot_price: Optional[float] = None
        self.perp_price: Optional[float] = None
        self.mark_price: Optional[float] = None
        self.index_price: Optional[float] = None
        
        # Liquidations
        self.recent_liquidations: deque = deque(maxlen=100)
        self.liquidation_stats = {
            "long_liquidations_1m": 0,  # USD value
            "short_liquidations_1m": 0,
            "last_reset": datetime.now(timezone.utc).timestamp()
        }
        
        # Correlation assets
        self.correlation_prices: Dict[str, float] = {
            "BTC": None,
            "ETH": None,
            "XRP": None,
            "SOL": None,
            "MATIC": None  # Polygon
        }
        self.correlation_history: Dict[str, deque] = {
            "BTC": deque(maxlen=100),
            "ETH": deque(maxlen=100),
            "XRP": deque(maxlen=100),
            "SOL": deque(maxlen=100),
            "MATIC": deque(maxlen=100)
        }
        
        # Callbacks
        self.on_liquidation: Optional[Callable] = None
        self.on_basis_update: Optional[Callable] = None
        self.on_correlation_update: Optional[Callable] = None
        
        self.running = False
        
    async def connect_spot_streams(self):
        """Connect to spot streams for BTC + correlation assets"""
        streams = [
            "btcusdt@trade",
            "ethusdt@trade",
            "xrpusdt@trade",
            "solusdt@trade",
            "maticusdt@trade"  # Polygon
        ]
        
        stream_path = "/".join(streams)
        url = f"{self.BINANCE_SPOT_WS}/{stream_path}"
        
        print(f"[Layer A] Connecting to SPOT streams: BTC, ETH, XRP, SOL, MATIC")
        
        async with websockets.connect(url) as ws:
            while self.running:
                try:
                    msg = await asyncio.wait_for(ws.recv(), timeout=30)
                    await self._handle_spot_message(json.loads(msg))
                except asyncio.TimeoutError:
                    print("[Spot] Timeout, reconnecting...")
                    break
                except websockets.exceptions.ConnectionClosed:
                    print("[Spot] Connection closed, reconnecting...")
                    break
                    
    async def connect_liquidation_stream(self):
        """Connect to @forceOrder for liquidations"""
        url = f"{self.BINANCE_FUTURES_WS}/btcusdt@forceOrder"
        
        print(f"[Layer A] Connecting to LIQUIDATION stream (@forceOrder)")
        
        async with websockets.connect(url) as ws:
            while self.running:
                try:
                    msg = await asyncio.wait_for(ws.recv(), timeout=30)
                    await self._handle_liquidation(json.loads(msg))
                except asyncio.TimeoutError:
                    print("[Liquidation] Timeout, reconnecting...")
                    break
                except websockets.exceptions.ConnectionClosed:
                    print("[Liquidation] Connection closed, reconnecting...")
                    break
                    
    async def connect_perp_stream(self):
        """Connect to perp mark price stream"""
        url = f"{self.BINANCE_FUTURES_WS}/btcusdt@markPrice@1s"
        
        print(f"[Layer A] Connecting to PERP stream (@markPrice)")
        
        async with websockets.connect(url) as ws:
            while self.running:
                try:
                    msg = await asyncio.wait_for(ws.recv(), timeout=30)
                    await self._handle_perp_message(json.loads(msg))
                except asyncio.TimeoutError:
                    print("[Perp] Timeout, reconnecting...")
                    break
                except websockets.exceptions.ConnectionClosed:
                    print("[Perp] Connection closed, reconnecting...")
                    break
                    
    async def _handle_spot_message(self, data: Dict):
        """Handle spot trade messages"""
        symbol = data.get("s", "").upper()
        price = float(data.get("p", 0))
        
        # Map symbol to asset name
        asset_map = {
            "BTCUSDT": "BTC",
            "ETHUSDT": "ETH",
            "XRPUSDT": "XRP",
            "SOLUSDT": "SOL",
            "MATICUSDT": "MATIC"
        }
        
        asset = asset_map.get(symbol)
        if asset:
            self.correlation_prices[asset] = price
            self.correlation_history[asset].append({
                "timestamp": data.get("T", 0) / 1000,
                "price": price
            })
            
            if asset == "BTC":
                self.spot_price = price
                await self._check_basis_update()
                
        if self.on_correlation_update:
            await self.on_correlation_update(asset, price)
            
    async def _handle_liquidation(self, data: Dict):
        """Handle liquidation events"""
        order = data.get("o", {})
        
        liq = LiquidationData(
            timestamp=data.get("E", 0) / 1000,
            symbol=order.get("s", "BTCUSDT"),
            side=order.get("S", ""),  # BUY = short liquidated, SELL = long liquidated
            price=float(order.get("p", 0)),
            quantity=float(order.get("q", 0)),
            usd_value=float(order.get("p", 0)) * float(order.get("q", 0))
        )
        
        self.recent_liquidations.append(liq)
        
        # Update stats
        current_time = datetime.now(timezone.utc).timestamp()
        if current_time - self.liquidation_stats["last_reset"] > 60:
            # Reset 1-minute stats
            self.liquidation_stats["long_liquidations_1m"] = 0
            self.liquidation_stats["short_liquidations_1m"] = 0
            self.liquidation_stats["last_reset"] = current_time
        
        if liq.side == "BUY":  # Shorts getting liquidated
            self.liquidation_stats["short_liquidations_1m"] += liq.usd_value
        else:  # Longs getting liquidated
            self.liquidation_stats["long_liquidations_1m"] += liq.usd_value
        
        print(f"🚨 LIQUIDATION: {liq.side} {liq.quantity:.4f} BTC (${liq.usd_value:,.0f}) @ ${liq.price:,.2f}")
        
        if self.on_liquidation:
            await self.on_liquidation(liq)
            
    async def _handle_perp_message(self, data: Dict):
        """Handle perp mark price updates"""
        self.perp_price = float(data.get("p", 0))  # Mark price
        self.mark_price = float(data.get("p", 0))
        self.index_price = float(data.get("i", 0))  # Index price
        funding_rate = float(data.get("r", 0))  # Funding rate
        
        await self._check_basis_update()
        
        # Print funding rate periodically
        if abs(funding_rate) > 0.0001:  # Only if significant
            print(f"📊 FUNDING: {funding_rate*100:.4f}%")
            
    async def _check_basis_update(self):
        """Check and notify on basis updates"""
        if self.spot_price and self.perp_price:
            basis = self.perp_price - self.spot_price
            basis_bps = (basis / self.spot_price) * 10000
            
            if abs(basis_bps) > 5:  # Only if significant (>5 bps)
                print(f"⚡ BASIS: {basis_bps:+.1f} bps (Spot: ${self.spot_price:,.2f}, Perp: ${self.perp_price:,.2f})")
                
                if self.on_basis_update:
                    await self.on_basis_update({
                        "spot": self.spot_price,
                        "perp": self.perp_price,
                        "basis_usd": basis,
                        "basis_bps": basis_bps
                    })
                    
    def calculate_correlations(self) -> Dict:
        """Calculate correlations between BTC and other assets"""
        import math
        
        btc_returns = self._calculate_returns("BTC")
        correlations = {}
        
        for asset in ["ETH", "XRP", "SOL", "MATIC"]:
            asset_returns = self._calculate_returns(asset)
            if len(btc_returns) >= 10 and len(asset_returns) >= 10:
                corr = self._pearson_correlation(btc_returns, asset_returns)
                if corr is not None:
                    correlations[asset] = round(corr, 3)
                    
        return correlations
    
    def _calculate_returns(self, asset: str) -> List[float]:
        """Calculate price returns for an asset"""
        history = list(self.correlation_history[asset])
        if len(history) < 2:
            return []
        
        returns = []
        for i in range(1, len(history)):
            if history[i-1]["price"] > 0:
                ret = (history[i]["price"] - history[i-1]["price"]) / history[i-1]["price"]
                returns.append(ret)
        
        return returns
    
    def _pearson_correlation(self, x: List[float], y: List[float]) -> Optional[float]:
        """Calculate Pearson correlation"""
        n = min(len(x), len(y))
        if n < 10:
            return None
        
        x = x[-n:]
        y = y[-n:]
        
        mean_x = sum(x) / n
        mean_y = sum(y) / n
        
        numerator = sum((xi - mean_x) * (yi - mean_y) for xi, yi in zip(x, y))
        denom_x = sum((xi - mean_x) ** 2 for xi in x) ** 0.5
        denom_y = sum((yi - mean_y) ** 2 for yi in y) ** 0.5
        
        if denom_x == 0 or denom_y == 0:
            return 0
        
        return numerator / (denom_x * denom_y)
    
    def get_liquidation_summary(self) -> Dict:
        """Get current liquidation summary"""
        return {
            "long_liquidations_1m_usd": self.liquidation_stats["long_liquidations_1m"],
            "short_liquidations_1m_usd": self.liquidation_stats["short_liquidations_1m"],
            "net_liquidation_pressure": self.liquidation_stats["short_liquidations_1m"] - 
                                        self.liquidation_stats["long_liquidations_1m"],
            "recent_count": len(self.recent_liquidations),
            "last_liquidation": self.recent_liquidations[-1].__dict__ if self.recent_liquidations else None
        }
    
    def get_basis(self) -> Optional[Dict]:
        """Get current spot-perp basis"""
        if self.spot_price and self.perp_price:
            basis = self.perp_price - self.spot_price
            return {
                "spot": self.spot_price,
                "perp": self.perp_price,
                "basis_usd": basis,
                "basis_bps": (basis / self.spot_price) * 10000
            }
        return None
    
    def stop(self):
        """Stop all streams"""
        self.running = False


class EnhancedLayerAOrchestrator:
    """
    Complete Layer A with user's chosen configuration
    """
    
    def __init__(self):
        self.collector = EnhancedBinanceCollector()
        self.collector.on_liquidation = self._on_liquidation
        self.collector.on_basis_update = self._on_basis_update
        
    async def start(self):
        """Start all streams"""
        print("="*70)
        print("LLMHQ Layer A - Enhanced Configuration")
        print("="*70)
        print("\n✅ Configuration:")
        print("   • Whale Watcher: Binance @forceOrder (liquidations)")
        print("   • Cross-Exchange: Binance Spot + Perp")
        print("   • Correlation: BTC, ETH, XRP, SOL, MATIC")
        print("\n" + "-"*70)
        
        self.collector.running = True
        
        # Run all streams concurrently
        await asyncio.gather(
            self.collector.connect_spot_streams(),
            self.collector.connect_liquidation_stream(),
            self.collector.connect_perp_stream(),
            self._periodic_summary()
        )
        
    async def _on_liquidation(self, liq):
        """Handle liquidation event"""
        pass  # Already printed in collector
        
    async def _on_basis_update(self, basis_data):
        """Handle basis update"""
        pass  # Already printed in collector
        
    async def _periodic_summary(self):
        """Print periodic summary"""
        await asyncio.sleep(10)  # Wait for data collection
        
        while self.collector.running:
            print("\n" + "="*70)
            print(f"[Layer A Summary] {datetime.now(timezone.utc).strftime('%H:%M:%S')} UTC")
            print("="*70)
            
            # Prices
            print("\n💰 PRICES:")
            for asset, price in self.collector.correlation_prices.items():
                if price:
                    print(f"   {asset}: ${price:,.2f}")
            
            # Basis
            basis = self.collector.get_basis()
            if basis:
                print(f"\n⚡ BASIS:")
                print(f"   Spot: ${basis['spot']:,.2f}")
                print(f"   Perp: ${basis['perp']:,.2f}")
                print(f"   Spread: {basis['basis_bps']:+.1f} bps")
            
            # Liquidations
            liq = self.collector.get_liquidation_summary()
            print(f"\n🚨 LIQUIDATIONS (1m):")
            print(f"   Longs liquidated: ${liq['long_liquidations_1m_usd']:,.0f}")
            print(f"   Shorts liquidated: ${liq['short_liquidations_1m_usd']:,.0f}")
            print(f"   Net pressure: ${liq['net_liquidation_pressure']:,.0f}")
            
            # Correlations
            correlations = self.collector.calculate_correlations()
            if correlations:
                print(f"\n🔗 CORRELATIONS (vs BTC):")
                for asset, corr in correlations.items():
                    print(f"   {asset}: {corr:+.3f}")
            
            print("-"*70)
            
            await asyncio.sleep(30)  # Update every 30 seconds
            
    def stop(self):
        self.collector.stop()


async def main():
    """Run enhanced Layer A"""
    orchestrator = EnhancedLayerAOrchestrator()
    
    try:
        await orchestrator.start()
    except KeyboardInterrupt:
        print("\n\nStopping...")
        orchestrator.stop()


if __name__ == "__main__":
    asyncio.run(main())
