// =============================================================================
// cache.rs — Redis connection and caching utilities
// =============================================================================
// LEARNING NOTE (Radu):
//   Redis is an in-memory key-value store. It's FAST — way faster than
//   PostgreSQL for simple reads. We use it for:
//   1. Caching — store frequently accessed data (like current prices)
//   2. Pub/Sub — real-time event broadcasting (e.g., "new trade executed!")
//   3. Rate limiting — track API call counts
//
//   Think of Redis as a super-fast sticky note board, while PostgreSQL
//   is the filing cabinet (permanent, structured, but slower).
// =============================================================================

use anyhow::{Context, Result};
use redis::AsyncCommands;

/// Create a Redis connection.
///
/// `redis_url` comes from your Railway REDIS_URL.
pub async fn connect_redis(redis_url: &str) -> Result<redis::aio::MultiplexedConnection> {
    // LEARNING: `parse()` converts the URL string into a Redis connection config.
    let client = redis::Client::open(redis_url)
        .context("Invalid REDIS_URL — check your Railway Redis connection string")?;

    let conn = client
        .get_multiplexed_async_connection()
        .await
        .context("Failed to connect to Redis on Railway. Check your REDIS_URL!")?;

    tracing::info!("Connected to Redis successfully!");

    Ok(conn)
}

/// Test the Redis connection by setting and getting a value.
pub async fn health_check(conn: &mut redis::aio::MultiplexedConnection) -> Result<()> {
    // LEARNING: Redis stores data as key-value pairs.
    // Here we SET a key "health_check" to "ok", then GET it back.
    let _: () = conn
        .set_ex("radu:health_check", "ok", 60) // expires in 60 seconds
        .await
        .context("Redis SET failed")?;

    let value: String = conn
        .get("radu:health_check")
        .await
        .context("Redis GET failed")?;

    if value == "ok" {
        tracing::info!("Redis health check passed!");
    }

    Ok(())
}

/// Cache a market price in Redis with expiration.
///
/// LEARNING: We prefix keys with "radu:" to namespace our data.
/// This prevents collisions if multiple apps share the same Redis.
pub async fn cache_price(
    conn: &mut redis::aio::MultiplexedConnection,
    symbol: &str,
    price: f64,
) -> Result<()> {
    let key = format!("radu:price:{}", symbol);
    let _: () = conn
        .set_ex(&key, price.to_string(), 300) // cache for 5 minutes
        .await
        .context("Failed to cache price in Redis")?;
    Ok(())
}

/// Get a cached price from Redis.
pub async fn get_cached_price(
    conn: &mut redis::aio::MultiplexedConnection,
    symbol: &str,
) -> Result<Option<f64>> {
    let key = format!("radu:price:{}", symbol);
    let value: Option<String> = conn.get(&key).await.context("Failed to get cached price")?;
    match value {
        Some(v) => Ok(Some(
            v.parse().context("Cached price is not a valid number")?,
        )),
        None => Ok(None),
    }
}
