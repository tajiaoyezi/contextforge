# ContextForge Phase 11 Fixture File 2 — JobRunner Heartbeat

Every 100 files or 5 seconds, whichever comes first, the indexer's progress
callback persists `processed_files`/`total_files`/`stage` into
`index_jobs.last_heartbeat_at_unix` via `SqliteJobStore.update_progress`.

The Phase 11 integration test `test_heartbeat_persists_every_100_files_or_5s`
verifies this contract by spinning a 200-file synthetic workspace and
sampling `processed_files` over a 5s window.

For contextforge users this matters because:

- the Console UI Index Jobs page polls `/v1/index-jobs/<id>` every few
  seconds; without heartbeats it would see the job stuck at processed=0
- a daemon that crashes mid-index leaves a stale `running` row that the
  orphan reaper marks `failed` on next startup
- `cancel_requested=1` flag is checked at the same heartbeat boundary, so
  cancel latency is bounded by 5s in the worst case
