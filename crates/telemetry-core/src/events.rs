use crate::enums::{ExecType, LiquidityRole, OrdStatus, OrdType, RiskCheckResult, Side, TimeInForce};
use crate::ids::{ClOrdId, DecisionId, ExecId, OrderId, Symbol, TraceId};
use crate::timestamps::Timestamp;
use crate::{timestamps, SCHEMA_VERSION};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Best bid/ask (Level 1) market data snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MarketL1 {
    pub bid_px: f64,
    pub bid_sz: u64,
    pub ask_px: f64,
    pub ask_sz: u64,
}

/// A single price level in the order book.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PriceLevel {
    pub px: f64,
    pub sz: u64,
    /// Number of orders at this price level.
    pub count: Option<u32>,
}

/// Level 2 order book snapshot with top N price levels.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MarketL2 {
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}

/// New order submission event.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrderNew {
    pub side: Side,
    pub ord_type: OrdType,
    pub order_qty: f64,
    /// Price for limit orders; None for market orders.
    pub price: Option<f64>,
    pub time_in_force: TimeInForce,
}

/// Order cancel request event.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrderCancel {
    pub side: Side,
    pub order_qty: f64,
}

/// Order replace (modify) request event.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrderReplace {
    pub side: Side,
    pub orig_cl_ord_id: ClOrdId,
    pub order_qty: f64,
    pub price: Option<f64>,
}

/// Exchange acknowledgment of an order action.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecAck {
    pub ord_status: OrdStatus,
    pub exec_type: ExecType,
    pub side: Side,
    pub order_qty: f64,
    pub cum_qty: f64,
    pub leaves_qty: f64,
}

/// Exchange rejection of an order action.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecReject {
    pub exec_type: ExecType,
    pub reject_reason: String,
    pub text: Option<String>,
}

/// Fill event (partial or full).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecFill {
    /// PARTIALLY_FILLED or FILLED.
    pub ord_status: OrdStatus,
    /// PARTIAL_FILL or FILL.
    pub exec_type: ExecType,
    pub side: Side,
    /// Fill quantity for this execution.
    pub last_qty: f64,
    /// Fill price for this execution.
    pub last_px: f64,
    pub order_qty: f64,
    /// Cumulative filled quantity.
    pub cum_qty: f64,
    /// Remaining open quantity.
    pub leaves_qty: f64,
    /// Volume-weighted average price of all fills.
    pub avg_px: f64,
    pub liquidity_role: LiquidityRole,
    pub trade_id: Option<String>,
}

/// Strategy signal/decision event.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StrategyDecision {
    /// Signal name, e.g. "MOMENTUM_LONG".
    pub signal: String,
    pub target_side: Side,
    pub target_qty: f64,
    /// Mid price at decision time.
    pub decision_price: f64,
    /// Model confidence score in [0.0, 1.0].
    pub confidence: Option<f64>,
    pub model_version: Option<String>,
}

/// Pre-trade risk check result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RiskCheck {
    pub result: RiskCheckResult,
    pub checks_passed: Vec<String>,
    pub checks_failed: Vec<String>,
    pub notional: f64,
    pub account_exposure: f64,
}

/// Internal latency measurement span.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SystemLatency {
    /// Name of the latency span, e.g. "decision_to_order_tx".
    pub span_name: String,
    /// Latency in microseconds.
    pub latency_us: u64,
    pub from_ts: Option<Timestamp>,
    pub to_ts: Option<Timestamp>,
}

/// Tagged enum of all telemetry event payloads.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event_type")]
pub enum TelemetryEvent {
    #[serde(rename = "cme.market.l1")]
    MarketL1(MarketL1),
    #[serde(rename = "cme.market.l2")]
    MarketL2(MarketL2),
    #[serde(rename = "cme.order.new")]
    OrderNew(OrderNew),
    #[serde(rename = "cme.order.cancel")]
    OrderCancel(OrderCancel),
    #[serde(rename = "cme.order.replace")]
    OrderReplace(OrderReplace),
    #[serde(rename = "cme.exec.ack")]
    ExecAck(ExecAck),
    #[serde(rename = "cme.exec.reject")]
    ExecReject(ExecReject),
    #[serde(rename = "cme.exec.fill")]
    ExecFill(ExecFill),
    #[serde(rename = "strategy.decision")]
    StrategyDecision(StrategyDecision),
    #[serde(rename = "risk.check")]
    RiskCheck(RiskCheck),
    #[serde(rename = "system.latency")]
    SystemLatency(SystemLatency),
}

/// Top-level telemetry envelope. Every JSONL line is one of these.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TelemetryEnvelope {
    pub schema_version: String,
    pub trace_id: TraceId,
    pub decision_id: Option<DecisionId>,
    pub cl_ord_id: Option<ClOrdId>,
    pub order_id: Option<OrderId>,
    pub exec_id: Option<ExecId>,
    pub symbol: Option<Symbol>,
    pub security_id: Option<u64>,
    pub ts_exchange: Option<Timestamp>,
    pub ts_local_rx: Timestamp,
    pub ts_decision: Option<Timestamp>,
    pub ts_order_tx: Option<Timestamp>,
    pub ts_ack_rx: Option<Timestamp>,
    pub ts_fill_rx: Option<Timestamp>,
    #[serde(flatten)]
    pub event: TelemetryEvent,
}

impl TelemetryEnvelope {
    /// Create a new envelope with the given trace ID and event payload.
    pub fn new(trace_id: TraceId, event: TelemetryEvent) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            trace_id,
            decision_id: None,
            cl_ord_id: None,
            order_id: None,
            exec_id: None,
            symbol: None,
            security_id: None,
            ts_exchange: None,
            ts_local_rx: timestamps::now(),
            ts_decision: None,
            ts_order_tx: None,
            ts_ack_rx: None,
            ts_fill_rx: None,
            event,
        }
    }

    /// Returns the event type string (matches the serde rename).
    pub fn event_type_str(&self) -> &'static str {
        match &self.event {
            TelemetryEvent::MarketL1(_) => "cme.market.l1",
            TelemetryEvent::MarketL2(_) => "cme.market.l2",
            TelemetryEvent::OrderNew(_) => "cme.order.new",
            TelemetryEvent::OrderCancel(_) => "cme.order.cancel",
            TelemetryEvent::OrderReplace(_) => "cme.order.replace",
            TelemetryEvent::ExecAck(_) => "cme.exec.ack",
            TelemetryEvent::ExecReject(_) => "cme.exec.reject",
            TelemetryEvent::ExecFill(_) => "cme.exec.fill",
            TelemetryEvent::StrategyDecision(_) => "strategy.decision",
            TelemetryEvent::RiskCheck(_) => "risk.check",
            TelemetryEvent::SystemLatency(_) => "system.latency",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::TraceId;

    fn make_l1_envelope() -> TelemetryEnvelope {
        TelemetryEnvelope::new(
            TraceId::new(),
            TelemetryEvent::MarketL1(MarketL1 {
                bid_px: 5799.75,
                bid_sz: 10,
                ask_px: 5800.00,
                ask_sz: 8,
            }),
        )
    }

    #[test]
    fn test_roundtrip_market_l1() {
        let env = make_l1_envelope();
        let json = serde_json::to_string(&env).unwrap();
        let decoded: TelemetryEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(env.trace_id, decoded.trace_id);
        assert_eq!(decoded.event_type_str(), "cme.market.l1");
    }

    #[test]
    fn test_roundtrip_all_event_types() {
        let trace = TraceId::new();

        let events: Vec<TelemetryEvent> = vec![
            TelemetryEvent::MarketL1(MarketL1 { bid_px: 1.0, bid_sz: 1, ask_px: 1.1, ask_sz: 1 }),
            TelemetryEvent::MarketL2(MarketL2 {
                bids: vec![PriceLevel { px: 1.0, sz: 10, count: Some(3) }],
                asks: vec![PriceLevel { px: 1.1, sz: 5, count: None }],
            }),
            TelemetryEvent::OrderNew(OrderNew {
                side: Side::Buy,
                ord_type: OrdType::Limit,
                order_qty: 10.0,
                price: Some(5800.0),
                time_in_force: TimeInForce::Day,
            }),
            TelemetryEvent::OrderCancel(OrderCancel { side: Side::Buy, order_qty: 10.0 }),
            TelemetryEvent::OrderReplace(OrderReplace {
                side: Side::Buy,
                orig_cl_ord_id: ClOrdId("orig-1".to_string()),
                order_qty: 5.0,
                price: Some(5801.0),
            }),
            TelemetryEvent::ExecAck(ExecAck {
                ord_status: OrdStatus::New,
                exec_type: ExecType::New,
                side: Side::Buy,
                order_qty: 10.0,
                cum_qty: 0.0,
                leaves_qty: 10.0,
            }),
            TelemetryEvent::ExecReject(ExecReject {
                exec_type: ExecType::Rejected,
                reject_reason: "RISK".to_string(),
                text: None,
            }),
            TelemetryEvent::ExecFill(ExecFill {
                ord_status: OrdStatus::Filled,
                exec_type: ExecType::Fill,
                side: Side::Buy,
                last_qty: 10.0,
                last_px: 5800.0,
                order_qty: 10.0,
                cum_qty: 10.0,
                leaves_qty: 0.0,
                avg_px: 5800.0,
                liquidity_role: LiquidityRole::Taker,
                trade_id: Some("T1".to_string()),
            }),
            TelemetryEvent::StrategyDecision(StrategyDecision {
                signal: "MOMENTUM_LONG".to_string(),
                target_side: Side::Buy,
                target_qty: 10.0,
                decision_price: 5799.875,
                confidence: Some(0.85),
                model_version: None,
            }),
            TelemetryEvent::RiskCheck(RiskCheck {
                result: RiskCheckResult::Pass,
                checks_passed: vec!["position_limit".to_string()],
                checks_failed: vec![],
                notional: 580000.0,
                account_exposure: 0.12,
            }),
            TelemetryEvent::SystemLatency(SystemLatency {
                span_name: "decision_to_ack".to_string(),
                latency_us: 1500,
                from_ts: None,
                to_ts: None,
            }),
        ];

        let expected_types = [
            "cme.market.l1", "cme.market.l2", "cme.order.new", "cme.order.cancel",
            "cme.order.replace", "cme.exec.ack", "cme.exec.reject", "cme.exec.fill",
            "strategy.decision", "risk.check", "system.latency",
        ];

        for (event, expected_type) in events.into_iter().zip(expected_types.iter()) {
            let env = TelemetryEnvelope::new(trace.clone(), event);
            assert_eq!(env.event_type_str(), *expected_type);
            let json = serde_json::to_string(&env).unwrap();
            let decoded: TelemetryEnvelope = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.event_type_str(), *expected_type);
        }
    }

    #[test]
    fn test_event_type_str() {
        let env = make_l1_envelope();
        assert_eq!(env.event_type_str(), "cme.market.l1");
    }
}
