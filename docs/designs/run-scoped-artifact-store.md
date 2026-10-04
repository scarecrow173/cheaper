---
id: "DESIGN-034zSR05NE5hXXhkTRLMgW"
type: "design"
status: "approved"
title: "Run-scoped artifact store"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/66"]
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["SPEC-034zSQRLxNXgJDxjmUBBB8", "ADR-034zTe2Gt1o8oAkX43ilEm"]
  derived-by: ["PLAN-034zThvLa4bK934vj52jaC", "TSPEC-034zTn31vnH33RcM6g7VYS"]
  verifies: []
  verified-by: []
  references: ["ADR-034zGZpjoQlppwkkOs5gur"]
  defers: []
  deferred-by: []
---

# Run-scoped artifact store

## Context

共有 ADR D4/D5/D7 を細化する。overview.md を参照し、workflow → tool-runtime の既存依存方向を維持する。

## Architecture and Public API

tool-runtime/src/artifact.ts に ArtifactReference、ArtifactSource/ReadSource（kind: artifact / workspace）、ArtifactReader を定義。workflow は同型を re-export し、store が port を実装する。逆向き import と既存 tool 実装変更は無い。
Reference は artifact URI、runId、producer: {jobId,name}、bytes、UTF-8 preview、truncated。URI は artifact://<run UUID>/<artifact UUID>。job/name は identity に埋め込まず provenance に保存し、host path の意味を持たせない。lookup は URI/run/provenance の整合を検証する。
ArtifactReference は getter/toJSON を持たない frozen plain JSON data object とする。job-result JsonValue 内で artifact:// URI + producer.jobId により走査可能であり、cloneJson の own enumerable data-only 契約を満たす。

## Storage and Bounds

RunArtifactStore(options) に maxArtifactBytes/maxTotalBytes/maxPreviewBytes/maxReadBytes を必須注入。全て非負 safe integer、read cap は正数。limit 超過は明示 error、put は変更前に検証し copy する。Map と byte counter は private。入力 byte を truncate して保存しない。UTF-8 preview は不正 byte を置換し、出力 byte cap と文字境界を守る。
put({producer,data}) は immutable reference を返す。重複 producer/name を拒否し provenance を一意にする。reader.read({kind:artifact,reference},{offset,maxBytes}) は cap 内の copy と EOF/next offset を返す。全 byte は明示的な bounded read のときだけ返る。
put の data は string | Uint8Array。string は UTF-8 byteLength、Uint8Array は byteLength を計上し、copy 前に storage cap を検証する。cap 値は owning invocation（engine）から必須注入し、RuntimePolicy の artifact schema 追加は engine/policy 統合 slice が所有する。

## Lifetime

withArtifactStore(options, async store => value) が finally dispose を保証する。optional signal は preabort を拒否し、abort 時に即 clear、listener を解除する。dispose は idempotent で entries/counter を消去、read/write は以後失敗。reader は store の private map を参照し、独立 content cache を持たない。先に返した copy を撤回する保証は無い。
abort 時 clear は tool drain と順序づけず、進行中 consumer の次の read は deterministic failure になる。tool drain/settlement は invocation runtime/engine が所有する。

## Envelope Projection

store.metadata({includePreview}) と toJSON は content を含めない。live metadata は availability: available、cleanup 後は storage と一覧が空になる。明示 projectArtifactMetadata(reference,{includePreview}) は caller が保持した metadata から availability: unavailable の final envelope 向け値を作り、store/content を保持しない。withArtifactStore の返却値は caller が選んだ metadata/reference。公開 final envelope は finally 後の unavailable projection を使う。CLI envelope 変更は行わない。

## Dependency Provenance

producer.jobId を #65-B inference が参照できる。既存 $from が metadata を運ぶ際も JSON clone で provenance が保持される。compiler/ref scanner に新 edge を追加する実装は #65-B に委ねる。

## Error Handling and Security

CheaperError の既存 INVALID_INPUT/POLICY_VIOLATION/ABORTED/INVALID_REQUEST を利用する。workspace source、invalid/foreign id、provenance mismatch は fail closed。filesystem import・path resolve は無い。

## Verification

workflow focused suite に容量原子性、UTF-8 preview、不正 id/foreign run、source 分離、metadata serialization、コピー隔離、completion/failure/preabort/inflight abort、borrowed reader 無効化を追加。lint/typecheck/build と scoped doc audit を evidence に残す。

## Deferred Integration

engine が store lifecycle と final projection のタイミングを所有し reader を runtime に注入する。実 adapter の token savings は配線・計測後のみ評価する。
注入 seam は engine 統合 slice が追加する InvocationToolRuntimeOptions.artifacts?: ArtifactReader。今回その options/interface を変更せず port 型を公開し、engine が同じ reader を借用注入できるようにする。
