CREATE TABLE story_unit (
    id TEXT PRIMARY KEY REFERENCES record_identity(record_id) ON DELETE RESTRICT,
    kind TEXT NOT NULL DEFAULT 'chapter' CHECK(kind='chapter'),
    title TEXT NOT NULL,
    reading_rank INTEGER NOT NULL UNIQUE,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL
);
CREATE TRIGGER story_kind_insert BEFORE INSERT ON story_unit
WHEN NOT EXISTS(SELECT 1 FROM record_identity WHERE record_id=NEW.id AND kind='story_unit')
BEGIN SELECT RAISE(ABORT, 'Chapter identity must be a Story Unit'); END;

CREATE TABLE rich_document (
    id TEXT PRIMARY KEY,
    owner_kind TEXT NOT NULL CHECK(owner_kind='story_unit'),
    owner_id TEXT NOT NULL REFERENCES story_unit(id) ON DELETE RESTRICT,
    area TEXT NOT NULL CHECK(area IN ('manuscript','plan','notes')),
    document_schema_version INTEGER NOT NULL,
    canonical_json TEXT NOT NULL, plain_text TEXT NOT NULL, word_count INTEGER NOT NULL,
    migration_state TEXT NOT NULL DEFAULT 'current',
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL,
    UNIQUE(owner_kind,owner_id,area)
);
CREATE TABLE story_role (
    id TEXT PRIMARY KEY, name TEXT NOT NULL,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL,
    retired_at TEXT NULL
);
CREATE TABLE story_link (
    id TEXT PRIMARY KEY,
    story_unit_id TEXT NOT NULL REFERENCES story_unit(id) ON DELETE RESTRICT,
    entry_id TEXT NULL REFERENCES record_identity(record_id) ON DELETE RESTRICT,
    unresolved_snapshot TEXT NULL,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL,
    UNIQUE(story_unit_id,entry_id),
    CHECK(entry_id IS NOT NULL OR unresolved_snapshot IS NOT NULL)
);
CREATE INDEX story_link_entry ON story_link(entry_id);
CREATE TRIGGER story_link_kind_insert BEFORE INSERT ON story_link
WHEN NEW.entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM entry WHERE id=NEW.entry_id)
BEGIN SELECT RAISE(ABORT, 'Story link must target an Entry'); END;
CREATE TRIGGER story_link_kind_update BEFORE UPDATE OF entry_id ON story_link
WHEN NEW.entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM entry WHERE id=NEW.entry_id)
BEGIN SELECT RAISE(ABORT, 'Story link must target an Entry'); END;
CREATE TABLE story_link_role (
    link_id TEXT NOT NULL REFERENCES story_link(id) ON DELETE CASCADE,
    role_id TEXT NOT NULL REFERENCES story_role(id) ON DELETE RESTRICT,
    PRIMARY KEY(link_id,role_id)
);
