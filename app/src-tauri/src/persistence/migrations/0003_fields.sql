CREATE TABLE field_definition (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    value_kind TEXT NOT NULL CHECK (value_kind IN ('short_text','number','boolean','choice','multi_choice')),
    retired_at TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL
);

CREATE TABLE field_availability (
    field_id TEXT NOT NULL REFERENCES field_definition(id) ON DELETE RESTRICT,
    provider_kind TEXT NOT NULL CHECK (provider_kind IN ('category','type','entry')),
    provider_id TEXT NOT NULL,
    PRIMARY KEY (field_id, provider_kind, provider_id)
);
CREATE INDEX field_availability_provider ON field_availability(provider_kind, provider_id);

CREATE TRIGGER field_provider_insert BEFORE INSERT ON field_availability BEGIN
    SELECT CASE WHEN
        (NEW.provider_kind = 'category' AND NOT EXISTS (SELECT 1 FROM category WHERE id = NEW.provider_id)) OR
        (NEW.provider_kind = 'type' AND NOT EXISTS (SELECT 1 FROM type_def WHERE id = NEW.provider_id)) OR
        (NEW.provider_kind = 'entry' AND NOT EXISTS (SELECT 1 FROM entry WHERE id = NEW.provider_id))
    THEN RAISE(ABORT, 'field provider does not exist') END;
END;
CREATE TRIGGER field_provider_immutable BEFORE UPDATE ON field_availability BEGIN
    SELECT RAISE(ABORT, 'replace availability with explicit detach and bind');
END;
CREATE TRIGGER field_category_restrict BEFORE DELETE ON category
WHEN EXISTS (SELECT 1 FROM field_availability WHERE provider_kind = 'category' AND provider_id = OLD.id)
BEGIN SELECT RAISE(ABORT, 'detach Category fields before deletion'); END;
CREATE TRIGGER field_type_restrict BEFORE DELETE ON type_def
WHEN EXISTS (SELECT 1 FROM field_availability WHERE provider_kind = 'type' AND provider_id = OLD.id)
BEGIN SELECT RAISE(ABORT, 'detach Type fields before deletion'); END;
CREATE TRIGGER field_entry_restrict BEFORE DELETE ON entry
WHEN EXISTS (SELECT 1 FROM field_availability WHERE provider_kind = 'entry' AND provider_id = OLD.id)
BEGIN SELECT RAISE(ABORT, 'detach local fields before deletion'); END;

CREATE TABLE choice_option (
    id TEXT PRIMARY KEY,
    field_id TEXT NOT NULL REFERENCES field_definition(id) ON DELETE RESTRICT,
    label TEXT NOT NULL,
    retired_at TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL,
    UNIQUE(id, field_id)
);
CREATE INDEX choice_option_field ON choice_option(field_id);

CREATE TABLE field_value (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL REFERENCES entry(id) ON DELETE RESTRICT,
    field_id TEXT NOT NULL REFERENCES field_definition(id) ON DELETE RESTRICT,
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
        (value_kind = 'short_text' AND text_value IS NOT NULL AND number_value IS NULL AND bool_value IS NULL) OR
        (value_kind = 'number' AND number_value IS NOT NULL AND text_value IS NULL AND bool_value IS NULL) OR
        (value_kind = 'boolean' AND bool_value IS NOT NULL AND text_value IS NULL AND number_value IS NULL) OR
        (value_kind IN ('choice','multi_choice') AND text_value IS NULL AND number_value IS NULL AND bool_value IS NULL)
    )
);
CREATE TRIGGER field_value_kind_insert BEFORE INSERT ON field_value
WHEN NOT EXISTS (SELECT 1 FROM field_definition WHERE id = NEW.field_id AND value_kind = NEW.value_kind)
BEGIN SELECT RAISE(ABORT, 'value kind must match its field'); END;
CREATE TRIGGER field_value_kind_update BEFORE UPDATE ON field_value
WHEN NOT EXISTS (SELECT 1 FROM field_definition WHERE id = NEW.field_id AND value_kind = NEW.value_kind)
BEGIN SELECT RAISE(ABORT, 'value kind must match its field'); END;
CREATE TRIGGER field_kind_immutable BEFORE UPDATE OF value_kind ON field_definition
WHEN NEW.value_kind <> OLD.value_kind
BEGIN SELECT RAISE(ABORT, 'kind conversion requires a reviewed migration'); END;

CREATE TABLE field_choice_value (
    value_id TEXT NOT NULL,
    field_id TEXT NOT NULL,
    option_id TEXT NOT NULL,
    PRIMARY KEY(value_id, option_id),
    FOREIGN KEY(value_id, field_id) REFERENCES field_value(id, field_id) ON DELETE RESTRICT,
    FOREIGN KEY(option_id, field_id) REFERENCES choice_option(id, field_id) ON DELETE RESTRICT
);
CREATE TRIGGER field_choice_kind_insert BEFORE INSERT ON field_choice_value BEGIN
    SELECT CASE WHEN NOT EXISTS (SELECT 1 FROM field_value WHERE id = NEW.value_id AND value_kind IN ('choice','multi_choice'))
        THEN RAISE(ABORT, 'choice selection needs a choice field') END;
    SELECT CASE WHEN EXISTS (SELECT 1 FROM field_value WHERE id = NEW.value_id AND value_kind = 'choice')
        AND EXISTS (SELECT 1 FROM field_choice_value WHERE value_id = NEW.value_id)
        THEN RAISE(ABORT, 'single Choice permits one selection') END;
END;
