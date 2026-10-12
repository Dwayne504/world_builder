-- Rebuild only the definition kind constraint; child FK names stay unchanged
-- under the migration runner's guarded legacy-alter transaction.
ALTER TABLE field_definition RENAME TO field_definition_before_references;
CREATE TABLE field_definition (
    id TEXT PRIMARY KEY, name TEXT NOT NULL,
    value_kind TEXT NOT NULL CHECK (value_kind IN ('short_text','number','boolean','choice','multi_choice','relationship','rich_text','entry_reference')),
    retired_at TEXT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL,
    unit TEXT NULL CHECK (unit IS NULL OR (value_kind = 'number' AND length(trim(unit)) > 0))
);
INSERT INTO field_definition SELECT * FROM field_definition_before_references;
DROP TABLE field_definition_before_references;
CREATE TRIGGER field_kind_immutable BEFORE UPDATE OF value_kind ON field_definition
WHEN NEW.value_kind <> OLD.value_kind
BEGIN SELECT RAISE(ABORT, 'kind conversion requires a reviewed migration'); END;
CREATE TRIGGER field_unit_preserve_values BEFORE UPDATE OF unit ON field_definition
WHEN NEW.unit IS NOT OLD.unit AND EXISTS (SELECT 1 FROM field_value WHERE field_id=OLD.id)
BEGIN SELECT RAISE(ABORT, 'unit changes on populated fields require explicit conversion'); END;

CREATE TABLE entry_reference_value (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL REFERENCES entry(id) ON DELETE RESTRICT,
    field_id TEXT NOT NULL REFERENCES field_definition(id) ON DELETE RESTRICT,
    target_record_id TEXT NULL REFERENCES record_identity(record_id) ON DELETE RESTRICT,
    target_kind TEXT NOT NULL CHECK(target_kind='entry'),
    unresolved_snapshot TEXT NULL,
    ordinal INTEGER NOT NULL CHECK(ordinal>=0),
    UNIQUE(entry_id,field_id,ordinal),
    UNIQUE(entry_id,field_id,target_record_id),
    CHECK((target_record_id IS NOT NULL AND unresolved_snapshot IS NULL) OR
          (target_record_id IS NULL AND unresolved_snapshot IS NOT NULL AND
           json_valid(unresolved_snapshot) AND
           json_type(unresolved_snapshot,'$.former_id') IS 'text' AND
           json_type(unresolved_snapshot,'$.label') IS 'text'))
);
CREATE INDEX entry_reference_target ON entry_reference_value(target_record_id);
CREATE TRIGGER entry_reference_insert BEFORE INSERT ON entry_reference_value
WHEN NOT EXISTS(SELECT 1 FROM field_definition WHERE id=NEW.field_id AND value_kind='entry_reference')
 OR (NEW.target_record_id IS NOT NULL AND NOT EXISTS(
     SELECT 1 FROM entry e JOIN record_identity i ON i.record_id=e.id
     WHERE e.id=NEW.target_record_id AND i.kind='entry'))
BEGIN SELECT RAISE(ABORT,'Reference must use an Entry Reference Field and an Entry target'); END;
CREATE TRIGGER entry_reference_update BEFORE UPDATE ON entry_reference_value
WHEN NOT EXISTS(SELECT 1 FROM field_definition WHERE id=NEW.field_id AND value_kind='entry_reference')
 OR (NEW.target_record_id IS NOT NULL AND NOT EXISTS(
     SELECT 1 FROM entry e JOIN record_identity i ON i.record_id=e.id
     WHERE e.id=NEW.target_record_id AND i.kind='entry'))
BEGIN SELECT RAISE(ABORT,'Reference must use an Entry Reference Field and an Entry target'); END;
CREATE TRIGGER entry_reference_keep_target BEFORE DELETE ON entry
WHEN EXISTS(SELECT 1 FROM entry_reference_value WHERE target_record_id=OLD.id)
BEGIN SELECT RAISE(ABORT,'Preserve incoming Entry references before permanent deletion'); END;
UPDATE derived_index_state SET dirty=1 WHERE id=1;
