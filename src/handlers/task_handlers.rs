// OptiTask/backend-api/src/task_handlers.rs
use crate::auth_utils::AuthenticatedUser;
use crate::db::DbPool;
use crate::error_handler::ServiceError;
use crate::models::{
    CreateTaskPayload, Label, NewTask, PaginatedResponse, Task, TaskApiResponse,
     UpdateTaskChangeset, UpdateTaskPayload,
};
use crate::schema::tasks::dsl::*;
use crate::schema::{labels, task_labels, tasks};
use actix_web::{delete, get, post, put, web, HttpResponse};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

// Struct pour les paramètres de requête de filtrage des tâches
#[derive(Deserialize, Debug)]
pub struct TaskQueryParams {
    pub project_id: Option<Uuid>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[post("")]
pub async fn create_task_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    payload: web::Json<CreateTaskPayload>,
) -> Result<HttpResponse, ServiceError> {
    let new_task_data = NewTask {
        user_id: authenticated_user.id,
        project_id: payload.project_id,
        title: payload.title.clone(),
        description: payload.description.clone(),
        status: payload.status.clone(),
        due_date: payload.due_date,
        order: payload.order,
    };

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // Exécuter la requête de manière async
    let task = diesel::insert_into(tasks::table)
        .values(&new_task_data)
        .get_result::<Task>(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    // Convertir en TaskApiResponse (sans labels pour l'instant)
    let task_response = TaskApiResponse::from(task);

    Ok(HttpResponse::Created().json(task_response))
}

#[get("")]
pub async fn list_tasks_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    query: web::Query<TaskQueryParams>,
) -> Result<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;

    // Paramètres de pagination
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(10);
    let offset = (page - 1) * per_page;

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // Construire la requête de base pour compter le total
    let mut count_query = tasks.filter(user_id.eq(user_uuid)).into_boxed();

    // Construire la requête principale
    let mut query_builder = tasks.filter(user_id.eq(user_uuid)).into_boxed();

    // Filtrer par projet si spécifié
    if let Some(project_uuid) = query.project_id {
        query_builder = query_builder.filter(project_id.eq(project_uuid));
        count_query = count_query.filter(project_id.eq(project_uuid));
    }

    // Filtrer par statut si spécifié
    if let Some(task_status) = &query.status {
        query_builder = query_builder.filter(status.eq(task_status));
        count_query = count_query.filter(status.eq(task_status));
    }

    // Compter le total d'éléments
    let total_items = count_query
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    // Exécuter la requête principale avec pagination
    let task_list = query_builder
        .order(tasks::created_at.desc())
        .limit(per_page)
        .offset(offset)
        .select(Task::as_select())
        .load::<Task>(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    // Convertir les tâches en TaskApiResponse et récupérer les labels
    let mut task_responses = Vec::new();

    for task in task_list {
        // Récupérer les labels pour cette tâche
        let task_labels_list = task_labels::table
            .filter(task_labels::task_id.eq(task.id))
            .inner_join(labels::table.on(labels::id.eq(task_labels::label_id)))
            .select(Label::as_select())
            .load::<Label>(&mut conn)
            .await
            .map_err(ServiceError::from)?;

        let mut task_response = TaskApiResponse::from(task);
        task_response.labels = task_labels_list;
        task_responses.push(task_response);
    }

    let total_pages = (total_items + per_page - 1) / per_page;

    let paginated_response = PaginatedResponse {
        items: task_responses,
        total_items,
        total_pages,
        page,
        per_page,
    };

    Ok(HttpResponse::Ok().json(paginated_response))
}

#[get("/{task_id_path}")]
pub async fn get_task_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    task_id_path: web::Path<Uuid>,
) -> Result<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    let task_to_find_id = task_id_path.into_inner();

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // Exécuter la requête de manière async
    let task_option = tasks
        .filter(user_id.eq(user_uuid))
        .filter(id.eq(task_to_find_id))
        .select(Task::as_select())
        .first::<Task>(&mut conn)
        .await
        .optional()
        .map_err(ServiceError::from)?;

    match task_option {
        Some(task) => {
            // Récupérer les labels pour cette tâche
            let task_labels_list = task_labels::table
                .filter(task_labels::task_id.eq(task.id))
                .inner_join(labels::table.on(labels::id.eq(task_labels::label_id)))
                .select(Label::as_select())
                .load::<Label>(&mut conn)
                .await
                .map_err(ServiceError::from)?;

            let mut task_response = TaskApiResponse::from(task);
            task_response.labels = task_labels_list;

            Ok(HttpResponse::Ok().json(task_response))
        }
        None => Err(ServiceError::NotFound(format!(
            "Task with id {} not found or not owned by user",
            task_to_find_id
        ))),
    }
}

#[put("/{task_id_path}")]
pub async fn update_task_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    task_id_path: web::Path<Uuid>,
    payload: web::Json<UpdateTaskPayload>,
) -> Result<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    let task_to_update_id = task_id_path.into_inner();

    let task_changes = UpdateTaskChangeset {
        project_id: payload.project_id.clone(),
        title: payload.title.clone(),
        description: payload.description.clone(),
        status: payload.status.clone(),
        due_date: payload.due_date.clone(),
        order: payload.order.clone(),
        updated_at: Some(Utc::now().naive_utc()),
    };

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // Exécuter la requête de manière async
    let updated_task = diesel::update(
        tasks
            .filter(id.eq(task_to_update_id))
            .filter(user_id.eq(user_uuid)),
    )
    .set(&task_changes)
    .get_result::<Task>(&mut conn)
    .await
    .map_err(ServiceError::from)?;

    // Récupérer les labels pour la tâche mise à jour
    let task_labels_list = task_labels::table
        .filter(task_labels::task_id.eq(updated_task.id))
        .inner_join(labels::table.on(labels::id.eq(task_labels::label_id)))
        .select(Label::as_select())
        .load::<Label>(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    let mut task_response = TaskApiResponse::from(updated_task);
    task_response.labels = task_labels_list;

    Ok(HttpResponse::Ok().json(task_response))
}

#[delete("/{task_id_path}")]
pub async fn delete_task_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    task_id_path: web::Path<Uuid>,
) -> Result<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    let task_to_delete_id = task_id_path.into_inner();

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // D'abord, supprimer les associations de labels
    diesel::delete(task_labels::table.filter(task_labels::task_id.eq(task_to_delete_id)))
        .execute(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    // Ensuite, supprimer la tâche
    let num_deleted = diesel::delete(
        tasks
            .filter(user_id.eq(user_uuid))
            .filter(id.eq(task_to_delete_id)),
    )
    .execute(&mut conn)
    .await
    .map_err(ServiceError::from)?;

    if num_deleted > 0 {
        Ok(HttpResponse::Ok().json(json!({
            "status": "success",
            "message": format!("Task with id {} deleted successfully", task_to_delete_id)
        })))
    } else {
        Err(ServiceError::NotFound(format!(
            "Task with id {} not found or not owned by user to delete",
            task_to_delete_id
        )))
    }
}

#[put("/{task_id_path}/toggle-completion")]
pub async fn toggle_task_completion_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    task_id_path: web::Path<Uuid>,
) -> Result<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    let task_to_toggle_id = task_id_path.into_inner();

    // Obtenir une connexion du pool
    let mut conn = pool.get().await?;

    // D'abord, récupérer la tâche pour connaître son statut actuel
    let current_task = tasks
        .filter(user_id.eq(user_uuid))
        .filter(id.eq(task_to_toggle_id))
        .select(Task::as_select())
        .first::<Task>(&mut conn)
        .await
        .optional()
        .map_err(ServiceError::from)?;

    let task = match current_task {
        Some(t) => t,
        None => {
            return Err(ServiceError::NotFound(format!(
                "Task with id {} not found or not owned by user",
                task_to_toggle_id
            )))
        }
    };

    // Déterminer le nouveau statut
    let new_status = if task.status == "completed" {
        "pending".to_string()
    } else {
        "completed".to_string()
    };

    let task_changes = UpdateTaskChangeset {
        project_id: None,
        title: None,
        description: None,
        status: Some(new_status),
        due_date: None,
        order: None,
        updated_at: Some(Utc::now().naive_utc()),
    };

    // Mettre à jour la tâche
    let updated_task = diesel::update(
        tasks
            .filter(id.eq(task_to_toggle_id))
            .filter(user_id.eq(user_uuid)),
    )
    .set(&task_changes)
    .get_result::<Task>(&mut conn)
    .await
    .map_err(ServiceError::from)?;

    // Récupérer les labels pour la tâche mise à jour
    let task_labels_list = task_labels::table
        .filter(task_labels::task_id.eq(updated_task.id))
        .inner_join(labels::table.on(labels::id.eq(task_labels::label_id)))
        .select(Label::as_select())
        .load::<Label>(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    let mut task_response = TaskApiResponse::from(updated_task);
    task_response.labels = task_labels_list;

    Ok(HttpResponse::Ok().json(task_response))
}
