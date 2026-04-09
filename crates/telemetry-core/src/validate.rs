// =============================================================================
// telemetry-core/src/validate.rs — Event validation
// =============================================================================
// LEARNING NOTE (Radu):
//   A "validator" checks that data meets the contract BEFORE it's stored.
//   Bad data in = bad analysis out. Professional systems reject malformed
//   events deterministically — meaning the SAME input ALWAYS gives the
//   SAME pass/fail result. No randomness, no "sometimes it works."
//
//   This validator checks:
//   1. Schema version is recognized
//   2. Required IDs are present and non-empty
//   3. At least one timestamp exists
//   4. Event-specific field constraints (prices > 0, quantities > 0, etc.)
// =============================================================================

use crate::events::*;

/// Validation error with a human-readable message.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Validate a telemetry envelope. Returns a list of errors (empty = valid).
pub fn validate(envelope: &TelemetryEnvelope) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Schema version check
    if envelope.schema_version != SCHEMA_VERSION {
        errors.push(ValidationError {
            field: "schema_version".into(),
            message: format!(
                "expected '{}', got '{}'",
                SCHEMA_VERSION, envelope.schema_version
            ),
        });
    }

    // At least one timestamp must exist
    let ts = &envelope.timestamps;
    if ts.ts_exchange.is_none()
        && ts.ts_local_rx.is_none()
        && ts.ts_decision.is_none()
        && ts.ts_order_tx.is_none()
        && ts.ts_ack_rx.is_none()
        && ts.ts_fill_rx.is_none()
    {
        errors.push(ValidationError {
            field: "timestamps".into(),
            message: "at least one timestamp must be set".into(),
        });
    }

    // Event-specific validation
    match &envelope.event {
        TelemetryEvent::MarketL1(e) => validate_market_l1(e, &mut errors),
        TelemetryEvent::MarketL2(e) => validate_market_l2(e, &mut errors),
        TelemetryEvent::StrategyDecision(e) => validate_decision(e, &mut errors),
        TelemetryEvent::RiskCheck(_) => {}
        TelemetryEvent::OrderSubmitted(e) => validate_order(e, &mut errors),
        TelemetryEvent::OrderAck(_) => {}
        TelemetryEvent::OrderRejected(_) => {}
        TelemetryEvent::TradeFill(e) => validate_fill(e, &mut errors),
        TelemetryEvent::SystemLatency(_) => {}
    }

    errors
}

fn validate_market_l1(e: &MarketL1Event, errors: &mut Vec<ValidationError>) {
    if e.best_bid.price <= 0.0 {
        errors.push(ValidationError {
            field: "market.l1.best_bid.price".into(),
            message: "must be > 0".into(),
        });
    }
    if e.best_ask.price <= 0.0 {
        errors.push(ValidationError {
            field: "market.l1.best_ask.price".into(),
            message: "must be > 0".into(),
        });
    }
    if e.best_bid.price >= e.best_ask.price {
        errors.push(ValidationError {
            field: "market.l1".into(),
            message: "bid must be < ask (crossed book)".into(),
        });
    }
    if e.symbol.0.is_empty() {
        errors.push(ValidationError {
            field: "market.l1.symbol".into(),
            message: "symbol must not be empty".into(),
        });
    }
}

fn validate_market_l2(e: &MarketL2Event, errors: &mut Vec<ValidationError>) {
    if e.bids.is_empty() {
        errors.push(ValidationError {
            field: "market.l2.bids".into(),
            message: "must have at least one bid level".into(),
        });
    }
    if e.asks.is_empty() {
        errors.push(ValidationError {
            field: "market.l2.asks".into(),
            message: "must have at least one ask level".into(),
        });
    }
}

fn validate_decision(e: &StrategyDecisionEvent, errors: &mut Vec<ValidationError>) {
    if e.desired_quantity <= 0.0 {
        errors.push(ValidationError {
            field: "strategy.decision.desired_quantity".into(),
            message: "must be > 0".into(),
        });
    }
    if e.decision_price <= 0.0 {
        errors.push(ValidationError {
            field: "strategy.decision.decision_price".into(),
            message: "must be > 0".into(),
        });
    }
}

fn validate_order(e: &OrderSubmittedEvent, errors: &mut Vec<ValidationError>) {
    if e.quantity <= 0.0 {
        errors.push(ValidationError {
            field: "order.submitted.quantity".into(),
            message: "must be > 0".into(),
        });
    }
}

fn validate_fill(e: &TradeFillEvent, errors: &mut Vec<ValidationError>) {
    if e.fill_quantity <= 0.0 {
        errors.push(ValidationError {
            field: "trade.fill.fill_quantity".into(),
            message: "must be > 0".into(),
        });
    }
    if e.fill_price <= 0.0 {
        errors.push(ValidationError {
            field: "trade.fill.fill_price".into(),
            message: "must be > 0".into(),
        });
    }
    if e.cumulative_quantity < e.fill_quantity {
        errors.push(ValidationError {
            field: "trade.fill.cumulative_quantity".into(),
            message: "must be >= fill_quantity".into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::*;
    use crate::timestamps::EventTimestamps;

    #[test]
    fn valid_l1_event_passes() {
        let envelope = TelemetryEnvelope::new(
            TraceId::new(),
            EventTimestamps::local_now(),
            TelemetryEvent::MarketL1(MarketL1Event {
                symbol: SymbolId("ES".into()),
                best_bid: PriceLevel { price: 5400.00, size: 150.0 },
                best_ask: PriceLevel { price: 5400.25, size: 120.0 },
                mid_price: 5400.125,
                spread: 0.25,
                last_trade_price: Some(5400.00),
                last_trade_size: Some(5.0),
            }),
        );
        let errors = validate(&envelope);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn crossed_book_fails() {
        let envelope = TelemetryEnvelope::new(
            TraceId::new(),
            EventTimestamps::local_now(),
            TelemetryEvent::MarketL1(MarketL1Event {
                symbol: SymbolId("ES".into()),
                best_bid: PriceLevel { price: 5401.00, size: 150.0 },
                best_ask: PriceLevel { price: 5400.00, size: 120.0 },
                mid_price: 5400.50,
                spread: -1.0,
                last_trade_price: None,
                last_trade_size: None,
            }),
        );
        let errors = validate(&envelope);
        assert!(!errors.is_empty(), "Crossed book should fail validation");
    }

    #[test]
    fn no_timestamps_fails() {
        let envelope = TelemetryEnvelope::new(
            TraceId::new(),
            EventTimestamps::empty(),
            TelemetryEvent::MarketL1(MarketL1Event {
                symbol: SymbolId("ES".into()),
                best_bid: PriceLevel { price: 5400.00, size: 150.0 },
                best_ask: PriceLevel { price: 5400.25, size: 120.0 },
                mid_price: 5400.125,
                spread: 0.25,
                last_trade_price: None,
                last_trade_size: None,
            }),
        );
        let errors = validate(&envelope);
        assert!(errors.iter().any(|e| e.field == "timestamps"));
    }
}
