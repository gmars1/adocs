use camino::Utf8PathBuf;
use ignore::gitignore::Gitignore;

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

    if patterns.is_empty() {
        return Ok(None);
    }

    if patterns.len() == 1 && patterns[0] == "." {
        return Ok(None);
    }

    let mut gitignore_content = String::from("*\n");
    for pattern in &patterns {
        gitignore_content.push('!');
        gitignore_content.push_str(pattern);
        gitignore_content.push('\n');
    }

    let synthetic_path = map_root
        .join(".adocs")
        .join(".hashes")
        .join(".watch_gitignore");
    if let Some(parent) = synthetic_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(synthetic_path.as_std_path(), &gitignore_content)?;

    let (gitignore, err) = Gitignore::new(synthetic_path.as_std_path());
    if let Some(err) = err {
        eprintln!("Warning: error reading .agentwatch patterns: {}", err);
    }

    Ok(Some(gitignore))
}
