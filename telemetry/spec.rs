// =============================================================================
// telemetry/spec.rs — Telemetry Specification v0.1.0
// =============================================================================
// This file IS the spec. Written in Rust so the compiler enforces it.
// See also: crates/telemetry-core/src/events.rs for the type definitions.
//
// ─────────────────────────────────────────────────────────────────────────────
// 1. EVENT NAMING CONVENTION
// ─────────────────────────────────────────────────────────────────────────────
//
//   Format: {domain}.{action}
//
//   market.l1              Level 1 (top-of-book) update
//   market.l2              Level 2 (depth) update
//   strategy.decision      Strategy committed to a trade
//   risk.check             Pre-trade risk check result
//   order.submitted        Order sent to venue
//   order.ack              Venue acknowledged the order
//   order.rejected         Venue rejected the order
//   trade.fill             Partial or full fill received
//   system.latency         Latency measurement event
//
// ─────────────────────────────────────────────────────────────────────────────
// 2. REQUIRED IDs (Causality Chain)
// ─────────────────────────────────────────────────────────────────────────────
//
//   trace_id          UUID v4, ties the full chain together
//   decision_id       UUID v4, unique per strategy decision
//   client_order_id   UUID v4, unique per order (survives cancel/replace)
//   order_id          String, venue-assigned
//   exec_id           String, venue-assigned per fill
//   symbol_id         String, canonical symbol (e.g., "ES", "NQ")
//
//   Causality: trace_id → decision_id → client_order_id → order_id → exec_id
//
// ─────────────────────────────────────────────────────────────────────────────
// 3. REQUIRED TIMESTAMPS
// ─────────────────────────────────────────────────────────────────────────────
//
//   ts_exchange        Exchange-reported time (their clock)
//   ts_local_rx        When your machine received the data
//   ts_decision        When your strategy committed
//   ts_order_tx        When the order was transmitted
//   ts_ack_rx          When the ack was received
//   ts_fill_rx         When the fill was received
//
//   Rule: Every event MUST have at least one timestamp set.
//   Rule: Use UTC everywhere. No local timezones in telemetry.
//
// ─────────────────────────────────────────────────────────────────────────────
// 4. SCHEMA VERSIONING
// ─────────────────────────────────────────────────────────────────────────────
//
//   Version format: semver (MAJOR.MINOR.PATCH)
//
//   PATCH: bug fixes, no field changes
//   MINOR: new optional fields added (backward compatible)
//   MAJOR: required fields changed/removed (breaking)
//
//   Every envelope carries `schema_version`. Consumers MUST check it.
//   Validators MUST reject events with unrecognized major versions.
//
//   Current version: 0.1.0
//
// ─────────────────────────────────────────────────────────────────────────────
// 5. KPI DEFINITIONS (exact formulas)
// ─────────────────────────────────────────────────────────────────────────────
//
// 5.1 SLIPPAGE
//
//   Definition: difference between reference price and actual fill price.
//
//   slippage_decision = avg_fill_price - decision_price
//     where: decision_price = mid at ts_decision
//     where: avg_fill_price = Σ(fill_qty × fill_price) / Σ(fill_qty)
//
//   slippage_arrival = avg_fill_price - arrival_price
//     where: arrival_price = mid at ts_order_tx
//
//   Convention: positive = unfavorable for buyer, favorable for seller.
//
//   Unit: points (or convert to ticks: slippage / tick_size)
//
// 5.2 ADVERSE SELECTION
//
//   Definition: how much the mid moves against you AFTER your fill.
//
//   adverse_selection(T) = mid(ts_fill + T) - fill_price
//     Measured at T = {1s, 5s, 30s, 60s}
//
//   For buys: positive = price went up (good), negative = went down (bad)
//   For sells: flip the sign.
//
// 5.3 LATENCY PERCENTILES
//
//   Measured at these hops:
//     decision_to_order   = ts_order_tx - ts_decision
//     order_to_ack        = ts_ack_rx - ts_order_tx
//     ack_to_first_fill   = ts_fill_rx(first) - ts_ack_rx
//     end_to_end          = ts_fill_rx(last) - ts_decision
//
//   Report: p50, p95, p99, p99.9 (over a session or day)
//   Unit: microseconds (μs)
//
// 5.4 FILL METRICS
//
//   fill_ratio = Σ(fill_qty) / desired_qty
//     1.0 = fully filled, <1.0 = partial, 0.0 = no fill
//
//   time_to_fill = ts_fill_rx(last) - ts_order_tx
//     For IOC: should be < 100ms. For GTC: can be hours.
//
//   time_to_first_fill = ts_fill_rx(first) - ts_order_tx
//
// 5.5 DATA QUALITY METRICS
//
//   missing_fields_ratio = count(events with missing required fields) / total
//   stale_book_ratio = count(L1 updates with same bid/ask as previous) / total
//   out_of_order_ratio = count(events where ts_local_rx < previous) / total
//
//   Target: all ratios < 0.01 (less than 1%)
//
// ─────────────────────────────────────────────────────────────────────────────
// 6. IMMUTABLE LOG RULES
// ─────────────────────────────────────────────────────────────────────────────
//
//   1. Events are APPEND-ONLY. Never modify or delete a written event.
//   2. Format: JSONL (one JSON object per line, newline-delimited).
//   3. Validation runs BEFORE write. Invalid events are rejected, not fixed.
//   4. File rotation: daily or by size (recommend 100MB per file).
//   5. Retention: minimum 90 days for audit/compliance.
//
// =============================================================================
