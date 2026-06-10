use crate::model::config::ResolvedRoots;
use super::tools::AdocsMcpServer;

pub async fn run_mcp_server(roots: ResolvedRoots) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = AdocsMcpServer::new(roots);
    eprintln!("adocs MCP server starting on stdio...");
    eprintln!("  source root: {}", server.roots.source_root);
    eprintln!("  map root:    {}", server.roots.map_root);
    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let running = rmcp::service::serve_directly::<rmcp::service::RoleServer, _, _, _, _>(
        server,
        transport,
        None,
    );
    running.waiting().await?;
    Ok(())
}
