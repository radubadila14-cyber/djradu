// =============================================================================
// main.rs — Entry point for Radu's Trading System
// =============================================================================
// LEARNING NOTE (Radu):
//   This is where your program starts. Every Rust program needs a `main()`
//   function. Here's the flow:
//
//   1. Load config from .env
//   2. Set up logging (tracing)
//   3. Connect to PostgreSQL (Railway)
//   4. Connect to Redis (Railway)
//   5. Initialize database tables
//   6. Create the Quant API client
//   7. Run!
//
//   The `#[tokio::main]` attribute transforms `main()` into an async
//   function. Without it, you can't use `await` in main.
//
//   `mod` declarations tell Rust "this module exists, load it."
//   Each `mod xyz;` looks for either `src/xyz.rs` or `src/xyz/mod.rs`.
// =============================================================================

mod cache;
mod config;
mod db;
mod journal;
mod models;
mod quant;
#[cfg(feature = "telemetry-integration")]
mod telemetry;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // -------------------------------------------------------------------------
    // STEP 1: Initialize logging
    // -------------------------------------------------------------------------
    // LEARNING: This sets up `tracing` so when we write `tracing::info!("...")`
    // it actually prints to the terminal. Without this, log messages are silent.
    tracing_subscriber::fmt::init();

    // Print the project banner (from journal.rs)
    journal::print_banner();

    // -------------------------------------------------------------------------
    // STEP 2: Load configuration
    // -------------------------------------------------------------------------
    // LEARNING: `?` at the end means "if this fails, return the error."
    // It's shorthand for a match statement that handles Ok/Err.
    let cfg = config::Config::from_env()?;
    tracing::info!("Configuration loaded!");

    // -------------------------------------------------------------------------
    // STEP 3: Connect to PostgreSQL on Railway
    // -------------------------------------------------------------------------
    // LEARNING: `.await` pauses here until the connection is established.
    // The program doesn't freeze though — Tokio can run other tasks meanwhile.
    let pg_pool = db::connect_postgres(&cfg.database_url).await?;
    tracing::info!("PostgreSQL connected!");

    // -------------------------------------------------------------------------
    // STEP 4: Initialize database schema (create tables if needed)
    // -------------------------------------------------------------------------
    db::init_schema(&pg_pool).await?;

    // -------------------------------------------------------------------------
    // STEP 5: Connect to Redis on Railway
    // -------------------------------------------------------------------------
    let mut redis_conn = cache::connect_redis(&cfg.redis_url).await?;
    cache::health_check(&mut redis_conn).await?;
    tracing::info!("Redis connected and healthy!");

    // -------------------------------------------------------------------------
    // STEP 6: Set up Quant API client
    // -------------------------------------------------------------------------
    let _quant = quant::QuantClient::new(&cfg.quant_api_base_url, &cfg.quant_api_key);
    tracing::info!("Quant API client ready!");

    // -------------------------------------------------------------------------
    // STEP 7: System ready!
    // -------------------------------------------------------------------------
    tracing::info!("==============================================");
    tracing::info!("  ALL SYSTEMS GO!");
    tracing::info!("  PostgreSQL: CONNECTED");
    tracing::info!("  Redis: CONNECTED");
    tracing::info!("  Quant API: READY");
    if cfg.lambda_labs_api_key.is_some() {
        tracing::info!("  Lambda Labs: API KEY SET");
    } else {
        tracing::info!("  Lambda Labs: NOT CONFIGURED (Phase 3)");
    }
    tracing::info!("==============================================");

    // -------------------------------------------------------------------------
    // DEMO: Test caching a price
    // -------------------------------------------------------------------------
    cache::cache_price(&mut redis_conn, "BTC/USD", 65000.0).await?;
    let cached = cache::get_cached_price(&mut redis_conn, "BTC/USD").await?;
    tracing::info!("Cached BTC/USD price: {:?}", cached);

    // Keep the application running
    tracing::info!("Trading system is running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down gracefully...");

    Ok(())
}
