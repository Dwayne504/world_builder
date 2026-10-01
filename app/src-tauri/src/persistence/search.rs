use super::PersistenceError;
use crate::domain::{search::*, EntryId};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const INDEX_VERSION: i64 = 1;
const CREATE_INDEX: &str = "CREATE VIRTUAL TABLE search_index USING fts5(payload UNINDEXED, terms, tokenize='unicode61 remove_diacritics 0')";
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

pub(super) fn aliases(conn: &Connection, entry: EntryId) -> Result<EntryAliases, PersistenceError> {
    if !conn
        .prepare("SELECT 1 FROM entry WHERE id=?1")?
        .exists([entry.to_string()])?
    {
        return Err(invalid("Entry not found in this Project"));
    }
    let aliases = conn
        .prepare(
            "SELECT id,alias_text FROM entry_alias WHERE entry_id=?1 ORDER BY normalized_alias,id",
        )?
        .query_map([entry.to_string()], |r| {
            Ok(EntryAlias {
                id: r.get(0)?,
                text: r.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(EntryAliases {
        global_revision: revision(conn)?,
        aliases,
    })
}
pub(super) fn apply_alias(
    conn: &mut Connection,
    entry: EntryId,
    expected: i64,
    command: AliasCommand,
) -> Result<EntryAliases, PersistenceError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current = revision(&tx)?;
    if current != expected {
        return Err(PersistenceError::StaleRevision { expected, current });
    }
    aliases(&tx, entry)?;
    match command {
        AliasCommand::Add { text } => {
            let text = crate::domain::require_definition_name(&text).map_err(invalid)?;
            let normalized = normalize_alias(&text);
            if tx
                .prepare("SELECT 1 FROM entry_alias WHERE entry_id=?1 AND normalized_alias=?2")?
                .exists(params![entry.to_string(), normalized])?
            {
                return Err(invalid("This Entry already has that alias"));
            }
            tx.execute(
                "INSERT INTO entry_alias VALUES(?1,?2,?3,?4)",
                params![
                    uuid::Uuid::now_v7().to_string(),
                    entry.to_string(),
                    text,
                    normalized
                ],
            )?;
        }
        AliasCommand::Delete { alias_id } => {
            if tx.execute(
                "DELETE FROM entry_alias WHERE id=?1 AND entry_id=?2",
                params![alias_id, entry.to_string()],
            )? != 1
            {
                return Err(invalid("Alias not found on this Entry"));
            }
        }
    }
    tx.execute("UPDATE project_meta SET last_committed_revision=last_committed_revision+1,updated_at=?1 WHERE id=1", [chrono::Utc::now().to_rfc3339()])?;
    let result = aliases(&tx, entry)?;
    tx.commit()?;
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Document {
    group: String,
    hit: SearchHit,
    identity: Option<String>,
    aliases: Vec<String>,
    text: String,
}
impl Document {
    fn terms(&self) -> String {
        tokens(&format!(
            "{} {} {}",
            self.identity.as_deref().unwrap_or(""),
            self.aliases.join(" "),
            self.text
        ))
        .join(" ")
    }
}
fn doc(
    group: &str,
    key: String,
    title: String,
    context: String,
    state: String,
    target: SearchTarget,
    text: String,
) -> Document {
    Document {
        group: group.into(),
        hit: SearchHit {
            key,
            title,
            context,
            workspace_state: state,
            reason: String::new(),
            excerpt: String::new(),
            target,
        },
        identity: None,
        aliases: vec![],
        text,
    }
}

/// Read only the authoritative tables. Hidden Fields still contain authored data.
fn sources(conn: &Connection) -> Result<Vec<Document>, PersistenceError> {
    let mut result = vec![];
    let mut entry_context = HashMap::new();
    let mut alias_map: HashMap<String, Vec<String>> = HashMap::new();
    for row in conn
        .prepare("SELECT entry_id,alias_text FROM entry_alias ORDER BY normalized_alias,id")?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
    {
        let (id, alias) = row?;
        alias_map.entry(id).or_default().push(alias);
    }
    for row in conn.prepare("SELECT e.id,COALESCE(e.authored_name,'[Unnamed Entry]'),c.name,COALESCE(t.name,''),i.workspace_state FROM entry e JOIN category c ON c.id=e.category_id LEFT JOIN type_def t ON t.id=e.type_id JOIN record_identity i ON i.record_id=e.id")?.query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?)))? {
        let (id,name,category,kind,state)=row?;
        let context = if kind.is_empty() {category} else {format!("{category} · {kind}")};
        entry_context.insert(id.clone(),(name.clone(),context.clone(),state.clone()));
        let mut d=doc("entries",format!("entry:{id}"),name.clone(),context,state,SearchTarget::Entry{entry_id:id.clone()},name.clone());
        d.identity=Some(id.clone());
        d.aliases=alias_map.remove(&id).unwrap_or_default();
        result.push(d);
    }
    let mut chapter_context = HashMap::new();
    for row in conn.prepare("SELECT s.id,s.title,i.workspace_state FROM story_unit s JOIN record_identity i ON i.record_id=s.id")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))? {
        let (id,title,state)=row?;
        let title=if title.trim().is_empty(){"[Untitled Chapter]".into()}else{title};
        chapter_context.insert(id.clone(),(title.clone(),state.clone()));
        let mut d=doc("chapters",format!("chapter:{id}"),title.clone(),"Chapter".into(),state,SearchTarget::Chapter{chapter_id:id.clone(),area:"manuscript".into()},title);
        d.identity=Some(id);
        result.push(d);
    }
    for row in conn.prepare("SELECT v.id,v.entry_id,f.name,COALESCE(v.text_value,CAST(v.number_value AS TEXT),CASE v.bool_value WHEN 1 THEN 'Yes true' WHEN 0 THEN 'No false' END,(SELECT group_concat(label,', ') FROM (SELECT o.label FROM field_choice_value x JOIN choice_option o ON o.id=x.option_id WHERE x.value_id=v.id ORDER BY o.label,o.id)),''),COALESCE(f.unit,'') FROM field_value v JOIN field_definition f ON f.id=v.field_id")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?)))? {
        let (id,entry,field,value,unit)=row?;
        let (title,context,state)=entry_context.get(&entry).ok_or_else(||invalid("Field owner is missing"))?;
        result.push(doc("structured",format!("field:{id}"),title.clone(),format!("{context} · Field: {field}"),state.clone(),SearchTarget::Entry{entry_id:entry},format!("{field}: {value} {unit}").trim().into()));
    }
    // Canonical participants supply names; never index a second semantic projection.
    for row in conn.prepare("SELECT r.id,d.name,d.forward_label,d.inverse_label,r.note,i.workspace_state,r.semantic_state,COALESCE(a.authored_name,json_extract(p.unresolved_snapshot,'$.label'),'[Unnamed Entry]'),COALESCE(b.authored_name,json_extract(q.unresolved_snapshot,'$.label'),'[Unnamed Entry]') FROM relationship_instance r JOIN relationship_definition d ON d.id=r.definition_id JOIN record_identity i ON i.record_id=r.id JOIN relationship_participant p ON p.instance_id=r.id AND p.slot='source' JOIN relationship_participant q ON q.instance_id=r.id AND q.slot='target' LEFT JOIN entry a ON a.id=p.record_id LEFT JOIN entry b ON b.id=q.record_id")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,String>(7)?,r.get::<_,String>(8)?)))? {
        let (id,name,forward,inverse,note,state,semantic,source,target)=row?;
        result.push(doc("structured",format!("relationship:{id}"),format!("{source} · {forward} · {target}"),format!("Relationship: {name} · {}",if semantic=="ended" {"Past"} else {"Current"}),state,SearchTarget::Relationship{relationship_id:id},format!("{name} {source} {forward} {target} {inverse}\n{note}")));
    }
    for row in conn.prepare("SELECT l.id,l.story_unit_id,COALESCE(e.authored_name,json_extract(l.unresolved_snapshot,'$.label'),'[Unnamed Entry]'),COALESCE((SELECT group_concat(name,', ') FROM (SELECT r.name FROM story_role r JOIN story_link_role lr ON lr.role_id=r.id WHERE lr.link_id=l.id ORDER BY r.name,r.id)),'') FROM story_link l LEFT JOIN entry e ON e.id=l.entry_id")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?)))? {
        let (id,chapter,entry,roles)=row?;
        let (title,state)=chapter_context.get(&chapter).ok_or_else(||invalid("Story link owner is missing"))?;
        result.push(doc("structured",format!("story-link:{id}"),title.clone(),"Linked Entry and Roles".into(),state.clone(),SearchTarget::Chapter{chapter_id:chapter,area:"manuscript".into()},format!("{entry} {roles}").trim().into()));
    }
    for row in conn.prepare("SELECT id,owner_id,area,document_schema_version,canonical_json,plain_text FROM rich_document")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?)))? {
        let (id,chapter,area,version,json,preserved)=row?;
        let (title,state)=chapter_context.get(&chapter).ok_or_else(||invalid("Document owner is missing"))?;
        let text=serde_json::from_str(&json).map_err(|e|e.to_string()).and_then(|v|crate::domain::story::document_text(version,&v)).map(|(text,_)|text).unwrap_or(preserved);
        result.push(doc("text",format!("document:{id}"),title.clone(),format!("{area} · Text match"),state.clone(),SearchTarget::Chapter{chapter_id:chapter,area},text));
    }
    Ok(result)
}

fn cached(conn: &Connection, query: &str) -> Result<Vec<Document>, PersistenceError> {
    let ready: bool=conn.query_row("SELECT dirty=0 AND schema_version=?1 AND source_revision=?2 AND indexed_revision=source_revision AND document_count=(SELECT count(*) FROM search_index) FROM derived_index_state WHERE id=1",params![INDEX_VERSION,revision(conn)?],|r|r.get(0))?;
    if !ready {
        return Err(invalid("Search cache needs refresh"));
    }
    conn.execute(
        "INSERT INTO search_index(search_index) VALUES('integrity-check')",
        [],
    )?;
    let words = tokens(query);
    let expression = words
        .iter()
        .map(|w| format!("\"{w}\"*"))
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = if words.is_empty() {
        "SELECT payload FROM search_index"
    } else {
        "SELECT payload FROM search_index WHERE terms MATCH ?1"
    };
    let parameters = if words.is_empty() {
        vec![]
    } else {
        vec![expression]
    };
    conn.prepare(sql)?
        .query_map(rusqlite::params_from_iter(parameters), |r| {
            r.get::<_, String>(0)
        })?
        .map(|row| serde_json::from_str(&row?).map_err(invalid))
        .collect()
}

/// A single transaction replaces only derived data. A crash leaves the old cache
/// dirty, and queries continue against source SQL. It never acknowledges a save.
pub(super) fn rebuild(conn: &Connection) -> Result<(), PersistenceError> {
    let tx = conn.unchecked_transaction()?;
    let documents = sources(&tx)?;
    tx.execute_batch("DROP TABLE IF EXISTS search_index")?;
    tx.execute_batch(CREATE_INDEX)?;
    {
        let mut insert = tx.prepare("INSERT INTO search_index(payload,terms) VALUES(?1,?2)")?;
        for d in &documents {
            insert.execute(params![
                serde_json::to_string(d).map_err(invalid)?,
                d.terms()
            ])?;
        }
    }
    tx.execute("UPDATE derived_index_state SET source_revision=?1,indexed_revision=?1,schema_version=?2,dirty=0,document_count=?3 WHERE id=1",params![revision(&tx)?,INDEX_VERSION,documents.len()])?;
    tx.commit()?;
    Ok(())
}

fn excerpt(text: &str, query: &str) -> String {
    let words = text.split_whitespace().collect::<Vec<_>>();
    let first = tokens(query).into_iter().next().unwrap_or_default();
    let at = words
        .iter()
        .position(|w| normalize_search(w).contains(&first))
        .unwrap_or(0);
    let start = at.saturating_sub(6);
    let joined = words[start..].join(" ");
    let preview = joined.chars().take(220).collect::<String>();
    format!(
        "{}{}{}",
        if start > 0 { "… " } else { "" },
        preview,
        if joined.chars().count() > 220 {
            "…"
        } else {
            ""
        }
    )
}
fn rank(d: &Document, query: &str) -> (u8, String) {
    let name = normalize_search(&d.hit.title);
    if d.identity.as_deref().is_some_and(|id| id == query) {
        return (0, "Exact ID".into());
    }
    if d.identity.is_some() && name == query {
        return (0, "Exact name".into());
    }
    if let Some(alias) = d.aliases.iter().find(|a| normalize_search(a) == query) {
        return (1, format!("Alias: {alias}"));
    }
    if d.identity.is_some() && name.starts_with(query) {
        return (2, "Name".into());
    }
    if let Some(alias) = d
        .aliases
        .iter()
        .find(|a| normalize_search(a).starts_with(query))
    {
        return (3, format!("Alias: {alias}"));
    }
    if d.identity.is_some() {
        let query_words = tokens(query);
        if let Some(alias) = d.aliases.iter().find(|a| {
            if query_words.is_empty() {
                normalize_search(a).contains(query)
            } else {
                let alias_words = tokens(a);
                query_words
                    .iter()
                    .all(|q| alias_words.iter().any(|w| w.starts_with(q)))
            }
        }) {
            return (4, format!("Alias: {alias}"));
        }
        return (4, "Name".into());
    }
    (
        5,
        if d.group == "text" {
            "Plain text · not a structural link"
        } else {
            "Structured context"
        }
        .into(),
    )
}
pub(super) fn query(
    conn: &Connection,
    request: SearchRequest,
) -> Result<SearchResults, PersistenceError> {
    if request.query.chars().count() > 256
        || tokens(&request.query).len() > 32
        || !(1..=100).contains(&request.limit_per_group)
    {
        return Err(invalid(
            "Use up to 256 characters and 32 search words; request 1–100 results per group",
        ));
    }
    let query = normalize_search(request.query.trim());
    let words = tokens(&query);
    let (documents, refresh) = if query.is_empty() {
        (vec![], false)
    } else {
        match cached(conn, &query) {
            Ok(d) => (d, false),
            Err(_) => (sources(conn)?, true),
        }
    };
    let mut matches = documents
        .into_iter()
        .filter(|d| {
            if !request.include_inactive && d.hit.workspace_state != "active" {
                return false;
            }
            if words.is_empty() {
                return normalize_search(&format!("{} {}", d.text, d.aliases.join(" ")))
                    .contains(&query);
            }
            let terms = d.terms();
            words
                .iter()
                .all(|q| terms.split_whitespace().any(|w| w.starts_with(q)))
        })
        .map(|mut d| {
            let (rank, reason) = rank(&d, &query);
            d.hit.reason = reason;
            if d.identity.is_none() {
                d.hit.excerpt = excerpt(&d.text, &query);
            }
            (rank, d)
        })
        .collect::<Vec<_>>();
    matches.sort_by(|(a, x), (b, y)| {
        a.cmp(b)
            .then_with(|| normalize_search(&x.hit.title).cmp(&normalize_search(&y.hit.title)))
            .then_with(|| x.hit.key.cmp(&y.hit.key))
    });
    let groups = ["entries", "chapters", "structured", "text"]
        .into_iter()
        .map(|kind| {
            let matching = matches
                .iter()
                .filter(|(_, d)| d.group == kind)
                .map(|(_, d)| d.hit.clone())
                .collect::<Vec<_>>();
            SearchGroup {
                kind: kind.into(),
                total: matching.len(),
                hits: matching.into_iter().take(request.limit_per_group).collect(),
            }
        })
        .collect();
    let result = SearchResults {
        global_revision: revision(conn)?,
        groups,
    };
    // A cache failure must never turn a successful source query into an error.
    // Lazy rebuilding avoids rewriting all prose on each autosave keystroke.
    if refresh {
        let _ = rebuild(conn);
    }
    Ok(result)
}
