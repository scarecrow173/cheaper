---
id: "DESIGN-034rBoPPFNHXR5eSfv0hsO"
type: "design"
status: "approved"
title: "Shell read-candidate extraction and shared large-read verdict"
created: "2026-09-20"
updated: "2026-09-20"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/shell-read-large-file-detection-boundary.md"
    - "https://github.com/scarecrow173/cheaper/issues/42"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/shell-read-large-file-blocking.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./cross-agent-native-routing-adapter-architecture.md"
    - "./huge-file-byte-cap-guard-and-fail-closed-hook.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/shell-read-large-file-blocking.md"
    - "../adr/conservative-shell-read-candidate-extraction-inside-the-shared-routing-policy.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/shell-read-blocking-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  design:
    kind: "detail"
---

# Shell read-candidate extraction and shared large-read verdict

## Context

`routing-policy.js` の `evaluate` は `kind:"file-read"` を
`evaluateFileRead`（workspace 境界 → stat → byte-size fast path →
`count-tokens` → threshold / `FILE_TOO_LARGE` verdict）で判定し、
`kind:"shell"` は allow-only の seam のままである。本設計は
`evaluateShell` に shell command からの read candidate 抽出を接続し、
抽出した各 file を `evaluateFileRead` と同一の判定経路へ流す。

adapter 層（#43）と `hooks.json` は変更しない。全 target の shell hook
payload は既に canonical `{kind:"shell", command, cwd}` request として
本モジュールに到達する。

## Scope

### In Scope

- `routing-policy.js` 内の shell tokenizer（pure function、shell を
  実行しない）と対象 command の operand 抽出器。
- `evaluateShell` の実装: command → segment 分割 → 対象 command の
  operand 抽出 → 各 operand を `evaluateFileRead` 相当の判定へ。
- deny reason に block 対象 command を含める diagnostics。
- contract test の拡張と `parent-agent-integration.md` の更新。

### Out of Scope

- adapter script / `hooks.json` / bundle pipeline の変更。
- `sed` / `awk` / `perl` / `python` 等の任意 code 経路、完全な shell
  AST 解析、shell 実行の sandbox 化（issue #42 の非目標）。
- `cd` 等による segment 間 cwd 変化の追跡（既知制限）。

## Components and Boundaries

### `routing-policy.js`（変更対象はこの module のみ）

追加する内部要素:

- `tokenizeShell(command)` → `string[][] | null`
  - 入力 command 文字列を segment 配列へ分解する。segment 境界は
    `|`・`||`・`&&`・`;`・改行。各 segment は token 配列。
  - `'` / `"` の quoting を認識し、quote 内の空白・区切りを literal として
    扱う。`\` による escape は quote 外では次の 1 文字を literal 化する。
  - `$`・`` ` ``・`<`・`>`・`*`・`?`・`[`・`]`・`{`・`}`・`~`・`#`・
    `(`,`)` を含む token、未閉鎖 quote、空でない残留状態は「解釈不能」
    としてその token / segment を skip 対象にする（fail-open 側）。
    tokenizer 全体が破綻した場合は `null` を返し allow に倒す。
- `READ_COMMANDS`（table）: 対象 command 名と option 規則。
  - `cat` / `less` / `more`: operand = `-` で始まらない全 token。
  - `head` / `tail`: 上記に加え、値を取る option（`-n`, `-c`, `-q`,
    `-v`, `-z` 等で分離形を取るもの）と `-<数字>` / `-n<num>` 等の
    結合形を option として skip する。
  - `--` は option 終了 marker として以降を全て operand にする。
- `extractReadCandidates(command)` → `string[]`
  - `tokenizeShell` で segment 分割し、先頭 token が対象 command の
    segment から operand を集める。`env`・`sudo`・`command`・`xargs`
    等の wrapper は unwrap しない（先頭が対象 command でなければ skip）。
    `VAR=val` 形式の環境変数 prefix token は skip してよい。
  - `cd` / `chdir` / `pushd` / `popd` / `Set-Location` 等の cwd 遷移
    command が先頭の segment が現れた時点で、それ以降の segment の
    read candidate は実行時 cwd を特定できない ambiguous として
    skip する（fail-open。誤った基準 dir で判定して誤 block /
    誤 pass を起こさない）。
- `evaluateShell(request, opts)`
  - `extractReadCandidates` で得た各 path を `{kind:"file-read", path,
    cwd}` として既存 `evaluateFileRead` へ流す（直接内部呼出し。
    判定ロジックの複製なし）。
  - 全 candidate が allow なら `{decision:"allow"}`。1 つでも deny なら
    その verdict を返すが、`reason` の先頭に
    `cheaper: blocked shell read: '<command>'` 相当の 1 行を付し、
    issue 要件の diagnostics（command / file / token・threshold /
    `FILE_TOO_LARGE` / bulk-read 誘導）を満たす。
  - candidate が 0 件、tokenize 失敗、対象外 command は allow。

### Adapter / hooks.json

変更なし。#43 の contract をそのまま利用する。

## Data and Control Flow

```text
native shell hook payload
  → adapter（既存）→ {kind:"shell", command, cwd}
  → evaluateShell
      tokenizeShell → segments
      segment 先頭が cat/head/tail/less/more → operand 抽出
      解釈不能 segment / 対象外 / operand なし → skip（fail-open）
      each operand → evaluateFileRead（同一判定経路）
        1 deny → deny（command 入り reason）
        all allow → allow
  → adapter が verdict を native response へ変換（既存）
```

## Non-Functional Constraints

- Node builtin のみ・依存追加なし・shell 実行なし（parser は文字列処理のみ）。
- deny guidance に executable command を埋め込まない既存方針を維持する
  （command の echo は引用符付きの表示専用文字列として扱う）。
- `CHEAPER_READ_TOKEN_THRESHOLD` / `CHEAPER_CLI` /
  `CHEAPER_COUNT_MAX_BYTES` contract を維持。

## Risks and Trade-offs

- cwd 遷移の後段: `cd src && cat huge.ts` の `huge.ts` を hook cwd
  基準で解決すると、実際に読まれる `src/huge.ts` ではなく誤った
  file を判定し、誤 block・誤 pass の両方があり得る。そのため
  cwd 遷移 command の後段 segment は ambiguous として fail-open
  に倒す。実体が巨大でも見逃す検出漏れは許容し、誤 block を
  避ける保守設計とする。`cat src/huge.ts` のように path が直接
  特定できる場合は変わらず判定される。
- PowerShell 構文（`cat` alias → `Get-Content` 等）: operand 規則は
  POSIX 寄りだが、tokenize 自体は quote/空白で動くため代表形はカバー
  される。PowerShell 固有構文（`;`・`%` 等）は解釈不能 → fail-open。
- tokenizer の誤判定で通常 command を block するリスク: operand 抽出を
  「先頭が対象 command かつ token が安全文字のみ」に限定し、疑わしきは
  skip する。

## Deferred Design Concerns

- 対象 command set の拡張（`sed -n`・`awk`・`find -exec cat` 等）は
  `READ_COMMANDS` table への追加で対応可能。別 issue。
- `cd` の segment 間追跡・`pushd`/`popd` の stack 解釈。

## Verification Notes

- [ ] `cross-agent-routing-adapters.test.ts` で代表 command の
      allow/deny を stdin 注入で固定する（issue の test リストを網羅）。
- [ ] file-read request と shell-read command が同一 file に対し同一
      verdict を返す regression test がある。
- [ ] `mise run check` が通る。
