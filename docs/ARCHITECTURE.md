# Architecture Reference

This document describes the dependency boundaries, design decisions, versioning
policy, and publishing strategy for the `djradu` telemetry framework.

---

## Repository Purpose

**`djradu` is the telemetry framework repo.** It is not the trading runtime.

| Repo | Role |
|---|---|
| `djradu` (this repo) | Telemetry library + companion CLIs |
| `radu-trading-system` | Live trading runtime (PostgreSQL, Redis, Quant APIs) |

The trading runtime **depends on** the telemetry library. The telemetry library
has **no knowledge** of the trading runtime.

---

## Crate Dependency Boundaries

```
┌───────────────────────────────────────────────────────────────────┐
│  FRAMEWORK REPO (djradu)                                          │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  telemetry-core                                             │  │
│  │  ─────────────                                              │  │
│  │  • Zero runtime deps (no postgres, no redis, no HTTP)       │  │
│  │  • Only: serde, serde_json, chrono, uuid, anyhow            │  │
│  │  • Defines the immutable telemetry CONTRACT                  │  │
│  │  • Publishable to crates.io                                 │  │
│  └─────────────────────────────────────────────────────────────┘  │
│          ▲                         ▲                              │
│          │                         │                              │
│  ┌───────┴──────┐         ┌────────┴────────┐                    │
│  │ telemetry-sim│         │ telemetry-viewer│                    │
│  │ ─────────────│         │ ────────────────│                    │
│  │ + rand       │         │ (no extra deps) │                    │
│  │ CLI binary   │         │ CLI binary      │                    │
│  └──────────────┘         └─────────────────┘                    │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
          ▲
          │  git dep or crates.io dep
          │
┌─────────┴─────────────────────────────────────────────────────────┐
│  APPLICATION REPO (radu-trading-system, separate)                  │
│                                                                   │
│  src/
│  ├── main.rs         ← wires telemetry-core into startup          │
│  ├── config.rs       ← env vars (DATABASE_URL, REDIS_URL, …)     │
│  ├── db.rs           ← sqlx PostgreSQL pool                       │
│  ├── cache.rs        ← redis connection                           │
│  ├── models.rs       ← domain structs (Trade, MarketData, …)     │
│  └── quant.rs        ← Quant API HTTP client                      │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

**Hard rule:** `telemetry-core` must never import from `sqlx`, `redis`,
`reqwest`, `tokio`, or any other runtime/IO crate. Its compile time should
stay under 5 seconds even on a cold cache.

---

## Event Contract

### Envelope

Every event is wrapped in `TelemetryEnvelope`:

```json
{
  "schema_version": "0.1.0",
  "created_at": "2026-04-09T18:00:00Z",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamps": {
    "ts_local_rx": "2026-04-09T18:00:00Z"
  },
  "event": {
    "type": "market.l1",
    "symbol": "ES",
    "best_bid": { "price": 5400.00, "size": 150.0 },
    "best_ask": { "price": 5400.25, "size": 120.0 },
    "mid_price": 5400.125,
    "spread": 0.25
  }
}
```

### Validation Rules (enforced in `validate.rs`)

| Check | Rule |
|---|---|
| Schema version | Must match `SCHEMA_VERSION` constant |
| Timestamps | At least one `ts_*` field must be set |
| Market L1 | `bid < ask`, both `> 0`, symbol non-empty |
| Market L2 | At least 1 bid level and 1 ask level |
| Strategy decision | `desired_quantity > 0`, `decision_price > 0` |
| Order submitted | `quantity > 0` |
| Trade fill | `fill_quantity > 0`, `fill_price > 0`, `cumulative_quantity >= fill_quantity` |

Events that fail validation are **rejected** (not written) by `JsonlWriter`
and counted in `events_rejected()`.

---

## KPIs Computed by `telemetry-viewer`

| KPI | Formula |
|---|---|
| Slippage vs decision | `avg_fill_price − decision_mid` |
| Fill ratio | `cumulative_qty / desired_qty × 100%` |
| Time-to-ack | `ts_ack_rx − ts_order_tx` (μs) |
| Time-to-first-fill | `ts_fill_rx (first) − ts_order_tx` (μs) |
| Time-to-complete | `ts_fill_rx (final full fill) − ts_order_tx` (μs/ms) |
| Total fees | `Σ fill.fee` |

---

## Versioning

This project follows [Semantic Versioning](https://semver.org/).

### Schema Version (`SCHEMA_VERSION` in `events.rs`)

The schema version is embedded in every envelope and checked by the validator.
Consumers that read old JSONL files need to handle version mismatches.

| Bump | When |
|---|---|
| Patch (`0.1.x`) | Bug fixes to validation logic, no contract change |
| Minor (`0.x.0`) | New optional event fields, new event variants |
| Major (`x.0.0`) | Remove/rename fields, breaking wire format changes |

### Crate Version (`Cargo.toml`)

The crate version follows the same semver policy. A `v1.0.0` tag signals
a stable, production-ready contract.

---

## Publishing Strategy

### Phase 1 — Git dependency (current)

```toml
telemetry-core = { git = "https://github.com/radubadila14-cyber/djradu", tag = "v0.1.0" }
```

Pin a tag, not a branch, to prevent surprise breaking changes.

### Phase 2 — crates.io

When the contract stabilises at v1.0.0:

```bash
# From repo root
cargo publish -p telemetry-core
```

`telemetry-sim` and `telemetry-viewer` are development/diagnostic tools and
do not need to be published to crates.io (they are workspace binaries, not
reusable library crates).

### Changelog

Maintain a `CHANGELOG.md` at repo root using
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format. Tag every
release with `git tag v0.x.y` before publishing.

---

## CI/CD Pipeline

`.github/workflows/blank.yml` runs on every push and PR to `main`:

```
fmt check → clippy (-D warnings) → cargo test
```

Scope is intentionally limited to the three telemetry crates. The root
`radu-trading-system` app crate requires live infrastructure (PostgreSQL,
Redis) to run and is tested separately in its own repo/pipeline.

---

## Adding New Event Types

1. Add the struct to `crates/telemetry-core/src/events.rs`.
2. Add a variant to `TelemetryEvent` enum (pick a `serde` rename matching
   the naming convention: `domain.action` in lower case).
3. Add validation logic in `validate.rs` (match arm + helper function).
4. Add a unit test in `validate.rs` `#[cfg(test)]` block.
5. Bump `SCHEMA_VERSION` minor if it's a new variant.
6. Update `telemetry-viewer` to extract and report any new KPIs.

---

## Golden Trace

The golden trace (`telemetry/golden_trace.jsonl`) is the canonical
reference dataset for the telemetry contract. It represents a complete,
deterministic ES futures scenario:

```
100 L1 updates + 5 L2 snapshots
→ 1 strategy decision (BUY 5 ES, confidence 0.73)
→ 1 risk check (PASS, 3 sub-checks)
→ 1 order submitted (Limit IOC)
→ 1 ack (850 μs)
→ 1 partial fill (3 contracts)
→ 1 full fill (2 contracts)
→ 5 latency breakdown events
                                 Total: 116 events, 0 rejected
```

Any consumer that reads this file and produces different KPIs than those
shown by `telemetry-viewer` has a bug.
