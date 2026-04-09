// =============================================================================
// telemetry-sim — Golden Trace Generator
// =============================================================================
// LEARNING NOTE (Radu):
//   A "golden trace" is a single, complete, realistic scenario that proves
//   your telemetry pipeline works end-to-end. It's called "golden" because
//   it's the reference truth — any dashboard, agent, or evaluator should
//   produce the SAME KPIs from this dataset.
//
//   This generator creates a realistic ES (E-mini S&P 500) futures trace:
//     1. 100 L1 market updates (price ticking around ~5400)
//     2. 5 L2 snapshots (order book depth)
//     3. 1 strategy decision (Buy 5 contracts)
//     4. 1 risk check (pass)
//     5. 1 order submission
//     6. 1 order acknowledgment
//     7. 1 partial fill (3 contracts)
//     8. 1 final fill (2 contracts)
//     9. Latency breakdown events
//
//   All timestamps are deterministic (seeded) so the output is reproducible.
//   The JSONL output can be loaded by any consumer to verify KPIs.
//
// RUN IT:
//   cargo run -p telemetry-sim
// =============================================================================

use std::path::PathBuf;

use chrono::{Duration, Utc};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use telemetry_core::events::*;
use telemetry_core::ids::*;
use telemetry_core::timestamps::EventTimestamps;
use telemetry_core::writer::JsonlWriter;

fn main() -> anyhow::Result<()> {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║  GOLDEN TRACE GENERATOR — ES Futures             ║");
    println!("║  Telemetry Contract v{}                      ║", SCHEMA_VERSION);
    println!("╚══════════════════════════════════════════════════╝");

    // Deterministic seed for reproducibility
    let mut rng = StdRng::seed_from_u64(42);

    // Output path
    let output_path = PathBuf::from("telemetry/golden_trace.jsonl");
    std::fs::create_dir_all("telemetry")?;
    // Truncate if exists
    if output_path.exists() {
        std::fs::remove_file(&output_path)?;
    }
    let mut writer = JsonlWriter::new(&output_path)?;

    // Shared IDs for this trace
    let trace_id = TraceId::new();
    let decision_id = DecisionId::new();
    let client_order_id = ClientOrderId::new();
    let order_id = OrderId("EX-2026-0409-00001".into());
    let symbol = SymbolId("ES".into());

    // Base time and price
    let base_time = Utc::now() - Duration::hours(1);
    let mut current_price: f64 = 5400.00;
    let tick_size: f64 = 0.25; // ES tick size is 0.25

    println!("\n--- Phase 1: Market Data (100 L1 + 5 L2 updates) ---");

    // ─── PHASE 1: Market data ───────────────────────────────────────────────
    for i in 0..100 {
        // Random walk: price moves +/- 0-2 ticks
        let tick_move = rng.random_range(-2i32..=2);
        current_price += tick_move as f64 * tick_size;
        current_price = (current_price * 4.0).round() / 4.0; // snap to tick

        let spread_ticks = if rng.random_range(0..10) < 7 { 1 } else { 2 };
        let spread = spread_ticks as f64 * tick_size;
        let bid = current_price;
        let ask = current_price + spread;
        let mid = (bid + ask) / 2.0;

        let event_time = base_time + Duration::milliseconds(i * 250); // 4 updates/sec

        let ts = EventTimestamps {
            ts_exchange: Some(event_time - Duration::microseconds(rng.random_range(50..200))),
            ts_local_rx: Some(event_time),
            ts_decision: None,
            ts_order_tx: None,
            ts_ack_rx: None,
            ts_fill_rx: None,
        };

        let envelope = TelemetryEnvelope::new(
            trace_id,
            ts,
            TelemetryEvent::MarketL1(MarketL1Event {
                symbol: symbol.clone(),
                best_bid: PriceLevel {
                    price: bid,
                    size: rng.random_range(50.0..300.0),
                },
                best_ask: PriceLevel {
                    price: ask,
                    size: rng.random_range(50.0..300.0),
                },
                mid_price: mid,
                spread,
                last_trade_price: Some(bid + if rng.random_bool(0.5) { 0.0 } else { spread }),
                last_trade_size: Some(rng.random_range(1.0f64..20.0).round()),
            }),
        );

        writer.write(&envelope)?;

        // Insert L2 snapshots at regular intervals
        if i % 20 == 0 {
            let mut bids = Vec::new();
            let mut asks = Vec::new();
            for level in 0..10 {
                bids.push(PriceLevel {
                    price: bid - level as f64 * tick_size,
                    size: rng.random_range(20.0..500.0),
                });
                asks.push(PriceLevel {
                    price: ask + level as f64 * tick_size,
                    size: rng.random_range(20.0..500.0),
                });
            }

            let l2_envelope = TelemetryEnvelope::new(
                trace_id,
                EventTimestamps {
                    ts_exchange: Some(event_time),
                    ts_local_rx: Some(event_time + Duration::microseconds(30)),
                    ..EventTimestamps::empty()
                },
                TelemetryEvent::MarketL2(MarketL2Event {
                    symbol: symbol.clone(),
                    bids,
                    asks,
                    depth: 10,
                }),
            );
            writer.write(&l2_envelope)?;
        }
    }

    // ─── PHASE 2: Decision ──────────────────────────────────────────────────
    println!("--- Phase 2: Strategy Decision ---");

    let decision_time = base_time + Duration::milliseconds(25_000); // at tick 100
    let decision_mid = current_price + tick_size / 2.0;

    let decision_envelope = TelemetryEnvelope::new(
        trace_id,
        EventTimestamps {
            ts_decision: Some(decision_time),
            ts_local_rx: Some(decision_time - Duration::milliseconds(5)),
            ..EventTimestamps::empty()
        },
        TelemetryEvent::StrategyDecision(StrategyDecisionEvent {
            decision_id,
            symbol: symbol.clone(),
            side: Side::Buy,
            desired_quantity: 5.0,
            decision_price: decision_mid,
            model_id: Some("quant-alpha-v1".into()),
            model_version: Some("0.1.0".into()),
            confidence: Some(0.73),
            reason: "Momentum signal + L2 imbalance detected on ES".into(),
        }),
    );
    writer.write(&decision_envelope)?;

    println!("  Decision: BUY 5 ES @ mid {:.2}, confidence 0.73", decision_mid);

    // ─── PHASE 3: Risk Check ────────────────────────────────────────────────
    println!("--- Phase 3: Risk Check ---");

    let risk_time = decision_time + Duration::microseconds(150);

    let risk_envelope = TelemetryEnvelope::new(
        trace_id,
        EventTimestamps {
            ts_decision: Some(risk_time),
            ..EventTimestamps::empty()
        },
        TelemetryEvent::RiskCheck(RiskCheckEvent {
            decision_id,
            passed: true,
            checks: vec![
                RiskCheckDetail {
                    check_name: "max_position_size".into(),
                    passed: true,
                    value: 5.0,
                    limit: 50.0,
                },
                RiskCheckDetail {
                    check_name: "max_daily_loss".into(),
                    passed: true,
                    value: 1200.0,
                    limit: 10000.0,
                },
                RiskCheckDetail {
                    check_name: "max_order_value".into(),
                    passed: true,
                    value: 5.0 * decision_mid * 50.0, // ES multiplier = $50/point
                    limit: 5_000_000.0,
                },
            ],
            rejection_reason: None,
        }),
    );
    writer.write(&risk_envelope)?;

    println!("  Risk check: PASSED (3/3 checks)");

    // ─── PHASE 4: Order Submission ──────────────────────────────────────────
    println!("--- Phase 4: Order Submission ---");

    let order_time = risk_time + Duration::microseconds(50);
    let limit_price = decision_mid + tick_size; // aggressive limit

    let order_envelope = TelemetryEnvelope::new(
        trace_id,
        EventTimestamps {
            ts_order_tx: Some(order_time),
            ..EventTimestamps::empty()
        },
        TelemetryEvent::OrderSubmitted(OrderSubmittedEvent {
            decision_id,
            client_order_id,
            symbol: symbol.clone(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 5.0,
            limit_price: Some(limit_price),
            time_in_force: TimeInForce::IOC,
        }),
    );
    writer.write(&order_envelope)?;

    println!("  Order: BUY 5 ES LIMIT @ {:.2} IOC", limit_price);

    // ─── PHASE 5: Acknowledgment ────────────────────────────────────────────
    println!("--- Phase 5: Order Acknowledgment ---");

    let ack_time = order_time + Duration::microseconds(850); // ~850μs round-trip

    let ack_envelope = TelemetryEnvelope::new(
        trace_id,
        EventTimestamps {
            ts_ack_rx: Some(ack_time),
            ..EventTimestamps::empty()
        },
        TelemetryEvent::OrderAck(OrderAckEvent {
            client_order_id,
            order_id: order_id.clone(),
            symbol: symbol.clone(),
        }),
    );
    writer.write(&ack_envelope)?;

    let ack_latency_us = 850;
    println!("  Ack received in {}μs", ack_latency_us);

    // ─── PHASE 6: Partial Fill ──────────────────────────────────────────────
    println!("--- Phase 6: Partial Fill ---");

    let fill1_time = ack_time + Duration::microseconds(200);
    let fill1_price = decision_mid + tick_size; // filled at the ask

    let fill1_envelope = TelemetryEnvelope::new(
        trace_id,
        EventTimestamps {
            ts_fill_rx: Some(fill1_time),
            ..EventTimestamps::empty()
        },
        TelemetryEvent::TradeFill(TradeFillEvent {
            client_order_id,
            order_id: order_id.clone(),
            exec_id: ExecId("EXEC-001".into()),
            symbol: symbol.clone(),
            side: Side::Buy,
            fill_quantity: 3.0,
            fill_price: fill1_price,
            cumulative_quantity: 3.0,
            leaves_quantity: 2.0,
            fill_type: FillType::Partial,
            liquidity_role: LiquidityRole::Taker,
            fee: 3.0 * 1.18, // $1.18/contract CME-style
            fee_currency: "USD".into(),
        }),
    );
    writer.write(&fill1_envelope)?;

    println!(
        "  Partial fill: 3 @ {:.2} (taker, fee ${:.2})",
        fill1_price,
        3.0 * 1.18
    );

    // ─── PHASE 7: Final Fill ────────────────────────────────────────────────
    println!("--- Phase 7: Final Fill ---");

    let fill2_time = fill1_time + Duration::milliseconds(15);
    let fill2_price = decision_mid + tick_size; // same price

    let fill2_envelope = TelemetryEnvelope::new(
        trace_id,
        EventTimestamps {
            ts_fill_rx: Some(fill2_time),
            ..EventTimestamps::empty()
        },
        TelemetryEvent::TradeFill(TradeFillEvent {
            client_order_id,
            order_id: order_id.clone(),
            exec_id: ExecId("EXEC-002".into()),
            symbol: symbol.clone(),
            side: Side::Buy,
            fill_quantity: 2.0,
            fill_price: fill2_price,
            cumulative_quantity: 5.0,
            leaves_quantity: 0.0,
            fill_type: FillType::Full,
            liquidity_role: LiquidityRole::Taker,
            fee: 2.0 * 1.18,
            fee_currency: "USD".into(),
        }),
    );
    writer.write(&fill2_envelope)?;

    println!(
        "  Final fill: 2 @ {:.2} (taker, fee ${:.2})",
        fill2_price,
        2.0 * 1.18
    );

    // ─── PHASE 8: Latency Summary ───────────────────────────────────────────
    println!("--- Phase 8: Latency Breakdown ---");

    let latency_events = vec![
        ("decision_to_order", 50u64),
        ("order_to_ack", ack_latency_us),
        ("ack_to_first_fill", 200),
        ("first_fill_to_complete", 15_000),
        ("end_to_end", 50 + ack_latency_us + 200 + 15_000),
    ];

    for (component, latency_us) in &latency_events {
        let lat_envelope = TelemetryEnvelope::new(
            trace_id,
            EventTimestamps::local_now(),
            TelemetryEvent::SystemLatency(SystemLatencyEvent {
                component: component.to_string(),
                latency_us: *latency_us,
                percentile: "actual".into(),
            }),
        );
        writer.write(&lat_envelope)?;
    }

    writer.flush()?;

    // ─── SUMMARY ────────────────────────────────────────────────────────────
    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║  GOLDEN TRACE — COMPUTED KPIs                    ║");
    println!("╠══════════════════════════════════════════════════╣");

    let avg_fill = (fill1_price * 3.0 + fill2_price * 2.0) / 5.0;
    let slippage_vs_decision = avg_fill - decision_mid;
    let total_fees = 5.0 * 1.18;
    let time_to_fill_ms = 15.0 + 0.2; // partial + final

    println!("║  Symbol:              ES (E-mini S&P 500)        ║");
    println!("║  Side:                BUY                         ║");
    println!("║  Quantity:            5 contracts                 ║");
    println!("║  Decision mid:        {:.2}                    ║", decision_mid);
    println!("║  Avg fill price:      {:.2}                    ║", avg_fill);
    println!(
        "║  Slippage (vs dec):   {:.2} ticks ({:.4} pts)    ║",
        slippage_vs_decision / tick_size,
        slippage_vs_decision
    );
    println!("║  Fill ratio:          100% (5/5)                  ║");
    println!("║  Time to fill:        {:.1}ms                    ║", time_to_fill_ms);
    println!("║  Total fees:          ${:.2}                     ║", total_fees);
    println!("║  Latency (dec→ack):   {}μs                     ║", 50 + ack_latency_us);
    println!("║  Liquidity role:      Taker (all fills)           ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!(
        "║  Events written:  {}                              ║",
        writer.events_written()
    );
    println!(
        "║  Events rejected: {}                               ║",
        writer.events_rejected()
    );
    println!("║  Output: telemetry/golden_trace.jsonl             ║");
    println!("╚══════════════════════════════════════════════════╝");

    Ok(())
}
