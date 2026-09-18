---
id: "TASK-034r05iHQeQpDpMem6J7OE"
type: "task"
status: "done"
title: "Update README and add provider setup guide"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/harden-repair-abort-grace-and-provider-failure-normalization.md"
  implemented-by: []
  depends-on:
    - "docs/plans/harden-repair-abort-grace-and-provider-failure-normalization.md"
    - "docs/tasks/extract-promptwithabortgrace-helper-and-apply-to-main-run.md"
    - "docs/tasks/apply-abort-grace-and-provider-failure-normalization-to-repair.md"
    - "docs/tasks/count-consumed-repair-attempts-in-meta-repairs-on-failure.md"
    - "docs/tasks/anchor-policy-timeout-at-runworker-start-and-amend-design-0004.md"
    - "docs/tasks/isolate-unit-tests-from-user-and-project-config-via-env.md"
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
---

# Update README and add provider setup guide

## Work

README の「開発初期（設計フェーズ）。実装はこれから。」を現状（V1 実装済み・実 provider 検証済み）に更新する。`docs/guides/providers.md` を新規作成し、実 provider 運用知識（Pi `models.json` の Ollama 設定、`models.yaml` の `overrides`/`preferences`、`reasoning_effort: "none"`、`response_format` は repair のみ・main では tool call 抑制のため不採用、`CHEAPER_CONFIG_DIR` による設定隔離）を集約する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md`
- `docs/impl/exp/ollama-real-provider-smoke-verification.jsonl`
- `docs/designs/ollama-real-provider-smoke-verification-setup.md`

## Dependencies

- TASK-034r05iHQeQpDpFFl8q6NC 〜 TASK-034r05iHQeQpDpKWISE2NB（コードの最終状態を反映するため最後に実施）

## Affected Files

- `README.md`
- `docs/guides/providers.md`（新規）

## Done When

- [ ] README が実装済み・実 provider 検証済みの現状を反映する
- [ ] provider 設定手順（Ollama / `models.json` / `models.yaml` / `reasoning_effort` / `response_format` の使い分け）が文書化される
- [ ] 秘密情報・環境依存パスを含まない

## Verification

- [ ] `docs/guides/providers.md` の手順で `models.json`/`models.yaml` が再現できる記述になっている
- [ ] `mise exec -- pnpm -r build` が green（文書のみの変更でも最終確認）
