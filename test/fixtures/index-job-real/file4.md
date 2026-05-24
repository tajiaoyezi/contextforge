# ContextForge Phase 11 Fixture File 4 — Orphan Reaper

If the contextforge-core daemon receives SIGKILL while an index job is
`status=running`, the SQLite row is left in an inconsistent state — the
JobRunner that owned the job is dead, but the row still reads `running`.

The orphan reaper is a function called during daemon startup that scans
all `status IN (queued, running)` rows and marks them as `failed` with
`error_message = "job lost: daemon restart"`. This ensures:

1. Console UI immediately sees an honest terminal status after restart
2. New enqueues for the same workspace are not blocked by stale rows
3. The reaper runs *before* the gRPC server begins accepting traffic, so
   there's no race window where a new Enqueue might see a stale running
   row from the previous boot

In a multi-instance daemon deployment (v1.0 scope) the reaper would need
leader-election semantics — that's out of scope for v0.4
[SPEC-DEFER:task-future.multi-daemon-leader-election].
