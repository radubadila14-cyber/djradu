// =============================================================================
// src/telemetry.rs — Optional telemetry integration (feature: telemetry-integration)
// =============================================================================
// This module is only compiled when the `telemetry-integration` feature is
// enabled. It wires `telemetry-core` into the three application boundaries
// where latency and reliability matter most:
//
//   1. Startup boundary  — system.startup event on successful boot
//   2. DB boundary       — system.latency events around PostgreSQL calls
//   3. Cache boundary    — system.latency events around Redis calls
//
// HOW TO ENABLE:
//   cargo build --features telemetry-integration
//   cargo run   --features telemetry-integration
//   cargo test  --features telemetry-integration
//
// DESIGN:
//   - No heavy dependencies added — we only use telemetry-core
//   - The writer appends JSONL to telemetry/live_trace.jsonl
//   - Errors in telemetry never propagate to the caller (non-fatal)
//   - Timestamps use chrono::Utc, matching the telemetry-core convention
// =============================================================================

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use telemetry_core::events::{
    Side, StrategyDecisionEvent, SystemLatencyEvent, TelemetryEnvelope, TelemetryEvent,
};
use telemetry_core::ids::{DecisionId, SymbolId, TraceId};
use telemetry_core::timestamps::EventTimestamps;
use telemetry_core::writer::JsonlWriter;

// ─────────────────────────────────────────────────────────────────────────────
// Telemetry session
// ─────────────────────────────────────────────────────────────────────────────

/// A live telemetry session. Create one at startup; drop it on shutdown.
///
/// All events share the same `trace_id` for the duration of the session.
/// Errors in write are logged but never propagate (telemetry is non-fatal).
pub struct TelemetrySession {
    trace_id: TraceId,
    writer: JsonlWriter,
}

#[allow(dead_code)]
impl TelemetrySession {
    /// Open a telemetry session, creating the JSONL file.
    ///
    /// # Errors
    /// Returns an error only if the output path cannot be created.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let writer = JsonlWriter::new(&path)?;
        Ok(Self {
            trace_id: TraceId::new(),
            writer,
        })
    }

    /// Emit an arbitrary event. Errors are swallowed and logged.
    fn emit(&mut self, event: TelemetryEvent, timestamps: EventTimestamps) {
        let env = TelemetryEnvelope::new(self.trace_id, timestamps, event);
        if let Err(e) = self.writer.write(&env) {
            tracing::warn!("telemetry write error (non-fatal): {}", e);
        }
    }

    // ── Boundary helpers ─────────────────────────────────────────────────────

    /// Emit a startup latency event (call once all systems are live).
    pub fn emit_startup(&mut self, startup_us: u64) {
        self.emit(
            TelemetryEvent::SystemLatency(SystemLatencyEvent {
                component: "startup".into(),
                latency_us: startup_us,
                percentile: "instant".into(),
            }),
            EventTimestamps::local_now(),
        );
    }

    /// Measure a synchronous DB or cache call and emit a latency event.
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = session.measure("db.init_schema", || db::init_schema(&pool).await);
    /// ```
    pub fn emit_boundary_latency(&mut self, component: &str, elapsed_us: u64) {
        self.emit(
            TelemetryEvent::SystemLatency(SystemLatencyEvent {
                component: component.to_string(),
                latency_us: elapsed_us,
                percentile: "instant".into(),
            }),
            EventTimestamps::local_now(),
        );
    }

    /// Emit a minimal strategy decision event.
    ///
    /// Use this when your strategy decides to trade — typically before
    /// any order is submitted.  Emits a `strategy.decision` event so
    /// the viewer can compute slippage vs the decision mid.
    pub fn emit_decision(
        &mut self,
        symbol: &str,
        side: Side,
        quantity: f64,
        mid_price: f64,
        reason: &str,
    ) {
        let decision_id = DecisionId::new();
        self.emit(
            TelemetryEvent::StrategyDecision(StrategyDecisionEvent {
                decision_id,
                symbol: SymbolId(symbol.into()),
                side,
                desired_quantity: quantity,
                decision_price: mid_price,
                model_id: None,
                model_version: None,
                confidence: None,
                reason: reason.to_string(),
            }),
            EventTimestamps {
                ts_decision: Some(chrono::Utc::now()),
                ..EventTimestamps::empty()
            },
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience timer (avoids pulling in tokio or metrics crates)
// ─────────────────────────────────────────────────────────────────────────────

/// Measure wall-clock microseconds for a synchronous closure.
///
/// ```rust
/// let (result, elapsed_us) = measure_us(|| expensive_call());
/// ```
#[allow(dead_code)]
pub fn measure_us<F, T>(f: F) -> (T, u64)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let us = start.elapsed().as_micros() as u64;
    (result, us)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_opens_and_emits_startup() {
        let dir = tempfile::tempdir().expect("tmp dir");
        let path = dir.path().join("trace.jsonl");

        let mut session = TelemetrySession::open(&path).expect("open session");
        session.emit_startup(12_500);
        session.emit_boundary_latency("db.connect", 250_000);
        session.emit_boundary_latency("cache.connect", 3_000);

        // Flush by dropping writer
        drop(session);

        let content = std::fs::read_to_string(&path).expect("read trace");
        let lines: Vec<_> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 3, "expected 3 events");

        // Verify JSON parses
        for line in &lines {
            let _: serde_json::Value = serde_json::from_str(line).expect("valid JSON");
        }
    }

    #[test]
    fn measure_us_returns_non_zero_for_sleep() {
        let (_, us) = measure_us(|| {
            std::thread::sleep(std::time::Duration::from_millis(1));
        });
        assert!(us >= 1_000, "expected at least 1ms, got {}μs", us);
    }

    #[test]
    fn emit_decision_writes_strategy_event() {
        let dir = tempfile::tempdir().expect("tmp dir");
        let path = dir.path().join("trace.jsonl");

        let mut session = TelemetrySession::open(&path).expect("open session");
        session.emit_decision("ES", Side::Buy, 5.0, 5400.125, "test decision");
        drop(session);

        let content = std::fs::read_to_string(&path).expect("read trace");
        assert!(content.contains("strategy.decision"));
    }
}
