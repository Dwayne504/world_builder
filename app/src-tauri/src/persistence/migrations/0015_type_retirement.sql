-- Retirement preserves identity, assignments and inheritance. Only new uses
-- of the retired definition are blocked; an active descendant stays usable.
ALTER TABLE type_def ADD COLUMN retired_at TEXT NULL;

CREATE TRIGGER entry_type_active_insert BEFORE INSERT ON entry
WHEN NEW.type_id IS NOT NULL AND EXISTS (
    SELECT 1 FROM type_def WHERE id=NEW.type_id AND retired_at IS NOT NULL
)
BEGIN SELECT RAISE(ABORT, 'This Type is retired. Restore it before assigning it to another Entry.'); END;

CREATE TRIGGER entry_type_active_update BEFORE UPDATE OF type_id ON entry
WHEN NEW.type_id IS NOT OLD.type_id AND EXISTS (
    SELECT 1 FROM type_def WHERE id=NEW.type_id AND retired_at IS NOT NULL
)
BEGIN SELECT RAISE(ABORT, 'This Type is retired. Restore it before assigning it to another Entry.'); END;

CREATE TRIGGER type_parent_active_insert BEFORE INSERT ON type_def
WHEN NEW.parent_type_id IS NOT NULL AND EXISTS (
    SELECT 1 FROM type_def WHERE id=NEW.parent_type_id AND retired_at IS NOT NULL
)
BEGIN SELECT RAISE(ABORT, 'This parent Type is retired. Restore it before creating a new child Type.'); END;

CREATE TRIGGER type_parent_active_update BEFORE UPDATE OF parent_type_id ON type_def
WHEN NEW.parent_type_id IS NOT OLD.parent_type_id AND EXISTS (
    SELECT 1 FROM type_def WHERE id=NEW.parent_type_id AND retired_at IS NOT NULL
)
BEGIN SELECT RAISE(ABORT, 'This parent Type is retired. Restore it before choosing it for another Type.'); END;
