// =============================================================================
// telemetry-viewer — KPI Computation CLI
// =============================================================================
// Reads a JSONL telemetry event log and computes professional trading KPIs:
//
//   Slippage     — avg fill price vs decision mid price
//   Fill ratio   — cumulative filled / desired quantity
//   Time-to-ack  — ts_order_tx → ts_ack_rx
//   Time-to-fill — ts_order_tx → ts_fill_rx (first fill)
//   Time-to-done — ts_order_tx → ts_fill_rx (final/complete fill)
//
// All CME/FIX-style fields are extracted from the typed telemetry contract
// defined in `telemetry-core`.
//
// RUN:
//   cargo run -p telemetry-viewer -- telemetry/golden_trace.jsonl
// =============================================================================

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::{Context, Result};
use telemetry_core::events::{FillType, TelemetryEnvelope, TelemetryEvent};

// ─────────────────────────────────────────────────────────────────────────────
// KPI accumulators (built by scanning the event log once)
// ─────────────────────────────────────────────────────────────────────────────

struct KpiState {
    // Decision
    decision_price: Option<f64>,
    desired_quantity: Option<f64>,

    // Order lifecycle timestamps (microseconds since epoch)
    ts_order_tx_us: Option<i64>,
    ts_ack_rx_us: Option<i64>,
    ts_first_fill_us: Option<i64>,
    ts_done_fill_us: Option<i64>,

    // Fill accumulation
    cumulative_quantity: f64,
    fill_notional: f64, // sum(fill_price * fill_qty)
    total_fees: f64,
    fill_count: u32,
    partial_fills: u32,
    complete_fills: u32,

    // Event counters
    events_total: u64,
    events_market_l1: u64,
    events_market_l2: u64,
}

impl KpiState {
    fn new() -> Self {
        Self {
            decision_price: None,
            desired_quantity: None,
            ts_order_tx_us: None,
            ts_ack_rx_us: None,
            ts_first_fill_us: None,
            ts_done_fill_us: None,
            cumulative_quantity: 0.0,
            fill_notional: 0.0,
            total_fees: 0.0,
            fill_count: 0,
            partial_fills: 0,
            complete_fills: 0,
            events_total: 0,
            events_market_l1: 0,
            events_market_l2: 0,
        }
    }

    fn ingest(&mut self, envelope: &TelemetryEnvelope) {
        self.events_total += 1;
        let ts = &envelope.timestamps;

        match &envelope.event {
            TelemetryEvent::MarketL1(_) => {
                self.events_market_l1 += 1;
            }
            TelemetryEvent::MarketL2(_) => {
                self.events_market_l2 += 1;
            }
            TelemetryEvent::StrategyDecision(e) => {
                self.decision_price = Some(e.decision_price);
                self.desired_quantity = Some(e.desired_quantity);
            }
            TelemetryEvent::OrderSubmitted(_) => {
                if let Some(t) = ts.ts_order_tx {
                    self.ts_order_tx_us = Some(t.timestamp_micros());
                }
            }
            TelemetryEvent::OrderAck(_) => {
                if let Some(t) = ts.ts_ack_rx {
                    self.ts_ack_rx_us = Some(t.timestamp_micros());
                }
            }
            TelemetryEvent::TradeFill(e) => {
                self.cumulative_quantity = e.cumulative_quantity;
                self.fill_notional += e.fill_price * e.fill_quantity;
                self.total_fees += e.fee;
                self.fill_count += 1;

                if let Some(t) = ts.ts_fill_rx {
                    let us = t.timestamp_micros();
                    if self.ts_first_fill_us.is_none() {
                        self.ts_first_fill_us = Some(us);
                    }
                    if e.fill_type == FillType::Full {
                        self.ts_done_fill_us = Some(us);
                        self.complete_fills += 1;
                    } else {
                        self.partial_fills += 1;
                    }
                }
            }
            TelemetryEvent::RiskCheck(_)
            | TelemetryEvent::OrderRejected(_)
            | TelemetryEvent::SystemLatency(_) => {}
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let path: PathBuf = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("telemetry/golden_trace.jsonl"));

    println!("╔══════════════════════════════════════════════════╗");
    println!("║  TELEMETRY VIEWER — KPI REPORT                   ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!("  Reading: {}", path.display());

    let file = File::open(&path).with_context(|| {
        format!(
            "Cannot open '{}'. Run `cargo run -p telemetry-sim` first to generate it.",
            path.display()
        )
    })?;
    let reader = BufReader::new(file);

    let mut state = KpiState::new();
    let mut parse_errors = 0u64;

    for (line_no, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("I/O error on line {}", line_no + 1))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<TelemetryEnvelope>(line) {
            Ok(envelope) => state.ingest(&envelope),
            Err(_) => parse_errors += 1,
        }
    }

    print_report(&state, parse_errors);
    Ok(())
}

fn print_report(s: &KpiState, parse_errors: u64) {
    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║  EVENT COUNTS                                    ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!(
        "║  Total events:      {:>6}                       ║",
        s.events_total
    );
    println!(
        "║  Market L1:         {:>6}                       ║",
        s.events_market_l1
    );
    println!(
        "║  Market L2:         {:>6}                       ║",
        s.events_market_l2
    );
    println!(
        "║  Parse errors:      {:>6}                       ║",
        parse_errors
    );

    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║  FILL QUALITY KPIs                               ║");
    println!("╠══════════════════════════════════════════════════╣");

    let avg_fill = if s.cumulative_quantity > 0.0 {
        s.fill_notional / s.cumulative_quantity
    } else {
        0.0
    };

    if let Some(dec) = s.decision_price {
        let slippage = avg_fill - dec;
        println!("║  Decision mid:      {:>10.4}                 ║", dec);
        println!("║  Avg fill price:    {:>10.4}                 ║", avg_fill);
        println!(
            "║  Slippage:          {:>+10.4} pts             ║",
            slippage
        );
    } else {
        println!("║  Decision mid:            (no decision event)  ║");
    }

    let fill_ratio = match (s.cumulative_quantity, s.desired_quantity) {
        (cum, Some(des)) if des > 0.0 => format!("{:.1}%", cum / des * 100.0),
        _ => "N/A".to_string(),
    };
    println!("║  Fill ratio:        {:>10}                 ║", fill_ratio);
    println!(
        "║  Total fees:        {:>10.4} USD              ║",
        s.total_fees
    );
    println!(
        "║  Fill events:       {:>6} ({} partial, {} full)    ║",
        s.fill_count, s.partial_fills, s.complete_fills
    );

    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║  LATENCY KPIs (microseconds)                     ║");
    println!("╠══════════════════════════════════════════════════╣");

    match (s.ts_order_tx_us, s.ts_ack_rx_us) {
        (Some(tx), Some(ack)) => {
            let latency = ack - tx;
            println!("║  Time-to-ack:       {:>8} μs               ║", latency);
        }
        _ => println!("║  Time-to-ack:              (insufficient data) ║"),
    }

    match (s.ts_order_tx_us, s.ts_first_fill_us) {
        (Some(tx), Some(fill)) => {
            let latency = fill - tx;
            println!("║  Time-to-first-fill:{:>8} μs               ║", latency);
        }
        _ => println!("║  Time-to-first-fill:       (insufficient data) ║"),
    }

    match (s.ts_order_tx_us, s.ts_done_fill_us) {
        (Some(tx), Some(done)) => {
            let latency_us = done - tx;
            println!(
                "║  Time-to-complete:  {:>8} μs ({:.1}ms)       ║",
                latency_us,
                latency_us as f64 / 1000.0
            );
        }
        _ => println!("║  Time-to-complete:         (insufficient data) ║"),
    }

    println!("╚══════════════════════════════════════════════════╝");
}
