use anyhow::Result;
use clap::Parser;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use telemetry_core::enums::{
    ExecType, LiquidityRole, OrdStatus, OrdType, RiskCheckResult, Side, TimeInForce,
};
use telemetry_core::events::{
    ExecAck, ExecFill, MarketL1, MarketL2, OrderNew, PriceLevel, RiskCheck, StrategyDecision,
    SystemLatency, TelemetryEnvelope, TelemetryEvent,
};
use telemetry_core::ids::{ClOrdId, DecisionId, ExecId, OrderId, Symbol, TraceId};
use telemetry_core::timestamps;
use telemetry_core::writer::{JsonlWriter, SyncPolicy, WriterConfig};
use uuid::Builder as UuidBuilder;

#[derive(Parser, Debug)]
#[command(name = "telemetry-sim", about = "CME telemetry golden trace simulator")]
struct Args {
    /// Output JSONL file path
    #[arg(long, default_value = "telemetry/golden_trace.jsonl")]
    out: String,

    /// Random seed for deterministic trace generation
    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// Sync policy: none, flush, or fsync
    #[arg(long, default_value = "flush")]
    sync_policy: String,

    /// Skip validation before writing
    #[arg(long)]
    no_validate: bool,
}

fn make_uuid(rng: &mut StdRng) -> uuid::Uuid {
    let bytes: [u8; 16] = rng.gen();
    UuidBuilder::from_random_bytes(bytes).into_uuid()
}

fn main() -> Result<()> {
    let args = Args::parse();

    let sync_policy = match args.sync_policy.as_str() {
        "none" => SyncPolicy::None,
        "fsync" => SyncPolicy::Fsync,
        _ => SyncPolicy::Flush,
    };

    let mut config = WriterConfig::new(&args.out);
    config.sync_policy = sync_policy;
    config.validate = !args.no_validate;

    let mut writer = JsonlWriter::with_config(config)?;
    let mut rng = StdRng::seed_from_u64(args.seed);

    let trace_id = TraceId(make_uuid(&mut rng));
    let decision_id = DecisionId(make_uuid(&mut rng));
    let symbol = Symbol("ESH5".to_string());
    let cl_ord_id = ClOrdId(format!("CLO-{:08X}", rng.gen::<u32>()));
    let order_id = OrderId(format!("ORD-{:08X}", rng.gen::<u32>()));
    let exec_id_ack = ExecId(format!("EXE-{:08X}", rng.gen::<u32>()));
    let exec_id_part = ExecId(format!("EXE-{:08X}", rng.gen::<u32>()));
    let exec_id_full = ExecId(format!("EXE-{:08X}", rng.gen::<u32>()));

    let mid_px: f64 = 5800.0 + rng.gen_range(-5.0..5.0_f64);
    let spread: f64 = 0.25;
    let bid_px = mid_px - spread / 2.0;
    let ask_px = mid_px + spread / 2.0;
    let limit_px = bid_px;

    let order_qty = 10.0_f64;
    let partial_qty = order_qty / 2.0;
    let remaining_qty = order_qty - partial_qty;

    let ts_decision = timestamps::now();
    let ts_order_tx = timestamps::now();
    let ts_ack_rx = timestamps::now();
    let ts_fill1_rx = timestamps::now();
    let ts_fill2_rx = timestamps::now();

    // 1. L1 market data
    let mut env = TelemetryEnvelope::new(
        trace_id.clone(),
        TelemetryEvent::MarketL1(MarketL1 {
            bid_px,
            bid_sz: rng.gen_range(5u64..50),
            ask_px,
            ask_sz: rng.gen_range(5u64..50),
        }),
    );
    env.symbol = Some(symbol.clone());
    writer.write(&env)?;

    // 2. L2 market data
    let mut bids = Vec::new();
    let mut asks = Vec::new();
    for i in 0..5u64 {
        bids.push(PriceLevel {
            px: bid_px - i as f64 * 0.25,
            sz: rng.gen_range(5u64..100),
            count: Some(rng.gen_range(1u32..10)),
        });
        asks.push(PriceLevel {
            px: ask_px + i as f64 * 0.25,
            sz: rng.gen_range(5u64..100),
            count: Some(rng.gen_range(1u32..10)),
        });
    }
    let mut env = TelemetryEnvelope::new(
        trace_id.clone(),
        TelemetryEvent::MarketL2(MarketL2 { bids, asks }),
    );
    env.symbol = Some(symbol.clone());
    writer.write(&env)?;

    // 3. Strategy decision
    let mut env = TelemetryEnvelope::new(
        trace_id.clone(),
        TelemetryEvent::StrategyDecision(StrategyDecision {
            signal: "MOMENTUM_LONG".to_string(),
            target_side: Side::Buy,
            target_qty: order_qty,
            decision_price: mid_px,
            confidence: Some(0.85),
            model_version: Some("v1.2.0".to_string()),
        }),
    );
    env.decision_id = Some(decision_id.clone());
    env.symbol = Some(symbol.clone());
    env.ts_decision = Some(ts_decision);
    writer.write(&env)?;

    // 4. Risk check pass
    let mut env = TelemetryEnvelope::new(
        trace_id.clone(),
        TelemetryEvent::RiskCheck(RiskCheck {
            result: RiskCheckResult::Pass,
            checks_passed: vec![
                "position_limit".to_string(),
                "notional_limit".to_string(),
                "fat_finger".to_string(),
            ],
            checks_failed: vec![],
            notional: order_qty * mid_px * 50.0,
            account_exposure: 0.12,
        }),
    );
    env.decision_id = Some(decision_id.clone());
    env.symbol = Some(symbol.clone());
    writer.write(&env)?;

    // 5. New order
    let mut env = TelemetryEnvelope::new(
        trace_id.clone(),
        TelemetryEvent::OrderNew(OrderNew {
            side: Side::Buy,
            ord_type: OrdType::Limit,
            order_qty,
            price: Some(limit_px),
            time_in_force: TimeInForce::Day,
        }),
    );
    env.decision_id = Some(decision_id.clone());
    env.cl_ord_id = Some(cl_ord_id.clone());
    env.symbol = Some(symbol.clone());
    env.ts_decision = Some(ts_decision);
    env.ts_order_tx = Some(ts_order_tx);
    writer.write(&env)?;

    // 6. Exchange ack
    let mut env = TelemetryEnvelope::new(
        trace_id.clone(),
        TelemetryEvent::ExecAck(ExecAck {
            ord_status: OrdStatus::New,
            exec_type: ExecType::New,
            side: Side::Buy,
            order_qty,
            cum_qty: 0.0,
            leaves_qty: order_qty,
        }),
    );
    env.cl_ord_id = Some(cl_ord_id.clone());
    env.order_id = Some(order_id.clone());
    env.exec_id = Some(exec_id_ack);
    env.symbol = Some(symbol.clone());
    env.ts_ack_rx = Some(ts_ack_rx);
    writer.write(&env)?;

    // 7. Partial fill
    let mut env = TelemetryEnvelope::new(
        trace_id.clone(),
        TelemetryEvent::ExecFill(ExecFill {
            ord_status: OrdStatus::PartiallyFilled,
            exec_type: ExecType::PartialFill,
            side: Side::Buy,
            last_qty: partial_qty,
            last_px: limit_px,
            order_qty,
            cum_qty: partial_qty,
            leaves_qty: remaining_qty,
            avg_px: limit_px,
            liquidity_role: LiquidityRole::Taker,
            trade_id: Some(format!("TRD-{:08X}", rng.gen::<u32>())),
        }),
    );
    env.cl_ord_id = Some(cl_ord_id.clone());
    env.order_id = Some(order_id.clone());
    env.exec_id = Some(exec_id_part);
    env.symbol = Some(symbol.clone());
    env.ts_fill_rx = Some(ts_fill1_rx);
    writer.write(&env)?;

    // 8. System latency: decision-to-ack
    let latency_decision_to_ack = rng.gen_range(500u64..2000);
    let mut env = TelemetryEnvelope::new(
        trace_id.clone(),
        TelemetryEvent::SystemLatency(SystemLatency {
            span_name: "decision_to_order_tx".to_string(),
            latency_us: latency_decision_to_ack,
            from_ts: Some(ts_decision),
            to_ts: Some(ts_ack_rx),
        }),
    );
    env.decision_id = Some(decision_id.clone());
    writer.write(&env)?;

    // 9. Another L1 update
    let jitter: f64 = rng.gen_range(-0.5..0.5_f64);
    let mut env = TelemetryEnvelope::new(
        trace_id.clone(),
        TelemetryEvent::MarketL1(MarketL1 {
            bid_px: bid_px + jitter,
            bid_sz: rng.gen_range(5u64..50),
            ask_px: ask_px + jitter,
            ask_sz: rng.gen_range(5u64..50),
        }),
    );
    env.symbol = Some(symbol.clone());
    writer.write(&env)?;

    // 10. Full fill
    let mut env = TelemetryEnvelope::new(
        trace_id.clone(),
        TelemetryEvent::ExecFill(ExecFill {
            ord_status: OrdStatus::Filled,
            exec_type: ExecType::Fill,
            side: Side::Buy,
            last_qty: remaining_qty,
            last_px: limit_px,
            order_qty,
            cum_qty: order_qty,
            leaves_qty: 0.0,
            avg_px: limit_px,
            liquidity_role: LiquidityRole::Taker,
            trade_id: Some(format!("TRD-{:08X}", rng.gen::<u32>())),
        }),
    );
    env.cl_ord_id = Some(cl_ord_id);
    env.order_id = Some(order_id);
    env.exec_id = Some(exec_id_full);
    env.symbol = Some(symbol.clone());
    env.ts_fill_rx = Some(ts_fill2_rx);
    writer.write(&env)?;

    // 11. System latency: decision-to-fill
    let latency_decision_to_fill = rng.gen_range(2000u64..10000);
    let mut env = TelemetryEnvelope::new(
        trace_id.clone(),
        TelemetryEvent::SystemLatency(SystemLatency {
            span_name: "decision_to_fill".to_string(),
            latency_us: latency_decision_to_fill,
            from_ts: Some(ts_decision),
            to_ts: Some(ts_fill2_rx),
        }),
    );
    env.decision_id = Some(decision_id);
    writer.write(&env)?;

    writer.flush()?;

    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│              telemetry-sim: Golden Trace                 │");
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  Symbol     : {:<42} │", symbol.0);
    println!("│  Trace ID   : {:<42} │", trace_id);
    println!("│  Output     : {:<42} │", args.out);
    println!("│  Seed       : {:<42} │", args.seed);
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  Events written   : {:<36} │", writer.events_written());
    println!("│  Events rejected  : {:<36} │", writer.events_rejected());
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  Mid price        : {:<36.4} │", mid_px);
    println!(
        "│  Latency d→ack    : {:<36} │",
        format!("{} µs", latency_decision_to_ack)
    );
    println!(
        "│  Latency d→fill   : {:<36} │",
        format!("{} µs", latency_decision_to_fill)
    );
    println!("└─────────────────────────────────────────────────────────┘");

    Ok(())
}
