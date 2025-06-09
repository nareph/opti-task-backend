// OptiTask/backend-api/src/error_handler.rs
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use diesel::result::Error as DieselError;
use diesel_async::pooled_connection::PoolError;
// The actual error type from your pool.get().await
use diesel_async::pooled_connection::bb8::RunError as BB8RunError;
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum ServiceError {
    InternalServerError(String),
    BadRequest(String),
    Unauthorized(String),
    DatabaseError(String), // Message déjà formaté
    NotFound(String),
    PoolError(String), // Message déjà formaté
}

impl ServiceError {
    fn from_diesel_error(error: DieselError) -> ServiceError {
        match error {
            DieselError::DatabaseError(kind, info) => {
                let detailed_message =
                    format!("Database error: {:?} - Info: {}", kind, info.message());
                log::error!("Internal Database Error: {}", detailed_message);
                // Pour l'utilisateur, on peut être plus vague ou spécifique selon le cas
                ServiceError::DatabaseError("A database operation failed.".to_string())
            }
            DieselError::NotFound => {
                ServiceError::NotFound("The requested record was not found.".to_string())
            }
            err => {
                log::error!("Unexpected Diesel error: {}", err);
                ServiceError::DatabaseError("An unexpected database error occurred.".to_string())
            }
        }
    }

    fn from_pool_error(error: PoolError) -> ServiceError {
        log::error!("Pool error: {:?}", error);
        ServiceError::PoolError("Could not connect to the database pool.".to_string())
    }

    fn from_bb8_run_error(error: BB8RunError) -> ServiceError {
        log::error!("BB8 connection pool error: {:?}", error);
        ServiceError::PoolError("Could not obtain connection from database pool.".to_string())
    }
}

impl From<DieselError> for ServiceError {
    fn from(error: DieselError) -> ServiceError {
        ServiceError::from_diesel_error(error)
    }
}

// Implementation pour PoolError (quand on utilise pool.get().await)
impl From<PoolError> for ServiceError {
    fn from(error: PoolError) -> ServiceError {
        ServiceError::from_pool_error(error)
    }
}

// Add implementation for diesel-async bb8 RunError
impl From<BB8RunError> for ServiceError {
    fn from(error: BB8RunError) -> ServiceError {
        ServiceError::from_bb8_run_error(error)
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
            ServiceError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ServiceError::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        // Le log de l'erreur détaillée est maintenant dans les constructeurs from_diesel_error/from_bb8_error
        // ou dans les handlers pour InternalServerError/BadRequest/Unauthorized s'ils sont créés manuellement.
        // Ici, on logue juste le message qui sera envoyé à l'utilisateur, pour le contexte.
        let user_facing_message = match status_code.as_u16() < 500 {
            true => self.to_string(),
            false => "An internal server error occurred. Please try again later.".to_string(),
        };

        if status_code.is_server_error() {
            // On pourrait logguer `self` ici si on veut la version formatée du Display
            // mais les détails sont déjà loggués dans from_diesel_error ou from_bb8_error
            log::error!(
                "Responding with server error ({}): {}",
                status_code,
                user_facing_message
            );
        } else {
            log::warn!(
                "Responding with client error ({}): {}",
                status_code,
                user_facing_message
            );
        }

        HttpResponse::build(status_code).json(json!({
            "status": "error",
            "statusCode": status_code.as_u16(),
            "message": user_facing_message
        }))
    }
}
