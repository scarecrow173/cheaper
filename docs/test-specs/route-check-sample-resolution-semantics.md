---
id: "TSPEC-034N7wJQaJuDb95eugnQne"
type: "test-spec"
status: "proposed"
title: "Route check sample resolution semantics"
created: "2026-10-27"
updated: "2026-10-27"
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
    - "docs/specs/route-check-diagnostics-issue-185.md"
    - "docs/designs/route-check-read-only-diagnostics-boundary.md"
  derived-by: []
  verifies:
    - "docs/specs/route-check-diagnostics-issue-185.md"
    - "docs/designs/route-check-read-only-diagnostics-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Route check sample resolution semantics

## Purpose

`route check` の sample-aware evaluator が `route resolve` と同一の match/viability 意味を保ち、no-sample・layer failure・policy 不在時の確定範囲が正しいことを保証する。sample resolution 規約が変更された場合にのみ廃止する。

## Feature

Feature: `route check` は指定された sample を shared match/viability 規約で評価し、sample 未指定や条件不足の場合は `requires-sample-input`/`unknown`/`indeterminate` として決して runnable と断定しない。

## Rules

- sample は `--command`（shared `parse_command` seam）または `--argv`（shared `parse_argv` seam）のいずれか 1 つで供給され、XOR（0 または 1 個）とする。
- `matched` は normalized argv がその row の宣言 `match.command` を prefix として満たすこと（suppressed/shadowed/invalid row の宣言 command も対象）、`selected` は effective route の longest-prefix winner であることを意味し、env-prefix strip・self-bypass・prefix match・longest/tie は shared `match_command` phase を使う。
- `selected: true` と `viability` は effective row にのみ存在し、non-effective row を含む non-winner は `matched: true, selected: false` で viability を報告しない。
- 全 layer が non-failed で policy が利用できる場合のみ shared resolve/viability を呼び、unsafe-command → invalid-route → missing-target → no-grant → stdin-unsupported → runnable の固定優先順を再利用する。`missing-target` 判定は source-aware `target_exists` predicate で行う。
- 全 layer が non-failed で policy が無い場合、`matched`/`selected` は `match_command` で確定できるが winner の `viability` は serialized `unknown` とする。policy-independent な unsafe/invalid/missing-target も viability としては部分評価しない。
- 1 つでも route layer が `failed` の場合、runtime の effective set を再現できないため全 row の `matched`/`selected` を `indeterminate`、winner `viability` と suppressed attribution を省略し、`route.sample-indeterminate` diagnostic を付ける。
- sample が無い場合、sample `status` は `requires-sample-input`、`matched`/`selected` は `not-applicable`、winner `viability` と suppressed attribution は省略し、いかなる route も `runnable` と断定しない。`suppressed` attribution は selected winner の command に prefix match する suppressed route がある場合のみ、`RouteResolution.suppressed` と同じ route-name 値で serialize する。

## Examples

- Given sample を指定しない invocation、when report を生成すると、sample status は `requires-sample-input`、matched/selected は `not-applicable`、viability は省略される。
- Given policy と全 layer が valid な構成に matching sample、when viability を評価すると、`route resolve` と同じ固定優先順で `runnable`/`no-grant`/`missing-target` 等が返る。
- Given winner とは別に宣言 command が prefix match する suppressed route、when sample を評価すると、suppressed attribution にその route name が載る。
- Given prefix match するが winner でない route row、when sample を評価すると、`matched: true, selected: false` で viability は省略される。
- Given 1 つの layer が `failed` の構成に sample、when match を評価すると、全 row の `matched`/`selected` が `indeterminate` になり `route.sample-indeterminate` diagnostic が付く。
- Given policy 不在で layer が全て valid、when sample を評価すると、`matched`/`selected` は確定し winner の `viability` は `unknown` になる。

## Guarantees

- match/viability が shared `match_command` と resolve viability の同一規約・同一優先順に従い、`route resolve` との parity を固定する。
- no-sample で `runnable` にならないこと、layer failure で `indeterminate`、policy 不在で winner viability `unknown` であることを固定する。
- non-winner と suppressed attribution の serialize 条件を固定する。
- sample-derived な `RouteResolution.executable` が `execReadiness` に混入しないことを固定する。

## Non-goals

- grant/pin・workflow readiness 等の静的判定自体は validation-readiness spec が担う。
- raw sample 値の出力 sanitize は output contract spec が担う。
- no-spawn/no-network probe は invocation-boundary spec が担う。

## Risk

これらの保証が欠けると、no-sample でも runnable と誤報され、layer failure 時に架空の winner が報告され、diagnostics の match/viability が `route resolve` の実挙動から drift する。
