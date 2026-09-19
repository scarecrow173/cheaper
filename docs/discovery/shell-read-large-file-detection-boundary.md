---
id: "DISC-034rBfZ3dqNmEQGXCIwiLn"
type: "discovery"
status: "resolved"
title: "Shell-read large-file detection boundary and parser scope"
created: "2026-09-20"
updated: "2026-09-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/42"
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
    - "./cross-agent-native-routing-adapter-contract-re-verification.md"
    - "./huge-file-token-count-guard-and-fail-closed-hook-behavior.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/shell-read-large-file-blocking.md"
    - "../adr/conservative-shell-read-candidate-extraction-inside-the-shared-routing-policy.md"
  verifies: []
  verified-by: []
  references:
    - "../adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "../adr/default-large-read-threshold.md"
    - "../adr/treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md"
    - "../designs/cross-agent-native-routing-adapter-architecture.md"
  defers: []
  deferred-by: []
metadata:
  discovery:
    question: "issue #42 の shell 経由巨大 file read 検出を、#43 で敷設した canonical shell seam にどの parser 境界で接続するか"
    scope: "cat/head/tail/less/more の保守的 read candidate 抽出、per-target shell hook 到達状況の棚卸し、fail-open/fail-closed contract の維持"
    outcome: "抽出器は routing-policy.js の evaluateShell に置き、抽出した各 path を既存 evaluateFileRead に流す構造が最適。全 target の adapter は #43 で既に canonical shell request を deliver 済みで、adapter 変更は不要"
---

# Shell-read large-file detection boundary and parser scope

## Exploration Goal

issue #42「Shell 経由の巨大ファイル読み込みを pre-read hook で検出・block
する」の実現方法を定める。親 Coding Agent は `cat` / `head` / `tail` /
`less` / `more` 等の shell command で `pre_read_code` hook を迂回し巨大
file を context に取り込める。#43 で全 target の adapter が canonical
`{kind:"shell"}` request を shared routing policy へ deliver する seam
（`routing-policy.js` の `evaluateShell`、現在 allow-only）が敷設済みなので、
本 issue の論点は「shell command 文字列から read 対象 file をどこまで
保守的に抽出するか」の parser 境界に絞られる。

## Key Issues and Assumptions

- parser は shell を実行・展開してはならない（glob・command substitution・
  変数展開の評価は禁止）。tokenize 結果だけから read candidate を得る。
- option と file operand の区別が誤判定の主要因。`-n 100` や `-100` の
  ような head/tail 固有の行数指定を path と誤認しないこと。
- pipe (`|`)・`&&`・`;` 等で連結された複合 command の各 segment を独立に
  評価する必要がある（`cd src && cat huge.ts`、`cat a.ts | grep foo`）。
- 安全に read target を特定できない構文（subshell・heredoc・redirect だけで
  読む command 等）は抽出を諦めて fail-open。誤 block より検出漏れを選ぶ
  conservative 設計が issue の要件。
- 判定本体（workspace 境界・byte-size fast path・count-tokens・threshold・
  FILE_TOO_LARGE fail-closed）は `evaluateFileRead` に既に存在する。
  重複実装せず再利用することが issue の実装方針。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| shared policy 内の保守的 tokenizer + evaluateFileRead 再利用 | 判定の単一 source of truth、adapter 無変更、fail-open が自然 | 独自 tokenizer の保守コスト | yes |
| 本格 shell parser（AST）の導入/自作 | 複合構文を網羅 | 依存追加・issue の非目標に抵触、過剰 | no |
| adapter 層で per-target に command を parse | target 差異を局所化 | 判定ロジックが target 数だけ重複し drift する | no |
| 正規表現だけで file 名を抽出 | 実装が最小 | quote・option・複合 command に脆弱、誤 block 多発 | no |

## Tentative Conclusions and Hypotheses

### Confirmed

- 全 target adapter（Windsurf `pre_run_command`、Claude/Codex
  `PreToolUse` matcher `Bash|PowerShell`、Cursor `beforeShellExecution`、
  Copilot `preToolUse` matcher `bash|powershell|shell`）は #43 で
  canonical `{kind:"shell", command, cwd}` request を deliver 済み。
  本 issue で adapter・hooks.json を変更する必要はない（confirmed:
  `packages/apm/.apm/hooks/*.js`・`hooks.json` 実読）。
- `evaluateShell` が唯一の接続点であり、そこに抽出器を置けば全 target に
  同一 policy が適用される（confirmed: `routing-policy.js` L135-138 の
  seam コメントが本 issue を明示）。
- shell command から抽出した file も `evaluateFileRead` を通せば、
  workspace 外・不存在・directory は allow、threshold 超過と
  `FILE_TOO_LARGE` は deny となり、issue の verdict 要件をそのまま
  満たす（confirmed: 既存実装の contract）。

### Hypothesis

- tokenize は「空白分割 + 単一/二重 quote + `|`・`&&`・`||`・`;` の
  segment 分割 + 先頭 token が対象 command 名かの判定 + 既知 option の
  skip」程度で十分。`$(`、` `、`>`、`<`、glob 文字を含む token 列は
  安全に解釈できないためその segment を skip して fail-open とする
  保守設計で要件を満たせる（hypothesis: test-spec で代表例を固定して検証）。
- deny reason は file-read のものを流用しつつ block された command を
  先頭に添える形で、issue 要件 7 の diagnostics（command・file・token /
  FILE_TOO_LARGE・threshold・bulk-read 誘導）を満たせる（hypothesis）。

## Open Questions

- `tail -f` / `less +F` のような follow mode は read というより監視だが、
  巨大 file を開く点では同じ。対象 command として扱うかは spec で
  明文化する（block 対象とする方向）。
- `head -c`、`tail -c` の byte 指定で file の一部だけ読む場合も、
  判定は file 全体の token 数で行う（既存 policy の粒度のまま）。

## Promotion Candidates

- [x] spec-doc needed: shell 経由 large-read 検出の要件（対象 command、
      抽出規則、fail-open/fail-closed verdict、target 別到達状況）
- [x] adr-doc needed: 保守的 tokenizer を shared policy 内に置き、
      adapter と判定本体を分離する構造決定

## Sources and Relations

- Issue #42: 要件の正。
- DISC-034rA43JiB611pUjLPA4yR（#43 contract re-verification）: 各 target
  の shell hook 到達可否と APM pass-through の実測結果。
- ADR-034rzTJYDRmre6D5OS8HDW: canonical request + per-target adapter の
  構造決定。本 issue はその `shell` request kind の policy 側を埋める。
