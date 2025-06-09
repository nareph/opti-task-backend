use crate::auth_utils::AuthenticatedUser;
use crate::db::DbPool;
use crate::error_handler::ServiceError;
use crate::models::{
    CreateTimeEntryPayload, NewTimeEntry, TimeEntry, UpdateTimeEntryChangeset,
    UpdateTimeEntryPayload,
};
use crate::schema::{
    tasks,                        // Import tasks for ownership verification
    time_entries::{self, dsl::*}, // dsl::* for filters etc.
};
use actix_web::{delete, get, post, put, web, HttpResponse, Result as ActixResult};
use chrono::{NaiveDateTime, Utc}; // Utc for Utc::now()
use diesel::prelude::*;
use diesel_async::RunQueryDsl; // Async traits
use serde_json::json; // For custom JSON responses
use uuid::Uuid;

// DTO for listing query parameters
#[derive(serde::Deserialize, Debug)]
pub struct ListTimeEntriesQuery {
    pub task_id: Option<Uuid>,
    pub date_from: Option<NaiveDateTime>, // ISO8601 format: YYYY-MM-DDTHH:MM:SS
    pub date_to: Option<NaiveDateTime>,   // ISO8601 format: YYYY-MM-DDTHH:MM:SS
                                          // pub page: Option<i64>, // For future pagination
                                          // pub per_page: Option<i64>,
}

// === POST /time-entries ===
#[post("")] // Relative to "/time-entries" scope in main.rs
pub async fn create_time_entry_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    payload: web::Json<CreateTimeEntryPayload>,
) -> ActixResult<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id; // Uuid is Copy

    log::info!(
        "User {} creating time entry with payload: {:?}",
        user_uuid,
        payload.0 // Access internal data of web::Json for logging
    );

    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    // 1. Verify that the associated task belongs to the user
    let _task_exists = tasks::table
        .filter(tasks::id.eq(payload.task_id))
        .filter(tasks::user_id.eq(user_uuid))
        .select(tasks::id)
        .first::<Uuid>(&mut conn)
        .await
        .map_err(|db_err| {
            // More fine-grained error handling for NotFound
            match db_err {
                diesel::result::Error::NotFound => ServiceError::NotFound(format!(
                    "Task with id {} not found or not owned by user",
                    payload.task_id
                )),
                _ => ServiceError::from(db_err),
            }
        })?;

    // 2. Calculate duration_seconds if end_time is provided and duration_seconds is not
    let mut final_duration_seconds = payload.duration_seconds;
    if let Some(end) = payload.end_time {
        if final_duration_seconds.is_none() && end > payload.start_time {
            final_duration_seconds = Some((end - payload.start_time).num_seconds() as i32);
        }
    }

    let new_time_entry_data = NewTimeEntry {
        user_id: user_uuid,
        task_id: payload.task_id,
        start_time: payload.start_time,
        end_time: payload.end_time,
        duration_seconds: final_duration_seconds,
        is_pomodoro_session: payload.is_pomodoro_session, // NewTimeEntry.is_pomodoro_session is Option<bool>
                                                          // DB has DEFAULT FALSE, so None here is ok.
    };

    // 3. Insert
    let created_entry = diesel::insert_into(time_entries::table)
        .values(&new_time_entry_data)
        .get_result::<TimeEntry>(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    log::info!("Time entry created successfully: {:?}", created_entry);
    Ok(HttpResponse::Created().json(created_entry))
}

// === GET /time-entries ===
#[get("")]
pub async fn list_time_entries_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    query_params: web::Query<ListTimeEntriesQuery>,
) -> ActixResult<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    let query_options = query_params.into_inner();
    log::info!(
        "User {} listing time entries with options: {:?}",
        user_uuid,
        query_options
    );

    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let mut query = time_entries
        .filter(user_id.eq(user_uuid))
        .order(start_time.desc()) // Most recent first
        .select(TimeEntry::as_select())
        .into_boxed();

    if let Some(t_id) = query_options.task_id {
        query = query.filter(task_id.eq(t_id));
    }
    if let Some(from_date) = query_options.date_from {
        query = query.filter(start_time.ge(from_date));
    }
    if let Some(to_date) = query_options.date_to {
        query = query.filter(start_time.le(to_date));
    }

    let entries = query
        .load::<TimeEntry>(&mut conn)
        .await
        .map_err(ServiceError::from)?;

    Ok(HttpResponse::Ok().json(entries))
}

// === GET /time-entries/{entry_id_path} ===
#[get("/{entry_id_path}")]
pub async fn get_time_entry_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    entry_id_path: web::Path<Uuid>,
) -> ActixResult<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    let entry_to_find_id = entry_id_path.into_inner();
    log::info!(
        "User {} fetching time_entry {}",
        user_uuid,
        entry_to_find_id
    );

    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let entry_option = time_entries
        .filter(user_id.eq(user_uuid))
        .filter(id.eq(entry_to_find_id))
        .select(TimeEntry::as_select())
        .first::<TimeEntry>(&mut conn)
        .await
        .optional()
        .map_err(ServiceError::from)?;

    match entry_option {
        Some(entry) => Ok(HttpResponse::Ok().json(entry)),
        None => Err(ServiceError::NotFound(format!(
            "TimeEntry with id {} not found or not owned by user",
            entry_to_find_id
        ))),
    }
}

// === PUT /time-entries/{entry_id_path} ===
#[put("/{entry_id_path}")]
pub async fn update_time_entry_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    entry_id_path: web::Path<Uuid>,
    payload: web::Json<UpdateTimeEntryPayload>,
) -> ActixResult<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    let entry_to_update_id = entry_id_path.into_inner();
    log::info!(
        "User {} updating time_entry {} with payload: {:?}",
        user_uuid,
        entry_to_update_id,
        payload.0 // Access internal data of web::Json for logging
    );

    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    // First, fetch the current start_time for duration calculation
    let current_entry_start_time_naive = time_entries
        .filter(id.eq(entry_to_update_id))
        .filter(user_id.eq(user_uuid))
        .select(start_time)
        .first::<NaiveDateTime>(&mut conn)
        .await
        .map_err(|db_err| match db_err {
            // More fine-grained handling of NotFound
            diesel::result::Error::NotFound => ServiceError::NotFound(format!(
                "TimeEntry with id {} not found or not owned by user for update",
                entry_to_update_id
            )),
            _ => ServiceError::from(db_err),
        })?;

    let mut changeset_duration = payload.duration_seconds.clone(); // payload.duration_seconds is Option<Option<i32>>

    // Conversion for comparison and duration calculation
    if let Some(Some(end_t_utc)) = payload.end_time {
        // end_t_utc is DateTime<Utc>
        let end_t_naive = end_t_utc.naive_utc(); // Convert to NaiveDateTime for comparison
        if changeset_duration.is_none() || changeset_duration == Some(None) {
            // Compare two NaiveDateTime
            if end_t_naive > current_entry_start_time_naive {
                changeset_duration = Some(Some(
                    (end_t_naive - current_entry_start_time_naive).num_seconds() as i32,
                ));
            }
        }
    }

    let entry_changes = UpdateTimeEntryChangeset {
        start_time: payload.start_time, // payload.start_time is Option<DateTime<Utc>>
        end_time: payload.end_time.clone(),
        duration_seconds: changeset_duration,
        is_pomodoro_session: payload.is_pomodoro_session,
        updated_at: Some(Utc::now().naive_utc()),
    };

    log::info!(
        "Changeset for time_entry {}: {:?}",
        entry_to_update_id,
        entry_changes
    );

    let updated_entry = diesel::update(
        time_entries
            .filter(id.eq(entry_to_update_id))
            .filter(user_id.eq(user_uuid)),
    )
    .set(&entry_changes)
    .get_result::<TimeEntry>(&mut conn)
    .await
    .map_err(ServiceError::from)?;

    Ok(HttpResponse::Ok().json(updated_entry))
}

// === DELETE /time-entries/{entry_id_path} ===
#[delete("/{entry_id_path}")]
pub async fn delete_time_entry_handler(
    pool: web::Data<DbPool>,
    authenticated_user: AuthenticatedUser,
    entry_id_path: web::Path<Uuid>,
) -> ActixResult<HttpResponse, ServiceError> {
    let user_uuid = authenticated_user.id;
    let entry_to_delete_id = entry_id_path.into_inner();
    log::info!(
        "User {} deleting time_entry {}",
        user_uuid,
        entry_to_delete_id
    );

    let mut conn = pool.get().await.map_err(ServiceError::from)?;

    let num_deleted = diesel::delete(
        time_entries
            .filter(user_id.eq(user_uuid))
            .filter(id.eq(entry_to_delete_id)),
    )
    .execute(&mut conn)
    .await
    .map_err(ServiceError::from)?;

    if num_deleted > 0 {
        Ok(HttpResponse::Ok().json(json!({
            "status": "success",
            "message": format!("TimeEntry with id {} deleted successfully", entry_to_delete_id)
        })))
    } else {
        Err(ServiceError::NotFound(format!(
            "TimeEntry with id {} not found or not owned by user to delete",
            entry_to_delete_id
        )))
    }
}
