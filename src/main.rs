// OptiTask/backend-api/src/main.rs
mod auth_utils;
mod db;
mod error_handler;
mod handlers;
mod models;
pub mod schema;

use actix_cors::Cors;
use actix_web::{http::header, middleware::Logger, web, App, HttpResponse, HttpServer};
use db::DbPool;
use std::env;

// Health check handler avec async
async fn health_check_handler(
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, error_handler::ServiceError> {
    // Test de connexion au pool
    match pool.get().await {
        Ok(_conn) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "status": "healthy",
            "message": "Backend is running and DB pool accessible"
        }))),
        Err(e) => {
            log::error!("Failed to get connection from pool: {:?}", e);
            Err(error_handler::ServiceError::InternalServerError(
                "Failed to check DB pool".to_string(),
            ))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialiser le logger
    env_logger::init();

    // Charger les variables d'environnement
    if cfg!(debug_assertions) {
        match dotenvy::dotenv() {
            Ok(path) => log::info!(".env file loaded from path: {}", path.display()),
            Err(e) => log::warn!(
                "Could not load .env file: {}, using environment variables.",
                e
            ),
        }
    }

    // Récupérer DATABASE_URL
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in environment variables or .env file");

    // Créer le pool de connexions async
    let pool = db::create_pool(&database_url)
        .await
        .expect("Failed to create database connection pool.");

    log::info!("🚀 OptiTask Backend Service starting...");

    // Configuration des URLs pour CORS
    let frontend_url_prod = env::var("FRONTEND_URL_PROD")
        .unwrap_or_else(|_| "https://opti-task-six.vercel.app".to_string());

    let frontend_url_dev =
        env::var("FRONTEND_URL_DEV").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Port et host configuration
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    log::info!("Server will start at http://{}:{}", host, port);

    // Démarrer le serveur HTTP
    HttpServer::new(move || {
        // Configuration CORS
        let mut cors = Cors::default()
            .allowed_origin(&frontend_url_prod)
            .allowed_origin(&frontend_url_dev)
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(web::Data::new(pool.clone()))
            .service(web::resource("/health").route(web::get().to(health_check_handler)))
            .service(
                web::scope("/projects")
                    .service(handlers::project_handlers::create_project_handler)
                    .service(handlers::project_handlers::list_projects_handler)
                    .service(handlers::project_handlers::get_project_handler)
                    .service(handlers::project_handlers::update_project_handler)
                    .service(handlers::project_handlers::delete_project_handler),
            )
            .service(
                web::scope("/tasks")
                    .service(handlers::task_handlers::create_task_handler)
                    .service(handlers::task_handlers::list_tasks_handler)
                    .service(handlers::task_handlers::get_task_handler)
                    .service(handlers::task_handlers::update_task_handler)
                    .service(handlers::task_handlers::delete_task_handler)
                    .service(handlers::task_label_handlers::add_label_to_task_handler)
                    .service(handlers::task_label_handlers::list_labels_for_task_handler)
                    .service(handlers::task_label_handlers::remove_label_from_task_handler),
            )
            .service(
                web::scope("/labels")
                    .service(handlers::label_handlers::create_label_handler)
                    .service(handlers::label_handlers::list_labels_handler)
                    .service(handlers::label_handlers::get_label_handler)
                    .service(handlers::label_handlers::update_label_handler)
                    .service(handlers::label_handlers::delete_label_handler),
            )
            .service(
                web::scope("/time-entries")
                    .service(handlers::time_entry_handlers::create_time_entry_handler)
                    .service(handlers::time_entry_handlers::list_time_entries_handler)
                    .service(handlers::time_entry_handlers::get_time_entry_handler)
                    .service(handlers::time_entry_handlers::update_time_entry_handler)
                    .service(handlers::time_entry_handlers::delete_time_entry_handler),
            )
            .service(
                web::scope("/analytics")
                    .service(handlers::analytics_handlers::get_time_by_project_handler)
                    .service(handlers::analytics_handlers::get_productivity_trend_handler),
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}
