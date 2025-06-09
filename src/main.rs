// OptiTask/backend-api/src/main.rs
mod auth_utils;
mod db;
mod error_handler;
mod handlers;
mod models;
pub mod schema;

use actix_cors::Cors;
use actix_web::{http::header, middleware::Logger, web, HttpResponse};
use db::DbPool;
use shuttle_actix_web::ShuttleActixWeb;

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

#[shuttle_runtime::main]
async fn actix_web_main(
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> ShuttleActixWeb<impl FnOnce(&mut web::ServiceConfig) + Send + Clone + 'static> {
    // Charger les variables d'environnement pour le développement local
    if cfg!(debug_assertions) {
        match dotenvy::dotenv() {
            Ok(path) => log::info!(".env file loaded from path: {}", path.display()),
            Err(e) => log::warn!("Could not load .env file: {}, using environment variables or Shuttle secrets if available.", e),
        }
    }

    // Récupérer DATABASE_URL
    let database_url = if let Some(url_from_secrets) = secrets.get("DATABASE_URL") {
        log::info!("DATABASE_URL loaded from Shuttle Secrets.");
        url_from_secrets
    } else {
        log::warn!("DATABASE_URL not found in Shuttle Secrets, attempting to load from environment variables.");
        std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set in .env for local or Secrets.toml for Shuttle")
    };

    // Créer le pool de connexions async
    let pool = db::create_pool(&database_url)
        .await
        .expect("Failed to create database connection pool.");

    log::info!("🚀 OptiTask Backend Service starting...");

    // Configuration CORS
    let frontend_url_prod = secrets.get("FRONTEND_URL_PROD").unwrap_or_else(|| {
        log::warn!("FRONTEND_URL_PROD not set in Shuttle Secrets, using default placeholder.");
        "https://opti-task-six.vercel.app".to_string()
    });

    let frontend_url_dev = secrets
        .get("FRONTEND_URL_DEV")
        .unwrap_or_else(|| "http://localhost:3000".to_string());

    let preview_url_1 = secrets.get("FRONTEND_PREVIEW_URL_1");

    let config_service = move |cfg: &mut web::ServiceConfig| {
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

        if let Some(preview_url) = &preview_url_1 {
            cors = cors.allowed_origin(preview_url);
        }

        cfg.service(
            web::scope("")
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
                ),
        );
    };

    Ok(config_service.into())
}
