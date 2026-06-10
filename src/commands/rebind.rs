use camino::Utf8PathBuf;

use crate::model::{FileId, FilesLedger, ResolvedRoots};

pub fn run_rebind(
    file_id: &FileId,
    new_path: &Utf8PathBuf,
    roots: &ResolvedRoots,
) -> Result<(), crate::error::AdocsError> {
    let hashes_dir = roots.map_root.join(".adocs").join(".hashes");
    let ledger_path = hashes_dir.join("files.json");

    let mut ledger = FilesLedger::load(&ledger_path)?;

    let record = ledger
        .files
        .get_mut(file_id)
        .ok_or_else(|| crate::error::AdocsError::SourceFileNotFound(file_id.to_string()))?;

    let old_path = record.observed_path.clone();
    record.path_history.push(old_path.clone());
    record.observed_path = new_path.clone();
    record.description_path = crate::model::paths::file_description_path(new_path.as_str());

    ledger.observed_path_index.remove(&old_path);
    ledger.observed_path_index.insert(new_path.clone(), file_id.clone());

    ledger.save(&ledger_path)?;

    println!("Rebound {} from {} to {}", file_id, old_path, new_path);
    Ok(())
}
