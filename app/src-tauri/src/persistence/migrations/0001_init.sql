-- Migration 0001: initial singleton project_meta table.
--
-- project_meta is the database-side authority for Project identity. Its
-- `project_id` must equal the package manifest's `project_id`; a mismatch is
-- rejected by the persistence layer rather than silently repaired.
CREATE TABLE IF NOT EXISTS project_meta (
    -- CHECK(id = 1) enforces the singleton constraint at the schema level.
    id INTEGER PRIMARY KEY CHECK (id = 1),
    project_id TEXT NOT NULL UNIQUE,
    format_version INTEGER NOT NULL,
    schema_version INTEGER NOT NULL,
    working_name TEXT NOT NULL,
    last_committed_revision INTEGER NOT NULL DEFAULT 0,
    restored_from_project_id TEXT NULL,
    restored_from_backup_id TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
