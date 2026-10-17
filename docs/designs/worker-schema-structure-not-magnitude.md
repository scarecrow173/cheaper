---
id: "DESIGN-EmeEdXi9TWZZW2PzZsp29y"
type: "design"
status: "approved"
title: "worker schema magnitude 撤廃と INVALID_OUTPUT raw output 永続化の実装設計"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/113"
    - "https://github.com/scarecrow173/cheaper/issues/114"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/worker-schema-structure-not-magnitude.md"
  implemented-by:
    - "../plans/worker-schema-structure-not-magnitude.md"
  depends-on:
    - "../adr/worker-schema-structural-validation-not-magnitude.md"
    - "../adr/invalid-output-raw-output-persistence.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/worker-schema-structure-not-magnitude.md"
    - "../adr/worker-schema-structural-validation-not-magnitude.md"
    - "../adr/invalid-output-raw-output-persistence.md"
    - "../discovery/worker-schema-magnitude-constraints-and-output-recovery.md"
    - "../plans/worker-schema-structure-not-magnitude.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/worker-schema-structural-validation-not-magnitude.md"
    - "../specs/worker-schema-structure-not-magnitude.md"
  derived-by:
    - "../plans/worker-schema-structure-not-magnitude.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/bulk-reader-contract.md"
  defers: []
  deferred-by: []
---

# worker schema magnitude 撤廃と INVALID_OUTPUT raw output 永続化の実装設計

## 概要

SPEC-s8dMxk0QAbLuWaOrsuikSp の実装設計。2 系統の変更と文書追随で構成する:

1. **worker schema の magnitude 制約撤廃**(ADR-BTZgr7PG9GhlAGhdhbirlM)—
   built-in worker 8 schema(input×4 / output×4)から `maxLength`・
   `maxItems`・`maximum` のうち payload 大きさを縛るものを除去する。
   構造制約(`required`・`type`・`enum`・`additionalProperties: false`・
   `minLength: 1`・path pattern・`lines >= 1`)は維持。
2. **`INVALID_OUTPUT` の raw output 永続化**
   (ADR-zHFZ3QOA0Q6lEoWmS5gzIQ)— repair 尽きた最終 raw output を
   `<userConfigDir>/invalid-outputs/<id>.txt` へ best-effort 保存し、
   error `details` に `rawOutputPath` を載せる。envelope には従来どおり
   先頭 2000 文字の `rawOutput` excerpt のみ残す。

背景: output schema は provider の structured output 機構には渡されず、
system prompt への text 埋め込み(`build_system_prompt`)と事後検証
(`extract_output`)にのみ使われる。したがって magnitude を schema から
除去しても model への品質ガイダンスは worker prose(worker `.md`)が
担い続け、schema は「構造 contract + 事後検証」の座に留まる。

## 1. schema magnitude 撤廃

### 1a. 除去対象(built-in 8 schema)

`crates/cheaper-cli/resources/workers/schemas/` 配下:

| schema | 除去する制約 |
|---|---|
| `bulk-reader.input` | `task.maxLength(4000)`、`paths.maxItems(50)`、`hints.maxItems(20)`、`maxFindings.maximum(30)` |
| `code-writer.input` | `task.maxLength(6000)`、`targets.maxItems(20)`、`targets[].notes.maxLength(500)`、`references.maxItems(30)`、`constraints.maxItems(20)`・item `maxLength(300)` |
| `doc-writer.input` | `task.maxLength(6000)`、`targets.maxItems(20)`、`targets[].notes.maxLength(500)`、`decisions`/`facts`/`references.maxItems(30)`・item `maxLength(1000)`、`constraints.maxItems(20)`・item `maxLength(300)` |
| `tool-operator.input` | `task.maxLength(4000)`、`context.maxItems(30)`・item `maxLength(1000)`、`constraints.maxItems(20)`・item `maxLength(300)`、`output.focus.maxItems(10)`・item `maxLength(300)`、`output.maxFindings.maximum(30)` |
| `bulk-reader.output` | `summary.maxLength(1500)`、`findings.maxItems(30)`、`title.maxLength(120)`、`detail.maxLength(1200)`、`locations.maxItems(10)`、`excerpt.maxLength(600)`、`relevantFiles.maxItems(30)`、`suggestedReads`/`openQuestions` の `maxItems`・item `maxLength(300)` |
| `code-writer.output` | `summary.maxLength(1500)`、`notes`/`unresolved.maxItems(20)`・item `maxLength(500)` |
| `doc-writer.output` | `summary.maxLength(1500)`、`notes`/`unresolved.maxItems(20)`・item `maxLength(500)` |
| `tool-operator.output` | `summary.maxLength(1500)`、findings block(`maxItems`・`title`/`detail.maxLength`)・`evidence.maxItems`・`unresolved`/`nextActions` の `maxItems`・`maxLength`、`evidence.commit.maxLength(100)`・`detail.maxLength(600)` |

### 1b. 維持する制約

- `required`・`type`・`enum`・`additionalProperties: false`
- `minLength: 1`(空文字は構造的に無意味)・構造的 `minItems`
  (例: code-writer `files.minItems: 1` — 対象無しの write は contract 外)
- `$defs.path` の workspace 相対 pattern・`lines.start/end >= 1`
- **`tool-operator.output` の `grantRequests` 内制約は維持**:
  `executable.maxLength(4096)`・`argvHint.maxItems(256)`・
  `name.maxLength(64)`・`name.pattern`。grantRequests は policy の
  grant 識別子規約(`propertyNames` pattern)と照合する宣言 payload で
  あり、ここの制約は magnitude ではなく policy vocabulary の構造制約に
  連動する。撤廃すると検証層間で矛盾が生じる。
- `maxFindings` / `focus.maxFindings` の `minimum: 1`(0 以下は無意味入力)

### 1c. `maxFindings` の optional/null 化

`bulk-reader.input` の `maxFindings` と `tool-operator.input` の
`focus.maxFindings`:

- `required` には元々含まれていない(optional は既存状態)。
- `maximum: 30` を除去し `minimum: 1` のみ残す。
- `type` を `["integer", "null"]` に拡張し、明示 `null` を「上限なし・
  model 判断に任せる」既定意味として受理する。runtime は
  `maxFindings` を読まない(調査済み — worker prose 経由の model 指示
  のみ)ため、`null` 受理は schema 契約の整備であり runtime 分岐を
  増やさない。

### 1d. worker prose との整合

`bulk-reader.md` の "Respect `maxFindings` when given" は変更なしで整合
済み。各 worker `.md` の品質ガイダンス(summary は簡潔に、excerpt は
verbatim 等)は prose として残し、schema の強制からは外す — schema=構造、
prose=品質ガイダンスの分離を維持。prose 中に schema 上限値への言及があれば
除去対象とする(文書追随タスクで確認)。

## 2. `INVALID_OUTPUT` raw output 永続化

### 2a. 保存責務と経路

`crates/cheaper-runtime/src/run/` に `invalid_output.rs`(仮)を追加、
または `checkpoint.rs` と同型の保存 helper を置く:

- 保存先: `<userConfigDir>/invalid-outputs/`(checkpoint の
  `checkpoints/` と同じく `deps.runtime.user_config_dir()` 配下の
  新規 dir)。
- ファイル名: `run-<pid>-<nanos>.txt`(checkpoint と同一採番規則)。
- 内容: repair loop 最終 attempt の raw output **全文**(fence 除去前
  original — `ExtractedOutput::Err.raw_output` と同一値)。
- 書き込み: `cheaper_core::fs_util::atomic_write`(`unix_mode:
  Some(0o600)`)— checkpoint と同じ atomic write + 権限規則。

### 2b. error details への露出

`repair.rs::invalid_output_error` が `details` を組み立てる単一点。
`repair_output` の loop 終了時に保存を試み、成功時のみ
`rawOutputPath`(絶対 path 文字列)を `details` に追加する:

```json
"details": {
  "errors": [...先頭20件],
  "rawOutput": "<先頭2000文字>",
  "rawOutputPath": "<userConfigDir>/invalid-outputs/run-<pid>-<nanos>.txt"
}
```

- 保存は **best-effort**: 書き込み失敗しても `INVALID_OUTPUT` 分類・
  exit code・`errors`/`rawOutput` は不変(`rawOutputPath` が省略される
  だけ)。checkpoint の `details.checkpoint` と同じ規則。
- `RepairOptions` に `user_config_dir: &Path` を追加し、呼び出し側
  (`extract_or_repair`)から `deps.runtime.user_config_dir()` を渡す。
  `RunWorkerDeps.runtime` は必須 field で workflow の worker job 経路も
  同一 `run_worker` を通るため、Option での経路差吸収は不要。
- 保存対象は **最終 attempt の raw output**(= `rawOutput` excerpt と同じ
  値)とする — details 内の excerpt と file が同一 source で表現 drift
  を起こさない。初回 output を別途保存する設計は取らない(repair 後の
  最終状態が envelope と一致する方が debug で矛盾しない)。

### 2c. 境界

- 保存責務は `repair_output` に置くため、適用範囲は自然に
  `cheaper run` + workflow の `kind: worker` job(同一 `run_worker`
  機構)の双方へ及ぶ — 分岐は設けない。workflow job error の
  details が workflow envelope にどう表れるかは別経路の問題であり、
  file 自体は書かれる。
- workflow の **output materialization** が出す `INVALID_OUTPUT`
  (engine の `materialize_output` 系)は details shape が別系統の
  ため本件の非対象とし、follow-up 候補に記録する。
- `NoAssistantMessage`(raw_output 空)でも保存を試みてよいが、空 file
  の保存価値は薄い — `raw_output.is_empty()` なら保存 skip とする。

## 3. `INVALID_INPUT` の診断性

`INVALID_INPUT` は既に `details.errors` に instancePath・message を載せる
契約がある(#113 調査で確認)。ただし `validate_worker_input` は errors を
**無制限に serialize** しており、`INVALID_OUTPUT` の先頭20件 cap
(`MAX_DETAIL_ERRORS`)と不整合 — envelope 肥大化防止の揃えとして input
側も同じ 20 件 cap を適用する(3行の整合修正、両経路で同一の診断面に
揃える)。残る構造違反(`required` 欠落・型違い・path pattern 違反)の
instancePath 出力を pin する regression test を足す。

## 4. 文書追随

- 上限値・`INVALID_OUTPUT` 回復手順に言及している確定箇所(調査済み):
  - `.agents/skills/cheaper-bulk-read/SKILL.md` と
    `packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md` —
    `task` の `≤4000` 表記と `INVALID_OUTPUT` 節の canonical limits・
    `rawOutput` 2000字説明を更新し `rawOutputPath` 回収手順を追記。
  - `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md` —
    `task` の `≤4000` 表記を除去。
  - `packages/apm/.apm/skills/cheaper-usage/SKILL.md` —
    `INVALID_OUTPUT` details 説明に `rawOutputPath` を追記。
- 方針: 「出力は schema 上限内に収める」記述 → 「構造 contract のみ強制、
  品質指針は prose」へ。
- worker `.md` の prose は品質ガイダンスとして維持するが、
  `bulk-reader.md`/`tool-operator.md` の "Respect `maxFindings` when
  given" に「省略・`null` の場合は件数は model 判断」を明記し、
  `null` 受理の semantics が consumer から読めるようにする。

## テスト方針

- **schema regression**(既存の schema corpus / golden test):
  - `task` に 4000/6000 文字超の入力が受理されること。
  - `findings[].title` 121 文字超の output が受理されること。
  - `maxFindings: null`・`maxFindings` 省略・`maxFindings: 100` が受理
    され、`maxFindings: 0` が拒否されること。
  - 構造違反(required 欠落・型違い・path pattern・`grantRequests.name`
    pattern)は依然拒否されること。
- **raw output 永続化**(dev-seam test):
  - `INVALID_OUTPUT` で `details.rawOutputPath` が存在し、file 内容が
    最終 raw output と一致すること。
  - 書き込み不能(user_config_dir を read-only 等)でも
    `INVALID_OUTPUT` + `rawOutput` excerpt が維持され `rawOutputPath`
    が省略されること。
  - `no-assistant-message`(raw 空)では `rawOutputPath` が付かないこと。
- **envelope 不変性**: `details.rawOutputPath` は pid/nanos を含み
  非決定的 — regression normalize に新規 rule を追加して scrub する
  (checkpoint の `details.checkpoint.path` scrub は別 key なので
  そのままでは効かない点に注意)。
- **INVALID_INPUT errors cap**: 21 件超の構造違反で `details.errors`
  が 20 件に収まること。

## 非対象

- workflow 経路の `INVALID_OUTPUT` details 統一(follow-up 候補)。
- provider 側の structured output API 配線(`SessionSpec.output_schema`
  の provider 実装 — 現状どの provider も未使用)。
- tool pagination cap・policy 値(`maxTurns`/`maxToolCalls`/
  `maxTokens`)等のリソース制約 — 本原則の適用外。
- `details.errors` 先頭 20 件・`rawOutput` 2000 文字 excerpt の cap —
  envelope 肥大化防止の診断用 excerpt であり magnitude 撤廃の対象外。

## 検証

- `mise run check` 全段。
- `cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
  (schema regression・dev-seam INVALID_OUTPUT 系を含む)。
- doc-id check・index 整合。
