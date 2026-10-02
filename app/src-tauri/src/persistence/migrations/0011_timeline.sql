-- Rebuild only the two registries whose fixed kind lists grow. migrate() keeps
-- foreign-key rewriting off, then checks the complete reference graph before commit.
ALTER TABLE record_identity RENAME TO old_timeline_identity;
CREATE TABLE record_identity (
    record_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('entry','story_unit','relationship_instance','temporal_occurrence')),
    workspace_state TEXT NOT NULL CHECK(workspace_state IN ('active','archived','trashed')),
    lifecycle_changed_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);
INSERT INTO record_identity SELECT * FROM old_timeline_identity;
DROP TABLE old_timeline_identity;
ALTER TABLE capability_def RENAME TO old_timeline_capability;
CREATE TABLE capability_def (id TEXT PRIMARY KEY CHECK(id IN ('base','spatial','event')));
INSERT INTO capability_def SELECT * FROM old_timeline_capability;
INSERT INTO capability_def VALUES ('event');
DROP TABLE old_timeline_capability;

CREATE TABLE timeline_calendar (
    id INTEGER PRIMARY KEY CHECK(id=1),
    schema_version INTEGER NOT NULL,
    config_json TEXT NOT NULL CHECK(json_valid(config_json)),
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL
);
CREATE TABLE temporal_occurrence (
    id TEXT PRIMARY KEY REFERENCES record_identity(record_id) ON DELETE RESTRICT,
    title TEXT NOT NULL, notes TEXT NOT NULL,
    event_entry_id TEXT REFERENCES entry(id) ON DELETE RESTRICT,
    year INTEGER, month INTEGER, day INTEGER,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revision INTEGER NOT NULL,
    CHECK ((year IS NULL AND month IS NULL AND day IS NULL) OR
           (year IS NOT NULL AND month IS NOT NULL AND day IS NOT NULL AND
            year BETWEEN -1000000 AND 1000000 AND month BETWEEN 1 AND 60 AND day BETWEEN 1 AND 1000))
);
CREATE INDEX occurrence_date ON temporal_occurrence(year,month,day,id);
CREATE INDEX occurrence_event ON temporal_occurrence(event_entry_id);
CREATE TRIGGER occurrence_identity_insert BEFORE INSERT ON temporal_occurrence
WHEN NOT EXISTS(SELECT 1 FROM record_identity WHERE record_id=NEW.id AND kind='temporal_occurrence')
BEGIN SELECT RAISE(ABORT,'Occurrence identity kind mismatch'); END;
CREATE TRIGGER occurrence_identity_immutable BEFORE UPDATE OF id ON temporal_occurrence
BEGIN SELECT RAISE(ABORT,'Occurrence identity cannot change'); END;
CREATE TRIGGER occurrence_event_insert BEFORE INSERT ON temporal_occurrence
WHEN NEW.event_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM entry_capability WHERE entry_id=NEW.event_entry_id AND capability_id='event')
BEGIN SELECT RAISE(ABORT,'An event page requires Event capability'); END;
CREATE TRIGGER occurrence_event_update BEFORE UPDATE OF event_entry_id ON temporal_occurrence
WHEN NEW.event_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM entry_capability WHERE entry_id=NEW.event_entry_id AND capability_id='event')
BEGIN SELECT RAISE(ABORT,'An event page requires Event capability'); END;
CREATE TRIGGER occurrence_event_preserve BEFORE DELETE ON entry_capability
WHEN OLD.capability_id='event' AND EXISTS(SELECT 1 FROM temporal_occurrence WHERE event_entry_id=OLD.entry_id)
BEGIN SELECT RAISE(ABORT,'Event capability is in use by the timeline'); END;
CREATE TABLE occurrence_entry (
    occurrence_id TEXT NOT NULL REFERENCES temporal_occurrence(id) ON DELETE RESTRICT,
    entry_id TEXT NOT NULL REFERENCES entry(id) ON DELETE RESTRICT,
    PRIMARY KEY(occurrence_id,entry_id)
);
CREATE INDEX occurrence_entry_target ON occurrence_entry(entry_id);
CREATE TABLE occurrence_chapter (
    occurrence_id TEXT NOT NULL REFERENCES temporal_occurrence(id) ON DELETE RESTRICT,
    chapter_id TEXT NOT NULL REFERENCES story_unit(id) ON DELETE RESTRICT,
    PRIMARY KEY(occurrence_id,chapter_id)
);
CREATE INDEX occurrence_chapter_target ON occurrence_chapter(chapter_id);
