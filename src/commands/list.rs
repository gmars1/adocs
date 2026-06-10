use crate::model::TrustState;
use crate::{FileStatusJson, FolderStatusJson, ListStateReport};

pub fn run_list(request: &crate::ListStateRequest) -> Result<ListStateReport, crate::error::AdocsError> {
    let status = crate::commands::status::run_status(&crate::StatusRequest {
        json: false,
        roots: request.roots.clone(),
        fail_on_stale: false,
        fail_on_missing_docs: false,
        fail_on_ambiguous: false,
    })?;

    let state_filter = request.state.unwrap_or(TrustState::Stale);
    let state_str = state_filter.to_string();

    let files: Vec<FileStatusJson> = status
        .files
        .into_iter()
        .filter(|f| f.state == state_str)
        .collect();

    let folders: Vec<FolderStatusJson> = status
        .folders
        .into_iter()
        .filter(|f| f.state == state_str)
        .collect();

    Ok(ListStateReport {
        state: state_str,
        kind: request.kind.clone().unwrap_or_else(|| "all".to_string()),
        files,
        folders,
    })
}
