# Sample Project

A minimal sample project used by the ContextForge quickstart. The keyword
`configuration` shows up here so `contextforge search "configuration"` has
a guaranteed hit during the smoke walkthrough.

## Layout

- `docs/` — project documentation, including the configuration reference.
- `src/` — a tiny Go entrypoint to give the language detector a non-text
  signal to identify.
- `logs/` — a small log sample for the log-source classifier.
- `.env` — denylisted; the scanner refuses to read it.
- `config.yaml` — contains a synthetic AWS key that the redactor must
  rewrite before indexing.
