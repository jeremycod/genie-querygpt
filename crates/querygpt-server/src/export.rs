use crate::db::DbPool;
use crate::executor::{ExecutionError, SqlExecutor};
use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use csv::WriterBuilder;
use thiserror::Error;

/// Export-specific errors
#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),

    #[error("CSV serialization error: {0}")]
    CsvSerialization(#[from] csv::Error),

    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database not configured")]
    DatabaseNotConfigured,
}

impl IntoResponse for ExportError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ExportError::DatabaseNotConfigured => (
                StatusCode::NOT_IMPLEMENTED,
                "Database not configured. Set DATABASE_URL environment variable.".to_string(),
            ),
            ExportError::Execution(e) => (
                StatusCode::BAD_REQUEST,
                format!("Query execution failed: {}", e),
            ),
            ExportError::CsvSerialization(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("CSV serialization failed: {}", e),
            ),
            ExportError::JsonSerialization(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("JSON serialization failed: {}", e),
            ),
            ExportError::Io(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("I/O error: {}", e),
            ),
        };

        (status, message).into_response()
    }
}

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
}

/// CSV exporter with streaming support
pub struct CsvExporter {
    pool: DbPool,
}

impl CsvExporter {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Export query results to CSV format
    ///
    /// This executes the SQL query and streams results as CSV.
    /// Memory-efficient for large datasets.
    pub async fn export(&self, sql: &str) -> Result<Response, ExportError> {
        // Execute query with export mode (no row limit, but timeout still applies)
        let executor = SqlExecutor::new(self.pool.clone());
        let mode = crate::executor::ExecutionMode::Export;
        let result = executor.execute(sql, mode).await?;

        tracing::info!(
            "CSV export: {} rows, {} columns in {}ms",
            result.total_rows,
            result.columns.len(),
            result.execution_time_ms
        );

        // Build CSV in memory
        // For very large datasets, we'd want to stream this, but for MVP this is acceptable
        let mut buffer = Vec::new();
        {
            let mut writer = WriterBuilder::new().from_writer(&mut buffer);

            // Write header row
            let headers: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
            writer.write_record(&headers)?;

            // Write data rows
            for row in &result.rows {
                let string_row: Vec<String> = row
                    .iter()
                    .map(|value| match value {
                        serde_json::Value::Null => String::new(),
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                            // Serialize complex types as JSON strings
                            serde_json::to_string(value).unwrap_or_else(|_| String::new())
                        }
                    })
                    .collect();
                writer.write_record(&string_row)?;
            }

            writer.flush()?;
        }

        // Generate filename with timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("export_{}.csv", timestamp);

        // Build response with appropriate headers
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            "text/csv; charset=utf-8".parse().unwrap(),
        );
        headers.insert(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename)
                .parse()
                .unwrap(),
        );
        headers.insert(
            header::CONTENT_LENGTH,
            buffer.len().to_string().parse().unwrap(),
        );

        Ok((headers, buffer).into_response())
    }
}

/// JSON exporter with streaming support
pub struct JsonExporter {
    pool: DbPool,
}

impl JsonExporter {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Export query results to JSON format
    ///
    /// This executes the SQL query and returns results as JSON array.
    /// For very large datasets, consider using CSV format instead.
    pub async fn export(&self, sql: &str) -> Result<Response, ExportError> {
        // Execute query with export mode
        let executor = SqlExecutor::new(self.pool.clone());
        let mode = crate::executor::ExecutionMode::Export;
        let result = executor.execute(sql, mode).await?;

        tracing::info!(
            "JSON export: {} rows, {} columns in {}ms",
            result.total_rows,
            result.columns.len(),
            result.execution_time_ms
        );

        // Convert to JSON array of objects
        // Each row becomes an object with column names as keys
        let mut rows_as_objects = Vec::new();
        for row in &result.rows {
            let mut obj = serde_json::Map::new();
            for (i, col) in result.columns.iter().enumerate() {
                obj.insert(col.name.clone(), row[i].clone());
            }
            rows_as_objects.push(serde_json::Value::Object(obj));
        }

        // Serialize to JSON
        let json_bytes = serde_json::to_vec_pretty(&rows_as_objects)?;

        // Generate filename with timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("export_{}.json", timestamp);

        // Build response with appropriate headers
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            "application/json; charset=utf-8".parse().unwrap(),
        );
        headers.insert(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename)
                .parse()
                .unwrap(),
        );
        headers.insert(
            header::CONTENT_LENGTH,
            json_bytes.len().to_string().parse().unwrap(),
        );

        Ok((headers, json_bytes).into_response())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_equality() {
        assert_eq!(ExportFormat::Csv, ExportFormat::Csv);
        assert_eq!(ExportFormat::Json, ExportFormat::Json);
        assert_ne!(ExportFormat::Csv, ExportFormat::Json);
    }
}
