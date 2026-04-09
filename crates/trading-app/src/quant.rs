// =============================================================================
// quant.rs — Quantitative Trading API Client
// =============================================================================
// LEARNING NOTE (Radu):
//   This module is a "client" — it wraps HTTP calls to an external API.
//   In Rust, we use `reqwest` for HTTP (like `requests` in Python or
//   `fetch` in JavaScript).
//
//   The `impl` block defines methods on the struct.
//   `&self` means the method borrows the struct (reads it without taking
//   ownership). This is a core Rust concept — ownership & borrowing.
// =============================================================================

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Client for the Quant trading API.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used via methods that will be called in future phases
pub struct QuantClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

/// Response from a market quote API.
#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct QuoteResponse {
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub timestamp: String,
}

impl QuantClient {
    /// Create a new Quant API client.
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
        }
    }

    /// Fetch a market quote for a given symbol.
    ///
    /// LEARNING: `async fn` means this function doesn't block while waiting
    /// for the HTTP response. The program can do other work in the meantime.
    #[allow(dead_code)]
    pub async fn get_quote(&self, symbol: &str) -> Result<QuoteResponse> {
        let url = format!("{}/quote/{}", self.base_url, symbol);

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .context(format!("Failed to fetch quote for {}", symbol))?;

        // LEARNING: We check if the API returned an error status (4xx, 5xx)
        let response = response
            .error_for_status()
            .context("Quant API returned an error")?;

        let quote: QuoteResponse = response
            .json()
            .await
            .context("Failed to parse quote response as JSON")?;

        tracing::info!(symbol = symbol, price = quote.price, "Fetched quote");
        Ok(quote)
    }

    /// Check if the Quant API is reachable.
    #[allow(dead_code)]
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        match self.http.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}
