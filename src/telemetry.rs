// =============================================================================
// telemetry.rs — Live telemetry pipeline wired into the trading system
// =============================================================================
// This module bridges telemetry-core (pure math) with the running system.
// It provides:
//   1. TelemetryPipeline — manages a shared trace, writer, and metrics
//   2. Convenience methods to emit events from any module
//   3. Professional latency histograms (p50/p95/p99) via the `metrics` crate
//   4. Event counters per type
//
// Architecture:
//   cache.rs / db.rs / quant.rs  →  telemetry.rs  →  telemetry-core (validate + write)
//                                                 →  metrics crate (histograms/counters)
//                                                 →  PostgreSQL (events table)
// =============================================================================

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use metrics::{counter, histogram};
use tokio::sync::Mutex;

use telemetry_core::events::*;
use telemetry_core::ids::*;
use telemetry_core::timestamps::EventTimestamps;
use telemetry_core::writer::JsonlWriter;

// ─────────────────────────────────────────────────────────────────────────────
// PIPELINE — the central telemetry hub
// ─────────────────────────────────────────────────────────────────────────────

/// The live telemetry pipeline. Thread-safe via Arc<Mutex<...>>.
/// Each session gets a trace_id; all events within a session share it.
pub struct TelemetryPipeline {
    /// Current session trace ID (ties all events together).
    trace_id: TraceId,
    /// JSONL writer — validates then writes to telemetry/events.jsonl.
    writer: Arc<Mutex<JsonlWriter>>,
    /// When this pipeline was created (for uptime tracking).
    started_at: Instant,
}

impl TelemetryPipeline {
    /// Initialize the telemetry pipeline.
    /// Creates the output directory and opens the JSONL writer.
    pub fn init(output_dir: &str) -> Result<Self> {
        let dir = PathBuf::from(output_dir);
        std::fs::create_dir_all(&dir)?;

        let path = dir.join("events.jsonl");
        let writer = JsonlWriter::new(&path)?;

        let trace_id = TraceId::new();

        tracing::info!(
            trace_id = %trace_id,
            path = %path.display(),
            "Telemetry pipeline initialized"
        );

        Ok(Self {
            trace_id,
            writer: Arc::new(Mutex::new(writer)),
            started_at: Instant::now(),
        })
    }

    /// Get the current session trace ID.
    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    /// Emit a telemetry event — validates, writes to JSONL, increments counters.
    pub async fn emit(&self, event: TelemetryEvent) -> Result<()> {
        let event_type = event_type_label(&event);
        let timestamps = EventTimestamps::local_now();
        let envelope = TelemetryEnvelope::new(self.trace_id, timestamps, event);

        let mut writer = self.writer.lock().await;
        let accepted = writer.write(&envelope)?;

        if accepted {
            counter!("telemetry.events.emitted", "type" => event_type.to_string()).increment(1);
        } else {
            counter!("telemetry.events.rejected", "type" => event_type.to_string()).increment(1);
            tracing::warn!(event_type, "Telemetry event rejected by validator");
        }

        Ok(())
    }

    /// Emit a market L1 event from a price update.
    pub async fn emit_market_l1(
        &self,
        symbol: &str,
        bid_price: f64,
        bid_size: f64,
        ask_price: f64,
        ask_size: f64,
    ) -> Result<()> {
        let mid = (bid_price + ask_price) / 2.0;
        let spread = ask_price - bid_price;

        let event = TelemetryEvent::MarketL1(MarketL1Event {
            symbol: SymbolId(symbol.to_string()),
            best_bid: PriceLevel { price: bid_price, size: bid_size },
            best_ask: PriceLevel { price: ask_price, size: ask_size },
            mid_price: mid,
            spread,
            last_trade_price: None,
            last_trade_size: None,
        });

        // Record spread as a metric for monitoring
        histogram!("market.spread", "symbol" => symbol.to_string()).record(spread);

        self.emit(event).await
    }

    /// Record a latency measurement and emit it as a telemetry event.
    pub async fn record_latency(
        &self,
        component: &str,
        start: Instant,
    ) -> Result<()> {
        let latency_us = start.elapsed().as_micros() as u64;

        // Professional: record into histogram for p50/p95/p99 computation
        histogram!(
            "system.latency_us",
            "component" => component.to_string()
        ).record(latency_us as f64);

        let event = TelemetryEvent::SystemLatency(SystemLatencyEvent {
            component: component.to_string(),
            latency_us,
            percentile: "raw".to_string(),
        });

        self.emit(event).await
    }

    /// Flush all buffered events to disk.
    pub async fn flush(&self) -> Result<()> {
        let mut writer = self.writer.lock().await;
        writer.flush()?;
        Ok(())
    }

    /// Report pipeline stats.
    pub async fn stats(&self) -> (u64, u64, f64) {
        let writer = self.writer.lock().await;
        let uptime = self.started_at.elapsed().as_secs_f64();
        (writer.events_written(), writer.events_rejected(), uptime)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HELPERS
// ─────────────────────────────────────────────────────────────────────────────

/// Extract a label string from a TelemetryEvent variant.
fn event_type_label(event: &TelemetryEvent) -> &'static str {
    match event {
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
