use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

struct TestProject {
    dir: tempfile::TempDir,
}

impl TestProject {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("src").join("main.rs"),
            "fn main() { println!(\"hello\"); }\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
        )
        .unwrap();
        Self { dir }
    }

    fn adocs(&self) -> Command {
        let mut cmd = Command::cargo_bin("adocs").unwrap();
        cmd.arg("--source-root").arg(self.dir.path());
        cmd.arg("--map-root").arg(self.dir.path());
        cmd
    }

    fn adocs_from_parent(&self) -> Command {
        let mut cmd = Command::cargo_bin("adocs").unwrap();
        let parent = self.dir.path().parent().unwrap();
        let relative_project = self.dir.path().file_name().unwrap();
        cmd.current_dir(parent);
        cmd.arg("--source-root").arg(relative_project);
        cmd.arg("--map-root").arg(relative_project);
        cmd
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }
}

#[test]
fn test_init_creates_structure() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();

    assert!(p.path().join(".adocs").exists());
    assert!(p.path().join(".adocs").join(".hashes").exists());
    assert!(p.path().join(".adocs").join("agents").exists());
    assert!(p.path().join(".adocs").join(".agenignore").exists());
}

#[test]
fn test_init_rejects_existing_without_force() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("init").assert().failure();
}

#[test]
fn test_init_force_overwrites() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("init").arg("--force").assert().success();
}

#[test]
fn test_sync_creates_templates() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs()
        .arg("sync")
        .assert()
        .success()
        .stdout(predicate::str::contains("created"));
}

#[test]
fn test_agentwatch_tracks_directory_contents() {
    let p = TestProject::new();
    fs::create_dir_all(p.path().join("tests")).unwrap();
    fs::write(
        p.path().join("tests").join("smoke.rs"),
        "#[test]\nfn smoke() {}\n",
    )
    .unwrap();

    p.adocs().arg("init").assert().success();
    fs::write(p.path().join(".adocs").join(".agentwatch"), "tests/\n").unwrap();

    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains(".adocs/agents/tests/smoke.rs.md"))
        .stdout(predicate::str::contains(".adocs/agents/src/main.rs.md").not());
}

#[test]
fn test_agentwatch_root_glob_tracks_root_files_only() {
    let p = TestProject::new();
    fs::write(p.path().join("root.py"), "print('root')\n").unwrap();
    fs::create_dir_all(p.path().join("pkg").join("deep")).unwrap();
    fs::write(
        p.path().join("pkg").join("deep").join("worker.py"),
        "print('ok')\n",
    )
    .unwrap();
    fs::write(
        p.path().join("pkg").join("deep").join("notes.txt"),
        "ignore me\n",
    )
    .unwrap();

    p.adocs().arg("init").assert().success();
    fs::write(p.path().join(".adocs").join(".agentwatch"), "*.py\n").unwrap();

    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains(".adocs/agents/root.py.md"))
        .stdout(predicate::str::contains(".adocs/agents/pkg/deep/worker.py.md").not())
        .stdout(predicate::str::contains("notes.txt").not())
        .stdout(predicate::str::contains(".adocs/agents/src/main.rs.md").not());
}

#[test]
fn test_agentwatch_globstar_tracks_tree_files() {
    let p = TestProject::new();
    fs::write(p.path().join("src").join("app.py"), "print('src')\n").unwrap();
    fs::create_dir_all(p.path().join("src").join("pkg")).unwrap();
    fs::write(
        p.path().join("src").join("pkg").join("worker.py"),
        "print('deep src')\n",
    )
    .unwrap();
    fs::create_dir_all(p.path().join("tests").join("unit")).unwrap();
    fs::write(
        p.path().join("tests").join("unit").join("test_app.py"),
        "def test_app(): pass\n",
    )
    .unwrap();
    fs::write(p.path().join("tests").join("notes.txt"), "ignore me\n").unwrap();

    p.adocs_from_parent().arg("init").assert().success();
    fs::write(
        p.path().join(".adocs").join(".agentwatch"),
        "src/**/*.py\ntests/**/*.py\n",
    )
    .unwrap();

    p.adocs_from_parent().arg("sync").assert().success();
    p.adocs_from_parent()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains(".adocs/agents/src/app.py.md"))
        .stdout(predicate::str::contains(
            ".adocs/agents/src/pkg/worker.py.md",
        ))
        .stdout(predicate::str::contains(
            ".adocs/agents/tests/unit/test_app.py.md",
        ))
        .stdout(predicate::str::contains("notes.txt").not())
        .stdout(predicate::str::contains(".adocs/agents/src/main.rs.md").not());
}

#[test]
fn test_status_shows_stale_for_new_files() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("File docs to update"))
        .stdout(predicate::str::contains(".adocs/agents/src/main.rs.md"));
}

#[test]
fn test_status_splits_file_docs_to_update_and_create() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("update")
        .arg("src/main.rs")
        .assert()
        .success();

    fs::write(
        p.path().join("src").join("main.rs"),
        "fn main() { println!(\"changed\"); }\n",
    )
    .unwrap();
    fs::write(p.path().join("src").join("new.rs"), "pub fn new() {}\n").unwrap();

    let assert = p.adocs().arg("status").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    let update_heading = stdout.find("File docs to update").unwrap();
    let create_heading = stdout.find("File docs to create").unwrap();
    let main_doc = stdout.find(".adocs/agents/src/main.rs.md").unwrap();
    let new_doc = stdout.find(".adocs/agents/src/new.rs.md").unwrap();

    assert!(main_doc > update_heading);
    assert!(main_doc < create_heading);
    assert!(new_doc > create_heading);
}

#[test]
fn test_update_promotes_to_valid() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("update")
        .arg("src/main.rs")
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
    p.adocs()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "2 files (1 current, 1 need update, 0 missing)",
        ))
        .stdout(predicate::str::contains("src/main.rs").not());
}

#[test]
fn test_edit_makes_file_stale_again() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("update")
        .arg("src/main.rs")
        .assert()
        .success();
    fs::write(
        p.path().join("src").join("main.rs"),
        "fn main() { println!(\"modified\"); }\n",
    )
    .unwrap();
    p.adocs()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains(".adocs/agents/src/main.rs.md"));
}

#[test]
fn test_list_stale() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("list")
        .arg("--state")
        .arg("stale")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"));
}

#[test]
fn test_list_valid_after_update() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("update")
        .arg("src/main.rs")
        .assert()
        .success();
    p.adocs()
        .arg("list")
        .arg("--state")
        .arg("valid")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"));
}

#[test]
fn test_stale_alias() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("stale")
        .assert()
        .success()
        .stdout(predicate::str::contains("stale files"));
}

#[test]
fn test_seal_refuses_stale() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("seal")
        .arg("src/main.rs")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot seal stale file"));
}

#[test]
fn test_seal_works_after_update() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("update")
        .arg("src/main.rs")
        .assert()
        .success();
    p.adocs()
        .arg("seal")
        .arg("src/main.rs")
        .assert()
        .success()
        .stdout(predicate::str::contains("sealed"));
}

#[test]
fn test_list_sealed() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("update")
        .arg("src/main.rs")
        .assert()
        .success();
    p.adocs().arg("seal").arg("src/main.rs").assert().success();
    p.adocs()
        .arg("list")
        .arg("--state")
        .arg("sealed")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"));
}

#[test]
fn test_status_json() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("status")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"files\""));
}

#[test]
fn test_changed_reports_new_files() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    fs::write(p.path().join("src").join("newfile.rs"), "pub fn x() {}\n").unwrap();
    p.adocs()
        .arg("changed")
        .assert()
        .success()
        .stdout(predicate::str::contains("added"));
}

#[test]
fn test_rename_preserves_state() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("update")
        .arg("src/main.rs")
        .assert()
        .success();
    fs::rename(
        p.path().join("src").join("main.rs"),
        p.path().join("src").join("app.rs"),
    )
    .unwrap();
    p.adocs()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("renamed"));
}

#[test]
fn test_missing_folder_purpose_reported() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Folder docs to create"));
}

#[test]
fn test_install_agent_outputs_config() {
    let p = TestProject::new();
    p.adocs()
        .arg("install-agent")
        .arg("opencode")
        .assert()
        .success()
        .stdout(predicate::str::contains("opencode.json"));
}

#[test]
fn test_docsunder_shows_valid_files() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("update")
        .arg("src/main.rs")
        .assert()
        .success();
    p.adocs()
        .arg("docsunder")
        .arg("src")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn test_docsunder_filesonly_excludes_folders() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("update")
        .arg("src/main.rs")
        .assert()
        .success();
    p.adocs()
        .arg("docsunder")
        .arg("src")
        .arg("--filesonly")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"));
}

#[test]
fn test_docsunder_json_output() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("update")
        .arg("src/main.rs")
        .assert()
        .success();
    p.adocs()
        .arg("docsunder")
        .arg("src")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"folder\""))
        .stdout(predicate::str::contains("\"docs\""));
}

#[test]
fn test_docsunder_excludes_stale() {
    let p = TestProject::new();
    p.adocs().arg("init").assert().success();
    p.adocs().arg("sync").assert().success();
    p.adocs()
        .arg("docsunder")
        .arg("src")
        .assert()
        .success()
        .stdout(predicate::str::contains("No valid docs under src"));
}

#[cfg(test)]
mod mcp_tests {
    use super::*;
    use adocs::mcp::tools::AdocsMcpServer;
    use adocs::model::config::resolve_roots;
    use serde_json::json;

    fn setup(project: &TestProject) -> AdocsMcpServer {
        let path = project.path().to_string_lossy().to_string();
        let roots = resolve_roots(
            Some(camino::Utf8PathBuf::from(&path)),
            Some(camino::Utf8PathBuf::from(&path)),
            None,
        )
        .unwrap();
        AdocsMcpServer::new(roots)
    }

    fn call(
        server: &AdocsMcpServer,
        name: &str,
        args: serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(server.dispatch(name, args)).unwrap();
        let content = result.content.first().unwrap();
        let inner: &rmcp::model::RawContent = content;
        match inner {
            rmcp::model::RawContent::Text(t) => {
                serde_json::from_str(&t.text).unwrap_or(json!({ "raw": t.text.clone() }))
            }
            _ => json!({}),
        }
    }

    #[test]
    fn test_mcp_read_context_stale() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        let server = setup(&p);

        let result = call(
            &server,
            "adocs_read_context",
            serde_json::from_str(r#"{"path":"src/main.rs"}"#).unwrap(),
        );
        assert_eq!(result["path"], "src/main.rs");
        assert_eq!(result["trust_state"], "stale");
        assert!(result["missing_file_description"] != true);
        assert!(!result["file_description"].as_str().unwrap_or("").is_empty());
    }

    #[test]
    fn test_mcp_read_file_description() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        let server = setup(&p);

        let result = call(
            &server,
            "adocs_read_file_description",
            serde_json::from_str(r#"{"path":"src/main.rs"}"#).unwrap(),
        );
        let raw = result.get("raw").and_then(|v| v.as_str()).unwrap_or("");
        assert!(raw.contains("# src/main.rs"));
    }

    #[test]
    fn test_mcp_read_folder_purpose() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        let server = setup(&p);

        let result = call(
            &server,
            "adocs_read_folder_purpose",
            serde_json::from_str(r#"{"path":"src"}"#).unwrap(),
        );
        let raw = result.get("raw").and_then(|v| v.as_str()).unwrap_or("");
        assert!(raw.contains("# src"));
    }

    #[test]
    fn test_mcp_update_doc_promotes_to_valid() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        let server = setup(&p);

        let result = call(
            &server,
            "adocs_update_doc",
            serde_json::from_str(r#"{"path":"src/main.rs"}"#).unwrap(),
        );
        assert_eq!(result["state"], "valid");

        let ctx = call(
            &server,
            "adocs_read_context",
            serde_json::from_str(r#"{"path":"src/main.rs"}"#).unwrap(),
        );
        assert_eq!(ctx["trust_state"], "valid");
    }

    #[test]
    fn test_mcp_explain_staleness() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        let server = setup(&p);

        let result = call(
            &server,
            "adocs_explain_staleness",
            serde_json::from_str(r#"{"path":"src/main.rs"}"#).unwrap(),
        );
        assert!(result["stale_reasons"].as_array().unwrap().len() > 0);
        assert_ne!(result["stale_reasons"][0], "not stale");
    }

    #[test]
    fn test_mcp_status_has_files() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        let server = setup(&p);

        let result = call(&server, "adocs_status", serde_json::Map::new());
        let files = result["files"].as_array().unwrap();
        assert!(files.len() >= 2);
    }

    #[test]
    fn test_mcp_request_seal_asks_human() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        let server = setup(&p);

        let result = call(
            &server,
            "adocs_request_seal",
            serde_json::from_str(r#"{"path":"src/main.rs"}"#).unwrap(),
        );
        let raw = result.get("raw").and_then(|v| v.as_str()).unwrap_or("");
        assert!(raw.contains("adocs seal"));
    }

    #[test]
    fn test_mcp_list_state_stale() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        let server = setup(&p);

        let result = call(
            &server,
            "adocs_list_state",
            serde_json::from_str(r#"{"state":"stale","kind":"files"}"#).unwrap(),
        );
        let files = result["files"].as_array().unwrap();
        assert!(files.iter().any(|f| f["path"] == "src/main.rs"));
    }

    #[test]
    fn test_mcp_sync_idempotent() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        let server = setup(&p);

        let result = call(&server, "adocs_sync", serde_json::Map::new());
        assert_eq!(result["templates_created"], 0);
    }

    #[test]
    fn test_mcp_read_folder_docs() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        p.adocs()
            .arg("update")
            .arg("src/main.rs")
            .assert()
            .success();
        let server = setup(&p);

        let result = call(
            &server,
            "adocs_read_folder_docs",
            serde_json::from_str(r#"{"path":"src"}"#).unwrap(),
        );
        assert_eq!(result["folder"], "src");
        let docs = result["docs"].as_array().unwrap();
        assert!(docs
            .iter()
            .any(|d| d["path"] == "src/main.rs" && d["kind"] == "file"));
    }

    #[test]
    fn test_mcp_read_folder_docs_filesonly() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        p.adocs()
            .arg("update")
            .arg("src/main.rs")
            .assert()
            .success();
        let server = setup(&p);

        let result = call(
            &server,
            "adocs_read_folder_docs",
            serde_json::from_str(r#"{"path":"src","files_only":true}"#).unwrap(),
        );
        let docs = result["docs"].as_array().unwrap();
        assert!(docs.iter().all(|d| d["kind"] == "file"));
        assert!(docs.iter().any(|d| d["path"] == "src/main.rs"));
    }

    #[test]
    fn test_mcp_read_folder_docs_foldersonly() {
        let p = TestProject::new();
        p.adocs().arg("init").assert().success();
        p.adocs().arg("sync").assert().success();
        p.adocs()
            .arg("update")
            .arg("src/main.rs")
            .assert()
            .success();
        let server = setup(&p);

        let result = call(
            &server,
            "adocs_read_folder_docs",
            serde_json::from_str(r#"{"path":"src","folders_only":true}"#).unwrap(),
        );
        let docs = result["docs"].as_array().unwrap();
        assert!(docs.iter().all(|d| d["kind"] == "folder"));
    }
}
