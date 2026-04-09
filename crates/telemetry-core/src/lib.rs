// =============================================================================
// telemetry-core — Professional trading telemetry for Radu's Trading System
// =============================================================================
//
// This crate provides:
//   - Strong types for all telemetry events (the "contract")
//   - Correlation IDs for causality tracking (trace_id, decision_id, etc.)
//   - Multi-timestamp model (exchange, local, decision, order, ack, fill)
//   - Deterministic validation (reject malformed events at the boundary)
//   - JSONL writer with validation-before-write
//
// Schema version: 0.1.0
// =============================================================================

pub mod events;
pub mod ids;
pub mod timestamps;
pub mod validate;
pub mod writer;
