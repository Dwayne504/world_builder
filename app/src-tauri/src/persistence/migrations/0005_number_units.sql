-- Units are definition metadata; the authored value remains a typed number.
ALTER TABLE field_definition ADD COLUMN unit TEXT NULL
    CHECK (unit IS NULL OR (value_kind = 'number' AND length(trim(unit)) > 0));

-- Changing a populated field's unit would reinterpret authored quantities.
-- Conversion requires a future explicit, reviewed operation.
CREATE TRIGGER field_unit_preserve_values BEFORE UPDATE OF unit ON field_definition
WHEN NEW.unit IS NOT OLD.unit AND EXISTS (SELECT 1 FROM field_value WHERE field_id=OLD.id)
BEGIN SELECT RAISE(ABORT, 'unit changes on populated fields require explicit conversion'); END;
