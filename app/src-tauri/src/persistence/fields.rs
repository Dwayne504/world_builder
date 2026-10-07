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
    read_snapshot(conn, Some(entry))
}
pub(super) fn read_catalog(conn: &Connection) -> Result<FieldCatalog, PersistenceError> {
    Ok(read_snapshot(conn, None)?.into())
}
fn read_snapshot(
    conn: &Connection,
    entry: Option<EntryId>,
) -> Result<EntryFields, PersistenceError> {
    let (category, type_id): (String, Option<String>) = if let Some(entry) = entry {
        conn.query_row(
            "SELECT category_id, type_id FROM entry WHERE id = ?1",
            [entry.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?
    } else {
        (String::new(), None)
    };
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
        "SELECT id, name, value_kind, retired_at IS NOT NULL, revision, unit
        FROM field_definition ORDER BY name COLLATE NOCASE, id",
    )?;
    let rows = statement.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, bool>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, Option<String>>(5)?,
        ))
    })?;
    let mut definitions = Vec::new();
    let mut fields = Vec::new();
    let has_projections: bool =
        conn.query_row("SELECT EXISTS(SELECT 1 FROM field_projection)", [], |r| {
            r.get(0)
        })?;
    let relationships = entry
        .filter(|_| has_projections)
        .map(|entry| super::relationships::read(conn, entry))
        .transpose()?;
    for row in rows {
        let (id, name, kind, retired, revision, unit) = row?;
        let field_id = FieldId::parse(&id).map_err(invalid)?;
        let projection = conn.query_row("SELECT relationship_definition_id,perspective FROM field_projection WHERE field_id=?1", [&id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).optional()?
            .map(|(definition, perspective)| -> Result<_, PersistenceError> { Ok(FieldProjection {
                relationship_definition_id: crate::domain::structure::RelationshipDefinitionId::parse(&definition).map_err(invalid)?,
                perspective: serde_json::from_value(serde_json::Value::String(perspective)).map_err(invalid)?,
            }) }).transpose()?;
        let kind = parse_kind(kind)?;
        if (kind == FieldKind::Relationship) != projection.is_some() {
            return Err(invalid(
                "Relationship Field projection metadata is missing or invalid",
            ));
        }
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
            kind,
            unit,
            retired,
            revision,
            options,
            bindings,
            projection,
        };
        if let Some(entry) = entry {
            let (hidden, removed): (bool, bool) = conn.query_row(
                "SELECT hidden, removed FROM entry_field_presentation WHERE entry_id=?1 AND field_id=?2",
                params![entry.to_string(), id], |r| Ok((r.get(0)?, r.get(1)?))
            ).optional()?.unwrap_or((false, false));
            let applicable = |b: &FieldBinding| match b.provider.kind {
                ProviderKind::Category => b.provider.id == category,
                ProviderKind::Type => ancestor_ids.contains(&b.provider.id),
                ProviderKind::Entry => b.provider.id == entry.to_string(),
            };
            let default_sources = definition
                .bindings
                .iter()
                .filter(|b| applicable(b))
                .filter(|b| b.provider.kind != ProviderKind::Entry)
                .cloned()
                .collect();
            let available = !retired && !removed && definition.bindings.iter().any(applicable);
            let value = read_value(conn, entry, &definition)?;
            let projected_relationships: Vec<_> = match (&definition.projection, &relationships) {
                (Some(projection), Some(relationships)) => {
                    let relationship_definition = relationships
                        .definitions
                        .iter()
                        .find(|d| d.id == projection.relationship_definition_id)
                        .ok_or_else(|| invalid("Relationship definition for Field is missing"))?;
                    relationships
                        .relationships
                        .iter()
                        .filter(|r| {
                            r.is_current()
                                && super::relationships::matches_projection(
                                    r,
                                    relationship_definition,
                                    entry,
                                    projection.perspective,
                                )
                        })
                        .cloned()
                        .collect()
                }
                _ => vec![],
            };
            let retained_projection = !removed
                && definition.bindings.iter().any(applicable)
                && !projected_relationships.is_empty();
            if available || value.is_some() || retained_projection {
                fields.push(EntryField {
                    definition: definition.clone(),
                    available,
                    hidden,
                    default_sources,
                    value,
                    projected_relationships,
                });
            }
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
        FieldKind::RichText => FieldValue::RichText(read_rich_value(conn, &id)?),
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
        FieldKind::Relationship => {
            return Err(invalid("Relationship Fields cannot contain scalar values"))
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
    apply_operation(conn, Some(entry), expected, command)
}

pub(super) fn apply_template(
    conn: &mut Connection,
    expected: i64,
    command: FieldCommand,
) -> Result<FieldCatalog, PersistenceError> {
    let provider =
        match &command {
            FieldCommand::Create {
                provider,
                value: None,
                ..
            }
            | FieldCommand::CreateProjection { provider, .. }
            | FieldCommand::Bind { provider, .. }
            | FieldCommand::Unbind { provider, .. } => provider,
            _ => return Err(invalid(
                "Template commands only configure default fields; they cannot write Entry values",
            )),
        };
    if provider.kind == ProviderKind::Entry {
        return Err(invalid("Select a Category or Type template"));
    }
    let table = if provider.kind == ProviderKind::Category {
        "category"
    } else {
        "type_def"
    };
    let exists: bool = conn.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
        [&provider.id],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(invalid("Template provider does not exist in this Project"));
    }
    Ok(apply_operation(conn, None, expected, command)?.into())
}

fn apply_operation(
    conn: &mut Connection,
    entry: Option<EntryId>,
    expected: i64,
    command: FieldCommand,
) -> Result<EntryFields, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let snapshot = read_snapshot(&tx, entry)?;
    if snapshot.global_revision != expected {
        return Err(PersistenceError::StaleRevision {
            expected,
            current: snapshot.global_revision,
        });
    }
    let now = Utc::now().to_rfc3339();
    match command {
        FieldCommand::CreateProjection {
            name: raw,
            relationship_definition_id,
            perspective,
            provider,
        } => {
            let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM relationship_definition WHERE id=?1 AND retired_at IS NULL)", [relationship_definition_id.to_string()], |r| r.get(0))?;
            if !exists {
                return Err(invalid(
                    "Choose an active relationship definition in this Project",
                ));
            }
            let field = FieldId::new();
            tx.execute("INSERT INTO field_definition(id,name,value_kind,created_at,updated_at,revision) VALUES(?1,?2,'relationship',?3,?3,1)",params![field.to_string(),name(&raw)?,now])?;
            tx.execute("INSERT INTO field_projection(field_id,relationship_definition_id,perspective,created_at,updated_at,revision) VALUES(?1,?2,?3,?4,?4,1)",params![field.to_string(),relationship_definition_id.to_string(),match perspective {crate::domain::relationships::Perspective::Source => "source",crate::domain::relationships::Perspective::Target => "target"},now])?;
            bind(&tx, field, &provider)?;
        }
        FieldCommand::EditProjection {
            field_id,
            other,
            instance_id,
        } => {
            let entry =
                entry.ok_or_else(|| invalid("Choose an Entry to edit a relationship Field"))?;
            let field = snapshot
                .fields
                .iter()
                .find(|f| f.definition.id == field_id && f.available)
                .ok_or_else(|| invalid("Relationship Field is not available on this Entry"))?;
            let projection = field
                .definition
                .projection
                .as_ref()
                .ok_or_else(|| invalid("Choose a relationship Field"))?;
            let selected = match instance_id {
                Some(id) if field.projected_relationships.iter().any(|r| r.id == id) => Some(id),
                Some(_) => return Err(invalid("Selected relationship is not current in this Field")),
                None => match field.projected_relationships.as_slice() {
                    [] => None,
                    [relationship] => Some(relationship.id),
                    _ => return Err(invalid("Multiple current relationships: choose an instance to edit, Replace, or Keep both")),
                },
            };
            if let Some(id) = selected {
                super::relationships::retarget(&tx, entry, projection, id, other, &now)?;
            } else {
                super::relationships::apply_in_transaction(
                    &tx,
                    entry,
                    crate::domain::relationships::RelationshipCommand::Connect {
                        definition_id: projection.relationship_definition_id,
                        perspective: projection.perspective,
                        other,
                        note: String::new(),
                        replace: vec![],
                    },
                    &now,
                )?;
            }
        }
        FieldCommand::RemoveProjection { field_id } => {
            let entry =
                entry.ok_or_else(|| invalid("Choose an Entry to remove a Field display"))?;
            if !snapshot
                .fields
                .iter()
                .any(|f| f.definition.id == field_id && f.definition.projection.is_some())
            {
                return Err(invalid("Relationship Field is not on this Entry"));
            }
            tx.execute("DELETE FROM field_availability WHERE provider_kind='entry' AND provider_id=?1 AND field_id=?2",params![entry.to_string(),field_id.to_string()])?;
            tx.execute("INSERT INTO entry_field_presentation(entry_id,field_id,hidden,removed,created_at,updated_at,revision) VALUES(?1,?2,0,1,?3,?3,1) ON CONFLICT(entry_id,field_id) DO UPDATE SET hidden=0,removed=1,updated_at=excluded.updated_at,revision=entry_field_presentation.revision+1",params![entry.to_string(),field_id.to_string(),now])?;
        }
        FieldCommand::SetHidden { field_id, hidden } => {
            let entry = entry.ok_or_else(|| invalid("Choose an Entry for local visibility"))?;
            if !snapshot.fields.iter().any(|f| f.definition.id == field_id) {
                return Err(invalid("Field is not on this Entry"));
            }
            tx.execute("INSERT INTO entry_field_presentation (entry_id,field_id,hidden,removed,created_at,updated_at,revision) VALUES (?1,?2,?3,0,?4,?4,1) ON CONFLICT(entry_id,field_id) DO UPDATE SET hidden=excluded.hidden,updated_at=excluded.updated_at,revision=entry_field_presentation.revision+1", params![entry.to_string(),field_id.to_string(),hidden,now])?;
        }
        FieldCommand::Create {
            name: raw,
            field_kind,
            unit,
            provider,
            options,
            value,
        } => {
            if field_kind == FieldKind::Relationship {
                return Err(invalid(
                    "Create relationship Fields with their projection configuration",
                ));
            }
            let field = FieldId::new();
            let label = name(&raw)?;
            let unit = unit
                .filter(|u| !u.trim().is_empty())
                .map(|u| name(&u))
                .transpose()?;
            if unit.is_some() && field_kind != FieldKind::Number {
                return Err(invalid("Only Number fields have units"));
            }
            if !matches!(field_kind, FieldKind::Choice | FieldKind::MultiChoice)
                && !options.is_empty()
            {
                return Err(invalid("Only choice fields have options"));
            }
            tx.execute("INSERT INTO field_definition (id,name,value_kind,created_at,updated_at,revision,unit) VALUES (?1,?2,?3,?4,?4,1,?5)",
                params![field.to_string(),label,field_kind.as_str(),now,unit])?;
            bind(&tx, field, &provider)?;
            for label in options {
                add_option(&tx, field, &label, &now)?;
            }
            if let Some(value) = value {
                set_value(
                    &tx,
                    entry.ok_or_else(|| invalid("An Entry is required for authored values"))?,
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
                set_value(
                    &tx,
                    entry.ok_or_else(|| invalid("An Entry is required for authored values"))?,
                    edit,
                    &now,
                )?;
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
            if provider.kind == ProviderKind::Entry {
                tx.execute("UPDATE entry_field_presentation SET hidden=0,removed=0,updated_at=?1,revision=revision+1 WHERE entry_id=?2 AND field_id=?3", params![now,provider.id,field_id.to_string()])?;
            }
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
    let updated = read_snapshot(&tx, entry)?;
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
    if d.projection.is_some() {
        return Err(invalid(
            "Relationship Fields edit canonical relationships, not scalar values",
        ));
    }
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
    if d.kind == FieldKind::RichText {
        return set_rich_value(
            tx,
            entry,
            edit.field_id,
            edit.value,
            field.value.as_ref(),
            now,
        );
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

/// Preview includes empty Entries whose available definitions will change, as
/// well as authored values retained after template detachment/retirement.
pub(super) fn preview_merge(
    conn: &Connection,
    source: FieldId,
    target: FieldId,
) -> Result<FieldMergePreview, PersistenceError> {
    if source == target {
        return Err(invalid("Choose two different Fields"));
    }
    let snapshot = read_snapshot(conn, None)?;
    let source = definition(&snapshot, source)?.clone();
    let target = definition(&snapshot, target)?.clone();
    let mut blockers = Vec::new();
    if source.projection.is_some() || target.projection.is_some() {
        blockers.push("Relationship Fields cannot be merged as scalar values. Their connections remain independently authored.".into());
    }
    if source.kind == FieldKind::RichText || target.kind == FieldKind::RichText {
        blockers.push("Rich Text Fields cannot be combined yet. Both documents are preserved; choose which Field to use without merging.".into());
    }
    if source.name.trim().to_lowercase() != target.name.trim().to_lowercase() {
        blockers.push("Only same-name duplicates can be merged. Rename deliberately before reviewing a merge.".into());
    }
    if source.kind != target.kind || source.unit != target.unit {
        blockers.push(
            "The Field kinds and units must match exactly. No conversion is performed.".into(),
        );
    }
    if matches!(source.kind, FieldKind::Choice | FieldKind::MultiChoice)
        || matches!(target.kind, FieldKind::Choice | FieldKind::MultiChoice)
    {
        blockers.push(
            "Choice Fields need an option-mapping review, which is not available yet.".into(),
        );
    }
    if source.retired || target.retired {
        blockers.push("Restore both Fields before merging them.".into());
    }
    let mut statement = conn.prepare("SELECT id, COALESCE(authored_name,'[Unnamed Entry]') FROM entry ORDER BY authored_name COLLATE NOCASE, id")?;
    let rows = statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut entries = Vec::new();
    for row in rows {
        let (id, name) = row?;
        let entry = EntryId::parse(&id).map_err(invalid)?;
        let fields = read_snapshot(conn, Some(entry))?;
        let src = fields.fields.iter().find(|f| f.definition.id == source.id);
        let dst = fields.fields.iter().find(|f| f.definition.id == target.id);
        // Local removals still change identity during a merge, even when both
        // Fields are currently absent from the reading surface.
        let has_presentation: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM entry_field_presentation WHERE entry_id=?1 AND field_id IN (?2,?3))",
            params![id, source.id.to_string(), target.id.to_string()], |r| r.get(0)
        )?;
        if src.is_none() && dst.is_none() && !has_presentation {
            continue;
        }
        let source_value = src.and_then(|f| f.value.clone());
        let target_value = dst.and_then(|f| f.value.clone());
        let conflict =
            source_value.is_some() && target_value.is_some() && source_value != target_value;
        entries.push(FieldMergeEntry {
            entry_id: entry,
            name,
            source_value,
            target_value,
            conflict,
        });
    }
    if entries.iter().any(|e| e.conflict) {
        blockers.push("Some Entries have different values. Review and resolve them before merging; neither value will be chosen automatically.".into());
    }
    Ok(FieldMergePreview {
        global_revision: snapshot.global_revision,
        source,
        target,
        entries,
        blockers,
    })
}

pub(super) fn merge(
    conn: &mut Connection,
    source: FieldId,
    target: FieldId,
    expected: i64,
) -> Result<i64, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let preview = preview_merge(&tx, source, target)?;
    if preview.global_revision != expected {
        return Err(PersistenceError::StaleRevision {
            expected,
            current: preview.global_revision,
        });
    }
    if !preview.blockers.is_empty() {
        return Err(invalid(preview.blockers.join(" ")));
    }
    let now = Utc::now().to_rfc3339();
    // Equal overlaps collapse only after explicit review and the service's
    // recovery backup. Non-overlapping value IDs and creation dates survive.
    for entry in &preview.entries {
        if entry.source_value.is_none() {
            continue;
        }
        if entry.target_value.is_some() {
            tx.execute(
                "DELETE FROM field_value WHERE entry_id=?1 AND field_id=?2",
                params![entry.entry_id.to_string(), source.to_string()],
            )?;
        } else {
            tx.execute("UPDATE field_value SET field_id=?1, updated_at=?2, revision=revision+1 WHERE entry_id=?3 AND field_id=?4", params![target.to_string(), now, entry.entry_id.to_string(), source.to_string()])?;
        }
    }
    for binding in &preview.source.bindings {
        bind(&tx, target, &binding.provider)?;
    }
    tx.execute(
        "DELETE FROM field_availability WHERE field_id=?1",
        [source.to_string()],
    )?;
    tx.execute("INSERT INTO entry_field_presentation (entry_id,field_id,hidden,removed,created_at,updated_at,revision) SELECT entry_id,?1,hidden,removed,created_at,?2,revision+1 FROM entry_field_presentation WHERE field_id=?3 ON CONFLICT(entry_id,field_id) DO NOTHING", params![target.to_string(),now,source.to_string()])?;
    tx.execute(
        "DELETE FROM entry_field_presentation WHERE field_id=?1",
        [source.to_string()],
    )?;
    tx.execute(
        "DELETE FROM field_definition WHERE id=?1",
        [source.to_string()],
    )?;
    touch(&tx, target, &now)?;
    tx.execute("UPDATE project_meta SET last_committed_revision=last_committed_revision+1, updated_at=?1 WHERE id=1", [now])?;
    tx.commit()?;
    Ok(expected + 1)
}

/// The application service creates a recovery snapshot before calling this.
/// Removal opts out of defaults locally; it never edits a shared definition.
pub(super) fn delete_local(
    conn: &mut Connection,
    entry: EntryId,
    field: FieldId,
    expected: i64,
) -> Result<EntryFields, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let snapshot = read(&tx, entry)?;
    if snapshot.global_revision != expected {
        return Err(PersistenceError::StaleRevision {
            expected,
            current: snapshot.global_revision,
        });
    }
    if !snapshot.fields.iter().any(|f| f.definition.id == field) {
        return Err(invalid("Field is no longer on this Entry"));
    }
    if definition(&snapshot, field)?.projection.is_some() {
        return Err(invalid("Remove the Field display or explicitly end its connections; scalar deletion cannot delete a relationship"));
    }
    if let Some(FieldValue::RichText(value)) = snapshot
        .fields
        .iter()
        .find(|f| f.definition.id == field)
        .and_then(|f| f.value.as_ref())
    {
        if let Some(reason) = &value.read_only_reason {
            return Err(invalid(reason));
        }
    }
    let now = Utc::now().to_rfc3339();
    tx.execute("DELETE FROM field_choice_value WHERE value_id IN (SELECT id FROM field_value WHERE entry_id=?1 AND field_id=?2)", params![entry.to_string(),field.to_string()])?;
    tx.execute(
        "DELETE FROM field_value WHERE entry_id=?1 AND field_id=?2",
        params![entry.to_string(), field.to_string()],
    )?;
    tx.execute("DELETE FROM field_availability WHERE provider_kind='entry' AND provider_id=?1 AND field_id=?2", params![entry.to_string(),field.to_string()])?;
    tx.execute("INSERT INTO entry_field_presentation (entry_id,field_id,hidden,removed,created_at,updated_at,revision) VALUES (?1,?2,0,1,?3,?3,1) ON CONFLICT(entry_id,field_id) DO UPDATE SET hidden=0,removed=1,updated_at=excluded.updated_at,revision=entry_field_presentation.revision+1", params![entry.to_string(),field.to_string(),now])?;
    tx.execute("UPDATE project_meta SET last_committed_revision=last_committed_revision+1,updated_at=?1 WHERE id=1", [now])?;
    let updated = read(&tx, entry)?;
    tx.commit()?;
    Ok(updated)
}

fn read_rich_value(conn: &Connection, value_id: &str) -> Result<RichFieldValue, PersistenceError> {
    let (schema_version, json, preserved, revision, migration_state): (i64, String, String, i64, String) = conn.query_row(
        "SELECT d.document_schema_version,d.canonical_json,d.plain_text,d.revision,d.migration_state FROM rich_document d JOIN field_value v ON v.document_id=d.id WHERE v.id=?1",
        [value_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
    let parsed = serde_json::from_str::<serde_json::Value>(&json);
    let validated = parsed
        .as_ref()
        .map_err(|_| {
            "This Rich Text value is damaged. Its original content is preserved.".to_string()
        })
        .and_then(|v| crate::domain::story::document_text(schema_version, v));
    let read_only_reason = if migration_state != "current" {
        Some("This Rich Text value needs recovery. Its original content is preserved.".into())
    } else {
        validated.as_ref().err().cloned()
    };
    Ok(RichFieldValue {
        schema_version,
        revision,
        plain_text: validated.map(|(text, _)| text).unwrap_or(preserved),
        content: if read_only_reason.is_none() {
            parsed.ok()
        } else {
            None
        },
        original_json: read_only_reason.as_ref().map(|_| json),
        read_only_reason,
    })
}

fn set_rich_value(
    tx: &Transaction<'_>,
    entry: EntryId,
    field: FieldId,
    submitted: Option<FieldValue>,
    prior: Option<&FieldValue>,
    now: &str,
) -> Result<(), PersistenceError> {
    let previous = match prior {
        Some(FieldValue::RichText(value)) => Some(value),
        _ => None,
    };
    if let Some(reason) = previous.and_then(|v| v.read_only_reason.as_ref()) {
        return Err(invalid(reason));
    }
    // Creation of an empty default has no document. Existing writing can only
    // be cleared with a validated, revision-checked empty document.
    let Some(FieldValue::RichText(value)) = submitted else {
        return if previous.is_none() && submitted.is_none() {
            Ok(())
        } else {
            Err(invalid(
                "Rich Text edits require a document and its saved revision",
            ))
        };
    };
    if value.revision != previous.map_or(0, |v| v.revision) {
        return Err(PersistenceError::StaleDocumentRevision);
    }
    let active: bool = tx.query_row(
        "SELECT workspace_state='active' FROM record_identity WHERE record_id=?1",
        [entry.to_string()],
        |r| r.get(0),
    )?;
    if !active {
        return Err(invalid(
            "Restore this Entry before editing its Rich Text Fields",
        ));
    }
    let content = value
        .content
        .ok_or_else(|| invalid("Rich Text content is required"))?;
    let (plain, count) =
        crate::domain::story::document_text(value.schema_version, &content).map_err(invalid)?;
    // Keep formatting-only documents; only the exact empty editor forms clear.
    let empty = content == serde_json::json!({"type":"doc","content":[{"type":"paragraph"}]})
        || content == serde_json::json!({"type":"doc","content":[]})
        || content == serde_json::json!({"type":"doc"});
    if empty {
        tx.execute(
            "DELETE FROM field_value WHERE entry_id=?1 AND field_id=?2",
            params![entry.to_string(), field.to_string()],
        )?;
        return Ok(());
    }
    let area = format!("field:{field}");
    // Use the next Project revision so a cleared/recreated value cannot reuse
    // an older document revision and accept an unrelated stale editor (ABA).
    let document_revision: i64 = tx.query_row(
        "SELECT last_committed_revision+1 FROM project_meta WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    tx.execute("INSERT INTO rich_document(id,owner_kind,owner_id,area,document_schema_version,canonical_json,plain_text,word_count,migration_state,created_at,updated_at,revision) VALUES(?1,'entry',?2,?3,?4,?5,?6,?7,'current',?8,?8,?9) ON CONFLICT(owner_kind,owner_id,area) DO UPDATE SET canonical_json=excluded.canonical_json,plain_text=excluded.plain_text,word_count=excluded.word_count,updated_at=excluded.updated_at,revision=excluded.revision",
        params![uuid::Uuid::now_v7().to_string(),entry.to_string(),area,value.schema_version,content.to_string(),plain,count,now,document_revision])?;
    let document_id: String = tx.query_row(
        "SELECT id FROM rich_document WHERE owner_kind='entry' AND owner_id=?1 AND area=?2",
        params![entry.to_string(), area],
        |r| r.get(0),
    )?;
    tx.execute("INSERT INTO field_value(id,entry_id,field_id,document_id,value_kind,created_at,updated_at,revision) VALUES(?1,?2,?3,?4,'rich_text',?5,?5,1) ON CONFLICT(entry_id,field_id,ordinal) DO UPDATE SET updated_at=excluded.updated_at,revision=field_value.revision+1",
        params![uuid::Uuid::now_v7().to_string(),entry.to_string(),field.to_string(),document_id,now])?;
    Ok(())
}
