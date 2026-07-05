# Task `51.3`: `go-rest-workspace-owner — Go REST workspace handler 用 verified identity`

**Status**: Ready
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 51 (workspace-isolation)
**Dependencies**: task-51.1/51.2（WorkspaceStore + proto owner）/ ADR-016 D3（Go thin proxy）

## 1. Background
workspace handler 不读 verified identity。本 task 让 POST/GET /v1/workspaces 从 context 读 verified userID 传给 grpcclient。

## 2. Goal
(1) POST /v1/workspaces：verified user → owner_id = verified userID；trusted-network → owner_id = ""（unowned）。
(2) GET /v1/workspaces：verified user → 仅自己 own 的 + unowned；trusted-network → all（byte-equiv）。
(3) GET /v1/workspaces/{id}：verified user → 仅自己 own 的或 unowned；否则 403；trusted-network → all。

## 3. Scope
- 改 `internal/consoleapi/handlers.go`：handleCreateWorkspace/handleListWorkspaces/handleGetWorkspace 读 verifiedUserIDKey context
- 改 `internal/consoleapi/grpcclient/grpcclient.go`：workspaceClient.Create 加 owner 参数；List/Get 加 owner filter 参数
- 改 `internal/consoleapi/types.go`：WorkspaceClient interface 加 owner 参数（或新方法 ListOwned/GetIfOwned）
- 单测：POST with per-user token → owner_id = userID；GET list → 仅自己 + unowned；trusted-network → all

## 4.1 行为契约（byte-equivalent 关键）
- **trusted-network（空 token）**：owner_id = ""（unowned）；list/get → all（v2.0.0-alpha 行为不变）
- **旧 shared token**：同 trusted-network（无 verified identity 注入）
- **per-user token**：owner_id = verified userID；list → 自己 own + unowned；get → 403 if 非 own 且非 unowned

## 6. AC
- [ ] **AC1**: POST workspace with per-user token → owner_id = verified userID — verified by **TEST-51.3.1**
- [ ] **AC2**: GET workspaces with per-user token → 仅自己 own 的 + unowned — verified by **TEST-51.3.2**
- [ ] **AC3**: trusted-network byte-equivalent（all visible，owner_id=""）— verified by **TEST-51.3.3**

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-51.3.1 | POST workspace owner_id=verified userID | go test | Not Started |
| TEST-51.3.2 | GET list 仅自己 own + unowned | go test | Not Started |
| TEST-51.3.3 | trusted-network byte-equivalent | go test | Not Started |

## 9. Verification
```bash
go test ./internal/consoleapi/ -run TestTask513 -v
go test ./internal/cli/ # no-regression
```

## 10. Completion Notes
**Status**: Ready
1. **完成日期**：<TBD-after-impl>
2. **改动文件**：<TBD-after-impl>
3. **commit 列表**：<TBD-after-impl>
4. **§9 Verification 结果**：<TBD-after-impl>
5. **剩余风险**：<TBD-after-impl>
6. **下游影响**：task-51.4（SearchService thin gate 据本 task verified identity 贯穿）
