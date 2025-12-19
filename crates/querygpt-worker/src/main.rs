use std::time::Duration;
use tokio_postgres::NoTls;

/// Minimal LISTEN/NOTIFY debouncer skeleton:
/// - LISTEN on offers_changed/campaigns_changed/products_changed/discounts_changed/skus_changed
/// - debounce for N seconds
/// - refresh MVs concurrently
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt().init();

    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let refresh_interval = std::env::var("REFRESH_INTERVAL_SECONDS")
        .unwrap_or_else(|_| "60".to_string())
        .parse::<u64>()
        .unwrap_or(60);

    let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("db connection error: {e}");
        }
    });

    // Listen channels
    for ch in [
        "offers_changed",
        "campaigns_changed",
        "products_changed",
        "discounts_changed",
        "skus_changed",
    ] {
        client.batch_execute(&format!("LISTEN {ch};")).await?;
    }

    // Simple periodic refresh instead of notification-based
    let refresh_interval = Duration::from_secs(refresh_interval);
    
    loop {
        tokio::time::sleep(refresh_interval).await;
        
        // Refresh all MVs periodically
        for mv in [
            "offers_latest",
            "campaigns_latest", 
            "products_latest",
            "discounts_latest",
            "skus_latest",
        ] {
            match client
                .batch_execute(&format!("REFRESH MATERIALIZED VIEW CONCURRENTLY {mv};"))
                .await {
                Ok(_) => tracing::info!("Successfully refreshed {mv}"),
                Err(e) => tracing::error!("Failed to refresh {mv}: {e}"),
            }
        }
    }
}
