-- Preserve every existing document, including unsupported versions, byte-for-byte.
DROP TRIGGER rich_document_owner_insert;
DROP TRIGGER rich_document_owner_update;
DROP TRIGGER rich_document_keep_chapter;
DROP TRIGGER rich_document_keep_entry;
CREATE TABLE rich_document_next (
    id TEXT PRIMARY KEY,
    owner_kind TEXT NOT NULL CHECK(owner_kind IN ('story_unit','entry')),
    owner_id TEXT NOT NULL REFERENCES record_identity(record_id) ON DELETE RESTRICT,
    area TEXT NOT NULL,
    document_schema_version INTEGER NOT NULL,
    canonical_json TEXT NOT NULL, plain_text TEXT NOT NULL, word_count INTEGER NOT NULL,
    migration_state TEXT NOT NULL DEFAULT 'current',
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL,
    UNIQUE(owner_kind,owner_id,area),
    CHECK((owner_kind='story_unit' AND area IN ('manuscript','plan','notes'))
       OR (owner_kind='entry' AND (area='description' OR (area LIKE 'field:%' AND length(area)=42))))
);
INSERT INTO rich_document_next SELECT * FROM rich_document;
DROP TABLE rich_document;
ALTER TABLE rich_document_next RENAME TO rich_document;

CREATE TRIGGER rich_document_owner_insert BEFORE INSERT ON rich_document
WHEN NOT EXISTS(SELECT 1 FROM record_identity WHERE record_id=NEW.owner_id AND kind=NEW.owner_kind)
 OR (NEW.owner_kind='entry' AND NOT EXISTS(SELECT 1 FROM entry WHERE id=NEW.owner_id))
 OR (NEW.owner_kind='story_unit' AND NOT EXISTS(SELECT 1 FROM story_unit WHERE id=NEW.owner_id))
BEGIN SELECT RAISE(ABORT, 'Document must belong to an existing Entry or Chapter'); END;
CREATE TRIGGER rich_document_owner_update BEFORE UPDATE OF owner_kind,owner_id ON rich_document
WHEN NOT EXISTS(SELECT 1 FROM record_identity WHERE record_id=NEW.owner_id AND kind=NEW.owner_kind)
 OR (NEW.owner_kind='entry' AND NOT EXISTS(SELECT 1 FROM entry WHERE id=NEW.owner_id))
 OR (NEW.owner_kind='story_unit' AND NOT EXISTS(SELECT 1 FROM story_unit WHERE id=NEW.owner_id))
BEGIN SELECT RAISE(ABORT, 'Document must belong to an existing Entry or Chapter'); END;
CREATE TRIGGER rich_document_keep_chapter BEFORE DELETE ON story_unit
WHEN EXISTS(SELECT 1 FROM rich_document WHERE owner_kind='story_unit' AND owner_id=OLD.id)
BEGIN SELECT RAISE(ABORT, 'Preserve authored Chapter documents'); END;
CREATE TRIGGER rich_document_keep_entry BEFORE DELETE ON entry
WHEN EXISTS(SELECT 1 FROM rich_document WHERE owner_kind='entry' AND owner_id=OLD.id)
BEGIN SELECT RAISE(ABORT, 'Preserve authored Entry descriptions'); END;
UPDATE derived_index_state SET dirty=1 WHERE id=1;

-- The migration runner disables FK enforcement only while rebuilding this
-- table, checks every FK before commit, and restores enforcement on all paths.
-- Legacy rename semantics keep the child tables referencing field_definition.
ALTER TABLE field_definition RENAME TO field_definition_before_rich_text;
CREATE TABLE field_definition (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    value_kind TEXT NOT NULL CHECK (value_kind IN ('short_text','number','boolean','choice','multi_choice','relationship','rich_text')),
    retired_at TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL,
    unit TEXT NULL CHECK (unit IS NULL OR (value_kind = 'number' AND length(trim(unit)) > 0))
);
INSERT INTO field_definition SELECT * FROM field_definition_before_rich_text;
DROP TABLE field_definition_before_rich_text;
CREATE TRIGGER field_kind_immutable BEFORE UPDATE OF value_kind ON field_definition
WHEN NEW.value_kind <> OLD.value_kind
BEGIN SELECT RAISE(ABORT, 'kind conversion requires a reviewed migration'); END;
CREATE TRIGGER field_unit_preserve_values BEFORE UPDATE OF unit ON field_definition
WHEN NEW.unit IS NOT OLD.unit AND EXISTS (SELECT 1 FROM field_value WHERE field_id=OLD.id)
BEGIN SELECT RAISE(ABORT, 'unit changes on populated fields require explicit conversion'); END;


ALTER TABLE field_value RENAME TO field_value_before_rich_text;
CREATE TABLE field_value (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL REFERENCES entry(id) ON DELETE RESTRICT,
    field_id TEXT NOT NULL REFERENCES field_definition(id) ON DELETE RESTRICT,
    document_id TEXT UNIQUE REFERENCES rich_document(id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL DEFAULT 0 CHECK (ordinal = 0),
    value_kind TEXT NOT NULL,
    text_value TEXT NULL,
    number_value REAL NULL,
    bool_value INTEGER NULL CHECK (bool_value IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL,
    UNIQUE(entry_id, field_id, ordinal),
    UNIQUE(id, field_id),
    CHECK (
        (document_id IS NULL AND (
        (value_kind = 'short_text' AND text_value IS NOT NULL AND number_value IS NULL AND bool_value IS NULL) OR
        (value_kind = 'number' AND number_value IS NOT NULL AND text_value IS NULL AND bool_value IS NULL) OR
        (value_kind = 'boolean' AND bool_value IS NOT NULL AND text_value IS NULL AND number_value IS NULL) OR
        (value_kind IN ('choice','multi_choice') AND text_value IS NULL AND number_value IS NULL AND bool_value IS NULL))) OR
        (value_kind='rich_text' AND document_id IS NOT NULL AND text_value IS NULL AND number_value IS NULL AND bool_value IS NULL)
    )
);
INSERT INTO field_value(id,entry_id,field_id,ordinal,value_kind,text_value,number_value,bool_value,created_at,updated_at,revision)
SELECT id,entry_id,field_id,ordinal,value_kind,text_value,number_value,bool_value,created_at,updated_at,revision FROM field_value_before_rich_text;
DROP TABLE field_value_before_rich_text;
CREATE TRIGGER field_value_kind_insert BEFORE INSERT ON field_value
WHEN NOT EXISTS (SELECT 1 FROM field_definition WHERE id = NEW.field_id AND value_kind = NEW.value_kind)
BEGIN SELECT RAISE(ABORT, 'value kind must match its field'); END;
CREATE TRIGGER field_value_kind_update BEFORE UPDATE ON field_value
WHEN NOT EXISTS (SELECT 1 FROM field_definition WHERE id = NEW.field_id AND value_kind = NEW.value_kind)
BEGIN SELECT RAISE(ABORT, 'value kind must match its field'); END;
CREATE TRIGGER projection_value_insert BEFORE INSERT ON field_value
WHEN EXISTS (SELECT 1 FROM field_projection WHERE field_id=NEW.field_id)
BEGIN SELECT RAISE(ABORT, 'relationship fields have no stored field values'); END;
CREATE TRIGGER projection_value_update BEFORE UPDATE ON field_value
WHEN EXISTS (SELECT 1 FROM field_projection WHERE field_id=NEW.field_id)
BEGIN SELECT RAISE(ABORT, 'relationship fields have no stored field values'); END;

CREATE TRIGGER field_document_owner_insert BEFORE INSERT ON field_value
WHEN NEW.document_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM rich_document WHERE id=NEW.document_id AND owner_kind='entry'
    AND owner_id=NEW.entry_id AND area='field:' || NEW.field_id
)
BEGIN SELECT RAISE(ABORT, 'Rich Text must belong to this Entry and Field'); END;
CREATE TRIGGER field_document_owner_update BEFORE UPDATE OF entry_id,field_id,document_id ON field_value
WHEN NEW.document_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM rich_document WHERE id=NEW.document_id AND owner_kind='entry'
    AND owner_id=NEW.entry_id AND area='field:' || NEW.field_id
)
BEGIN SELECT RAISE(ABORT, 'Rich Text must belong to this Entry and Field'); END;
CREATE TRIGGER field_document_owner_preserve BEFORE UPDATE OF owner_kind,owner_id,area ON rich_document
WHEN EXISTS(SELECT 1 FROM field_value WHERE document_id=OLD.id)
AND (NEW.owner_kind<>OLD.owner_kind OR NEW.owner_id<>OLD.owner_id OR NEW.area<>OLD.area)
BEGIN SELECT RAISE(ABORT, 'Rich Text ownership cannot change'); END;
-- Only an explicit value clear/delete removes its owning Field document.
CREATE TRIGGER field_document_delete AFTER DELETE ON field_value WHEN OLD.document_id IS NOT NULL
BEGIN DELETE FROM rich_document WHERE id=OLD.document_id; END;
UPDATE derived_index_state SET dirty=1 WHERE id=1;
