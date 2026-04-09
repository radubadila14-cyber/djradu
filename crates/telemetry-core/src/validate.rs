use crate::events::TelemetryEnvelope;
use anyhow::{anyhow, Result};
use jsonschema::JSONSchema;
use schemars::schema_for;
use serde_json::Value;
use std::sync::OnceLock;

static COMPILED_SCHEMA: OnceLock<JSONSchema> = OnceLock::new();

fn get_schema() -> &'static JSONSchema {
    COMPILED_SCHEMA.get_or_init(|| {
        let schema = schema_for!(TelemetryEnvelope);
        let schema_value = serde_json::to_value(&schema).expect("schema serialization failed");
        JSONSchema::compile(&schema_value).expect("schema compilation failed")
    })
}

/// Validate a `TelemetryEnvelope` against the canonical JSON Schema.
pub fn validate_envelope(envelope: &TelemetryEnvelope) -> Result<()> {
    let value = serde_json::to_value(envelope)?;
    let schema = get_schema();
    schema.validate(&value).map_err(|errors| {
        let msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
        anyhow!("Validation failed: {}", msgs.join("; "))
    })
}

/// Validate a raw JSON value against the canonical JSON Schema.
pub fn validate_value(value: &Value) -> Result<()> {
    let schema = get_schema();
    schema.validate(value).map_err(|errors| {
        let msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
        anyhow!("Validation failed: {}", msgs.join("; "))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{MarketL1, TelemetryEvent, TelemetryEnvelope};
    use crate::ids::TraceId;

    #[test]
    fn test_valid_envelope_passes() {
        let env = TelemetryEnvelope::new(
            TraceId::new(),
            TelemetryEvent::MarketL1(MarketL1 {
                bid_px: 5799.75,
                bid_sz: 10,
                ask_px: 5800.00,
                ask_sz: 8,
            }),
        );
        assert!(validate_envelope(&env).is_ok());
    }

    #[test]
    fn test_malformed_json_fails() {
        let bad: Value = serde_json::json!({ "not_valid": true });
        assert!(validate_value(&bad).is_err());
    }
}
