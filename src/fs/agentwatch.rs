use camino::Utf8PathBuf;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

pub const DEFAULT_WATCH_PATTERNS: &[&str] = &["."];

pub fn write_default_agentwatch(map_root: &Utf8PathBuf) -> Result<(), crate::error::AdocsError> {
    let path = map_root.join(".adocs").join(".agentwatch");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = DEFAULT_WATCH_PATTERNS.join("\n") + "\n";
    std::fs::write(path.as_std_path(), content)?;
    Ok(())
}

pub fn build_watch_matcher(
    source_root: &Utf8PathBuf,
    map_root: &Utf8PathBuf,
) -> Result<Option<Gitignore>, crate::error::AdocsError> {
    let watch_file = map_root.join(".adocs").join(".agentwatch");

    if !watch_file.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(watch_file.as_std_path())?;
    let patterns: Vec<String> = content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    if patterns.is_empty() || patterns.iter().any(|p| p == ".") {
        return Ok(None);
    }

    let mut builder = GitignoreBuilder::new(source_root.as_std_path());
    builder.add_line(None, "*")?;

    for pattern in &patterns {
        let pattern = pattern.trim_end_matches('/');
        if pattern.is_empty() {
            continue;
        }
        builder.add_line(None, &format!("!{}", pattern))?;
        builder.add_line(None, &format!("!{}/**", pattern))?;
    }

    let gitignore = builder.build().map_err(crate::error::AdocsError::from)?;
    Ok(Some(gitignore))
}
