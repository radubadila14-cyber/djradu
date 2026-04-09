// =============================================================================
// db.rs — PostgreSQL connection pool
// =============================================================================
// LEARNING NOTE (Radu):
//   A "connection pool" is a set of reusable database connections.
//   Instead of opening a new connection for every query (slow!), we keep
//   a pool of connections ready to go. Think of it like a taxi stand —
//   taxis wait there, you grab one, use it, and return it.
//
//   SQLx is the Rust crate we use. It's "async" — meaning it doesn't block
//   the program while waiting for the database to respond.
// =============================================================================

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Create a PostgreSQL connection pool.
///
/// `database_url` comes from your Railway DATABASE_URL.
/// Returns a pool that can be shared across the entire application.
pub async fn connect_postgres(database_url: &str) -> Result<PgPool> {
    // LEARNING: `await` means "wait for this async operation to finish"
    // `.context()` adds a human-readable error message if it fails
    let pool = PgPoolOptions::new()
        .max_connections(5) // Max 5 simultaneous connections
        .connect(database_url)
        .await
        .context("Failed to connect to PostgreSQL on Railway. Check your DATABASE_URL!")?;

    tracing::info!("Connected to PostgreSQL successfully!");

    // Run a quick test query to make sure the connection works
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .context("PostgreSQL is connected but can't execute queries")?;

    tracing::info!("PostgreSQL health check passed!");

    Ok(pool)
}

/// Initialize the database schema.
/// Creates the tables we need if they don't exist yet.
///
/// LEARNING NOTE (Radu):
/// PostgreSQL doesn't allow multiple CREATE TABLE statements in one
/// prepared query. So we run each one separately. This is normal.
pub async fn init_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS trades (
            id          BIGSERIAL PRIMARY KEY,
            symbol      VARCHAR(20) NOT NULL,
            side        VARCHAR(4) NOT NULL CHECK (side IN ('BUY', 'SELL')),
            quantity     DOUBLE PRECISION NOT NULL,
            price       DOUBLE PRECISION NOT NULL,
            status      VARCHAR(20) NOT NULL DEFAULT 'PENDING',
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create trades table")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS market_data (
            id          BIGSERIAL PRIMARY KEY,
            symbol      VARCHAR(20) NOT NULL,
            price       DOUBLE PRECISION NOT NULL,
            volume      DOUBLE PRECISION NOT NULL,
            timestamp   TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create market_data table")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS model_configs (
            id          BIGSERIAL PRIMARY KEY,
            name        VARCHAR(100) NOT NULL,
            parameters  JSONB NOT NULL DEFAULT '{}',
            is_active   BOOLEAN NOT NULL DEFAULT false,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create model_configs table")?;

    tracing::info!("Database schema initialized!");
    Ok(())
}
