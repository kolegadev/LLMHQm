"""
LLMHQ Layer A - Real-Time Sensory Array
Binance WebSocket Collector for BTC data
"""

import asyncio
import json
import websockets
from datetime import datetime
from typing import Callable, Dict, Optional, List
from dataclasses import dataclass, field
from collections import deque

@dataclass
class TickData:
    """Single tick/price update"""
    timestamp: float
    price: float
    quantity: float
    is_buyer_maker: bool  # True if seller is maker (buyer is taker)
    
@dataclass
class OrderBookLevel:
    """Single price level in order book"""
    price: float
    quantity: float
    
@dataclass
class OrderBookSnapshot:
    """Full order book state"""
    timestamp: float
    last_update_id: int
    bids: List[OrderBookLevel] = field(default_factory=list)
    asks: List[OrderBookLevel] = field(default_factory=list)

@dataclass
class CandleData:
    """OHLCV candle"""
    timestamp: int  # Open time
    open: float
    high: float
    low: float
    close: float
    volume: float
    quote_volume: float
    trades: int

class BinanceWebSocketCollector:
    """
    Real-time WebSocket collector from Binance
    Collects: trades, order book depth, klines
    """
    
    BINANCE_WS_URL = "wss://stream.binance.com:9443/ws"
    
    def __init__(self, symbol: str = "btcusdt"):
        self.symbol = symbol.lower()
        self.running = False
        
        # Data storage with size limits for memory management
        self.recent_trades: deque = deque(maxlen=1000)
        self.order_book: Optional[OrderBookSnapshot] = None
        self.current_candle: Optional[CandleData] = None
        self.candle_history: deque = deque(maxlen=100)
        
        # Callbacks for real-time processing
        self.on_trade: Optional[Callable] = None
        self.on_orderbook_update: Optional[Callable] = None
        self.on_candle: Optional[Callable] = None
        
        # Stats
        self.messages_received = 0
        self.connection_start: Optional[datetime] = None
        
    async def connect(self, streams: List[str] = None):
        """
        Connect to Binance WebSocket streams
        
        Available streams:
        - {symbol}@trade - Real-time trades
        - {symbol}@depth@100ms - Order book updates (100ms)
        - {symbol}@kline_1m - 1-minute candles
        - {symbol}@ticker - 24hr ticker
        """
        if streams is None:
            streams = [
                f"{self.symbol}@trade",
                f"{self.symbol}@depth@100ms",
                f"{self.symbol}@kline_1m",
                f"{self.symbol}@ticker"
            ]
        
        # Build combined stream URL
        stream_path = "/".join(streams)
        url = f"{self.BINANCE_WS_URL}/{stream_path}"
        
        self.running = True
        self.connection_start = datetime.utcnow()
        
        print(f"[Layer A] Connecting to Binance WebSocket...")
        print(f"[Layer A] Streams: {streams}")
        
        async with websockets.connect(url) as websocket:
            print(f"[Layer A] Connected to {url}")
            
            while self.running:
                try:
                    message = await asyncio.wait_for(websocket.recv(), timeout=30)
                    await self._handle_message(json.loads(message))
                except asyncio.TimeoutError:
                    print("[Layer A] Connection timeout, reconnecting...")
                    break
                except websockets.exceptions.ConnectionClosed:
                    print("[Layer A] Connection closed, reconnecting...")
                    break
                    
    async def _handle_message(self, data: Dict):
        """Route incoming WebSocket messages"""
        self.messages_received += 1
        
        stream = data.get("e", "")
        
        if stream == "trade":
            await self._handle_trade(data)
        elif stream == "depthUpdate":
            await self._handle_depth_update(data)
        elif stream == "kline":
            await self._handle_kline(data)
            
    async def _handle_trade(self, data: Dict):
        """Process trade tick"""
        tick = TickData(
            timestamp=data["T"] / 1000,  # ms to seconds
            price=float(data["p"]),
            quantity=float(data["q"]),
            is_buyer_maker=data["m"]
        )
        
        self.recent_trades.append(tick)
        
        if self.on_trade:
            await self.on_trade(tick)
            
    async def _handle_depth_update(self, data: Dict):
        """Process order book update"""
        # For simplicity, we'll accumulate updates
        # In production, you'd maintain a full order book with diffs
        bids = [OrderBookLevel(float(p), float(q)) for p, q in data.get("b", [])]
        asks = [OrderBookLevel(float(p), float(q)) for p, q in data.get("a", [])]
        
        self.order_book = OrderBookSnapshot(
            timestamp=data["E"] / 1000,
            last_update_id=data["u"],
            bids=bids,
            asks=asks
        )
        
        if self.on_orderbook_update:
            await self.on_orderbook_update(self.order_book)
            
    async def _handle_kline(self, data: Dict):
        """Process kline/candle update"""
        k = data["k"]
        candle = CandleData(
            timestamp=k["t"],
            open=float(k["o"]),
            high=float(k["h"]),
            low=float(k["l"]),
            close=float(k["c"]),
            volume=float(k["v"]),
            quote_volume=float(k["q"]),
            trades=k["n"]
        )
        
        self.current_candle = candle
        
        # Store completed candles
        if k["x"]:  # Is candle closed
            self.candle_history.append(candle)
            if self.on_candle:
                await self.on_candle(candle)
                
    def get_recent_trade_prices(self, n: int = 100) -> List[float]:
        """Get recent trade prices for indicator calculation"""
        return [t.price for t in list(self.recent_trades)[-n:]]
    
    def get_order_book_imbalance(self, depth_levels: int = 10) -> Dict:
        """Calculate order book imbalance from current snapshot"""
        if not self.order_book:
            return {"obi": 0, "bid_depth": 0, "ask_depth": 0}
        
        bids = self.order_book.bids[:depth_levels]
        asks = self.order_book.asks[:depth_levels]
        
        bid_depth = sum(b.quantity for b in bids)
        ask_depth = sum(a.quantity for a in asks)
        
        total = bid_depth + ask_depth
        obi = (bid_depth - ask_depth) / total if total > 0 else 0
        
        return {
            "obi": obi,
            "obi_normalized": (obi + 1) / 2,  # 0-1 scale
            "bid_depth": bid_depth,
            "ask_depth": ask_depth,
            "bid_ask_ratio": bid_depth / ask_depth if ask_depth > 0 else 1.0,
            "best_bid": bids[0].price if bids else 0,
            "best_ask": asks[0].price if asks else 0,
            "spread": asks[0].price - bids[0].price if bids and asks else 0
        }
    
    def get_candle_history(self, n: int = 100) -> List[CandleData]:
        """Get historical candles"""
        return list(self.candle_history)[-n:]
    
    def stop(self):
        """Stop the collector"""
        self.running = False
        
    def get_stats(self) -> Dict:
        """Get collector statistics"""
        return {
            "messages_received": self.messages_received,
            "trades_buffered": len(self.recent_trades),
            "candles_buffered": len(self.candle_history),
            "connected_since": self.connection_start.isoformat() if self.connection_start else None
        }


# =============================================================================
# INDEPENDENT ANALYZER CALCULATORS
# Each can be refined independently
# =============================================================================

class TapeReaderCalculator:
    """
    Analyst 1: Tape Reader
    Calculates order book pressure and flow metrics
    """
    
    def __init__(self):
        self.obi_history: deque = deque(maxlen=100)
        self.pressure_readings: deque = deque(maxlen=50)
        
    def calculate(self, order_book_data: Dict) -> Dict:
        """
        Calculate tape/flow metrics
        
        FREE DATA SOURCE: Binance WebSocket (depth@100ms)
        """
        obi = order_book_data.get("obi", 0)
        self.obi_history.append(obi)
        
        # OBI velocity (rate of change)
        obi_velocity = 0
        if len(self.obi_history) >= 10:
            recent = list(self.obi_history)[-10:]
            obi_velocity = (recent[-1] - recent[0]) / len(recent)
        
        # Pressure classification
        obi_norm = order_book_data.get("obi_normalized", 0.5)
        if obi_norm > 0.75:
            pressure = "strong_buy"
        elif obi_norm > 0.6:
            pressure = "buy"
        elif obi_norm < 0.25:
            pressure = "strong_sell"
        elif obi_norm < 0.4:
            pressure = "sell"
        else:
            pressure = "neutral"
        
        # Spread analysis
        spread = order_book_data.get("spread", 0)
        mid = (order_book_data.get("best_bid", 0) + order_book_data.get("best_ask", 0)) / 2
        spread_bps = (spread / mid * 10000) if mid > 0 else 0
        
        reading = {
            "timestamp": datetime.utcnow().isoformat(),
            "obi": obi,
            "obi_normalized": obi_norm,
            "obi_velocity": obi_velocity,
            "pressure": pressure,
            "bid_depth": order_book_data.get("bid_depth", 0),
            "ask_depth": order_book_data.get("ask_depth", 0),
            "spread_bps": spread_bps,
            "signal": "accumulate" if pressure in ["buy", "strong_buy"] else "distribute" if pressure in ["sell", "strong_sell"] else "neutral"
        }
        
        self.pressure_readings.append(reading)
        return reading


class MomentumEngineCalculator:
    """
    Analyst 2: Momentum Engine
    Calculates HMA, slope, ROC, RSI
    
    FREE DATA SOURCE: Binance WebSocket (kline_1m) + REST API
    """
    
    def __init__(self, hma_period: int = 14):
        self.hma_period = hma_period
        self.price_history: deque = deque(maxlen=200)
        self.hma_history: deque = deque(maxlen=100)
        
    def _wma(self, prices: List[float], period: int) -> float:
        """Weighted Moving Average"""
        weights = list(range(1, period + 1))
        return sum(p * w for p, w in zip(prices, weights)) / sum(weights)
    
    def _hma(self, prices: List[float]) -> float:
        """Hull Moving Average - reduces lag"""
        half = self.hma_period // 2
        sqrt_p = int(self.hma_period ** 0.5)
        
        wma_half = self._wma(prices[-half:], half)
        wma_full = self._wma(prices, self.hma_period)
        
        raw_hma = 2 * wma_half - wma_full
        return raw_hma
    
    def calculate(self, candles: List[CandleData]) -> Dict:
        """Calculate momentum indicators"""
        if len(candles) < self.hma_period:
            return {"error": f"Need {self.hma_period} candles, have {len(candles)}"}
        
        prices = [c.close for c in candles]
        
        # HMA
        hma = self._hma(prices)
        self.hma_history.append(hma)
        
        # HMA slope (degrees)
        hma_slope = 0
        if len(self.hma_history) >= 2:
            prev_hma = list(self.hma_history)[-2]
            if prev_hma != 0:
                pct_change = (hma - prev_hma) / prev_hma
                hma_slope = pct_change * 4500  # Rough degree conversion
                hma_slope = max(-90, min(90, hma_slope))
        
        # ROC (3-period)
        roc = ((prices[-1] - prices[-4]) / prices[-4] * 100) if len(prices) >= 4 else 0
        
        # Simple RSI
        rsi = self._calculate_rsi(prices[-15:])
        
        return {
            "hma": round(hma, 2),
            "hma_slope": round(hma_slope, 2),
            "roc_3m": round(roc, 4),
            "rsi": round(rsi, 2) if rsi else None,
            "current_price": prices[-1],
            "hma_trend": "up" if hma_slope > 15 else "down" if hma_slope < -15 else "flat"
        }
    
    def _calculate_rsi(self, prices: List[float]) -> Optional[float]:
        """Calculate RSI for given prices"""
        if len(prices) < 14:
            return None
        
        gains = []
        losses = []
        
        for i in range(1, len(prices)):
            change = prices[i] - prices[i-1]
            gains.append(max(change, 0))
            losses.append(abs(min(change, 0)))
        
        avg_gain = sum(gains) / len(gains)
        avg_loss = sum(losses) / len(losses)
        
        if avg_loss == 0:
            return 100.0
        
        rs = avg_gain / avg_loss
        return 100 - (100 / (1 + rs))


class MicrostructureCalculator:
    """
    Analyst 3: Microstructure Engine
    Calculates VPIN, volatility, liquidity voids
    
    FREE DATA SOURCE: Binance WebSocket (trade + depth)
    """
    
    def __init__(self, lookback: int = 50):
        self.lookback = lookback
        self.trade_history: deque = deque(maxlen=500)
        self.volatility_window: deque = deque(maxlen=100)
        
    def add_trade(self, tick: TickData):
        """Add trade tick for microstructure analysis"""
        self.trade_history.append(tick)
        
    def calculate(self, current_price: float) -> Dict:
        """
        Calculate microstructure metrics
        """
        trades = list(self.trade_history)
        if len(trades) < 20:
            return {"error": f"Need 20+ trades, have {len(trades)}"}
        
        # Volume Partitioning (simplified VPIN)
        buy_volume = sum(t.quantity for t in trades if not t.is_buyer_maker)
        sell_volume = sum(t.quantity for t in trades if t.is_buyer_maker)
        total_volume = buy_volume + sell_volume
        
        vpin = abs(buy_volume - sell_volume) / total_volume if total_volume > 0 else 0
        
        # Realized volatility (recent trades)
        prices = [t.price for t in trades[-self.lookback:]]
        returns = [(prices[i] - prices[i-1]) / prices[i-1] for i in range(1, len(prices))]
        
        if len(returns) > 1:
            mean_ret = sum(returns) / len(returns)
            variance = sum((r - mean_ret) ** 2 for r in returns) / len(returns)
            volatility = (variance ** 0.5) * 100  # As percentage
        else:
            volatility = 0
        
        self.volatility_window.append(volatility)
        
        # Volatility regime
        avg_vol = sum(self.volatility_window) / len(self.volatility_window) if self.volatility_window else 0
        
        if volatility > avg_vol * 1.5:
            vol_regime = "expanding"
        elif volatility < avg_vol * 0.5:
            vol_regime = "compressing"
        else:
            vol_regime = "normal"
        
        # Toxicity signal
        toxicity = "elevated" if vpin > 0.7 else "normal"
        
        return {
            "vpin": round(vpin, 3),
            "volatility": round(volatility, 4),
            "volatility_regime": vol_regime,
            "buy_sell_imbalance": round((buy_volume - sell_volume) / total_volume, 3) if total_volume else 0,
            "toxicity": toxicity,
            "trade_count": len(trades),
            "volume_analyzed": round(total_volume, 4)
        }


# =============================================================================
# ORCHESTRATOR
# =============================================================================

class LayerAOrchestrator:
    """
    Orchestrates all Layer A data collection and analysis
    """
    
    def __init__(self):
        self.collector = BinanceWebSocketCollector("btcusdt")
        
        # Independent calculators
        self.tape_reader = TapeReaderCalculator()
        self.momentum_engine = MomentumEngineCalculator(hma_period=14)
        self.microstructure = MicrostructureCalculator()
        
        # Latest readings
        self.latest_readings: Dict = {}
        
    async def start(self):
        """Start the data collection and analysis loop"""
        
        # Set up callbacks
        self.collector.on_trade = self._on_trade
        self.collector.on_orderbook_update = self._on_orderbook
        self.collector.on_candle = self._on_candle
        
        # Start collection
        await self.collector.connect()
        
    async def _on_trade(self, tick: TickData):
        """Process new trade"""
        self.microstructure.add_trade(tick)
        
        # Calculate microstructure on every Nth trade
        if len(self.microstructure.trade_history) % 10 == 0:
            micro = self.microstructure.calculate(tick.price)
            self.latest_readings["microstructure"] = micro
            
    async def _on_orderbook(self, ob: OrderBookSnapshot):
        """Process order book update"""
        ob_data = self.collector.get_order_book_imbalance(depth_levels=10)
        tape = self.tape_reader.calculate(ob_data)
        self.latest_readings["tape_reader"] = tape
        
    async def _on_candle(self, candle: CandleData):
        """Process completed candle"""
        candles = self.collector.get_candle_history(50)
        if len(candles) >= 14:
            momentum = self.momentum_engine.calculate(candles)
            self.latest_readings["momentum"] = momentum
            
        # Print summary on candle close
        await self._print_summary()
        
    async def _print_summary(self):
        """Print current analysis summary"""
        print("\n" + "="*60)
        print(f"[Layer A] Analysis Summary - {datetime.utcnow().strftime('%H:%M:%S')} UTC")
        print("="*60)
        
        if "tape_reader" in self.latest_readings:
            tape = self.latest_readings["tape_reader"]
            print(f"\n📊 TAPE READER:")
            print(f"   OBI: {tape['obi']:.3f} ({tape['pressure']})")
            print(f"   OBI Velocity: {tape['obi_velocity']:.4f}")
            print(f"   Spread: {tape['spread_bps']:.2f} bps")
            print(f"   Signal: {tape['signal'].upper()}")
            
        if "momentum" in self.latest_readings:
            mom = self.latest_readings["momentum"]
            print(f"\n📈 MOMENTUM ENGINE:")
            print(f"   HMA: {mom['hma']:,.2f}")
            print(f"   HMA Slope: {mom['hma_slope']:.1f}°")
            print(f"   ROC(3m): {mom['roc_3m']:.3f}%")
            print(f"   RSI: {mom['rsi']}")
            print(f"   Trend: {mom['hma_trend'].upper()}")
            
        if "microstructure" in self.latest_readings:
            micro = self.latest_readings["microstructure"]
            print(f"\n🔬 MICROSTRUCTURE:")
            print(f"   VPIN: {micro['vpin']:.3f}")
            print(f"   Volatility: {micro['volatility']:.4f}%")
            print(f"   Regime: {micro['volatility_regime'].upper()}")
            print(f"   Toxicity: {micro['toxicity'].upper()}")
            
    def stop(self):
        """Stop all collection"""
        self.collector.stop()


# =============================================================================
# CLI Entry Point
# =============================================================================

async def main():
    """Run Layer A collector"""
    print("="*60)
    print("LLMHQ Layer A - Real-Time Sensory Array")
    print("="*60)
    print("\nStarting data collection from Binance...")
    print("Press Ctrl+C to stop\n")
    
    orchestrator = LayerAOrchestrator()
    
    try:
        await orchestrator.start()
    except KeyboardInterrupt:
        print("\n\nStopping...")
        orchestrator.stop()
        print(f"Stats: {orchestrator.collector.get_stats()}")


if __name__ == "__main__":
    asyncio.run(main())
