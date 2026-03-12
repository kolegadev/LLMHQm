"""
LLMHQ Feature Calculator
Computes technical indicators and microstructure features
"""

import math
from typing import List, Dict, Optional
from datetime import datetime

class FeatureCalculator:
    """Calculate trading indicators and features"""
    
    @staticmethod
    def wma(prices: List[float], period: int) -> Optional[float]:
        """Weighted Moving Average"""
        if len(prices) < period:
            return None
        weights = list(range(1, period + 1))
        recent = prices[-period:]
        return sum(p * w for p, w in zip(recent, weights)) / sum(weights)
    
    @staticmethod
    def hma(prices: List[float], period: int = 14) -> Optional[float]:
        """Hull Moving Average - reduces lag"""
        if len(prices) < period:
            return None
        
        # HMA = WMA(2*WMA(n/2) - WMA(n)), sqrt(n))
        half_period = period // 2
        sqrt_period = int(math.sqrt(period))
        
        wma_half = FeatureCalculator.wma(prices, half_period)
        wma_full = FeatureCalculator.wma(prices, period)
        
        if wma_half is None or wma_full is None:
            return None
        
        # Create intermediate series
        raw_hma = 2 * wma_half - wma_full
        # For proper HMA we'd need the series, approximating with current value
        return raw_hma
    
    @staticmethod
    def hma_slope(prices: List[float], period: int = 14) -> Optional[float]:
        """Calculate HMA slope in degrees"""
        if len(prices) < period + 2:
            return None
        
        hma_current = FeatureCalculator.hma(prices, period)
        hma_prev = FeatureCalculator.hma(prices[:-1], period)
        
        if hma_current is None or hma_prev is None or hma_prev == 0:
            return None
        
        # Calculate percentage change and convert to approximate degrees
        pct_change = (hma_current - hma_prev) / hma_prev
        # Rough conversion: 1% ≈ 45° for visualization
        slope_deg = pct_change * 4500
        return max(-90, min(90, slope_deg))  # Clamp to ±90°
    
    @staticmethod
    def roc(prices: List[float], period: int = 3) -> Optional[float]:
        """Rate of Change - velocity measure"""
        if len(prices) < period + 1:
            return None
        current = prices[-1]
        previous = prices[-(period + 1)]
        if previous == 0:
            return None
        return ((current - previous) / previous) * 100
    
    @staticmethod
    def rsi(prices: List[float], period: int = 14) -> Optional[float]:
        """Relative Strength Index"""
        if len(prices) < period + 1:
            return None
        
        gains = []
        losses = []
        
        for i in range(1, period + 1):
            change = prices[-i] - prices[-(i + 1)]
            if change > 0:
                gains.append(change)
                losses.append(0)
            else:
                gains.append(0)
                losses.append(abs(change))
        
        avg_gain = sum(gains) / period
        avg_loss = sum(losses) / period
        
        if avg_loss == 0:
            return 100.0
        
        rs = avg_gain / avg_loss
        rsi = 100 - (100 / (1 + rs))
        return rsi
    
    @staticmethod
    def volatility(prices: List[float], period: int = 14) -> Optional[float]:
        """Calculate price volatility (std dev of returns)"""
        if len(prices) < period + 1:
            return None
        
        returns = []
        for i in range(1, period + 1):
            ret = (prices[-i] - prices[-(i + 1)]) / prices[-(i + 1)]
            returns.append(ret)
        
        mean_ret = sum(returns) / len(returns)
        variance = sum((r - mean_ret) ** 2 for r in returns) / len(returns)
        return math.sqrt(variance) * 100  # As percentage
    
    @staticmethod
    def classify_regime(hma_slope: float, volatility: float, obi: float) -> str:
        """Classify market regime based on indicators"""
        
        # High volatility regime
        if volatility > 2.0:
            if abs(hma_slope) > 30:
                return "volatile_expansion"
            return "manipulative"
        
        # Quiet/compressed
        if volatility < 0.5 and abs(hma_slope) < 10:
            return "quiet_compression"
        
        # Trending
        if abs(hma_slope) > 20:
            return "trending"
        
        # Ranging/mean reversion
        return "ranging"
    
    @staticmethod
    def generate_narrative(
        price: float,
        hma_slope: Optional[float],
        obi: float,
        obi_prev: Optional[float],
        roc_val: Optional[float],
        regime: str,
        spread_bps: float
    ) -> str:
        """Generate semantic narrative for CIO"""
        
        parts = []
        
        # Price context
        parts.append(f"BTC at ${price:,.2f}.")
        
        # Momentum narrative
        if hma_slope is not None:
            if hma_slope > 30:
                parts.append(f"Strong upward momentum (HMA slope +{hma_slope:.1f}°).")
            elif hma_slope > 15:
                parts.append(f"Moderate upward momentum (HMA slope +{hma_slope:.1f}°).")
            elif hma_slope < -30:
                parts.append(f"Strong downward momentum (HMA slope {hma_slope:.1f}°).")
            elif hma_slope < -15:
                parts.append(f"Moderate downward momentum (HMA slope {hma_slope:.1f}°).")
            elif abs(hma_slope) < 5:
                parts.append(f"Flat momentum (HMA slope {hma_slope:.1f}°).")
        
        # OBI narrative
        obi_norm = (obi + 1) / 2  # Convert -1..1 to 0..1
        if obi_norm > 0.7:
            parts.append("Buy-side absorption dominant.")
        elif obi_norm > 0.6:
            parts.append("Buy-side pressure elevated.")
        elif obi_norm < 0.3:
            parts.append("Sell-side absorption dominant.")
        elif obi_norm < 0.4:
            parts.append("Sell-side pressure elevated.")
        else:
            parts.append("Order book relatively balanced.")
        
        # OBI velocity
        if obi_prev is not None:
            obi_change = obi_norm - ((obi_prev + 1) / 2)
            if obi_change > 0.1:
                parts.append("OBI accelerating bullish.")
            elif obi_change < -0.1:
                parts.append("OBI accelerating bearish.")
        
        # ROC/velocity
        if roc_val is not None:
            if roc_val > 0.5:
                parts.append(f"Price velocity positive ({roc_val:.2f}% over 3m).")
            elif roc_val < -0.5:
                parts.append(f"Price velocity negative ({roc_val:.2f}% over 3m).")
        
        # Regime
        if regime == "trending":
            parts.append("Market in trending regime.")
        elif regime == "ranging":
            parts.append("Market in ranging regime.")
        elif regime == "volatile_expansion":
            parts.append("High volatility expansion.")
        elif regime == "quiet_compression":
            parts.append("Quiet compression, potential breakout setup.")
        elif regime == "manipulative":
            parts.append("Elevated noise, possible manipulation.")
        
        # Spread context
        if spread_bps > 10:
            parts.append("Wide spreads, lower liquidity.")
        
        return " ".join(parts)

if __name__ == "__main__":
    # Test with sample data
    test_prices = [82000, 82150, 82300, 82200, 82400, 82600, 82500, 82700, 82900, 82800, 
                   83000, 83200, 83100, 83300, 83500, 83400, 83600, 83800, 83700, 83900]
    
    calc = FeatureCalculator()
    
    print("=== LLMHQ Feature Calculator Test ===")
    print(f"Test prices: {test_prices[-5:]}")
    
    hma = calc.hma(test_prices, 14)
    slope = calc.hma_slope(test_prices, 14)
    roc = calc.roc(test_prices, 3)
    rsi = calc.rsi(test_prices, 14)
    vol = calc.volatility(test_prices, 14)
    regime = calc.classify_regime(slope or 0, vol or 1.0, 0.3)
    
    print(f"\nIndicators:")
    print(f"  HMA(14): {hma:,.2f}" if hma else "  HMA(14): N/A")
    print(f"  HMA Slope: {slope:.1f}°" if slope else "  HMA Slope: N/A")
    print(f"  ROC(3): {roc:.2f}%" if roc else "  ROC(3): N/A")
    print(f"  RSI(14): {rsi:.1f}" if rsi else "  RSI(14): N/A")
    print(f"  Volatility: {vol:.2f}%" if vol else "  Volatility: N/A")
    print(f"  Regime: {regime}")
    
    narrative = calc.generate_narrative(
        price=test_prices[-1],
        hma_slope=slope,
        obi=0.3,
        obi_prev=0.2,
        roc_val=roc,
        regime=regime,
        spread_bps=5
    )
    print(f"\nNarrative:\n  {narrative}")
