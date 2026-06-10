use chrono::Utc;

use crate::fs::hash;
use crate::model::{file_state, FilesLedger, SealEvidence, TrustState};
use crate::SealReport;

pub fn run_seal(request: &crate::SealRequest) -> Result<SealReport, crate::error::AdocsError> {
    let path_str = request.path.as_str();

    let hashes_dir = request.roots.map_root.join(".adocs").join(".hashes");
    let ledger_path = hashes_dir.join("files.json");

    let mut ledger = FilesLedger::load(&ledger_path)
        .unwrap_or_else(|_| FilesLedger::new(
            request.roots.source_root.clone(),
            request.roots.map_root.clone(),
        ));

    let source_abs = request.roots.source_root.join(&request.path);
    if !source_abs.exists() {
        return Err(crate::error::AdocsError::SourceFileNotFound(path_str.to_string()));
    }

    let current_hash = hash::hash_file(source_abs.as_std_path())?;

    let file_id = ledger
        .observed_path_index
        .get(&request.path)
        .cloned()
        .ok_or_else(|| {
            crate::error::AdocsError::SourceFileNotFound(path_str.to_string())
        })?;

    let record = ledger
        .files
        .get(&file_id)
        .ok_or_else(|| {
            crate::error::AdocsError::SourceFileNotFound(path_str.to_string())
        })?;

    let description_exists = request
        .roots
        .map_root
        .join(&record.description_path)
        .exists();

    let current_state = file_state(
        &current_hash,
        record.doc.as_ref(),
        record.seal.as_ref(),
        description_exists,
    );

    if current_state == TrustState::Stale {
        return Err(crate::error::AdocsError::CannotSealStale(path_str.to_string()));
    }

    let seal = SealEvidence {
        source_sha256: current_hash.clone(),
        sealed_at: Utc::now(),
    };

    let mut updated = record.clone();
    updated.seal = Some(seal);
    ledger.files.insert(file_id, updated);
    ledger.save(&ledger_path)?;

    Ok(SealReport {
        path: path_str.to_string(),
        state: TrustState::Sealed,
    })
}
