use std::collections::BTreeMap;

use camino::Utf8PathBuf;

use crate::fs::{discover, hash};
use crate::model::{
    file_state, folder_purpose_state, FileChange, FilesLedger, FoldersLedger,
};
use crate::{
    AmbiguityJson, ChangedEntry, ChangedReport, FileStatusJson, FolderStatusJson,
    StatusReport, VerificationStatusJson,
};

pub fn run_status(request: &crate::StatusRequest) -> Result<StatusReport, crate::error::AdocsError> {
    let observed = discover::discover_source_files(&request.roots.source_root, &request.roots.map_root, true)?;
    let hashes_dir = request.roots.map_root.join(".adocs").join(".hashes");

    let prev_ledger = FilesLedger::load(&hashes_dir.join("files.json"))
        .unwrap_or_else(|_| FilesLedger::new(
            request.roots.source_root.clone(),
            request.roots.map_root.clone(),
        ));

    let prev_folders = FoldersLedger::load(&hashes_dir.join("docs.json"))
        .unwrap_or_else(|_| FoldersLedger::new(
            request.roots.source_root.clone(),
            request.roots.map_root.clone(),
        ));

    let changes = detect_changes(&prev_ledger, &observed);
    let file_statuses = compute_file_status(&prev_ledger, &observed, &changes);
    let folder_statuses = compute_folder_status(
        &request.roots.map_root,
        &observed,
        &prev_folders,
    );
    let ambiguities = extract_ambiguities(&changes);

    let config = load_optional_config(&request.roots.map_root);
    let verification_policy = config.and_then(|c| c.verification.and_then(|v| v.default));

    let report = StatusReport {
        files: file_statuses,
        folders: folder_statuses,
        verification: VerificationStatusJson {
            required: true,
            policy: verification_policy,
        },
        ambiguous: ambiguities,
        changed: changes
            .iter()
            .filter(|c| !matches!(c, FileChange::Unchanged { .. }))
            .map(|c| {
                let (change, path, from) = match c {
                    FileChange::Added { path } => ("added".to_string(), path.to_string(), None),
                    FileChange::Modified { path } => ("modified".to_string(), path.to_string(), None),
                    FileChange::Deleted { path } => ("deleted".to_string(), path.to_string(), None),
                    FileChange::Moved { from, to } => ("moved".to_string(), to.to_string(), Some(from.to_string())),
                    FileChange::Renamed { from, to } => ("renamed".to_string(), to.to_string(), Some(from.to_string())),
                    FileChange::Ambiguous { reason: _, paths } => ("ambiguous".to_string(), paths.first().map(|p| p.to_string()).unwrap_or_default(), None),
                    FileChange::Unchanged { .. } => unreachable!(),
                };
                ChangedEntry { change, path, from }
            })
            .collect(),
    };

    Ok(report)
}

fn load_optional_config(map_root: &Utf8PathBuf) -> Option<crate::model::config::AdocsConfig> {
    let config_path = map_root.join("adocs.toml");
    if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
    } else {
        let cwd = std::env::current_dir().ok()?;
        let cwd = Utf8PathBuf::from_path_buf(cwd).ok()?;
        let cwd_config = cwd.join("adocs.toml");
        if cwd_config.exists() {
            std::fs::read_to_string(&cwd_config)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
        } else {
            None
        }
    }
}

pub fn run_changed(request: &crate::ChangedRequest) -> Result<ChangedReport, crate::error::AdocsError> {
    let status = run_status(&crate::StatusRequest {
        json: request.json,
        roots: request.roots.clone(),
        fail_on_stale: false,
        fail_on_missing_docs: false,
        fail_on_ambiguous: false,
    })?;
    Ok(ChangedReport {
        changed: status.changed,
    })
}

fn detect_changes(
    prev: &FilesLedger,
    current: &[discover::ObservedFile],
) -> Vec<FileChange> {
    let mut changes = Vec::new();
    let mut current_by_path: BTreeMap<&Utf8PathBuf, &discover::ObservedFile> = BTreeMap::new();
    let mut current_by_hash: BTreeMap<&str, Vec<&discover::ObservedFile>> = BTreeMap::new();

    for obs in current {
        current_by_path.insert(&obs.source_path, obs);
        current_by_hash
            .entry(&obs.content_sha256)
            .or_default()
            .push(obs);
    }

    let mut previous_paths: BTreeMap<&Utf8PathBuf, &crate::model::FileRecord> = BTreeMap::new();
    let mut previous_ids: BTreeMap<&crate::model::FileId, &crate::model::FileRecord> = BTreeMap::new();

    for (id, record) in &prev.files {
        previous_paths.insert(&record.observed_path, record);
        previous_ids.insert(id, record);
    }

    for (path, record) in &previous_paths {
        if let Some(current_obs) = current_by_path.get(path) {
            if current_obs.content_sha256 == record.observed_content_sha256 {
                changes.push(FileChange::Unchanged {
                    path: (*path).clone(),
                });
            } else {
                changes.push(FileChange::Modified {
                    path: (*path).clone(),
                });
            }
        } else {
            let candidates = current_by_hash
                .get(record.observed_content_sha256.as_str())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            if candidates.is_empty() {
                changes.push(FileChange::Deleted {
                    path: (*path).clone(),
                });
            } else if candidates.len() == 1 {
                let candidate = candidates[0];
                let dir_same = path.parent() == candidate.source_path.parent();
                if dir_same {
                    changes.push(FileChange::Renamed {
                        from: (*path).clone(),
                        to: candidate.source_path.clone(),
                    });
                } else {
                    changes.push(FileChange::Moved {
                        from: (*path).clone(),
                        to: candidate.source_path.clone(),
                    });
                }
            } else {
                let paths: Vec<Utf8PathBuf> =
                    candidates.iter().map(|c| c.source_path.clone()).collect();
                changes.push(FileChange::Ambiguous {
                    reason: format!(
                        "content hash {} matches multiple files",
                        record.observed_content_sha256
                    ),
                    paths,
                });
            }
        }
    }

    for (path, _obs) in &current_by_path {
        if !previous_paths.contains_key(path) {
            let already_matched = changes.iter().any(|c| match c {
                FileChange::Moved { to, .. } | FileChange::Renamed { to, .. } => to == *path,
                _ => false,
            });
            if !already_matched {
                changes.push(FileChange::Added {
                    path: (*path).clone(),
                });
            }
        }
    }

    changes
}

fn compute_file_status(
    prev: &FilesLedger,
    current: &[discover::ObservedFile],
    changes: &[FileChange],
) -> Vec<FileStatusJson> {
    let mut statuses = Vec::new();

    let change_map: BTreeMap<&Utf8PathBuf, &FileChange> = changes
        .iter()
        .map(|c| {
            let path = match c {
                FileChange::Added { path } | FileChange::Modified { path } | FileChange::Unchanged { path } => path,
                FileChange::Moved { to, .. } | FileChange::Renamed { to, .. } => to,
                FileChange::Deleted { path } => path,
                FileChange::Ambiguous { paths, .. } => paths.first().unwrap(),
            };
            (path, c)
        })
        .collect();

    for obs in current {
        let record = prev
            .observed_path_index
            .get(&obs.source_path)
            .and_then(|id| prev.files.get(id));

        let change = change_map.get(&obs.source_path);
        let description_exists = record
            .map(|r| {
                let desc_abs = prev
                    .map_root
                    .join(&r.description_path);
                desc_abs.exists()
            })
            .unwrap_or(false);

        let state = file_state(
            &obs.content_sha256,
            record.and_then(|r| r.doc.as_ref()),
            record.and_then(|r| r.seal.as_ref()),
            description_exists,
        );

        let change_str = change.map(|c| match c {
            FileChange::Added { .. } => "added",
            FileChange::Modified { .. } => "modified",
            FileChange::Deleted { .. } => "deleted",
            FileChange::Moved { .. } => "moved",
            FileChange::Renamed { .. } => "renamed",
            FileChange::Ambiguous { .. } => "ambiguous",
            FileChange::Unchanged { .. } => "unchanged",
        });

        statuses.push(FileStatusJson {
            path: obs.source_path.to_string(),
            state: state.to_string(),
            content_sha256: obs.content_sha256.clone(),
            description_doc_exists: description_exists,
            doc_current: record
                .and_then(|r| r.doc.as_ref())
                .map(|d| d.accepted_source_sha256 == obs.content_sha256)
                .unwrap_or(false),
            sealed_current: record
                .and_then(|r| r.seal.as_ref())
                .map(|s| s.source_sha256 == obs.content_sha256)
                .unwrap_or(false),
            change: change_str.map(|s| s.to_string()),
        });
    }

    statuses.sort_by(|a, b| a.path.cmp(&b.path));
    statuses
}

fn compute_folder_status(
    map_root: &Utf8PathBuf,
    observed: &[discover::ObservedFile],
    prev_folders: &FoldersLedger,
) -> Vec<FolderStatusJson> {
    let mut folders: BTreeMap<String, bool> = BTreeMap::new();

    for obs in observed {
        if let Some(parent) = obs.source_path.parent() {
            folders.entry(parent.to_string()).or_insert(true);
        }
    }

    let mut statuses = Vec::new();

    for (folder_path, _) in &folders {
        let purpose_path = crate::model::paths::folder_purpose_path(folder_path);
        let purpose_abs = map_root.join(&purpose_path);
        let purpose_exists = purpose_abs.exists();

        let purpose_hash = if purpose_exists {
            hash::hash_file(purpose_abs.as_std_path()).ok()
        } else {
            None
        };

        let folder_record = prev_folders.folders.get(
            &Utf8PathBuf::from(folder_path.as_str()),
        );

        let state = folder_purpose_state(
            purpose_exists,
            folder_record.and_then(|r| r.doc.as_ref()),
            purpose_hash.as_deref(),
            folder_record.and_then(|r| r.seal.as_ref()),
        );

        statuses.push(FolderStatusJson {
            path: folder_path.to_string(),
            state: state.to_string(),
            purpose_doc_exists: purpose_exists,
        });
    }

    for (folder_path, record) in &prev_folders.folders {
        if !folders.contains_key(folder_path.as_str()) {
            let purpose_abs = map_root.join(&record.purpose_path);
            if !purpose_abs.exists() {
                continue;
            }
            statuses.push(FolderStatusJson {
                path: folder_path.to_string(),
                state: "stale".to_string(),
                purpose_doc_exists: true,
            });
        }
    }

    statuses.sort_by(|a, b| a.path.cmp(&b.path));
    statuses
}

fn extract_ambiguities(changes: &[FileChange]) -> Vec<AmbiguityJson> {
    changes
        .iter()
        .filter_map(|c| {
            if let FileChange::Ambiguous { reason, paths } = c {
                Some(AmbiguityJson {
                    reason: reason.clone(),
                    paths: paths.iter().map(|p| p.to_string()).collect(),
                })
            } else {
                None
            }
        })
        .collect()
}
