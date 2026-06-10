use camino::Utf8PathBuf;
use ignore::gitignore::Gitignore;
use std::path::Path;

pub fn build_ignore_matcher(map_root: &Utf8PathBuf) -> Result<Option<Gitignore>, crate::error::AdocsError> {
    let ignore_file = map_root.join(".adocs").join(".agenignore");

    if !ignore_file.exists() {
        return Ok(None);
    }

    let (gitignore, err) = Gitignore::new(ignore_file.as_std_path());
    if let Some(err) = err {
        eprintln!("Warning: error reading .agenignore: {}", err);
    }
    Ok(Some(gitignore))
}

pub const BUILTIN_IGNORE_PATTERNS: &[&str] = &[
    ".git/",
    ".adocs/",
    "node_modules/",
    "target/",
    "dist/",
    "build/",
    "coverage/",
    "*.log",
];

pub fn write_default_agenignore(map_root: &Utf8PathBuf) -> Result<(), crate::error::AdocsError> {
    let path = map_root.join(".adocs").join(".agenignore");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = BUILTIN_IGNORE_PATTERNS.join("\n") + "\n";
    std::fs::write(path.as_std_path(), content)?;
    Ok(())
}

pub fn is_path_ignored(
    source_path: &Utf8PathBuf,
    map_root: &Utf8PathBuf,
) -> Result<bool, crate::error::AdocsError> {
    let matcher = build_ignore_matcher(map_root)?;
    let Some(m) = matcher else {
        return Ok(false);
    };
    let path = Path::new(source_path.as_str());
    let matched = m.matched(path, false);
    Ok(matched.is_ignore())
}
