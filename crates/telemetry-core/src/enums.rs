use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Order side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Side {
    Buy,
    Sell,
}

/// Order type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrdType {
    Market,
    Limit,
}

/// Time in force for orders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeInForce {
    Day,
    Ioc,
    Fok,
    Gtc,
}

/// Order status (FIX 39).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrdStatus {
    PendingNew,
    New,
    PartiallyFilled,
    Filled,
    PendingCancel,
    Canceled,
    Rejected,
}

/// Execution type (FIX 150).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecType {
    PendingNew,
    New,
    PartialFill,
    Fill,
    PendingCancel,
    Canceled,
    Rejected,
    Replaced,
}

/// Result of a pre-trade risk check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskCheckResult {
    Pass,
    Fail,
}

/// Liquidity role for a fill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiquidityRole {
    Maker,
    Taker,
}
