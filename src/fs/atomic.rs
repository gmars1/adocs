use camino::Utf8PathBuf;
use std::io::Write;

pub fn write_atomic(path: &Utf8PathBuf, content: &str) -> Result<(), crate::error::AdocsError> {
    let tmp_path = Utf8PathBuf::from(format!("{}.tmp", path));
    if let Some(parent) = tmp_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(tmp_path.as_std_path())?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    std::fs::rename(tmp_path.as_std_path(), path.as_std_path())?;
    Ok(())
}
