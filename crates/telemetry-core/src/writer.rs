use crate::events::TelemetryEnvelope;
use crate::validate;
use anyhow::Result;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Controls how aggressively writes are synced to disk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncPolicy {
    /// No explicit flush or sync.
    None,
    /// Flush the user-space buffer after each write.
    Flush,
    /// Flush and call `fsync` after each write (highest durability).
    Fsync,
}

/// Configuration for a `JsonlWriter`.
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Base file path (e.g. "telemetry/events.jsonl").
    pub path: PathBuf,
    /// Rotate file when it exceeds this many bytes (`None` = no rotation).
    pub max_bytes: Option<u64>,
    /// How aggressively to sync to disk.
    pub sync_policy: SyncPolicy,
    /// Validate events before writing; reject on validation failure.
    pub validate: bool,
}

impl WriterConfig {
    /// Create a config with sensible defaults for the given path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            max_bytes: Some(100 * 1024 * 1024), // 100 MB
            sync_policy: SyncPolicy::Flush,
            validate: true,
        }
    }
}

/// Writes `TelemetryEnvelope` events as JSONL with optional rotation and validation.
pub struct JsonlWriter {
    config: WriterConfig,
    writer: BufWriter<File>,
    current_bytes: u64,
    rotation_index: u32,
    events_written: u64,
    events_rejected: u64,
}

impl JsonlWriter {
    /// Open (or create) a JSONL writer at the given path with default config.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Self::with_config(WriterConfig::new(path.as_ref()))
    }

    /// Open (or create) a JSONL writer with explicit config.
    pub fn with_config(config: WriterConfig) -> Result<Self> {
        if let Some(parent) = config.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = open_append(&config.path)?;
        let current_bytes = file.metadata()?.len();
        Ok(Self {
            config,
            writer: BufWriter::new(file),
            current_bytes,
            rotation_index: 0,
            events_written: 0,
            events_rejected: 0,
        })
    }

    /// Write an envelope. Returns `true` if written, `false` if rejected by validation.
    pub fn write(&mut self, envelope: &TelemetryEnvelope) -> Result<bool> {
        if self.config.validate {
            if let Err(e) = validate::validate_envelope(envelope) {
                eprintln!("[telemetry-core] Rejected: {}", e);
                self.events_rejected += 1;
                return Ok(false);
            }
        }

        let mut line = serde_json::to_string(envelope)?;
        line.push('\n');
        let bytes = line.len() as u64;

        if let Some(max) = self.config.max_bytes {
            if self.current_bytes + bytes > max {
                self.rotate()?;
            }
        }

        self.writer.write_all(line.as_bytes())?;
        self.current_bytes += bytes;
        self.events_written += 1;

        match self.config.sync_policy {
            SyncPolicy::None => {}
            SyncPolicy::Flush => {
                self.writer.flush()?;
            }
            SyncPolicy::Fsync => {
                self.writer.flush()?;
                self.writer.get_ref().sync_all()?;
            }
        }

        Ok(true)
    }

    /// Flush the write buffer.
    pub fn flush(&mut self) -> Result<()> {
        Ok(self.writer.flush()?)
    }

    /// Number of events successfully written.
    pub fn events_written(&self) -> u64 {
        self.events_written
    }

    /// Number of events rejected by validation.
    pub fn events_rejected(&self) -> u64 {
        self.events_rejected
    }

    fn rotate(&mut self) -> Result<()> {
        self.writer.flush()?;
        self.rotation_index += 1;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let stem = self
            .config
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("events");
        let ext = self
            .config
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jsonl");
        let parent = self.config.path.parent().unwrap_or(Path::new("."));
        let rotated = parent.join(format!(
            "{}.{}.{:04}.{}",
            stem, ts, self.rotation_index, ext
        ));
        std::fs::rename(&self.config.path, &rotated)?;
        eprintln!("[telemetry-core] Rotated log to {}", rotated.display());
        let file = open_append(&self.config.path)?;
        self.writer = BufWriter::new(file);
        self.current_bytes = 0;
        Ok(())
    }
}

fn open_append(path: &Path) -> Result<File> {
    Ok(OpenOptions::new().create(true).append(true).open(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{MarketL1, TelemetryEvent, TelemetryEnvelope};
    use crate::ids::TraceId;
    use std::io::{BufRead, BufReader};

    #[test]
    fn test_write_and_read_back() {
        let dir = PathBuf::from("telemetry_writer_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_events.jsonl");
        let _ = std::fs::remove_file(&path);

        let mut config = WriterConfig::new(&path);
        config.validate = false;
        config.sync_policy = SyncPolicy::None;

        let mut writer = JsonlWriter::with_config(config).unwrap();

        for _ in 0..3 {
            let env = TelemetryEnvelope::new(
                TraceId::new(),
                TelemetryEvent::MarketL1(MarketL1 {
                    bid_px: 5799.75,
                    bid_sz: 10,
                    ask_px: 5800.0,
                    ask_sz: 5,
                }),
            );
            assert!(writer.write(&env).unwrap());
        }
        writer.flush().unwrap();
        assert_eq!(writer.events_written(), 3);
        assert_eq!(writer.events_rejected(), 0);

        let file = File::open(&path).unwrap();
        let lines: Vec<_> = BufReader::new(file).lines().collect();
        assert_eq!(lines.len(), 3);
        for line in lines {
            let _env: TelemetryEnvelope = serde_json::from_str(&line.unwrap()).unwrap();
        }

        std::fs::remove_file(&path).unwrap();
        let _ = std::fs::remove_dir(&dir);
    }
}
