// @generated automatically by Diesel CLI.

diesel::table! {
    labels (id) {
        id -> Uuid,
        user_id -> Uuid,
        name -> Text,
        color -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    projects (id) {
        id -> Uuid,
        user_id -> Uuid,
        name -> Text,
        color -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    task_labels (task_id, label_id) {
        task_id -> Uuid,
        label_id -> Uuid,
    }
}

diesel::table! {
    tasks (id) {
        id -> Uuid,
        user_id -> Uuid,
        project_id -> Nullable<Uuid>,
        title -> Text,
        description -> Nullable<Text>,
        status -> Text,
        due_date -> Nullable<Date>,
        task_order -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    time_entries (id) {
        id -> Uuid,
        user_id -> Uuid,
        task_id -> Uuid,
        start_time -> Timestamptz,
        end_time -> Nullable<Timestamptz>,
        duration_seconds -> Nullable<Int4>,
        is_pomodoro_session -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Varchar,
        seed -> Varchar,
        channel_address -> Varchar,
        last_message -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(task_labels -> labels (label_id));
diesel::joinable!(task_labels -> tasks (task_id));
diesel::joinable!(tasks -> projects (project_id));
diesel::joinable!(time_entries -> tasks (task_id));

diesel::allow_tables_to_appear_in_same_query!(
    labels,
    projects,
    task_labels,
    tasks,
    time_entries,
    users,
);
