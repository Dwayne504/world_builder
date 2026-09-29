CREATE TABLE relationship_definition (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    forward_label TEXT NOT NULL,
    inverse_label TEXT NOT NULL,
    directed INTEGER NOT NULL CHECK (directed IN (0,1)),
    expected_targets_per_source INTEGER NULL CHECK (expected_targets_per_source > 0),
    expected_sources_per_target INTEGER NULL CHECK (expected_sources_per_target > 0),
    retired_at TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL,
    CHECK (directed = 1 OR (forward_label = inverse_label AND expected_targets_per_source IS expected_sources_per_target))
);
CREATE TABLE relationship_instance (
    id TEXT PRIMARY KEY REFERENCES record_identity(record_id) ON DELETE RESTRICT,
    definition_id TEXT NOT NULL REFERENCES relationship_definition(id) ON DELETE RESTRICT,
    semantic_state TEXT NOT NULL CHECK (semantic_state IN ('active','ended')),
    ended_at TEXT NULL,
    note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL,
    CHECK ((semantic_state = 'active' AND ended_at IS NULL) OR (semantic_state = 'ended' AND ended_at IS NOT NULL))
);
CREATE INDEX relationship_instance_definition ON relationship_instance(definition_id, semantic_state);
CREATE TRIGGER relationship_identity_insert BEFORE INSERT ON relationship_instance
WHEN NOT EXISTS (SELECT 1 FROM record_identity WHERE record_id = NEW.id AND kind = 'relationship_instance')
BEGIN SELECT RAISE(ABORT, 'relationship identity kind mismatch'); END;
CREATE TRIGGER relationship_identity_update BEFORE UPDATE OF id ON relationship_instance
WHEN NEW.id <> OLD.id
BEGIN SELECT RAISE(ABORT, 'relationship identity is immutable'); END;
CREATE TRIGGER relationship_direction_immutable BEFORE UPDATE OF directed ON relationship_definition
WHEN NEW.directed <> OLD.directed
BEGIN SELECT RAISE(ABORT, 'direction changes require a reviewed semantic migration'); END;

-- One participant store is the semantic truth; no parallel source/target columns.
CREATE TABLE relationship_participant (
    instance_id TEXT NOT NULL REFERENCES relationship_instance(id) ON DELETE CASCADE,
    slot TEXT NOT NULL CHECK (slot IN ('source','target')),
    record_id TEXT NULL REFERENCES record_identity(record_id) ON DELETE RESTRICT,
    record_kind TEXT NOT NULL CHECK (record_kind IN ('entry','story_unit','relationship_instance')),
    unresolved_snapshot TEXT NULL,
    PRIMARY KEY(instance_id, slot),
    CHECK ((record_id IS NOT NULL AND unresolved_snapshot IS NULL) OR (record_id IS NULL AND unresolved_snapshot IS NOT NULL))
);
CREATE INDEX relationship_participant_record ON relationship_participant(record_id, instance_id);
CREATE TRIGGER relationship_participant_kind_insert BEFORE INSERT ON relationship_participant
WHEN NEW.record_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM record_identity WHERE record_id = NEW.record_id AND kind = NEW.record_kind)
BEGIN SELECT RAISE(ABORT, 'participant identity kind mismatch'); END;
CREATE TRIGGER relationship_participant_kind_update BEFORE UPDATE ON relationship_participant
WHEN NEW.record_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM record_identity WHERE record_id = NEW.record_id AND kind = NEW.record_kind)
BEGIN SELECT RAISE(ABORT, 'participant identity kind mismatch'); END;
