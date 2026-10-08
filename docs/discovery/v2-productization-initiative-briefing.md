---
id: "DISC-034r05iHy2gdnp7GkO7QDK"
type: "discovery"
status: "resolved"
title: "V2 productization initiative briefing: worker restructure, token counting, APM delegation"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/cheaper-v1-worker-runtime-cli.md"
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  refines: []
  refined-by: []
  derives-from:
    - "../impl/exp/ollama-real-provider-smoke-verification.jsonl"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# V2 productization initiative briefing

## Exploration Goal

親 Coding Agent から安価なモデルへ処理を委譲するという本来目的を実運用で達成するため、
最新 `main`（`4206fd0`）の現状と最終指示との gap を確定し、Phase 分割と主要判断を記録する。

## Current State (verified)

- pnpm モノレポ: `@cheaper/core` / `@cheaper/runtime-pi` / `cheaper`（CLI）
- built-in Worker: `bulk-reader` + `cheap-writer`
- 検証 baseline green: `pnpm -r build` / `pnpm -r test` / `pnpm lint` / `pnpm typecheck`
- V1 機構（Worker Resolver・Policy・Tool Vocabulary・Model Resolver・Pi session・
  sandboxed tools・journal → `meta.changes`・structured output + repair・usage・
  CLI envelope）は全て再利用可能。再実装しない
- Ollama（Docker）による実 provider smoke 済み（`docs/impl/exp/ollama-real-provider-smoke-verification.jsonl`）

## Gap Analysis

| 領域 | gap |
|---|---|
| Workers | `cheap-writer` 残存、`code-writer` / `doc-writer` 未作成 |
| Token counting | Token Counter / `count_tokens` tool / `count-tokens` CLI / threshold 全てなし |
| APM delegation | `cheaper-usage`・3 Worker Skill・Read Hook 全てなし |
| 安全性 | 書込み scope は prompt 命令のみ。workspace 内 secret の read deny なし |
| 配布 | `private: true`・`version 0.0.0`・source build 必須 |
| Economics | benchmark・token threshold の実測根拠なし |
| Provider | Ollama のみ。production-relevant provider 未検証 |
| CI / Release | CI なし、versioned release なし |

## Confirmed Findings

- **tokenizer は依存ツリーに存在しない**: `@earendil-works/pi-ai` /
  `pi-coding-agent` / `pi-agent-core` を全走査。`tokenize` の一致は markdown
  lexer・JSON parser・AWS auth token provider のみで、LLM tokenizer はない。
  新規 dependency が必要
- **APM hook/skill 規約**: `.apm/skills/<name>/`・`.apm/hooks/` が pack/install
  対称の推奨 layout。hook は JSON descriptor + script で、`${PLUGIN_ROOT}` 等の
  path 変数と event 名を target 別に APM が正規化する。hook は LLM を起動せず
  `cheaper count-tokens` を呼ぶだけの構造にできる
- **APM に正式な skill include/dependency 機構は見当たらない**: Worker skill は
  `cheaper-usage` を名前参照して重複を避ける
- **`.claude/skills`・`.agents/skills`・`apm_modules` は gitignore 対象**:
  配布物は git 管理される source（`.apm/` 等）に置き `apm install` で展開する

## User Decisions (confirmed)

| 判断 | 決定 |
|---|---|
| Token Counter の tokenizer | `js-tiktoken`（pure JS・依存ゼロ・MIT。新規 dependency として追加） |
| canonical tokenizer 方針 | provider billing 再現ではなく large-file 判定用の単一の決定的 metric。tokenizer 名/version を結果から観測可能にする |
| CLI distribution | **APM package のみ**（npm registry / GitHub Release 等は作らない） |
| E2E 検証の最初の parent agent | **Windsurf** |
| Ollama 以外の production provider | **OpenAI** |
| APM package の置き場所 | このリポジトリ（`.apm/` に skills/hooks を同梱し CLI と同バージョンで出荷）— エージェント判断 |

## Alternatives and Comparison

- Token Counter 配置: `packages/core` を選択。workspace boundary（`workspace/paths.ts`）
  が既に core にあり、cli・runtime-pi 両方が core に依存するため単一実装を共有できる。
  core の "no pi deps" 不変条件とも整合
- distribution: npm/GitHub Release は却下（ユーザー決定）。APM package 内に
  prebuilt CLI を同梱し、`node ${PLUGIN_ROOT}/...` または package 内 shim で
  `cheaper` 相当を実行可能にする設計が Phase 4 で必要
- Windsurf Read hook: PreToolUse 相当の Read インターセプト可否は未確認。
  不可能なら hook ではなく skill instruction（Read 前に count-tokens を呼ぶ規律）
  に縮退するか、次点 target（Claude Code）を検討する

## Open Investigations

- Windsurf の hook 機構が file Read を事前にインターセプトできるか
- APM package 内で CLI（node スクリプト）を `cheaper` コマンドとして提供する最適形
- Writer の write-scope 機械的制約（invocation-scoped writable paths）と
  workspace 内 secret の read deny の最小機構
- large-file token threshold の実測根拠（benchmark 後に確定）

## Phase Plan (承認済み進行順)

1. built-in Worker 再編（`cheap-writer` 削除 → `code-writer` / `doc-writer`）
2. Canonical Token Counter（core）+ `count_tokens` tool + `count-tokens` CLI
3. APM skills（`cheaper-usage` + 3 Worker skill）+ Read hook
4. APM package 配布・versioning・fresh install smoke
5. Parent → APM → token count → bulk-reader E2E（Windsurf）
6. bulk-reader benchmark → threshold 確定
7. Writer skills + write-scope/privacy hardening + Writer E2E
8. Writer benchmark
9. OpenAI provider E2E
10. CI / cross-platform / release
11. docs / diagnostics / final experiments

## Sources and Relations

- 上流: `docs/impl/exp/ollama-real-provider-smoke-verification.jsonl`（実 provider smoke の完了を受けての製品化指示）
- 下流: SPEC-034r05iHK8bA7gaTnVhXWI 以降の各 Phase の spec/ADR/design/plan/task
