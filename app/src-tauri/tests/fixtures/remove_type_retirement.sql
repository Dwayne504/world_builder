-- Synthetic downgrade helper: remove only the schema-15 extension before
-- a test constructs an older database. Never run against author Projects.
DROP TRIGGER entry_type_active_insert;
DROP TRIGGER entry_type_active_update;
DROP TRIGGER type_parent_active_insert;
DROP TRIGGER type_parent_active_update;
ALTER TABLE type_def DROP COLUMN retired_at;
