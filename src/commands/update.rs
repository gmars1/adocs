use camino::Utf8PathBuf;
use chrono::Utc;

use crate::fs::hash;
use crate::model::{DocEvidence, FilesLedger, FoldersLedger, TrustState};
use crate::UpdateDocReport;

pub fn run_update(request: &crate::UpdateDocRequest) -> Result<UpdateDocReport, crate::error::AdocsError> {
    let path_str = request.path.as_str();

    let source_abs = request.roots.source_root.join(&request.path);
    if !source_abs.exists() {
        return Err(crate::error::AdocsError::SourceFileNotFound(path_str.to_string()));
    }

    if source_abs.is_dir() {
        return update_folder(path_str, &request);
    }

    update_file(path_str, &request)
}

fn update_file(path_str: &str, request: &crate::UpdateDocRequest) -> Result<UpdateDocReport, crate::error::AdocsError> {
    let desc_path = crate::model::paths::file_description_path(path_str);
    let desc_abs = request.roots.map_root.join(&desc_path);
    if !desc_abs.exists() {
        return Err(crate::error::AdocsError::FileDescriptionNotFound(path_str.to_string()));
    }

    let source_abs = request.roots.source_root.join(&request.path);
    let current_source_hash = hash::hash_file(source_abs.as_std_path())?;
    let doc_content = std::fs::read_to_string(desc_abs.as_std_path())?;
    let doc_hash = hash::hash_string(&doc_content);

    let hashes_dir = request.roots.map_root.join(".adocs").join(".hashes");
    let ledger_path = hashes_dir.join("files.json");

    let mut ledger = FilesLedger::load(&ledger_path)
        .unwrap_or_else(|_| FilesLedger::new(
            request.roots.source_root.clone(),
            request.roots.map_root.clone(),
        ));

    let file_id = ledger
        .observed_path_index
        .get(&request.path)
        .cloned()
        .unwrap_or_else(|| crate::model::FileId::generate());

    let size = source_abs.metadata().map(|m| m.len()).unwrap_or(0);
    let mtime = source_abs
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let evidence = DocEvidence {
        accepted_source_sha256: current_source_hash.clone(),
        doc_sha256: doc_hash,
        accepted_at: Utc::now(),
    };

    let mut record = ledger.files.get(&file_id).cloned().unwrap_or_else(|| {
        crate::model::FileRecord {
            observed_path: request.path.clone(),
            path_history: vec![request.path.clone()],
            observed_content_sha256: current_source_hash.clone(),
            observed_at: Utc::now(),
            size,
            mtime,
            description_path: desc_path.clone(),
            doc: None,
            seal: None,
        }
    });

    record.doc = Some(evidence);
    record.observed_content_sha256 = current_source_hash.clone();
    record.description_path = desc_path;

    ledger.files.insert(file_id.clone(), record);
    ledger.observed_path_index.insert(request.path.clone(), file_id);
    ledger.save(&ledger_path)?;

    Ok(UpdateDocReport {
        path: path_str.to_string(),
        state: TrustState::Valid,
    })
}

fn update_folder(path_str: &str, request: &crate::UpdateDocRequest) -> Result<UpdateDocReport, crate::error::AdocsError> {
    let purpose_path = crate::model::paths::folder_purpose_path(path_str);
    let purpose_abs = request.roots.map_root.join(&purpose_path);
    if !purpose_abs.exists() {
        return Err(crate::error::AdocsError::FileDescriptionNotFound(path_str.to_string()));
    }

    let doc_content = std::fs::read_to_string(purpose_abs.as_std_path())?;
    let doc_hash = hash::hash_string(&doc_content);

    let hashes_dir = request.roots.map_root.join(".adocs").join(".hashes");
    let docs_ledger_path = hashes_dir.join("docs.json");

    let mut ledger = FoldersLedger::load(&docs_ledger_path)
        .unwrap_or_else(|_| FoldersLedger::new(
            request.roots.source_root.clone(),
            request.roots.map_root.clone(),
        ));

    let evidence = DocEvidence {
        accepted_source_sha256: doc_hash.clone(),
        doc_sha256: doc_hash.clone(),
        accepted_at: Utc::now(),
    };

    let record = crate::model::FolderRecord {
        purpose_path: purpose_path.clone(),
        doc_sha256: Some(doc_hash),
        doc: Some(evidence),
        seal: None,
    };

    ledger.folders.insert(Utf8PathBuf::from(path_str), record);
    ledger.save(&docs_ledger_path)?;

    Ok(UpdateDocReport {
        path: path_str.to_string(),
        state: TrustState::Valid,
    })
}
