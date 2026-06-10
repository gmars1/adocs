use clap::Parser;

use adocs::cli::{Cli, Command, KindFilter, StateFilter};
use adocs::model::config::resolve_roots;
use adocs::model::TrustState;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let roots = resolve_roots(
        cli.source_root.clone(),
        cli.map_root.clone(),
        cli.config.clone(),
    )?;

    match cli.command {
        Command::Init { force } => {
            adocs::commands::init::run_init(&roots, force)?;
        }
        Command::Sync => {
            let request = adocs::SyncRequest { roots: roots.clone() };
            let report = adocs::sync(&request)?;
            adocs::output::human::print_sync(&report);
        }
        Command::Status {
            json,
            fail_on_stale,
            fail_on_missing_docs,
            fail_on_ambiguous,
        } => {
            let request = adocs::StatusRequest {
                json,
                roots: roots.clone(),
                fail_on_stale,
                fail_on_missing_docs,
                fail_on_ambiguous,
            };
            let report = adocs::status(&request)?;
            if json {
                adocs::output::json::print_status_json(&report);
            } else {
                adocs::output::human::print_status(&report);
            }
            let mut exit_code = 0;
            if fail_on_stale && report.files.iter().any(|f| f.state == "stale") {
                exit_code = 2;
            }
            if fail_on_missing_docs && report.folders.iter().any(|f| !f.purpose_doc_exists) {
                exit_code = 2;
            }
            if fail_on_ambiguous && !report.ambiguous.is_empty() {
                exit_code = 2;
            }
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Command::Changed { json } => {
            let request = adocs::ChangedRequest { json, roots: roots.clone() };
            let report = adocs::changed(&request)?;
            if json {
                adocs::output::json::print_changed_json(&report);
            } else {
                adocs::output::human::print_changed(&report);
            }
        }
        Command::List { state, kind, json } => {
            let request = adocs::ListStateRequest {
                state: state_to_trust_state(state),
                kind: Some(kind_to_str(kind)),
                json,
                roots: roots.clone(),
            };
            let report = adocs::list_state(&request)?;
            if json {
                adocs::output::json::print_list_json(&report);
            } else {
                adocs::output::human::print_list(&report);
            }
        }
        Command::Stale { json } => {
            let request = adocs::ListStateRequest {
                state: Some(TrustState::Stale),
                kind: Some("all".to_string()),
                json,
                roots: roots.clone(),
            };
            let report = adocs::list_state(&request)?;
            if json {
                adocs::output::json::print_list_json(&report);
            } else {
                adocs::output::human::print_list_stale(&report);
            }
        }
        Command::Valid { json } => {
            let request = adocs::ListStateRequest {
                state: Some(TrustState::Valid),
                kind: Some("all".to_string()),
                json,
                roots: roots.clone(),
            };
            let report = adocs::list_state(&request)?;
            if json {
                adocs::output::json::print_list_json(&report);
            } else {
                adocs::output::human::print_list_valid(&report);
            }
        }
        Command::Context { path } => {
            adocs::output::human::print_context(&path, &roots)?;
        }
        Command::Update { path } => {
            let request = adocs::UpdateDocRequest { path, roots: roots.clone() };
            let report = adocs::update_doc(&request)?;
            adocs::output::human::print_update(&report);
        }
        Command::Seal { path } => {
            let request = adocs::SealRequest { path, roots: roots.clone() };
            let report = adocs::seal(&request)?;
            adocs::output::human::print_seal(&report);
        }
        Command::Rebind { file_id, new_path } => {
            let fid = adocs::model::FileId(file_id);
            adocs::rebind(&fid, &new_path, &roots)?;
        }
        Command::Serve { mcp: _ } => {
            adocs::mcp::server::run_mcp_server(roots).await
                .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;
        }
        Command::InstallAgent { agent } => {
            adocs::commands::install_agent::run_install_agent(&roots, &agent)?;
        }
        Command::DocsUnder {
            path,
            foldersonly,
            filesonly,
            json,
        } => {
            let request = adocs::DocsUnderRequest {
                path,
                folders_only: foldersonly,
                files_only: filesonly,
                json,
                roots: roots.clone(),
            };
            let report = adocs::docs_under(&request)?;
            if json {
                adocs::output::json::print_docs_under_json(&report);
            } else {
                adocs::output::human::print_docs_under(&report);
            }
        }
    }

    Ok(())
}

fn state_to_trust_state(state: StateFilter) -> Option<TrustState> {
    match state {
        StateFilter::Stale => Some(TrustState::Stale),
        StateFilter::Valid => Some(TrustState::Valid),
        StateFilter::Sealed => Some(TrustState::Sealed),
        StateFilter::All => None,
    }
}

fn kind_to_str(kind: KindFilter) -> String {
    match kind {
        KindFilter::Files => "files".to_string(),
        KindFilter::Folders => "folders".to_string(),
        KindFilter::All => "all".to_string(),
    }
}
