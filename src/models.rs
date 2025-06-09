use crate::schema::{labels, projects, task_labels, tasks, time_entries};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Deserializer, Serialize}; // Deserializer est nécessaire pour deserialize_with
use uuid::Uuid;

use diesel::sql_types::BigInt; // Pour les sommes de durées

// --- Fonctions Helper pour la Désérialisation des Champs Optionnels/Nullables ---

// Pour Option<Option<String>>
fn deserialize_opt_opt_string<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<String>::deserialize(deserializer) {
        Ok(Some(s)) => Ok(Some(Some(s))),
        Ok(None) => Ok(Some(None)), // JSON null -> Some(None)
        Err(e) => Err(e),
    }
}

// Pour Option<Option<Uuid>>
fn deserialize_opt_opt_uuid<'de, D>(deserializer: D) -> Result<Option<Option<Uuid>>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<Uuid>::deserialize(deserializer) {
        Ok(Some(u)) => Ok(Some(Some(u))),
        Ok(None) => Ok(Some(None)),
        Err(e) => Err(e),
    }
}

// Pour Option<Option<NaiveDate>>
fn deserialize_opt_opt_naivedate<'de, D>(
    deserializer: D,
) -> Result<Option<Option<NaiveDate>>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<NaiveDate>::deserialize(deserializer) {
        Ok(Some(d)) => Ok(Some(Some(d))),
        Ok(None) => Ok(Some(None)),
        Err(e) => Err(e),
    }
}

// Pour Option<Option<i32>>
fn deserialize_opt_opt_i32<'de, D>(deserializer: D) -> Result<Option<Option<i32>>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<i32>::deserialize(deserializer) {
        Ok(Some(i)) => Ok(Some(Some(i))),
        Ok(None) => Ok(Some(None)),
        Err(e) => Err(e),
    }
}

// NOUVELLE FONCTION HELPER pour Option<Option<DateTime<Utc>>>
fn deserialize_opt_opt_datetime_utc<'de, D>(
    deserializer: D,
) -> Result<Option<Option<DateTime<Utc>>>, D::Error>
// <<< Notez DateTime<Utc> ici
where
    D: Deserializer<'de>,
{
    match Option::<DateTime<Utc>>::deserialize(deserializer) {
        // <<< Et ici
        Ok(Some(dt)) => Ok(Some(Some(dt))),
        Ok(None) => Ok(Some(None)), // JSON null -> Some(None)
        Err(e) => Err(e),
    }
}

// --- Project Model ---
#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[diesel(table_name = projects)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Project {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize, Debug)]
#[diesel(table_name = projects)]
pub struct NewProject {
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = projects)]
pub struct UpdateProjectChangeset {
    pub name: Option<String>,
    pub color: Option<Option<String>>,
    pub updated_at: Option<NaiveDateTime>,
}

// --- Task Model (Diesel Queryable) ---
// Cette struct est pour interagir avec la DB. Elle ne contiendra pas directement les labels.
#[derive(
    Queryable, Selectable, Identifiable, Associations, Deserialize, Debug, Clone, PartialEq,
)]
#[diesel(table_name = tasks)]
#[diesel(belongs_to(Project, foreign_key = project_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Task {
    pub id: Uuid,
    pub user_id: Uuid,
    pub project_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    #[diesel(column_name = task_order)]
    pub order: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// === NOUVELLE STRUCT POUR LA RÉPONSE API DE TÂCHE ===
// C'est ce que le frontend recevra pour une tâche.
#[derive(Serialize, Deserialize, Debug, Clone)] // Ajouter Deserialize pour la cohérence si besoin
pub struct TaskApiResponse {
    // Champs de la tâche (copiés de la struct Task)
    pub id: Uuid,
    pub user_id: Uuid,
    pub project_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    #[serde(rename = "order")] // S'assurer que le JSON correspond à 'order' que le frontend attend
    pub task_order: Option<i32>, // Utiliser un nom de champ différent de Task.order pour éviter confusion
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    // Labels associés
    pub labels: Vec<Label>,
}

// Helper pour convertir une Task DB en TaskApiResponse (sans labels au début)
// Les labels seront ajoutés séparément.
impl From<Task> for TaskApiResponse {
    fn from(task_db: Task) -> Self {
        TaskApiResponse {
            id: task_db.id,
            user_id: task_db.user_id,
            project_id: task_db.project_id,
            title: task_db.title,
            description: task_db.description,
            status: task_db.status,
            due_date: task_db.due_date,
            task_order: task_db.order, // Mapper depuis Task.order
            created_at: task_db.created_at,
            updated_at: task_db.updated_at,
            labels: Vec::new(), // Initialisé vide, sera peuplé dans le handler
        }
    }
}

#[derive(Insertable, Deserialize, Debug)]
#[diesel(table_name = tasks)]
pub struct NewTask {
    pub user_id: Uuid,
    pub project_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub due_date: Option<NaiveDate>,
    #[diesel(column_name = task_order)]
    pub order: Option<i32>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = tasks)]
pub struct UpdateTaskChangeset {
    pub project_id: Option<Option<Uuid>>,
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<String>,
    pub due_date: Option<Option<NaiveDate>>,
    #[diesel(column_name = task_order)]
    pub order: Option<Option<i32>>,
    pub updated_at: Option<NaiveDateTime>,
}

// --- Label Model ---
#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[diesel(table_name = labels)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Label {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize, Debug)]
#[diesel(table_name = labels)]
pub struct NewLabel {
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = labels)]
pub struct UpdateLabelChangeset {
    pub name: Option<String>,
    pub color: Option<Option<String>>,
    pub updated_at: Option<NaiveDateTime>,
}

// --- TaskLabel Model ---
#[derive(
    Queryable,
    Selectable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
)]
#[diesel(table_name = task_labels)]
#[diesel(belongs_to(Task))]
#[diesel(belongs_to(Label))]
#[diesel(primary_key(task_id, label_id))]
pub struct TaskLabel {
    pub task_id: Uuid,
    pub label_id: Uuid,
}

#[derive(Insertable, Deserialize, Debug)]
#[diesel(table_name = task_labels)]
pub struct NewTaskLabelAssociation {
    pub task_id: Uuid,
    pub label_id: Uuid,
}

// --- TimeEntry Model ---
#[derive(
    Queryable,
    Selectable,
    Identifiable,
    Associations,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
)]
#[diesel(table_name = time_entries)]
#[diesel(belongs_to(Task))]
pub struct TimeEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub task_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
    pub is_pomodoro_session: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize, Debug)]
#[diesel(table_name = time_entries)]
pub struct NewTimeEntry {
    pub user_id: Uuid,
    pub task_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
    pub is_pomodoro_session: Option<bool>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = time_entries)]
pub struct UpdateTimeEntryChangeset {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<Option<DateTime<Utc>>>,
    pub duration_seconds: Option<Option<i32>>,
    pub is_pomodoro_session: Option<bool>,
    pub updated_at: Option<NaiveDateTime>,
}

// --- PAYLOAD DTOs ---

#[derive(Deserialize, Debug)]
pub struct CreateProjectPayload {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateProjectPayload {
    pub name: Option<String>,
    #[serde(deserialize_with = "deserialize_opt_opt_string", default)]
    pub color: Option<Option<String>>,
}

#[derive(Deserialize, Debug)]
pub struct CreateTaskPayload {
    pub project_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub order: Option<i32>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateTaskPayload {
    #[serde(deserialize_with = "deserialize_opt_opt_uuid", default)]
    pub project_id: Option<Option<Uuid>>,
    pub title: Option<String>,
    #[serde(deserialize_with = "deserialize_opt_opt_string", default)]
    pub description: Option<Option<String>>,
    pub status: Option<String>,
    #[serde(deserialize_with = "deserialize_opt_opt_naivedate", default)]
    pub due_date: Option<Option<NaiveDate>>,
    #[serde(deserialize_with = "deserialize_opt_opt_i32", default)]
    pub order: Option<Option<i32>>,
}

#[derive(Deserialize, Debug)]
pub struct CreateLabelPayload {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateLabelPayload {
    pub name: Option<String>,
    #[serde(deserialize_with = "deserialize_opt_opt_string", default)]
    pub color: Option<Option<String>>,
}

#[derive(Deserialize, Debug)]
pub struct CreateTimeEntryPayload {
    pub task_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
    pub is_pomodoro_session: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateTimeEntryPayload {
    pub start_time: Option<DateTime<Utc>>, // Pourrait être Option<Option<NaiveDateTime>> si on veut le mettre à NULL
    #[serde(deserialize_with = "deserialize_opt_opt_datetime_utc", default)]
    pub end_time: Option<Option<DateTime<Utc>>>,
    #[serde(deserialize_with = "deserialize_opt_opt_i32", default)]
    pub duration_seconds: Option<Option<i32>>,
    pub is_pomodoro_session: Option<bool>, // Boolean ne peut pas vraiment être "absent vs null", juste true/false/absent
}

// --- Pagination DTOs ---
#[derive(Deserialize, Debug)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}
fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    10
}
#[derive(Serialize, Debug)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total_items: i64,
    pub total_pages: i64,
    pub page: i64,
    pub per_page: i64,
}

// --- Analytics Models ---

#[derive(QueryableByName, Serialize, Deserialize, Debug, Clone)] // QueryableByName si on utilise du SQL brut
#[diesel(check_for_backend(diesel::pg::Pg))] // Nécessaire pour QueryableByName avec un backend spécifique
pub struct TimeByProjectStat {
    #[diesel(sql_type = diesel::sql_types::Uuid)] // Spécifier le type SQL pour QueryableByName
    pub project_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub project_name: String,
    // Diesel sum sur i32 retourne i64 (BigInt). Optionnel si certains projets n'ont pas de temps.
    #[diesel(sql_type = BigInt)] // Diesel sum sur i32/Option<i32> retourne BigInt/Option<BigInt>
    pub total_duration_seconds: i64, // Stocker en i64 car la somme peut dépasser i32
}

#[derive(QueryableByName, Serialize, Deserialize, Debug, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ProductivityTrendPoint {
    #[diesel(sql_type = diesel::sql_types::Date)]
    // Ou Timestamptz si vous groupez par heure/jour exact
    pub date_point: NaiveDate, // Représente le jour ou le début de la semaine/mois
    #[diesel(sql_type = BigInt)]
    pub total_duration_seconds: i64,
}

// DTO pour les paramètres de requête des analytics
#[derive(Deserialize, Debug)]
pub struct AnalyticsQueryPeriod {
    // Ex: "week", "month", "last7days", "last30days", ou des dates spécifiques
    pub period: Option<String>,
    pub start_date: Option<NaiveDate>, // YYYY-MM-DD
    pub end_date: Option<NaiveDate>,   // YYYY-MM-DD
}
