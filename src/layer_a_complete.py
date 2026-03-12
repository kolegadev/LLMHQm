"""
LLMHQ Complete Layer A Orchestrator
Integrates all 8 analysts with configurable data sources
"""

import asyncio
import sys
sys.path.insert(0, '/root/.openclaw/workspace/LLMHQm-work/src')

from layer_a_collector import (
    BinanceWebSocketCollector, LayerAOrchestrator,
    TapeReaderCalculator, MomentumEngineCalculator, MicrostructureCalculator
)
from analysts_external import (
    WhaleWatcherCalculator, SocialSentimentCalculator,
    CrossExchangeCalculator, LiquidityMapCalculator, CorrelationCalculator,
    print_data_source_options
)

class CompleteLayerAOrchestrator:
    """
    Full Layer A with all 8 analysts
    """
    
    def __init__(self, config: dict = None):
        self.config = config or {}
        
        # Base Layer A (Binance WebSocket)
        self.base = LayerAOrchestrator()
        
        # Additional analysts
        self.whale_watcher = WhaleWatcherCalculator(
            data_source=self.config.get("whale_source", "coinglass")
        )
        self.sentiment = SocialSentimentCalculator(
            data_source=self.config.get("sentiment_source", "fear_greed")
        )
        self.cross_exchange = CrossExchangeCalculator()
        self.liquidity_map = LiquidityMapCalculator()
        self.correlation = CorrelationCalculator(
            correlation_assets=self.config.get("correlation_assets", ["ETH", "SOL"])
        )
        
        self.all_readings = {}
        
    async def start(self):
        """Start complete data collection"""
        print("="*70)
        print("LLMHQ Layer A - Complete 8-Analyst Sensory Array")
        print("="*70)
        
        # Print data source config
        print("\n📋 DATA SOURCE CONFIGURATION:")
        print(f"   Whale Watcher: {self.whale_watcher.data_source}")
        print(f"   Sentiment: {self.sentiment.data_source}")
        print(f"   Correlation assets: {self.correlation.correlation_assets}")
        
        # Override base callbacks
        self.base.collector.on_candle = self._on_candle_complete
        
        # Start base collection
        await self.base.start()
        
    async def _on_candle_complete(self, candle):
        """Extended candle handler with all analysts"""
        
        # Run base analysis
        candles = self.base.collector.get_candle_history(50)
        if len(candles) >= 14:
            momentum = self.base.momentum_engine.calculate(candles)
            self.all_readings["momentum"] = momentum
        
        # Get order book data
        ob_data = self.base.collector.get_order_book_imbalance(10)
        tape = self.base.tape_reader.calculate(ob_data)
        self.all_readings["tape_reader"] = tape
        
        # Microstructure
        micro = self.base.microstructure.calculate(candle.close)
        self.all_readings["microstructure"] = micro
        
        # Liquidity Map (uses order book)
        if self.base.collector.order_book:
            bids = [[b.price, b.quantity] for b in self.base.collector.order_book.bids[:20]]
            asks = [[a.price, a.quantity] for a in self.base.collector.order_book.asks[:20]]
            liquidity = self.liquidity_map.calculate(bids, asks, candle.close)
            self.all_readings["liquidity_map"] = liquidity
        
        # Cross-exchange (if perp feed available)
        self.cross_exchange.update_spot(candle.close)
        cross = self.cross_exchange.calculate()
        self.all_readings["cross_exchange"] = cross
        
        # Correlation (update BTC price)
        self.correlation.update_price("BTC", candle.close)
        corr = self.correlation.calculate()
        self.all_readings["correlation"] = corr
        
        # Sentiment (fear/greed) - polled less frequently
        sentiment = self.sentiment.calculate()
        self.all_readings["sentiment"] = sentiment
        
        # Whale watcher (placeholder until configured)
        whale = self.whale_watcher.calculate()
        self.all_readings["whale_watcher"] = whale
        
        # Print complete summary
        await self._print_complete_summary()
        
    async def _print_complete_summary(self):
        """Print all 8 analyst readings"""
        from datetime import datetime
        
        print("\n" + "="*70)
        print(f"[Layer A] Complete Analysis - {datetime.utcnow().strftime('%H:%M:%S')} UTC")
        print("="*70)
        
        # 1. Tape Reader
        if "tape_reader" in self.all_readings:
            t = self.all_readings["tape_reader"]
            print(f"\n1️⃣  TAPE READER (Order Book Pressure)")
            print(f"    OBI: {t['obi']:.3f} | Pressure: {t['pressure']} | Signal: {t['signal']}")
            print(f"    OBI Velocity: {t['obi_velocity']:.4f} | Spread: {t['spread_bps']:.1f} bps")
        
        # 2. Momentum Engine
        if "momentum" in self.all_readings:
            m = self.all_readings["momentum"]
            print(f"\n2️⃣  MOMENTUM ENGINE (HMA/ROC/RSI)")
            print(f"    HMA: {m['hma']:,.2f} | Slope: {m['hma_slope']:.1f}° | Trend: {m['hma_trend'].upper()}")
            print(f"    ROC: {m['roc_3m']:.3f}% | RSI: {m['rsi']}")
        
        # 3. Microstructure
        if "microstructure" in self.all_readings:
            ms = self.all_readings["microstructure"]
            print(f"\n3️⃣  MICROSTRUCTURE ENGINE (VPIN/Volatility)")
            print(f"    VPIN: {ms['vpin']:.3f} | Volatility: {ms['volatility']:.4f}%")
            print(f"    Regime: {ms['volatility_regime'].upper()} | Toxicity: {ms['toxicity'].upper()}")
        
        # 4. Whale Watcher
        if "whale_watcher" in self.all_readings:
            w = self.all_readings["whale_watcher"]
            print(f"\n4️⃣  WHALE WATCHER (Exchange Flows/Liquidations)")
            print(f"    Source: {w['data_source']}")
            print(f"    Status: {'⚠️ ' + w['note'] if w['signal'] == 'neutral' else '✅ Active'}")
        
        # 5. Sentiment
        if "sentiment" in self.all_readings:
            s = self.all_readings["sentiment"]
            print(f"\n5️⃣  SOCIAL SENTIMENT FILTER")
            if 'fear_greed_value' in s:
                print(f"    Fear & Greed: {s['fear_greed_value']}/100 ({s['classification']})")
                print(f"    Sentiment: {s['sentiment']} | Signal: {s['signal']}")
            else:
                print(f"    Status: {s['note']}")
        
        # 6. Cross-Exchange
        if "cross_exchange" in self.all_readings:
            cx = self.all_readings["cross_exchange"]
            print(f"\n6️⃣  CROSS-EXCHANGE MONITOR (Spot vs Perp)")
            if 'basis_bps' in cx:
                print(f"    Spot: ${cx['spot_price']:,.2f} | Perp: ${cx['perp_price']:,.2f}")
                print(f"    Basis: {cx['basis_bps']:+.1f} bps | Signal: {cx['signal']}")
            else:
                print(f"    Status: {cx['note']}")
        
        # 7. Liquidity Map
        if "liquidity_map" in self.all_readings:
            lm = self.all_readings["liquidity_map"]
            print(f"\n7️⃣  LIQUIDITY MAP (Voids/Walls)")
            print(f"    Signal: {lm['signal']}")
            if lm.get('near_bid_wall'):
                print(f"    Bid Wall: ${lm['bid_wall']['price']:,.2f} ({lm['bid_wall']['size']:.2f} BTC)")
            if lm.get('near_ask_wall'):
                print(f"    Ask Wall: ${lm['ask_wall']['price']:,.2f} ({lm['ask_wall']['size']:.2f} BTC)")
            voids = lm.get('voids', {})
            if voids.get('closest_resistance_void'):
                v = voids['closest_resistance_void']
                print(f"    Closest Void Above: +{v['distance_from_price']:.2f}%")
        
        # 8. Correlation
        if "correlation" in self.all_readings:
            c = self.all_readings["correlation"]
            print(f"\n8️⃣  CORRELATION CHECKER")
            if c.get('correlations'):
                for asset, corr in c['correlations'].items():
                    print(f"    BTC-{asset}: {corr:+.3f}")
                print(f"    Regime: {c['regime']} | Signal: {c['signal']}")
            else:
                print(f"    Status: {c['note']}")
        
        print("\n" + "-"*70)


async def main():
    """Run complete Layer A"""
    
    # Show data source options first
    print_data_source_options()
    
    print("\n" + "="*70)
    print("Starting Layer A with default configuration...")
    print("="*70 + "\n")
    
    config = {
        "whale_source": "coinglass",  # User can change this
        "sentiment_source": "fear_greed",
        "correlation_assets": ["ETH", "SOL"]
    }
    
    orchestrator = CompleteLayerAOrchestrator(config)
    
    try:
        await orchestrator.start()
    except KeyboardInterrupt:
        print("\n\nStopping...")
        orchestrator.base.stop()


if __name__ == "__main__":
    asyncio.run(main())
