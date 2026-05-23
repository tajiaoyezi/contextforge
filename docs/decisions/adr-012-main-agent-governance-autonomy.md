# ADR `012`: `main-agent-governance-autonomy`

**Status**: Accepted
**Category**: Governance / 单驱动自治
**Date**: 2026-05-23
**Decided By**: tajiaoyezi objective + main agent execution
**Related**: ADR-011 / AGENTS.md §2A §4 §8 / docs/s2v-adapter.md / _dispatch/README.md / R3 / R6 / R7

## Context

ADR-011 removed external worker terminals and made the project a single-driver variant:

- main agent is the only coordinator;
- implementation/review subagents return structured objects inside the main agent context;
- user copy-paste dispatch is retired.

After ADR-011, several governance paths still carried pre-ADR wording that required explicit user review even when the trade-off was already bounded by PRD, task spec, ADR, or a direct user objective:

1. Draft/§2A preflight review could still imply waiting for user approval before promoting a spec to Ready.
2. PR merge was mechanically main-agent-only, but some wording still treated merge as a separate user decision after Gate 0-5.
3. R7 dependency handling still required the main agent to run the chore-dep PR, but did not explicitly say the main agent may self-decide bounded dependency additions.
4. §8 Waive path still said option C should go to user review even when the waiver criteria, replacement verification, and residual risk were clear.

The current v0.1 ship objective explicitly authorizes autonomous completion and says environment and governance blockers should be handled by the agent directly when bounded. Keeping user-review wording in these paths reintroduces manual latency that ADR-011 was meant to remove.

## Decision

The project adopts **main-agent governance autonomy** for bounded execution decisions:

- **§2A / Draft Ready review**: main agent may perform the preflight review itself and promote Draft/In Progress flow fields to Ready/In Progress when Scope, Behavior Contract, AC, traceability, and verification can be grounded in PRD/spec/ADR/user objective evidence. If business contract fields are ambiguous or contain `<TBD-by-user>`, main agent must first fill them from cited project sources or create a spec-fix PR.
- **R6 merge decision**: after AGENTS.md §4 Gate 0-5 pass, main agent may merge the PR without a separate user confirmation. R6 remains physical: no business commit on `master`, no `reset --hard`, no force push, and merge remains `--no-ff` through a feature/chore branch.
- **R7 dependency decision**: subagents still cannot edit lockfiles. Main agent may autonomously create and merge a dedicated chore-dep PR for dependencies required by an accepted task/spec, as long as the PR records package/version/use and passes verification.
- **§8 Waive decision**: main agent may choose Waive without user roundtrip when the waiver is bounded by PRD/spec/ADR/user objective, uses the conservative order `backward compatibility > spec wording > smallest change`, and records the full Waiver block plus normalized §10 Completion Notes. User review is still required for unanchored product trade-offs, privacy/security weakening, release integrity weakening, or destructive operations.
- **Branch mismatch exception**: R3/R6 physical insurance remains stricter than other autonomy paths. On branch mismatch, the agent must stop, preserve evidence, create `BLOCKED-branch-mismatch.md`, and only proceed automatically when recovery is deterministic and non-destructive (for example reflog/cherry-pick onto the expected branch with the wrong commit preserved by tag/backup branch). Ambiguous or destructive recovery still escalates to user review.

## Rationale

- ADR-011 makes the main agent the single decision chain. Requiring user review for bounded execution decisions contradicts that topology and recreates the latency of the old worker-terminal process.
- The safety boundary should be evidence and physical git safeguards, not manual approval for every routine governance choice.
- R3/R6 and `BLOCKED-branch-mismatch.md` remain because branch mismatch can lose code. This is a different risk class from a bounded waiver or dependency chore PR.
- R7 still protects supply chain changes from subagents while allowing the main agent to unblock implementation without waiting for a separate human approval when the need is explicit.

## Consequences

- Main agent can close v0.1 without pausing on routine §2A, merge, dependency, or waiver decisions.
- Every autonomous decision must leave auditable evidence in the normal carrier: ADR, task §10, commit message, merge commit, or `BLOCKED-branch-mismatch.md`.
- User review is reserved for ambiguity, destructive recovery, security/privacy weakening, release integrity weakening, or decisions with no project source anchor.
- Existing Phase 1-7 history is not rewritten.

## Rollback Or Migration Plan

If autonomous governance causes incorrect merges or unacceptable trade-offs:

1. Create a new ADR superseding this one.
2. Restore explicit user-review gates for §2A, R7 chore-dep PRs, §8 Waive, or merge decision as needed.
3. Keep R3/R6 physical safeguards unchanged; they are not relaxed by this ADR.

## Follow-ups

- Use Phase 8 as the first full dogfood pass for autonomous §2A, Waive, and release decisions.
- If branch mismatch recovery happens, review the generated `BLOCKED-branch-mismatch.md` and decide whether deterministic recovery should remain allowed.
