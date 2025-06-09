// OptiTask/backend-api/src/label_handlers.rs
use crate::auth_utils::AuthenticatedUser;
use crate::db::DbPool;
use crate::error_handler::ServiceError;
use crate::models::{
    CreateLabelPayload, Label, NewLabel, UpdateLabelChangeset, UpdateLabelPayload,
};
use crate::schema::labels::{self, dsl::*}; // dsl::* pour user_id, id etc.
use actix_web::{delete, get, post, put, web, HttpResponse};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl; // Import async version
use serde_json::json;
use uuid::Uuid;

// === POST /labels ===
#[post("")] // Relatif au scope "/labels" dans main.rs
pub async fn create_label_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    payload: web::Json<CreateLabelPayload>,
) -> Result<HttpResponse, ServiceError> {
    log::info!("Create label payload received: {:?}", payload);

    let new_label_data = NewLabel {
        user_id: authenticated_user.id,
        name: payload.name.clone(),
        color: payload.color.clone(),
    };

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // Exécuter la requête de manière async
    let created_label = diesel::insert_into(labels::table)
        .values(&new_label_data)
        .get_result::<Label>(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    log::info!("Label created successfully: {:?}", created_label);
    Ok(HttpResponse::Created().json(created_label))
}

// === GET /labels ===
#[get("")] // Relatif au scope "/labels" dans main.rs
pub async fn list_labels_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
) -> Result<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    log::info!("Listing labels for user: {}", user_uuid);

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // Exécuter la requête de manière async
    let label_list = labels
        .filter(user_id.eq(user_uuid))
        .order(name.asc()) // Ordonner par nom par exemple
        .select(Label::as_select())
        .load::<Label>(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    Ok(HttpResponse::Ok().json(label_list))
}

// === GET /labels/{label_id_path} ===
#[get("/{label_id_path}")]
pub async fn get_label_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    label_id_path: web::Path<Uuid>,
) -> Result<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    let label_to_find_id = label_id_path.into_inner();

    log::info!("Fetching label {} for user {}", label_to_find_id, user_uuid);

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // Exécuter la requête de manière async
    let label_option = labels
        .filter(user_id.eq(user_uuid))
        .filter(id.eq(label_to_find_id))
        .select(Label::as_select())
        .first::<Label>(&mut conn)
        .await
        .optional()
        .map_err(ServiceError::from)?;

    match label_option {
        Some(label) => Ok(HttpResponse::Ok().json(label)),
        None => Err(ServiceError::NotFound(format!(
            "Label with id {} not found or not owned by user",
            label_to_find_id
        ))),
    }
}

// === PUT /labels/{label_id_path} ===
#[put("/{label_id_path}")]
pub async fn update_label_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    label_id_path: web::Path<Uuid>,
    payload: web::Json<UpdateLabelPayload>,
) -> Result<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    let label_to_update_id = label_id_path.into_inner();

    log::info!(
        "Update label payload for label {}: {:?}",
        label_to_update_id,
        payload
    );

    let label_changes = UpdateLabelChangeset {
        name: payload.name.clone(),
        color: payload.color.clone(), // payload.color est Option<Option<String>>
        updated_at: Some(Utc::now().naive_utc()),
    };

    log::info!(
        "Changeset to apply for label {}: {:?}",
        label_to_update_id,
        label_changes
    );

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // Exécuter la requête de manière async
    let updated_label = diesel::update(
        labels
            .filter(id.eq(label_to_update_id))
            .filter(user_id.eq(user_uuid)),
    )
    .set(&label_changes)
    .get_result::<Label>(&mut conn)
    .await
    .map_err(ServiceError::from)?;

    Ok(HttpResponse::Ok().json(updated_label))
}

// === DELETE /labels/{label_id_path} ===
#[delete("/{label_id_path}")]
pub async fn delete_label_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    label_id_path: web::Path<Uuid>,
) -> Result<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    let label_to_delete_id = label_id_path.into_inner();

    log::info!(
        "Deleting label {} for user {}",
        label_to_delete_id,
        user_uuid
    );

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // Avant de supprimer un label, vous pourriez vouloir vérifier s'il est utilisé
    // par des tâches et décider du comportement (ex: interdire la suppression,
    // ou supprimer les associations dans task_labels).
    // Pour l'instant, suppression simple.
    let num_deleted = diesel::delete(
        labels
            .filter(user_id.eq(user_uuid))
            .filter(id.eq(label_to_delete_id)),
    )
    .execute(&mut conn)
    .await
    .map_err(ServiceError::from)?;

    if num_deleted > 0 {
        Ok(HttpResponse::Ok().json(json!({
            "status": "success",
            "message": format!("Label with id {} deleted successfully", label_to_delete_id)
        })))
    } else {
        Err(ServiceError::NotFound(format!(
            "Label with id {} not found or not owned by user to delete",
            label_to_delete_id
        )))
    }
}
