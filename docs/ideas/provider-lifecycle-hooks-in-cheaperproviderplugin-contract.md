---
id: "IDEA-034r05iHy2gdnpfT84JZ5h"
type: "idea"
status: "parked"
title: "Provider lifecycle hooks in CheaperProviderPlugin contract"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/pull/31 (post-merge review, non-blocker)"
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
  related:
    - "../adr/provider-only-registration-via-explicit-bundled-plugin-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Provider lifecycle hooks in CheaperProviderPlugin contract

## Summary

`CheaperProviderPlugin` に provider lifecycle hook（例: post-logout）
を contract の面として持たせ、`logoutProvider` 等の runtime-pi 側
adapter が一箇所で dispatch する構造に拡張する。

## Problem and Motivation

- Devin plugin は Pi に provider 単位の logout hook が無いため、
  plugin 内で `runtime.logout` を instance 単位で wrap している
  （`packages/runtime-pi/src/providers/devin/plugin.ts`）。
- bundled provider が 1 件の現状では問題ないが、provider plugin が
  増えて lifecycle hook を必要とするケースが増えると、wrap の
  合成順序や責務の所在が暗黙になる。
- PR #31 レビューで non-blocker として指摘された deferred point。

## Expected Value

- hook の dispatch が runtime-pi 側の一箇所に集約され、plugin ごと
  の wrap 実装・合成順序を気にしなくて済む。
- contract 上で lifecycle hook の存在が明示され、plugin 作者が
  「何を登録してよいか」を type から読み取れる。

## Open Questions

- contract の hook shape（`logout(context)` 単発か、
  `onLogout`/`onLogin`/`onCredentialChange` の粒度か）。
- hook 呼び出し失敗時の扱い（provider logout 自体を fail させるか
  best-effort にするか）。
- `register` で wrap する現行方式と併用する場合の dispatch 順序。
- 必要になる trigger（2 件目の bundled provider、pre-login・
  credential migration 等の hook 要求）が実際に発生するか。

## Next Action

- [ ] Promote to discovery-doc for deeper exploration
- [ ] Promote directly to spec-doc if requirements are clear
- [x] Park for later reconsideration — bundled provider が devin
      1 件・必要 hook が logout のみの現状では contract 拡張は
      YAGNI。2 件目の provider や別 lifecycle hook の要求が出た
      時点で再検討する
- [ ] Discard — reason: <!-- why -->
