---
id: "DISC-034zSP8CzaOAsOOhj0ZpcW"
type: "discovery"
status: "resolved"
title: "One-dispatch release workflow"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/73"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/version-pinned-launcher-acquisition.md"
    - "../adr/version-pinned-launcher-acquisition.md"
    - "../designs/version-pinned-launcher-acquisition.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../ideas/one-dispatch-release-workflow.md"]
  derived-by:
    - "../specs/one-dispatch-release-workflow.md"
    - "../adr/one-dispatch-release-workflow.md"
    - "../designs/one-dispatch-release-workflow.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/version-pinned-launcher-acquisition.md"
    - "../../scripts/test/launcher-acquisition.test.mjs"
  defers: []
  deferred-by: []
---

# One-dispatch release workflow

## Exploration Goal

#73-A の固定 version launcher に対応する公開物を、一度の dispatch から作成できるかを調べる。Entry Decision は A-4（brief に scope / acceptance が明記済み）。dispatched worker の明示 skill 指示により briefing-flow の SUBAGENT-STOP を適用し、idea / discovery / spec / ADR の文書 delegate を直接実行する。

## Key Issues and Assumptions

- 確認済み: reusable build-executables は3 native runner で SEA build / --version / models smoke を行い、strict_freshness=false の採取経路と true の既存 release 経路がある。
- 確認済み: launcher の asset は cheaper-linux-x64 / cheaper-linux-arm64 / cheaper-windows-x64.exe。v<version>/SHA256SUMS に64 hex + ASCII space 2個 + basename の行を要求する。
- 確認済み: APM install は local package path を扱える。既存 build-apm-bundle は checkout の payload を更新するため、新 release 組立では staging のコピーにだけ同一 run の executable を入れる。
- 制約: packages/**・依存・LFS・旧 workflow の mandatory 経路は保持。新 script は Node stdlib のみ。実 Release dispatch は行わない。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 新 workflow + reusable native build + staged bundle | 同一 run の raw asset と bundle が一致、旧経路を保持 | 移行期間に2経路併存 | 推奨 |
| 既存 release の strict check を外す | file 数は少ない | 現行 mandatory 経路を破壊する | 不採用 |
| LFS artifact 往復を継続 | 既存契約を維持 | #73-B の1 dispatch を満たさない | 旧経路のみ |

## Tentative Conclusions and Hypotheses

- 提案: release-acquisition.yml を追加。入力 tag を commit SHA に解決・固定し、package / version.json / APM version を照合してから reusable build を strict_freshness: false で呼ぶ。既存 caller の true と byte-match gate は保持する。
- 提案: raw3 asset を検証して SHA256SUMS を生成し、staged packages/apm に同じ executable と manifest を入れ cheaper-apm-v<version>.tar.gz を組み立てる。これは plain staged package snapshot（extract + apm local path install 用）。apm pack は arbitrary file を落とすため使用しない。SHA256SUMS は cli/SHA256SUMS に同梱。git checkout の payload は変更しない。
- manifest invariant: UTF-8 ASCII subset、LF only（CR 禁止、Windows CRLF も拒否）、64 hex + ASCII space 2個 + asset basename、空行なし、64KiB 以下、3 target 各 exactly 1 entry、全 basename duplicate 禁止。権威は acquisition design の Release Contract と launcher-acquisition.test.ts。
- 提案: 公開前に bundle を native3 runner で展開し、APM local install → launcher first run を smoke。publish job のみ contents:write。draft に全 asset を upload してから公開し partial public release を避ける。既存 tag Release は拒否し上書きしない。
- CI の native runner evidence は未取得。local stdlib unit / dry-run / workflow structure で代替し、実3 runner PASS とは主張しない。

## Open Questions

- pre-implementation blocker: ADR intent、spec / ADR / design / plan の coordinator 承認。
- 管理可能な後続: #73-C の git payload / CI / check script 移行。旧 SEA ADR は SEA 選択を保持するので related でリンクし全体 supersede しない。

## Promotion Candidates

- [x] spec-doc: 固定 asset / manifest / version / native smoke / additive workflow の acceptance を SPEC-034zSQV3zqlEncx2GoLKSR に提案。
- [x] adr-doc: staging bundle と native build、draft publish を持つ1 dispatch 経路を ADR-034zThoekpLqCcHhYO4MPe に提案。

## Sources and Relations

- #73 Triage 2026-10-04、coordinator brief-73b.md / brief-common-2.md、既存 build-executables.yml / release.yml / launcher、ADR-034zIOoBXZtluQdBMlJQge。
