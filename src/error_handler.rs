// OptiTask/backend-api/src/error_handler.rs
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde_json::json;
use std::fmt;

// Import spécifique pour les erreurs de pool diesel-async
use diesel_async::pooled_connection::{bb8, PoolError};

#[derive(Debug)]
pub enum ServiceError {
    InternalServerError(String),
    BadRequest(String),
    Unauthorized(String),
    DatabaseError(String),
    NotFound(String),
    PoolError(String),
    ValidationError(String),
    ConflictError(String),
}

impl ServiceError {
    fn from_pool_error(error: PoolError) -> ServiceError {
        log::error!("Database pool error: {}", error);
        ServiceError::PoolError("Database connection pool error.".to_string())
    }
}

impl From<diesel::result::Error> for ServiceError {
    fn from(error: diesel::result::Error) -> ServiceError {
        match error {
            diesel::result::Error::NotFound => {
                ServiceError::NotFound("The requested item was not found".to_string())
            }
            diesel::result::Error::DatabaseError(kind, info) => {
                log::error!("Database error: {:?} - {}", kind, info.message());
                ServiceError::DatabaseError("A database error occurred".to_string())
            }
            _ => {
                log::error!("Database operation error: {}", error);
                ServiceError::DatabaseError(format!("Database operation failed: {}", error))
            }
        }
    }
}

impl From<bb8::RunError> for ServiceError {
    fn from(error: bb8::RunError) -> ServiceError {
        match error {
            bb8::RunError::User(pool_error) => ServiceError::from(pool_error),
            bb8::RunError::TimedOut => {
                ServiceError::PoolError("Database connection timed out".to_string())
            }
        }
    }
}

// Implémentation From pour les erreurs de pool
impl From<PoolError> for ServiceError {
    fn from(error: PoolError) -> ServiceError {
        ServiceError::from_pool_error(error)
    }
}

// Ajout pour les erreurs de validation serde
impl From<serde_json::Error> for ServiceError {
    fn from(error: serde_json::Error) -> ServiceError {
        log::error!("JSON serialization/deserialization error: {}", error);
        ServiceError::BadRequest("Invalid JSON format.".to_string())
    }
}

// Ajout pour les erreurs UUID
impl From<uuid::Error> for ServiceError {
    fn from(error: uuid::Error) -> ServiceError {
        log::error!("UUID parsing error: {}", error);
        ServiceError::BadRequest("Invalid UUID format.".to_string())
    }
}

// Ajout pour les erreurs de parsing de nombres
impl From<std::num::ParseIntError> for ServiceError {
    fn from(error: std::num::ParseIntError) -> ServiceError {
        log::error!("Number parsing error: {}", error);
        ServiceError::BadRequest("Invalid number format.".to_string())
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServiceError::InternalServerError(msg) => write!(f, "Internal Server Error: {}", msg),
            ServiceError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            ServiceError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ServiceError::DatabaseError(msg) => write!(f, "Database Error: {}", msg),
            ServiceError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            ServiceError::PoolError(msg) => write!(f, "Pool Error: {}", msg),
            ServiceError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            ServiceError::ConflictError(msg) => write!(f, "Conflict Error: {}", msg),
        }
    }
}

impl ResponseError for ServiceError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ServiceError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServiceError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServiceError::PoolError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServiceError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ServiceError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ServiceError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ServiceError::NotFound(_) => StatusCode::NOT_FOUND,
            ServiceError::ConflictError(_) => StatusCode::CONFLICT,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();

        // Message à envoyer au client
        let user_message = match self {
            // Pour les erreurs serveur, on envoie un message générique
            ServiceError::InternalServerError(_)
            | ServiceError::DatabaseError(_)
            | ServiceError::PoolError(_) => {
                "An internal server error occurred. Please try again later.".to_string()
            }
            // Pour les erreurs client, on peut être plus spécifique
            _ => match self {
                ServiceError::BadRequest(msg) => msg.clone(),
                ServiceError::ValidationError(msg) => msg.clone(),
                ServiceError::Unauthorized(msg) => msg.clone(),
                ServiceError::NotFound(msg) => msg.clone(),
                ServiceError::ConflictError(msg) => msg.clone(),
                _ => "An error occurred.".to_string(),
            },
        };

        // Logging approprié selon le type d'erreur
        if status_code.is_server_error() {
            log::error!("Server error ({}): {}", status_code, self);
        } else if status_code.is_client_error() {
            log::warn!("Client error ({}): {}", status_code, self);
        }

        // Construction de la réponse JSON
        let mut response_body = json!({
            "status": "error",
            "code": status_code.as_u16(),
            "message": user_message
        });

        // En mode debug, on peut ajouter plus de détails
        #[cfg(debug_assertions)]
        {
            response_body["debug_info"] = json!({
                "error_type": format!("{:?}", self),
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
        }

        HttpResponse::build(status_code).json(response_body)
    }
}

// Fonctions utilitaires pour créer des erreurs communes
impl ServiceError {
    pub fn bad_request<T: Into<String>>(msg: T) -> Self {
        ServiceError::BadRequest(msg.into())
    }

    pub fn not_found<T: Into<String>>(msg: T) -> Self {
        ServiceError::NotFound(msg.into())
    }

    pub fn unauthorized<T: Into<String>>(msg: T) -> Self {
        ServiceError::Unauthorized(msg.into())
    }

    pub fn internal_error<T: Into<String>>(msg: T) -> Self {
        ServiceError::InternalServerError(msg.into())
    }

    pub fn validation_error<T: Into<String>>(msg: T) -> Self {
        ServiceError::ValidationError(msg.into())
    }

    pub fn conflict<T: Into<String>>(msg: T) -> Self {
        ServiceError::ConflictError(msg.into())
    }
}
