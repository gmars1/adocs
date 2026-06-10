use camino::Utf8PathBuf;
use crate::fs::hash;
use crate::model::{file_state, folder_purpose_state, FilesLedger, FoldersLedger, ResolvedRoots};
use crate::{
    ChangedReport, DocsUnderReport, ListStateReport, SealReport, StatusReport, SyncReport,
    UpdateDocReport,
};

pub fn print_status(report: &StatusReport) {
    if !report.changed.is_empty() {
        println!("changed files:");
        for entry in &report.changed {
            match entry.from.as_ref() {
                Some(from) => println!("  {} {} -> {}", entry.change, from, entry.path),
                None => println!("  {} {}", entry.change, entry.path),
            }
        }
        println!();
    }

    println!("states:");
    for file in &report.files {
        println!("  {} {}", file.state, file.path);
    }
    for folder in &report.folders {
        println!("  {} {} (folder)", folder.state, folder.path);
    }
    println!();

    let stale_docs: Vec<_> = report.files.iter().filter(|f| f.state == "stale").collect();
    if !stale_docs.is_empty() {
        println!("stale docs:");
        for file in &stale_docs {
            println!("  {}", crate::model::paths::file_description_path(&file.path));
        }
        println!();
    }

    let missing_purpose: Vec<_> = report.folders.iter().filter(|f| !f.purpose_doc_exists).collect();
    if !missing_purpose.is_empty() {
        println!("missing docs:");
        for folder in &missing_purpose {
            println!("  {}", crate::model::paths::folder_purpose_path(&folder.path));
        }
        println!();
    }

    if !report.ambiguous.is_empty() {
        println!("ambiguous:");
        for a in &report.ambiguous {
            println!("  {}: {}", a.reason, a.paths.join(", "));
        }
        println!();
    }

    if report.verification.required {
        if let Some(ref policy) = report.verification.policy {
            println!("verification policy: {}", policy);
        } else {
            println!("verification: external tool required before seal");
        }
    }
}

pub fn print_changed(report: &ChangedReport) {
    for entry in &report.changed {
        match entry.from.as_ref() {
            Some(from) => println!("{} {} -> {}", entry.change, from, entry.path),
            None => println!("{} {}", entry.change, entry.path),
        }
    }
}

pub fn print_list(report: &ListStateReport) {
    if !report.files.is_empty() {
        println!("{} files:", report.state);
        for file in &report.files {
            println!("  {}", crate::model::paths::file_description_path(&file.path));
        }
    }
    if !report.folders.is_empty() {
        if !report.files.is_empty() {
            println!();
        }
        println!("{} folders:", report.state);
        for folder in &report.folders {
            println!("  {}", crate::model::paths::folder_purpose_path(&folder.path));
        }
    }
}

pub fn print_list_stale(report: &ListStateReport) {
    if !report.files.is_empty() {
        println!("stale files:");
        for file in &report.files {
            println!("  {}", crate::model::paths::file_description_path(&file.path));
        }
    }
    if !report.folders.is_empty() {
        if !report.files.is_empty() {
            println!();
        }
        println!("stale folders:");
        for folder in &report.folders {
            println!("  {}", crate::model::paths::folder_purpose_path(&folder.path));
        }
    }
}

pub fn print_list_valid(report: &ListStateReport) {
    if !report.files.is_empty() {
        println!("valid files:");
        for file in &report.files {
            println!("  {}", crate::model::paths::file_description_path(&file.path));
        }
    }
    if !report.folders.is_empty() {
        if !report.files.is_empty() {
            println!();
        }
        println!("valid folders:");
        for folder in &report.folders {
            println!("  {}", crate::model::paths::folder_purpose_path(&folder.path));
        }
    }
}

pub fn print_context(path: &camino::Utf8PathBuf, roots: &ResolvedRoots) -> Result<(), crate::AdocsError> {
    let desc_path = crate::model::paths::file_description_path(path.as_str());
    let desc_abs = roots.map_root.join(&desc_path);

    println!("path: {}", path);
    println!();

    let hashes_dir = roots.map_root.join(".adocs").join(".hashes");

    match std::fs::read_to_string(desc_abs.as_std_path()) {
        Ok(content) => {
            let state = FilesLedger::load(&hashes_dir.join("files.json")).ok().and_then(|ledger| {
                ledger.observed_path_index.get(path).and_then(|fid| {
                    ledger.files.get(fid).map(|rec| {
                        let source_abs = roots.source_root.join(path);
                        let ch = hash::hash_file(source_abs.as_std_path()).unwrap_or_default();
                        let de = roots.map_root.join(&rec.description_path).exists();
                        file_state(&ch, rec.doc.as_ref(), rec.seal.as_ref(), de)
                    })
                })
            }).unwrap_or(crate::model::TrustState::Stale);
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
            let state = FoldersLedger::load(&hashes_dir.join("docs.json")).ok().and_then(|ledger| {
                ledger.folders.get(&Utf8PathBuf::from(parent.as_str())).map(|rec| {
                    let purpose_hash = hash::hash_file(purp_abs.as_std_path()).ok();
                    folder_purpose_state(
                        true,
                        rec.doc.as_ref(),
                        purpose_hash.as_deref(),
                        rec.seal.as_ref(),
                    )
                })
            }).unwrap_or(crate::model::TrustState::Stale);
            println!("folder purpose ({}):", purp_path);
            println!("trust state: {}", state);
            println!("{}", content);
        }
    }

    Ok(())
}

pub fn print_update(report: &UpdateDocReport) {
    println!("{} is now {}", report.path, report.state);
}

pub fn print_sync(report: &SyncReport) {
    println!(
        "Synced: {} templates created, {} docs moved, {} docs deleted, {} ambiguous skipped",
        report.templates_created, report.docs_moved, report.docs_deleted, report.ambiguous_skipped,
    );
}

pub fn print_seal(report: &SealReport) {
    println!("{} is now {}", report.path, report.state);
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
            println!("  {} ({})", entry.path, entry.trust_state.as_deref().unwrap_or("stale"));
        }
    }

    if !files.is_empty() {
        if !folders.is_empty() {
            println!();
        }
        println!("file descriptions:");
        for entry in &files {
            println!("  {} ({})", entry.path, entry.trust_state.as_deref().unwrap_or("stale"));
        }
    }
}
