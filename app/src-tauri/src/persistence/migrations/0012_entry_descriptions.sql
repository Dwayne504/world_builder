-- Widen the existing versioned document store without touching Chapter prose.
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
       OR (owner_kind='entry' AND area='description'))
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
