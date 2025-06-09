// OptiTask/backend-api/src/db.rs
use diesel_async::pooled_connection::bb8::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use std::time::Duration;

// Type alias pour notre pool
pub type DbPool = Pool<AsyncPgConnection>;

pub async fn create_pool(database_url: &str) -> Result<DbPool, Box<dyn std::error::Error>> {
    // Configuration du gestionnaire de connexions
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

    // Configuration du pool BB8
    let pool = Pool::builder()
        .max_size(15) // Nombre maximum de connexions
        .min_idle(Some(5)) // Nombre minimum de connexions inactives
        .max_lifetime(Some(Duration::from_secs(30 * 60))) // 30 minutes
        .idle_timeout(Some(Duration::from_secs(10 * 60))) // 10 minutes
        .connection_timeout(Duration::from_secs(30)) // 30 secondes pour obtenir une connexion
        .retry_connection(true)
        .build(config)
        .await?;

    log::info!("Database connection pool created successfully");

    // Test de connexion
    {
        let _conn = pool.get().await?;
        log::info!("Database connection test successful");
    }

    Ok(pool)
}
