# Quickstart Fixture

This directory holds the canonical fixture used by `scripts/quickstart_smoke.sh`
and the README **Quick Start** walkthrough.

Layout:

```
examples/quickstart/
  sample-project/    # synthetic source repo for `contextforge index`
    README.md          (contains the keyword `configuration` — search target)
    docs/config.md
    docs/setup.md
    src/main.go
    logs/app.log
    .env               (denylist — scanner skips by default)
    config.yaml        (contains a fake AWS key — secret redaction)
  hermes-memory/     # Hermes-shaped fixture for `contextforge import hermes`
    MEMORY.md
    USER.md
```

Total size: well under 100 KB.

See the top-level [README.md](../../README.md) for the runnable commands and
[scripts/quickstart_smoke.sh](../../scripts/quickstart_smoke.sh) for the
one-shot smoke-test script that drives all seven steps end-to-end.
