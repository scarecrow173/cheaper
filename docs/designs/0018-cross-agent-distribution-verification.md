---
id: "DESIGN-0018"
type: "design"
status: "approved"
title: "Cross-agent distribution verification"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0013-cross-agent-distribution-verification.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0013-cross-agent-distribution-verification.md"
  implemented-by: []
  depends-on:
    - "./0013-apm-skills-and-pre-read-hook.md"
    - "./0014-cli-bundle-layout.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md"
    - "../discovery/0005-windsurf-cascade-hooks-and-apm-hook-deployment.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  derived-by: []
  verifies:
    - "../adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  verified-by: []
  references:
    - "https://github.com/openai/codex/issues/20204"
  defers: []
  deferred-by: []
metadata: {}
---

# DESIGN-0018: Cross-agent distribution verification

## Context

SPEC-0013 / ADR-0016 の実装設計。本 design は **verification feature**
であり、成果物は per-target の機能適応ではなく test と canonical
contract matrix という証拠である。

現状: `apm.yml` は 6 target（`copilot`, `codex`, `claude`, `cursor`,
`windsurf`, `agent-skills`）を宣言し、`apm install` は `.apm/hooks/`
（`hooks.json` + `pre-read-code.js` + `cli/` bundle）と `.apm/skills/`
を全 hook 対象 target へ deploy する。生成される per-target hook config
はすべて Windsurf native event `pre_read_code` を verbatim で登録して
おり、DISC-0009 の調査により non-windsurf target では **documented
no-op** になることが確定済み（documented contract ベースの分類で
runtime loader の実機挙動は未検証。ADR-0016 で維持決定）。e2e
coverage は 2 file: `packages/cli/test/e2e/apm-install.test.ts`
（windsurf の install + hook runtime 挙動）と
`packages/cli/test/e2e/apm-install-targets.test.ts`（全 hook
target + agent-skills の table-driven smoke。windsurf の
generated config 完全性はこちらでカバー）。

## Scope

### In Scope

- `packages/cli/test/e2e/apm-fixture.ts` の target パラメータ化と、全
  declared target を網羅する table-driven verification test の追加
- per-target assertion: deploy file layout、generated config の存在と
  parse 可能な shape、bundled `cheaper.mjs` の `--version` /
  `count-tokens` 実行、登録 event name が `pre_read_code` verbatim
  であることの記録（no-op の文書化）
- contract matrix の canonical 化: DISC-0009 を single source of truth
  として最終分類を記録し、consumer 向け文書からは参照のみを張る
- windsurf 経路の regression-free 維持（既存 test の無変更）

### Out of Scope

- per-target hook adapter の実装（event-name mapping、stdin/stdout
  translation、fail-closed semantics 対応）— ADR-0016 で defer 確定
- 実 agent 駆動 e2e（Claude Code / Cursor / Copilot 等の実 binary 実行）
- `apm.yml` の target 一覧・`.apm/` 配布物の変更
- 既存 `apm-install.test.ts` / `pre-read-hook.test.ts` /
  `delegation-loop.test.ts` の変更（windsurf の結果は不変）

## Components and Boundaries

### 1. Fixture の target パラメータ化（`apm-fixture.ts`）

選択: **option (a) — 既存 helper を optional `target` パラメータで拡張し、
新規 table-driven test file で網羅する**。既存 file 無変更の要件と
重複排除を両立させるため、fixture 拡張は default 値 `"windsurf"` で
後方互換にし、table は新規 file に置く。

- `makeConsumerLayout(root, target = "windsurf")`: consumer `apm.yml`
  の `targets:` を `- <target>` で生成（pkg 側は不変）
- `apmInstall(consumer, home, target = "windsurf")`: `apm install
  --target <target>` を spawn
- `bundleRoot(consumer, target = "windsurf")`: target → deploy dir map
- `generatedConfigPaths(consumer, target)`: target → 生成 config path
  一覧（windsurf / claude / codex / cursor は 2 file、agent-skills
  は空）

棄却: option (b)（per-target test file）は、差分が (target, deploy
path, config path) のみで fixture boilerplate が 6 重複するため。
既存 `apm-install.test.ts` 自体を table-drive に書き換える案も棄却 —
spec の「既存 windsurf e2e は変更せず」を文字通り保てない。

target map（consumer fixture では dep dir 名 = `pkg`）:

| target | bundleRoot | generated config |
| --- | --- | --- |
| `windsurf` | `.windsurf/hooks/pkg` | `.windsurf/hooks.json` + `.windsurf/apm-hooks.json` |
| `claude` | `.claude/hooks/pkg` | `.claude/apm-hooks.json` + `.claude/settings.json` |
| `codex` | `.codex/hooks/pkg` | `.codex/hooks.json` + `.codex/apm-hooks.json` |
| `cursor` | `.cursor/hooks/pkg` | `.cursor/hooks.json` + `.cursor/apm-hooks.json` |
| `copilot` | `.github/hooks/scripts/pkg` | `.github/hooks/<dep>-hooks.json` |
| `agent-skills` | （hook deploy なし） | （なし） |

### 2. Test file

新規 `packages/cli/test/e2e/apm-install-targets.test.ts`。table は
hook 対象 5 target（`windsurf`, `copilot`, `codex`, `claude`,
`cursor`）+ `agent-skills` を回す。windsurf は既存 file が
hooks.json + hook runtime 挙動を網羅済みだが、generated config の
完全性（`apm-hooks.json` を含む）は既存 file の対象外のため本
table の windsurf case でカバーする。`apmAvailable()` による
skip は既存パターン踏襲。

### 3. Per-target assertions

hook 対象 5 target 共通:

- `install.status === 0`
- bundle layout が存在: `<bundleRoot>/pre-read-code.js`,
  `cli/bin/cheaper.mjs`, `cli/workers/`, `cli/policy.yaml`
- generated config が agent-correct path に存在し JSON parse 可能
- config 内の登録 event key が **`pre_read_code` verbatim** であること
  を assert — deploy の証明であると同時に、「この event は
  non-windsurf では発火しない = documented no-op」を test 上の記録と
  して固定する
- bundled CLI が consumer workspace で動く:
  `node <cli> count-tokens <file> --cwd <consumer>` → exit 0

target 固有:

- `cursor` / `copilot`: hooks.json 系 config の `version === 1`
- `claude`: `.claude/apm-hooks.json` と `.claude/settings.json` の双方が
  存在し、settings.json が nested `hooks → event → matcher → hooks`
  shape で `pre_read_code` を verbatim 含む（`${CLAUDE_PROJECT_DIR}`
  補間はそのまま残る）
- `windsurf` / `codex` / `cursor`: `hooks.json` に加え、同じ
  `pre_read_code` 登録を top-level event key shape で持つ
  `apm-hooks.json` も生成される（apm 0.28.0 実測）
- `copilot`: `.github/hooks/` 配下の `*-hooks.json` は file 名が dep
  名由来のため hardcode せず `readdirSync` で一意に解決する
- `copilot` の version / schema 欠落（現状の観測挙動の記録であって
  ideal layout ではない）: APM 0.28.0 は copilot の scripts bundle
  からすべての `.json` を除外するため `cli/version.json`・
  `cli/package.json`・`cli/workers/schemas/*.json` が deploy されない
  （`cli/package.json` は APM が hook bundle descriptor として deploy
  対象外にするため、version の sidecar は `version.json` —
  `scripts/bundle-cli.mjs` が生成）。このため `--version` は exit 70
  （"failed to resolve package version"）で失敗し、file-schema
  worker は schema 欠落で `OUTPUT_SCHEMA_NOT_FOUND`（exit 3）に
  なる — 実装はこれを assert として記録する。non-copilot target
  では `cli/version.json` 存在 + `--version` → exit 0 +
  `/^\d+\.\d+\.\d+$/` を引き続き assert する
- `agent-skills`: `.agents/skills/` 配下に 4 skill の `SKILL.md` が
  deploy され、hook bundle / hook config が **存在しない** ことを
  assert（skills-only deploy が正しい挙動）

### 4. Contract matrix の canonical 化

- canonical location は **DISC-0009 のみ**。同 doc の per-target matrix
  に最終分類（windsurf = functional / claude・cursor・copilot =
  deploys-but-nonfunctional (expected no-op) / codex =
  deploys-but-nonfunctional (expected no-op) かつ unsupported
  （openai/codex#20204 の coverage gap）/ agent-skills = N/A
  skills-only）と本 test への evidence 参照を追記して close する。
  「expected no-op」は documented contract ベースの分類であり、
  runtime loader の accept/ignore 挙動は実機 E2E 未検証である旨を
  matrix 注記として添える
- consumer 向け文書には pointer のみ: `.apm/skills/cheaper-usage/
  SKILL.md` の "Locating the CLI" 節に 1 文「pre-read hook は現状
  Windsurf でのみ機能する。per-target contract は DISC-0009 の
  canonical matrix を参照」を追加する
- matrix 内容を README・skill・spec 等へ複写しない（DRY — contract
  知識の分散を spec requirement として禁止されている）

## Data and Control Flow

1. `mkdtempSync` → `makeConsumerLayout(root, target)`（repo `.apm/` を
   minimal pkg に copy + consumer `apm.yml` 生成）
2. `apmInstall(consumer, root, target)` → exit 0 assert
3. target map に従い bundle layout・generated config・event key・
   bundled CLI を assert
4. per-target の pass/fail が個別の `it` として記録される

## Non-Functional Constraints

- 既存 e2e file 無変更（windsurf regression）。fixture 拡張は default
  引数で後方互換
- 新規 dependency なし。実行は `pnpm --filter cheaper test`、apm CLI
  不在環境では `apmAvailable()` で skip
- **Copilot fail-closed caveat（将来 adapter の不変条件として固定）**:
  現状の verbatim deploy が安全なのは `pre_read_code` が発火しない
  ため。Copilot の command `preToolUse` hook は fail-closed であり、
  将来の adapter は「意図的 block 以外で絶対に non-zero exit /
  `permissionDecision: "deny"` を返さない」を保証しなければならない
  （fail-open 前提の現行 contract が Copilot では反転する）。本
  verification は generated config の静的検証であり hook を Copilot
  event に結線しないため、このリスクを test に持ち込まない
- config assertion は「存在 + JSON parse + event key」レベルに留め、
  apm の生成 schema 内部への過剰結合を避ける

## Risks and Trade-offs

- 実 agent での no-op 確認は install/config/CLI 層の test と公式 doc
  証拠（DISC-0009）の組合せに留まる。agent 側の contract drift は実
  機 e2e なしでは検出できない（spec が許容済みの trade-off）
- `agent-skills` の「hook が deploy されない」assertion は apm 側の
  挙動変化（例: 空の hooks dir 生成）で壊れ得るため、存在しないこと
  の assert は generated config file と bundle dir に限定する
- fixture の default `"windsurf"` は call site を守るための互換層
  であり、新規 code からは必ず明示 target を渡す
- APM version は unpinned（mise 経由で `latest`）。copilot の
  expected-failure assert（`.json` 欠落・`--version` exit 70・
  worker `OUTPUT_SCHEMA_NOT_FOUND`）は APM が `.json` stripping を
  修正した時点で fail し始める — test 破壊ではなく contract drift
  の検出 signal として扱う

## Deferred Design Concerns

- per-target hook adapter（event-name mapping・stdin/stdout
  translation）| ADR-0016 で defer 確定 | matrix の検証済み contract
  を入力とする per-target の独立した ADR/spec chain | 放置すると
  non-windsurf hook は no-op のまま
- 実 agent 駆動 e2e | 実 binary の入手・実行環境が重い | CI 環境整備後
  | agent 側の contract drift を検出できない
- `.windsurf/hooks.json` の deployment drift（powershell の
  `exit $LASTEXITCODE` 欠落、DISC-0009 open question）| 本検証は
  consumer への fresh install を対象とし dogfood 環境の drift 是正は
  対象外 | 別途是正 | dogfood 環境が stale なまま残る

## Verification Notes

- [ ] `pnpm --filter cheaper test` で新規 table-driven e2e が全
      target pass（apm 不在環境では skip）
- [ ] 既存 `apm-install.test.ts` / `pre-read-hook.test.ts` /
      `delegation-loop.test.ts` が無変更で pass（windsurf regression
      なし）
- [ ] DISC-0009 matrix に最終分類（functional /
      deploys-but-nonfunctional (expected no-op) / unsupported /
      N/A）と Codex coverage gap・Copilot fail-closed semantics が
      記録されている
- [ ] `cheaper-usage` SKILL.md に matrix への pointer があり、
      contract 知識の複写が存在しない
