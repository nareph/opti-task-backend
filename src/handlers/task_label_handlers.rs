use crate::auth_utils::AuthenticatedUser;
use crate::db::DbPool;
use crate::error_handler::ServiceError;
use crate::models::{Label, NewTaskLabelAssociation}; // TaskLabel pour la suppression, Label pour le listage
use crate::schema::{labels, task_labels, tasks}; // tasks est nécessaire pour vérifier la propriété de la tâche
use actix_web::{delete, get, post, web, HttpResponse, Result as ActixResult};
use diesel::prelude::*;
use diesel_async::RunQueryDsl; // Import async version
use serde::Deserialize; // Pour le DTO du payload
use serde_json::json;
use uuid::Uuid;

// DTO pour le payload de POST /tasks/{taskId}/labels
#[derive(Deserialize, Debug)]
pub struct AddLabelToTaskPayload {
    pub label_id: Uuid,
}

// === POST /tasks/{task_id_path}/labels ===
// Ajoute un label existant à une tâche existante
#[post("/{task_id_path}/labels")]
pub async fn add_label_to_task_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    path_params: web::Path<(Uuid,)>, // web::Path attend un tuple pour un seul paramètre, ou une struct
    payload: web::Json<AddLabelToTaskPayload>,
) -> ActixResult<HttpResponse, ServiceError> {
    let (task_id_from_path,) = path_params.into_inner(); // Extrait l'UUID du tuple
    let user_uuid = authenticated_user.id;
    let label_to_add_id = payload.label_id;

    log::info!(
        "User {} attempting to add label {} to task {}",
        user_uuid,
        label_to_add_id,
        task_id_from_path
    );

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // 1. Vérifier que la tâche appartient à l'utilisateur
    let _task_check = tasks::table
        .filter(tasks::id.eq(task_id_from_path))
        .filter(tasks::user_id.eq(user_uuid))
        .select(tasks::id)
        .first::<Uuid>(&mut conn)
        .await
        .optional()
        .map_err(ServiceError::from)?;

    if _task_check.is_none() {
        return Err(ServiceError::NotFound(format!(
            "Task with id {} not found or not owned by user",
            task_id_from_path
        )));
    }

    // 2. Vérifier que le label appartient à l'utilisateur (ou est public, si vous avez cette notion)
    let _label_check = labels::table
        .filter(labels::id.eq(label_to_add_id))
        .filter(labels::user_id.eq(user_uuid)) // Assumant que les labels sont aussi par utilisateur
        .select(labels::id)
        .first::<Uuid>(&mut conn)
        .await
        .optional()
        .map_err(ServiceError::from)?;

    if _label_check.is_none() {
        return Err(ServiceError::NotFound(format!(
            "Label with id {} not found or not owned by user",
            label_to_add_id
        )));
    }

    // 3. Vérifier si l'association existe déjà pour éviter les doublons
    let existing_association = task_labels::table
        .filter(task_labels::task_id.eq(task_id_from_path))
        .filter(task_labels::label_id.eq(label_to_add_id))
        .select((task_labels::task_id, task_labels::label_id))
        .first::<(Uuid, Uuid)>(&mut conn)
        .await
        .optional()
        .map_err(ServiceError::from)?;

    if existing_association.is_some() {
        return Ok(HttpResponse::Ok().json(json!({
            "status": "success",
            "message": "Label already associated with task",
            "task_id": task_id_from_path,
            "label_id": label_to_add_id
        })));
    }

    // 4. Créer l'association
    let new_association = NewTaskLabelAssociation {
        task_id: task_id_from_path,
        label_id: label_to_add_id,
    };

    let _rows_affected = diesel::insert_into(task_labels::table)
        .values(&new_association)
        .execute(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    Ok(HttpResponse::Created().json(json!({
        "status": "success",
        "message": "Label added to task successfully",
        "task_id": task_id_from_path,
        "label_id": label_to_add_id
    })))
}

// === GET /tasks/{task_id_path}/labels ===
// Liste tous les labels associés à une tâche spécifique
#[get("/{task_id_path}/labels")]
pub async fn list_labels_for_task_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    path_params: web::Path<(Uuid,)>,
) -> ActixResult<HttpResponse, ServiceError> {
    let (task_id_from_path,) = path_params.into_inner();
    let user_uuid = authenticated_user.id;

    log::info!(
        "User {} listing labels for task {}",
        user_uuid,
        task_id_from_path
    );

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // 1. Vérifier que la tâche appartient à l'utilisateur
    let _task_check = tasks::table
        .filter(tasks::id.eq(task_id_from_path))
        .filter(tasks::user_id.eq(user_uuid))
        .select(tasks::id)
        .first::<Uuid>(&mut conn)
        .await
        .optional()
        .map_err(ServiceError::from)?;

    if _task_check.is_none() {
        return Err(ServiceError::NotFound(format!(
            "Task with id {} not found or not owned by user",
            task_id_from_path
        )));
    }

    // 2. Récupérer les labels associés
    // Utilise une jointure implicite ou explicite
    let labels_for_task = task_labels::table
        .filter(task_labels::task_id.eq(task_id_from_path))
        .inner_join(labels::table.on(labels::id.eq(task_labels::label_id)))
        .select(Label::as_select()) // Sélectionne tous les champs du Label
        .load::<Label>(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    Ok(HttpResponse::Ok().json(labels_for_task))
}

// === DELETE /tasks/{task_id_path}/labels/{label_id_path_param} ===
// Retire un label spécifique d'une tâche spécifique
#[delete("/{task_id_path}/labels/{label_id_to_remove_path}")]
pub async fn remove_label_from_task_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    path_params: web::Path<(Uuid, Uuid)>, // Tuple pour task_id et label_id
) -> ActixResult<HttpResponse, ServiceError> {
    let (task_id_from_path, label_id_to_remove) = path_params.into_inner();
    let user_uuid = authenticated_user.id;

    log::info!(
        "User {} attempting to remove label {} from task {}",
        user_uuid,
        label_id_to_remove,
        task_id_from_path
    );

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // 1. Vérifier que la tâche appartient à l'utilisateur (important pour la sécurité)
    // Ceci empêche un utilisateur de manipuler les labels d'une tâche qui ne lui appartient pas
    // même s'il connaît l'ID de la tâche et du label.
    let _task_owner_check = tasks::table
        .filter(tasks::id.eq(task_id_from_path))
        .filter(tasks::user_id.eq(user_uuid))
        .select(tasks::id)
        .first::<Uuid>(&mut conn)
        .await
        .optional()
        .map_err(ServiceError::from)?;

    if _task_owner_check.is_none() {
        return Err(ServiceError::NotFound(format!(
            "Task with id {} not found or not owned by user",
            task_id_from_path
        )));
    }

    // 2. Supprimer l'association
    let num_deleted = diesel::delete(
        task_labels::table
            .filter(task_labels::task_id.eq(task_id_from_path))
            .filter(task_labels::label_id.eq(label_id_to_remove)),
    )
    .execute(&mut conn)
    .await
    .map_err(ServiceError::from)?;

    if num_deleted > 0 {
        Ok(HttpResponse::Ok().json(json!({
            "status": "success",
            "message": "Label removed from task successfully",
            "task_id": task_id_from_path,
            "label_id": label_id_to_remove
        })))
    } else {
        // Cela peut se produire si l'association n'existait pas,
        // ou si la tâche/label n'existe pas (déjà géré par les vérifications précédentes si elles étaient strictes).
        Err(ServiceError::NotFound(format!(
            "Association between task {} and label {} not found, or task not owned by user.",
            task_id_from_path, label_id_to_remove
        )))
    }
}
