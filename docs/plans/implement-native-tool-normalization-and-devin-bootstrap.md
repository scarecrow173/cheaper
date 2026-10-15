---
id: "PLAN-034GtndRTaEqU3lA6Tnquu"
type: "plan"
status: "in-progress"
title: "Implement native tool normalization and Devin bootstrap"
created: "2026-10-15"
updated: "2026-10-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/107"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/case-insensitive-native-tools-and-devin-registration.md"
    - "docs/adr/apm-distributed-devin-native-hook-bootstrap.md"
  implemented-by:
    - "docs/tasks/normalize-tools-and-add-devin-hook-capability.md"
    - "docs/tasks/register-devin-hooks-from-apm-skill.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/case-insensitive-native-tools-and-devin-registration.md"
    - "docs/adr/apm-distributed-devin-native-hook-bootstrap.md"
    - "docs/designs/devin-native-hooks-and-apm-bootstrap.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/native-tool-name-normalization-and-devin-hook-contract.md"
    - "docs/test-specs/apm-distributed-devin-registration-preservation.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    horizon: "single-chain"
---

# Implement native tool normalization and Devin bootstrap Implementation Plan

## Goal

approved spec（SPEC-034GssA0IjLKSiDdIHpAC8）・accepted ADR
（ADR-034GssAufIiId3hgh4jrSQ）・approved design
（DESIGN-034Gt74p3BYG40NTwu2kST）に基づき、（T1）Pretooluse
`tool_name` の ASCII lowercase 正規化 + `grep` の `FileRead` 接続 +
`HookTarget::Devin` + `cheaper hook targets --json` capability
query + 配布 `hooks.json` matcher 拡大と、（T2）APM skill bundle
配布の `install-devin-hooks.ps1` による `.devin/config.json` native
登録を実装する。issue #107 の範囲評価本体は含まず `Refs #107` のみ
（`Closes` 不使用）。

## Scope

単一 implementation stream、2 task。T1（capability 露出）→ T2
（登録補助）は critical path で結合される: T2 の apply は T1 の
`hook targets --json` handshake を書込み前提に使うため分割できない。
共通 release intent で、無関係 subsystem の混在ではない。

## Upstream Documents

- `docs/specs/case-insensitive-native-tools-and-devin-registration.md`
  （SPEC-034GssA0IjLKSiDdIHpAC8, approved）
- `docs/adr/apm-distributed-devin-native-hook-bootstrap.md`
  （ADR-034GssAufIiId3hgh4jrSQ, accepted）

## Design Inputs

- `docs/designs/devin-native-hooks-and-apm-bootstrap.md`
  （DESIGN-034Gt74p3BYG40NTwu2kST, approved。exact interface・値・
  script algorithm・検証設計は本 doc をそのまま適用する）

## File Map

### T1: Normalize native tools and expose Devin hook capability

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/cheaper-cli/src/hook/targets.rs` | `HookTarget::Devin`、`pub const ALL`（windsurf, copilot, pretooluse, cursor, devin）、共有 `tool_use_normalize`、`devin_render` | `as_str` match が文字列 authority。Devin は `shell_tools: &[]`（native exec 不対応を明確化） |
| `crates/cheaper-cli/src/hook/mod.rs` | argv 完全一致 `["targets","--json"]` の capability 分岐（stdin 非読取・usage 非記録・exit 0 1 行 JSON） | native event ではない utility。他 argv は既存 parse → fail-open 維持 |
| `crates/cheaper-cli/tests/hook.rs` | 既存 `Fixture`/`run_hook` で contract test | red → impl → green |
| `packages/apm/.apm/hooks/hooks.json` | `PreToolUse` matcher を ASCII case-insensitive exact char class へ | spec の exact regex |
| `scripts/test/devin-hooks-install.test.mjs` | source matcher test 部分のみ（T1 時点） | installer 部分は T2 |

### T2: Register Devin native hooks from the APM-distributed skill（depends on T1）

| Path | Responsibility | Notes |
| --- | --- | --- |
| `packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1` | preview/`-Apply`・capability handshake・保全 gate・conflict scan・atomic write（design の algorithm をそのまま実装） | 新規。Windows PS 5.1・ASCII・comment 追加なし。strict preservation の `Get-JsonShape` は design `## JSON Preservation Helper` の canonical copy をそのまま使用（ignored `tmp` への依存を残さない） |
| `scripts/test/devin-hooks-install.test.mjs` | installer test 部分追加 | Windows のみ実行・他 OS 明示 skip |
| `docs/guides/distribution.md` | Devin native 登録の消費手順（新規/既存 consumer 分離）追記 | |
| `packages/apm/.apm/skills/cheaper-usage/SKILL.md` | lead-authored exact copy の挿入 + 既存 Claude bullet の read/grep case 対応明記 | 文面は後段で lead が exact copy を供給。書き換え・追加生成しない |

`install-devin-hooks.ps1` は APM の skill resource として verbatim
配布されるため、package config / lockfile は変更しない。runtime hook
は PATH 上の CLI を直接起動する。新規 dependency なし、APM 0.28.0
pin 維持。

## Implementation Sequence

### T1（Normalize + capability）

- [ ] `hook.rs` に fail test を追加する: `read`/`grep` 各 case
      variant の同一 routing、threshold 超過 file deny / 小 file
      allow / `file_path` 欠落・非 string fail-open / 未知・
      prefix/suffix tool fail-open、`tool_input` 非 object・root 非
      object・`tool_name` 欠落の fail-open、`grep` の `path` 欠落→
      cwd directory / directory allow、既存 `Bash`/`PowerShell`
      shell guard 維持、Devin target の native event gate
      （`hook_event_name=="PreToolUse"` のみ受理）・deny の
      `{"decision":"block","reason":...}` 1 行・`reason`
      passthrough・allow/fallback silent、`ALL` parse roundtrip、
      `targets --json` の 1 行 JSON・stdin 非読取・usage 非記録。
- [ ] `scripts/test/devin-hooks-install.test.mjs` の source matcher
      test 部分を作成する（各 case 一致 + prefix/suffix 未知 tool
      不一致、crossplatform 実行可能部分のみ。hooks.json 変更前に
      置く）。
- [ ] `mise exec -- cargo test -p cheaper-cli --features dev-seam
      --test hook` と `mise exec -- node --test
      scripts/test/devin-hooks-install.test.mjs` の双方で red を
      確認する。
- [ ] `targets.rs`（`HookTarget::Devin`・`ALL`・`tool_use_normalize`・
      `devin_render`）と `mod.rs`（capability 分岐）を実装し、
      `packages/apm/.apm/hooks/hooks.json` の `PreToolUse` matcher を
      spec の char class exact に更新する。
- [ ] 同一 cargo test + node test の双方を再実行して green を確認
      する。

### T2（APM-distributed registration helper）

- [ ] `devin-hooks-install.test.mjs` に installer test を追加する
      （design の fixture/stub/semantics: repo `tmp` 下 fixture、PS 5.1
      full path spawn、`fixturebin/cheaper.cmd` stub で固定 capability
      出力、PATH は fixturebin + 必要 systemdir のみ、preview が file
      非生成、apply で entry 追加 + 既存 object/array/null/deep data
      保持 + idempotent bytewise、既知 legacy duplicate 各 source、
      shell-only legacy 非 conflict、changed own entry、bad
      JSON/schema、reparse point（junction）、capability 拒否各種
      — nonzero / empty / malformed / unknown schema / `devin` 欠落 /
      `'Devin'` case 違い拒否、probe 例の ordinary/specialKey 受理と
      duplicate/caseCollision/escapedDuplicate/hugeNumber/
      high-precision float/exponent の保全拒否、secret-sentinel
      非出現）。
- [ ] 同一 node test を実行して red を確認する。
- [ ] `install-devin-hooks.ps1` を design の algorithm で実装する
      （`param([string]$ProjectRoot,[switch]$Apply)`、schema 検査、
      design `## JSON Preservation Helper` の `Get-JsonShape` 保全
      gate、conflict scan、capability handshake、temp + atomic
      `File.Replace`/`File.Move`、`finally` cleanup、notice 出力）。
- [ ] 同一 node test を再実行して green を確認する。
- [ ] `docs/guides/distribution.md` に消費手順を追記し、
      `SKILL.md` に lead-authored exact copy（下記）を挿入する。
      既存 Claude bullet は lead-authored 差し替え copy（下記
      authoritative copy）で現行 4 行（L118-120 付近）を置換する。
      文面は生成しない。
- [ ] 手動 targeted integration gate を 1 回実行する（後述
      Verification）。

T2 `SKILL.md` へ挿入する lead-authored exact copy（英文、そのまま
転記してよい引用。script/skill の改変は T2 実装時にのみ行う）:

```text
### Devin CLI native registration (Windows)

APM distributes the registration helper as part of this skill. For a new
Devin-only consumer, install the package with `apm install --target agent-skills`.
For an existing consumer, keep its configured targets and use the normal
`apm install` flow; do not replace targets merely to obtain the helper.
Use the actual deployed skill path when `--legacy-skill-paths` is enabled.

From the consumer root, preview with:

powershell -NoProfile -ExecutionPolicy Bypass -File .agents/skills/cheaper-usage/scripts/install-devin-hooks.ps1

Add `-Apply` to register native read/grep hooks in `.devin/config.json`.
The helper requires a CLI that reports `devin` through `cheaper hook targets --json`;
updating APM resources does not update the separately installed CLI. It preserves
unrelated configuration and refuses known duplicate cheaper read hooks. Resolve
legacy registrations explicitly, then use `/hooks` to check all loaded sources,
including global and ancestor configurations. Native shell tools are not registered.

This is supplemental registration, not an APM-managed hook target. APM uninstall
does not remove the native entry; remove only its `cheaper hook pre-tool-use --target devin`
group manually when uninstalling. The guard still measures whole files: native
registration and case normalization do not resolve bounded-read issue #107.
```

（SKILL.md 上は上記 powershell 行を fenced code block で囲む）

T2 `SKILL.md` の既存 Claude bullet 差し替え用 lead-authored
authoritative copy（英文 verbatim。現行の 4 行 block をこれで
置換する）:

```text
  - **Claude Code** — `PreToolUse` (case-insensitive exact `read`/`grep`/
    `bash`/`powershell` matcher); single-file reads/searches and shell reads
    use the shared whole-file guard. Denies return
    `hookSpecificOutput.permissionDecision: "deny"` on stdout with exit 0.
```

## Dependency Graph

| Step | Depends on | Blocks |
| --- | --- | --- |
| T1 tests（red） | — | T1 impl |
| T1 impl + matcher | T1 red | T1 green、T2 tests |
| T1 green | T1 impl | T2 |
| T2 installer tests（red） | T1 green（capability contract 確定） | T2 impl |
| T2 impl | T2 red | T2 green |
| T2 green + docs | T2 impl | manual gate |
| Manual APM integration gate | T1 + T2 green | 完了 |
| `mise run check` 最終 gate | 全 task + review 完了 | — |

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | 旧 CLI で capability 非対応 → hook が silent allow | med | high | `-Apply` は `hook targets --json` の schemaVersion 1 + exact `devin` 必須（T2 tests）。version 判定禁止 |
| 2 | 既存 multi-target consumer の `apm install --target` 置換で dropped-target reconciliation が既存 artifact を drop | med | high | 手順を consumer 状態別に分離。SKILL/distribution 文面は lead-authored copy で固定 |
| 3 | malformed / duplicate key / precision loss を含む既存 config の黙っての上書き | med | high | `Get-JsonShape` 保全 gate（probe 採用済）+ schema 検査 + 差異時は bytes 不変 error |
| 4 | 既知 legacy 重複 hook との二重 deny | med | med | 限定 scan → apply 停止 + 場所通知。shell-only legacy `pre-run-command` は非 conflict |
| 5 | symlink/reparse・changed own entry・書込み中の競合変更 | low | high | reparse 検査 + own entry 検証 + 原 bytes 再検査 + atomic write + `finally` cleanup |
| 6 | matcher 拡大による breaking change: `bash`/`powershell` でも Pretooluse hook が発火し発火頻度が増える | med | low | tool 固有 routing + fail-open で判定は不変（全文 policy 維持）。T1 tests で shell guard 維持と fail-open を assert |
| 7 | 外部 runtime availability（mise/node/PS 5.1/APM 0.28.0 pin）の欠落で検証不能 | low | med | 欠落時は skip と環境整備を区別して報告。dependency・pin 変更は禁止 |
| 8 | APM 配布経路の drift | low | med | 手動 integration gate で hash 一致を 1 回確認（CI には足さない） |

## Rollback Strategy

- **Trigger**: native 登録が誤動作（誤 deny / 二重 deny / config 破損
  報告）。
- **Procedure**: 自動 git rollback ではなく、consumer 側の managed
  native command group（`cheaper hook pre-tool-use --target devin`
  の group のみ）を手動で `.devin/config.json` から除去する（他
  hook・他 event・他 config は保持）。repo 側の変更は review 済み
  PR の revert で戻す。
- **Verification**: `/hooks` builtin で `pre_tool` 登録から該当
  command group が消えたことを確認。
- helper は今回の scope で実 consumer へ自動 apply しない（検証は
  repo 内 fixture のみ）。

## Verification

| Step | Command / criteria | Pass condition |
| --- | --- | --- |
| T1 tests | `mise exec -- cargo test -p cheaper-cli --features dev-seam --test hook` | red → impl → green |
| T1 matcher | `mise exec -- node --test scripts/test/devin-hooks-install.test.mjs` | source matcher 部分 pass（crossplatform） |
| T2 tests | 同一 node command（installer 部分含む全体） | green。Windows のみ実実行、他 OS は明示 skip |
| Integration gate | repo 内 ignored `tmp` の fresh isolated consumer（`apm.yml` に `packages/apm` local-path dependency）で APM 0.28.0 `apm install --target agent-skills` を 1 回 | helper が deploy され source と hash 一致、`.windsurf`/`.claude` hook 未生成。network/model 不要、admission/policy bypass なし、global/既存 consumer 設定無変更 |
| Final gate | `mise run check`（全実装 review 後に 1 回のみ） | exit 0 |

再 verify は変更範囲の targeted command のみ。test 失敗の修正は
対象 test の再実行。`cargo fmt` は変更 file 前提で、root 全体 fmt
の noise が出たら無関係 format はしない。build/setup repair は必要
時のみ。APM 0.28.0 pin・dependency・security policy の変更禁止。

## Open Questions / Gaps

- test-spec / task doc は生成済み:
  `TSPEC-034GtADUaoOPo8wHPI3Liu`・`TSPEC-034GtAElEShJ9SxImZE2ot`
  （proposed）、`TASK-034GtBzfyOaeVS6nlm1ZkC`・
  `TASK-034GtBzKmwPf5z06kAahym`（todo）。
- `SKILL.md` の Claude bullet 差し替え copy は lead-authored 済み
  （Implementation Sequence に authoritative copy として記録）。
- PS 5.1 保全 gate の実現性は probe で解消済み（design Review
  Record の frozen 結果と `## JSON Preservation Helper` を参照）。

## Self-Review

- [x] spec の各 requirement が T1/T2 のいずれかの step に map する。
- [x] placeholder なし（未供給文面は「lead が exact copy 供給」と
      明示）。
- [x] file path・relation key・status・command・用語は上流 doc と
      一致。

## Implementation Handoff

同一環境で実装・検証を委譲し、lead が task ごとに差分/証拠を
review。ユーザー指定により approval gate は review-protocol で
confirmed P0/P1 無しを承認代替とする。追加の独立
subagent/remote context は起動しない。

## Review Handoff

- 前提: 上流 3 doc は承認済み。wire bounds の存在・消失地点・PS 5.1
  保全 gate 実現性は frozen 証拠として再検証しない。
- Reviewer focus（各項目は既定 test に map 済み）:
  1. 旧 CLI capability 欠落 → silent allow（T2 capability 拒否
     tests）。
  2. 既存 multi-target consumer の APM target drop（手順分離 +
     lead-authored 文面）。
  3. malformed/duplicate/precision JSON の保全（`Get-JsonShape`
     gate + schema 検査）。
  4. 既知 project legacy duplicate vs shell-only 非 conflict（conflict
     scan tests）。
  5. symlink / changed own entry / concurrent file change の保全
     （reparse 検査 + bytes 再検査 + atomic write）。
- 上流: spec（要件/AC）、ADR（登録構造決定）、design（interface・
  algorithm・検証設計）。

## Review Record

- 2026-10-15: review-protocol の role-separated pass（Reviewer A →
  Reviewer B → Validator → Judge）を実施。Reviewer B の P-E（
  Implementation Handoff が approval 代替方針と矛盾）は P1 blocking
  として lead 固定文へ置換して解消。Reviewer A の P-A（P2、Node
  matcher test を hooks.json 変更前の red へ移動・双方 red 確認）、
  P-B（P3、breaking change / runtime availability の risk 2 行）、
  P-C（P3、root/tool_input/`tool_name` malformed fail-open の test
  列挙追加）、P-D（P3、`Get-JsonShape` を design
  `## JSON Preservation Helper` へ inline 化して ignored tmp 依存を
  除去）はすべて適用済み。`SKILL.md` Claude bullet の
  lead-authored authoritative copy も記録済みで open question は
  解消。修正後差分は lead が全文確認して ACCEPT、status approved
  へ更新済み。
