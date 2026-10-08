---
id: "IMPL-034Bh4xhHhdbrsNKFkZ6iH"
type: "impl"
status: "completed"
title: "Phase 0: Rig PoC go/no-go spike"
created: "2026-10-06"
updated: "2026-10-06"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "directory"
        path: "crates/poc-rig/"
      - type: "file"
        path: "docs/research/rig-agent-runtime/poc-results.md"
    modified:
      - type: "file"
        path: "docs/designs/rust-rig-runtime-migration-architecture.md"
      - type: "file"
        path: "docs/plans/phase-3-port-runtime-adapter-providers.md"
      - type: "file"
        path: ".gitignore"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/phase-0-rig-poc-go-no-go-gate.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/rig-poc-go-no-go-conditions-measurement.jsonl"
    rejected: []
---

# Phase 0: Rig PoC go/no-go spike

## Summary

rig 0.43 の実機 PoC spike を `crates/poc-rig/` に実装し、go/no-go ゲート
9 条件を実測した。結果は **go(条件付き)** — 条件は macOS バイナリ未実測
(環境なし、Phase 5 の release matrix 初回実行で確認)。

## Context

TASK-034BgK4O6Xu8NEvlYtW6lJ(`docs/tasks/phase-0-rig-poc-go-no-go-gate.md`)/
PLAN-034BepX6xCr0AAqkJ63aOv を実装。設計書の §Verification Notes 9 条件に
対応する T1-T7。対象は `crates/poc-rig/` と `poc-results.md` に限定し、
既存 TS 資産・`mise.toml`・CI contract は不変。

## Implementation

### Changes

- `crates/poc-rig/`(新設): `run`(Ollama 実 run)/`run-scripted`/
  `repair-run`/`error-probe`/`abort-probe`/`panic-probe` 6 コマンド。
  `Accumulator`(hook 逐次集計、PromptResponse 非依存)が usage/turns/
  toolCalls/invalidToolCalls/repairs の唯一の供給源。usage は
  `on_outcome` の completion outcome で per-call 加算し、rollback された
  attempt は両 hook 不発火のため `on_run_settled` で run 総量へ
  reconcile(max)する二段方式(レビュー指摘を受けて修正・実測証明)。
- `src/providers/devin_wire.rs`: custom `Wire`/`Decoder`/`Transport` 骨格
  (Connect framing/gzip/JWT/metadata/context map の trait 面表現を証明)。
- `.gitignore` に `crates/*/target/` を追加。
- PoC 知見を design(§invalid tool call)と phase-3 plan(手順 10)に
  loopback 反映:`max_invalid_tool_call_retries` 既定 0 の罠。

### Decisions During Implementation

- repair は Rig の extractor/output_tool ではなく run→jsonschema 検証→
  corrective feedback 再 run の手動 loop で証明(設計の「repair session」
  契約を直接検証するため)。accumulator の `Arc` 共有で合算を確認。
- 実 timeout の分類は scripted model では再現不能なため、
  `ProviderError::Http(ErrorKind::Http)` を合成して `error-probe` に
  transport case として追加。
- scripted fake provider には `rig` の `test-utils` feature
  (`MockCompletionModel`/`MockTurn`/`MockStreamEvent`)を使用。

## Related Experiments

### Adopted

`rig-poc-go-no-go-conditions-measurement.jsonl`: サイズ実測・
invalid-retry budget・error surface・hook 集計の観測を記録し、全て
設計前提の裏付けとして採用。

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `cargo check` / `cargo build` / `cargo build --release` 成功
  (warning は skeleton 未使用のみ)。
- Windows x64 release: 7,986,176 B / Linux musl(docker alpine): 7,761,504 B。
- 実 Ollama run(qwen3.5:4b): turns=2・toolCalls=1・usage 1155/122/1277。
- scripted probe 全 pass(provider-error snapshot・unknown-tool-call 復帰・
  repair 合算・429/5xx/transport 分類・abort 部分 snapshot・panic→
  INTERNAL_ERROR exit 70)。
- 判定記録: `docs/research/rig-agent-runtime/poc-results.md`。

## Risks

- macOS build 未実測(条件付き go)。
- Rig 0.x の breaking change 頻度は継続リスク(`=0.43.0` pin で対処)。
- 実 provider での 429/backoff 実測は Phase 3 へ繰越。

## Follow-ups

Phase 1(TASK-034Bgiqu2ZlaprN931tnY、`cheaper-core` 移植)が runnable。
