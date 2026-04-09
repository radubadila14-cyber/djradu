// =============================================================================
// main.rs — Entry point for Radu's Trading System
// =============================================================================
// LEARNING NOTE (Radu):
//   This is where your program starts. The flow:
//
//   1. Load config from .env
//   2. Set up logging (tracing) + Prometheus metrics
//   3. Initialize telemetry pipeline (telemetry-core wired in)
//   4. Connect to PostgreSQL (Railway) — measure latency
//   5. Connect to Redis (Railway) — measure latency
//   6. Initialize database schema (including telemetry_events table)
//   7. Create the Quant API client
//   8. Emit startup telemetry, run!
// =============================================================================

mod cache;
mod config;
mod db;
mod journal;
mod models;
mod quant;
mod telemetry;

use std::time::Instant;

use anyhow::Result;
use metrics::{counter, describe_counter, describe_histogram, gauge};

#[tokio::main]
async fn main() -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // STEP 1: Logging + Prometheus metrics exporter
    // ─────────────────────────────────────────────────────────────────────
    tracing_subscriber::fmt::init();
    journal::print_banner();

    // Install Prometheus metrics recorder (serves on /metrics when HTTP added)
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    builder.install()?;

    // Describe metrics for professional observability
    describe_counter!("telemetry.events.emitted", "Total telemetry events emitted by type");
    describe_counter!("telemetry.events.rejected", "Total telemetry events rejected by validator");
    describe_histogram!("system.latency_us", "Latency in microseconds per component");
    describe_histogram!("market.spread", "Bid-ask spread per symbol");
    describe_counter!("system.connections", "Connection attempts per subsystem");

    // ─────────────────────────────────────────────────────────────────────
    // STEP 2: Load configuration
    // ─────────────────────────────────────────────────────────────────────
    let cfg = config::Config::from_env()?;
    tracing::info!("Configuration loaded");

    // ─────────────────────────────────────────────────────────────────────
    // STEP 3: Initialize telemetry pipeline (telemetry-core)
    // ─────────────────────────────────────────────────────────────────────
    let telem = telemetry::TelemetryPipeline::init("telemetry")?;
    tracing::info!(trace_id = %telem.trace_id(), "Telemetry pipeline active");

    // ─────────────────────────────────────────────────────────────────────
    // STEP 4: Connect PostgreSQL — measure connection latency
    // ─────────────────────────────────────────────────────────────────────
    let pg_start = Instant::now();
    let pg_pool = db::connect_postgres(&cfg.database_url).await?;
    telem.record_latency("postgres.connect", pg_start).await?;
    counter!("system.connections", "subsystem" => "postgres").increment(1);
    tracing::info!("PostgreSQL connected");

    // ─────────────────────────────────────────────────────────────────────
    // STEP 5: Initialize database schema (trades + market_data + telemetry)
    // ─────────────────────────────────────────────────────────────────────
    let schema_start = Instant::now();
    db::init_schema(&pg_pool).await?;
    telem.record_latency("postgres.schema_init", schema_start).await?;

    // ─────────────────────────────────────────────────────────────────────
    // STEP 6: Connect Redis — measure connection latency
    // ─────────────────────────────────────────────────────────────────────
    let redis_start = Instant::now();
    let mut redis_conn = cache::connect_redis(&cfg.redis_url).await?;
    telem.record_latency("redis.connect", redis_start).await?;
    counter!("system.connections", "subsystem" => "redis").increment(1);

    let health_start = Instant::now();
    cache::health_check(&mut redis_conn).await?;
    telem.record_latency("redis.health_check", health_start).await?;
    tracing::info!("Redis connected and healthy");

    // ─────────────────────────────────────────────────────────────────────
    // STEP 7: Set up Quant API client
    // ─────────────────────────────────────────────────────────────────────
    let _quant = quant::QuantClient::new(&cfg.quant_api_base_url, &cfg.quant_api_key);
    tracing::info!("Quant API client ready");

    // ─────────────────────────────────────────────────────────────────────
    // STEP 8: ALL SYSTEMS GO — emit telemetry for startup
    // ─────────────────────────────────────────────────────────────────────
    gauge!("system.up").set(1.0);
    tracing::info!("==============================================");
    tracing::info!("  ALL SYSTEMS GO!");
    tracing::info!("  PostgreSQL:  CONNECTED");
    tracing::info!("  Redis:       CONNECTED");
    tracing::info!("  Quant API:   READY");
    tracing::info!("  Telemetry:   ACTIVE (trace {})", telem.trace_id());
    if cfg.lambda_labs_api_key.is_some() {
        tracing::info!("  Lambda Labs: API KEY SET");
    } else {
        tracing::info!("  Lambda Labs: NOT CONFIGURED (Phase 3)");
    }
    tracing::info!("==============================================");

    // ─────────────────────────────────────────────────────────────────────
    // STEP 9: Demo — cache a price and emit L1 telemetry
    // ─────────────────────────────────────────────────────────────────────
    let cache_start = Instant::now();
    cache::cache_price(&mut redis_conn, "BTC/USD", 65000.0).await?;
    telem.record_latency("redis.cache_price", cache_start).await?;

    // Emit a professional L1 market data event
    telem
        .emit_market_l1("BTC/USD", 64990.0, 1.5, 65010.0, 2.0)
        .await?;

    let cached = cache::get_cached_price(&mut redis_conn, "BTC/USD").await?;
    tracing::info!("Cached BTC/USD price: {:?}", cached);

    // ─────────────────────────────────────────────────────────────────────
    // STEP 10: Persist telemetry events to PostgreSQL
    // ─────────────────────────────────────────────────────────────────────
    telem.flush().await?;

    let (written, rejected, uptime) = telem.stats().await;
    tracing::info!(
        written,
        rejected,
        uptime_secs = format!("{:.2}", uptime),
        "Telemetry pipeline stats"
    );

    // ─────────────────────────────────────────────────────────────────────
    // RUN — keep alive until Ctrl+C
    // ─────────────────────────────────────────────────────────────────────
    tracing::info!("Trading system is running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;

    // Graceful shutdown — flush remaining events
    gauge!("system.up").set(0.0);
    telem.flush().await?;
    tracing::info!("Shutdown complete. All telemetry flushed.");

    Ok(())
}
