# LLMHQ Deployment Guide

## Overview

This guide covers deploying LLMHQ in production using Docker Compose.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    DOCKER COMPOSE STACK                          │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │   LLMHQ     │  │ PostgreSQL  │  │ Prometheus  │             │
│  │   Engine    │  │   (DB)      │  │  (Metrics)  │             │
│  │   :9090     │  │   :5432     │  │   :9091     │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                     │
│         └────────────────┴────────────────┘                     │
│                          │                                      │
│                   ┌──────┴──────┐                              │
│                   │   Grafana   │                              │
│                   │   :3000     │                              │
│                   └─────────────┘                              │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

```bash
# 1. Clone repository
git clone https://github.com/kolegadev/LLMHQm.git
cd LLMHQm

# 2. Set environment variables
cp rust/.env.example rust/.env
# Edit rust/.env with your configuration

# 3. Start all services
docker-compose up -d

# 4. Check logs
docker-compose logs -f llmhq

# 5. Access Grafana dashboard
open http://localhost:3000
# Login: admin / admin
```

## Configuration

### Environment Variables (.env)

```bash
# Database
DATABASE_URL=postgres://llmhq:llmhq_pass@postgres:5432/llmhq

# Polymarket - BTC 5M Market
# The timestamp auto-increments by 300 (5 minutes) each block
POLYMARKET_MARKET_BASE=btc-updown-5m
POLYMARKET_MARKET_TIMESTAMP=1773358800
POLYMARKET_BLOCK_DURATION=300

# Trading
INITIAL_BALANCE=10000
MIN_CONFIDENCE=65
MAX_POSITION_PCT=95
YES_ODDS_THRESHOLD=0.505
BLOCK_INTERVAL_MINUTES=5

# Logging
RUST_LOG=info,llmhq=debug
```

### Polymarket Market ID Explained

The market URL is: `https://polymarket.com/event/btc-updown-5m-1773358800`

- `btc-updown-5m`: Market type (BTC up/down, 5-minute)
- `1773358800`: Unix timestamp of block start
- Each new 5-minute block: timestamp += 300

The system automatically handles market rotation.

## Services

### 1. LLMHQ Engine
- **Image**: Built from `./rust/Dockerfile`
- **Port**: 9090 (metrics)
- **Connects to**: PostgreSQL, Binance WebSocket, Polymarket API
- **Logs**: `docker-compose logs -f llmhq`

### 2. PostgreSQL
- **Image**: `postgres:15-alpine`
- **Port**: 5432
- **Data**: Persisted in `postgres_data` volume
- **Init**: Runs migrations from `./rust/migrations/`

### 3. Prometheus
- **Image**: `prom/prometheus:latest`
- **Port**: 9091
- **Scrapes**: LLMHQ metrics (:9090), PostgreSQL (:9187)
- **Config**: `./monitoring/prometheus/prometheus.yml`

### 4. Grafana
- **Image**: `grafana/grafana:latest`
- **Port**: 3000
- **Dashboards**: Pre-loaded from `./monitoring/grafana/dashboards/`
- **Login**: admin / admin

### 5. PostgreSQL Exporter
- **Image**: `prometheuscommunity/postgres-exporter`
- **Port**: 9187
- **Provides**: Database metrics to Prometheus

## Monitoring

### Accessing Dashboards

| Service | URL | Credentials |
|---------|-----|-------------|
| Grafana | http://localhost:3000 | admin/admin |
| Prometheus | http://localhost:9091 | - |

### Key Metrics

The Grafana dashboard shows:
- **Current BTC Price**: Live from Binance
- **HMA Slope**: Momentum indicator (-90° to +90°)
- **OBI**: Order book imbalance (0-100% buy-side)
- **VPIN**: Volume-synchronized PIN (0-1)
- **Recent Trades**: Table of executed trades
- **P&L Over Time**: Cumulative profit/loss
- **Win Rate**: Percentage of winning trades
- **Basis**: Spot-perp spread
- **Liquidations**: 1-minute liquidation volume

### Database Queries

Access PostgreSQL:
```bash
docker-compose exec postgres psql -U llmhq -d llmhq
```

Useful queries:
```sql
-- Recent decisions
SELECT block_number, direction, confidence, veto_applied 
FROM cio_decisions ORDER BY time DESC LIMIT 10;

-- Open trades
SELECT * FROM open_paper_trades;

-- Win rate
SELECT calculate_win_rate(NOW() - INTERVAL '24 hours', NOW());

-- Daily performance
SELECT * FROM recent_performance;
```

## Maintenance

### Update System

```bash
# Pull latest code
git pull origin main

# Rebuild and restart
docker-compose up -d --build
```

### Backup Database

```bash
# Create backup
docker-compose exec postgres pg_dump -U llmhq llmhq > backup.sql

# Restore backup
cat backup.sql | docker-compose exec -T postgres psql -U llmhq -d llmhq
```

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f llmhq

# Last 100 lines
docker-compose logs --tail=100 llmhq
```

### Restart Service

```bash
# Restart LLMHQ
docker-compose restart llmhq

# Restart everything
docker-compose restart
```

## Troubleshooting

### Issue: LLMHQ can't connect to PostgreSQL

**Symptoms**: Logs show connection errors

**Fix**:
```bash
# Check PostgreSQL is healthy
docker-compose ps

# Restart PostgreSQL
docker-compose restart postgres

# Wait for health check, then restart LLMHQ
sleep 10
docker-compose restart llmhq
```

### Issue: No data in Grafana

**Symptoms**: Empty dashboard

**Fix**:
1. Check Prometheus targets: http://localhost:9091/targets
2. Verify LLMHQ is exporting metrics: `curl http://localhost:9090/metrics`
3. Check Grafana data source configuration

### Issue: WebSocket connection failures

**Symptoms**: "WebSocket error" in logs

**Fix**:
- Check internet connectivity
- Binance may rate-limit; restart after 1 minute
- Verify no firewall blocking outbound connections

## Production Considerations

### Security

1. **Change default passwords**:
   ```bash
   # PostgreSQL
   docker-compose exec postgres psql -U llmhq -c "ALTER USER llmhq WITH PASSWORD 'new_secure_password';"
   
   # Grafana
   # Login to web UI and change password
   ```

2. **Use secrets management** for production:
   - Docker Secrets
   - HashiCorp Vault
   - AWS Secrets Manager

3. **Enable SSL**:
   - Use reverse proxy (nginx/traefik)
   - TLS certificates (Let's Encrypt)

### Performance

1. **Resource limits** (docker-compose.yml):
   ```yaml
   services:
     llmhq:
       deploy:
         resources:
           limits:
             cpus: '2.0'
             memory: 2G
   ```

2. **Database tuning**:
   - Increase shared_buffers
   - Enable connection pooling (PgBouncer)

### Monitoring & Alerting

1. **Add AlertManager** to Prometheus config
2. **Set up notifications** (Slack, PagerDuty, email)
3. **Key alerts**:
   - LLMHQ process down
   - Database connection lost
   - Win rate drops below threshold
   - No trades for >1 hour

## Upgrading

### Database Migrations

When schema changes:
```bash
# Run migrations
docker-compose exec llmhq ./llmhq migrate

# Or manually:
docker-compose exec postgres psql -U llmhq -d llmhq -f /migrations/002_update.sql
```

### Version Compatibility

Check compatibility matrix:
| LLMHQ Version | PostgreSQL | Grafana |
|---------------|------------|---------|
| 0.1.x | 15+ | 9+ |
| 0.2.x | 15+ | 10+ |

## Uninstall

```bash
# Stop all services
docker-compose down

# Remove volumes (DELETES ALL DATA)
docker-compose down -v

# Remove images
docker-compose down --rmi all
```

## Support

- **Issues**: https://github.com/kolegadev/LLMHQm/issues
- **Documentation**: https://github.com/kolegadev/LLMHQm/docs

---

**Ready to deploy?** Run `docker-compose up -d` and visit http://localhost:3000
