-- Capabilities are owned by Entries. Defaults are sampled only at creation.
CREATE TABLE capability_def (
    id TEXT PRIMARY KEY CHECK(id IN ('base', 'spatial'))
);
INSERT INTO capability_def VALUES ('base'), ('spatial');
CREATE TABLE category_capability_default (
    category_id TEXT NOT NULL REFERENCES category(id) ON DELETE RESTRICT,
    capability_id TEXT NOT NULL REFERENCES capability_def(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL,
    PRIMARY KEY(category_id, capability_id)
);
CREATE TABLE type_capability_default (
    type_id TEXT NOT NULL REFERENCES type_def(id) ON DELETE RESTRICT,
    capability_id TEXT NOT NULL REFERENCES capability_def(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL,
    PRIMARY KEY(type_id, capability_id)
);
CREATE TABLE entry_capability (
    entry_id TEXT NOT NULL REFERENCES entry(id) ON DELETE RESTRICT,
    capability_id TEXT NOT NULL REFERENCES capability_def(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL,
    PRIMARY KEY(entry_id, capability_id)
);
INSERT INTO entry_capability
SELECT id, 'base', created_at, updated_at, 1 FROM entry;
CREATE TABLE spatial_node (
    entry_id TEXT PRIMARY KEY REFERENCES entry(id) ON DELETE RESTRICT,
    primary_parent_id TEXT REFERENCES spatial_node(entry_id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL,
    CHECK(entry_id IS NOT primary_parent_id)
);
CREATE INDEX spatial_parent ON spatial_node(primary_parent_id);

CREATE TRIGGER entry_materialize_capabilities AFTER INSERT ON entry BEGIN
    INSERT INTO entry_capability
    SELECT NEW.id, id, NEW.created_at, NEW.updated_at, 1 FROM capability_def
    WHERE id='base' OR id IN (
        SELECT capability_id FROM category_capability_default WHERE category_id=NEW.category_id
        UNION
        SELECT capability_id FROM type_capability_default WHERE type_id IN (
            WITH RECURSIVE ancestors(id) AS (
                SELECT NEW.type_id UNION
                SELECT t.parent_type_id FROM type_def t JOIN ancestors a ON t.id=a.id
                WHERE t.parent_type_id IS NOT NULL
            ) SELECT id FROM ancestors
        )
    );
END;
CREATE TRIGGER capability_create_spatial AFTER INSERT ON entry_capability
WHEN NEW.capability_id='spatial' BEGIN
    INSERT INTO spatial_node VALUES (NEW.entry_id, NULL, NEW.created_at, NEW.updated_at, 1);
END;
CREATE TRIGGER capability_no_identity_update BEFORE UPDATE OF entry_id,capability_id ON entry_capability
BEGIN SELECT RAISE(ABORT, 'Capability identity cannot change'); END;
CREATE TRIGGER capability_remove_guard BEFORE DELETE ON entry_capability
WHEN OLD.capability_id='base' OR EXISTS(SELECT 1 FROM spatial_node WHERE entry_id=OLD.entry_id)
BEGIN SELECT RAISE(ABORT, 'Base is required; Spatial structure must be repaired before removal'); END;
CREATE TRIGGER spatial_require_capability BEFORE INSERT ON spatial_node
WHEN NOT EXISTS(SELECT 1 FROM entry_capability WHERE entry_id=NEW.entry_id AND capability_id='spatial')
BEGIN SELECT RAISE(ABORT, 'Spatial requires the Entry feature'); END;
CREATE TRIGGER spatial_identity_guard BEFORE UPDATE OF entry_id ON spatial_node
BEGIN SELECT RAISE(ABORT, 'Spatial identity cannot change'); END;
CREATE TRIGGER spatial_no_cycle BEFORE UPDATE OF primary_parent_id ON spatial_node
WHEN NEW.primary_parent_id IS NOT NULL BEGIN
    SELECT RAISE(ABORT, 'A place cannot contain itself or one of its ancestors')
    WHERE NEW.entry_id IN (
        WITH RECURSIVE ancestors(id) AS (
            SELECT NEW.primary_parent_id UNION
            SELECT s.primary_parent_id FROM spatial_node s JOIN ancestors a ON s.entry_id=a.id
            WHERE s.primary_parent_id IS NOT NULL
        ) SELECT id FROM ancestors
    );
END;
CREATE TRIGGER spatial_insert_no_cycle BEFORE INSERT ON spatial_node
WHEN NEW.primary_parent_id IS NOT NULL BEGIN
    SELECT RAISE(ABORT, 'A place cannot contain itself or one of its ancestors')
    WHERE NEW.entry_id IN (
        WITH RECURSIVE ancestors(id) AS (
            SELECT NEW.primary_parent_id UNION
            SELECT s.primary_parent_id FROM spatial_node s JOIN ancestors a ON s.entry_id=a.id
            WHERE s.primary_parent_id IS NOT NULL
        ) SELECT id FROM ancestors
    );
END;
