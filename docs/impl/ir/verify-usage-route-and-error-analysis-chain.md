---
id: "IMPL-034MpJiRnTxGKRzkCAn04f"
type: "impl"
status: "completed"
title: "Verify usage route and error analysis chain"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/text_format.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_report.rs"
      - type: "file"
        path: "docs/designs/usage-grouped-route-error-analysis-issue-184.md"
      - type: "file"
        path: "docs/specs/usage-analysis-scope-issue-184.md"
      - type: "file"
        path: "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
      - type: "file"
        path: "docs/tasks/implement-usage-by-error-aggregation.md"
      - type: "file"
        path: "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
      - type: "file"
        path: "docs/impl/ir/wire-usage-group-by-flags-text-and-guide-issue-184-t4.md"
      - type: "file"
        path: "docs/plans/implement-usage-route-error-analysis-issue-184.md"
      - type: "file"
        path: "docs/tasks/verify-usage-analysis-chain-and-audit.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/verify-usage-analysis-chain-and-audit.md"
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
  verifies:
    - "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
  verified-by: []
  references:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
    - "docs/tasks/verify-usage-analysis-chain-and-audit.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Verify usage route and error analysis chain

## Summary

T6 の closeout 後に PR review 指摘を修正し、focused tests、canonical
`mise run check`、doc-status と IR 監査を再実行した。

## Context

承認済み plan の T6 と usage route/error contract test-spec を検証した。
PR review で明確になった malformed JSONL の read-side privacy/shape guard
と text/JSON parity を修正して文書化した。

## Implementation

### Changes

T6 task と plan の検証記録を PR review 後の状態へ更新した。read-side
`error.reason` は abort allowlist に限定し、`execution` echo は number/null
だけを許容する。Text count detail は map key を直接描画して `=` を保持する。
不正 reason/execution 値と `a=b` text key の回帰 test を追加した。

### Decisions During Implementation

T4 review の non-blocking `=` 表示指摘は PR review で受入条件との不一致が
再確認されたため、本 PR 内に修正した。writer が生成しない arbitrary reason
string と execution primitive は不正な保存入力として read-side で無視する。

## Related Experiments

### Adopted

既存 T5 Usage JSONL peak-memory 証跡を plan/task と照合した。本 T6 では
新しい experiment log を作成していない。

### Rejected

なし。

## Validation

| Check | Result |
| --- | --- |
| `usage_report::` | 67 passed, 0 failed after PR review fixes |
| `args::` | 47 passed, 0 failed |
| `usage_log::` | 21 passed, 0 failed |
| `mise run check` | exit 0; doc ID scan 1244 Markdown files; Node 41 passed / 2 skipped / 0 failed; all other stages passed |
| `audit_docs.js --type all --json` | exit 0; 1076 files, 841 non-blocking warnings, 0 blocking; no findings in #184 artifacts |
| `audit_impl_record.js --json` | 186 files, 0 findings |

## Risks

None remaining from the PR review. Invalid `error.reason` and `execution`
primitive values from persisted JSONL are omitted at read time; valid writer
records are unchanged. Text detail rendering preserves keys verbatim.

## Follow-ups

T6 and PR review follow-up are complete. All three PR review findings are
addressed on this branch; canonical verification and doc-status pass.
