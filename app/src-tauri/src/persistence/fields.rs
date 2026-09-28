//! Typed field storage, used only on the Project worker's serialized connection.
use super::PersistenceError;
use crate::domain::{
    fields::*,
    require_definition_name,
    structure::{ChoiceOptionId, FieldId},
    EntryId,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use std::collections::HashSet;

fn invalid(message: impl ToString) -> PersistenceError {
    PersistenceError::Other(message.to_string())
}
fn name(raw: &str) -> Result<String, PersistenceError> {
    require_definition_name(raw).map_err(invalid)
}
fn parse_kind(raw: String) -> Result<FieldKind, PersistenceError> {
    serde_json::from_value(serde_json::Value::String(raw)).map_err(invalid)
}

pub(super) fn read(conn: &Connection, entry: EntryId) -> Result<EntryFields, PersistenceError> {
    let (category, type_id): (String, Option<String>) = conn.query_row(
        "SELECT category_id, type_id FROM entry WHERE id = ?1",
        [entry.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let mut types = conn.prepare(
        "WITH RECURSIVE ancestors(id, parent) AS (
        SELECT id, parent_type_id FROM type_def WHERE id = ?1 UNION
        SELECT t.id, t.parent_type_id FROM type_def t JOIN ancestors a ON t.id = a.parent
        ) SELECT id FROM ancestors",
    )?;
    let ancestor_ids: HashSet<String> = types
        .query_map([type_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let global_revision = conn.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id = 1",
        [],
        |r| r.get(0),
    )?;
    let mut statement = conn.prepare(
        "SELECT id, name, value_kind, retired_at IS NOT NULL, revision
        FROM field_definition ORDER BY name COLLATE NOCASE, id",
    )?;
    let rows = statement.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, bool>(3)?,
            r.get::<_, i64>(4)?,
        ))
    })?;
    let mut definitions = Vec::new();
    let mut fields = Vec::new();
    for row in rows {
        let (id, name, kind, retired, revision) = row?;
        let field_id = FieldId::parse(&id).map_err(invalid)?;
        let mut options = conn.prepare("SELECT id, label, retired_at IS NOT NULL FROM choice_option WHERE field_id = ?1 ORDER BY created_at, id")?;
        let options = options
            .query_map([&id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, bool>(2)?,
                ))
            })?
            .map(|row| {
                let (id, label, retired) = row?;
                Ok(ChoiceOption {
                    id: ChoiceOptionId::parse(&id).map_err(invalid)?,
                    label,
                    retired,
                })
            })
            .collect::<Result<Vec<_>, PersistenceError>>()?;
        let mut bindings = conn.prepare("SELECT a.provider_kind, a.provider_id, COALESCE(c.name,t.name,e.authored_name,'[Unnamed Entry]')
            FROM field_availability a
            LEFT JOIN category c ON a.provider_kind = 'category' AND c.id = a.provider_id
            LEFT JOIN type_def t ON a.provider_kind = 'type' AND t.id = a.provider_id
            LEFT JOIN entry e ON a.provider_kind = 'entry' AND e.id = a.provider_id
            WHERE a.field_id = ?1 ORDER BY a.provider_kind, a.provider_id")?;
        let bindings = bindings
            .query_map([&id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .map(|row| {
                let (kind, id, label) = row?;
                Ok(FieldBinding {
                    provider: FieldProvider {
                        kind: serde_json::from_value(serde_json::Value::String(kind))
                            .map_err(invalid)?,
                        id,
                    },
                    label,
                })
            })
            .collect::<Result<Vec<_>, PersistenceError>>()?;
        let definition = FieldDefinition {
            id: field_id,
            name,
            kind: parse_kind(kind)?,
            retired,
            revision,
            options,
            bindings,
        };
        let available = !retired
            && definition.bindings.iter().any(|b| match b.provider.kind {
                ProviderKind::Category => b.provider.id == category,
                ProviderKind::Type => ancestor_ids.contains(&b.provider.id),
                ProviderKind::Entry => b.provider.id == entry.to_string(),
            });
        let value = read_value(conn, entry, &definition)?;
        if available || value.is_some() {
            fields.push(EntryField {
                definition: definition.clone(),
                available,
                value,
            });
        }
        definitions.push(definition);
    }
    Ok(EntryFields {
        global_revision,
        fields,
        definitions,
    })
}

fn read_value(
    conn: &Connection,
    entry: EntryId,
    definition: &FieldDefinition,
) -> Result<Option<FieldValue>, PersistenceError> {
    type ValueRow = (String, Option<String>, Option<f64>, Option<bool>);
    let row: Option<ValueRow> = conn.query_row("SELECT id,text_value,number_value,bool_value FROM field_value WHERE entry_id = ?1 AND field_id = ?2",
        params![entry.to_string(), definition.id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    let Some((id, text, number, boolean)) = row else {
        return Ok(None);
    };
    Ok(Some(match definition.kind {
        FieldKind::ShortText => {
            FieldValue::Text(text.ok_or_else(|| invalid("missing text value"))?)
        }
        FieldKind::Number => {
            FieldValue::Number(number.ok_or_else(|| invalid("missing number value"))?)
        }
        FieldKind::Boolean => {
            FieldValue::Boolean(boolean.ok_or_else(|| invalid("missing boolean value"))?)
        }
        FieldKind::Choice | FieldKind::MultiChoice => {
            let mut selections = conn.prepare(
                "SELECT option_id FROM field_choice_value WHERE value_id = ?1 ORDER BY option_id",
            )?;
            let values = FieldValue::Choices(
                selections
                    .query_map([id], |r| r.get::<_, String>(0))?
                    .map(|row| ChoiceOptionId::parse(&row?).map_err(invalid))
                    .collect::<Result<Vec<_>, _>>()?,
            );
            values
        }
    }))
}

/// One revision-checked transaction covers a complete user operation, including
/// multi-field value saves. No partial values, availability or option edits commit.
pub(super) fn apply(
    conn: &mut Connection,
    entry: EntryId,
    expected: i64,
    command: FieldCommand,
) -> Result<EntryFields, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let snapshot = read(&tx, entry)?;
    if snapshot.global_revision != expected {
        return Err(PersistenceError::StaleRevision {
            expected,
            current: snapshot.global_revision,
        });
    }
    let now = Utc::now().to_rfc3339();
    match command {
        FieldCommand::Create {
            name: raw,
            field_kind,
            provider,
            options,
            value,
        } => {
            let field = FieldId::new();
            let label = name(&raw)?;
            if !matches!(field_kind, FieldKind::Choice | FieldKind::MultiChoice)
                && !options.is_empty()
            {
                return Err(invalid("Only choice fields have options"));
            }
            tx.execute("INSERT INTO field_definition (id,name,value_kind,created_at,updated_at,revision) VALUES (?1,?2,?3,?4,?4,1)",
                params![field.to_string(),label,field_kind.as_str(),now])?;
            bind(&tx, field, &provider)?;
            for label in options {
                add_option(&tx, field, &label, &now)?;
            }
            if let Some(value) = value {
                set_value(
                    &tx,
                    entry,
                    FieldEdit {
                        field_id: field,
                        value: Some(value),
                    },
                    &now,
                )?;
            }
        }
        FieldCommand::SetValues { edits } => {
            let mut seen = HashSet::new();
            for edit in edits {
                if !seen.insert(edit.field_id) {
                    return Err(invalid("Duplicate field edit"));
                }
                set_value(&tx, entry, edit, &now)?;
            }
        }
        FieldCommand::Rename {
            field_id,
            name: raw,
        } => {
            definition(&snapshot, field_id)?;
            tx.execute(
                "UPDATE field_definition SET name = ?1 WHERE id = ?2",
                params![name(&raw)?, field_id.to_string()],
            )?;
            touch(&tx, field_id, &now)?;
        }
        FieldCommand::SetRetired { field_id, retired } => {
            definition(&snapshot, field_id)?;
            tx.execute(
                "UPDATE field_definition SET retired_at = ?1 WHERE id = ?2",
                params![retired.then_some(&now), field_id.to_string()],
            )?;
            touch(&tx, field_id, &now)?;
        }
        FieldCommand::Bind { field_id, provider } => {
            if definition(&snapshot, field_id)?.retired {
                return Err(invalid(
                    "Restore this field before making it available for new use",
                ));
            }
            bind(&tx, field_id, &provider)?;
            touch(&tx, field_id, &now)?;
        }
        FieldCommand::Unbind { field_id, provider } => {
            definition(&snapshot, field_id)?;
            tx.execute("DELETE FROM field_availability WHERE field_id = ?1 AND provider_kind = ?2 AND provider_id = ?3",
                params![field_id.to_string(),provider.kind.as_str(),provider.id])?;
            touch(&tx, field_id, &now)?;
        }
        FieldCommand::AddChoice { field_id, label } => {
            let d = definition(&snapshot, field_id)?;
            if d.retired || !matches!(d.kind, FieldKind::Choice | FieldKind::MultiChoice) {
                return Err(invalid("Select an active choice field"));
            }
            add_option(&tx, field_id, &label, &now)?;
            touch(&tx, field_id, &now)?;
        }
        FieldCommand::RenameChoice { option_id, label } => {
            let field = option_field(&snapshot, option_id)?;
            tx.execute("UPDATE choice_option SET label = ?1, updated_at = ?2, revision = revision + 1 WHERE id = ?3",
                params![name(&label)?,now,option_id.to_string()])?;
            touch(&tx, field, &now)?;
        }
        FieldCommand::SetChoiceRetired { option_id, retired } => {
            let field = option_field(&snapshot, option_id)?;
            tx.execute("UPDATE choice_option SET retired_at = ?1, updated_at = ?2, revision = revision + 1 WHERE id = ?3",
                params![retired.then_some(&now),now,option_id.to_string()])?;
            touch(&tx, field, &now)?;
        }
    }
    tx.execute("UPDATE project_meta SET last_committed_revision = last_committed_revision + 1, updated_at = ?1 WHERE id = 1",[now])?;
    let updated = read(&tx, entry)?;
    tx.commit()?;
    Ok(updated)
}

fn definition(snapshot: &EntryFields, id: FieldId) -> Result<&FieldDefinition, PersistenceError> {
    snapshot
        .definitions
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| invalid("Field not found in this Project"))
}
fn option_field(snapshot: &EntryFields, id: ChoiceOptionId) -> Result<FieldId, PersistenceError> {
    snapshot
        .definitions
        .iter()
        .find(|d| d.options.iter().any(|o| o.id == id))
        .map(|d| d.id)
        .ok_or_else(|| invalid("Choice option not found in this Project"))
}
fn touch(tx: &Transaction<'_>, field: FieldId, now: &str) -> Result<(), PersistenceError> {
    tx.execute(
        "UPDATE field_definition SET updated_at = ?1, revision = revision + 1 WHERE id = ?2",
        params![now, field.to_string()],
    )?;
    Ok(())
}
fn bind(
    tx: &Transaction<'_>,
    field: FieldId,
    provider: &FieldProvider,
) -> Result<(), PersistenceError> {
    tx.execute("INSERT INTO field_availability (field_id,provider_kind,provider_id) VALUES (?1,?2,?3) ON CONFLICT DO NOTHING",
        params![field.to_string(),provider.kind.as_str(),provider.id])?;
    Ok(())
}
fn add_option(
    tx: &Transaction<'_>,
    field: FieldId,
    raw: &str,
    now: &str,
) -> Result<(), PersistenceError> {
    tx.execute("INSERT INTO choice_option (id,field_id,label,created_at,updated_at,revision) VALUES (?1,?2,?3,?4,?4,1)",
        params![ChoiceOptionId::new().to_string(),field.to_string(),name(raw)?,now])?;
    Ok(())
}
fn set_value(
    tx: &Transaction<'_>,
    entry: EntryId,
    edit: FieldEdit,
    now: &str,
) -> Result<(), PersistenceError> {
    let snapshot = read(tx, entry)?;
    let d = definition(&snapshot, edit.field_id)?;
    let field = snapshot
        .fields
        .iter()
        .find(|f| f.definition.id == edit.field_id)
        .ok_or_else(|| invalid("Field is not available on this Entry"))?;
    // Existing authored values remain editable after detachment or retirement.
    // Retirement cannot be used to start a new value on an empty Entry.
    if !field.available && field.value.is_none() {
        return Err(invalid("Field is not available for new values"));
    }
    let value = match edit.value {
        Some(FieldValue::Text(ref s)) if s.is_empty() => None,
        Some(FieldValue::Choices(ref ids)) if ids.is_empty() => None,
        other => other,
    };
    if let Some(ref value) = value {
        if !value.matches_kind(d.kind) {
            return Err(invalid(
                "Value does not match the field kind or cardinality",
            ));
        }
        if let FieldValue::Choices(ids) = value {
            let prior = match &field.value {
                Some(FieldValue::Choices(ids)) => ids.as_slice(),
                _ => &[],
            };
            let mut unique = HashSet::new();
            for id in ids {
                if !unique.insert(id) {
                    return Err(invalid("Duplicate choice selection"));
                }
                let option =
                    d.options.iter().find(|o| o.id == *id).ok_or_else(|| {
                        invalid("Choice option belongs to another field or Project")
                    })?;
                if option.retired && !prior.contains(id) {
                    return Err(invalid("Retired choice options cannot be newly selected"));
                }
            }
        }
    }
    let old_id: Option<String> = tx
        .query_row(
            "SELECT id FROM field_value WHERE entry_id = ?1 AND field_id = ?2",
            params![entry.to_string(), edit.field_id.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(ref id) = old_id {
        tx.execute("DELETE FROM field_choice_value WHERE value_id = ?1", [id])?;
    }
    let Some(value) = value else {
        if let Some(id) = old_id {
            tx.execute("DELETE FROM field_value WHERE id = ?1", [id])?;
        }
        return Ok(());
    };
    let value_id = old_id.unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
    let text = match &value {
        FieldValue::Text(s) => Some(s.as_str()),
        _ => None,
    };
    let number = match value {
        FieldValue::Number(n) => Some(n),
        _ => None,
    };
    let boolean = match value {
        FieldValue::Boolean(b) => Some(b),
        _ => None,
    };
    tx.execute("INSERT INTO field_value (id,entry_id,field_id,value_kind,text_value,number_value,bool_value,created_at,updated_at,revision)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,1) ON CONFLICT(entry_id,field_id,ordinal) DO UPDATE SET
        text_value=excluded.text_value,number_value=excluded.number_value,bool_value=excluded.bool_value,updated_at=excluded.updated_at,revision=field_value.revision+1",
        params![value_id,entry.to_string(),edit.field_id.to_string(),d.kind.as_str(),text,number,boolean,now])?;
    if let FieldValue::Choices(ids) = value {
        for id in ids {
            tx.execute(
                "INSERT INTO field_choice_value (value_id,field_id,option_id) VALUES (?1,?2,?3)",
                params![value_id, edit.field_id.to_string(), id.to_string()],
            )?;
        }
    }
    Ok(())
}
