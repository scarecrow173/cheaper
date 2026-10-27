---
id: "TSPEC-034N7wIDvgWFtQ1Yaklezu"
type: "test-spec"
status: "proposed"
title: "Route check layer and policy diagnostics"
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
    - "docs/adr/use-a-read-only-embedded-resource-boundary-for-route-check.md"
    - "docs/designs/route-check-read-only-diagnostics-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Route check layer and policy diagnostics

## Purpose

builtin/user/project 各 route layer と policy layer が、absent/loaded/failed・merge determinacy・valid sibling 保持の規約どおりに診断されることを保証する。layer/merge 規約が変更された場合にのみ廃止する。

## Feature

Feature: `route check` は各 route/policy layer を独立に診断し、merge status と layer 失敗を route 行を失わずに報告する。

## Rules

- 各 route layer は独立に評価され、absent `routes.yaml` は warning ではなく正常な空 layer として報告される。
- read/parse/schema failure はその layer のみの `failed` entry となり、valid sibling layer の route 行は保持される。failed layer 自身からは route 行を生成しない。
- production builtin source は embedded catalog（logical `/builtin/` root）から read in place され、temp dir への materialization なしに builtin diagnostics が成立する。
- merge status は determinacy 規約に従う: user layer が non-failed なら builtin route は `active`/`shadowed` を確定でき、failed なら `indeterminate`。valid な user route は常に `active`。
- project route は loaded な builtin/user の effective（非 invalid）route と `match.command` が一致すれば `suppressed`、無ければ builtin+user が両方 non-failed の場合のみ `active`、どちらか failed なら `indeterminate`。invalid bucket の下位 route は merge 対象外であり shadow/suppress に寄与しない。
- 同一 layer duplicate は他 layer の状態に関わらず常に `invalid` であり、name/command/layer/trust/`invalidReason` のみを持つ shallow row とする。`suppressed`/`shadowed`/`indeterminate` row は per-layer parsed `Route` に merge provenance を join した宣言値を保持する。`trust` は宣言 key ではなく layer provenance（builtin/user = `trusted`、project = `untrusted`）から導く。
- `RoutePolicyReport.layers` は builtin/user/project それぞれの `absent|loaded|failed` を保持する。per-layer 失敗は layer diagnostic、post-merge validation・default resolution・denylist 失敗は pseudo-layer `merged`・source `policy:effective` として報告する。
- shared policy/denylist load 失敗時も valid layer の route 行は残り、policy-dependent status は `unknown`/`unverified` になる。

## Examples

- Given user/project の `routes.yaml` が存在しない構成、when `route check` が layer を評価すると、両 layer は `absent` の正常空 layer となり merge determinacy が保たれる。
- Given malformed な project `routes.yaml`、when layer を評価すると、project layer が `failed` entry となり builtin/user の route 行は残る。
- Given builtin route と同じ `match.command` を持つ user route、when merge が評価されると、builtin row は `shadowed` になる。
- Given effective builtin route と同じ command を持つ project route、when merge が評価されると、project row は `suppressed` になる。
- Given 同一 layer 内の duplicate route 宣言、when merge が評価されると、name/command/layer/trust/`invalidReason` のみの shallow な `invalid` row になる。
- Given `builtin_dir: None` の production path、when builtin diagnostics が生成されると、embedded catalog 経由で builtin route/worker/schema が解決され false `missing-target` や worker/schema failure を報告しない。

## Guarantees

- absent layer が正常な空 layer、failed layer が per-layer diagnostic、valid sibling row の保持を固定する。
- `active`/`suppressed`/`shadowed`/`invalid`/`indeterminate` の determinacy 規約が design の merge 規約どおりであることを固定する。
- embedded builtin resource が materialization なしに診断できることを clean-temp・production-path fixture で固定する。
- `RoutePolicyReport.layers` の per-layer 状態と pseudo-layer `merged`/`policy:effective` attribution を固定する。

## Non-goals

- grant/pin/viability 等の policy-dependent 判定そのものの意味づけは readiness spec が担う。
- detection・logging suppression・no-mutation probe は invocation-boundary spec が担う。
- output envelope・exit mapping・sanitization は output contract spec が担う。

## Risk

これらの保証が欠けると、1 層の破損で全 route 診断が失われ、merge 状態が実際の resolution と乖離し、clean install で builtin diagnostics が成立しなくなる。
