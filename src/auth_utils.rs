// OptiTask/backend-api/src/auth_utils.rs
use actix_web::{dev::Payload, Error as ActixWebError, FromRequest, HttpRequest};
use futures_util::future::{err, ok, Ready};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AuthenticatedUser {
    pub id: Uuid,
}

impl FromRequest for AuthenticatedUser {
    type Error = ActixWebError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        log::debug!(
            "Headers received by AuthenticatedUser extractor: {:?}",
            req.headers()
        ); // Gardez ce log pour le debug

        if let Some(user_id_header_value) = req.headers().get("X-User-Id") {
            if let Ok(user_id_str) = user_id_header_value.to_str() {
                if user_id_str.is_empty() {
                    // Vérifier si le header est présent mais vide
                    log::warn!("X-User-Id header is present but empty.");
                    return err(actix_web::error::ErrorBadRequest(
                        "X-User-Id header cannot be empty.",
                    ));
                }
                match Uuid::parse_str(user_id_str) {
                    Ok(user_id_uuid) => {
                        log::debug!("Successfully parsed X-User-Id: {}", user_id_uuid);
                        return ok(AuthenticatedUser { id: user_id_uuid });
                    }
                    Err(parse_err) => {
                        log::warn!(
                            "Failed to parse X-User-Id '{}' to UUID: {}",
                            user_id_str,
                            parse_err
                        );
                        // Retourner un 400 Bad Request pour un format invalide
                        return err(actix_web::error::ErrorBadRequest(
                            "Invalid X-User-Id header format (not a valid UUID).",
                        ));
                    }
                }
            } else {
                log::warn!("X-User-Id header is not valid UTF-8.");
                return err(actix_web::error::ErrorBadRequest(
                    "X-User-Id header contains invalid characters.",
                ));
            }
        } else {
            log::warn!("X-User-Id header was NOT found in request headers.");
            // Retourner un 401 Unauthorized pour un header manquant
            return err(actix_web::error::ErrorUnauthorized(
                "Missing X-User-Id header. Authentication required.",
            ));
        }
    }
}
