---
id: "SPEC-034zSQRLxNXgJDxjmUBBB8"
type: "spec"
status: "approved"
title: "Run-scoped artifact store"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/66"]
  implements: []
  implemented-by: ["PLAN-034zThvLa4bK934vj52jaC"]
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["DISC-034zSQKiU63uXPcMSjjodn"]
  derived-by: ["DESIGN-034zSR05NE5hXXhkTRLMgW", "PLAN-034zThvLa4bK934vj52jaC"]
  verifies: []
  verified-by: ["TSPEC-034zTn31vnH33RcM6g7VYS"]
  references: ["ADR-034zGZpjoQlppwkkOs5gur"]
  defers: []
  deferred-by: []
---

# Run-scoped artifact store

## What Should Be Built

1 workflow invocation が所有する bounded Artifact Store と consumer 向け読み取り contract。

## Why Now

大きな中間出力を下流 prompt や parent response に全文埋め込むと、delegation の token 削減を損なう。

## Users and Value

Workflow Engine、resource inference、canonical Tool Runtime の read 系 adapter が、producer を保持した reference と必要部分だけの読み取りを使える。

## Scope

### In Scope

- run-scoped reference/value metadata、storage/read/preview bounds の注入、cleanup、borrowed ArtifactReader port、metadata-only projection。

### Out of Scope

- Engine/CLI/bulk-reader 全面配線、core ResourceClaim の変更、永続 cache、remote store、cross-workflow ids、自動 build cache。

## Requirements

- 共有 ADR D4: artifact と workspace permission は別 namespace。artifact id は host path に解決しない。
- D5: reference に producer job provenance を保持して #65-B が data dependency を推論できる。既存 binding と literal の意味は変更しない。
- D7: store は invocation 所有、runtime は reader を借りる。completion/failure/abort で取得不能にする。
- artifact は process を超えて永続化しない。public identity に filesystem detail を露出しない。
- storage 超過を拒否し、preview は byte cap 以内に truncate。選択的 read にも注入 cap を適用する。
- cap 値は owning workflow invocation（engine）が供給する。artifact bounds の RuntimePolicy schema surface はこの slice 外で engine/policy 統合 slice が所有し、現状は store options への必須注入を契約とする。
- cleanup 後の metadata reference は provenance 専用で取得可能な handle ではない。明示 bounded materialize の copy は caller 所有。

## Acceptance Criteria

- [x] AC1: reference に opaque run/id、producer job/name、bytes、preview、truncated がある。
- [x] AC2: per-artifact / per-run overflow は明示拒否、失敗時に状態が変わらず、preview は UTF-8 byte cap 内。
- [x] AC3: completion、failure、abort の cleanup は deterministic/idempotent、storage が空、後続 read/write は失敗。
- [x] AC4: traversal、file/host path、malformed URI、別 run の reference を拒否。任意 filesystem access を行わない。
- [x] AC5: artifact source と workspace source は discriminated union で分離、artifact port は workspace source を拒否。
- [x] AC6: envelope 向け projection と既定 JSON serialization は metadata/選択 preview のみ。large content を dump しない。
- [x] AC7: borrowed reader は cleanup 後に content を保持・取得せず、signal listener が解除される。保持済み metadata は unavailable と表現する。

## Sources and Relations

issue #66/Triage と共有 ADR D4/D5/D7。token 削減の実測、dependency inference 本体、実 bulk-reader 統合は本 slice の成果として主張しない。
