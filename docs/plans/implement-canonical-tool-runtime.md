---
id: "PLAN-034zQ97RFO1AbY6enJw9jO"
type: "plan"
status: "completed"
title: "Canonical Tool Runtime を抽出して Pi adapter を接続する"
created: "2026-10-04"
updated: "2026-10-04"
owners: []
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/63"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/canonical-tool-runtime.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/canonical-tool-runtime.md"
    - "docs/designs/canonical-tool-runtime.md"
  derived-by: []
  verifies: []
  verified-by: ["docs/test-specs/canonical-tool-runtime.md"]
  references: []
  defers: []
  deferred-by: []
---

## 目的と scope

SPEC-034zILh8aauVSReoD33mtV と DESIGN-034zIUgKgoIuBD5BY6jLjf、共有 ADR D1/D7 を実装する。
read/grep/find/count_tokens の正本と journal を tool-runtime に移し、Pi と direct を同じ入口に接続する。
handler 抽出と adapter の切替は共通の critical path を持つ一つの計画とする。
core/workflow/cli、依存、hot file、scheduler/OperationManager/ResourceManager の実装は scope 外。

## File Map

| Path | 責務 |
| --- | --- |
| packages/tool-runtime/src/{read,grep,find,count-tokens,walker,line-framer,limits}.ts | Pi-free handler と既存 bounded I/O |
| packages/tool-runtime/src/{catalog,registry,accounting}.ts | 7 tool schema、canonical validation、一回の budget/meter |
| packages/tool-runtime/src/{journal,invocation,index}.ts | journal 正本、scope lifecycle、公開 API |
| packages/runtime-pi/src/tools/{registry,read,grep,find,count-tokens,budget,meter,walker,line-framer,limits}.ts | Pi adapter、helper 注入、互換 exports |
| packages/runtime-pi/src/{journal,index,run}.ts | journal re-export と D7 最小所有配線 |
| packages/tool-runtime/test/** | direct malformed/catalog/lifecycle/dependency guard |
| packages/runtime-pi/test/tool-runtime-contract.test.ts | 実 fixture parity、拒否、identity、accounting |
| packages/runtime-pi/test/{run,sibling-parallelism,no-execution-mode}.test.ts | owned/borrowed lifecycle と既存 pin |
| docs/test-specs、docs/tasks、docs/impl 下の本 chain | 契約と検証 evidence |

## API と実行経路

canonical handler port は `(input: unknown, signal: AbortSignal) => Promise<CanonicalToolResult>`。
canonical descriptor は inputSchema、outputSchema、label、description と handler を保持する。
入口の compileSchema は runtime 構築時に compile し、call ごとに一回 validate する。
helper injection adapter は toolCallId を内部生成し、scope と caller を結合した signal を渡す。
helper onUpdate/ctx は未使用なので no-op/undefined とし、prepareArguments/prompt metadata を Pi に保持する。
write/edit/ls schema は pinned helper schema と contract test で一致を検証する。
count_tokens は JSON text と同じ構造化値を details に返す。read の page details は保持する。

budget は handler の await より前に予約し、meter は成功 output の text のみ数える。
既存 public wrapToolBudget/wrapToolMeter は Pi-free accounting primitive に委譲し、registry は wrapper を再適用しない。
unknown/未注入 handler は UNKNOWN_TOOL、malformed は INVALID_INPUT として副作用前に throw する。

runWorker は execution が無ければ facade を一つ作り所有する。execution があれば child scope だけを所有する。
runAbort.signal を scope に渡し、requestAbort の全原因（timeout、外部 signal、maxTurns、maxToolCalls）を
その scope の AbortController に伝播する。root facade abort は全 scope を abort、child abort は sibling に伝播しない。
scope の journal/budget/meter は各 Worker 専用。finally で owned facade または borrowed child の dispose を await する。
drain は abortGraceMs（既定 5000ms）まで settle を待ち、期限後に tracking を放棄して有限時間で return する。
root/symlink policy 不一致、duplicate owner、未対応 parentLease は fail closed。
未実装 service/lease は型で明示し成功する no-op は作らない。effective concurrency policy の固定と scheduler は #57。

## Dependency Graph と実行手順

| Step | Depends on | 単一 action |
| --- | --- | --- |
| T1 | 承認 | contract/lifecycle/accounting test を作成する |
| T2 | T1 | targeted test を実行し新 API 不在による red を確認する |
| T3 | T2 | canonical handler/catalog/journal/accounting を抽出する |
| T4 | T3 | Pi thin adapter と helper injection を接続する |
| T5 | T4 | facade と runWorker finally 所有を実装する |
| T6 | T5 | targeted tests/typecheck を実行して green を確認する |
| T7 | T6 | 論理的な code/test/docs の変更を commit する |
| T8 | T7 | canonical check と index maintenance、exit-audit を実行する |

critical path は T1→T8。adapter/lifecycle が registry に依存するため inline で順次実行する。
承認後 test-spec を作成し plan verified-by に接続、task-doc と build_task_graph を作成する。

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| mutation ordering 漏れによる data loss | low | high | Pi helper operations を維持、same-file write→edit/partial mutation test |
| factory/旧 journal import の互換破壊 | medium | high | thin compatibility exports と instanceof/class identity assert |
| helper schema drift | medium | medium | pinned Pi definitions との schema equality |
| countFileTokens が abort を受け取れない | high | medium | admission abort と有限 drain、late settle を無害化 |
| 外部 provider 不在 | medium | low | deterministic stream seam を使い実 provider 証明を主張しない |

## Rollback

disk format migration は無い。public API/adapter の不整合が判明したら本 slice の論理 commit を
coordinator 統合時に revert し、mise run check と旧 journal identity/registry tests で復元を確認する。
作業中は無関係なファイルを戻さず、対象変更を修正する。依存や shared contract の変更は行わない。

## Verification Matrix

| Step | command / criteria | Pass |
| --- | --- | --- |
| T1/T2 | mise exec -- pnpm exec vitest run packages/tool-runtime/test packages/runtime-pi/test/tool-runtime-contract.test.ts | 新 API 不在の red、既存契約は破壊しない |
| T3 | tool-runtime tests と dependency guard | direct callable、schema malformed 拒否、副作用なし catalog |
| T4 | contract test + runtime-pi/test/tools | 同一実 fixture の content/details、deny/boundary/writeScope、truncation/count 一致 |
| T5 | facade lifecycle + run tests | owned dispose/borrowed isolation、abort、期限内 drain |
| T6 | mise exec -- pnpm exec vitest run packages/tool-runtime/test packages/runtime-pi/test; mise exec -- pnpm typecheck | budget/meter 一回、sibling pins、journal identity/order/partial mutation、exit 0 |
| T7 | git show --stat、doc-status audit | ownership 内の意味ある commit と整合文書 |
| T8 | mise run check; doc-maintenance plan→apply; graph exit-audit | check exit 0、index 整合、chain 終端、exit-audit pass |

schema default は core compileSchema によって適用されない。read offset/limit、grep/find limits は
handler の fallback を保持する契約を test-spec に明記する。
CLI e2e 20s timeout が発生した場合は該当 file を単独再実行し、両 evidence を report に記録する。

## Self-Review と Review Handoff

spec の全 AC を T1–T8 と verification に対応付けた。D1/D7 の契約を変更せず Pi helper を composition boundary に留める。
既存 approved design の finite drain と非ブロッキング notes を反映した。
plan 作成まで graph focus-required の承認済み例外で手動進行し、この plan を focus に router を再開する。
docs/designs/overview.md:49 の帰属 drift は hot file のため coordinator への follow-up とする。
未解決の実装 blocker は無い。plan 承認を coordinator に求め、承認まで本体を変更しない。

## 承認 evidence

coordinator msg_6d6b34d1d51a: APPROVED: plan、P0/P1 なし。test-spec を先に記述し T1 contract test を続ける。既存 runtime-pi/test/tools は互換 exports 経由で維持する。owned facade の abortGraceMs は deps.abortGraceMs（既定 5000）を使う。T3 は未注入 write/edit/ls の UNKNOWN_TOOL 拒否を assert する。
