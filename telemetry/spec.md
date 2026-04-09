# Telemetry Specification v0.1.0

CME-style trading telemetry contract for the `djradu` workspace.
Canonical product: **ES (E-mini S&P 500 futures)**.

---

## 1. Event Naming Convention

All event types follow the format `{domain}.{action}`:

| Event type           | Description                                      |
|----------------------|--------------------------------------------------|
| `market.l1`          | Level 1 (top-of-book) update                     |
| `market.l2`          | Level 2 (order book depth) snapshot              |
| `strategy.decision`  | Strategy committed to a trade                    |
| `risk.check`         | Pre-trade risk check result                      |
| `order.submitted`    | Order sent to venue                              |
| `order.ack`          | Venue acknowledged the order                     |
| `order.rejected`     | Venue rejected the order                         |
| `trade.fill`         | Partial or full fill received                    |
| `system.latency`     | Latency measurement at a boundary                |

Rust types live in `crates/telemetry-core/src/events.rs`.

---

## 2. Causality Chain (Required IDs)

Every event envelope carries a `trace_id`.  
Events deeper in the lifecycle also carry narrower IDs:

```
trace_id                 ← ties the entire chain together
  └── decision_id        ← unique per strategy decision
        └── client_order_id  ← unique per order (survives cancel/replace)
              └── order_id   ← venue-assigned
                    └── exec_id  ← venue-assigned, unique per fill
```

| ID                | Type       | Set by      |
|-------------------|------------|-------------|
| `trace_id`        | UUIDv4     | Your system |
| `decision_id`     | UUIDv4     | Strategy    |
| `client_order_id` | UUIDv4     | Your OMS    |
| `order_id`        | String     | Venue/CME   |
| `exec_id`         | String     | Venue/CME   |
| `symbol_id`       | String     | Your system |

---

## 3. Multi-Timestamp Model

Professional trading systems capture **multiple timestamps per event**:

| Field          | Meaning                                     | Required for       |
|----------------|---------------------------------------------|--------------------|
| `ts_exchange`  | Exchange-reported time (venue clock)        | market data        |
| `ts_local_rx`  | When your machine received the data         | market data, fills |
| `ts_decision`  | When strategy committed to an action        | decisions          |
| `ts_order_tx`  | When the order was transmitted              | orders             |
| `ts_ack_rx`    | When the ack was received                   | order lifecycle    |
| `ts_fill_rx`   | When the fill confirmation was received     | fills              |

**Rules:**
- All timestamps MUST be UTC.  No local timezone offsets.
- Every event MUST have at least one timestamp set.
- `ts_exchange` comes from the FIX `TransactTime (tag 60)` field.

---

## 4. CME / FIX Field Mapping

The schema maps directly to FIX 4.4 / FIX 5.0 execution report fields:

| Telemetry field     | FIX tag | FIX name          |
|---------------------|---------|-------------------|
| `side`              | 54      | Side              |
| `order_type`        | 40      | OrdType           |
| `time_in_force`     | 59      | TimeInForce       |
| `quantity`          | 38      | OrderQty          |
| `limit_price`       | 44      | Price             |
| `fill_price`        | 31      | LastPx            |
| `fill_quantity`     | 32      | LastQty           |
| `cumulative_qty`    | 14      | CumQty            |
| `leaves_qty`        | 151     | LeavesQty         |
| `avg_fill_price`    | 6       | AvgPx             |
| `exec_type`         | 150     | ExecType          |
| `ord_status`        | 39      | OrdStatus         |
| `client_order_id`   | 11      | ClOrdID           |
| `order_id`          | 37      | OrderID           |
| `exec_id`           | 17      | ExecID            |
| `ts_exchange`       | 60      | TransactTime      |

---

## 5. Maker / Taker Inference

Every `trade.fill` event carries a `liquidity_role` field:
`Maker`, `Taker`, or `Unknown`.

### Rule of thumb

| Role    | Meaning                                     | CME FIX hint                  |
|---------|---------------------------------------------|-------------------------------|
| `Maker` | Your order **rested** on the book and was hit | `AggressorIndicator (tag 1057) = N` |
| `Taker` | Your order **crossed** the spread immediately | `AggressorIndicator (tag 1057) = Y` |
| `Unknown` | Cannot be determined from available data  | Tag 1057 absent               |

### How to infer when tag 1057 is absent

If the venue does not send `AggressorIndicator`:

1. **Limit order + immediate fill**: Compare `limit_price` to best opposite price at `ts_order_tx`.
   - Buy limit `≥ best_ask` at submission → **Taker** (crossed the spread).
   - Buy limit `< best_ask` at submission → **Maker** (rested, was hit later).

2. **Market order**: always **Taker**.

3. **L2 imbalance heuristic**: if `ts_fill_rx - ts_order_tx < 500μs` on a limit order, likely **Taker**.

### Why it matters

- **Fees**: CME Globex charges takers ~$0.25/side more than makers.
- **Slippage**: taker fills always cross the spread; maker fills do not.
- **Fill probability**: maker orders may not fill at all; taker orders fill immediately.
- **Strategy evaluation**: if your strategy is always taker, you're always paying the spread.

```rust
// Correct field to check in TradeFillEvent:
match fill.liquidity_role {
    LiquidityRole::Maker  => { /* received rebate / crossed by other side */ }
    LiquidityRole::Taker  => { /* crossed spread, paid fee */ }
    LiquidityRole::Unknown => { /* infer from context above */ }
}
```

---

## 6. KPI Definitions

### 6.1 Slippage

```
slippage_decision = avg_fill_price - decision_price      (for Buy)
slippage_decision = decision_price - avg_fill_price      (for Sell)
```

- `decision_price` = mid at `ts_decision`
- `avg_fill_price` = Σ(fill_qty × fill_price) / Σ(fill_qty)
- Unit: **points** (1 ES point = $50)
- Convert to ticks: `slippage / 0.25` (ES tick = 0.25 pts)

### 6.2 Latency Breakdown

| Metric                | Formula                                  |
|-----------------------|------------------------------------------|
| `decision_to_order`   | `ts_order_tx − ts_decision`              |
| `order_to_ack`        | `ts_ack_rx − ts_order_tx`                |
| `ack_to_first_fill`   | `ts_fill_rx(first) − ts_ack_rx`          |
| `end_to_end`          | `ts_fill_rx(last) − ts_decision`         |

Report: **p50, p95, p99** (over session or day). Unit: **microseconds (μs)**.

### 6.3 Fill Ratio

```
fill_ratio = Σ(fill_qty) / desired_qty
```

- `1.0` = fully filled
- `< 1.0` = partial fill
- `0.0` = no fill (rejected or cancelled)

### 6.4 Adverse Selection (post-fill)

```
adverse_selection(T) = mid(ts_fill + T) − fill_price    (for Buy)
```

Measure at T = 1 s, 5 s, 30 s, 60 s.  
Negative = price went against you after the fill (got picked off).

---

## 7. Immutable Log Rules

1. Events are **append-only**. Never modify a written event.
2. Format: **JSONL** (one JSON object per line, newline-delimited).
3. **Validation runs before write.** Invalid events are rejected outright.
4. File rotation: daily, or when file exceeds 100 MB.
5. Retention: minimum **90 days** for audit / compliance.

---

## 8. Schema Versioning

Format: `MAJOR.MINOR.PATCH` (semver)

| Change type            | Bump   |
|------------------------|--------|
| Bug fix, no field change | PATCH |
| New optional fields added (backward compat) | MINOR |
| Required fields changed/removed (breaking)  | MAJOR |

Every envelope carries `schema_version`. Consumers **MUST** check it.
Validators **MUST** reject events with unrecognized MAJOR versions.

Current version: **`0.1.0`**

---

## 9. Validation Rules

Implemented in `crates/telemetry-core/src/validate.rs`:

| Rule                              | Error                               |
|-----------------------------------|-------------------------------------|
| No timestamps set                 | `"event has no timestamps"`         |
| L1 crossed book (bid ≥ ask)       | `"crossed book: bid >= ask"`        |
| L1 negative spread                | `"negative spread"`                 |
| Fill quantity ≤ 0                 | `"fill_quantity must be positive"`  |
| Fill price ≤ 0                    | `"fill_price must be positive"`     |
