---
id: "IMPL-0017"
type: "impl"
status: "completed"
title: "Delegation usage measurement log (PLAN-0022)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/cli/src/usage-log.ts"
      - type: "file"
        path: "packages/cli/test/usage-log.test.ts"
    modified:
      - type: "file"
        path: "packages/cli/src/cli.ts"
      - type: "file"
        path: "packages/cli/test/e2e/exec.test.ts"
      - type: "file"
        path: "packages/cli/test/e2e/real-provider-ollama.test.ts"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "packages/apm/.apm/hooks/cli/bin/cheaper.mjs"
  implements:
    - "docs/tasks/0110-implement-usage-log-writer-and-cli-wiring.md"
    - "docs/tasks/0111-usage-log-e2e-docs-and-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0022-delegation-usage-measurement-log.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0027-delegation-usage-measurement-log.md"
    - "docs/specs/0022-delegation-usage-measurement-log.md"
    - "docs/adr/0025-usage-jsonl-in-user-config-dir-best-effort-append.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0022-delegation-usage-measurement-log.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Delegation usage measurement log (PLAN-0022)

## Summary

`cheaper run`（worker alias 含む）と `count-tokens` の実行ごとに、
usage 実測・事前見積を `<userConfigDir>/usage.jsonl` へ 1 行
JSONL で継続記録する仕組みを実装した。記録は best-effort で、
書き込み失敗は exit code・stdout contract を変えない。delegation
単位の削減効果（事前見積 vs 実測）を parent agent が JSONL から
参照・集計できる形になった。

## Context

TASK-0110/0111（PLAN-0022 / SPEC-0022 / ADR-0025 / DESIGN-0027）
に従う。Issue #13。SPEC-0021 の real-provider e2e が `meta.usage`
の実測 assert 経路を確立済みのため、本実装では log への append
経路と実測 record の assert を追加した。

## Implementation

### Changes

- `packages/cli/src/usage-log.ts`（新規）: `UsageLogRecord`（
  `schemaVersion:1`・`ts`・`kind:"run"|"count"`・`worker`・
  `root`・`ok`・`durationMs` + kind 別 field）、
  `buildUsageRecord`（CliEnvelope → record、対象外は null）、
  `appendUsageLog`（`mkdir -p` + 1 行 appendFile）、
  `recordUsage`（build→append を try/catch、失敗は quiet 以外で
  stderr `usage-log:` 1 行のみ — swallow をここに集約）
- `packages/cli/src/cli.ts`: `executeRun` に `onRunContext`
  callback を追加し canonical root・userConfigDir を runCli へ
  伝搬。run success・run error（catch）・count-tokens の 3 箇所で
  envelope 書出し後に `recordUsage` を呼ぶ。`resolvedUserConfigDir`
  fallback は executeRun が dirs 解決前に throw した場合用
- `packages/cli/test/usage-log.test.ts`（新規）: record shape・
  append・swallow/quiet の 8 件
- `packages/cli/test/e2e/exec.test.ts`: fake-stream run + count-
  tokens で `CHEAPER_CONFIG_DIR/usage.jsonl` への append assert
- `packages/cli/test/e2e/real-provider-ollama.test.ts`:
  `expectUsageRecord(worker)` を追加し、bundled CLI が
  `kind:"run"`・provider `ollama`・`usage.input>0` の record を
  書いたことを assert
- `AGENTS.md`: 「delegation 計測ログ」節を追加
- `packages/apm/.apm/hooks/cli/bin/cheaper.mjs`: bundle 再生成

### Decisions During Implementation

- **記録の choke point は runCli**: envelope 確定・stdout 書出しの
  直後に記録し、success・error・count の 3 経路をカバー。
  auth・help・version は呼ばない（計測対象外）。
- **swallow は `recordUsage` 内に集約**: 呼出し側を 1 行に保ち、
  「失敗しても exit/stdout 不変」を unit test で直接検証可能に。
- **`onRunContext` callback**: `workerName` と同じ out-param 方式
  で error path（executeRun throw）でも root/userConfigDir を
  捕捉。dirs 解決前の throw 用に `resolvedUserConfigDir` fallback
  を用意（resolveConfigDirs は純粋関数）。
- **error envelope は usage ありのみ記録**: `meta.usage===null`
  （worker 解決失敗等）は record を作らない — usage の無い
  delegation は計測対象外（SPEC-0022 の「usage 付き error」）。
- **IMPL 番号**: in-flight で 0010（#8）・0013（#14）・0014
  （#11）・0015（#10）・0016（#12）が使用済みのため 0017。

## Validation

- `vitest run test/usage-log.test.ts`: 8/8 pass（run/count/error
  record shape・append 追記・書込不可 dir での swallow・quiet
  suppress）
- fake-stream e2e: `CHEAPER_CONFIG_DIR` 隔離下で run・count
  record の append を assert — pass
- real-provider e2e（live Ollama・bundled CLI）: 3 worker で
  `kind:"run"`・`provider==="ollama"`・`usage.input>0` の record
  を assert — 3/3 pass（約 41s）
- `mise run check` exit 0: lint（warning 4 件、全て既存）→
  typecheck → test（core 270 + runtime-pi 255 + cli 143 = 668
  tests 全 pass）→ bundle freshness clean
- `node .agents/skills/doc-status/scripts/audit_docs.js --type all`:
  no findings

## Risks

- `usage.jsonl` は rotation 無しで無限に伸びる（1 行/run・数百
  byte）。長期運用では rotation・集計の follow-up が必要。
- 複数 process の同時 append は行単位では実害が小さいが、破損行が
  混じる可能性は残る（consumer 側で per-line parse skip 可能）。
- `mkdir -p` が `userConfigDir` を副作用として作る — config と
  同じ canonical 場所のため許容。
- record の `root` に workspace abs path が残る — user 自身の
  config dir 内のみで外部送信は無い。

## Follow-ups

- usage 集計・summary コマンド（JSONL を読んで削減効果を可視化
  する utility。要件が具体化してから）
- log rotation / retention
- Issue #15: README の user journey 再構成（計測 log の案内を
  どこに置くかは IA 再構成と一緒に検討）
