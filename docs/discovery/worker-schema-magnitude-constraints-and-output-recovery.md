---
id: "DISC-z4FTQSzd835hAlRqSPb7QY"
type: "discovery"
status: "resolved"
title: "worker schema の magnitude 制約と INVALID_OUTPUT 回復不能性の構造分析"
created: "2026-10-17"
updated: "2026-10-17"
owners: ["scarecrow173"]
relations:
  source:
    - "crates/cheaper-cli/resources/workers/schemas/bulk-reader.input.schema.json"
    - "crates/cheaper-cli/resources/workers/schemas/bulk-reader.output.schema.json"
    - "crates/cheaper-cli/resources/workers/schemas/code-writer.input.schema.json"
    - "crates/cheaper-cli/resources/workers/schemas/code-writer.output.schema.json"
    - "crates/cheaper-cli/resources/workers/schemas/doc-writer.input.schema.json"
    - "crates/cheaper-cli/resources/workers/schemas/doc-writer.output.schema.json"
    - "crates/cheaper-cli/resources/workers/schemas/tool-operator.input.schema.json"
    - "crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json"
    - "crates/cheaper-core/src/worker/validate.rs"
    - "crates/cheaper-runtime/src/run/repair.rs"
    - "crates/cheaper-runtime/src/run/output.rs"
    - "crates/cheaper-runtime/src/run/checkpoint.rs"
    - "crates/cheaper-rig-adapter/src/agent.rs"
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
    - "../adr/budget-exhaustion-checkpoint-and-resume.md"
    - "../specs/worker-schema-structure-not-magnitude.md"
    - "../adr/worker-schema-structural-validation-not-magnitude.md"
    - "../adr/invalid-output-raw-output-persistence.md"
    - "../designs/worker-schema-structure-not-magnitude.md"
    - "../plans/worker-schema-structure-not-magnitude.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/worker-schema-structure-not-magnitude.md"
    - "../adr/worker-schema-structural-validation-not-magnitude.md"
    - "../adr/invalid-output-raw-output-persistence.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/bulk-reader-contract.md"
    - "../designs/bulk-reader-output-limits-and-invalid-output-skill-docs.md"
  defers: []
  deferred-by: []
---

# worker schema の magnitude 制約と INVALID_OUTPUT 回復不能性の構造分析

issue #113・#114 のコード調査記録。#113 は `bulk-reader` の `task` 入力が
約 4000 文字超で `INVALID_INPUT` になる(上限未文書化)、#114 は完走済み
run が `findings[].title` の 120 文字超過一件で `INVALID_OUTPUT` となり
結果ごと廃棄される、という 2 件。ユーザーとの設計対話で「schema は構造を
検証し、magnitude(文字数・件数)を検証しない」という統一原則と、
`INVALID_OUTPUT` の raw output を file へ保存して回復可能にする方向が
確定した。

## Exploration Goal

1. magnitude 制約(`maxLength`/`maxItems`/`maximum`)の根拠と実態を確定する
2. 同系制限を全 worker schema と runtime に網羅的に棚卸しする
3. output schema が model にどう渡るか(structured output か prompt 埋め込みか)
   を確定し、magnitude 撤廃の波及面を特定する
4. `INVALID_OUTPUT` 時の raw output 回復可能性を機構レベルで評価する

## Observed Mechanism(事実確定)

### `task` maxLength 4000 の出自 — 根拠なしの設計値

- `docs/designs/bulk-reader-contract.md`(DESIGN-034r05iHrZUE6WxkSJi9GF)で
  「1–4000 文字」と表形式で定められたのみ。根拠の記述は一切ない。
  Rust port(`6e2e49d`/`6397ece`)時にそのまま引き継がれた。
- #113 が「regression」と呼ぶ点は不正確: `maxLength: 4000` と
  `validate_worker_input` 呼び出しは 7f687cf 時点でも同一。以前通った
  なら PATH 上の install 済み binary が古かった可能性が高い
  (`cargo install` 配布のため binary は repo HEAD に追従しない)。
- worker 間で不整合: `code-writer`/`doc-writer` の `task` は 6000、
  `bulk-reader`/`tool-operator` は 4000 — 恣意的な証拠。
- `paths` は workspace 相対 path のみ受理(pattern 制約)するため、
  **inline でしか渡せない内容**(貼り付け diff・外部 text・workspace 外
  資料)は `task` に入れるしかなく、4000 は実在の用途を機械的に潰す。

### output schema の model への到達経路 — structured output ではない

- `spec.output_schema` は provider 向け hint field だが、**どの provider
  も読まない**(`devin/rig_provider.rs` は `let _ = spec.output_schema`)。
- 実際の用途は 2 つ: (a) `build_system_prompt` が "Output schema:" 節に
  `JSON.stringify(schema, null, 2)` を埋め込む model 向け指示、
  (b) `extract_output` による事後検証 → `INVALID_OUTPUT`/repair。
- したがって schema から magnitude を撤くと「指示文からも検証基準からも
  消える」。品質ガイダンスは worker.md の prose("at most a few
  sentences"、"excerpts ≤600 characters and 20 lines")が担い続ける。

### `maxFindings` — runtime が一切参照しない prompt 専用 field

- `maxFindings` はコード内の参照がゼロ — input JSON が user prompt に
  そのまま埋め込まれるのみ。Optional/null 化に機械的な懸念なし。
- `bulk-reader.md` の指示文も "Respect `maxFindings` **when given**" と
  条件付き表現で Optional と整合済み。

### `INVALID_OUTPUT` の廃棄機構

- `repair.rs`: schema 違反時は `repairAttempts`(builtin=1)回だけ repair
  prompt を送り、直らなければ `invalid_output_error` で `INVALID_OUTPUT`。
- `details` = `errors` 先頭 20 件 + `rawOutput` **先頭 2000 文字のみ**。
  大きな result(1M token run 等)は `rawOutput` からも完全には取り出せず
  実質消失。#114 の「rawOutput から復元できる」は 2000 字で切られるため
  実際には部分的救出にしかならない。
- envelope は親 agent の context に入るため、`details.rawOutput` に
  全文を直置きする案は offload 目的を failure path で破壊する —
  #111 の checkpoint と同型の「details は安全な参照、本体は file」が
  一貫する。

### 同系制限の棚卸し(網羅)

- **撤廃対象(magnitude)**: 全 built-in input/output schema の
  `maxLength`・`maxItems`・`maximum`(下表)。
- **維持(構造/セキュリティ)**: `path` pattern、`required`、`minLength:1`、
  `minItems`、`additionalProperties: false`、enum、`lines.start/end ≥1`、
  grant name pattern(`^[a-z][a-z0-9_-]{0,63}$` — policy propertyNames と
  連動)、`tool-operator.output` の `grantRequests`(executable ≤4096・
  argv ≤256×≤4096・name ≤64 — policy vocabulary と照合する構造制約)。
- **別クラス(対象外)**: policy 値(timeout/maxTurns/maxToolCalls/
  maxTokens/repairAttempts/maxOperations)、tool pagination
  (read 50KB/2000 行・grep batch 1MB・find ≤1000・ls 50KB・
  count_tokens 16MiB — 全て「続きを読む手段」を伴う機能的上限)、
  内部 cap(`details.errors` 20 件、checkpoint entry 2000 字/100KB、
  hook routing scan cap)。

## Alternatives Considered

| 案 | 評価 |
| --- | --- |
| A. `task` 上限を 6000/32k へ引き上げ | 恣意的な別の値を選ぶだけで、inline-only 用途の根本解決にならない |
| B. magnitude 制約を全撤廃し schema は構造のみ検証(採用方向) | 「schema=構造 contract・prose=品質ガイダンス」の綺麗な分離。#114 の「些細な違反で完走 run を廃棄」という失敗様式自体を根絶する |
| C. worker 定義に failure-mode knob を追加 | 失敗要因で挙動が変わるのに一様な knob では表現不能。実際の選択肢は「捨てる/そのまま返す」の 2 択しかなく YAGNI |
| D. scalar 長超過を auto-truncate で受理 | schema 検証が形骸化し、model の非準拠シグナルを隠蔽する。擬装を入れるより制約自体を撤く方が単純 |

## Open Items

- magnitude 撤廃後、bulk-reader の「短い findings に圧縮」目的は prose
  指示のみに依存する — 肥大出力の暴走は品質問題として受容(親 agent は
  肥大結果を加工できるが、廃棄された結果は取り戻せない)。
- `details.rawOutput` の file 回復は run 経路のみ — workflow 側
  `INVALID_OUTPUT` は details shape が別系統(#108 レビュー指摘済み)。
  workflow への適用は需要が確認されてから。
- golden fixture(`load.json`/`schema.json` corpus)・regression
  fixture(`run-invalid-output` 系)の expected 更新が要る。
- APM skill 文書(#108 で追加した limits 表)が obsolete になる —
  書き換えが要る。

## Sources and Relations

- 一次ソース: GitHub issue #113・#114(scarecrow173)
- 上流 spec: SPEC-034r05iHK8bA7g6PcCiDBp(bulk-reader)、
  SPEC-034r05iHK8bA7g59V25ltC(v1 runtime)
- 関連 design: DESIGN-034r05iHrZUE6WxkSJi9GF(bulk-reader contract —
  上限値の出元)、bulk-reader output-limits skill docs(#108)
- 関連 ADR: budget-exhaustion-checkpoint-and-resume(#111 — file 参照
  方式の先例)
