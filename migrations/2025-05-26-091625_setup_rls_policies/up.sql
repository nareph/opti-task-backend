-- migrations/YYYY-MM-DD-HHMMSS_setup_rls_policies/up.sql

-- Projects
ALTER TABLE projects ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Users can manage their own projects" ON projects
    FOR ALL
    TO authenticated
    USING (auth.uid() = user_id)
    WITH CHECK (auth.uid() = user_id);

-- Tasks
ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Users can manage their own tasks" ON tasks
    FOR ALL
    TO authenticated
    USING (auth.uid() = user_id)
    WITH CHECK (auth.uid() = user_id);

-- Labels
ALTER TABLE labels ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Users can manage their own labels" ON labels
    FOR ALL
    TO authenticated
    USING (auth.uid() = user_id)
    WITH CHECK (auth.uid() = user_id);

-- TaskLabels - RLS est un peu plus délicat ici car il n'y a pas de user_id direct.
-- On se base sur le user_id de la tâche ou du label lié.
-- Pour la simplicité, si les tâches et labels ont déjà RLS, les jointures seront sécurisées.
-- Une politique plus explicite pourrait être :
ALTER TABLE task_labels ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Users can manage task_labels for their own tasks" ON task_labels
    FOR ALL
    TO authenticated
    USING (
        EXISTS (
            SELECT 1 FROM tasks
            WHERE tasks.id = task_labels.task_id AND tasks.user_id = auth.uid()
        )
        AND
        EXISTS (
            SELECT 1 FROM labels
            WHERE labels.id = task_labels.label_id AND labels.user_id = auth.uid()
        )
    )
    WITH CHECK (
         EXISTS (
            SELECT 1 FROM tasks
            WHERE tasks.id = task_labels.task_id AND tasks.user_id = auth.uid()
        )
        AND
        EXISTS (
            SELECT 1 FROM labels
            WHERE labels.id = task_labels.label_id AND labels.user_id = auth.uid()
        )
    );


-- TimeEntries
ALTER TABLE time_entries ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Users can manage their own time_entries" ON time_entries
    FOR ALL
    TO authenticated
    USING (auth.uid() = user_id)
    WITH CHECK (auth.uid() = user_id);