-- Aliases are authored names. Search documents and state are disposable caches.
CREATE TABLE entry_alias (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    alias_text TEXT NOT NULL,
    normalized_alias TEXT NOT NULL,
    UNIQUE(entry_id, normalized_alias)
);
CREATE INDEX entry_alias_lookup ON entry_alias(normalized_alias);

CREATE TABLE derived_index_state (
    id INTEGER PRIMARY KEY CHECK(id=1),
    source_revision INTEGER NOT NULL,
    indexed_revision INTEGER NOT NULL DEFAULT -1,
    schema_version INTEGER NOT NULL DEFAULT 1,
    dirty INTEGER NOT NULL DEFAULT 1,
    document_count INTEGER NOT NULL DEFAULT 0
);
INSERT INTO derived_index_state(id,source_revision)
VALUES(1,COALESCE((SELECT last_committed_revision FROM project_meta WHERE id=1),0));

-- Every authored command already advances this revision in its transaction.
-- Invalidation therefore commits or rolls back with the source, including renames.
CREATE TRIGGER search_source_updated AFTER UPDATE OF last_committed_revision ON project_meta
BEGIN UPDATE derived_index_state SET source_revision=NEW.last_committed_revision,dirty=1 WHERE id=1; END;
CREATE TRIGGER search_source_created AFTER INSERT ON project_meta
BEGIN UPDATE derived_index_state SET source_revision=NEW.last_committed_revision,dirty=1 WHERE id=1; END;

CREATE VIRTUAL TABLE search_index USING fts5(payload UNINDEXED, terms, tokenize='unicode61 remove_diacritics 0');
