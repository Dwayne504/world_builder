use super::{export_publication as publication, state::OpenProject, AppError, AppState};
use crate::domain::{manuscript_export::ExportChapter, structure::ChapterId, ProjectId};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant},
};

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("{0}")]
    Invalid(String),
    #[error("Export could not finish: {0}. Existing files were preserved; choose and review the destination again.")]
    Io(#[from] std::io::Error),
    #[error("{message} Recovery file: {path}")]
    Recovery { message: String, path: String },
}
fn invalid(message: &str) -> AppError {
    ExportError::Invalid(message.into()).into()
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreview {
    pub preview_id: String,
    pub project_id: ProjectId,
    pub global_revision: i64,
    pub chapters: Vec<ExportChapter>,
    pub word_count: usize,
    pub markdown: String,
    pub suggested_file_name: String,
    pub format_notes: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDestination {
    pub destination_id: String,
    pub path: String,
    pub replaces_existing: bool,
    pub existing_bytes: Option<u64>,
    pub existing_modified_at: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReceipt {
    pub path: String,
    pub chapter_count: usize,
    pub word_count: usize,
    pub bytes_written: usize,
}
#[derive(Clone)]
struct Preview {
    dto: ExportPreview,
    session: Weak<OpenProject>,
    created: Instant,
}
struct Destination {
    preview_id: String,
    path: PathBuf,
    fingerprint: Option<publication::Fingerprint>,
}
#[derive(Default)]
struct Pending {
    previews: HashMap<String, Preview>,
    destinations: HashMap<String, Destination>,
}
#[derive(Default)]
pub struct ExportStore {
    pending: Mutex<Pending>,
}
impl ExportStore {
    pub fn preview(
        &self,
        state: &AppState,
        project: ProjectId,
        chapter_ids: Vec<ChapterId>,
    ) -> Result<ExportPreview, AppError> {
        let open = open_project(state, project)?;
        let guard = open.worker.lock().expect("worker mutex poisoned");
        let worker = guard.as_ref().ok_or(AppError::ProjectNotOpen(project))?;
        let selected = worker.read_manuscripts(chapter_ids)?;
        let dto = ExportPreview {
            preview_id: uuid::Uuid::new_v4().to_string(),
            project_id: project,
            global_revision: selected.global_revision,
            chapters: selected.chapters,
            word_count: selected.word_count,
            markdown: selected.markdown,
            suggested_file_name: format!(
                "{}.md",
                crate::package::layout::sanitize_directory_stem(&selected.working_name)
            ),
            format_notes: selected.format_notes,
        };
        let mut pending = self.pending.lock().expect("export mutex poisoned");
        pending.previews.retain(|_, p| {
            p.created.elapsed() < Duration::from_secs(30 * 60) && p.session.strong_count() > 0
        });
        if pending.previews.len() >= 4 {
            if let Some(oldest) = pending
                .previews
                .iter()
                .min_by_key(|(_, p)| p.created)
                .map(|(id, _)| id.clone())
            {
                pending.previews.remove(&oldest);
            }
        }
        let retained: Vec<_> = pending.previews.keys().cloned().collect();
        pending
            .destinations
            .retain(|_, d| retained.contains(&d.preview_id));
        pending.previews.insert(
            dto.preview_id.clone(),
            Preview {
                dto: dto.clone(),
                session: Arc::downgrade(&open),
                created: Instant::now(),
            },
        );
        Ok(dto)
    }
    fn reviewed(
        &self,
        state: &AppState,
        project: ProjectId,
        preview_id: &str,
    ) -> Result<(Preview, Arc<OpenProject>), AppError> {
        let preview = self
            .pending
            .lock()
            .expect("export mutex poisoned")
            .previews
            .get(preview_id)
            .cloned()
            .ok_or_else(|| invalid("This export preview expired. Review the Chapters again."))?;
        let open = open_project(state, project)?;
        if preview.dto.project_id != project
            || preview.created.elapsed() >= Duration::from_secs(30 * 60)
            || !preview
                .session
                .upgrade()
                .is_some_and(|p| Arc::ptr_eq(&p, &open))
        {
            return Err(invalid("This export preview belongs to a different or closed Project session. Review the Chapters again."));
        }
        Ok((preview, open))
    }
    pub fn suggested_name(
        &self,
        state: &AppState,
        project: ProjectId,
        preview_id: &str,
    ) -> Result<String, AppError> {
        Ok(self
            .reviewed(state, project, preview_id)?
            .0
            .dto
            .suggested_file_name)
    }
    pub fn destination(
        &self,
        state: &AppState,
        project: ProjectId,
        preview_id: &str,
        path: Option<&Path>,
    ) -> Result<Option<ExportDestination>, AppError> {
        self.reviewed(state, project, preview_id)?;
        let Some(path) = path else {
            return Ok(None);
        };
        let path = publication::safe_target(path)?;
        publication::check_recovery(&path)?;
        let fingerprint = publication::fingerprint(&path)?;
        let dto = ExportDestination {
            destination_id: uuid::Uuid::new_v4().to_string(),
            path: path.display().to_string(),
            replaces_existing: fingerprint.is_some(),
            existing_bytes: fingerprint.as_ref().map(|f| f.bytes),
            existing_modified_at: fingerprint
                .as_ref()
                .and_then(|f| f.modified_nanos)
                .and_then(|n| {
                    i64::try_from(n / 1_000_000_000).ok().and_then(|s| {
                        chrono::DateTime::from_timestamp(s, (n % 1_000_000_000) as u32)
                    })
                })
                .map(|t| t.to_rfc3339()),
        };
        let mut pending = self.pending.lock().expect("export mutex poisoned");
        if !pending.previews.contains_key(preview_id) {
            return Err(invalid(
                "This export was cancelled. Review the Chapters again.",
            ));
        }
        pending
            .destinations
            .retain(|_, d| d.preview_id != preview_id);
        pending.destinations.insert(
            dto.destination_id.clone(),
            Destination {
                preview_id: preview_id.into(),
                path,
                fingerprint,
            },
        );
        Ok(Some(dto))
    }
    pub fn publish(
        &self,
        state: &AppState,
        project: ProjectId,
        preview_id: &str,
        destination_id: &str,
        replace_existing: bool,
    ) -> Result<ExportReceipt, AppError> {
        let (preview, open) = self.reviewed(state, project, preview_id)?;
        let destination = self
            .pending
            .lock()
            .expect("export mutex poisoned")
            .destinations
            .remove(destination_id)
            .ok_or_else(|| invalid("Choose and review an export destination first."))?;
        if destination.preview_id != preview_id {
            return Err(invalid(
                "This destination belongs to a different export preview.",
            ));
        }
        if destination.fingerprint.is_some() && !replace_existing {
            return Err(invalid(
                "Confirm Replace to overwrite the reviewed file, or choose another filename.",
            ));
        }
        let guard = open.worker.lock().expect("worker mutex poisoned");
        let worker = guard.as_ref().ok_or(AppError::ProjectNotOpen(project))?;
        if !Arc::ptr_eq(&open_project(state, project)?, &open) {
            return Err(invalid(
                "The Project session changed. Review the export again.",
            ));
        }
        let current =
            worker.read_manuscripts(preview.dto.chapters.iter().map(|c| c.id).collect())?;
        if current.global_revision != preview.dto.global_revision {
            return Err(crate::persistence::PersistenceError::StaleRevision {
                expected: preview.dto.global_revision,
                current: current.global_revision,
            }
            .into());
        }
        if current.markdown != preview.dto.markdown {
            return Err(invalid(
                "The manuscript changed after preview. Review the export again.",
            ));
        }
        publication::publish(
            &destination.path,
            destination.fingerprint.as_ref(),
            preview.dto.markdown.as_bytes(),
        )?;
        self.discard(project, preview_id);
        Ok(ExportReceipt {
            path: destination.path.display().to_string(),
            chapter_count: preview.dto.chapters.len(),
            word_count: preview.dto.word_count,
            bytes_written: preview.dto.markdown.len(),
        })
    }
    pub fn discard(&self, project: ProjectId, preview_id: &str) {
        let mut pending = self.pending.lock().expect("export mutex poisoned");
        if pending
            .previews
            .get(preview_id)
            .is_some_and(|p| p.dto.project_id == project)
        {
            pending.previews.remove(preview_id);
            pending
                .destinations
                .retain(|_, d| d.preview_id != preview_id);
        }
    }
    pub fn close_project(&self, project: ProjectId) {
        let mut pending = self.pending.lock().expect("export mutex poisoned");
        pending.previews.retain(|_, p| p.dto.project_id != project);
        let retained: Vec<_> = pending.previews.keys().cloned().collect();
        pending
            .destinations
            .retain(|_, d| retained.contains(&d.preview_id));
    }
}
fn open_project(state: &AppState, project: ProjectId) -> Result<Arc<OpenProject>, AppError> {
    state
        .open_projects
        .lock()
        .expect("registry mutex poisoned")
        .get(&project)
        .cloned()
        .ok_or(AppError::ProjectNotOpen(project))
}
