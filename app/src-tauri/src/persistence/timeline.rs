use super::PersistenceError;
use crate::domain::{structure::OccurrenceId, timeline::*};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

fn invalid(message: impl ToString) -> PersistenceError {
    PersistenceError::Other(message.to_string())
}
fn revision(conn: &Connection) -> Result<i64, PersistenceError> {
    Ok(conn.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id=1",
        [],
        |r| r.get(0),
    )?)
}
fn calendar(conn: &Connection) -> Result<Option<Calendar>, PersistenceError> {
    let row = conn
        .query_row(
            "SELECT schema_version,config_json FROM timeline_calendar WHERE id=1",
            [],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?;
    row.map(|(version, json)| {
        if version != 1 {
            return Err(invalid(
                "This calendar needs a compatible Worldcrafter version. It has not been changed.",
            ));
        }
        let value: Calendar = serde_json::from_str(&json).map_err(invalid)?;
        value.validate().map_err(invalid)?;
        Ok(value)
    })
    .transpose()
}
fn links(
    conn: &Connection,
    id: OccurrenceId,
    chapter: bool,
) -> Result<Vec<TimelineLink>, PersistenceError> {
    let sql = if chapter {
        "SELECT s.id,CASE WHEN trim(s.title)='' THEN '[Untitled Chapter]' ELSE s.title END,i.workspace_state FROM occurrence_chapter l JOIN story_unit s ON s.id=l.chapter_id JOIN record_identity i ON i.record_id=s.id WHERE l.occurrence_id=?1 ORDER BY s.reading_rank"
    } else {
        "SELECT e.id,COALESCE(e.authored_name,'[Unnamed Entry]'),i.workspace_state FROM occurrence_entry l JOIN entry e ON e.id=l.entry_id JOIN record_identity i ON i.record_id=e.id WHERE l.occurrence_id=?1 ORDER BY e.authored_name,e.id"
    };
    let mut statement = conn.prepare(sql)?;
    let result = statement
        .query_map([id.to_string()], |r| {
            Ok(TimelineLink {
                id: r.get(0)?,
                label: r.get(1)?,
                workspace_state: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(result)
}
pub(super) fn read(conn: &Connection) -> Result<TimelineSnapshot, PersistenceError> {
    let calendar = calendar(conn)?;
    let occurrences = read_occurrences(conn)?;
    for occurrence in &occurrences {
        if let Some(date) = occurrence.date {
            date.validate(
                calendar
                    .as_ref()
                    .ok_or_else(|| invalid("Dated occurrences require their calendar"))?,
            )
            .map_err(invalid)?;
        }
    }
    Ok(TimelineSnapshot {
        global_revision: revision(conn)?,
        calendar,
        occurrences,
    })
}
// Search needs identities, writing and links, not an interpretation of calendar dates.
// Keep that read available if a calendar payload requires repair or a newer app.
pub(super) fn read_occurrences(conn: &Connection) -> Result<Vec<Occurrence>, PersistenceError> {
    let mut statement=conn.prepare("SELECT o.id,o.title,o.notes,o.year,o.month,o.day,o.event_entry_id,e.authored_name,ei.workspace_state,i.workspace_state FROM temporal_occurrence o JOIN record_identity i ON i.record_id=o.id LEFT JOIN entry e ON e.id=o.event_entry_id LEFT JOIN record_identity ei ON ei.record_id=e.id ORDER BY o.year IS NULL,o.year,o.month,o.day,o.id")?;
    let rows = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<i32>>(3)?,
                r.get::<_, Option<u16>>(4)?,
                r.get::<_, Option<u16>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<String>>(8)?,
                r.get::<_, String>(9)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut occurrences = Vec::new();
    for (id, title, notes, year, month, day, event, label, event_state, workspace_state) in rows {
        let id = OccurrenceId::parse(&id).map_err(invalid)?;
        let date = match (year, month, day) {
            (None, None, None) => None,
            (Some(year), Some(month), Some(day)) => Some(FictionalDate { year, month, day }),
            _ => {
                return Err(invalid(
                    "An occurrence has an incomplete date; its original data is preserved",
                ))
            }
        };
        occurrences.push(Occurrence {
            id,
            title,
            notes,
            date,
            event_entry: event.map(|id| TimelineLink {
                id,
                label: label.unwrap_or_else(|| "[Unnamed Entry]".into()),
                workspace_state: event_state.unwrap_or_else(|| "unresolved".into()),
            }),
            entries: links(conn, id, false)?,
            chapters: links(conn, id, true)?,
            workspace_state,
        });
    }
    Ok(occurrences)
}
fn require_target(
    conn: &Connection,
    id: &str,
    kind: &str,
    already_linked: bool,
) -> Result<(), PersistenceError> {
    let state = conn
        .query_row(
            "SELECT workspace_state FROM record_identity WHERE record_id=?1 AND kind=?2",
            params![id, kind],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    match state { Some(state) if state=="active" || already_linked=>Ok(()),_=>Err(invalid("Choose an active record in this Project; existing inactive links can be kept or removed")) }
}
pub(super) fn apply(
    conn: &mut Connection,
    expected: i64,
    command: TimelineCommand,
) -> Result<TimelineSnapshot, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current = revision(&tx)?;
    if current != expected {
        return Err(PersistenceError::StaleRevision { expected, current });
    }
    // Fail closed even for non-calendar commands if this calendar is corrupt/newer.
    let previous = read(&tx)?;
    let now = chrono::Utc::now().to_rfc3339();
    match command {
        TimelineCommand::ConfigureCalendar { calendar } => {
            calendar.validate().map_err(invalid)?;
            if previous.occurrences.iter().any(|o| o.date.is_some())
                && previous
                    .calendar
                    .as_ref()
                    .is_some_and(|old| !old.same_structure(&calendar))
            {
                return Err(invalid("Dated occurrences use this calendar. Month counts and lengths cannot change while dates exist, including in Archive or Trash. Labels can still be renamed."));
            }
            tx.execute("INSERT INTO timeline_calendar VALUES(1,1,?1,?2,?2,1) ON CONFLICT(id) DO UPDATE SET config_json=excluded.config_json,updated_at=excluded.updated_at,revision=timeline_calendar.revision+1",params![serde_json::to_string(&calendar).map_err(invalid)?,now])?;
        }
        TimelineCommand::Create => {
            let id = OccurrenceId::new().to_string();
            tx.execute(
                "INSERT INTO record_identity VALUES(?1,'temporal_occurrence','active',?2,?2)",
                params![id, now],
            )?;
            tx.execute(
                "INSERT INTO temporal_occurrence VALUES(?1,'','',NULL,NULL,NULL,NULL,?2,?2,1)",
                params![id, now],
            )?;
        }
        TimelineCommand::Save { id, draft } => {
            let old = previous
                .occurrences
                .iter()
                .find(|o| o.id == id)
                .ok_or_else(|| invalid("Occurrence not found in this Project"))?;
            if old.workspace_state != "active" {
                return Err(invalid("Restore this occurrence before editing it"));
            }
            if draft.title.chars().count() > 1000 || draft.notes.len() > 1_000_000 {
                return Err(invalid(
                    "Use up to 1,000 title characters and 1 MB of notes",
                ));
            }
            if draft.entry_ids.len() > 1000 || draft.chapter_ids.len() > 1000 {
                return Err(invalid(
                    "Use up to 1,000 Entry and Chapter links per occurrence",
                ));
            }
            if let Some(date) = draft.date {
                date.validate(
                    previous
                        .calendar
                        .as_ref()
                        .ok_or_else(|| invalid("Set up a calendar before adding dates"))?,
                )
                .map_err(invalid)?;
            }
            if let Some(event) = draft.event_entry_id {
                let key = event.to_string();
                require_target(
                    &tx,
                    &key,
                    "entry",
                    old.event_entry.as_ref().is_some_and(|e| e.id == key),
                )?;
                // Choosing a full event page is the explicit opt-in, independent of Category.
                tx.execute("INSERT INTO entry_capability VALUES(?1,'event',?2,?2,1) ON CONFLICT DO NOTHING",params![key,now])?;
            }
            for entry in &draft.entry_ids {
                let key = entry.to_string();
                require_target(&tx, &key, "entry", old.entries.iter().any(|e| e.id == key))?;
            }
            for chapter in &draft.chapter_ids {
                let key = chapter.to_string();
                require_target(
                    &tx,
                    &key,
                    "story_unit",
                    old.chapters.iter().any(|e| e.id == key),
                )?;
            }
            tx.execute("UPDATE temporal_occurrence SET title=?2,notes=?3,event_entry_id=?4,year=?5,month=?6,day=?7,updated_at=?8,revision=revision+1 WHERE id=?1",params![id.to_string(),draft.title,draft.notes,draft.event_entry_id.map(|e|e.to_string()),draft.date.map(|d|d.year),draft.date.map(|d|d.month),draft.date.map(|d|d.day),now])?;
            tx.execute(
                "DELETE FROM occurrence_entry WHERE occurrence_id=?1",
                [id.to_string()],
            )?;
            tx.execute(
                "DELETE FROM occurrence_chapter WHERE occurrence_id=?1",
                [id.to_string()],
            )?;
            for entry in draft.entry_ids {
                tx.execute(
                    "INSERT INTO occurrence_entry VALUES(?1,?2) ON CONFLICT DO NOTHING",
                    params![id.to_string(), entry.to_string()],
                )?;
            }
            for chapter in draft.chapter_ids {
                tx.execute(
                    "INSERT INTO occurrence_chapter VALUES(?1,?2) ON CONFLICT DO NOTHING",
                    params![id.to_string(), chapter.to_string()],
                )?;
            }
        }
        TimelineCommand::SetState { id, state } => {
            if !previous.occurrences.iter().any(|o| o.id == id) {
                return Err(invalid("Occurrence not found in this Project"));
            }
            tx.execute("UPDATE record_identity SET workspace_state=?2,lifecycle_changed_at=?3 WHERE record_id=?1",params![id.to_string(),state.as_str(),now])?;
            tx.execute(
                "UPDATE temporal_occurrence SET updated_at=?2,revision=revision+1 WHERE id=?1",
                params![id.to_string(), now],
            )?;
        }
    }
    tx.execute("UPDATE project_meta SET last_committed_revision=last_committed_revision+1,updated_at=?1 WHERE id=1",[now])?;
    let result = read(&tx)?;
    tx.commit()?;
    Ok(result)
}
