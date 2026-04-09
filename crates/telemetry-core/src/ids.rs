// =============================================================================
// telemetry-core/src/ids.rs — Correlation IDs for causality tracking
// =============================================================================
// LEARNING NOTE (Radu):
//   These IDs are the "thread" that connects events together.
//   When you see a fill, you can trace back: which order? which decision?
//   which market snapshot triggered it? That chain is the "trace."
//
//   We use UUIDs (Universally Unique Identifiers) — 128-bit random values
//   that are practically guaranteed unique across all machines, forever.
//   Example: "550e8400-e29b-41d4-a716-446655440000"
// =============================================================================

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an end-to-end trace.
/// Ties: market snapshot → decision → risk check → order → fills → PnL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(pub Uuid);

/// Unique identifier for a strategy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DecisionId(pub Uuid);

/// Exchange/venue-assigned order ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub String);

/// Client-assigned order ID (survives retries and cancel/replace).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientOrderId(pub Uuid);

/// Execution report ID (unique per fill event).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecId(pub String);

/// Internal canonical symbol identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub String);

impl TraceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl DecisionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl ClientOrderId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for SymbolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
