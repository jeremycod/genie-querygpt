/// Database connection pool management
///
/// Provides a thread-safe PostgreSQL connection pool using deadpool-postgres.
/// Configuration is loaded from environment variables.
use deadpool_postgres::{Config, ManagerConfig, Pool, PoolError, RecyclingMethod, Runtime};
use std::time::Duration;

/// Errors that can occur during database operations
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database URL not configured")]
    MissingDatabaseUrl,

    #[error("Failed to create connection pool: {0}")]
    PoolCreation(String),

    #[error("Failed to acquire connection from pool: {0}")]
    PoolAcquisition(#[from] PoolError),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Database connection pool
///
/// Thread-safe wrapper around deadpool-postgres Pool.
/// Configured via environment variables:
/// - DATABASE_URL (required): PostgreSQL connection string
/// - DB_POOL_MAX_SIZE (optional, default: 10): Maximum connections in pool
/// - DB_CONNECT_TIMEOUT_SECS (optional, default: 5): Timeout for acquiring connection
/// - DB_QUERY_TIMEOUT_SECS (optional, default: 30): Timeout for query execution
#[derive(Clone)]
pub struct DbPool {
    pool: Pool,
    query_timeout: Duration,
}

impl DbPool {
    /// Create a new database pool from environment configuration
    ///
    /// Reads configuration from:
    /// - DATABASE_URL: PostgreSQL connection string (e.g., postgresql://user:pass@host:5432/db)
    /// - DB_POOL_MAX_SIZE: Max connections (default: 10)
    /// - DB_CONNECT_TIMEOUT_SECS: Connection timeout (default: 5)
    /// - DB_QUERY_TIMEOUT_SECS: Query timeout (default: 30)
    pub fn from_env() -> Result<Self, DbError> {
        // Get database URL (required)
        let database_url =
            std::env::var("DATABASE_URL").map_err(|_| DbError::MissingDatabaseUrl)?;

        // Parse optional configuration
        let max_size = std::env::var("DB_POOL_MAX_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        let connect_timeout_secs = std::env::var("DB_CONNECT_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        let query_timeout_secs = std::env::var("DB_QUERY_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        // Build deadpool configuration
        let mut config = Config::new();
        config.url = Some(database_url);
        config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });
        config.pool = Some(deadpool::managed::PoolConfig {
            max_size,
            timeouts: deadpool::managed::Timeouts {
                wait: Some(Duration::from_secs(connect_timeout_secs)),
                ..Default::default()
            },
            queue_mode: deadpool::managed::QueueMode::Fifo,
        });

        // Create pool
        let pool = config
            .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
            .map_err(|e| DbError::PoolCreation(e.to_string()))?;

        // Set timeouts
        let query_timeout = Duration::from_secs(query_timeout_secs);

        tracing::info!(
            "Database pool created: max_size={}, connect_timeout={}s, query_timeout={}s",
            max_size,
            connect_timeout_secs,
            query_timeout_secs
        );

        Ok(Self {
            pool,
            query_timeout,
        })
    }

    /// Get a connection from the pool
    ///
    /// Returns an error if the pool is exhausted or connection fails
    pub async fn get(&self) -> Result<deadpool_postgres::Object, DbError> {
        self.pool
            .get()
            .await
            .map_err(|e| DbError::PoolAcquisition(e))
    }

    /// Get the configured query timeout duration
    pub fn query_timeout(&self) -> Duration {
        self.query_timeout
    }

    /// Get pool status for monitoring
    pub fn status(&self) -> PoolStatus {
        PoolStatus {
            size: self.pool.status().size,
            available: self.pool.status().available,
            waiting: self.pool.status().waiting,
        }
    }
}

/// Pool status information for monitoring
#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub size: usize,
    pub available: usize,
    pub waiting: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_error_display() {
        let err = DbError::MissingDatabaseUrl;
        assert_eq!(err.to_string(), "Database URL not configured");

        let err = DbError::InvalidConfig("test".to_string());
        assert_eq!(err.to_string(), "Invalid configuration: test");
    }

    #[test]
    fn test_pool_status() {
        // Test that PoolStatus can be created and fields accessed
        let status = PoolStatus {
            size: 10,
            available: 8,
            waiting: 0,
        };
        assert_eq!(status.size, 10);
        assert_eq!(status.available, 8);
        assert_eq!(status.waiting, 0);
    }
}
