// =============================================================================
// telemetry-core/src/timestamps.rs — Multi-timestamp model
// =============================================================================
// LEARNING NOTE (Radu):
//   Professional trading systems don't use ONE timestamp per event.
//   They use MULTIPLE, because each captures a different moment:
//
//   ts_exchange  — when the exchange says it happened (their clock)
//   ts_local_rx  — when YOUR machine received the data
//   ts_decision  — when your strategy committed to a decision
//   ts_order_tx  — when you sent the order out
//   ts_ack_rx    — when you received the acknowledgment
//   ts_fill_rx   — when you received the fill confirmation
//
//   This lets you compute REAL latency: "how long from my decision
//   to the exchange acknowledging my order?" That's ts_ack_rx - ts_order_tx.
//
//   Without multiple timestamps, you're guessing. With them, you KNOW.
// =============================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Timestamps captured at various points in the event lifecycle.
/// All fields are Option because not every event has every timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTimestamps {
    /// Exchange-reported timestamp (their clock).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_exchange: Option<DateTime<Utc>>,

    /// Local receipt timestamp (when we received the data).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_local_rx: Option<DateTime<Utc>>,

    /// Decision timestamp (when strategy committed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_decision: Option<DateTime<Utc>>,

    /// Order transmit timestamp (when order was sent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_order_tx: Option<DateTime<Utc>>,

    /// Acknowledgment receipt timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_ack_rx: Option<DateTime<Utc>>,

    /// Fill receipt timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_fill_rx: Option<DateTime<Utc>>,
}

impl EventTimestamps {
    /// Create empty timestamps (to be filled as events flow).
    pub fn empty() -> Self {
        Self {
            ts_exchange: None,
            ts_local_rx: None,
            ts_decision: None,
            ts_order_tx: None,
            ts_ack_rx: None,
            ts_fill_rx: None,
        }
    }

    /// Create with just the local receive time (most common for market data).
    pub fn local_now() -> Self {
        Self {
            ts_local_rx: Some(Utc::now()),
            ..Self::empty()
        }
    }
}
