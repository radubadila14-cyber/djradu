# djradu — Rust CME-Style Trading Telemetry Framework

[![CI](https://github.com/radubadila14-cyber/djradu/actions/workflows/blank.yml/badge.svg)](https://github.com/radubadila14-cyber/djradu/actions/workflows/blank.yml)

A **professional-grade, Rust-only** telemetry framework for CME/FIX-style trading systems.
This repo is the **telemetry library** — it defines the contract, validates events, and ships
a golden-trace simulator and KPI viewer as companion CLI tools.

---

## Architecture Decision — Library-First Workspace

**Chosen setup: Library-first workspace (not a monorepo)**

| Option | Pros | Cons |
|---|---|---|
| **Library workspace** ✅ | Clear SRP — one repo = one concern; publishable to crates.io; any app depends on it as a cargo dep | App lives in a separate repo |
| Monorepo | Everything in one place | Telemetry changes force app rebuilds; harder to publish the library independently |

**Rationale:**
- `telemetry-core` is a reusable contract crate. Any trading app (`radu-trading-system`, or any
  future system) should add it as `telemetry-core = { git = "...", tag = "v0.1.0" }`. That is
  the standard Rust/crates.io pattern for shared libraries.
- Keeping the app logic separate prevents database/runtime dependencies from leaking into the
  telemetry crate's dependency graph, which keeps compile times low and the API surface stable.
- The workspace also contains `telemetry-sim` (golden-trace generator) and `telemetry-viewer`
  (KPI computation CLI) as sibling crates — both depend on `telemetry-core`, not on each other.

---

## Workspace Layout

```
djradu/
├── Cargo.toml                   ← workspace root + radu-trading-system app crate
├── src/                         ← integration example (runtime app, uses pg/redis)
│   ├── main.rs
│   ├── config.rs
│   ├── db.rs
│   ├── cache.rs
│   ├── models.rs
│   └── quant.rs
│
├── crates/
│   ├── telemetry-core/          ← THE LIBRARY — publishable, no runtime deps
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ids.rs           ← TraceId, DecisionId, ClientOrderId, OrderId, ExecId
│   │       ├── timestamps.rs    ← EventTimestamps (multi-clock model)
│   │       ├── events.rs        ← TelemetryEvent enum + all event structs (THE CONTRACT)
│   │       ├── validate.rs      ← deterministic validation (3 unit tests)
│   │       └── writer.rs        ← JSONL writer (validate-before-write)
│   │
│   ├── telemetry-sim/           ← CLI: generate deterministic ES golden trace
│   │   └── src/main.rs          ← cargo run -p telemetry-sim
│   │
│   └── telemetry-viewer/        ← CLI: read JSONL and compute KPIs
│       └── src/main.rs          ← cargo run -p telemetry-viewer
│
├── telemetry/
│   ├── spec.rs                  ← contract reference document
│   └── golden_trace.jsonl       ← output of telemetry-sim (git-ignored in production)
│
└── docs/
    └── ARCHITECTURE.md          ← full architecture, boundaries, publishing guide
```

**Dependency graph (no cycles, strict boundaries):**

```
telemetry-sim   ──depends-on──►  telemetry-core
telemetry-viewer ─depends-on──►  telemetry-core
radu-trading-system (external) ── git dep ──► telemetry-core
```

---

## Quick Start

### Prerequisites

- Rust stable ≥ 1.85 (2024 edition)

```bash
rustup update stable
```

### Generate a golden trace

```bash
cargo run -p telemetry-sim
# writes → telemetry/golden_trace.jsonl
```

### Compute KPIs from the trace

```bash
cargo run -p telemetry-viewer
# or point at a custom file:
cargo run -p telemetry-viewer -- path/to/trace.jsonl
```

**Sample KPI output:**

```
╔══════════════════════════════════════════════════╗
║  FILL QUALITY KPIs                               ║
╠══════════════════════════════════════════════════╣
║  Decision mid:       5407.3750                   ║
║  Avg fill price:     5407.6250                   ║
║  Slippage:             +0.2500 pts               ║
║  Fill ratio:            100.0%                   ║
║  Total fees:            5.9000 USD               ║
╠══════════════════════════════════════════════════╣
║  LATENCY KPIs (microseconds)                     ║
╠══════════════════════════════════════════════════╣
║  Time-to-ack:            850 μs                  ║
║  Time-to-first-fill:    1050 μs                  ║
║  Time-to-complete:     16050 μs (16.1ms)         ║
╚══════════════════════════════════════════════════╝
```

### Run tests / CI checks locally

```bash
cargo fmt --all -- --check
cargo clippy -p telemetry-core -p telemetry-sim -p telemetry-viewer -- -D warnings
cargo test -p telemetry-core -p telemetry-sim -p telemetry-viewer
```

---

## Telemetry Contract (CME/FIX Semantics)

The contract is defined as Rust types in `crates/telemetry-core/src/events.rs`.

### Correlation IDs

| ID | Purpose |
|---|---|
| `TraceId` | End-to-end trace (market snapshot → decision → fills → PnL) |
| `DecisionId` | A single strategy decision |
| `ClientOrderId` | Survives cancel/replace |
| `OrderId` | Exchange-assigned (assigned on ack) |
| `ExecId` | Unique per fill/execution report |

### Event Lifecycle (FIX-style)

```
market.l1 / market.l2          ← market data feed
    │
    ▼
strategy.decision               ← model commits to a direction + price
    │
    ▼
risk.check                      ← pre-trade risk gate (pass/fail with details)
    │
    ▼
order.submitted                 ← order sent to venue (ClOrdID assigned)
    │
    ▼
order.ack / order.rejected      ← venue response (OrderID assigned on ack)
    │
    ▼
trade.fill (partial)            ← partial fill (CumQty < OrdQty)
trade.fill (full)               ← final fill (CumQty == OrdQty, LeavesQty == 0)
    │
    ▼
system.latency                  ← latency breakdown segments
```

### Multi-Timestamp Model

Every event carries `EventTimestamps` with optional fields for each clock:

| Field | What it captures |
|---|---|
| `ts_exchange` | Exchange-reported timestamp |
| `ts_local_rx` | Local machine receipt time |
| `ts_decision` | When the strategy committed |
| `ts_order_tx` | When the order was transmitted |
| `ts_ack_rx` | When the ack was received |
| `ts_fill_rx` | When the fill was received |

---

## Consuming `telemetry-core` as a Dependency

### From a private git repo

```toml
# In your Cargo.toml
[dependencies]
telemetry-core = { git = "https://github.com/radubadila14-cyber/djradu", tag = "v0.1.0" }
```

### From crates.io (future, once published)

```toml
[dependencies]
telemetry-core = "0.1"
```

### Integration example

```rust
use telemetry_core::events::{
    TelemetryEnvelope, TelemetryEvent, MarketL1Event, PriceLevel,
};
use telemetry_core::ids::{TraceId, SymbolId};
use telemetry_core::timestamps::EventTimestamps;
use telemetry_core::writer::JsonlWriter;

let trace_id = TraceId::new();
let mut writer = JsonlWriter::new(std::path::Path::new("events.jsonl"))?;

let envelope = TelemetryEnvelope::new(
    trace_id,
    EventTimestamps::local_now(),
    TelemetryEvent::MarketL1(MarketL1Event {
        symbol: SymbolId("ES".into()),
        best_bid: PriceLevel { price: 5400.00, size: 150.0 },
        best_ask: PriceLevel { price: 5400.25, size: 120.0 },
        mid_price: 5400.125,
        spread: 0.25,
        last_trade_price: None,
        last_trade_size: None,
    }),
);

writer.write(&envelope)?; // validates before writing
writer.flush()?;
```

---

## Versioning & Publishing Strategy

| Tag | Meaning |
|---|---|
| `v0.x.y` | Pre-stable — contract may change; semver patch for bug-fixes, minor for new fields |
| `v1.0.0` | Stable — full semver; breaking changes only on major bumps |

**Schema evolution rules:**
- Adding optional fields to an event struct → patch bump (backward compatible)
- Adding a new `TelemetryEvent` variant → minor bump
- Removing or renaming a field → major bump + migration guide

**Publishing to crates.io:**

```bash
# When ready
cargo publish -p telemetry-core
```

---

## Integration with `radu-trading-system`

Your separate runtime app (`radu-trading-system`) connects to Railway (PostgreSQL + Redis)
and executes live orders. To wire telemetry into it:

1. Add `telemetry-core` as a git dependency (see above).
2. Create a `JsonlWriter` at startup, pointing to your log path.
3. Emit events at every system boundary:
   - Market data handler → `TelemetryEvent::MarketL1`
   - Strategy callback → `TelemetryEvent::StrategyDecision`
   - Order send path → `TelemetryEvent::OrderSubmitted`
   - FIX/exchange callback → `TelemetryEvent::OrderAck`, `TelemetryEvent::TradeFill`
4. Run `telemetry-viewer` against the live log to see real KPIs.

See `docs/ARCHITECTURE.md` for the full dependency boundary diagram.

---

## CI

The GitHub Actions workflow (`.github/workflows/blank.yml`) runs on every push/PR to `main`:

1. `cargo fmt --check` — enforces consistent style
2. `cargo clippy -D warnings` — zero-warning policy on all telemetry crates
3. `cargo test` — 3 unit tests in `telemetry-core` (contract validation)

