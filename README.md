# Radu Trading System

A high-performance trading system built in Rust.

## Architecture

```
Rust Backend
├── PostgreSQL (Railway) — trades, market data, model configs
├── Redis (Railway)      — caching, real-time data
├── Quant APIs           — quantitative trading strategies
└── Lambda Labs          — GPU model training (Phase 3)
```

## Quick Start

### Prerequisites
- Rust (installed via `rustup`)
- Railway account with PostgreSQL and Redis

### Setup

1. Copy environment template:
   ```powershell
   copy .env.example .env
   ```

2. Fill in your Railway credentials in `.env`:
   - Go to [railway.app](https://railway.app) → your project
   - PostgreSQL plugin → Connect → copy `DATABASE_URL`
   - Redis plugin → Connect → copy `REDIS_URL`

3. Run:
   ```powershell
   cargo run
   ```

## Project Structure

```
src/
├── main.rs     — Entry point, startup sequence
├── config.rs   — Environment variable loading
├── db.rs       — PostgreSQL connection & schema
├── cache.rs    — Redis connection & caching
├── models.rs   — Data structures (Trade, MarketData, etc.)
└── quant.rs    — Quant API client
```

## Roadmap

- [x] Phase 1: Core infrastructure (Postgres, Redis, config)
- [ ] Phase 2: Trading data models & Quant API integration
- [ ] Phase 3: Lambda Labs model training pipeline
