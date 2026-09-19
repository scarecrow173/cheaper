---
id: "TSPEC-034rBrs4K5W7pe0ixjegZk"
type: "test-spec"
status: "approved"
title: "Shell-read blocking contract tests"
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
    - "docs/tasks/shell-read-candidate-extractor-in-routing-policy.md"
    - "docs/tasks/shell-read-contract-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/shell-read-candidate-extraction-implementation.md"
  derived-by: []
  verifies:
    - "docs/specs/shell-read-large-file-blocking.md"
    - "docs/designs/shell-read-candidate-extraction.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  test-spec:
    level: "e2e"
---

# Shell-read blocking contract tests

## Purpose

issue #42 の完了条件「shell 経由の対象 command に large-read detection が
働き、threshold 超過 / `FILE_TOO_LARGE` は block、infrastructure / parse
failure は fail-open、通常 read と同一判定を共有する」を pin する。
`evaluateShell` の保守的 parser が誤 block・誤 pass しないことを回帰
として固定する。

## Feature

shell hook payload の `command` 文字列から `cat`/`head`/`tail`/`less`/
`more` の file operand を抽出し、抽出した各 file を canonical
large-read 判定（`evaluateFileRead`）へ流して allow/deny を返す。

## Rules

- Rule: `cat`/`head`/`tail`/`less`/`more` で threshold 超過 file を開く
  command は deny。各 target adapter の native deny 表現（windsurf
  exit 2+stderr、pretooluse `hookSpecificOutput` deny、cursor
  `{permission:"deny"}`、copilot `permissionDecision:"deny"`）で返る。
- Rule: `-n 200`・`-100`・`-n +5` 等の option token は file operand と
  誤認しない。`--` 以降は operand。
- Rule: 複数 operand のうち 1 つでも deny なら command 全体を deny。
- Rule: 対象外 command・operand なし（`cat` 単独）・pipe 後段・
  解釈不能構文（`$`・redirect・glob・未閉鎖 quote）・file 不存在・
  directory・workspace 外 path・malformed payload・CLI 起動失敗・
  `count-tokens` output parse 失敗は全て allow（fail-open）。
- Rule: `FILE_TOO_LARGE` envelope は fail-closed で deny。
- Rule: `CHEAPER_READ_TOKEN_THRESHOLD` override は shell-read にも適用
  され、file-read と同一の threshold で判定される。
- Rule: deny diagnostics は block された command・対象 file・token 数
  または `FILE_TOO_LARGE`・threshold・`bulk-read` 誘導を含む。

## Examples

- Given `cat big.txt`（big.txt が threshold 超過）→ deny、reason に
  command と `workspace-relative path: big.txt`・`bulk-read` を含む。
- Given `head -n 200 big.txt` / `tail -100 big.txt` / `less big.txt` /
  `more big.txt` → いずれも deny。
- Given `cat small.txt`（threshold 内）→ allow。
- Given `cat small.txt big.txt`（片方のみ巨大）→ deny。
- Given `cat big.txt | grep foo` → deny（前段の read が対象）。
  Given `grep foo small.txt | head` → allow（head の operand なし）。
- Given `cat "path with spaces/big.txt"` → quoted path を1 operand として
  復元し deny。
- Given `cd src && cat big.txt`（big.txt が hook cwd 直下に存在）→
  deny（cwd 追跡しない既知制限の挙動を固定）。
- Given `cat $(ls)` / `cat *.txt` / `cat ~/secret` 等の解釈不能構文 →
  allow（fail-open）。
- Given file-read `{kind:"file-read"}` と shell `cat <same file>` → 同一
  verdict（regression で共有判定を保証）。
- Given `CHEAPER_CLI` が不在の binary path → allow（fail-open）。
- Given `count-tokens` が `FILE_TOO_LARGE` envelope を返す file（byte cap
  超過）への `cat` → deny。

## Guarantees

- shell 側に threshold・`count-tokens`・`FILE_TOO_LARGE` 解釈の複製が
  ない（判定は `evaluateFileRead` に集約）。
- hook 自体の不具合で通常の shell 利用が壊れない（fail-open）。
- adapter / `hooks.json` は無変更のまま全 target で同一挙動。

## Non-goals

- `sed`/`awk`/`python` 等の任意 code 経由 read の検出。
- 実 agent binary での runtime E2E。
- `cd` / `pushd` による cwd 追跡。

## Risk

- この test spec を外すと、shell parser の誤 block（通常 ops 破壊）や
  fail-open/fail-closed contract の drift を検出できず、large-read
  routing の強制が再び不完全になる。
