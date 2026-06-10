use camino::Utf8PathBuf;
use std::collections::BTreeSet;

use ignore::{DirEntry, WalkBuilder};

use super::agenignore;
use super::agentwatch;
use super::hash;

#[derive(Debug, Clone)]
pub struct ObservedFile {
    pub source_path: Utf8PathBuf,
    pub absolute_path: std::path::PathBuf,
    pub content_sha256: String,
    pub size: u64,
    pub mtime: i64,
}

pub fn discover_source_files(
    source_root: &Utf8PathBuf,
    map_root: &Utf8PathBuf,
    respect_gitignore: bool,
) -> Result<Vec<ObservedFile>, crate::error::AdocsError> {
    let source_abs = source_root.as_std_path().canonicalize().map_err(|e| {
        crate::error::AdocsError::SourceRootMissing(format!("{}: {}", source_root, e))
    })?;
    let source_abs = Utf8PathBuf::from_path_buf(source_abs)
        .map_err(|_| crate::error::AdocsError::InvalidUtf8Path)?;

    let matcher = agenignore::build_ignore_matcher(map_root)?;
    let watch = agentwatch::build_watch_matcher(&source_abs, map_root)?;

    let mut builder = WalkBuilder::new(source_abs.as_std_path());
    builder.standard_filters(respect_gitignore);
    builder.hidden(false);
    builder.ignore(false);
    builder.parents(false);
    builder.git_ignore(respect_gitignore);
    builder.git_exclude(respect_gitignore);
    builder.require_git(false);

    if let Some(m) = matcher {
        builder.filter_entry(move |entry: &DirEntry| -> bool {
            let is_dir = entry
                .file_type()
                .map_or(false, |ft: std::fs::FileType| ft.is_dir());
            let matched = m.matched(entry.path(), is_dir);
            if matched.is_ignore() {
                return false;
            }
            if let Some(ref w) = watch {
                if !is_dir && w.matched(entry.path(), false).is_ignore() {
                    return false;
                }
            }
            if let Some(name) = entry
                .path()
                .file_name()
                .and_then(|n: &std::ffi::OsStr| n.to_str())
            {
                if name == ".adocs" || name == ".git" {
                    return false;
                }
            }
            true
        });
    } else if let Some(w) = watch {
        builder.filter_entry(move |entry: &DirEntry| -> bool {
            let is_dir = entry
                .file_type()
                .map_or(false, |ft: std::fs::FileType| ft.is_dir());
            if !is_dir && w.matched(entry.path(), false).is_ignore() {
                return false;
            }
            if let Some(name) = entry
                .path()
                .file_name()
                .and_then(|n: &std::ffi::OsStr| n.to_str())
            {
                if name == ".adocs" || name == ".git" {
                    return false;
                }
            }
            true
        });
    }

    let walker = builder.build();
    let mut observed = Vec::new();
    let mut seen = BTreeSet::new();

    for result in walker {
        let entry: DirEntry = result?;
        if !entry
            .file_type()
            .map_or(false, |ft: std::fs::FileType| ft.is_file())
        {
            continue;
        }

        let abs_path = entry.into_path();
        let abs_path = abs_path.canonicalize().unwrap_or(abs_path);

        if !seen.insert(abs_path.clone()) {
            continue;
        }

        let rel_path = pathdiff::diff_paths(&abs_path, &source_abs).ok_or_else(|| {
            crate::error::AdocsError::PathEscapesRoot(abs_path.display().to_string())
        })?;

        let rel_path = Utf8PathBuf::from_path_buf(rel_path)
            .map_err(|_| crate::error::AdocsError::InvalidUtf8Path)?;

        let content_hash = hash::hash_file(&abs_path)?;
        let metadata = std::fs::metadata(&abs_path)?;
        let mtime = metadata
            .modified()
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0)
            })
            .unwrap_or(0);

        observed.push(ObservedFile {
            source_path: rel_path,
            absolute_path: abs_path,
            content_sha256: content_hash,
            size: metadata.len(),
            mtime,
        });
    }

    observed.sort_by(|a, b| a.source_path.cmp(&b.source_path));
    Ok(observed)
}
