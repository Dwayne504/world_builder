-- Navigation metadata has no authority over authored content or draft recovery.
CREATE TABLE project_navigation_settings (
    id INTEGER PRIMARY KEY CHECK(id=1),
    recent_limit INTEGER NOT NULL CHECK(recent_limit BETWEEN 1 AND 100)
);
INSERT INTO project_navigation_settings VALUES(1,20);
CREATE TABLE project_pin (
    record_kind TEXT NOT NULL CHECK(record_kind IN ('entry','story_unit','temporal_occurrence')),
    record_id TEXT NOT NULL REFERENCES record_identity(record_id) ON DELETE RESTRICT,
    pinned_order INTEGER NOT NULL,
    PRIMARY KEY(record_kind,record_id)
);
CREATE TABLE project_recent (
    record_kind TEXT NOT NULL CHECK(record_kind IN ('entry','story_unit','temporal_occurrence')),
    record_id TEXT NOT NULL REFERENCES record_identity(record_id) ON DELETE RESTRICT,
    access_rank INTEGER NOT NULL,
    PRIMARY KEY(record_kind,record_id)
);
CREATE INDEX project_recent_order ON project_recent(access_rank DESC);
