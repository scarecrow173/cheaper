---
id: "TSPEC-034zTTi4YDeiIdQUZuSsAC"
type: "test-spec"
status: "proposed"
title: "Workflow resource inference の ownership と safety 保証"
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
    - "../specs/workflow-resource-inference.md"

  derived-by: []
  verifies:
    - "../specs/workflow-resource-inference.md"
    - "../designs/workflow-resource-inference.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow resource inference の ownership と safety 保証

## Purpose
D4/D5 の semantic dependency と safety scope の区別を保護する。inference 契約廃止時のみ見直す。

## Feature
compile 前の manifest、ownership、依存、conflict 推論。

## Rules
### Rule: authoritative access と no-access
catalog と Worker writeScope を正本とし omitted/{} fallback と [] を区別する。known access の underclaim は拒否。
### Rule: dependency と conflict
known/declared read と produces の overlap は依存、unknown broad read は依存を発明しない。exclusive overlap は conflict。
### Rule: ownership identity
複数 producer の exact/subtree/alias overlap は needs があっても error。identity は core が所有。
### Rule: snapshot と決定性
全 job manifest、annotation clone、dedupe/cycle、caller mutation 隔離、input 順序によらない JSON。
### Rule: process source
named-task inputs/outputs は必須、undeclared process は INVALID_REQUEST。

## Examples
- AC1: catalog-read-write-edit-scopes、catalog-subtree-and-default、metadata-absent-fallback。
- AC2: worker-write-scope-targets、worker-read-only-and-unknown-mutation、worker-underclaim-rejection、worker-ancestor-mkdir-safety。
- AC3: produces-to-read、needs-from-resource-dedupe、inferred-cycle-rejection、no-self-edge。
- AC4: writers-conflict-without-edge、unknown-read-does-not-order、exclusive-read-write-overlap。
- AC5: duplicate-exact-output、subtree-overlapping-owners、canonical-alias-and-case-output、ordered-duplicate-still-rejected。
- AC6: omitted-empty-object-fallback、declared-empty-read-array。
- AC7: reversed-job-and-claim-byte-equality、no-input-mutation、full-manifests-on-injected-port、cloned-annotation-results。
- AC8: core-boundary-escape、glob-safety-and-ownership、component-boundary、core API import review。
- AC9: process-config-input-output-edge、undeclared-process-rejection、declared-empty-process-scopes。

## Guarantees
named cases が AC1–AC9 を保護。不存在 output と alias identity は core に委ねる。

## Non-goals
handler/model/provider、scheduler/ResourceManager/engine、OS tracing、dynamic discovery、artifact namespace。

## Risk
保証を失うと誤順序・ambiguous ownership・unsafe underclaim を成功として返す。
