# LLMHQ Database Configuration

## Database Setup Complete ✅

PostgreSQL database `llmhq` is configured and ready.

### Connection Details

```
Host: localhost
Port: 5432
Database: llmhq
Username: llmhq
Password: llmhq_pass
```

Connection URL:
```
postgres://llmhq:llmhq_pass@localhost:5432/llmhq
```

### Tables Created

| Table | Purpose |
|-------|---------|
| `market_ticks` | Raw trade data from exchanges |
| `order_book_snapshots` | L2 order book state |
| `analyst_readings` | All 8 analyst outputs |
| `narratives` | Layer B Markdown narratives |
| `cio_decisions` | Layer C decision outputs |
| `paper_trades` | Paper trading ledger |
| `ghost_trade_analysis` | Layer E post-mortem (future) |
| `performance_metrics` | Aggregated statistics |
| `policy_updates` | RLAIF policy changes (future) |

### Views Created

| View | Purpose |
|------|---------|
| `open_paper_trades` | Currently open positions |
| `recent_performance` | Daily P&L summary |
| `daily_summary` | Daily prediction summary |

### Functions Created

| Function | Purpose |
|----------|---------|
| `calculate_win_rate(start, end)` | Win rate over period |
| `resolve_trade(trade_id, exit_price)` | Close trade with outcome |
| `get_paper_balance(initial)` | Current paper balance |

### Quick Queries

```sql
-- View recent decisions
SELECT block_number, direction, confidence, veto_applied 
FROM cio_decisions 
ORDER BY time DESC 
LIMIT 10;

-- View open trades
SELECT * FROM open_paper_trades;

-- View recent performance
SELECT * FROM recent_performance;

-- View daily summary
SELECT * FROM daily_summary;

-- Calculate win rate for last 24 hours
SELECT calculate_win_rate(NOW() - INTERVAL '24 hours', NOW());

-- Get current paper balance
SELECT get_paper_balance(10000);
```

### Notes

- Standard PostgreSQL (TimescaleDB extension optional)
- All tables have proper indexes for performance
- User `llmhq` has full permissions on all objects
- UUID extension enabled for paper_trade IDs
