//! Relationship records and both Entry perspectives share one revision-checked transaction.
use super::PersistenceError;
use crate::domain::{
    authored_name,
    relationships::*,
    structure::{RelationshipDefinitionId, RelationshipId},
    EntryId,
};
use chrono::Utc;
use rusqlite::{params, Connection, TransactionBehavior};
use std::collections::HashSet;

fn invalid(message: impl ToString) -> PersistenceError {
    PersistenceError::Other(message.to_string())
}

fn participant(conn: &Connection, id: &str, slot: &str) -> Result<Participant, PersistenceError> {
    let (record, kind, snapshot, label, state): (Option<String>, String, Option<String>, Option<String>, Option<String>) = conn.query_row(
        "SELECT p.record_id,p.record_kind,p.unresolved_snapshot,COALESCE(e.authored_name,'[Unnamed Entry]'),i.workspace_state
         FROM relationship_participant p LEFT JOIN record_identity i ON i.record_id=p.record_id
         LEFT JOIN entry e ON e.id=p.record_id WHERE p.instance_id=?1 AND p.slot=?2",
        params![id,slot], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
    if kind != "entry" {
        return Err(invalid("This build cannot display this participant kind"));
    }
    match record {
        Some(id) => Ok(Participant {
            id: Some(EntryId::parse(&id).map_err(invalid)?),
            label: label.unwrap_or_else(|| "[Unnamed Entry]".into()),
            workspace_state: state.ok_or_else(|| invalid("Missing participant identity"))?,
        }),
        None => {
            let snapshot: serde_json::Value = serde_json::from_str(
                &snapshot.ok_or_else(|| invalid("Missing unresolved participant snapshot"))?,
            )
            .map_err(invalid)?;
            Ok(Participant {
                id: None,
                label: snapshot
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("[Missing Entry]")
                    .into(),
                workspace_state: "missing".into(),
            })
        }
    }
}

fn all(conn: &Connection) -> Result<Vec<Relationship>, PersistenceError> {
    let mut statement = conn.prepare("SELECT r.id,r.definition_id,r.note,r.semantic_state='ended',i.workspace_state,r.revision FROM relationship_instance r JOIN record_identity i ON i.record_id=r.id ORDER BY r.created_at,r.id")?;
    let rows = statement.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, bool>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, i64>(5)?,
        ))
    })?;
    rows.map(|row| {
        let (id, definition, note, ended, workspace_state, revision) = row?;
        Ok(Relationship {
            id: RelationshipId::parse(&id).map_err(invalid)?,
            definition_id: RelationshipDefinitionId::parse(&definition).map_err(invalid)?,
            source: participant(conn, &id, "source")?,
            target: participant(conn, &id, "target")?,
            note,
            ended,
            workspace_state,
            revision,
            warnings: vec![],
        })
    })
    .collect()
}

pub(super) fn read(
    conn: &Connection,
    entry: EntryId,
) -> Result<EntryRelationships, PersistenceError> {
    conn.query_row(
        "SELECT id FROM entry WHERE id=?1",
        [entry.to_string()],
        |_| Ok(()),
    )?;
    read_snapshot(conn, Some(entry))
}

pub(super) fn read_project(conn: &Connection) -> Result<RelationshipSnapshot, PersistenceError> {
    read_snapshot(conn, None)
}

fn read_snapshot(
    conn: &Connection,
    entry: Option<EntryId>,
) -> Result<RelationshipSnapshot, PersistenceError> {
    let global_revision = conn.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    let mut statement = conn.prepare("SELECT id,name,forward_label,inverse_label,directed,expected_targets_per_source,expected_sources_per_target,retired_at IS NOT NULL,revision FROM relationship_definition ORDER BY name COLLATE NOCASE,id")?;
    let definitions = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                DefinitionDraft {
                    name: r.get(1)?,
                    forward_label: r.get(2)?,
                    inverse_label: r.get(3)?,
                    directed: r.get(4)?,
                    expected_targets_per_source: r.get(5)?,
                    expected_sources_per_target: r.get(6)?,
                },
                r.get::<_, bool>(7)?,
                r.get::<_, i64>(8)?,
            ))
        })?
        .map(|row| {
            let (id, draft, retired, revision) = row?;
            Ok(RelationshipDefinition {
                id: RelationshipDefinitionId::parse(&id).map_err(invalid)?,
                draft,
                retired,
                revision,
            })
        })
        .collect::<Result<Vec<_>, PersistenceError>>()?;
    let connections = all(conn)?;
    let mut relationships: Vec<_> = connections
        .iter()
        .filter(|r| entry.is_none_or(|id| r.involves(id)))
        .cloned()
        .collect();
    for r in &mut relationships {
        if !r.is_current() {
            continue;
        }
        let d = definitions
            .iter()
            .find(|d| d.id == r.definition_id)
            .ok_or_else(|| invalid("Missing relationship definition"))?;
        for (p, source, limit, label) in [
            (
                &r.source,
                true,
                d.draft.expected_targets_per_source,
                &d.draft.forward_label,
            ),
            (
                &r.target,
                false,
                d.draft.expected_sources_per_target,
                &d.draft.inverse_label,
            ),
        ] {
            if let (Some(id), Some(limit)) = (p.id, limit) {
                let count = connections
                    .iter()
                    .filter(|other| {
                        other.is_current()
                            && other.definition_id == d.id
                            && if d.draft.directed {
                                if source {
                                    other.source.id == Some(id)
                                } else {
                                    other.target.id == Some(id)
                                }
                            } else {
                                other.involves(id)
                            }
                    })
                    .count();
                if count > limit as usize {
                    r.warnings.push(format!(
                        "{} — {}: expected at most {}; found {}.",
                        p.label, label, limit, count
                    ));
                }
            }
        }
        r.warnings.dedup();
    }
    let mut statement = conn.prepare("SELECT e.id,COALESCE(e.authored_name,'[Unnamed Entry]'),c.name FROM entry e JOIN category c ON c.id=e.category_id JOIN record_identity i ON i.record_id=e.id WHERE i.workspace_state='active' ORDER BY e.authored_name COLLATE NOCASE,e.id")?;
    let entries = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .map(|row| {
            let (id, label, category_name) = row?;
            Ok(RelationshipEntry {
                id: EntryId::parse(&id).map_err(invalid)?,
                label,
                category_name,
            })
        })
        .collect::<Result<_, PersistenceError>>()?;
    Ok(EntryRelationships {
        global_revision,
        definitions,
        relationships,
        entries,
    })
}

fn definition(
    snapshot: &EntryRelationships,
    id: RelationshipDefinitionId,
) -> Result<&RelationshipDefinition, PersistenceError> {
    snapshot
        .definitions
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| invalid("Relationship definition not found in this Project"))
}
fn relationship(
    snapshot: &EntryRelationships,
    id: RelationshipId,
) -> Result<&Relationship, PersistenceError> {
    snapshot
        .relationships
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| invalid("Relationship does not belong to this Entry"))
}
fn active_entry(conn: &Connection, entry: EntryId) -> Result<(), PersistenceError> {
    let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM entry e JOIN record_identity i ON i.record_id=e.id WHERE e.id=?1 AND i.kind='entry' AND i.workspace_state='active')",[entry.to_string()],|r| r.get(0))?;
    if !exists {
        return Err(invalid("Choose an active Entry in this Project"));
    }
    Ok(())
}
fn ensure_unique(
    conn: &Connection,
    d: &RelationshipDefinition,
    source: EntryId,
    target: EntryId,
    except: Option<RelationshipId>,
) -> Result<(), PersistenceError> {
    if all(conn)?.iter().any(|r| {
        r.is_current()
            && Some(r.id) != except
            && r.definition_id == d.id
            && ((r.source.id == Some(source) && r.target.id == Some(target))
                || (!d.draft.directed
                    && r.source.id == Some(target)
                    && r.target.id == Some(source)))
    }) {
        return Err(invalid(
            "This active relationship already exists. Open it to edit its note.",
        ));
    }
    Ok(())
}
fn set_ended(
    conn: &Connection,
    id: RelationshipId,
    ended: bool,
    now: &str,
) -> Result<(), PersistenceError> {
    conn.execute("UPDATE relationship_instance SET semantic_state=?1,ended_at=?2,updated_at=?3,revision=revision+1 WHERE id=?4",params![if ended {"ended"} else {"active"},ended.then_some(now),now,id.to_string()])?;
    Ok(())
}

pub(super) fn apply(
    conn: &mut Connection,
    entry: EntryId,
    expected: i64,
    command: RelationshipCommand,
) -> Result<EntryRelationships, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let snapshot = read(&tx, entry)?;
    if expected != snapshot.global_revision {
        return Err(PersistenceError::StaleRevision {
            expected,
            current: snapshot.global_revision,
        });
    }
    let now = Utc::now().to_rfc3339();
    apply_in_transaction(&tx, entry, command, &now)?;
    tx.execute("UPDATE project_meta SET last_committed_revision=last_committed_revision+1,updated_at=?1 WHERE id=1",[now])?;
    let updated = read(&tx, entry)?;
    tx.commit()?;
    Ok(updated)
}

/// Shares the caller's revision-checked transaction; never commits a second store.
pub(super) fn apply_in_transaction(
    conn: &Connection,
    entry: EntryId,
    command: RelationshipCommand,
    now: &str,
) -> Result<(), PersistenceError> {
    let snapshot = read(conn, entry)?;
    match command {
        RelationshipCommand::CreateDefinition { draft } => {
            let d = draft.validated().map_err(invalid)?;
            conn.execute("INSERT INTO relationship_definition(id,name,forward_label,inverse_label,directed,expected_targets_per_source,expected_sources_per_target,created_at,updated_at,revision) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8,1)",params![RelationshipDefinitionId::new().to_string(),d.name,d.forward_label,d.inverse_label,d.directed,d.expected_targets_per_source,d.expected_sources_per_target,now])?;
        }
        RelationshipCommand::UpdateDefinition {
            definition_id,
            draft,
        } => {
            let old = definition(&snapshot, definition_id)?;
            let d = draft.validated().map_err(invalid)?;
            if old.draft.directed != d.directed {
                return Err(invalid(
                    "Direction cannot change on an existing relationship definition",
                ));
            }
            conn.execute("UPDATE relationship_definition SET name=?1,forward_label=?2,inverse_label=?3,expected_targets_per_source=?4,expected_sources_per_target=?5,updated_at=?6,revision=revision+1 WHERE id=?7",params![d.name,d.forward_label,d.inverse_label,d.expected_targets_per_source,d.expected_sources_per_target,now,definition_id.to_string()])?;
        }
        RelationshipCommand::RetireDefinition {
            definition_id,
            retired,
        } => {
            definition(&snapshot, definition_id)?;
            conn.execute("UPDATE relationship_definition SET retired_at=?1,updated_at=?2,revision=revision+1 WHERE id=?3",params![retired.then_some(now),now,definition_id.to_string()])?;
        }
        RelationshipCommand::Connect {
            definition_id,
            perspective,
            other,
            note,
            replace,
        } => {
            let d = definition(&snapshot, definition_id)?;
            if d.retired {
                return Err(invalid(
                    "Restore this definition before creating relationships",
                ));
            }
            active_entry(conn, entry)?;
            let other = resolve_other(conn, other, now)?;
            let mut seen = HashSet::new();
            for id in replace {
                let r = relationship(&snapshot, id)?;
                let matches_side = if !d.draft.directed {
                    r.involves(entry)
                } else {
                    match perspective {
                        Perspective::Source => r.source.id == Some(entry),
                        Perspective::Target => r.target.id == Some(entry),
                    }
                };
                if !seen.insert(id) || !r.is_current() || r.definition_id != d.id || !matches_side {
                    return Err(invalid(
                        "Choose current relationships on this side to replace",
                    ));
                }
                set_ended(conn, id, true, now)?;
            }
            let (mut source, mut target) = match perspective {
                Perspective::Source => (entry, other),
                Perspective::Target => (other, entry),
            };
            if !d.draft.directed && source.to_string() > target.to_string() {
                std::mem::swap(&mut source, &mut target);
            }
            ensure_unique(conn, d, source, target, None)?;
            let id = RelationshipId::new().to_string();
            conn.execute("INSERT INTO record_identity(record_id,kind,workspace_state,lifecycle_changed_at,created_at) VALUES(?1,'relationship_instance','active',?2,?2)",params![id,now])?;
            conn.execute("INSERT INTO relationship_instance(id,definition_id,semantic_state,note,created_at,updated_at,revision) VALUES(?1,?2,'active',?3,?4,?4,1)",params![id,d.id.to_string(),note,now])?;
            for (slot, record) in [("source", source), ("target", target)] {
                conn.execute("INSERT INTO relationship_participant(instance_id,slot,record_id,record_kind) VALUES(?1,?2,?3,'entry')",params![id,slot,record.to_string()])?;
            }
        }
        RelationshipCommand::SetNote { id, note } => {
            relationship(&snapshot, id)?;
            conn.execute("UPDATE relationship_instance SET note=?1,updated_at=?2,revision=revision+1 WHERE id=?3",params![note,now,id.to_string()])?;
        }
        RelationshipCommand::SetEnded { id, ended } => {
            let r = relationship(&snapshot, id)?;
            if !ended {
                let d = definition(&snapshot, r.definition_id)?;
                if d.retired || r.workspace_state != "active" {
                    return Err(invalid("Restore the definition and workspace record first"));
                }
                let source = r
                    .source
                    .id
                    .ok_or_else(|| invalid("The source Entry is missing"))?;
                let target = r
                    .target
                    .id
                    .ok_or_else(|| invalid("The target Entry is missing"))?;
                active_entry(conn, source)?;
                active_entry(conn, target)?;
                ensure_unique(conn, d, source, target, Some(id))?;
            }
            set_ended(conn, id, ended, now)?;
        }
    }
    Ok(())
}

pub(super) fn retarget(
    conn: &Connection,
    entry: EntryId,
    projection: &crate::domain::fields::FieldProjection,
    id: RelationshipId,
    other: OtherEntry,
    now: &str,
) -> Result<(), PersistenceError> {
    let snapshot = read(conn, entry)?;
    let d = definition(&snapshot, projection.relationship_definition_id)?;
    if d.retired {
        return Err(invalid(
            "Restore this relationship definition before changing targets",
        ));
    }
    let r = relationship(&snapshot, id)?;
    if !r.is_current() || !matches_projection(r, d, entry, projection.perspective) {
        return Err(invalid(
            "Choose a current relationship belonging to this Field",
        ));
    }
    active_entry(conn, entry)?;
    let other = resolve_other(conn, other, now)?;
    let (mut source, mut target) = match projection.perspective {
        Perspective::Source => (entry, other),
        Perspective::Target => (other, entry),
    };
    if !d.draft.directed && source.to_string() > target.to_string() {
        std::mem::swap(&mut source, &mut target);
    }
    ensure_unique(conn, d, source, target, Some(id))?;
    for (slot, record) in [("source", source), ("target", target)] {
        conn.execute("UPDATE relationship_participant SET record_id=?1,record_kind='entry',unresolved_snapshot=NULL WHERE instance_id=?2 AND slot=?3", params![record.to_string(),id.to_string(),slot])?;
    }
    conn.execute(
        "UPDATE relationship_instance SET updated_at=?1,revision=revision+1 WHERE id=?2",
        params![now, id.to_string()],
    )?;
    Ok(())
}

pub(super) fn matches_projection(
    r: &Relationship,
    d: &RelationshipDefinition,
    entry: EntryId,
    perspective: Perspective,
) -> bool {
    r.definition_id == d.id
        && if !d.draft.directed {
            r.involves(entry)
        } else {
            match perspective {
                Perspective::Source => r.source.id == Some(entry),
                Perspective::Target => r.target.id == Some(entry),
            }
        }
}

fn resolve_other(
    conn: &Connection,
    other: OtherEntry,
    now: &str,
) -> Result<EntryId, PersistenceError> {
    let id = match other {
        OtherEntry::Existing { id } => {
            active_entry(conn, id)?;
            id
        }
        OtherEntry::Create { name, category_id } => {
            let category = match category_id {
                Some(id) => id.to_string(),
                None => conn.query_row(
                    "SELECT id FROM category WHERE is_uncategorized=1",
                    [],
                    |r| r.get(0),
                )?,
            };
            let name = authored_name(name);
            let id = EntryId::new();
            conn.execute("INSERT INTO record_identity(record_id,kind,workspace_state,lifecycle_changed_at,created_at) VALUES(?1,'entry','active',?2,?2)",params![id.to_string(),now])?;
            conn.execute("INSERT INTO entry(id,category_id,authored_name,created_at,updated_at,revision) VALUES(?1,?2,?3,?4,?4,1)",params![id.to_string(),category,name,now])?;
            id
        }
    };
    Ok(id)
}
