use crate::fs::hash;
use crate::model::{file_state, folder_purpose_state, FilesLedger, FoldersLedger, ResolvedRoots};
use crate::{
    ChangedReport, DocsUnderReport, ListStateReport, SealReport, StatusReport, SyncReport,
    UpdateDocReport,
};
use camino::Utf8PathBuf;

fn use_color() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stdout())
}

fn green(s: &str) -> String {
    if use_color() {
        format!("\u{001b}[32m{}\u{001b}[0m", s)
    } else {
        s.to_string()
    }
}
fn yellow(s: &str) -> String {
    if use_color() {
        format!("\u{001b}[33m{}\u{001b}[0m", s)
    } else {
        s.to_string()
    }
}
fn cyan(s: &str) -> String {
    if use_color() {
        format!("\u{001b}[36m{}\u{001b}[0m", s)
    } else {
        s.to_string()
    }
}
fn red(s: &str) -> String {
    if use_color() {
        format!("\u{001b}[31m{}\u{001b}[0m", s)
    } else {
        s.to_string()
    }
}

pub fn print_status(report: &StatusReport) {
    let file_docs_to_update: Vec<_> = report
        .files
        .iter()
        .filter(|f| f.state == "stale" && f.description_doc_exists)
        .collect();
    let missing_descriptions: Vec<_> = report
        .files
        .iter()
        .filter(|f| !f.description_doc_exists)
        .collect();
    let current_files = report
        .files
        .iter()
        .filter(|f| f.state == "valid" || f.state == "sealed")
        .count();
    let folder_docs_to_update: Vec<_> = report
        .folders
        .iter()
        .filter(|f| f.state == "stale" && f.purpose_doc_exists)
        .collect();
    let missing_purpose: Vec<_> = report
        .folders
        .iter()
        .filter(|f| !f.purpose_doc_exists)
        .collect();
    let current_folders = report
        .folders
        .iter()
        .filter(|f| f.state == "valid" || f.state == "sealed")
        .count();

    if !report.changed.is_empty() {
        println!("  Source changes ({})", report.changed.len());
        for entry in &report.changed {
            let icon = match entry.change.as_str() {
                "added" => green("+"),
                "modified" => yellow("~"),
                "deleted" => red("-"),
                "moved" => cyan("\u{2192}"),
                "renamed" => cyan("\u{21c4}"),
                _ => "?".to_string(),
            };
            match entry.from.as_ref() {
                Some(from) => println!(
                    "  {} {:8}  {} \u{2192} {}",
                    icon, entry.change, from, entry.path
                ),
                None => println!("  {} {:8}  {}", icon, entry.change, entry.path),
            }
        }
        println!();
    }

    if !file_docs_to_update.is_empty() {
        println!(
            "  {} ({})",
            yellow("File docs to update"),
            file_docs_to_update.len()
        );
        for file in &file_docs_to_update {
            println!(
                "  {}",
                crate::model::paths::file_description_path(&file.path)
            );
        }
        println!();
    }

    if !missing_descriptions.is_empty() {
        println!(
            "  {} ({})",
            yellow("File docs to create"),
            missing_descriptions.len()
        );
        for file in &missing_descriptions {
            println!(
                "  {}",
                crate::model::paths::file_description_path(&file.path)
            );
        }
        println!();
    }

    if !folder_docs_to_update.is_empty() {
        println!(
            "  {} ({})",
            yellow("Folder docs to update"),
            folder_docs_to_update.len()
        );
        for folder in &folder_docs_to_update {
            println!(
                "  {}",
                crate::model::paths::folder_purpose_path(&folder.path)
            );
        }
        println!();
    }

    if !missing_purpose.is_empty() {
        println!(
            "  {} ({})",
            yellow("Folder docs to create"),
            missing_purpose.len()
        );
        for folder in &missing_purpose {
            println!(
                "  {}",
                crate::model::paths::folder_purpose_path(&folder.path)
            );
        }
        println!();
    }

    if !report.ambiguous.is_empty() {
        println!("  Ambiguous");
        for a in &report.ambiguous {
            println!("  {}: {}", a.reason, a.paths.join(", "));
        }
        println!();
    }

    let mut parts = Vec::new();
    parts.push(format!(
        "{} files ({} current, {} need update, {} missing)",
        report.files.len(),
        current_files,
        file_docs_to_update.len(),
        missing_descriptions.len(),
    ));
    parts.push(format!(
        "{} folders ({} current, {} need update, {} missing)",
        report.folders.len(),
        current_folders,
        folder_docs_to_update.len(),
        missing_purpose.len(),
    ));
    if !report.changed.is_empty() {
        parts.push(format!("{} source changes", report.changed.len()));
    }
    if !report.ambiguous.is_empty() {
        parts.push(format!("{} ambiguous", report.ambiguous.len()));
    }

    println!("  \u{2500}\u{2500}  {}", parts.join("  \u{00b7}  "));
}

pub fn print_changed(report: &ChangedReport) {
    if report.changed.is_empty() {
        println!("  (nothing changed)");
        return;
    }
    for entry in &report.changed {
        let icon = match entry.change.as_str() {
            "added" => "+",
            "modified" => "~",
            "deleted" => "-",
            "moved" => "\u{2192}",
            "renamed" => "\u{21c4}",
            _ => "?",
        };
        match entry.from.as_ref() {
            Some(from) => println!(
                "{} {:8}  {} \u{2192} {}",
                icon, entry.change, from, entry.path
            ),
            None => println!("{} {:8}  {}", icon, entry.change, entry.path),
        }
    }
}

pub fn print_list(report: &ListStateReport) {
    if report.files.is_empty() && report.folders.is_empty() {
        println!("  (none)");
        return;
    }
    if !report.files.is_empty() {
        println!("  {} files:", report.state);
        for file in &report.files {
            println!(
                "  {}",
                crate::model::paths::file_description_path(&file.path)
            );
        }
    }
    if !report.folders.is_empty() {
        if !report.files.is_empty() {
            println!();
        }
        println!("  {} folders:", report.state);
        for folder in &report.folders {
            println!(
                "  {}",
                crate::model::paths::folder_purpose_path(&folder.path)
            );
        }
    }
}

pub fn print_list_stale(report: &ListStateReport) {
    print_list(report);
}

pub fn print_list_valid(report: &ListStateReport) {
    print_list(report);
}

pub fn print_context(
    path: &camino::Utf8PathBuf,
    roots: &ResolvedRoots,
) -> Result<(), crate::AdocsError> {
    let desc_path = crate::model::paths::file_description_path(path.as_str());
    let desc_abs = roots.map_root.join(&desc_path);

    println!("path: {}", path);
    println!();

    let hashes_dir = roots.map_root.join(".adocs").join(".hashes");

    match std::fs::read_to_string(desc_abs.as_std_path()) {
        Ok(content) => {
            let state = FilesLedger::load(&hashes_dir.join("files.json"))
                .ok()
                .and_then(|ledger| {
                    ledger.observed_path_index.get(path).and_then(|fid| {
                        ledger.files.get(fid).map(|rec| {
                            let source_abs = roots.source_root.join(path);
                            let ch = hash::hash_file(source_abs.as_std_path()).unwrap_or_default();
                            let de = roots.map_root.join(&rec.description_path).exists();
                            file_state(&ch, rec.doc.as_ref(), rec.seal.as_ref(), de)
                        })
                    })
                })
                .unwrap_or(crate::model::TrustState::Stale);
            println!("file description ({}):", desc_path);
            println!("trust state: {}", state);
            println!("{}", content);
        }
        Err(_) => {
            println!("  (no file description)");
        }
    }

    if let Some(parent) = camino::Utf8PathBuf::from(path.as_str()).parent() {
        let purp_path = crate::model::paths::folder_purpose_path(parent.as_str());
        let purp_abs = roots.map_root.join(&purp_path);
        if let Ok(content) = std::fs::read_to_string(purp_abs.as_std_path()) {
            let state = FoldersLedger::load(&hashes_dir.join("docs.json"))
                .ok()
                .and_then(|ledger| {
                    ledger
                        .folders
                        .get(&Utf8PathBuf::from(parent.as_str()))
                        .map(|rec| {
                            let purpose_hash = hash::hash_file(purp_abs.as_std_path()).ok();
                            folder_purpose_state(
                                true,
                                rec.doc.as_ref(),
                                purpose_hash.as_deref(),
                                rec.seal.as_ref(),
                            )
                        })
                })
                .unwrap_or(crate::model::TrustState::Stale);
            println!("folder purpose ({}):", purp_path);
            println!("trust state: {}", state);
            println!("{}", content);
        }
    }

    Ok(())
}

pub fn print_update(report: &UpdateDocReport) {
    println!(
        "  {} \u{2192} {}",
        report.path,
        green(&report.state.to_string())
    );
}

pub fn print_seal(report: &SealReport) {
    println!(
        "  {} \u{2192} {}",
        report.path,
        cyan(&report.state.to_string())
    );
}

pub fn print_sync(report: &SyncReport) {
    let mut parts = Vec::new();
    if report.templates_created > 0 {
        parts.push(green(&format!("+{} created", report.templates_created)));
    }
    if report.docs_moved > 0 {
        parts.push(cyan(&format!("\u{2192}{} moved", report.docs_moved)));
    }
    if report.docs_deleted > 0 {
        parts.push(red(&format!("-{} deleted", report.docs_deleted)));
    }
    if report.ambiguous_skipped > 0 {
        parts.push(format!("{} skipped (ambiguous)", report.ambiguous_skipped));
    }
    if parts.is_empty() {
        println!("  (up to date)");
    } else {
        println!("  {}", parts.join("  "));
    }
}

pub fn print_docs_under(report: &DocsUnderReport) {
    if report.docs.is_empty() {
        println!("No valid docs under {}", report.folder);
        return;
    }

    let files: Vec<_> = report.docs.iter().filter(|d| d.kind == "file").collect();
    let folders: Vec<_> = report.docs.iter().filter(|d| d.kind == "folder").collect();

    if !folders.is_empty() {
        println!("folder purposes:");
        for entry in &folders {
            println!(
                "  {} ({})",
                entry.path,
                entry.trust_state.as_deref().unwrap_or("stale")
            );
        }
    }

    if !files.is_empty() {
        if !folders.is_empty() {
            println!();
        }
        println!("file descriptions:");
        for entry in &files {
            println!(
                "  {} ({})",
                entry.path,
                entry.trust_state.as_deref().unwrap_or("stale")
            );
        }
    }
}
