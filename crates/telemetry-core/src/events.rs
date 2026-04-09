// =============================================================================
// telemetry-core/src/events.rs — The Telemetry Contract v0.1
// =============================================================================
// LEARNING NOTE (Radu):
//   This is the CONTRACT. Every event in your entire trading system must
//   be one of these types. The Rust compiler enforces it — if a field is
//   missing or the wrong type, your code WON'T COMPILE.
//
//   This is why Rust is used in professional trading: bugs that would be
//   runtime crashes in Python are compile-time errors in Rust.
//
//   The `enum TelemetryEvent` is a "tagged union" — it can be ANY of the
//   listed variants, but only ONE at a time. When you serialize to JSON,
//   the variant name becomes a "type" field.
//
//   EVENT NAMING CONVENTION:
//     market.l1          — Level 1 (top-of-book) update
//     market.l2          — Level 2 (depth) update
//     strategy.decision  — Strategy committed to an action
//     risk.check         — Pre-trade risk check result
//     order.submitted     — Order sent to venue
//     order.ack          — Venue acknowledged the order
//     order.rejected     — Venue rejected the order
//     trade.fill         — Partial or full fill received
//     system.latency     — Latency measurement event
// =============================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::*;
use crate::timestamps::EventTimestamps;

// ─────────────────────────────────────────────────────────────────────────────
// SCHEMA VERSION — bump this when the contract changes
// ─────────────────────────────────────────────────────────────────────────────
pub const SCHEMA_VERSION: &str = "0.1.0";

// ─────────────────────────────────────────────────────────────────────────────
// ENVELOPE — wraps every event with common metadata
// ─────────────────────────────────────────────────────────────────────────────

/// Every telemetry event is wrapped in this envelope.
/// The envelope provides: schema version, timing, and correlation IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEnvelope {
    /// Schema version for backward compatibility checking.
    pub schema_version: String,

    /// When this envelope was created.
    pub created_at: DateTime<Utc>,

    /// The trace this event belongs to.
    pub trace_id: TraceId,

    /// Multi-timestamp model.
    pub timestamps: EventTimestamps,

    /// The actual event payload.
    pub event: TelemetryEvent,
}

// ─────────────────────────────────────────────────────────────────────────────
// EVENT UNION — all possible telemetry events
// ─────────────────────────────────────────────────────────────────────────────

/// Tagged union of all telemetry event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TelemetryEvent {
    /// Level 1 market data (top-of-book).
    #[serde(rename = "market.l1")]
    MarketL1(MarketL1Event),

    /// Level 2 market data (order book depth).
    #[serde(rename = "market.l2")]
    MarketL2(MarketL2Event),

    /// Strategy decision.
    #[serde(rename = "strategy.decision")]
    StrategyDecision(StrategyDecisionEvent),

    /// Pre-trade risk check.
    #[serde(rename = "risk.check")]
    RiskCheck(RiskCheckEvent),

    /// Order submitted to venue.
    #[serde(rename = "order.submitted")]
    OrderSubmitted(OrderSubmittedEvent),

    /// Order acknowledged by venue.
    #[serde(rename = "order.ack")]
    OrderAck(OrderAckEvent),

    /// Order rejected by venue.
    #[serde(rename = "order.rejected")]
    OrderRejected(OrderRejectedEvent),

    /// Trade fill (partial or complete).
    #[serde(rename = "trade.fill")]
    TradeFill(TradeFillEvent),

    /// System latency measurement.
    #[serde(rename = "system.latency")]
    SystemLatency(SystemLatencyEvent),
}

// ─────────────────────────────────────────────────────────────────────────────
// MARKET DATA EVENTS
// ─────────────────────────────────────────────────────────────────────────────

/// A single price level in the order book.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub size: f64,
}

/// Level 1: top-of-book snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketL1Event {
    pub symbol: SymbolId,
    pub best_bid: PriceLevel,
    pub best_ask: PriceLevel,
    /// Mid price = (best_bid + best_ask) / 2
    pub mid_price: f64,
    /// Spread in ticks or absolute
    pub spread: f64,
    pub last_trade_price: Option<f64>,
    pub last_trade_size: Option<f64>,
}

/// Level 2: order book depth snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketL2Event {
    pub symbol: SymbolId,
    /// Bid levels, sorted best (highest) first.
    pub bids: Vec<PriceLevel>,
    /// Ask levels, sorted best (lowest) first.
    pub asks: Vec<PriceLevel>,
    /// Number of levels provided.
    pub depth: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// STRATEGY EVENTS
// ─────────────────────────────────────────────────────────────────────────────

/// Which direction and how much.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}

/// Strategy decision: "I want to trade."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDecisionEvent {
    pub decision_id: DecisionId,
    pub symbol: SymbolId,
    pub side: Side,
    pub desired_quantity: f64,
    /// Price at decision time (the "decision mid").
    pub decision_price: f64,
    /// Model that generated this decision (if any).
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    /// Confidence score from the model (0.0 - 1.0).
    pub confidence: Option<f64>,
    /// Human-readable reason.
    pub reason: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// RISK EVENTS
// ─────────────────────────────────────────────────────────────────────────────

/// Pre-trade risk check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskCheckEvent {
    pub decision_id: DecisionId,
    pub passed: bool,
    /// Which checks ran and their results.
    pub checks: Vec<RiskCheckDetail>,
    /// If rejected, why.
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskCheckDetail {
    pub check_name: String,
    pub passed: bool,
    pub value: f64,
    pub limit: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// ORDER EVENTS (FIX-style lifecycle)
// ─────────────────────────────────────────────────────────────────────────────

/// Order type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
}

/// Time in force.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good till cancel.
    GTC,
    /// Immediate or cancel.
    IOC,
    /// Fill or kill.
    FOK,
    /// Day order.
    Day,
}

/// Order submitted to venue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSubmittedEvent {
    pub decision_id: DecisionId,
    pub client_order_id: ClientOrderId,
    pub symbol: SymbolId,
    pub side: Side,
    pub order_type: OrderType,
    pub quantity: f64,
    pub limit_price: Option<f64>,
    pub time_in_force: TimeInForce,
}

/// Venue acknowledged the order (it's live on the book or accepted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAckEvent {
    pub client_order_id: ClientOrderId,
    pub order_id: OrderId,
    pub symbol: SymbolId,
}

/// Venue rejected the order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRejectedEvent {
    pub client_order_id: ClientOrderId,
    pub symbol: SymbolId,
    pub reason: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// FILL EVENTS
// ─────────────────────────────────────────────────────────────────────────────

/// Whether this is a partial or complete fill.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FillType {
    Partial,
    Full,
}

/// Maker or taker (liquidity role).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LiquidityRole {
    Maker,
    Taker,
    Unknown,
}

/// A trade fill event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeFillEvent {
    pub client_order_id: ClientOrderId,
    pub order_id: OrderId,
    pub exec_id: ExecId,
    pub symbol: SymbolId,
    pub side: Side,
    /// How much was filled in this event.
    pub fill_quantity: f64,
    /// Price of this fill.
    pub fill_price: f64,
    /// Running total filled so far.
    pub cumulative_quantity: f64,
    /// Remaining to fill.
    pub leaves_quantity: f64,
    pub fill_type: FillType,
    pub liquidity_role: LiquidityRole,
    /// Fee charged/rebated for this fill.
    pub fee: f64,
    pub fee_currency: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// SYSTEM EVENTS
// ─────────────────────────────────────────────────────────────────────────────

/// Latency measurement event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLatencyEvent {
    pub component: String,
    /// Latency in microseconds.
    pub latency_us: u64,
    pub percentile: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// ENVELOPE CONSTRUCTOR
// ─────────────────────────────────────────────────────────────────────────────

impl TelemetryEnvelope {
    /// Wrap an event in an envelope with the given trace and timestamps.
    pub fn new(trace_id: TraceId, timestamps: EventTimestamps, event: TelemetryEvent) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            created_at: Utc::now(),
            trace_id,
            timestamps,
            event,
        }
    }

    /// Serialize to a single JSON line (for JSONL format).
    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}
