// =============================================================================
// models.rs — Data structures for the trading system
// =============================================================================
// LEARNING NOTE (Radu):
//   In Rust, we define data structures using `struct`. Think of a struct like
//   a blueprint for an object. Unlike Python dicts or JS objects, Rust structs
//   are TYPED — every field has a specific type, and the compiler enforces it.
//
//   `#[derive(...)]` is an "attribute macro" — it auto-generates code:
//     - Debug: lets you print the struct with {:?}
//     - Clone: lets you copy the struct
//     - Serialize/Deserialize: converts to/from JSON
//     - sqlx::FromRow: lets SQLx map database rows to this struct
// =============================================================================

use serde::{Deserialize, Serialize};

/// Represents a trade order in the system.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct Trade {
    pub id: i64,
    pub symbol: String,
    pub side: String, // "BUY" or "SELL"
    pub quantity: f64,
    pub price: f64,
    pub status: String, // "PENDING", "FILLED", "CANCELLED"
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// For creating a new trade (no id or timestamps yet).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct NewTrade {
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
}

/// Market data snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct MarketData {
    pub id: i64,
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// ML model configuration.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct ModelConfig {
    pub id: i64,
    pub name: String,
    pub parameters: serde_json::Value, // JSONB maps to serde_json::Value
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
