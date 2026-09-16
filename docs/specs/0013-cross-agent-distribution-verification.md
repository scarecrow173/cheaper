---
id: "SPEC-0013"
type: "spec"
status: "proposed"
title: "Cross-agent distribution verification"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./0008-apm-delegation-skills-and-read-hook.md"
    - "./0009-apm-self-contained-cli-distribution.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/0005-windsurf-cascade-hooks-and-apm-hook-deployment.md"
    - "../designs/0013-apm-skills-and-pre-read-hook.md"
    - "../adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "must"
---

# SPEC-0013: Cross-agent distribution verification

## What Should Be Built

cheaper の APM 配布について、`apm.yml` に宣言された全 target（copilot、
codex、claude、cursor、windsurf、agent-skills）で「`apm install` が何を
deploy し、生成された hook config が各 agent で機能するか / no-op か」
を検証済みの証拠として確立する。本 spec は verification feature で
あり、成果物は per-agent の機能適応ではなく、test と canonical contract
matrix という証拠である。

- 全 declared target を対象にした `apm install` smoke/verification:
  deploy される file layout、生成 config の存在と shape、bundled CLI
  （`cheaper.mjs`）の実行可能性（`--version`、`count-tokens`）
- canonical contract matrix の文書化: target ごとに、認識される event
  名、deploy された `pre_read_code` config が no-op かどうか、windsurf
  contract との stdin/stdout/blocking の差分、機能的な pre-read
  blocking の実現可能性
- per-target config validation: 各 agent の hook config loader が未知の
  `pre_read_code` event を error なく accept/ignore するか（自動化可能な
  範囲は test で、それ以外は公式 doc・config validation による証拠）
- windsurf 経路の regression-free 維持（既存 test の結果は不変）

## Why Now

SPEC-0008/0009 で hook bundle と self-contained CLI の配布を実装したが、
e2e 検証は windsurf target のみ（`packages/cli/test/e2e/apm-install.test.ts`）
で、全 generated config は Windsurf native event `pre_read_code` をそのまま
登録している。DISC-0005 は non-windsurf target の deploy 挙動を
「Phase 4/5 で確認」へ延期し、DESIGN-0013 も「未知 event は no-op」の
想定を Phase 5 の実機確認待ちとしていた。

外部 contract 調査（DISC-0009）により、`pre_read_code` が Claude Code
（`PreToolUse` + matcher `Read`）、Cursor（`beforeReadFile`）、Copilot
（`view` toolName の `preToolUse`）には存在せず、deploy された config は
それらでは no-op（無害だが非機能）になること、Codex CLI の hook は file
read を intercept できないこと（upstream issue openai/codex#20204）、
Copilot の command hook は fail-closed（非 zero exit が deny）、Cursor は
`failClosed: true` を付けない限り fail-open であることが判明した。
この Phase 5 延期を閉じ、宣言全 target の配布結果を検証済みの記録として
固定する。

## Users and Value

- cheaper maintainer: 各 target で hook が機能するか / no-op かを
  canonical matrix で把握でき、後続の adapter 実装の優先度付けに使える
- parent agent / consumer: 自分の agent で cheaper の pre-read
  delegation が動くかどうかを文書で確認できる
- ユーザー: no-op のまま「hook が動いている」と誤認する silent
  failure を防ぐ

## Scope

### In Scope

- 全 declared target（copilot、codex、claude、cursor、windsurf、
  agent-skills）の `apm install` smoke/verification — deploy file
  layout、生成 config の存在/shape、bundled CLI の `--version` /
  `count-tokens` 実行
- canonical contract matrix の文書化 — recognized event 名、deploy
  config の no-op 判定、stdin/stdout/blocking の windsurf contract との
  delta、pre-read blocking の実現可能性
- per-target config validation — 未知 `pre_read_code` event を各 agent
  の hook config loader が error なく accept/ignore するかの証拠
- windsurf path の regression-free 維持（既存 test の outcome 不変）

### Out of Scope

- per-target hook contract adapter の実装（event-name mapping、
  stdin/stdout translation、fail-closed semantics への対応）— 本検証の
  matrix を入力とする後続 chain で扱う
- 実 agent 駆動 e2e（Claude Code / Cursor / Copilot 等の実 binary の
  実行）— 検証は install/config/CLI レベルと公式 doc の contract 証拠に
  留める
- `apm.yml` の target 一覧の変更

## Requirements

- 検証は宣言された全 target を網羅し、target ごとの結果を個別に記録する
- contract matrix は repo 内の canonical 文書 1 箇所に集約し、同じ
  contract 知識を複数箇所に分散させない
- matrix には Codex の upstream coverage gap（openai/codex#20204）、
  Copilot の fail-closed semantics（非 zero exit が deny）、Cursor の
  fail-open default を明記する
- 自動化可能な検証（install layout、生成 config schema、CLI 実行）は
  test 化し、自動化できない contract 確認は文書化された手順と公式 doc
  引用の証拠とする
- 既存 windsurf e2e は変更せず、結果を維持する

## Acceptance Criteria

- [ ] 全 declared target について、`apm install` が deploy する内容
      （hook bundle file + 生成 config の path/schema）を証明する
      automated test または文書化された手順が存在する
- [ ] 全 declared target について、deploy された hook config が
      functional か no-op か、およびその理由（未認識 event 名 /
      非対応の tool coverage）が canonical record に記載される
- [ ] contract matrix が各 target の native pre-read / pre-tool-use
      event、stdin shape、blocking mechanism を記録し、Codex の
      upstream coverage gap と Copilot の fail-closed semantics を含む
- [ ] 既存 windsurf e2e（`apm-install.test.ts`、`pre-read-hook.test.ts`、
      `delegation-loop.test.ts`）が引き続き pass する
- [ ] 各 target が functional / deploys-but-noop / unsupported のいずれか
      に分類され、後続判断の入力となる

## Deferred Design Concerns

- per-target adapter 実装（event-name mapping・stdin/stdout translation）
  | 本 spec は verification のみ | matrix 確定後の後続 spec で実施 |
  放置すると non-windsurf target の hook は no-op のまま
- 実 agent 駆動 e2e | 実 binary の入手・実行環境が重い | CI 環境整備後 |
  実 agent 側の contract drift を検出できない

## Sources and Relations

- DISC-0009: cross-agent hook distribution と contract matrix の外部調査
  （source / derives-from）
- DISC-0005: APM hook deploy 形式の実測（windsurf）と Phase 4/5 延期
- DESIGN-0013: hook/skill 設計と「未知 event は no-op」の仮定
- SPEC-0008: hook/skill 本体、SPEC-0009: self-contained CLI 配布
  （depends-on — 検証対象の配布実装）
- ADR-0016: non-windsurf target での verbatim `pre_read_code` deploy を
  documented no-op として維持し per-target adapter を延期する決定
