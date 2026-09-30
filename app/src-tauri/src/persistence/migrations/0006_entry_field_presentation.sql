-- Entry-local presentation is independent from shared defaults and values.
CREATE TABLE entry_field_presentation (
    entry_id TEXT NOT NULL REFERENCES entry(id) ON DELETE RESTRICT,
    field_id TEXT NOT NULL REFERENCES field_definition(id) ON DELETE RESTRICT,
    hidden INTEGER NOT NULL DEFAULT 0 CHECK (hidden IN (0,1)),
    removed INTEGER NOT NULL DEFAULT 0 CHECK (removed IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL,
    PRIMARY KEY (entry_id, field_id)
);
