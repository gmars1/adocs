use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FileId(pub String);

impl std::fmt::Display for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesLedger {
    pub version: u32,
    pub source_root: Utf8PathBuf,
    pub map_root: Utf8PathBuf,
    pub files: BTreeMap<FileId, FileRecord>,
    pub observed_path_index: BTreeMap<Utf8PathBuf, FileId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub observed_path: Utf8PathBuf,
    pub path_history: Vec<Utf8PathBuf>,
    pub observed_content_sha256: String,
    pub observed_at: DateTime<Utc>,
    pub size: u64,
    pub mtime: i64,
    pub description_path: Utf8PathBuf,
    pub doc: Option<DocEvidence>,
    pub seal: Option<SealEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEvidence {
    pub accepted_source_sha256: String,
    pub doc_sha256: String,
    pub accepted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealEvidence {
    pub source_sha256: String,
    pub sealed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldersLedger {
    pub version: u32,
    pub source_root: Utf8PathBuf,
    pub map_root: Utf8PathBuf,
    pub folders: BTreeMap<Utf8PathBuf, FolderRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderRecord {
    pub purpose_path: Utf8PathBuf,
    pub doc_sha256: Option<String>,
    pub doc: Option<DocEvidence>,
    pub seal: Option<SealEvidence>,
}

impl FilesLedger {
    pub fn new(source_root: Utf8PathBuf, map_root: Utf8PathBuf) -> Self {
        Self {
            version: 1,
            source_root,
            map_root,
            files: BTreeMap::new(),
            observed_path_index: BTreeMap::new(),
        }
    }

    pub fn load(path: &Utf8PathBuf) -> Result<Self, crate::error::AdocsError> {
        if !path.exists() {
            return Err(crate::error::AdocsError::AgentMapMissing);
        }
        let contents = std::fs::read_to_string(path)?;
        let ledger: FilesLedger = serde_json::from_str(&contents)?;
        Ok(ledger)
    }

    pub fn save(&self, path: &Utf8PathBuf) -> Result<(), crate::error::AdocsError> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &json)?;
        Ok(())
    }
}

impl FoldersLedger {
    pub fn new(source_root: Utf8PathBuf, map_root: Utf8PathBuf) -> Self {
        Self {
            version: 1,
            source_root,
            map_root,
            folders: BTreeMap::new(),
        }
    }

    pub fn load(path: &Utf8PathBuf) -> Result<Self, crate::error::AdocsError> {
        let contents = std::fs::read_to_string(path)?;
        let ledger: FoldersLedger = serde_json::from_str(&contents)?;
        Ok(ledger)
    }

    pub fn save(&self, path: &Utf8PathBuf) -> Result<(), crate::error::AdocsError> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &json)?;
        Ok(())
    }
}

impl FileId {
    pub fn generate() -> Self {
        let id = ulid::Ulid::new().to_string().to_lowercase();
        FileId(format!("file_{}", id))
    }
}
