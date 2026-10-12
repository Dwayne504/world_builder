use rusqlite::Connection;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use tempfile::{tempdir, TempDir};
use worldcrafter_lib::{
    application::{manuscript_export::ExportStore, AppState, ProjectService, ProjectSummary},
    domain::{story::*, structure::ChapterId},
};

struct Fixture {
    dir: TempDir,
    state: AppState,
    project: ProjectSummary,
    exports: ExportStore,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let state = AppState::default();
        let project = ProjectService::create_project(&state, dir.path(), "Export fixture").unwrap();
        Self {
            dir,
            state,
            project,
            exports: ExportStore::default(),
        }
    }
    fn apply(&self, command: StoryCommand) -> ChapterSnapshot {
        let revision = ProjectService::read_story(&self.state, self.project.project_id)
            .unwrap()
            .global_revision;
        ProjectService::apply_story(&self.state, self.project.project_id, revision, command)
            .unwrap()
    }
    fn chapter(&self, title: &str) -> ChapterId {
        self.apply(StoryCommand::Create {
            title: title.into(),
        })
        .chapter
        .id
    }
    fn save(&self, id: ChapterId, content: Value) {
        self.apply(StoryCommand::Save {
            chapter_id: id,
            title: None,
            documents: vec![
                DocumentEdit {
                    area: DocumentArea::Manuscript,
                    schema_version: 1,
                    content,
                },
                DocumentEdit {
                    area: DocumentArea::Plan,
                    schema_version: 1,
                    content: paragraph("PLAN_EXCLUDED_SENTINEL"),
                },
                DocumentEdit {
                    area: DocumentArea::Notes,
                    schema_version: 1,
                    content: paragraph("NOTES_EXCLUDED_SENTINEL"),
                },
            ],
        });
    }
    fn path(&self) -> PathBuf {
        self.dir.path().join("manuscript.md")
    }
    fn db(&self) -> Connection {
        Connection::open(PathBuf::from(&self.project.package_path).join("data/project.sqlite"))
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = ProjectService::close_project(&self.state, self.project.project_id);
    }
}
fn paragraph(text: &str) -> Value {
    json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":text}]}]})
}
fn formats() -> Value {
    json!({"type":"doc","content":[
        {"type":"heading","attrs":{"level":1},"content":[{"type":"text","text":"One"}]},
        {"type":"heading","attrs":{"level":2},"content":[{"type":"text","text":"Two"}]},
        {"type":"heading","attrs":{"level":3},"content":[{"type":"text","text":"Three"}]},
        {"type":"paragraph","content":[
            {"type":"text","text":"Bold Ω","marks":[{"type":"bold"}]},
            {"type":"text","text":"italic","marks":[{"type":"italic"}]},
            {"type":"text","text":"both","marks":[{"type":"italic"},{"type":"bold"}]},
            {"type":"text","text":" deleted ","marks":[{"type":"strike"}]},
            {"type":"text","text":"underlined","marks":[{"type":"underline"}]},
            {"type":"hardBreak"},
            {"type":"text","text":"`x` & <tag>\nnext","marks":[{"type":"code"}]},
            {"type":"text","text":" café 日本語 🪐 <script>alert('x')</script> & *not italic* [not a link](https://bad.invalid)"}
        ]},
        {"type":"paragraph","content":[{"type":"text","text":"    four spaces stay prose"}]},
        {"type":"paragraph","content":[{"type":"text","text":"\ttab starts prose"}]},
        {"type":"paragraph","content":[{"type":"text","text":"1. literal list\n# literal heading"}]},
        {"type":"blockquote","content":[{"type":"paragraph","content":[{"type":"text","text":"Quoted thought"}]}]},
        {"type":"bulletList","content":[{"type":"listItem","content":[
            {"type":"paragraph","content":[{"type":"text","text":"Outer bullet"}]},
            {"type":"orderedList","attrs":{"start":3,"type":null},"content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Nested numbered"}]}]}]}
        ]}]},
        {"type":"orderedList","attrs":{"start":4,"type":"I"},"content":[{"type":"listItem","content":[
            {"type":"paragraph","content":[{"type":"text","text":"Roman fourth"}]},
            {"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Nested in Roman"}]}]}]}
        ]}]},
        {"type":"horizontalRule"},
        {"type":"paragraph"}
    ]})
}
#[test]
fn export_reader_fixture_preserves_formatting_order_unicode_and_excludes_other_areas() {
    let f = Fixture::new();
    let second = f.chapter("Second — 終");
    let first = f.chapter("First Ω");
    f.save(first, formats());
    f.save(second, paragraph("SECOND_CHAPTER_SENTINEL"));
    f.apply(StoryCommand::Move {
        chapter_id: first,
        before_id: Some(second),
    });
    let source_before =
        ProjectService::read_chapter(&f.state, f.project.project_id, first).unwrap();
    let preview = f
        .exports
        .preview(&f.state, f.project.project_id, vec![second, first])
        .unwrap();
    assert_eq!(
        preview.chapters.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![first, second]
    );
    assert_eq!(
        preview.word_count,
        preview.chapters.iter().map(|c| c.word_count).sum::<usize>()
    );
    assert!(preview.markdown.starts_with("# First Ω\n"));
    for expected in [
        "<strong>Bold Ω</strong><em>italic</em>",
        "<u>underlined</u>",
        "<ol start=\"4\" type=\"I\">",
        "café 日本語 🪐",
        "&#32;&#32;&#32;&#32;four spaces",
        "&#9;tab starts prose",
        "&lt;script&gt;",
    ] {
        assert!(
            preview.markdown.contains(expected),
            "{expected}: {}",
            preview.markdown
        );
    }
    assert!(!preview.markdown.contains("EXCLUDED_SENTINEL"));
    assert!(!preview.markdown.contains("<script>"));
    assert!(!preview.format_notes.is_empty());
    let dest = f
        .exports
        .destination(
            &f.state,
            f.project.project_id,
            &preview.preview_id,
            Some(&f.path()),
        )
        .unwrap()
        .unwrap();
    let receipt = f
        .exports
        .publish(
            &f.state,
            f.project.project_id,
            &preview.preview_id,
            &dest.destination_id,
            false,
        )
        .unwrap();
    assert_eq!(receipt.chapter_count, 2);
    assert_eq!(fs::read_to_string(&receipt.path).unwrap(), preview.markdown);
    assert_eq!(
        serde_json::to_value(
            ProjectService::read_chapter(&f.state, f.project.project_id, first).unwrap()
        )
        .unwrap(),
        serde_json::to_value(source_before).unwrap()
    );
    // Only an explicitly supplied disposable reader-fixture path may receive a
    // test artifact. create_new prevents overwriting any existing supplied file.
    if let Some(path) = std::env::var_os("WORLDCRAFTER_EXPORT_READER_FIXTURE") {
        use std::io::Write;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        output.write_all(&fs::read(receipt.path).unwrap()).unwrap();
    }
}
#[test]
fn selection_is_explicit_unnamed_and_archived_are_reviewable_foreign_ids_fail() {
    let f = Fixture::new();
    let a = f.chapter("");
    let b = f.chapter("Not selected");
    f.save(b, paragraph("UNSELECTED_SENTINEL"));
    f.apply(StoryCommand::SetState {
        chapter_id: a,
        state: WorkspaceState::Archived,
    });
    let p = f
        .exports
        .preview(&f.state, f.project.project_id, vec![a])
        .unwrap();
    assert_eq!(p.chapters[0].title, "[Unnamed Chapter]");
    assert_eq!(p.chapters[0].workspace_state, "archived");
    assert!(!p.markdown.contains("UNSELECTED_SENTINEL"));
    for ids in [vec![], vec![a, a], vec![ChapterId::new()]] {
        assert!(f
            .exports
            .preview(&f.state, f.project.project_id, ids)
            .is_err());
    }
    let other = Fixture::new();
    let other_chapter = other.chapter("Foreign");
    assert!(f
        .exports
        .preview(&f.state, f.project.project_id, vec![other_chapter])
        .is_err());
}
#[test]
fn malformed_or_newer_manuscripts_fail_without_cached_text_fallback() {
    for (version, json, state) in [
        (
            2,
            r#"{"type":"doc","content":[{"type":"paragraph"}]}"#,
            "current",
        ),
        (1, "broken json", "current"),
        (
            1,
            r#"{"type":"doc","content":[{"type":"unknown"}]}"#,
            "current",
        ),
        (
            1,
            r#"{"type":"doc","content":[{"type":"paragraph"}]}"#,
            "preserved",
        ),
    ] {
        let f = Fixture::new();
        let chapter = f.chapter("Recoverable");
        f.db().execute("UPDATE rich_document SET document_schema_version=?1,canonical_json=?2,migration_state=?3,plain_text='CACHED_NOT_AUTHORED' WHERE owner_id=?4 AND area='manuscript'",rusqlite::params![version,json,state,chapter.to_string()]).unwrap();
        assert!(f
            .exports
            .preview(&f.state, f.project.project_id, vec![chapter])
            .is_err());
        let actual: String = f
            .db()
            .query_row(
                "SELECT canonical_json FROM rich_document WHERE owner_id=?1 AND area='manuscript'",
                [chapter.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(actual, json);
    }
}
#[test]
fn cancel_discard_close_and_source_changes_never_publish() {
    let f = Fixture::new();
    let chapter = f.chapter("A");
    f.save(chapter, paragraph("before"));
    let p = f
        .exports
        .preview(&f.state, f.project.project_id, vec![chapter])
        .unwrap();
    assert!(f
        .exports
        .destination(&f.state, f.project.project_id, &p.preview_id, None)
        .unwrap()
        .is_none());
    assert!(!f.path().exists());
    let d = f
        .exports
        .destination(
            &f.state,
            f.project.project_id,
            &p.preview_id,
            Some(&f.path()),
        )
        .unwrap()
        .unwrap();
    f.save(chapter, paragraph("after"));
    assert_eq!(
        f.exports
            .publish(
                &f.state,
                f.project.project_id,
                &p.preview_id,
                &d.destination_id,
                false
            )
            .unwrap_err()
            .kind(),
        "revision_conflict"
    );
    assert!(!f.path().exists());
    let p = f
        .exports
        .preview(&f.state, f.project.project_id, vec![chapter])
        .unwrap();
    f.exports.discard(f.project.project_id, &p.preview_id);
    assert!(f
        .exports
        .destination(
            &f.state,
            f.project.project_id,
            &p.preview_id,
            Some(&f.path())
        )
        .is_err());
    let p = f
        .exports
        .preview(&f.state, f.project.project_id, vec![chapter])
        .unwrap();
    ProjectService::close_project(&f.state, f.project.project_id).unwrap();
    ProjectService::open_project(
        &f.state,
        std::path::Path::new(&f.project.package_path),
        false,
    )
    .unwrap();
    assert!(f
        .exports
        .destination(
            &f.state,
            f.project.project_id,
            &p.preview_id,
            Some(&f.path())
        )
        .is_err());
    f.exports.close_project(f.project.project_id);
}
#[test]
fn replacement_requires_review_and_rejects_changed_destination_and_package_paths() {
    let f = Fixture::new();
    let chapter = f.chapter("A");
    f.save(chapter, paragraph("new manuscript"));
    fs::write(f.path(), "original").unwrap();
    let p = f
        .exports
        .preview(&f.state, f.project.project_id, vec![chapter])
        .unwrap();
    let d = f
        .exports
        .destination(
            &f.state,
            f.project.project_id,
            &p.preview_id,
            Some(&f.path()),
        )
        .unwrap()
        .unwrap();
    assert!(d.replaces_existing);
    assert!(f
        .exports
        .publish(
            &f.state,
            f.project.project_id,
            &p.preview_id,
            &d.destination_id,
            false
        )
        .is_err());
    assert_eq!(fs::read_to_string(f.path()).unwrap(), "original");
    let d = f
        .exports
        .destination(
            &f.state,
            f.project.project_id,
            &p.preview_id,
            Some(&f.path()),
        )
        .unwrap()
        .unwrap();
    fs::write(f.path(), "concurrent edit").unwrap();
    assert!(f
        .exports
        .publish(
            &f.state,
            f.project.project_id,
            &p.preview_id,
            &d.destination_id,
            true
        )
        .is_err());
    assert_eq!(fs::read_to_string(f.path()).unwrap(), "concurrent edit");
    let d = f
        .exports
        .destination(
            &f.state,
            f.project.project_id,
            &p.preview_id,
            Some(&f.path()),
        )
        .unwrap()
        .unwrap();
    f.exports
        .publish(
            &f.state,
            f.project.project_id,
            &p.preview_id,
            &d.destination_id,
            true,
        )
        .unwrap();
    assert_eq!(fs::read_to_string(f.path()).unwrap(), p.markdown);
    let p = f
        .exports
        .preview(&f.state, f.project.project_id, vec![chapter])
        .unwrap();
    for path in [
        PathBuf::from(&f.project.package_path).join("export.md"),
        f.dir.path().join("missing/export.md"),
    ] {
        assert!(f
            .exports
            .destination(&f.state, f.project.project_id, &p.preview_id, Some(&path))
            .is_err());
        assert!(!path.exists());
    }
}

#[test]
fn source_payload_and_project_session_are_checked_even_if_revision_is_reused() {
    let f = Fixture::new();
    let chapter = f.chapter("A");
    f.save(chapter, paragraph("reviewed"));
    let p = f
        .exports
        .preview(&f.state, f.project.project_id, vec![chapter])
        .unwrap();
    let d = f
        .exports
        .destination(
            &f.state,
            f.project.project_id,
            &p.preview_id,
            Some(&f.path()),
        )
        .unwrap()
        .unwrap();
    f.db()
        .execute(
            "UPDATE rich_document SET canonical_json=?1 WHERE owner_id=?2 AND area='manuscript'",
            rusqlite::params![
                paragraph("external changed content").to_string(),
                chapter.to_string()
            ],
        )
        .unwrap();
    assert!(f
        .exports
        .publish(
            &f.state,
            f.project.project_id,
            &p.preview_id,
            &d.destination_id,
            false
        )
        .is_err());
    assert!(!f.path().exists());
    let other = Fixture::new();
    assert!(f
        .exports
        .destination(
            &other.state,
            other.project.project_id,
            &p.preview_id,
            Some(&other.path())
        )
        .is_err());
    assert!(!other.path().exists());
}
