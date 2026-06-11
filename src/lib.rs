pub mod fs;
pub mod model;
pub mod error;
pub mod cli;
pub mod commands;
pub mod output;
pub mod mcp;
pub mod watch;

pub use error::AdocsError;
pub use model::config::{resolve_roots, AdocsConfig, ResolvedRoots, VerificationPolicy};
pub use model::ledger::{
    DocEvidence, FileId, FileRecord, FilesLedger, FolderRecord, FoldersLedger,
    SealEvidence,
};
pub use model::state::{file_state, folder_purpose_state, TrustState};
pub use model::change::{FileChange, FolderChange};
pub use model::paths::{file_description_path, folder_purpose_path};

use camino::Utf8PathBuf;

pub fn status(request: &StatusRequest) -> Result<StatusReport, AdocsError> {
    commands::status::run_status(request)
}

pub fn changed(request: &ChangedRequest) -> Result<ChangedReport, AdocsError> {
    commands::status::run_changed(request)
}

pub fn sync(request: &SyncRequest) -> Result<SyncReport, AdocsError> {
    commands::init::run_sync(request)
}

pub fn list_state(request: &ListStateRequest) -> Result<ListStateReport, AdocsError> {
    commands::list::run_list(request)
}

pub fn update_doc(request: &UpdateDocRequest) -> Result<UpdateDocReport, AdocsError> {
    commands::update::run_update(request)
}

pub fn docs_under(request: &DocsUnderRequest) -> Result<DocsUnderReport, AdocsError> {
    commands::docsunder::run_docs_under(request)
}

pub fn seal(request: &SealRequest) -> Result<SealReport, AdocsError> {
    commands::seal::run_seal(request)
}

pub fn rebind(file_id: &FileId, new_path: &Utf8PathBuf, roots: &ResolvedRoots) -> Result<(), AdocsError> {
    commands::rebind::run_rebind(file_id, new_path, roots)
}

#[derive(Debug, Clone)]
pub struct StatusRequest {
    pub json: bool,
    pub roots: ResolvedRoots,
    pub fail_on_stale: bool,
    pub fail_on_missing_docs: bool,
    pub fail_on_ambiguous: bool,
}

#[derive(Debug, Clone)]
pub struct ChangedRequest {
    pub json: bool,
    pub roots: ResolvedRoots,
}

#[derive(Debug, Clone)]
pub struct SyncRequest {
    pub roots: ResolvedRoots,
}

#[derive(Debug, Clone)]
pub struct ListStateRequest {
    pub state: Option<TrustState>,
    pub kind: Option<String>,
    pub json: bool,
    pub roots: ResolvedRoots,
}

#[derive(Debug, Clone)]
pub struct UpdateDocRequest {
    pub path: Utf8PathBuf,
    pub roots: ResolvedRoots,
}

#[derive(Debug, Clone)]
pub struct DocsUnderRequest {
    pub path: Utf8PathBuf,
    pub folders_only: bool,
    pub files_only: bool,
    pub json: bool,
    pub roots: ResolvedRoots,
}

#[derive(Debug, Clone)]
pub struct SealRequest {
    pub path: Utf8PathBuf,
    pub roots: ResolvedRoots,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StatusReport {
    pub files: Vec<FileStatusJson>,
    pub folders: Vec<FolderStatusJson>,
    pub verification: VerificationStatusJson,
    pub ambiguous: Vec<AmbiguityJson>,
    pub changed: Vec<ChangedEntry>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChangedReport {
    pub changed: Vec<ChangedEntry>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncReport {
    pub templates_created: usize,
    pub docs_moved: usize,
    pub docs_deleted: usize,
    pub ambiguous_skipped: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ListStateReport {
    pub state: String,
    pub kind: String,
    pub files: Vec<FileStatusJson>,
    pub folders: Vec<FolderStatusJson>,
}

#[derive(Debug, Clone)]
pub struct UpdateDocReport {
    pub path: String,
    pub state: TrustState,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DocsUnderReport {
    pub folder: String,
    pub docs: Vec<DocUnderEntry>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DocUnderEntry {
    pub path: String,
    pub kind: String,
    pub description: Option<String>,
    pub trust_state: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SealReport {
    pub path: String,
    pub state: TrustState,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileStatusJson {
    pub path: String,
    pub state: String,
    pub content_sha256: String,
    pub description_doc_exists: bool,
    pub doc_current: bool,
    pub sealed_current: bool,
    pub change: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FolderStatusJson {
    pub path: String,
    pub state: String,
    pub purpose_doc_exists: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VerificationStatusJson {
    pub required: bool,
    pub policy: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AmbiguityJson {
    pub reason: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChangedEntry {
    pub change: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}
