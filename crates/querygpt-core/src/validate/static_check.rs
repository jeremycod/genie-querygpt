use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

/// Basic static SQL parse check (extend with schema-aware validation).
pub fn parse_ok(sql: &str) -> anyhow::Result<()> {
    let dialect = PostgreSqlDialect {};
    Parser::parse_sql(&dialect, sql)?;
    Ok(())
}
