# Configuration

`adocs` has three config layers:

- roots: where source files live and where `.adocs/` is stored
- watch scope: which source files are tracked
- ignore scope: which files are never tracked

## Precedence

Highest to lowest:

1. CLI flags: `--config`, `--source-root`, `--map-root`
2. environment: `ADOCS_SOURCE_ROOT`, `ADOCS_MAP_ROOT`
3. `adocs.toml`
4. current directory

If `--config` is not set, `adocs` looks for `adocs.toml` in the current directory and then walks upward.

## `adocs.toml`

Use this file for project defaults.

```toml
source_root = "source"
map_root = "adocs"
```

Relative paths are resolved from the directory that contains `adocs.toml`.

`source_root` is the tree `adocs` watches. `map_root` is the directory that contains `.adocs/`.

If a root is not set anywhere, it defaults to the current directory.

The optional `[verification]` block is surfaced in `adocs status`.

## `.adocs/.agentwatch`

This file is an include list. One pattern per line. Blank lines and `#` comments are ignored.

Patterns are matched relative to `source_root`.

- `.` tracks everything under `source_root`
- `*.py` tracks only `.py` files at the root of `source_root`
- `**/*.py` tracks `.py` files at any depth
- `src/` tracks everything under `src/`
- `tests/` tracks everything under `tests/`
- `src/**/*.py` tracks only Python files under `src/`
- `tests/**/*.py` tracks only Python files under `tests/`

Run `adocs sync` after changing `.agentwatch`.

## `.adocs/.agenignore`

This is the exclude list. It uses gitignore-style patterns. If a path matches here, it is skipped even if `.agentwatch` includes it.

Use it for generated files, build output, caches, and other paths you never want tracked.

```text
target/
dist/
build/
coverage/
node_modules/
```

Run `adocs sync` after changing `.agenignore`.

## Check What Is Tracked

`adocs status` is action-oriented. For the full inventory, use:

```bash
adocs list --state all --kind files
adocs list --state all --kind folders
```

Use `adocs status --json` if you need a machine-readable snapshot.
