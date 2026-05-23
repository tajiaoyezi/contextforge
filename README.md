# ContextForge

ContextForge is a local-first context indexing and retrieval tool for agent memory, rules, source files, logs, and project notes.

v0.1 ships as a minimal Linux x86_64 tarball with two binaries:

- `contextforge`: Go control-plane CLI, REST/MCP adapter, export and eval entrypoint.
- `contextforge-core`: Rust data-plane daemon for scan, parse, chunk, index, and retrieval.

## Quick Start

```bash
tar -xzf contextforge-linux-amd64.tar.gz
cd contextforge-linux-amd64
export PATH="$PWD:$PATH"

contextforge init --root "$HOME/.contextforge"
contextforge index --source ./example --data-dir "$HOME/.contextforge" --collection default --resume
contextforge search "configuration" --collections default --top-k 5 --explain
contextforge eval run --collection default
```

Use `contextforge.example.toml` as the starting point for collection allowlists and local-only provider settings.
