---
id: "TSPEC-034zTn31vnH33RcM6g7VYS"
type: "test-spec"
status: "proposed"
title: "Run-scoped artifact boundaries"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/run-scoped-artifact-store.md"
  derived-by: []
  verifies:
    - "docs/specs/run-scoped-artifact-store.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Run-scoped artifact boundaries

## Purpose

容量・namespace・lifetime の境界が将来の adapter 配線で消えないようにする。run-local store 自体を廃止し別契約で代替する際のみ retired とする。

## Feature

Workflow の大きな中間出力を metadata と bounded read で受け渡す。

## Rules

- cap 超過を保存前に拒否し、既存 artifact を壊さない。
- run identity と producer provenance は改変不能な JSON metadata。
- source namespace を分け、path escape/foreign run は取得不能。
- finally/abort は content と取得権限を終わらせる。read copy は caller 所有。

## Examples

| AC | Named test（packages/workflow/test/artifacts.test.ts） | Example |
| --- | --- | --- |
| AC1 | retains provenance and plain JSON metadata with an opaque artifact URI | A1/stdout の 12 UTF-8 bytes を保持、binding clone で producer を保存 |
| AC2 | rejects per-artifact and total overflow atomically | 8 byte artifact cap / 12 byte total で境界許容・超過拒否 |
| AC2 | bounds UTF-8 and invalid-byte previews | 日/emoji の文字境界、U+FFFD byte 膨張も cap 内 |
| AC2 | preserves a UTF-8 BOM in bounded preview metadata | BOM を黙って落とさず byte cap 内に保持 |
| AC3/7 | withArtifactStore disposes on completion and failure | callback success/rejection 後に borrowed reader が取得不能 |
| AC3/7 | rejects pre-aborted signals without invoking the callback | abort 済み signal は store を使わせない |
| AC3/7 | clears immediately on inflight abort and removes its listener | delayed consumer の next read は ABORTED、listener 解除 |
| AC3/7 | keeps the invocation signal fixed when caller options change | options の後続変更で abort を成功扱いにしない |
| AC4 | rejects foreign runs, malformed URIs and altered provenance | artifact:// URI format、../、file URI、host path を拒否 |
| AC5 | rejects a workspace source at the artifact reader boundary | workspace/path が artifact lookup に入らない |
| AC6 | serializes metadata only and supports includePreview | large content 非 dump、final availability unavailable、preview 選択 |
| AC7 | returns bounded copies without retaining caller mutations | input/output copy 改変で保存 byte 不変、dispose は冪等 |

## Guarantees

full source を parent response に自動展開しない。全 storage/read/preview は owning invocation 注入 cap に従う。参照 metadata は cleanup 後も記録として保持できるが取得可能性は無い。

## Non-goals

Engine/CLI/tool adapter 全面配線、実モデル token savings、#65-B dependency inference 本体は検証しない。

## Risk

境界を落とすと cap bypass、foreign run の data 漏えい、abort 後取得、metadata output の大容量 dump が起きる。
