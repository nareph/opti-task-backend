-- migrations/YYYY-MM-DD-HHMMSS_add_timestamps_to_labels_and_time_entries/down.sql

-- Drop triggers first if they were created
DROP TRIGGER IF EXISTS set_time_entries_timestamp ON time_entries;
DROP TRIGGER IF EXISTS set_labels_timestamp ON labels;

-- Note: The trigger_set_timestamp() function is more general.
-- Only drop it here if no other tables are using it.
-- If other tables (like projects, tasks) also use it from previous migrations,
-- you would typically drop the function in a very final migration or manage it separately.
-- For simplicity here, we'll assume it can be dropped if this is the only place it's defined.
-- A safer approach is to NOT drop the function in individual migration down.sql files
-- unless you are certain it's not used elsewhere.
-- DROP FUNCTION IF EXISTS trigger_set_timestamp(); -- Consider this carefully

-- Remove timestamp columns from the 'time_entries' table
ALTER TABLE time_entries
DROP COLUMN updated_at,
DROP COLUMN created_at;

-- Remove timestamp columns from the 'labels' table
ALTER TABLE labels
DROP COLUMN updated_at,
DROP COLUMN created_at;