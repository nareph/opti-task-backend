// OptiTask/backend-api/src/db.rs
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{pooled_connection::bb8::Pool, AsyncPgConnection};

// Type alias pour le pool de connexions
pub type DbPool = Pool<AsyncPgConnection>;

// Fonction pour créer le pool de connexions
pub async fn create_pool(database_url: &str) -> Result<DbPool, Box<dyn std::error::Error>> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let pool = Pool::builder()
        .max_size(10) // Nombre maximum de connexions dans le pool
        .build(config)
        .await?;

    Ok(pool)
}
