---
id: "DISC-034Aqz6hU4LHgqL2xB5dJS"
type: "discovery"
status: "active"
title: "One-shot workflow execution boundaries"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/62"
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
  derives-from: ["../ideas/one-shot-workflow-engine-and-run-cli.md"]
  derived-by: ["../specs/one-shot-workflow-engine-and-run-cli.md"]
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md", "../adr/keep-artifact-storage-invocation-local-and-bounded.md"]
  defers: []
  deferred-by: []
---

# One-shot workflow execution boundaries

## Exploration Goal

issue #62 engine と CLI を既存 compiler/runtime/store に結合するときの責務・lifetime・大値の運搬を探索する。Entry Decision は A-2: acceptance は既知だが admission と artifact consumer の具体化に選択肢がある。
Dispatch override: briefing-flow の SUBAGENT-STOP と common brief に従い指定 skill を inline 実行し、profile discovery と nested dispatch は行わない。

## Key Issues and Assumptions

- 確認済み: compiler.ts:122/159 が job input と個別 output の materialization/validation を所有。engine は再実装しない。
- 確認済み: invocation.ts:82/100/137 が job lease、child scope、owner/global execution metrics を公開する。
- 確認済み: runtime-pi/run.ts:271-297 は parentLease を渡すが compiled manifest を渡す seam がなく、narrow parent と default scope の containment 不一致が起こり得る。
- 確認済み: policy schema に workflow.maxJobs はまだ無いが merge.ts に project min restriction key がある。shared ADR D2 は default 1 と別 operation cap を要求する。
- 確認済み: artifact store と reader port は存在するが registry/read/invocation の consumer 配線は未導入。
- 確認済み: resource-inference.ts は Worker の ancestor mkdir safety のため root-exclusive writes を追加し、ResourceAccess.prepareScope は manifest writes が authoritative writeScope を越えると拒否する。compiled safety manifest を無条件に child の permission manifest として渡す配線は不適切。
- 確認済み: args.ts と cli.ts は workflow interface のみ。shared ADR D6 の run <path>、input 省略 {} を追加する。
- 制約: D1 dependency direction、D4 lease containment、D5 schema/binding、D6 private outputs、D7 invocation 単一性を維持する。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| engine が Pi と全 service を生成 | 呼出しは少ない | D1/D7 違反、single-worker の再実装 | 不可 |
| injected executors + CLI composition root | 既存 runtime/compiler を再利用、fake executor で契約検証可能 | lease と runtime borrowing の配線が必要 | 候補 |
| ready job の resource waiter が workflow slot を保持 | 単純 | resource-blocked job が無関係 job の起動を妨げる | 非推奨 |
| resource admission と executor capacity を分離 | conflict を edge にせず非 conflict job を実行 | admitted lease の cancellation cleanup が必要 | 候補 |
| 大きな結果を自動 reference 置換 | 自動で小さくなる | $from と declared schema の型・値を壊す | 不可 |
| 明示 artifact publication + bounded artifact read | 原 payload 型を変えず reference を運べる | canonical read schema と借用 publisher/reader の配線が必要 | 候補 |

## Tentative Conclusions and Hypotheses

- 仮説: engine は ready queue、terminal state、output projection のみ所有し、CLI が同じ runtime と store を全 executor へ渡すと責務が整合する。
- 仮説: runtime-pi の writeScope 抽出を core の一箇所に置き、execution binding に child scope の resourceManifest を追加すると narrow reads と parent の containment を揃えられる。親 lease には compiled safety manifest を使い、Worker child の writes は authoritative writeScope に制限した manifest を使う必要がある。具体的な一箇所の導出を design で確認する。
- 仮説: read の明示 artifact mode が store へ publication し、read が artifact source を byte cap 内で読む方式なら、大値を Worker prompt に強制 materialize せず運べる。従来 path read は維持する。
- 仮説: outer abort は cooperative executor の settle を待ち、grace 超過では outcome と実 settle を区別して lease を late settle まで保持する必要がある。

## Open Questions

- pre-implementation blocker: coordinator が spec と ADR intent summary を承認すること。artifact mode、policy cap の既定・merge、lease/capacity 分離は ADR/design gate で確定する。

## Promotion Candidates

- [x] spec-doc: one-shot behavior、public/private projection、CLI 互換、失敗と cancellation の acceptance。
- [ ] adr-doc: injected execution/admission/lifetime と explicit artifact consumer を具体化する。

## Sources and Relations

canonical issue Triage 2026-10-04、brief-62.md、brief-common-5.md、shared ADR D1-D7、現在の compiler/invocation/run/read/policy source を読んだ。外部モデル実験は行っていない。
