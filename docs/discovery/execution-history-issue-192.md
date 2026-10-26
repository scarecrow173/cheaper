---
id: "DISC-034MEa1cEgBDoAonnW1YkV"
type: "discovery"
status: "resolved"
title: "Execution History for Issue 192"
created: "2026-10-26"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/192"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/usage-analysis-scope-issue-184.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/execution-history-issue-192.md"
    - "../adr/record-per-execution-history-with-bounded-best-effort-storage.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Execution History for Issue 192

## Exploration Goal

決定・実行の個別調査に必要な History の範囲と実装境界を定める。Issue #192 は outcome と主要な安全要件を定義する一方、永続形式、相関IDの発行・継承、記録粒度、retention、同時書込みとquery方法を比較可能な形にしていない。usage集計を履歴イベントへ置き換えず、cheaper が観測できた事実だけを追跡する。

## Key Issues and Assumptions

- Issue #192 の明示済み範囲: Worker/Workflow Job/Route実行と、意味のあるHook/Route判断を時系列で確認する。Hook受信監査、上流agentの未観測行動、無制限transcript保存、権限変更、外部送信は対象外。
- HookのDenyは`bulk-reader`推奨でありWorker開始ではない。RouteのRewrite発行と実際のRoute runも別の観測事実として扱う。
- `usage.jsonl`はコスト・token集計の正本。#184はoptional execution IDsを受容するが、その発行・相関・履歴保存は#192の責務とする。
- parent/trace metadataは相関の手掛かりでありgrant/policyの根拠にしない。環境変数等のcross-process carrierはforgeableなので、値を受信してもactual parent edgeにせずinherited-unverified hintとして保持・表示する。actual `parentExecutionId`はin-process lifecycleが確認した包含関係に限る。
- Hypothesis: 既存Rust依存だけでID/storeを実装し、queryがstoreを作らず、記録失敗を呼出し結果へ伝播しない構成を優先する。
- Briefing override: Issue本文がintent/success/non-goalsを十分具体化しているため、profileのconditional `deep-dive` は質問の反復を避けて省略する。残る技術選択はこのDiscovery/ADRで比較し、独立reviewで確認する。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| One shared JSONL file with a cross-process lock | Single file; naturally ordered stream | Every decision/terminal write contends; stale lock and tail recovery need cross-platform rules | no |
| One JSONL segment per process/session | Single writer avoids cross-process interleaving | Active-file retention race, long-lived writers, and stale crash segments require lifecycle/heartbeat rules | no |
| One execution directory with immutable start/terminal files, atomically published from pending dir | Per-execution writes are isolated; crash after start remains inspectable; no descriptor stays open for execution lifetime | More directories/files; bounded scan and best-effort retention required, stale unpublished directories need cleanup | selected in ADR after targeted review |
| SQLite/indexed database | Query/paging and transactional updates | Adds database migration/locking and dependency cost | no unless measured file scan fails |
| UUID v4 per execution unit; explicit context propagation for parent/trace hints | Existing `uuid` dependency; IDs reveal no input and grant no authority | Caller wiring across nested boundaries remains necessary; external hints are unverified | likely |
| Reuse Route `runId` as execution ID | No extra identifier for route runs | Violates independent execution identity and refetch semantics | no |

## Tentative Conclusions and Hypotheses

- Per-execution directories with immutable start/terminal files offer the selected cross-process writer boundary. A truncated terminal stays invalid and cannot replace a competing terminal (`create_new`, first-writer-wins); the earlier valid start remains visible as incomplete/unknown. Started executions publish from a pending directory; bounded writer sweeps remove stale pending/empty directories. Queries ignore pending entries.
- Hypothesis: assign a fresh UUID v4 `executionId` per execution unit and a fresh `traceId` per root; propagate parent/trace hints only over explicit cheaper-controlled boundaries. Keep Route `runId`/`logRef` separately and record their mapping.
- Every record uses `recordedAt` as the shared stable ordering field; list/trace use `(recordedAt DESC, executionId DESC)`. An incomplete bounded scan has no continuation cursor; pagination is guaranteed only when the retained set can be fully scanned within budget.
- The ADR defines field-specific metric caps and limits provider/model projection to resolved catalog identity fields. Out-of-range metrics are omitted, never clamped; raw endpoint values are not persisted.
- Resolved provider/model identity must be projected from catalog identity fields only; endpoint/base URL and raw user-supplied model strings are outside that projection. The catalog owns custom ID validation, preserving official colon-bearing IDs.
- Pending-directory cleanup after 24 hours is separate from final-record retention. A final directory is an orphan only when it has neither a valid started nor a valid decision record; a valid start with missing/corrupt terminal remains incomplete under normal retention.
- Metric values are finite nonnegative integers within field caps. `exitCode` is a separate signed integer field so conversion does not erase source sign.
- Confirmed by #192: terminal event may be absent on process kill; readers report incomplete/unknown instead of synthesizing success. Store only allowlisted classified fields; no raw argv/env, payload, tool text, or error body.
- Confirmed by #184 scope: adding optional IDs must not alter usage cardinality, savings, route-refetch aggregation, or orphan join semantics.

## Remaining Validation

- Cross-platform tests must verify pending-directory publication/cleanup and retention races while keeping Hook writes best-effort.
- Implementation tests must verify event→usage owner mappings, field allowlists/caps, complete-scan pagination, and incomplete-scan cursor omission.
- Measure proposed retention/query targets with capped fixtures; report budget limits as incomplete rather than claiming complete results.

## Promotion Candidates

- [x] spec-doc needed: observable per-execution state, relationship, decision reason, and History CLI contracts for Issue #192.
- [x] adr-doc needed: event schema, IDs, storage/append/query/retention and sensitive-data policy.

## Sources and Relations

- Primary: [Issue #192](https://github.com/scarecrow173/cheaper/issues/192).
- Related contract: `docs/specs/usage-analysis-scope-issue-184.md` (usage aggregate ownership and optional-ID tolerance).
- Source-audit targets: `crates/cheaper-cli/src/usage_log.rs`, Worker/Workflow/Route CLI entrypoints, Hook decision paths, workflow job lifecycle, and existing route log/refetch behavior.
