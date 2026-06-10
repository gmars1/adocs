use camino::Utf8PathBuf;

pub fn file_description_path(source: &str) -> Utf8PathBuf {
    Utf8PathBuf::from(format!(".adocs/agents/{}.md", source))
}

pub fn folder_purpose_path(source_folder: &str) -> Utf8PathBuf {
    let folder = if source_folder.ends_with('/') {
        source_folder.to_string()
    } else {
        format!("{}/", source_folder)
    };
    Utf8PathBuf::from(format!(".adocs/agents/{}folder_purpose.md", folder))
}
