---
id: "ADR-BTZgr7PG9GhlAGhdhbirlM"
type: "adr"
status: "proposed"
title: "worker JSON Schema は構造を検証し magnitude を検証しない"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
supersedes: []
superseded-by: []
derives-from:
  - "../discovery/worker-schema-magnitude-constraints-and-output-recovery.md"
  - "../specs/worker-schema-structure-not-magnitude.md"
implements: []
derived-by:
  - "../designs/worker-schema-structure-not-magnitude.md"
related:
  - "../discovery/worker-schema-magnitude-constraints-and-output-recovery.md"
  - "../specs/worker-schema-structure-not-magnitude.md"
  - "../adr/invalid-output-raw-output-persistence.md"
  - "../designs/worker-schema-structure-not-magnitude.md"
  - "../designs/bulk-reader-contract.md"
---

## Context

built-in worker の input/output JSON Schema は `maxLength`・`maxItems`・
`maximum` といった magnitude 制約を多数課している(bulk-reader `task` ≤4000、
`findings[].title` ≤120、code-writer/doc-writer `task` ≤6000 等)。
この値には設計上の根拠がなく(`bulk-reader-contract.md` で表形式に定められた
のみ)、worker 間でも不整合(4000 vs 6000)である。

この制約が2つの defect を生んでいる:

- **#113**: `bulk-reader` の `task` が ~4000 文字超で `INVALID_INPUT`。
  inline でしか渡せない内容(貼り付け diff・外部 text)は `paths` に
  載せられず、実在の用途を機械的に潰す。入力長は provider/model の
  context window が担保する制約であり、cheaper が不透明な値で先回りする
  理由がない。
- **#114**: 完走済み run(19 turns・42 tool calls・~1M tokens)が
  `findings[].title` の 120 文字超過一件で `INVALID_OUTPUT` に。
  「構造 contract 違反」ではなく「提示品質の超過」で実作業を捨てる
  不均衡。

### Forces

- worker output schema は model へ provider structured output としては
  渡されず、(a) system prompt への text 埋め込みと (b) 事後検証の
  2 用途のみ(discovery 確定済み)。schema から magnitude を撤いても
  「指示」と「検証」が同時に緩むだけで、prose 指示文は残る。
- 肥大 output が親 agent context へ返る懸念は bulk-reader の存在意義
  (圧縮)と緊張するが、「肥大した結果」は親 agent が加工可能で、
  「廃棄された結果」は取り戻せない。制御として schema 強制は破壊的
  副作用の方が大きい(実測 #114)。
- `INVALID_INPUT`/`INVALID_OUTPUT` の構造検証自体は contract として
  必要 — required 欠落・型違い・JSON 非適合は真の破綻を捕まえる。
- magnitude 制約には例外がある: `grantRequests`(tool-operator)の
  name/argv/executable 制約は policy `propertyNames`・argv pattern と
  照合する識別子規約で、撤くと grant/policy 整合が壊れる。
- `maxFindings` は runtime 非参照の prompt 専用 field — Optional
  (`minimum: 1` のみ)に留めるのが自然。

## Considered Options

### Option 1: magnitude 制約を全 built-in schema から撤廃(採用)

- 全 built-in input/output schema の `maxLength`・`maxItems`・`maximum`
  を削除し、schema は構造 contract(型・required・`minLength: 1`・
  `minItems`・`pattern`・enum・`additionalProperties: false`)のみを
  検証する。
- `maxFindings`/`focus.maxFindings` は `maximum` を撤廃し Optional の
  `minimum: 1` のみ残す。型を `["integer", "null"]` に拡張し、明示
  `null` を省略と同義(件数指示なし)として受理する。
- `grantRequests`(tool-operator.output)の制約は policy vocabulary と
  連動する構造制約として維持する。
- 品質ガイダンス(短い summary/findings、excerpt の簡潔さ)は
  worker.md の prose 指示に委譲する — schema=構造、prose=品質の分離。
- **長所**: #114 の「些細な違反で完走 run を廃棄」様式を根絶し、
  #113 の inline-only 用途を解放する。概念が単純で、worker 数に比例
  しない一括適用。恣意的な値を再選定する必要もない。
- **短所**: model が暴走した場合に巨大な result が親 agent context へ
  返る(品質問題として受容 — Open Items に観測手段を記録)。
  repair prompt へ巨大な raw output がそのまま埋め込まれる懸念も
  あったが、`repair_user_prompt` は本変更以前から常に無制限の raw
  output を載せており(schema 違反は magnitude 以外でも起き得る)、
  本撤廃が新規に作るリスクではない — 既存状態の明示化に留まる。

### Option 2: 上限値の引き上げのみ(不採用)

- `task` を 6000/32k 等へ引き上げ、出力側上限も緩める案。
- **短所**: 恣意的な別の値を選ぶだけで inline-only 用途の根本解決に
  ならず、「なぜこの値か」が常に再燃する。magnitude 制約の存在意義
  自体が問われているのに値だけ変えるのは対症療法。

### Option 3: worker 定義に失敗モード knob を追加(不採用)

- `INVALID_OUTPUT` 時の挙動(discard/truncate/partial)を worker 定義で
  選べる案。
- **短所**: 失敗要因で適切な挙動が変わるのに per-worker の一様な
  knob では表現不能。実際の選択肢も「捨てる/返す」の2択しかなく
  YAGNI。ユーザー判断により不採用。

### Option 4: magnitude 違反を auto-truncate で受理(不採用)

- 違反時に scalar を機械的に切り詰めて受理する案。
- **短所**: model が出力したものと異なる結果を success として返すため
  schema 検証が形骸化し、非準拠シグナルを隠蔽する。制約を擬装するより
  制約自体を撤く方が単純。

## Decision Outcome

1. 全 built-in input/output schema から `maxLength`・`maxItems`・
   `maximum` を削除する。
2. 構造制約(型・required・`minLength: 1`・`minItems`・`pattern`・
   enum・`additionalProperties: false`・`lines ≥1`)は維持する。
3. `maxFindings`/`focus.maxFindings` は Optional・`minimum: 1` のみと
   する(runtime は参照しない prompt 専用 field)。型を
   `["integer", "null"]` に拡張し、明示 `null` を省略と同義として
   受理する。
4. `grantRequests`(tool-operator.output)の制約は維持する — policy
   vocabulary 連動の構造制約であり magnitude ではない。
5. `bulk-reader-contract.md`(DESIGN-034r05iHrZUE6WxkSJi9GF)の上限表は
   本 ADR で magnitude 部分を supersede する — 文書は歴史的記録として
   残し、関係を relation で明示する。
6. prose 指示(worker.md の "at most a few sentences" 等)は変更しない —
   品質ガイダンスの責務は prose に残る。

## Consequences

- `INVALID_INPUT` が発生するのは構造違反(必須欠落・型違い・空文字・
  path pattern 違反)のみになる — `task` 長での reject は消滅。
- `INVALID_OUTPUT` は構造違反(JSON 非適合・required 欠落・型違い)専用に
  なり、「完走したが少し長い」run が廃棄される経路は消滅する。
- bulk-reader の「圧縮された findings」は schema 強制ではなく prose
  指示と model 品質に委ねられる — 肥大化した場合の観測は
  `usage.jsonl` の `meta.usage`/`savings` で可能。
- consumer 独自 worker の schema は引き続き作者の自由 — 本原則は
  built-in に適用する規約であり、worker 作者が magnitude を課すことは
  禁じない。
- SKILL.md(#108 で追加した limits 表)・guides・golden/regression
  fixture の追従が要る。
