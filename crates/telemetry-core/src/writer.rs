// =============================================================================
// telemetry-core/src/writer.rs — JSONL event log writer
// =============================================================================
// LEARNING NOTE (Radu):
//   JSONL = "JSON Lines" — one JSON object per line, no wrapping array.
//   This is the industry standard for event logs because:
//   1. You can append without reading the whole file
//   2. Each line is independently parseable (one bad line doesn't ruin the file)
//   3. Tools like grep, jq, and streaming processors handle it natively
//   4. You can tail it in real-time (like watching a live feed)
//
//   Example JSONL:
//     {"type":"market.l1","symbol":"ES","bid":5400.00,...}
//     {"type":"strategy.decision","side":"Buy",...}
//     {"type":"order.submitted","quantity":5,...}
// =============================================================================

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::events::TelemetryEnvelope;
use crate::validate;

/// Writes telemetry events to a JSONL file.
/// Validates each event before writing — bad events are rejected.
pub struct JsonlWriter {
    writer: BufWriter<File>,
    events_written: u64,
    events_rejected: u64,
}

impl JsonlWriter {
    /// Open or create a JSONL file for writing.
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(Self {
            writer: BufWriter::new(file),
            events_written: 0,
            events_rejected: 0,
        })
    }

    /// Write a validated event. Returns Ok(true) if written, Ok(false) if rejected.
    pub fn write(&mut self, envelope: &TelemetryEnvelope) -> anyhow::Result<bool> {
        let errors = validate::validate(envelope);

        if !errors.is_empty() {
            self.events_rejected += 1;
            return Ok(false);
        }

        let line = serde_json::to_string(envelope)?;
        writeln!(self.writer, "{}", line)?;
        self.events_written += 1;

        Ok(true)
    }

    /// Flush buffered writes to disk.
    pub fn flush(&mut self) -> anyhow::Result<()> {
        self.writer.flush()?;
        Ok(())
    }

    /// How many events were successfully written.
    pub fn events_written(&self) -> u64 {
        self.events_written
    }

    /// How many events were rejected by validation.
    pub fn events_rejected(&self) -> u64 {
        self.events_rejected
    }
}
