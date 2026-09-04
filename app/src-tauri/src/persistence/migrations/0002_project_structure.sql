CREATE TABLE category (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    is_uncategorized INTEGER NOT NULL DEFAULT 0 CHECK (is_uncategorized IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL
);

CREATE UNIQUE INDEX one_uncategorized_category
    ON category(is_uncategorized) WHERE is_uncategorized = 1;

CREATE TABLE type_def (
    id TEXT PRIMARY KEY,
    category_id TEXT NOT NULL REFERENCES category(id) ON DELETE RESTRICT,
    parent_type_id TEXT NULL REFERENCES type_def(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL,
    CHECK (parent_type_id IS NULL OR parent_type_id <> id)
);

CREATE INDEX type_def_category ON type_def(category_id);

CREATE TRIGGER type_parent_valid_insert
BEFORE INSERT ON type_def
WHEN NEW.parent_type_id IS NOT NULL
 AND NOT EXISTS (
    SELECT 1 FROM type_def
    WHERE id = NEW.parent_type_id AND category_id = NEW.category_id
 )
BEGIN
    SELECT RAISE(ABORT, 'parent Type must belong to the same Category');
END;

CREATE TRIGGER type_parent_cycle_update
BEFORE UPDATE OF parent_type_id, category_id ON type_def
WHEN NEW.parent_type_id IS NOT NULL
BEGIN
    SELECT CASE WHEN EXISTS (
        WITH RECURSIVE ancestors(id) AS (
            SELECT NEW.parent_type_id
            UNION ALL
            SELECT type_def.parent_type_id
            FROM type_def JOIN ancestors ON type_def.id = ancestors.id
            WHERE type_def.parent_type_id IS NOT NULL
        )
        SELECT 1 FROM ancestors WHERE id = NEW.id
    ) THEN RAISE(ABORT, 'parent Type cycle') END;
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1 FROM type_def
        WHERE id = NEW.parent_type_id AND category_id = NEW.category_id
    ) THEN RAISE(ABORT, 'parent Type must belong to the same Category') END;
END;

CREATE TABLE record_identity (
    record_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('entry', 'story_unit', 'relationship_instance')),
    workspace_state TEXT NOT NULL DEFAULT 'active'
        CHECK (workspace_state IN ('active', 'archived', 'trashed')),
    lifecycle_changed_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE entry (
    id TEXT PRIMARY KEY REFERENCES record_identity(record_id) ON DELETE RESTRICT,
    category_id TEXT NOT NULL REFERENCES category(id) ON DELETE RESTRICT,
    type_id TEXT NULL REFERENCES type_def(id) ON DELETE RESTRICT,
    authored_name TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL
);

CREATE INDEX entry_category_type ON entry(category_id, type_id);

CREATE TRIGGER entry_type_category_insert
BEFORE INSERT ON entry
WHEN NEW.type_id IS NOT NULL
 AND NOT EXISTS (
    SELECT 1 FROM type_def
    WHERE id = NEW.type_id AND category_id = NEW.category_id
 )
BEGIN
    SELECT RAISE(ABORT, 'entry Type must belong to its Category');
END;

CREATE TRIGGER entry_type_category_update
BEFORE UPDATE OF category_id, type_id ON entry
WHEN NEW.type_id IS NOT NULL
 AND NOT EXISTS (
    SELECT 1 FROM type_def
    WHERE id = NEW.type_id AND category_id = NEW.category_id
 )
BEGIN
    SELECT RAISE(ABORT, 'entry Type must belong to its Category');
END;
