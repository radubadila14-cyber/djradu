/// UTC timestamp type used throughout the telemetry system.
pub type Timestamp = chrono::DateTime<chrono::Utc>;

/// Returns the current UTC timestamp.
pub fn now() -> Timestamp {
    chrono::Utc::now()
}
