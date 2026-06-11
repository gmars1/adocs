use std::collections::HashMap;
use std::sync::Arc;

use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value, Map};

use crate::fs::hash;
use crate::model::{file_state, FilesLedger, ResolvedRoots, TrustState};
use crate::{
    ChangedRequest, DocsUnderRequest, ListStateRequest, StatusRequest, SyncRequest,
    UpdateDocRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadContextParams { pub path: String }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListStateParams { pub state: String, #[serde(default)] pub kind: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateDocParams { pub path: String }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RequestSealParams { pub path: String }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExplainStalenessParams { pub path: String }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadFolderDocsParams {
    pub path: String,
    #[serde(default)]
    pub folders_only: bool,
    #[serde(default)]
    pub files_only: bool,
}

fn json_content(v: Value) -> Result<Content, McpError> {
    Content::json(v).map_err(|e| McpError::internal_error(e.to_string(), None))
}

pub struct AdocsMcpServer {
    pub roots: ResolvedRoots,
    tools: HashMap<&'static str, rmcp::model::Tool>,
}

impl AdocsMcpServer {
    pub fn new(roots: ResolvedRoots) -> Self {
        let mut tools = HashMap::new();
        macro_rules! rt {
            ($n:expr, $d:expr, $t:ty) => {{
                let s = schemars::schema_for!($t);
                let v: Value = serde_json::to_value(&s).unwrap_or_default();
                let props = v.get("properties").and_then(|p| p.as_object()).cloned().unwrap_or_default();
                let required = v.get("required").and_then(|r| r.as_array()).cloned().unwrap_or_default();
                let mut schema_obj = Map::new();
                schema_obj.insert("type".into(), json!("object"));
                schema_obj.insert("properties".into(), Value::Object(props));
                if !required.is_empty() { schema_obj.insert("required".into(), Value::Array(required)); }
                tools.insert($n, rmcp::model::Tool::new($n, $d, Arc::new(schema_obj)));
            }};
        }
        rt!("adocs_status", "Workspace health: trust states, changed files, stale docs, missing docs, verification policy, ambiguous identities", ());
        rt!("adocs_changed", "List added, modified, deleted, moved, renamed, and ambiguous source files", ());
        rt!("adocs_sync", "Materialize missing templates, move docs for unambiguous same-hash moves, delete docs for removed files", ());
        rt!("adocs_list_state", "List files or folder purpose docs filtered by trust state: stale, valid, sealed, or all", ListStateParams);
        rt!("adocs_read_context", "Return folder purpose + file description + trust state + seal evidence + warnings for a path", ReadContextParams);
        rt!("adocs_read_file_description", "Read one file_description.md for a source path with its trust state", ReadContextParams);
        rt!("adocs_read_folder_purpose", "Read one folder_purpose.md for a source folder with its trust state", ReadContextParams);
        rt!("adocs_explain_staleness", "Explain why a path is stale: missing doc, source hash changed, doc hash changed, ambiguity, etc.", ExplainStalenessParams);
        rt!("adocs_update_doc", "Accept a file_description.md only after you have updated it to match the current source content (stale -> valid)", UpdateDocParams);
        rt!("adocs_request_seal", "Request human seal review for a valid path. Agents must not seal directly.", RequestSealParams);
        rt!("adocs_read_folder_docs", "Get every valid doc under a folder. Use this when you need the full set and can handle a larger response; prefer the smallest folder that answers the question. Use folders_only or files_only to filter.", ReadFolderDocsParams);
        Self { roots, tools }
    }

    fn status_json(&self) -> Result<Value, String> {
        crate::status(&StatusRequest { json: true, roots: self.roots.clone(), fail_on_stale: false, fail_on_missing_docs: false, fail_on_ambiguous: false })
            .map(|r| serde_json::to_value(r).unwrap_or_default()).map_err(|e| e.to_string())
    }

    fn changed_json(&self) -> Result<Value, String> {
        crate::changed(&ChangedRequest { json: true, roots: self.roots.clone() })
            .map(|r| serde_json::to_value(r).unwrap_or_default()).map_err(|e| e.to_string())
    }

    pub async fn dispatch(&self, name: &str, args: Map<String, Value>) -> Result<CallToolResult, McpError> {
        let a = Value::Object(args);
        match name {
            "adocs_status" => Ok(CallToolResult::success(vec![json_content(self.status_json().map_err(|e| McpError::internal_error(e, None))?)?])),
            "adocs_changed" => Ok(CallToolResult::success(vec![json_content(self.changed_json().map_err(|e| McpError::internal_error(e, None))?)?])),
            "adocs_sync" => {
                let r = crate::sync(&SyncRequest{roots:self.roots.clone()}).map_err(|e|McpError::internal_error(e.to_string(),None))?;
                Ok(CallToolResult::success(vec![json_content(serde_json::to_value(r).unwrap_or_default())?]))
            }
            "adocs_list_state" => {
                let p: ListStateParams = serde_json::from_value(a).map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                let s = match p.state.as_str() { "stale"=>Some(TrustState::Stale), "valid"=>Some(TrustState::Valid), "sealed"=>Some(TrustState::Sealed), "all"=>None, _=>return Err(McpError::invalid_params("state must be stale/valid/sealed/all",None)) };
                let r = crate::list_state(&ListStateRequest{state:s,kind:p.kind,json:true,roots:self.roots.clone()}).map_err(|e|McpError::internal_error(e.to_string(),None))?;
                Ok(CallToolResult::success(vec![json_content(serde_json::to_value(r).unwrap_or_default())?]))
            }
            "adocs_read_context" => {
                let p: ReadContextParams = serde_json::from_value(a).map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                Ok(self.read_context(&p.path))
            }
            "adocs_read_file_description" => {
                let p: ReadContextParams = serde_json::from_value(a).map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                let d = crate::model::paths::file_description_path(&p.path);
                match std::fs::read_to_string(self.roots.map_root.join(&d)) { Ok(c)=>Ok(CallToolResult::success(vec![Content::text(c)])), Err(_)=>Err(McpError::invalid_params(format!("no description for {}",p.path),None)) }
            }
            "adocs_read_folder_purpose" => {
                let p: ReadContextParams = serde_json::from_value(a).map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                let d = crate::model::paths::folder_purpose_path(&p.path);
                match std::fs::read_to_string(self.roots.map_root.join(&d)) { Ok(c)=>Ok(CallToolResult::success(vec![Content::text(c)])), Err(_)=>Err(McpError::invalid_params(format!("no purpose for {}",p.path),None)) }
            }
            "adocs_explain_staleness" => {
                let p: ExplainStalenessParams = serde_json::from_value(a).map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                Ok(self.explain(&p.path))
            }
            "adocs_update_doc" => {
                let p: UpdateDocParams = serde_json::from_value(a).map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                let r = crate::update_doc(&UpdateDocRequest{path:camino::Utf8PathBuf::from(&p.path),roots:self.roots.clone()}).map_err(|e|McpError::internal_error(e.to_string(),None))?;
                Ok(CallToolResult::success(vec![json_content(json!({"path":r.path,"state":r.state.to_string()}))?]))
            }
            "adocs_request_seal" => {
                let p: RequestSealParams = serde_json::from_value(a).map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                let msg = format!("Human review requested: run `adocs seal {}` after external verification", p.path);
                Ok(CallToolResult::success(vec![Content::text(msg)]))
            }
            "adocs_read_folder_docs" => {
                let p: ReadFolderDocsParams = serde_json::from_value(a).map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                let r = crate::docs_under(&DocsUnderRequest {
                    path: camino::Utf8PathBuf::from(&p.path),
                    folders_only: p.folders_only,
                    files_only: p.files_only,
                    json: true,
                    roots: self.roots.clone(),
                }).map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![json_content(serde_json::to_value(r).unwrap_or_default())?]))
            }
            _ => Err(McpError::method_not_found::<rmcp::model::CallToolRequestMethod>()),
        }
    }

    fn read_context(&self, path: &str) -> CallToolResult {
        let mut r = Map::new();
        r.insert("path".into(), json!(path));
        let desc = self.roots.map_root.join(&crate::model::paths::file_description_path(path));
        if desc.exists() { if let Ok(c) = std::fs::read_to_string(&desc) { r.insert("file_description".into(), json!(c)); } }
        else { r.insert("file_description".into(), json!(null)); r.insert("missing_file_description".into(), json!(true)); }
        let src = self.roots.source_root.join(path);
        if src.exists() { if let Ok(h) = hash::hash_file(src.as_std_path()) { r.insert("content_sha256".into(), json!(h)); } }
        if let Some(parent) = camino::Utf8PathBuf::from(path).parent() {
            let f = parent.to_string();
            let purp = self.roots.map_root.join(&crate::model::paths::folder_purpose_path(&f));
            if purp.exists() { if let Ok(c) = std::fs::read_to_string(&purp) { r.insert("folder_purpose".into(), json!(c)); } }
            else { r.insert("missing_folder_purpose".into(), json!(true)); }
            r.insert("folder".into(), json!(f));
        }
        if let Ok(ledger) = FilesLedger::load(&self.roots.map_root.join(".adocs").join(".hashes").join("files.json")) {
            if let Some(fid) = ledger.observed_path_index.get(&camino::Utf8PathBuf::from(path)) {
                if let Some(rec) = ledger.files.get(fid) {
                    let ch = hash::hash_file(self.roots.source_root.join(path).as_std_path()).unwrap_or_default();
                    let de = self.roots.map_root.join(&rec.description_path).exists();
                    r.insert("trust_state".into(), json!(file_state(&ch, rec.doc.as_ref(), rec.seal.as_ref(), de).to_string()));
                    if let Some(s) = &rec.seal {
                        r.insert("sealed_at".into(), json!(s.sealed_at.to_rfc3339()));
                    }
                }
            }
        }
        CallToolResult::success(vec![Content::json(Value::Object(r)).expect("valid json")])
    }

    fn explain(&self, path: &str) -> CallToolResult {
        let mut reasons = Vec::new();
        let desc = self.roots.map_root.join(&crate::model::paths::file_description_path(path));
        if !desc.exists() { reasons.push("missing file description".into()); }
        let src = self.roots.source_root.join(path);
        let ch = if src.exists() { hash::hash_file(src.as_std_path()).unwrap_or_default() } else { reasons.push("source file deleted".into()); String::new() };
        if let Ok(ledger) = FilesLedger::load(&self.roots.map_root.join(".adocs").join(".hashes").join("files.json")) {
            if let Some(fid) = ledger.observed_path_index.get(&camino::Utf8PathBuf::from(path)) {
                if let Some(rec) = ledger.files.get(fid) {
                    if let Some(doc) = &rec.doc {
                        if doc.accepted_source_sha256 != ch { reasons.push(format!("content changed (accepted:{} current:{})", &doc.accepted_source_sha256[..16.min(doc.accepted_source_sha256.len())], &ch[..16.min(ch.len())])); }
                    } else { reasons.push("no accepted doc evidence".into()); }
                }
            }
        }
        if reasons.is_empty() { reasons.push("not stale".into()); }
        CallToolResult::success(vec![Content::json(json!({"path":path,"stale_reasons":reasons})).expect("valid json")])
    }
}

impl ServerHandler for AdocsMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: Default::default(),
            server_info: Implementation {
                name: "adocs".into(),
                title: None,
                version: env!("CARGO_PKG_VERSION").into(),
                description: Some("Local-first trust map for code repositories".into()),
                icons: None,
                website_url: None,
            },
            instructions: Some("Use adocs_* tools before broad filesystem exploration. Check adocs_status first for workspace health, then use adocs_read_context for specific paths. Use adocs_read_folder_docs only when you need all docs for a folder and can handle a larger response. Use adocs_update_doc only after the file description has been updated to match current source.".into()),
        }
    }

    fn list_tools(&self, _: Option<rmcp::model::PaginatedRequestParams>, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        let tools: Vec<_> = self.tools.values().cloned().collect();
        std::future::ready(Ok(ListToolsResult { tools, next_cursor: None, meta: None }))
    }

    fn call_tool(&self, req: CallToolRequestParams, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let n = req.name; let a = req.arguments.unwrap_or_default();
        async move { self.dispatch(&n, a).await }
    }
}
