-- migrations/YYYY-MM-DD-HHMMSS_setup_rls_policies/down.sql

-- TimeEntries
DROP POLICY IF EXISTS "Users can manage their own time_entries" ON time_entries;
ALTER TABLE time_entries DISABLE ROW LEVEL SECURITY;

-- TaskLabels
DROP POLICY IF EXISTS "Users can manage task_labels for their own tasks" ON task_labels;
ALTER TABLE task_labels DISABLE ROW LEVEL SECURITY;

-- Labels
DROP POLICY IF EXISTS "Users can manage their own labels" ON labels;
ALTER TABLE labels DISABLE ROW LEVEL SECURITY;

-- Tasks
DROP POLICY IF EXISTS "Users can manage their own tasks" ON tasks;
ALTER TABLE tasks DISABLE ROW LEVEL SECURITY;

-- Projects
DROP POLICY IF EXISTS "Users can manage their own projects" ON projects;
ALTER TABLE projects DISABLE ROW LEVEL SECURITY;