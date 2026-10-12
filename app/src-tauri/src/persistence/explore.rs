use super::PersistenceError;
use crate::domain::{explore::*, relationships::Perspective, search::normalize_search};
use rusqlite::{params, Connection, OptionalExtension};

fn invalid(message: &str) -> PersistenceError {
    PersistenceError::Other(message.into())
}

fn entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExploreEntry> {
    Ok(ExploreEntry {
        id: row.get(0)?,
        name: row.get(1)?,
        category: row.get(2)?,
        type_name: row.get(3)?,
        workspace_state: row.get(4)?,
        spatial: row.get(5)?,
        relationship_match: None,
    })
}

const ENTRY_COLUMNS: &str = "e.id,COALESCE(e.authored_name,'[Unnamed Entry]'),c.name,t.name,i.workspace_state,EXISTS(SELECT 1 FROM spatial_node WHERE entry_id=e.id)";
const ENTRY_JOINS: &str = "FROM entry e JOIN category c ON c.id=e.category_id LEFT JOIN type_def t ON t.id=e.type_id JOIN record_identity i ON i.record_id=e.id";

pub(super) fn query(
    conn: &Connection,
    request: ExploreRequest,
) -> Result<ExploreResults, PersistenceError> {
    let tx = conn.unchecked_transaction()?;
    let result = read(&tx, request)?;
    tx.commit()?;
    Ok(result)
}

fn read(conn: &Connection, request: ExploreRequest) -> Result<ExploreResults, PersistenceError> {
    if request.query.chars().count() > 200
        || !(1..=100).contains(&request.page_size)
        || request.page.checked_mul(request.page_size).is_none()
    {
        return Err(invalid(
            "Explore needs a search of at most 200 characters and 1–100 results per page",
        ));
    }
    if !["active", "archived", "trashed", "all"].contains(&request.workspace_state.as_str()) {
        return Err(invalid("Choose Active, Archived, Trash, or All Entries"));
    }
    let global_revision = conn.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    let categories = conn
        .prepare("SELECT id,name,NULL FROM category ORDER BY name COLLATE NOCASE,id")?
        .query_map([], |r| {
            Ok(ExploreChoice {
                id: r.get(0)?,
                name: r.get(1)?,
                category_id: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let types = conn
        .prepare("SELECT id,name,category_id FROM type_def ORDER BY name COLLATE NOCASE,id")?
        .query_map([], |r| {
            Ok(ExploreChoice {
                id: r.get(0)?,
                name: r.get(1)?,
                category_id: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let capabilities = conn
        .prepare("SELECT id FROM capability_def ORDER BY id")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let definitions = conn.prepare("SELECT id,name,forward_label,inverse_label,directed,retired_at IS NOT NULL FROM relationship_definition ORDER BY name COLLATE NOCASE,id")?.query_map([], |r| Ok(ExploreDefinition { id:r.get(0)?, name:r.get(1)?, forward_label:r.get(2)?, inverse_label:r.get(3)?, directed:r.get(4)?, retired:r.get(5)? }))?.collect::<Result<Vec<_>,_>>()?;
    let mut result = ExploreResults {
        global_revision,
        categories,
        types,
        capabilities,
        definitions,
        selected_other: None,
        issues: vec![],
        entries: vec![],
        total: 0,
        page: request.page,
        page_size: request.page_size,
    };
    let category = request.category_id.map(|id| id.to_string());
    let type_id = request.type_id.map(|id| id.to_string());
    if category
        .as_ref()
        .is_some_and(|id| !result.categories.iter().any(|c| &c.id == id))
    {
        result.issues.push(
            "The selected Category is unavailable. Choose another or clear this filter.".into(),
        );
    }
    if let Some(id) = &type_id {
        match result.types.iter().find(|t| &t.id == id) {
            None => result.issues.push(
                "The selected Type is unavailable. Choose another or clear this filter.".into(),
            ),
            Some(t) if category.is_some() && t.category_id != category => result.issues.push(
                "The selected Type belongs to a different Category. Adjust either filter.".into(),
            ),
            _ => (),
        }
    }
    if request
        .capability
        .as_ref()
        .is_some_and(|id| !result.capabilities.contains(id))
    {
        result.issues.push(
            "The selected capability is unavailable. Choose another or clear this filter.".into(),
        );
    }
    let mut definition = None;
    let mut directed = true;
    let mut perspective = "source";
    let mut other = None;
    let mut contained = false;
    if let Some(filter) = &request.relationship {
        definition = Some(filter.definition_id.to_string());
        match result.definitions.iter().find(|d| Some(&d.id)==definition.as_ref()) {
            Some(d) if !d.retired => directed=d.directed,
            _ => result.issues.push("The selected relationship is unavailable or retired. Choose another or clear this filter.".into()),
        }
        perspective = match filter.perspective {
            Perspective::Source => "source",
            Perspective::Target => "target",
        };
        other = filter.other_entry_id.map(|id| id.to_string());
        contained = filter.include_contained;
        if let Some(id) = &other {
            result.selected_other = conn
                .query_row(
                    &format!("SELECT {ENTRY_COLUMNS} {ENTRY_JOINS} WHERE e.id=?1"),
                    [id],
                    entry,
                )
                .optional()?;
            if result.selected_other.is_none() {
                result.issues.push(
                    "The selected other Entry is unavailable. Choose another or clear this filter."
                        .into(),
                );
            }
        }
        if contained && !result.selected_other.as_ref().is_some_and(|e| e.spatial) {
            result
                .issues
                .push("Contained locations need a selected Spatial Entry.".into());
        }
    }
    // Invalid identities remain visible in the response; they never become unfiltered queries.
    if !result.issues.is_empty() {
        return Ok(result);
    }
    // Recursive traversal follows only the canonical primary Spatial tree. Workspace state of
    // intermediate places does not sever containment; the result Entry state is independent.
    let sql = format!("WITH RECURSIVE targets(id) AS (
        SELECT ?6 WHERE ?6 IS NOT NULL UNION SELECT s.entry_id FROM spatial_node s JOIN targets t ON s.primary_parent_id=t.id WHERE ?7
    ), matching(entry_id,other_id) AS (
        SELECT p.record_id,q.record_id FROM relationship_instance r
        JOIN record_identity ri ON ri.record_id=r.id
        JOIN relationship_participant p ON p.instance_id=r.id
        JOIN relationship_participant q ON q.instance_id=r.id AND q.slot<>p.slot
        JOIN entry opposite ON opposite.id=q.record_id
        WHERE r.definition_id=?4 AND r.semantic_state='active' AND ri.workspace_state='active'
        AND p.record_kind='entry' AND q.record_kind='entry' AND (?8=0 OR p.slot=?5)
        AND (?6 IS NULL OR q.record_id IN (SELECT id FROM targets))
    ) SELECT {ENTRY_COLUMNS},
        (SELECT json_group_array(alias_text) FROM entry_alias WHERE entry_id=e.id),
        EXISTS(SELECT 1 FROM matching WHERE entry_id=e.id AND other_id=?6)
        {ENTRY_JOINS}
        WHERE (?1 IS NULL OR e.category_id=?1) AND (?2 IS NULL OR e.type_id=?2)
        AND (?3 IS NULL OR EXISTS(SELECT 1 FROM entry_capability WHERE entry_id=e.id AND capability_id=?3))
        AND (?9='all' OR i.workspace_state=?9)
        AND (?4 IS NULL OR EXISTS(SELECT 1 FROM matching WHERE entry_id=e.id))
        ORDER BY COALESCE(e.authored_name,'[Unnamed Entry]') COLLATE NOCASE,e.id");
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(
        params![
            category,
            type_id,
            request.capability,
            definition,
            perspective,
            other,
            contained,
            directed,
            request.workspace_state
        ],
        |r| Ok((entry(r)?, r.get::<_, String>(6)?, r.get::<_, bool>(7)?)),
    )?;
    let needle = normalize_search(&request.query);
    let offset = request.page * request.page_size;
    // Stream candidates: only one page is retained. Unicode matching uses the same normalization
    // as Search, without relying on SQLite's ASCII-only lower()/NOCASE for authored names.
    for row in rows {
        let (mut candidate, aliases, direct) = row?;
        let aliases: Vec<String> =
            serde_json::from_str(&aliases).map_err(|e| PersistenceError::Other(e.to_string()))?;
        if !needle.is_empty()
            && !normalize_search(&candidate.name).contains(&needle)
            && !aliases
                .iter()
                .any(|a| normalize_search(a).contains(&needle))
        {
            continue;
        }
        if result.total >= offset && result.entries.len() < request.page_size {
            if definition.is_some() {
                candidate.relationship_match = Some(
                    if other.is_none() {
                        "current"
                    } else if direct {
                        "direct"
                    } else {
                        "contained"
                    }
                    .into(),
                );
            }
            result.entries.push(candidate);
        }
        result.total += 1;
    }
    Ok(result)
}
