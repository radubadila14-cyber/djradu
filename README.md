# Radu Trading System — Telemetry Foundation

Professional CME-style trading telemetry in Rust. This workspace provides
a strongly-typed event contract, a deterministic golden-trace simulator, and
a command-line viewer — all wired together for real futures trading workflows.

## Workspace Layout

```
djradu/
├── Cargo.toml                   # workspace root (resolver = 2)
├── crates/
│   ├── telemetry-core/          # library — event model, writer, validation
│   ├── telemetry-sim/           # binary — deterministic golden-trace generator
│   └── telemetry-viewer/        # binary — JSONL summary / cat / filter
└── telemetry/
    └── schemas/                 # JSON Schema files (generated, git-ignored)
```

## Quick Start

```bash
# build everything
cargo build -p telemetry-core -p telemetry-sim -p telemetry-viewer

# generate a deterministic golden trace (seed = 42, ESH5)
cargo run -p telemetry-sim -- --out telemetry/golden_trace.jsonl

# view summary
cargo run -p telemetry-viewer -- summary telemetry/golden_trace.jsonl

# pretty-print each event
cargo run -p telemetry-viewer -- cat telemetry/golden_trace.jsonl

# filter to fills only
cargo run -p telemetry-viewer -- filter telemetry/golden_trace.jsonl --event-type cme.exec.fill

# generate JSON Schemas into telemetry/schemas/
cargo run --bin generate-schemas -- telemetry/schemas
```

## Event Model

Every JSONL line is a flat `TelemetryEnvelope` (schema version `0.1.0`).

### Correlation IDs

| Field         | Type   | Purpose                         |
|---------------|--------|---------------------------------|
| `trace_id`    | UUID   | End-to-end causality chain      |
| `decision_id` | UUID?  | Which strategy decision         |
| `cl_ord_id`   | String?| Client-assigned order id        |
| `order_id`    | String?| Exchange-assigned order id      |
| `exec_id`     | String?| Execution report id             |
| `symbol`      | String?| CME product (e.g. `ESH5`)       |
| `security_id` | u64?   | Numeric security identifier     |

### Timestamps

| Field         | Meaning                              |
|---------------|--------------------------------------|
| `ts_local_rx` | Wall-clock when event was written    |
| `ts_exchange` | Exchange-sourced timestamp           |
| `ts_decision` | When the strategy fired              |
| `ts_order_tx` | When order was sent to exchange      |
| `ts_ack_rx`   | When the ack was received            |
| `ts_fill_rx`  | When the fill was received           |

### Event Types

| `event_type`         | Description                       |
|----------------------|-----------------------------------|
| `cme.market.l1`      | Best bid/ask price + size         |
| `cme.market.l2`      | Top-N price levels (arrays)       |
| `cme.order.new`      | New order intent                  |
| `cme.order.cancel`   | Cancel request                    |
| `cme.order.replace`  | Modify (replace) request          |
| `cme.exec.ack`       | Exchange acknowledgment           |
| `cme.exec.reject`    | Exchange rejection                |
| `cme.exec.fill`      | Partial or full fill              |
| `strategy.decision`  | Trading signal + decision price   |
| `risk.check`         | Pre-trade risk gate result        |
| `system.latency`     | Named latency span (μs)           |

## Tests & Linting

```bash
cargo test -p telemetry-core -p telemetry-sim -p telemetry-viewer
cargo clippy -p telemetry-core -p telemetry-sim -p telemetry-viewer -- -D warnings
```

## Architecture

- **telemetry-core** — pure library, no I/O dependencies beyond `std`.
  Publishable as a standalone crate; external apps (e.g. `radu-trading-system`)
  can depend on it via a git path dep.
- **telemetry-sim** — deterministic simulator with `--seed`, `--out`,
  `--sync-policy` flags. Re-running with the same seed produces identical JSONL.
- **telemetry-viewer** — read-only analysis tool; never mutates the log.

## CME/FIX Enums

`Side` · `OrdType` · `TimeInForce` · `OrdStatus` · `ExecType` ·
`RiskCheckResult` · `LiquidityRole`

All serialise as `SCREAMING_SNAKE_CASE` strings (e.g. `"PARTIALLY_FILLED"`).
