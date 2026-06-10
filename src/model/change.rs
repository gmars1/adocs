use camino::Utf8PathBuf;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChange {
    Added { path: Utf8PathBuf },
    Modified { path: Utf8PathBuf },
    Deleted { path: Utf8PathBuf },
    Moved { from: Utf8PathBuf, to: Utf8PathBuf },
    Renamed { from: Utf8PathBuf, to: Utf8PathBuf },
    Ambiguous { reason: String, paths: Vec<Utf8PathBuf> },
    Unchanged { path: Utf8PathBuf },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderChange {
    Added { path: Utf8PathBuf },
    Removed { path: Utf8PathBuf },
    Unchanged { path: Utf8PathBuf },
}

impl FileChange {
    pub fn path_str(&self) -> String {
        match self {
            FileChange::Added { path } => path.to_string(),
            FileChange::Modified { path } => path.to_string(),
            FileChange::Deleted { path } => path.to_string(),
            FileChange::Moved { to, .. } => to.to_string(),
            FileChange::Renamed { to, .. } => to.to_string(),
            FileChange::Ambiguous { paths, .. } => {
                paths.first().map(|p| p.to_string()).unwrap_or_default()
            }
            FileChange::Unchanged { path } => path.to_string(),
        }
    }
}
