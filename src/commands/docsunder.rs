use std::collections::BTreeSet;

use camino::Utf8PathBuf;

use crate::fs::{discover, hash};
use crate::model::{file_state, folder_purpose_state, FilesLedger, FoldersLedger, TrustState};
use crate::{DocsUnderReport, DocUnderEntry};

pub fn run_docs_under(request: &crate::DocsUnderRequest) -> Result<DocsUnderReport, crate::error::AdocsError> {
    let all_files = discover::discover_source_files(&request.roots.source_root, &request.roots.map_root, true)?;
    let hashes_dir = request.roots.map_root.join(".adocs").join(".hashes");

    let files_ledger = FilesLedger::load(&hashes_dir.join("files.json"))
        .unwrap_or_else(|_| FilesLedger::new(request.roots.source_root.clone(), request.roots.map_root.clone()));
    let folders_ledger = FoldersLedger::load(&hashes_dir.join("docs.json"))
        .unwrap_or_else(|_| FoldersLedger::new(request.roots.source_root.clone(), request.roots.map_root.clone()));

    let folder_prefix: Utf8PathBuf = request.path.clone();
    let folder_prefix_str = folder_prefix.as_str();

    let mut matching_files: Vec<&discover::ObservedFile> = all_files
        .iter()
        .filter(|f| {
            let p = f.source_path.as_str();
            p == folder_prefix_str || p.starts_with(&format!("{}/", folder_prefix_str))
        })
        .collect();

    // Collect unique subfolders from matching files
    let mut folders: BTreeSet<Utf8PathBuf> = BTreeSet::new();
    // Always include the requested folder itself
    folders.insert(folder_prefix.clone());

    for obs in &matching_files {
        if let Some(parent) = obs.source_path.parent() {
            let parent_str = parent.as_str();
            if parent_str == folder_prefix_str || parent_str.starts_with(&format!("{}/", folder_prefix_str)) {
                folders.insert(parent.to_path_buf());
            }
        }
    }

    let mut docs = Vec::new();

    // File descriptions
    if !request.folders_only {
        matching_files.sort_by(|a, b| a.source_path.cmp(&b.source_path));
        for obs in &matching_files {
            let record = files_ledger
                .observed_path_index
                .get(&obs.source_path)
                .and_then(|id| files_ledger.files.get(id));

            let desc_path = crate::model::paths::file_description_path(obs.source_path.as_str());
            let desc_abs = request.roots.map_root.join(&desc_path);
            let desc_exists = desc_abs.exists();
            let content = if desc_exists {
                std::fs::read_to_string(desc_abs.as_std_path()).ok()
            } else {
                None
            };

            let state = file_state(
                &obs.content_sha256,
                record.and_then(|r| r.doc.as_ref()),
                record.and_then(|r| r.seal.as_ref()),
                desc_exists,
            );

            if state != TrustState::Valid {
                continue;
            }

            docs.push(DocUnderEntry {
                path: obs.source_path.to_string(),
                kind: "file".to_string(),
                description: content,
                trust_state: Some(state.to_string()),
            });
        }
    }

    // Folder purposes
    if !request.files_only {
        let mut sorted_folders: Vec<_> = folders.into_iter().collect();
        sorted_folders.sort();
        for folder in &sorted_folders {
            let purp_path = crate::model::paths::folder_purpose_path(folder.as_str());
            let purp_abs = request.roots.map_root.join(&purp_path);
            let purp_exists = purp_abs.exists();
            let content = if purp_exists {
                std::fs::read_to_string(purp_abs.as_std_path()).ok()
            } else {
                None
            };

            let purp_hash = if purp_exists {
                hash::hash_file(purp_abs.as_std_path()).ok()
            } else {
                None
            };

            let folder_record = folders_ledger.folders.get(folder);

            let state = folder_purpose_state(
                purp_exists,
                folder_record.and_then(|r| r.doc.as_ref()),
                purp_hash.as_deref(),
                folder_record.and_then(|r| r.seal.as_ref()),
            );

            if state != TrustState::Valid {
                continue;
            }

            docs.push(DocUnderEntry {
                path: folder.to_string(),
                kind: "folder".to_string(),
                description: content,
                trust_state: Some(state.to_string()),
            });
        }
    }

    Ok(DocsUnderReport {
        folder: folder_prefix.to_string(),
        docs,
    })
}
