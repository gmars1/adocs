use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdocsError {
    #[error("not inside configured roots")]
    NotInsideRoots,

    #[error(".adocs/ missing — run `adocs init` first")]
    AgentMapMissing,

    #[error("source root missing: {0}")]
    SourceRootMissing(String),

    #[error("map root missing: {0}")]
    MapRootMissing(String),

    #[error("invalid UTF-8 path")]
    InvalidUtf8Path,

    #[error("path escapes its root: {0}")]
    PathEscapesRoot(String),

    #[error("source file not found: {0}")]
    SourceFileNotFound(String),

    #[error("file description not found: {0}")]
    FileDescriptionNotFound(String),

    #[error("cannot seal stale file: {0}")]
    CannotSealStale(String),

    #[error("ambiguous file identity: {reason}, paths: {paths:?}")]
    AmbiguousIdentity { reason: String, paths: Vec<String> },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    Toml(#[from] toml::de::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),

    #[error(transparent)]
    Ignore(#[from] ignore::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}
