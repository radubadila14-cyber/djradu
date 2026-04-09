// =============================================================================
// config.rs — Loads environment variables into a typed Config struct
// =============================================================================
// WHY THIS EXISTS (Radu's learning note):
//   In Rust, we don't just read env vars everywhere like in Python/JS.
//   We load them ONCE at startup into a struct, which gives us:
//   1. Type safety — the compiler checks we use them correctly
//   2. Single source of truth — one place to see all config
//   3. Fail fast — if a var is missing, we crash at startup, not mid-trade!
// =============================================================================

use anyhow::{Context, Result};

/// App configuration loaded from environment variables.
///
/// Each field maps to an environment variable.
/// `Clone` lets us copy this struct around.
/// `Debug` lets us print it for debugging.
#[derive(Debug, Clone)]
pub struct Config {
    /// PostgreSQL connection string from Railway
    pub database_url: String,
    /// Redis connection string from Railway
    pub redis_url: String,
    /// Quant trading API key
    pub quant_api_key: String,
    /// Quant trading API base URL
    pub quant_api_base_url: String,
    /// Lambda Labs API key (for model training)
    pub lambda_labs_api_key: Option<String>,
    /// Host to bind the app to
    pub app_host: String,
    /// Port to bind the app to
    pub app_port: u16,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// LEARNING NOTE: `Result<Config>` means this function can either:
    ///   - Succeed and return a Config (Ok(config))
    ///   - Fail and return an error (Err(e))
    /// The `?` operator at the end of lines propagates errors upward.
    pub fn from_env() -> Result<Config> {
        // Load .env file. The `ok()` means we don't crash if .env is missing
        // (in production, Railway sets env vars directly).
        dotenvy::dotenv().ok();

        Ok(Config {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set — get it from Railway PostgreSQL")?,
            redis_url: std::env::var("REDIS_URL")
                .context("REDIS_URL must be set — get it from Railway Redis")?,
            quant_api_key: std::env::var("QUANT_API_KEY")
                .context("QUANT_API_KEY must be set")?,
            quant_api_base_url: std::env::var("QUANT_API_BASE_URL")
                .unwrap_or_else(|_| "https://api.example.com/v1".to_string()),
            lambda_labs_api_key: std::env::var("LAMBDA_LABS_API_KEY").ok(),
            // Railway requires 0.0.0.0; locally default to 127.0.0.1
            app_host: std::env::var("APP_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            // Railway sets PORT; fall back to APP_PORT or 8080
            app_port: std::env::var("PORT")
                .or_else(|_| std::env::var("APP_PORT"))
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("APP_PORT must be a valid number")?,
        })
    }
}
