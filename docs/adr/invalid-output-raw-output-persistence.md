---
id: "ADR-zHFZ3QOA0Q6lEoWmS5gzIQ"
type: "adr"
status: "proposed"
title: "INVALID_OUTPUT の raw output を file へ保存し details に参照を返す"
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
  - "../adr/worker-schema-structural-validation-not-magnitude.md"
  - "../adr/budget-exhaustion-checkpoint-and-resume.md"
  - "../designs/worker-schema-structure-not-magnitude.md"
---

## Context

`INVALID_OUTPUT`(repair 尽くしても output schema 検証が失敗)のとき、
run は error `details` に `errors`(先頭20件)と `rawOutput`(**先頭
2000 文字**)を載せて捨てられる。schema の magnitude 撤廃(併走 ADR)で
`INVALID_OUTPUT` の発生は構造違反に限定されるが、構造違反でも model が
作った出力本体を回収する手段は依然として必要 — 親 agent が修正して
使う、次の run の入力に再利用する等。#114 は「rawOutput があると
分かっていれば救える」と指摘するが、2000 字 cap では大きな result を
実質回収できない。

### Forces

- envelope は親 agent の context に入る — `details.rawOutput` に全文を
  直置きすると数 MB 級の model 出力が error envelope に流れ、cheaper
  の offload 目的を failure path で破壊する。
- #111 で確立した先例:「`details` には安全な参照(path・count)を載せ、
  本体は `<userConfigDir>` 配下の file」— checkpoint 機構が同じ形で
  動いている(atomic write・best-effort・0o600)。
- raw output は JSON 非適合もあり得る(解析不能 output も
  `INVALID_OUTPUT` の対象) — file は生 text として保存する。
- workflow の **output materialization** が出す `INVALID_OUTPUT` は
  details shape が別系統(#108 レビューで確認済み)で、envelope に
  乗らない — 同仕組みをそのまま適用できない。一方 workflow の
  `kind: worker` job は同一の `run_worker`/`repair_output` を共有
  するため、worker 出力の `INVALID_OUTPUT` に限れば本機構は
  workflow 経路にも自然に及ぶ(job error details が workflow
  envelope にどう表れるかは別問題)。

## Considered Options

### Option 1: raw output を file へ保存し details に path を返す(採用)

- `INVALID_OUTPUT` 確定時に `<userConfigDir>/invalid-outputs/run-<id>.txt` へ
  raw output 全文を atomic write(temp + rename、unix mode 0o600 —
  tool 出力や機微を含み得るため credential と同じ権限)。
- `details` に `rawOutputPath`(または同等 key)を追加し、既存の
  `errors`(先頭20件)と `rawOutput`(先頭2000字 excerpt)は診断用に残す。
- 保存失敗は best-effort — error 分類・exit code を変えず path key を
  省略するだけ。
- **長所**: envelope は小さいまま、回収は完全。checkpoint の先例と
  同一パターンで実装面積が小さい。raw output の大きさに無関係に
  機能する。
- **短所**: file 参照のため回収に1手間要る;`<userConfigDir>/invalid-outputs/`
  という新規 write surface;retention/清掃機構がない(Open Items)。

### Option 2: `details.rawOutput` の cap を撤廃または拡大(不採用)

- excerpt を 2000 字から大幅拡大、あるいは全文化する案。
- **短所**: envelope が親 agent context に入るため、全文化は offload
  の目的を壊す。中途半端な拡大は依然として大きな result を救えない。

### Option 3: partial result を `result` field で返す(不採用)

- schema-invalid な出力を `result` として返し warning を併記する案。
- **短所**: envelope contract(`ok:true` の `result` は worker output
  schema 適合)を破壊する。消費者が「valid なはずの result」を疑う
  必要が生じ、影響面が大きい。

## Decision Outcome

1. `INVALID_OUTPUT`(run 経路・repair 尽くした後)で、raw output 全文を
   `<userConfigDir>/invalid-outputs/run-<id>.txt` へ atomic write する。
   `<id>` は checkpoint と同じ pid+nanos 生成系を使う。
2. envelope 最外層 error の `details` に file path を追加する。
   `PARTIAL_MUTATION` wrap 時は wrapper 側の details に付ける
   (envelope 直列化が inner cause details を落とすのは checkpoint と
   同じ)。既存 `errors`・`rawOutput` excerpt key は維持する。
3. 保存は best-effort — 失敗時は path key を省略するだけで error
   分類・exit code は不変。
4. file は生 text(JSON 非適合でもそのまま)、unix mode 0o600。
   retention/TTL は設けない(Open Items)。
5. workflow 経路の `INVALID_OUTPUT` は対象外 — details shape が別系統
   で envelope に乗らない。需要が実測されたら別途検討。

## Consequences

- 構造違反の `INVALID_OUTPUT` でも model の作った出力を完全に回収
  できる — 親 agent が修復して再利用する実務経路が成立する。
- envelope `details` への追加は additive で後方互換(既存 key 変更
  なし)。付与点は「envelope 最外層の error」で、wrap 経路は
  wrapper の details に付く。
- `<userConfigDir>/invalid-outputs/` は新規 write surface — checkpoint と
  同じ user config scope に置き workspace を汚さない。
- `details.rawOutput` excerpt(2000 字)は診断用途に限られ、回収経路の
  正規手段は file path になる — Open Items に excerpt 廃止の余地を
  記録。
