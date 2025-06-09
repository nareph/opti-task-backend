-- migrations/YYYY-MM-DD-HHMMSS_add_timestamps_to_labels_and_time_entries/up.sql

-- Add timestamp columns to the 'labels' table
ALTER TABLE labels
ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Add timestamp columns to the 'time_entries' table
ALTER TABLE time_entries
ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Optional: Create triggers to automatically update 'updated_at' on row update for these tables
-- This is a common pattern to avoid manually setting updated_at in your application code.

-- Trigger function (create if it doesn't exist, or use an existing one)
CREATE OR REPLACE FUNCTION trigger_set_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for 'labels'
CREATE TRIGGER set_labels_timestamp
BEFORE UPDATE ON labels
FOR EACH ROW
EXECUTE FUNCTION trigger_set_timestamp();

-- Trigger for 'time_entries'
CREATE TRIGGER set_time_entries_timestamp
BEFORE UPDATE ON time_entries
FOR EACH ROW
EXECUTE FUNCTION trigger_set_timestamp();