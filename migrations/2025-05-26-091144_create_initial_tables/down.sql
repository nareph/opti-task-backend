-- migrations/YYYY-MM-DD-HHMMSS_create_initial_tables/down.sql
DROP TABLE time_entries;
DROP TABLE task_labels;
DROP TABLE labels;
DROP TABLE tasks;
DROP TABLE projects;
-- Vous n'avez généralement pas besoin de DROP EXTENSION ici, sauf si c'est la seule migration qui l'utilise.