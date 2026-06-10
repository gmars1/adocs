use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdocsConfig {
    pub source_root: Option<Utf8PathBuf>,
    pub map_root: Option<Utf8PathBuf>,
    pub verification: Option<VerificationPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPolicy {
    pub default: Option<String>,
    pub fallback: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedRoots {
    pub source_root: Utf8PathBuf,
    pub map_root: Utf8PathBuf,
}

pub fn resolve_roots(
    cli_source_root: Option<Utf8PathBuf>,
    cli_map_root: Option<Utf8PathBuf>,
    config_path: Option<Utf8PathBuf>,
) -> Result<ResolvedRoots, crate::error::AdocsError> {
    let cwd = std::env::current_dir()
        .map_err(anyhow::Error::from)?;
    let cwd = Utf8PathBuf::from_path_buf(cwd)
        .map_err(|_| crate::error::AdocsError::InvalidUtf8Path)?;

    let mut source_root: Option<Utf8PathBuf> = cli_source_root.clone();
    let mut map_root: Option<Utf8PathBuf> = cli_map_root.clone();

    if source_root.is_none() {
        if let Ok(val) = std::env::var("ADOCS_SOURCE_ROOT") {
            source_root = Some(resolve_path(&cwd, &val));
        }
    }
    if map_root.is_none() {
        if let Ok(val) = std::env::var("ADOCS_MAP_ROOT") {
            map_root = Some(resolve_path(&cwd, &val));
        }
    }

    let config_file_path = config_path
        .or_else(|| find_config_upwards(&cwd))
        .or_else(|| {
            let file = cwd.join("adocs.toml");
            if file.exists() { Some(file) } else { None }
        });

    if let Some(config_file) = &config_file_path {
        let config_dir = config_file
            .parent()
            .unwrap_or(&cwd);
        let contents = std::fs::read_to_string(config_file)
            .map_err(|e| crate::error::AdocsError::Io(e))?;
        let config: AdocsConfig = toml::from_str(&contents)?;

        if source_root.is_none() {
            if let Some(sr) = &config.source_root {
                source_root = Some(resolve_path(config_dir, sr.as_str()));
            }
        }
        if map_root.is_none() {
            if let Some(mr) = &config.map_root {
                map_root = Some(resolve_path(config_dir, mr.as_str()));
            }
        }
    }

    if source_root.is_none() {
        source_root = Some(cwd.clone());
    }
    if map_root.is_none() {
        map_root = Some(cwd.clone());
    }

    let source_root = source_root.unwrap();
    let map_root = map_root.unwrap();

    Ok(ResolvedRoots {
        source_root,
        map_root,
    })
}

fn find_config_upwards(cwd: &Utf8PathBuf) -> Option<Utf8PathBuf> {
    let mut current = cwd.clone();
    loop {
        let candidate = current.join("adocs.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn resolve_path(base: &Utf8Path, path: &str) -> Utf8PathBuf {
    let p = Utf8PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        let b = Utf8PathBuf::from(base);
        b.join(&p)
    }
}
