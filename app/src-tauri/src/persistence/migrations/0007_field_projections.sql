-- The migration runner disables FK enforcement only while rebuilding this
-- table, checks every FK before commit, and restores enforcement on all paths.
-- Legacy rename semantics keep the child tables referencing field_definition.
ALTER TABLE field_definition RENAME TO field_definition_before_projections;
CREATE TABLE field_definition (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    value_kind TEXT NOT NULL CHECK (value_kind IN ('short_text','number','boolean','choice','multi_choice','relationship')),
    retired_at TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL,
    unit TEXT NULL CHECK (unit IS NULL OR (value_kind = 'number' AND length(trim(unit)) > 0))
);
INSERT INTO field_definition SELECT * FROM field_definition_before_projections;
DROP TABLE field_definition_before_projections;
CREATE TRIGGER field_kind_immutable BEFORE UPDATE OF value_kind ON field_definition
WHEN NEW.value_kind <> OLD.value_kind
BEGIN SELECT RAISE(ABORT, 'kind conversion requires a reviewed migration'); END;
CREATE TRIGGER field_unit_preserve_values BEFORE UPDATE OF unit ON field_definition
WHEN NEW.unit IS NOT OLD.unit AND EXISTS (SELECT 1 FROM field_value WHERE field_id=OLD.id)
BEGIN SELECT RAISE(ABORT, 'unit changes on populated fields require explicit conversion'); END;

CREATE TABLE field_projection (
    field_id TEXT PRIMARY KEY REFERENCES field_definition(id) ON DELETE RESTRICT,
    relationship_definition_id TEXT NOT NULL REFERENCES relationship_definition(id) ON DELETE RESTRICT,
    perspective TEXT NOT NULL CHECK (perspective IN ('source','target')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL
);
CREATE INDEX field_projection_relationship ON field_projection(relationship_definition_id);
CREATE TRIGGER field_projection_kind_insert BEFORE INSERT ON field_projection
WHEN NOT EXISTS (SELECT 1 FROM field_definition WHERE id=NEW.field_id AND value_kind='relationship')
BEGIN SELECT RAISE(ABORT, 'projection requires a relationship field'); END;
CREATE TRIGGER field_projection_immutable BEFORE UPDATE ON field_projection
BEGIN SELECT RAISE(ABORT, 'projection changes require a reviewed semantic migration'); END;
CREATE TRIGGER projection_value_insert BEFORE INSERT ON field_value
WHEN EXISTS (SELECT 1 FROM field_projection WHERE field_id=NEW.field_id)
BEGIN SELECT RAISE(ABORT, 'relationship fields have no stored field values'); END;
CREATE TRIGGER projection_value_update BEFORE UPDATE ON field_value
WHEN EXISTS (SELECT 1 FROM field_projection WHERE field_id=NEW.field_id)
BEGIN SELECT RAISE(ABORT, 'relationship fields have no stored field values'); END;
