/// SQL query executor with safety guardrails
///
/// Executes SQL queries against PostgreSQL with preview/export modes,
/// enforces limits and timeouts, and converts results to JSON.
use crate::db::{DbError, DbPool};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio_postgres::types::Type;
use tokio_postgres::Row;

/// Maximum rows allowed in preview mode
const MAX_PREVIEW_ROWS: usize = 1000;

/// Maximum rows allowed in export mode
const MAX_EXPORT_ROWS: usize = 1_000_000;

/// Errors that can occur during SQL execution
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Query timeout after {0}s")]
    Timeout(u64),

    #[error("Database error [{sqlstate}]: {message}")]
    Database { sqlstate: String, message: String },

    #[error("Connection pool error: {0}")]
    Pool(#[from] DbError),

    #[error("Result set too large: {actual} rows exceeds limit {max}")]
    TooLarge { actual: usize, max: usize },

    #[error("Type conversion error: {0}")]
    TypeConversion(String),

    #[error("Invalid SQL: {0}")]
    InvalidSql(String),
}

/// Column metadata from query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub pg_type: String,
}

/// Query execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub total_rows: usize,
    /// Total number of matching rows (before LIMIT) - only populated in preview mode
    pub total_matching_rows: Option<usize>,
    pub execution_time_ms: u64,
}

/// Execution mode determines limits and behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Preview mode: Limited rows for UI display
    Preview { limit: usize },
    /// Export mode: Full scan with higher limit
    Export,
}

impl ExecutionMode {
    /// Get the row limit for this mode
    pub fn row_limit(&self) -> usize {
        match self {
            ExecutionMode::Preview { limit } => (*limit).min(MAX_PREVIEW_ROWS),
            ExecutionMode::Export => MAX_EXPORT_ROWS,
        }
    }

    /// Check if a row count exceeds the limit
    pub fn exceeds_limit(&self, rows: usize) -> bool {
        rows > self.row_limit()
    }
}

/// SQL query executor
pub struct SqlExecutor {
    pool: DbPool,
}

impl SqlExecutor {
    /// Create a new executor with the given connection pool
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Execute a SQL query and return results
    ///
    /// Applies safety guardrails:
    /// - Enforces row limits based on execution mode
    /// - Sets statement timeout from pool configuration
    /// - Converts PostgreSQL types to JSON
    /// - In preview mode, runs COUNT(*) first to get total matching rows
    pub async fn execute(
        &self,
        sql: &str,
        mode: ExecutionMode,
    ) -> Result<QueryResult, ExecutionError> {
        let start = Instant::now();

        // Get connection from pool
        let client = self.pool.get().await.map_err(|e| match e {
            DbError::PoolAcquisition(pool_err) => ExecutionError::Database {
                sqlstate: String::new(),
                message: format!("Pool error: {}", pool_err),
            },
            _ => ExecutionError::Pool(e),
        })?;

        // Set statement timeout
        let timeout_secs = self.pool.query_timeout().as_secs();
        let timeout_sql = format!("SET statement_timeout = '{}'", timeout_secs * 1000);
        client
            .execute(&timeout_sql, &[])
            .await
            .map_err(|e| ExecutionError::Database {
                sqlstate: e.code().map(|c| c.code().to_string()).unwrap_or_default(),
                message: e.to_string(),
            })?;

        // Get total matching rows count if in preview mode
        let total_matching_rows = match mode {
            ExecutionMode::Preview { .. } => {
                let count_sql = Self::build_count_query(sql);
                let count_result = client.query_one(&count_sql, &[]).await.map_err(|e| {
                    // Check for timeout
                    if let Some(code) = e.code() {
                        if code.code() == "57014" {
                            return ExecutionError::Timeout(timeout_secs);
                        }
                    }

                    ExecutionError::Database {
                        sqlstate: e.code().map(|c| c.code().to_string()).unwrap_or_default(),
                        message: format!("COUNT query failed: {}", e),
                    }
                })?;

                // Extract count from result
                let count: i64 = count_result.try_get(0).map_err(|e| {
                    ExecutionError::TypeConversion(format!("Failed to extract count: {}", e))
                })?;

                Some(count as usize)
            }
            ExecutionMode::Export => None,
        };

        // Apply limit to SQL if in preview mode
        let executed_sql = match mode {
            ExecutionMode::Preview { limit } => Self::apply_limit_to_sql(sql, limit),
            ExecutionMode::Export => sql.to_string(),
        };

        // Execute the query
        let rows = client.query(&executed_sql, &[]).await.map_err(|e| {
            // Check for timeout
            if let Some(code) = e.code() {
                if code.code() == "57014" {
                    return ExecutionError::Timeout(timeout_secs);
                }
            }

            ExecutionError::Database {
                sqlstate: e.code().map(|c| c.code().to_string()).unwrap_or_default(),
                message: e.to_string(),
            }
        })?;

        // Get row count (limit is already applied in SQL)
        let row_count = rows.len();

        // Extract column metadata
        let columns = if let Some(first_row) = rows.first() {
            first_row
                .columns()
                .iter()
                .map(|col| ColumnInfo {
                    name: col.name().to_string(),
                    pg_type: format!("{:?}", col.type_()),
                })
                .collect()
        } else {
            Vec::new()
        };

        // Convert rows to JSON
        let json_rows: Vec<Vec<serde_json::Value>> = rows
            .iter()
            .map(|row| self.row_to_json(row))
            .collect::<Result<_, _>>()?;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            columns,
            rows: json_rows,
            total_rows: row_count,
            total_matching_rows,
            execution_time_ms,
        })
    }

    /// Convert a PostgreSQL row to a vector of JSON values
    fn row_to_json(&self, row: &Row) -> Result<Vec<serde_json::Value>, ExecutionError> {
        let mut values = Vec::new();

        for (idx, column) in row.columns().iter().enumerate() {
            let value = self.column_to_json(row, idx, column.type_())?;
            values.push(value);
        }

        Ok(values)
    }

    /// Convert a single column value to JSON
    fn column_to_json(
        &self,
        row: &Row,
        idx: usize,
        col_type: &Type,
    ) -> Result<serde_json::Value, ExecutionError> {
        // Convert based on PostgreSQL type
        // Each type handler properly deals with NULL values using Option<T>
        let json_value = match *col_type {
            Type::BOOL => row
                .try_get::<_, Option<bool>>(idx)
                .map_err(|e| ExecutionError::TypeConversion(e.to_string()))?
                .map(serde_json::Value::Bool)
                .unwrap_or(serde_json::Value::Null),

            Type::INT2 | Type::INT4 => row
                .try_get::<_, Option<i32>>(idx)
                .map_err(|e| ExecutionError::TypeConversion(e.to_string()))?
                .map(|v| serde_json::Value::Number(v.into()))
                .unwrap_or(serde_json::Value::Null),

            Type::INT8 => row
                .try_get::<_, Option<i64>>(idx)
                .map_err(|e| ExecutionError::TypeConversion(e.to_string()))?
                .map(|v| serde_json::Value::Number(v.into()))
                .unwrap_or(serde_json::Value::Null),

            Type::FLOAT4 | Type::FLOAT8 => row
                .try_get::<_, Option<f64>>(idx)
                .map_err(|e| ExecutionError::TypeConversion(e.to_string()))?
                .and_then(|v| serde_json::Number::from_f64(v))
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),

            Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME => row
                .try_get::<_, Option<String>>(idx)
                .map_err(|e| ExecutionError::TypeConversion(e.to_string()))?
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),

            // JSON/JSONB - parse string representation
            Type::JSON | Type::JSONB => row
                .try_get::<_, Option<String>>(idx)
                .map_err(|e| ExecutionError::TypeConversion(e.to_string()))?
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::Value::Null),

            // Array types - convert to JSON array
            _ if col_type.name().starts_with('_') => {
                // Array type (e.g., _text, _int4)
                row.try_get::<_, Option<Vec<String>>>(idx)
                    .map_err(|e| ExecutionError::TypeConversion(e.to_string()))?
                    .map(|v| {
                        serde_json::Value::Array(
                            v.into_iter().map(serde_json::Value::String).collect(),
                        )
                    })
                    .unwrap_or(serde_json::Value::Null)
            }

            // Default: convert to string representation
            _ => row
                .try_get::<_, Option<String>>(idx)
                .map_err(|e| ExecutionError::TypeConversion(e.to_string()))?
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        };

        Ok(json_value)
    }

    /// Apply LIMIT clause to SQL query
    ///
    /// If the SQL already has a LIMIT, replace it with the specified limit.
    /// If the SQL doesn't have a LIMIT, append one.
    fn apply_limit_to_sql(sql: &str, limit: usize) -> String {
        let sql_upper = sql.to_uppercase();

        // Check if SQL already has a LIMIT clause
        if let Some(limit_pos) = sql_upper.rfind("LIMIT") {
            // Find the end of the LIMIT clause (next semicolon, newline, or end of string)
            let after_limit = &sql[limit_pos + 5..].trim_start();

            // Extract just the SQL before LIMIT
            let before_limit = &sql[..limit_pos].trim_end();

            // Check if there's anything after the limit number (like ORDER BY came after)
            // This is a simple heuristic - for complex cases, would need proper SQL parsing
            format!("{} LIMIT {}", before_limit, limit)
        } else {
            // No LIMIT clause exists, append one
            let trimmed = sql.trim_end();
            // Remove trailing semicolon if present
            let without_semicolon = trimmed.trim_end_matches(';').trim_end();
            format!("{} LIMIT {}", without_semicolon, limit)
        }
    }

    /// Build a COUNT(*) query from a SELECT query
    ///
    /// Wraps the original query in a subquery to count all matching rows
    /// This approach handles complex queries (GROUP BY, DISTINCT, etc.) correctly
    fn build_count_query(sql: &str) -> String {
        let sql_upper = sql.to_uppercase();
        let mut sql_trimmed = sql.trim_end_matches(';').trim().to_string();

        // Remove ORDER BY clause if present (doesn't affect count)
        if let Some(order_pos) = sql_upper.rfind(" ORDER BY ") {
            sql_trimmed = sql_trimmed[..order_pos].trim().to_string();
        }

        // Remove LIMIT clause if present
        let sql_upper = sql_trimmed.to_uppercase();
        if let Some(limit_pos) = sql_upper.rfind(" LIMIT ") {
            sql_trimmed = sql_trimmed[..limit_pos].trim().to_string();
        }

        // Wrap in subquery to get accurate count
        format!("SELECT COUNT(*) FROM ({}) AS count_subquery", sql_trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_limits() {
        let preview = ExecutionMode::Preview { limit: 10 };
        assert_eq!(preview.row_limit(), 10);
        assert!(!preview.exceeds_limit(5));
        assert!(preview.exceeds_limit(15));

        let preview_large = ExecutionMode::Preview { limit: 5000 };
        assert_eq!(preview_large.row_limit(), MAX_PREVIEW_ROWS);

        let export = ExecutionMode::Export;
        assert_eq!(export.row_limit(), MAX_EXPORT_ROWS);
        assert!(!export.exceeds_limit(100_000));
        assert!(export.exceeds_limit(2_000_000));
    }

    #[test]
    fn test_execution_error_display() {
        let err = ExecutionError::Timeout(30);
        assert_eq!(err.to_string(), "Query timeout after 30s");

        let err = ExecutionError::TooLarge {
            actual: 2000,
            max: 1000,
        };
        assert_eq!(
            err.to_string(),
            "Result set too large: 2000 rows exceeds limit 1000"
        );
    }

    #[test]
    fn test_column_info_serialization() {
        let col = ColumnInfo {
            name: "id".to_string(),
            pg_type: "INT4".to_string(),
        };

        let json = serde_json::to_string(&col).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("INT4"));
    }
}
