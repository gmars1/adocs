use chrono::Utc;

use crate::fs::{atomic, discover};
use crate::model::{FileId, FileRecord, FilesLedger, ResolvedRoots};

pub fn run_scan(roots: &ResolvedRoots) -> Result<(), crate::error::AdocsError> {
    let observed = discover::discover_source_files(&roots.source_root, &roots.map_root, true)?;

    let hashes_dir = roots.map_root.join(".adocs").join(".hashes");
    let files_ledger_path = hashes_dir.join("files.json");

    let mut ledger = FilesLedger::new(roots.source_root.clone(), roots.map_root.clone());

    for obs in &observed {
        let existing_id = ledger.observed_path_index.get(&obs.source_path);

        let file_id = match existing_id {
            Some(id) => id.clone(),
            None => FileId::generate(),
        };

        let record = FileRecord {
            observed_path: obs.source_path.clone(),
            path_history: {
                if let Some(existing) = existing_id.and_then(|id| ledger.files.get(id)) {
                    let mut history = existing.path_history.clone();
                    if !history.contains(&obs.source_path) {
                        history.push(obs.source_path.clone());
                    }
                    history
                } else {
                    vec![obs.source_path.clone()]
                }
            },
            observed_content_sha256: obs.content_sha256.clone(),
            observed_at: Utc::now(),
            size: obs.size,
            mtime: obs.mtime,
            description_path: crate::model::paths::file_description_path(obs.source_path.as_str()),
            doc: ledger.files.get(&file_id).and_then(|r| r.doc.clone()),
            seal: ledger.files.get(&file_id).and_then(|r| r.seal.clone()),
        };

        ledger.files.insert(file_id.clone(), record);
        ledger.observed_path_index.insert(obs.source_path.clone(), file_id);
    }

    let json = serde_json::to_string_pretty(&ledger)?;
    atomic::write_atomic(&files_ledger_path, &json)?;

    println!("Scanned {} source files", observed.len());
    Ok(())
}
