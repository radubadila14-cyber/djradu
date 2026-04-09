// =============================================================================
// telemetry-viewer — KPI Calculator
// =============================================================================
// Reads a JSONL telemetry trace and computes professional trading KPIs:
//   - Decision-to-fill slippage
//   - Latency breakdown (decision→order, order→ack, ack→fill)
//   - Fill ratio and partial fill stats
//   - Maker/taker fill split
//
// USAGE:
//   cargo run -p telemetry-viewer -- telemetry/golden_trace.jsonl
//   cargo run -p telemetry-viewer            (defaults to the golden trace)
// =============================================================================

use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs};

use anyhow::Result;
use chrono::{DateTime, Utc};
use telemetry_core::events::{LiquidityRole, Side, TelemetryEnvelope, TelemetryEvent};

fn main() -> Result<()> {
    let path: PathBuf = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("telemetry/golden_trace.jsonl"));

    let content = fs::read_to_string(&path)?;
    let events: Vec<TelemetryEnvelope> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line)
                .map_err(|e| anyhow::anyhow!("Line {}: parse error: {}", i + 1, e))
        })
        .collect::<Result<_>>()?;

    println!("═══════════════════════════════════════════════════════");
    println!("  TELEMETRY VIEWER — KPI Report");
    println!("  File: {}", path.display());
    println!("  Events: {}", events.len());
    println!("═══════════════════════════════════════════════════════\n");

    let report = compute_kpis(&events);
    report.print();

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// KPI Accumulators
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct KpiReport {
    // Event counts by type
    event_counts: HashMap<&'static str, usize>,

    // Strategy
    decisions: Vec<StrategyDecisionSummary>,

    // Fills
    fills: Vec<FillSummary>,

    // Latency (microseconds)
    decision_to_order_us: Vec<i64>,
    order_to_ack_us: Vec<i64>,
    ack_to_fill_us: Vec<i64>,

    // Order state
    submitted_qty: f64,
}

struct StrategyDecisionSummary {
    decision_price: f64,
    desired_qty: f64,
    side: Side,
}

struct FillSummary {
    fill_price: f64,
    fill_qty: f64,
    role: LiquidityRole,
}

impl KpiReport {
    fn print(&self) {
        // ── Event Breakdown ───────────────────────────────────────────────
        println!("┌─ Event Breakdown ─────────────────────────────────────┐");
        let mut types: Vec<_> = self.event_counts.iter().collect();
        types.sort_by_key(|(k, _)| *k);
        for (kind, count) in &types {
            println!("│  {:<30} {:>6}", kind, count);
        }
        println!("└───────────────────────────────────────────────────────┘\n");

        // ── Fill KPIs ─────────────────────────────────────────────────────
        if !self.fills.is_empty() {
            let total_fill_qty: f64 = self.fills.iter().map(|f| f.fill_qty).sum();
            let wavg_fill_price: f64 = self
                .fills
                .iter()
                .map(|f| f.fill_price * f.fill_qty)
                .sum::<f64>()
                / total_fill_qty;

            let maker_qty: f64 = self
                .fills
                .iter()
                .filter(|f| matches!(f.role, LiquidityRole::Maker))
                .map(|f| f.fill_qty)
                .sum();
            let taker_qty: f64 = self
                .fills
                .iter()
                .filter(|f| matches!(f.role, LiquidityRole::Taker))
                .map(|f| f.fill_qty)
                .sum();

            println!("┌─ Fill KPIs ────────────────────────────────────────────┐");
            println!("│  Fills received:      {}", self.fills.len());
            println!("│  Total qty filled:    {:.2}", total_fill_qty);

            if self.submitted_qty > 0.0 {
                println!(
                    "│  Fill ratio:          {:.1}%",
                    (total_fill_qty / self.submitted_qty) * 100.0
                );
            }
            println!("│  Wavg fill price:     {:.4}", wavg_fill_price);
            println!(
                "│  Maker qty:           {:.2}  ({:.1}%)",
                maker_qty,
                if total_fill_qty > 0.0 {
                    maker_qty / total_fill_qty * 100.0
                } else {
                    0.0
                }
            );
            println!(
                "│  Taker qty:           {:.2}  ({:.1}%)",
                taker_qty,
                if total_fill_qty > 0.0 {
                    taker_qty / total_fill_qty * 100.0
                } else {
                    0.0
                }
            );
            println!("└───────────────────────────────────────────────────────┘\n");

            // ── Slippage ───────────────────────────────────────────────────
            for dec in &self.decisions {
                let slippage = match dec.side {
                    Side::Buy => wavg_fill_price - dec.decision_price,
                    Side::Sell => dec.decision_price - wavg_fill_price,
                };
                let slippage_ticks = slippage / 0.25; // ES tick size
                println!("┌─ Slippage (vs decision mid) ──────────────────────────┐");
                println!("│  Decision mid:        {:.4}", dec.decision_price);
                println!("│  Desired qty:         {:.2}", dec.desired_qty);
                println!("│  Wavg fill price:     {:.4}", wavg_fill_price);
                println!("│  Slippage (pts):      {:+.4}", slippage);
                println!("│  Slippage (ticks):    {:+.2}", slippage_ticks);
                println!("│  Note: positive = unfavorable for {:?}", dec.side);
                println!("└───────────────────────────────────────────────────────┘\n");
            }
        }

        // ── Latency ───────────────────────────────────────────────────────
        if !self.decision_to_order_us.is_empty()
            || !self.order_to_ack_us.is_empty()
            || !self.ack_to_fill_us.is_empty()
        {
            println!("┌─ Latency Breakdown ────────────────────────────────────┐");
            print_latency_row("decision→order (μs)", &self.decision_to_order_us);
            print_latency_row("order→ack (μs)     ", &self.order_to_ack_us);
            print_latency_row("ack→first fill (μs)", &self.ack_to_fill_us);
            println!("└───────────────────────────────────────────────────────┘\n");
        }
    }
}

fn print_latency_row(label: &str, samples: &[i64]) {
    if samples.is_empty() {
        return;
    }
    let mut s = samples.to_vec();
    s.sort_unstable();
    let p50 = s[s.len() / 2];
    let p95 = s[(s.len() as f64 * 0.95) as usize];
    let p99 = s[(s.len() as f64 * 0.99) as usize];
    println!("│  {}: p50={} p95={} p99={}", label, p50, p95, p99);
}

// ─────────────────────────────────────────────────────────────────────────────
// KPI computation pass
// ─────────────────────────────────────────────────────────────────────────────

fn compute_kpis(events: &[TelemetryEnvelope]) -> KpiReport {
    let mut report = KpiReport::default();

    // We need to correlate by client_order_id across events.
    // Keep the latest ts_order_tx and ts_ack_rx for each order.
    let mut order_tx_time: Option<DateTime<Utc>> = None;
    let mut ack_rx_time: Option<DateTime<Utc>> = None;
    let mut decision_tx_time: Option<DateTime<Utc>> = None;
    let mut first_fill_seen = false;

    for env in events {
        let kind = event_kind(&env.event);
        *report.event_counts.entry(kind).or_insert(0) += 1;

        match &env.event {
            TelemetryEvent::StrategyDecision(d) => {
                decision_tx_time = env.timestamps.ts_decision;
                report.decisions.push(StrategyDecisionSummary {
                    decision_price: d.decision_price,
                    desired_qty: d.desired_quantity,
                    side: d.side,
                });
            }
            TelemetryEvent::OrderSubmitted(o) => {
                report.submitted_qty = o.quantity;
                if let Some(t) = env.timestamps.ts_order_tx {
                    // decision → order latency
                    if let Some(dt) = decision_tx_time {
                        let us = t.signed_duration_since(dt).num_microseconds().unwrap_or(0);
                        report.decision_to_order_us.push(us);
                    }
                    order_tx_time = Some(t);
                }
            }
            TelemetryEvent::OrderAck(_) => {
                if let Some(t) = env.timestamps.ts_ack_rx {
                    if let Some(ot) = order_tx_time {
                        let us = t.signed_duration_since(ot).num_microseconds().unwrap_or(0);
                        report.order_to_ack_us.push(us);
                    }
                    ack_rx_time = Some(t);
                }
            }
            TelemetryEvent::TradeFill(f) => {
                if let Some(t) = env.timestamps.ts_fill_rx
                    && !first_fill_seen
                {
                    if let Some(at) = ack_rx_time {
                        let us = t.signed_duration_since(at).num_microseconds().unwrap_or(0);
                        report.ack_to_fill_us.push(us);
                    }
                    first_fill_seen = true;
                }
                report.fills.push(FillSummary {
                    fill_price: f.fill_price,
                    fill_qty: f.fill_quantity,
                    role: f.liquidity_role,
                });
            }
            _ => {}
        }
    }

    report
}

fn event_kind(e: &TelemetryEvent) -> &'static str {
    match e {
        TelemetryEvent::MarketL1(_) => "market.l1",
        TelemetryEvent::MarketL2(_) => "market.l2",
        TelemetryEvent::StrategyDecision(_) => "strategy.decision",
        TelemetryEvent::RiskCheck(_) => "risk.check",
        TelemetryEvent::OrderSubmitted(_) => "order.submitted",
        TelemetryEvent::OrderAck(_) => "order.ack",
        TelemetryEvent::OrderRejected(_) => "order.rejected",
        TelemetryEvent::TradeFill(_) => "trade.fill",
        TelemetryEvent::SystemLatency(_) => "system.latency",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_trace_produces_empty_report() {
        let report = compute_kpis(&[]);
        assert!(report.fills.is_empty());
        assert!(report.decisions.is_empty());
        assert!(report.event_counts.is_empty());
    }

    #[test]
    fn event_kind_covers_all_variants() {
        use telemetry_core::events::*;
        use telemetry_core::ids::*;
        use telemetry_core::timestamps::EventTimestamps;

        let trace_id = TraceId::new();
        let make =
            |ev: TelemetryEvent| TelemetryEnvelope::new(trace_id, EventTimestamps::local_now(), ev);

        let samples = vec![
            make(TelemetryEvent::MarketL1(MarketL1Event {
                symbol: SymbolId("ES".into()),
                best_bid: PriceLevel {
                    price: 5400.0,
                    size: 100.0,
                },
                best_ask: PriceLevel {
                    price: 5400.25,
                    size: 100.0,
                },
                mid_price: 5400.125,
                spread: 0.25,
                last_trade_price: None,
                last_trade_size: None,
            })),
            make(TelemetryEvent::SystemLatency(SystemLatencyEvent {
                component: "test".into(),
                latency_us: 42,
                percentile: "p50".into(),
            })),
        ];

        let report = compute_kpis(&samples);
        assert_eq!(*report.event_counts.get("market.l1").unwrap(), 1);
        assert_eq!(*report.event_counts.get("system.latency").unwrap(), 1);
    }
}
