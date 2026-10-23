---
id: "DISC-034KTzzHPIbmtD8j3iHBtg"
type: "discovery"
status: "active"
title: "route grant-set 投影 + viability 整理 + record split（#166）"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/166"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "../discovery/permission-model-responsibility-redesign.md"
    - "../impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "../impl/ir/unified-exec-grant-model-issue-165.md"
  defers: []
  deferred-by: []
---

# route grant-set 投影 + viability 整理 + record split（issue #166）

## Exploration Goal（seed — briefing 前の最小記録）

issue #166（#151 実装分解 4/7）。route/hook 側で内容審査に依存
する分岐を整理し、非 exec routing deferred sketch
（ADR-034KtTzOJGqmzmBW3SwGp5）の採用要素 — grant-set 投影・
条件付き viability 弱化・record split — を実装する。
routing ≠ authorization（D2）は維持。

### Scope（issue 本文）

- **viability の内容審査呼出し整理** — viability は authorization
  （grant/allowlist 検査 = `check_command_execution` 委譲）を
  呼ぶが内容審査は呼ばない構造へ。
- **`project_grant` → route 宣言 `grants:[name,…]` set 投影へ
  一般化**（統一 grant モデル上 — #165 済）。grant 参照は
  trusted-scoped のみ解決（M3）。confused-deputy 半径は
  declared grant argv union に明示的に閉じる。
- **viability の条件付き弱化** — byte-matched mode では原
  argv = 実行 argv（D11）のため `check_command_execution`
  argv precheck（`resolve.rs:304-335`）は**維持**（無条件に
  弱めると argv-mismatch が「route 不発→原 command 直接実行」
  から「route 発火→exec job 失敗」へ後退）。弱めるのは set
  内の非 exec grant のみ — 存在・pin 解決可能性のみ
  viability で検査し argv 照合は job 実行時へ委譲（direct
  `workflow run --grant` と同一）。
- **`kind:"route"` usage record を `{command, executed}` に
  拡張** — 実行されなかった command を attribution にしない。
  route 帰属（route × firing gate の認可 grant）は
  `route.grant` に anchor。`executed` は実 exec argv（byte-
  matched では原 argv）を記録。
- **grant anchor 規約**: `expected_input.name` は exec job が
  実際に走らせる grant（byte-matched/grant-set mode では
  `route.grant`）。deadline/timeout 等の lifecycle bound は
  `expected_input` が指す実行 grant の宣言値（`command::execute`
  が named grant の `timeout`/`killGrace`/`exitCodes` を読む
  既存規約）。set 内の非 exec grant は呼ぶ追加 job で自身の
  宣言 bound を使う。

### Acceptance Criteria（issue 本文）

- viability が authorization を呼ぶが内容審査を呼ばない構造
- `grants:[name,…]` set 投影が trusted-scoped のみ解決
- byte-matched の argv precheck 維持・argv-mismatch で route
  不発→原 command 直接実行 fallback 保持
- set 内非 exec grant が存在+pin 解決可能性のみ viability 検査
- `kind:"route"` record が `{command, executed}` で attribution
  が `route.grant` に anchor

### seed の前提

- deps: #163・#165 両方 merged（PR #172・#174）。
- 非 exec routing 全体は #167（substitute-exec mode）の scope —
  本 issue は grant-set 投影・viability 整理・record split に
  限定。
- `argv_shell_re` viability 参照（`resolve.rs`）は #163 で
  「viability 側の審査依存整理は本 issue の scope」と記録済み —
  briefing で現行呼出しを実測。
- 引用 file:line は issue 本文のもの（main `c45872c` 時点 —
  #165 後の drift を briefing で実測修正する）。

## Key Issues and Assumptions

- briefing で確定する。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| briefing で確定する | | | |

## Tentative Conclusions and Hypotheses

briefing で確定する。

## Open Questions

- briefing で確定する。

## Promotion Candidates

- [ ] spec-doc needed: yes（set 投影・record schema・弱化条件の intent）
- [ ] adr-doc needed: yes（viability 弱化条件・anchor 規約の決定記録）

## Sources and Relations

- issue #166・親 issue #151 chain・非 exec routing deferred ADR
- #163 IR（argv_shell_re viability 参照の記録）・#165 IR（統一
  grant モデル・`is_route_eligible`・`project_grant` 投影）
