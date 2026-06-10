use crate::model::config::ResolvedRoots;

pub fn run_install_agent(roots: &ResolvedRoots, agent: &str) -> Result<(), crate::error::AdocsError> {
    let agent_lower = agent.to_lowercase();

    eprintln!("Installing adocs MCP configuration for: {}", agent);
    eprintln!("  source root: {}", roots.source_root);
    eprintln!("  map root:    {}", roots.map_root);

    match agent_lower.as_str() {
        "opencode" => install_opencode(roots)?,
        "cursor" => install_cursor(roots)?,
        "claude" | "claude-code" => install_claude(roots)?,
        "codex" => install_codex(roots)?,
        _ => {
            eprintln!(
                "Agent '{}' not yet supported. Supported: opencode, cursor, claude-code, codex",
                agent
            );
            eprintln!(
                "To configure manually, add adocs MCP server with command: adocs serve --mcp"
            );
        }
    }

    println!(
        "Installation instructions printed above. Restart your agent to pick up changes."
    );
    Ok(())
}

fn install_opencode(roots: &ResolvedRoots) -> Result<(), crate::error::AdocsError> {
    let source_root = roots.source_root.to_string();
    let map_root = roots.map_root.to_string();

    let config = format!(
        r#"{{
  "mcpServers": {{
    "adocs": {{
      "command": "adocs",
      "args": ["serve", "--mcp", "--source-root", "{source_root}", "--map-root", "{map_root}"]
    }}
  }}
}}"#,
        source_root = source_root,
        map_root = map_root,
    );

    println!("OpenCode MCP configuration (add to ~/.config/opencode/opencode.json):");
    println!("{}", config);
    Ok(())
}

fn install_cursor(roots: &ResolvedRoots) -> Result<(), crate::error::AdocsError> {
    let source_root = roots.source_root.to_string();
    let map_root = roots.map_root.to_string();

    println!(
        "Cursor MCP configuration (add to .cursor/mcp.json in your project):"
    );
    println!(
        r#"{{
  "mcpServers": {{
    "adocs": {{
      "command": "adocs",
      "args": ["serve", "--mcp", "--source-root", "{}", "--map-root", "{}"]
    }}
  }}
}}"#,
        source_root, map_root
    );
    Ok(())
}

fn install_claude(roots: &ResolvedRoots) -> Result<(), crate::error::AdocsError> {
    let source_root = roots.source_root.to_string();
    let map_root = roots.map_root.to_string();

    println!(
        "Claude Code MCP configuration (add to .claude/mcp.json or ~/.claude/claude_desktop_config.json):"
    );
    println!(
        r#"{{
  "mcpServers": {{
    "adocs": {{
      "command": "adocs",
      "args": ["serve", "--mcp", "--source-root", "{}", "--map-root", "{}"]
    }}
  }}
}}"#,
        source_root, map_root
    );
    Ok(())
}

fn install_codex(roots: &ResolvedRoots) -> Result<(), crate::error::AdocsError> {
    let source_root = roots.source_root.to_string();
    let map_root = roots.map_root.to_string();

    println!(
        "Codex MCP configuration (add to .codex/config.toml in your project):"
    );
    println!(
        r#"[mcp_servers.adocs]
command = "adocs"
args = ["serve", "--mcp", "--source-root", "{}", "--map-root", "{}"]"#,
        source_root, map_root
    );
    Ok(())
}
