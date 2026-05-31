# language: en
# Maps to:
#   - docs/specs/phases/phase-27-memory-ops-hardening.md
#   - docs/specs/tasks/task-27.1-memory-pin-actor-and-timestamp.md
#   - docs/specs/tasks/task-27.2-memory-pin-unpin-split-and-hard-delete.md
#   - docs/specs/tasks/task-27.3-closeout-v0.20.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 27 memory-ops-hardening。Scenario ID 在各 task spec §7 追踪表映射到具体测试。

Feature: phase-27-memory-ops-hardening
  In order to 让 memory pin 语义可审计（谁/何时置顶）、生命周期可显式操作与彻底清除、并能从审计回填历史 pin 状态
  As Phase 27 内核（pin-actor + timestamp + Pin/Unpin 拆分 + hard-delete + is_pinned audit backfill + v0.20.0 收口）
  I want MemoryItem 加 add-only pinned_by/pinned_at_unix + 显式 Unpin/HardDelete RPC（X-Confirm gated）+ is_pinned 从 audit 重放，且全 add-only 不破坏既有 v0.6-v0.19 client、proto-freeze guard 过、受阻态如实记录不伪造

  # ---
  # Maps to: docs/specs/tasks/task-27.1-memory-pin-actor-and-timestamp.md (TEST-27.1.1/27.1.2/27.1.3)
  Scenario: SCEN-27.1.1 — 对应 AC1（pin-actor + pinned-at-timestamp add-only 字段 + 写穿 round-trip）
    Given proto MemoryItem 加 add-only string pinned_by + int64 pinned_at_unix（序号在既有 field 10 后追加，不动既有 tag）+ memory_items add-only migration（缺省 backfill）+ SqliteMemoryStore actor-aware set_pinned
    When  pin=true（actor-aware）→ get/list 投影，或 pin=false → get；并跑 proto-freeze guard
    Then  pin=true 写穿 pinned_by=actor + pinned_at_unix>0、pin=false 归 pinned_by=''+pinned_at_unix=0（TEST-27.1.2）；MemoryServer.pin RPC 写穿 actor（source console-api）+ memory_to_pb 投影 + migration 既有行缺省 backfill 不破坏既有 5 memory 单测（TEST-27.1.3）；FROZEN 契约新字段为 superset 追加不退化（TEST-27.1.1）；默认构建 0 新 dep（rusqlite/serde 已 direct）；actor 真实 per-user 透传 [SPEC-DEFER:phase-future.memory-actor-propagation]

  # ---
  # Maps to: docs/specs/tasks/task-27.2-memory-pin-unpin-split-and-hard-delete.md (TEST-27.2.1/27.2.2/27.2.3/27.2.4)
  Scenario: SCEN-27.2.1 — 对应 AC2（Pin/Unpin 显式拆分 + hard-delete + X-Confirm 兜底）
    Given proto add-only Unpin/HardDelete RPC + 4 message（既有 5 RPC + Pin{bool pin} 签名不动）+ SqliteMemoryStore::hard_delete 物理删除 + AuditOperation::MemoryHardDelete + console-api unpin/hard-delete 路由
    When  跑 hard_delete（DELETE FROM memory_items）后 get-by-id、unpin 显式幂等、console-api POST /v1/memory/{id}/hard-delete 缺/带 X-Confirm
    Then  hard_delete 后 get-by-id 返 None（vs soft-delete 仍返行）+ 行不存在 NotFound + RPC emit MemoryHardDelete audit（TEST-27.2.2）；unpin = set_pinned(false) + emit MemoryUnpin 幂等、既有 pin toggle 不变（TEST-27.2.3）；console-api hard-delete 缺 X-Confirm → 412、带 X-Confirm:yes（或 ?confirm=true）→ 204、unpin → 204、既有 deprecate/soft-delete 412 不退化（TEST-27.2.4）；FROZEN 契约 service/message superset 追加不退化（TEST-27.2.1）；hard-delete 级联清理 [SPEC-DEFER:phase-future.memory-hard-delete-cascade]

  # ---
  # Maps to: docs/specs/tasks/task-27.3-closeout-v0.20.0.md (TEST-27.3.1/27.3.2/27.3.3/27.3.5)
  Scenario: SCEN-27.3.1 — 对应 AC1/AC3/AC5（is_pinned audit backfill + smoke v17 + v0.20.0 收口 + ADR-032 ratify）
    Given is_pinned audit backfill（按 memory_pin/memory_unpin 事件时序重放）+ scripts/console_smoke.sh v17 + v0.20.0 release docs + ADR-032（memory-ops-hardening）
    When  构造已知 audit 序（pin,unpin,pin）+ is_pinned=false legacy item → opt-in 一次性 reconcile（非热路径）；smoke v17 跑 memory ops 硬化断言；ADR-032 据 task-27.1/27.2 真实结果 ratify
    Then  backfill 后 is_pinned = 重放末态（末次 pin → true）+ 无 audit 事件 item 保持原态（TEST-27.3.1）；audit 缺失/被裁剪 item 覆盖率 caveat 如实记录 [SPEC-DEFER:phase-future.is-pinned-backfill-from-audit]；smoke 既有 step 不退化 + bash -n exit 0；ADR-032 据真实非合成验证 Proposed→Accepted + ADR-022 §Trade-offs 三条 marker add-only Amendment（pin_actor→pinned_by / memory-pinned-at-timestamp→pinned_at_unix / is-pinned-backfill 落地，不溯改正文 D1-D5）；phase-27 §6 全 met；ADR-014 D1-D5（第十八次激活）全通过
