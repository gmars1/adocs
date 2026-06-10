use clap::Parser;

#[derive(Debug, Parser)]
struct McpCli {
    #[arg(long)]
    pub source_root: Option<camino::Utf8PathBuf>,
    #[arg(long)]
    pub map_root: Option<camino::Utf8PathBuf>,
    #[arg(long)]
    pub config: Option<camino::Utf8PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = McpCli::parse();
    let roots = adocs::model::config::resolve_roots(cli.source_root, cli.map_root, cli.config)?;
    adocs::mcp::server::run_mcp_server(roots).await.map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;
    Ok(())
}
