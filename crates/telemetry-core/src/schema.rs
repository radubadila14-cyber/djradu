use anyhow::Result;
use schemars::schema_for;
use std::path::Path;
use crate::events::TelemetryEnvelope;

/// Generate JSON Schema for `TelemetryEnvelope` and write to `out_dir/telemetry_envelope.json`.
pub fn generate_schemas(out_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(out_dir)?;
    let schema = schema_for!(TelemetryEnvelope);
    let json = serde_json::to_string_pretty(&schema)?;
    let path = out_dir.join("telemetry_envelope.json");
    std::fs::write(&path, json)?;
    eprintln!("Generated schema: {}", path.display());
    Ok(())
}
