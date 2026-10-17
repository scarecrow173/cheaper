---
id: "SPEC-s8dMxk0QAbLuWaOrsuikSp"
type: "spec"
status: "proposed"
title: "worker schema の magnitude 制約撤廃と INVALID_OUTPUT raw output の回復経路"
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
  implements: []
  implemented-by:
    - "../designs/worker-schema-structure-not-magnitude.md"
    - "../plans/worker-schema-structure-not-magnitude.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/worker-schema-magnitude-constraints-and-output-recovery.md"
    - "../adr/worker-schema-structural-validation-not-magnitude.md"
    - "../adr/invalid-output-raw-output-persistence.md"
    - "../designs/worker-schema-structure-not-magnitude.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/worker-schema-magnitude-constraints-and-output-recovery.md"
  derived-by:
    - "../adr/worker-schema-structural-validation-not-magnitude.md"
    - "../adr/invalid-output-raw-output-persistence.md"
    - "../designs/worker-schema-structure-not-magnitude.md"
    - "../plans/worker-schema-structure-not-magnitude.md"
---

# worker schema の magnitude 制約撤廃と INVALID_OUTPUT raw output の回復経路

## 背景と目的

Issue #113・#114。2 件は同じ根本原因を共有する: built-in worker の
input/output JSON Schema が `maxLength`・`maxItems`・`maximum` という
**大きさ(magnitude)制約**を課しており、その上限値には設計上の根拠が
ない(`bulk-reader-contract.md` で表形式に定められたのみで、worker 間でも
`task` 4000/6000 と不整合)。

- **#113**: `bulk-reader` の `task` が 4000 文字超で `INVALID_INPUT`。
  inline でしか渡せない内容(貼り付け diff・外部 text・workspace 外資料)は
  `paths` に載せられないため、実在の用途を機械的に潰す。入力長は
  provider/model の context window が担保する制約であり、cheaper が
  不透明な値で先回りする理由がない。
- **#114**: 完走済み run(19 turns・42 tool calls・~1M tokens)が
  `findings[].title` の 120 文字超過 1 件で `INVALID_OUTPUT` となり結果
  ごと廃棄される。schema 検証が「構造 contract」ではなく「提示品質」を
  掴んで実作業を捨てる不均衡。さらに回復経路の `details.rawOutput` は
  先頭 2000 文字に切り詰められ、大きな result は実質消失する。

目的は **「JSON Schema は構造的 contract を検証し、magnitude を検証
しない」** という統一原則を built-in worker schema に適用し、あわせて
`INVALID_OUTPUT` の raw output を完全に回復可能にすること(ユーザー決定
2026-10-17)。品質ガイダンス(短い findings 等)は schema ではなく
worker 指示文(prose)の責務として分離する。

## スコープ

**対象:**

- 全 built-in worker の input schema(`bulk-reader`/`code-writer`/
  `doc-writer`/`tool-operator`)から magnitude 制約(`maxLength`/
  `maxItems`/`maximum`)を撤廃する。
- 全 built-in worker の output schema から同様に magnitude 制約を撤廃する。
- `maxFindings`(bulk-reader・tool-operator `focus.maxFindings`)は Optional
  のまま `maximum` を撤廃し、`minimum: 1` は構造的意味として維持する。
  明示 `null` も受理し、省略と同義(件数指示なし・model 判断)とする。
- `INVALID_OUTPUT` 発生時に raw output 全文を `<userConfigDir>` 配下へ
  atomic write し、envelope 最外層 error の `details` に file 参照を
  追加する。既存の 2000 文字 excerpt は診断用に残す。保存責務は
  `repair_output` が担うため `cheaper run` と workflow の worker job
  (同一 `run_worker` 機構を共有)の双方に適用される。workflow 自身の
  output materialization が出す `INVALID_OUTPUT` は別経路のままとする。
- 追従文書: `cheaper-usage`・`cheaper-bulk-read` SKILL.md(#108 で追加した
  limits 表は obsolete)、`docs/guides/configuration.md`/`usage.md`、
  golden/regression fixture の expected。

**除外:**

- `path` pattern(workspace 相対強制)、`required`、`minLength: 1`、
  `minItems`、`additionalProperties: false`、enum、`lines.start/end ≥1`
  等の構造制約 — 変更しない。
- `tool-operator.output` の `grantRequests` 制約(`executable` ≤4096・
  `argv` ≤256件・`name` ≤64+pattern)— policy `propertyNames` と連動する
  構造的識別子規約であり magnitude 制約ではない。維持。
- policy 値(timeout/maxTurns/maxToolCalls/maxTokens/repairAttempts/
  maxOperations)、tool pagination(read/grep/find/ls/count-tokens 上限)、
  内部 cap(`details.errors` 20 件・checkpoint 制約・hook routing scan
  cap)— リソース制御・機構的境界であり本件の対象外。
- workflow 経路の `INVALID_OUTPUT` — details shape が別系統のため
  file 回復は run 経路のみ(Open Items に記録)。
- checkpoint entry/合計 byte cap(#111)の変更。
- consumer 独自 worker の schema — built-in のみ対象。独自 worker の
  schema 記述は作者の自由。

## 要件

### R1: input schema の magnitude 撤廃

1. 全 built-in input schema から `maxLength`・`maxItems`・`maximum` を
   削除する。`required`・`minLength: 1`・`minItems: 1`(意味を持つ非空
   制約)・`pattern`(path 構造)・型は維持する。
2. `task` は `minLength: 1` のみ残り、長さ上限は provider/model の能力に
   委譲される。`INVALID_INPUT` の `details.errors` は引き続き違反箇所を
   返す(変更なし)。
3. `maxFindings` は Optional のまま `maximum: 30` を撤廃し、
   `minimum: 1` のみ残す(runtime は参照しない prompt 専用 field)。
   型を `["integer", "null"]` に拡張し、明示 `null` を「件数指示なし」
   (省略と同義)として受理する。

### R2: output schema の magnitude 撤廃

1. 全 built-in output schema から `maxLength`・`maxItems`・`maximum` を
   削除する。構造制約(型・required・enum・`additionalProperties: false`・
   `lines.start/end ≥ 1`・`grantRequests` block)は維持する。
2. 撤廃により output schema は「構造 contract」のみを検証し、
   「`title` が 121 文字」等の magnitude 違反で完走 run が
   `INVALID_OUTPUT` になる経路は消滅する。
3. `INVALID_OUTPUT` は構造違反(JSON 非適合・required 欠落・型違い)専用と
   して存続し、`repairAttempts` による repair 経路は変更しない。
4. 出力の圧縮品質(短い summary/findings)は worker.md の prose 指示が
   担う — schema による強制は行わない。

### R3: `INVALID_OUTPUT` の raw output 回復経路(run 経路)

1. repair を尽くしても schema 検証が失敗する場合、raw output 全文を
   `<userConfigDir>` 配下の専用 dir へ atomic write(temp + rename)
   し、envelope 最外層 error の `details` に file 参照(path)を付与する。
2. `details.rawOutput`(先頭 2000 文字 excerpt)は診断用に変更せず残す。
   `details.errors`(先頭 20 件)も同様。
3. file 保存は best-effort — 保存失敗でも error 分類・exit code を
   変えず、file 参照 key を省略するだけ。
4. file には schema 適合を判定されなかった生の model 出力をそのまま
   書く(JSON でなくてもよい — 解析不能出力も回収対象)。
5. workflow 経路の `INVALID_OUTPUT` は対象外(details shape が別系統)。

### R4: 文書追随

1. `cheaper-usage`/`cheaper-bulk-read` SKILL.md の limits 表・
   `INVALID_OUTPUT` 回復指針を新 contract に書き換える(上限値表の削除、
   raw output file 回復の記載)。
2. `docs/guides/configuration.md`・`usage.md`・関連 spec/design の
   上限値記述を更新する。
3. `bulk-reader-contract.md` design 文書の上限表は歴史的記録として
   残し、supersede/更新の関係を本件 ADR から明示する。

## Acceptance Criteria

- [ ] `{"task": <4000 文字超>}` の bulk-reader input が `INVALID_INPUT`
      にならず受理される(schema 検証通過)。
- [ ] `findings[].title` が 120 文字超の output が schema 検証を通り
      `INVALID_OUTPUT` にならない。
- [ ] required 欠落・型違い・JSON 非適合の出力は引き続き
      `INVALID_OUTPUT`(exit 6)になる。
- [ ] `INVALID_OUTPUT` 時に envelope 最外層 `error.details` が全文 raw
      output の file path を含み、その file が完全な raw output を保持する。
      保存失敗時は path key が省略され error 分類は不変。
- [ ] `tool-operator` の `grantRequests` 制約は引き続き強制される
      (executable/argv/name の上限・pattern 違反は `INVALID_OUTPUT`)。
- [ ] `maxFindings` 省略・`null`・任意の正整数で受理され、`0`/負は
      拒否される。
- [ ] golden/regression fixture と `mise run check` が pass する。

## Open Items

- workflow の output materialization 由来 `INVALID_OUTPUT` への file
  回復適用(details shape 差 — worker job 経由の `INVALID_OUTPUT`
  自体は共有 `run_worker` 経由で本件に含まれる)。
- magnitude 撤廃後の肥大 output 暴走に対する観測(usage.jsonl の
  `meta.usage`/`savings` で実測可能)— 問題が実測されたら prompt 指示の
  強化を別途検討。
- `details.rawOutput` excerpt(2000 字)の存続意義は file 回復があれば
  診断用程度 — 廃止は contract 簡素化の余地として記録(非破壊のため保留)。

## Sources and Relations

- 一次ソース: GitHub issue #113・#114(scarecrow173)
- 調査記録: DISC-z4FTQSzd835hAlRqSPb7QY
- 関連 design: DESIGN-034r05iHrZUE6WxkSJi9GF(bulk-reader contract —
  上限値の出元。本件で magnitude 部分を supersede)
- 関連 ADR: budget-exhaustion-checkpoint-and-resume(#111 — details に
  安全な参照・本体は file、の先例)
