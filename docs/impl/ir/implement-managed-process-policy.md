---
id: "IMPL-034A9AU7e20SHvZ2sH66sP"
type: "impl"
status: "in-progress"
title: "Implement managed process policy"
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
  implements:
    - "../../tasks/implement-managed-process-policy.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement managed process policy

## Summary

ProcessPolicy の strict config、named task 解決、pure resource 宣言を core に実装した。executor はこの slice では公開しない。

## Context

task A / approved plan、spec・ADR・design 承認 msg_126e0d3e23d0、plan 承認 msg_e3d23f7a3334 に従う。

## Implementation

### Changes

core policy/process/schema/load/index、builtin policy ×2、policy tests を変更。legacy omission 補完後に既存 min merge を適用し、project が cap を拡大できない。ResourceManifest の glob/ownership 正規化は core の既存実装を使う。

### Decisions During Implementation

argv は shell syntax と既知 shell/shim を拒否。cwd/env/config は schema/resolver で検証、task/env は clone する。output glob は filesystem を列挙せず core ownership 検証へ渡す。

## Related Experiments

### Adopted

なし（既知の設計に従う）。

### Rejected

なし。

## Validation

- RED: `mise exec -- pnpm exec vitest run packages/core/test/policy/process.test.ts` — 30 failed（未実装 config/API）。
- GREEN: `mise exec -- pnpm exec vitest run packages/core/test/policy packages/core/test/resources.test.ts` — 167 passed、2 platform skips、8 files passed。
- `mise exec -- pnpm lint` / `pnpm typecheck` / `pnpm build` — pass。lint の既存 warnings は変更範囲外。
- builtin policy 完全一致期待を process default に更新した。
- implementation review は B 完了時に同じ change 全体を coordinator へ提出するため、IR は in-progress を維持する。

## Risks

declared I/O は OS tracing/sandbox ではない。executor/lifecycle は task B の責務。

## Follow-ups

task B の実装と coordinator review。canonical check deferred to coordinator merge run。
