<p align="center">
  <img src="https://img.shields.io/badge/status-alpha-orange?style=flat-square" alt="Status: Alpha">
  <img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="License: MIT">
</p>

# adocs

**An agent map for your codebase.**

`adocs` builds and maintains a navigable map of your repository for LLM
coding agents. It tracks every source file's content hash alongside its
agent-facing documentation, so agents can instantly tell what's current and
what went stale after an edit.

No more guessing whether last week's `file_description.md` still matches
the code. `adocs` tells you.

---

## The map

For each source file, `adocs` keeps a `file_description.md` — a plain-text
doc written by you or your agent that explains what the file does, its API,
edge cases, and gotchas. For each folder, a `folder_purpose.md` explains why
that directory exists.

When source content changes, its doc flips to **stale**. The agent (or you)
edits the doc to reflect the new code, runs `adocs update`, and it goes back
to **valid**. A human can then `adocs seal` it, meaning it's reviewed and
solid.

```
stale  →  valid  →  sealed
```

- **stale** — code changed since the doc was last accepted, or doc is missing
- **valid** — doc matches current code (agent accepted it with `adocs update`)
- **sealed** — doc matches current code AND a human reviewed it (`adocs seal`)

Move or rename a file without changing its content? The doc follows it
automatically. Edit the content? The doc goes stale so the agent knows to
refreshen it.

---

## Install

Build from source:

```bash
git clone https://github.com/gmars1/adocs
cd adocs
cargo build --release
```

---

## Quick start

```bash
# Create the map skeleton in your project
adocs init

# Generate missing doc templates for new files, move docs for renamed files,
# remove docs for deleted files
adocs sync

# See what changed in source since last observation
adocs changed

# Action list: source changes + docs that need attention
adocs status

# After editing a file_description.md to match new code, accept it
adocs update src/auth/session.rs

# Human review — mark as reviewed
adocs seal src/auth/session.rs

# List everything stale
adocs stale
```

---

## Configuration

`adocs.toml`, `.adocs/.agentwatch`, and `.adocs/.agenignore` are documented in [configuration.md](configuration.md).

---

## CLI

| Command | |
|---|---|
| `adocs init` | Create `.adocs/` in the project root |
| `adocs sync` | Materialize missing templates, move docs for same-hash renames, delete orphans |
| `adocs changed [--json]` | Read-only: added, modified, deleted, moved, renamed, ambiguous |
| `adocs status [--json]` | Read-only action list: source changes, docs to update/create, ambiguity, and summary counts. Use `--json` for the full inventory |
| `adocs list --state <stale\|valid\|sealed\|all>` | Filter by state; add `--kind files` or `--kind folders` |
| `adocs stale` | Shorthand for `list --state stale` |
| `adocs valid` | Shorthand for `list --state valid` |
| `adocs context <path>` | Show description, purpose, state, and seal metadata for a path |
| `adocs docsunder <path>` | List all valid docs under a folder |
| `adocs update <path>` | Accept current doc for current source hash (stale → valid) |
| `adocs seal <path>` | Human marks the file as reviewed and sealed |
| `adocs rebind <file-id> <new-path>` | Manually reconnect a doc when the move was ambiguous |
| `adocs serve --mcp` | Start the MCP server |
| `adocs install-agent <agent>` | Print MCP config for opencode, cursor, claude-code, or codex |

Global flags: `--source-root`, `--map-root`, `--config`.

---

## MCP server

Agents talk to `adocs` over MCP. Start the server:

```bash
adocs serve --mcp
```

Or auto-configure your agent:

```bash
adocs install-agent opencode
```

Tools exposed over stdio:

| Tool | |
|---|---|
| `adocs_status` | Full workspace snapshot: states, changes, missing docs, ambiguity |
| `adocs_changed` | Source changes since last observation |
| `adocs_sync` | Bring `.adocs/` in line with the source tree |
| `adocs_list_state` | List files/folders by state |
| `adocs_read_context` | Description + purpose + state + seal metadata for a path |
| `adocs_read_file_description` | Read one file description with its state |
| `adocs_read_folder_purpose` | Read one folder purpose with its state |
| `adocs_read_folder_docs` | All valid docs under a folder; can be large |
| `adocs_explain_staleness` | Why is this path stale? |
| `adocs_update_doc` | Accept a doc |
| `adocs_request_seal` | Ask a human to seal a path (agents cannot seal) |

Agents should call `adocs_status` on entry, then `adocs_read_context` before
diving into source files.

---

## Split workspace

Keep agent docs out of your source repo:

```
workspace/
├── source/
└── adocs/
```

```toml
# adocs.toml
source_root = "source"
map_root = "adocs"
```

---

## Layout on disk

```
.adocs/
├── agents/
│   └── src/auth/
│       ├── folder_purpose.md
│       └── session.rs/
│           └── file_description.md
└── .hashes/
    └── files.json          # content hashes and doc linkage
```

---

## License

MIT 
