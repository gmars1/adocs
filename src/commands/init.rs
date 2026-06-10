use camino::Utf8PathBuf;
use std::collections::{BTreeMap, BTreeSet};

use crate::fs::{agenignore, agentwatch, discover};
use crate::model::{FileRecord, FileId, FilesLedger, FoldersLedger, ResolvedRoots};
use crate::SyncReport;

pub fn run_init(roots: &ResolvedRoots, force: bool) -> Result<(), crate::error::AdocsError> {
    let adocs = roots.map_root.join(".adocs");
    let hashes = adocs.join(".hashes");
    let agents = adocs.join("agents");

    if adocs.exists() && !force {
        return Err(crate::error::AdocsError::Other(anyhow::anyhow!(
            ".adocs/ already exists under map root. Use --force to reinitialize."
        )));
    }

    std::fs::create_dir_all(&hashes)?;
    std::fs::create_dir_all(&agents)?;
    agenignore::write_default_agenignore(&roots.map_root)?;
    agentwatch::write_default_agentwatch(&roots.map_root)?;

    let files_ledger = crate::model::ledger::FilesLedger::new(
        roots.source_root.clone(),
        roots.map_root.clone(),
    );
    files_ledger.save(&hashes.join("files.json"))?;

    let folders_ledger = crate::model::ledger::FoldersLedger::new(
        roots.source_root.clone(),
        roots.map_root.clone(),
    );
    folders_ledger.save(&hashes.join("docs.json"))?;

    println!("Initialized .adocs/ under {}", roots.map_root);
    println!("  source root: {}", roots.source_root);
    println!("  map root:    {}", roots.map_root);
    Ok(())
}

pub fn run_sync(request: &crate::SyncRequest) -> Result<SyncReport, crate::error::AdocsError> {
    let observed = discover::discover_source_files(&request.roots.source_root, &request.roots.map_root, true)?;

    let hashes_dir = request.roots.map_root.join(".adocs").join(".hashes");
    let files_ledger_path = hashes_dir.join("files.json");
    let docs_ledger_path = hashes_dir.join("docs.json");

    let prev_ledger = FilesLedger::load(&files_ledger_path)
        .unwrap_or_else(|_| FilesLedger::new(
            request.roots.source_root.clone(),
            request.roots.map_root.clone(),
        ));

    let mut prev_folders = FoldersLedger::load(&docs_ledger_path)
        .unwrap_or_else(|_| FoldersLedger::new(
            request.roots.source_root.clone(),
            request.roots.map_root.clone(),
        ));

    let mut ledger = prev_ledger.clone();

    let mut current_by_path: BTreeMap<Utf8PathBuf, &discover::ObservedFile> = BTreeMap::new();
    let mut current_by_hash: BTreeMap<String, Vec<&discover::ObservedFile>> = BTreeMap::new();
    for obs in &observed {
        current_by_path.insert(obs.source_path.clone(), obs);
        current_by_hash
            .entry(obs.content_sha256.clone())
            .or_default()
            .push(obs);
    }

    let mut templates_created = 0usize;
    let mut docs_moved = 0usize;
    let mut docs_deleted = 0usize;
    let mut ambiguous_skipped = 0usize;

    let prev_paths_and_records: Vec<(Utf8PathBuf, crate::model::FileRecord)> = prev_ledger
        .observed_path_index
        .iter()
        .filter_map(|(path, id)| {
            prev_ledger.files.get(id).map(|r| (path.clone(), r.clone()))
        })
        .collect();

    for (prev_path, record) in &prev_paths_and_records {
        if current_by_path.contains_key(prev_path) {
            continue;
        }

        let candidates = current_by_hash
            .get(&record.observed_content_sha256)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        if candidates.is_empty() {
            let desc_abs = request.roots.map_root.join(&record.description_path);
            if desc_abs.exists() {
                std::fs::remove_file(&desc_abs)?;
                docs_deleted += 1;
            }
            if let Some(file_id) = ledger.observed_path_index.remove(prev_path) {
                ledger.files.remove(&file_id);
            }
        } else if candidates.len() == 1 {
            let new_path = &candidates[0].source_path;
            let old_desc = record.description_path.clone();
            let new_desc = crate::model::paths::file_description_path(new_path.as_str());

            if old_desc != new_desc {
                let old_abs = request.roots.map_root.join(&old_desc);
                let new_abs = request.roots.map_root.join(&new_desc);
                if old_abs.exists() {
                    if let Some(parent) = new_abs.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::rename(&old_abs, &new_abs)?;
                    docs_moved += 1;
                }
            }

            let file_id = prev_ledger
                .observed_path_index
                .get(prev_path)
                .cloned()
                .unwrap_or_else(|| crate::model::FileId::generate());

            let mut updated = record.clone();
            updated.observed_path = new_path.clone();
            if !updated.path_history.contains(new_path) {
                updated.path_history.push(new_path.clone());
            }
            updated.description_path = new_desc;
            ledger.observed_path_index.remove(prev_path);
            ledger.observed_path_index.insert(new_path.clone(), file_id.clone());
            ledger.files.insert(file_id, updated);
        } else {
            ambiguous_skipped += 1;
        }
    }

    for obs in &observed {
        let already_known = prev_ledger.observed_path_index.contains_key(&obs.source_path);
        if already_known {
            continue;
        }

        let desc_path = crate::model::paths::file_description_path(obs.source_path.as_str());
        let desc_abs = request.roots.map_root.join(&desc_path);
        if !desc_abs.exists() {
            if let Some(parent) = desc_abs.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(
                &desc_abs,
                format!("# {}\n\nTODO: describe this file\n", obs.source_path),
            )?;
            templates_created += 1;
        }

        if let Some(folder_path) = obs.source_path.parent() {
            let purpose_path = crate::model::paths::folder_purpose_path(folder_path.as_str());
            let purpose_abs = request.roots.map_root.join(&purpose_path);
            if !purpose_abs.exists() {
                if let Some(parent) = purpose_abs.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(
                    &purpose_abs,
                    format!("# {}\n\nTODO: describe why this folder exists\n", folder_path),
                )?;
                templates_created += 1;

                prev_folders.folders.entry(folder_path.to_owned()).or_insert_with(|| {
                    crate::model::FolderRecord {
                        purpose_path: purpose_path.clone(),
                        doc_sha256: None,
                        doc: None,
                        seal: None,
                    }
                });
            }
        }
    }

    let now = chrono::Utc::now();
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
            observed_at: now,
            size: obs.size,
            mtime: obs.mtime,
            description_path: crate::model::paths::file_description_path(obs.source_path.as_str()),
            doc: ledger.files.get(&file_id).and_then(|r| r.doc.clone()),
            seal: ledger.files.get(&file_id).and_then(|r| r.seal.clone()),
        };

        ledger.files.insert(file_id.clone(), record);
        ledger.observed_path_index.insert(obs.source_path.clone(), file_id);
    }

    {
        let active_folders: BTreeSet<Utf8PathBuf> = observed
            .iter()
            .filter_map(|obs| obs.source_path.parent().map(|p| p.to_owned()))
            .collect();

        for folder_path in &active_folders {
            let purpose_path = crate::model::paths::folder_purpose_path(folder_path.as_str());
            let purpose_abs = request.roots.map_root.join(&purpose_path);
            if !purpose_abs.exists() {
                if let Some(parent) = purpose_abs.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(
                    &purpose_abs,
                    format!("# {}\n\nTODO: describe why this folder exists\n", folder_path),
                )?;
                templates_created += 1;

                prev_folders.folders.entry(folder_path.clone()).or_insert_with(|| {
                    crate::model::FolderRecord {
                        purpose_path: purpose_path.clone(),
                        doc_sha256: None,
                        doc: None,
                        seal: None,
                    }
                });
            }
        }

        let orphaned: Vec<Utf8PathBuf> = prev_folders
            .folders
            .keys()
            .filter(|k| !active_folders.contains(*k))
            .cloned()
            .collect();

        for folder_path in &orphaned {
            if let Some(record) = prev_folders.folders.remove(folder_path) {
                let purpose_abs = request.roots.map_root.join(&record.purpose_path);
                if purpose_abs.exists() {
                    std::fs::remove_file(&purpose_abs)?;
                    docs_deleted += 1;
                }
            }
        }
    }

    ledger.save(&files_ledger_path)?;
    prev_folders.save(&docs_ledger_path)?;

    Ok(SyncReport {
        templates_created,
        docs_moved,
        docs_deleted,
        ambiguous_skipped,
    })
}

