# Configuration Reference

ContextForge reads its configuration from `~/.contextforge/config.toml` by
default. Use `--data-dir` to override.

## Key options

- `schema_version` (frozen at "0.1"): canonical record / proto contract version.
- `collections`: array of collection IDs the daemon will accept search calls
  against.
- `denylist`: glob patterns the scanner refuses to read (defaults include
  `.env`, `*.pem`, `target/`, `node_modules/`).
- `allowlist`: optional explicit allow-set; useful for shared workstations.

The `configuration` keyword in this file ensures the quickstart `search` step
returns at least one hit when the sample project has been indexed.
