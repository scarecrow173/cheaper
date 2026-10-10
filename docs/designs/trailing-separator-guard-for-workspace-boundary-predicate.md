---
id: "DESIGN-034D8wP2Y8qWiuviJbQINn"
type: "design"
status: "approved"
title: "Trailing-separator guard for workspace boundary predicate"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
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
    - "../specs/volume-root-workspace-boundary-acceptance.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/volume-root-workspace-boundary-check-double-separator.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/volume-root-workspace-boundary-acceptance.md"
    - "../adr/guard-trailing-separator-in-workspace-boundary-predicate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Trailing-separator guard for workspace boundary predicate

## Context

`crates/cheaper-core/src/workspace/paths.rs` の `is_inside_workspace_root`
は `c == r || c.starts_with(&format!("{r}{}", nodepath::SEP))` で
inside 判定を行う。root が path separator で終わる場合（`C:\`、`/`）
に prefix が二重化して配下 path を全て outside と誤判定する（issue
#77、TS 由来 defect の持ち越し）。

`resource_contains`（`crates/cheaper-core/src/resources/predicates.rs`
:36-40）には同形状の `ends_with(nodepath::SEP)` guard が既にあり、本
設計はその guard を workspace predicate にも適用するだけの最小補完。

## Scope

### In Scope

- `is_inside_workspace_root` の trailing-separator guard 追加
- volume-root の unit test（`is_inside_workspace_root_cases` 拡張 +
  `resolve_workspace_path` の volume-root case）
- `mise run check` の pass 確認

### Out of Scope

- `canonicalize_workspace_root` の root 正規化変更
- `nodepath` 層の実装変更
- `resource_contains`・walker・deny_paths 等の他呼び出し側変更
- golden fixture（`tests/golden/fixtures/workspace/paths.json`）の拡張

## Components and Boundaries

- `is_inside_workspace_root`（paths.rs）— 修正対象。pure string
  predicate であり、呼び出し側の canonicalize 済み root を前提とする。
- `resolve_workspace_path` — 呼び出し側として動作確認の対象。変更不要。
  同 predicate の call site は全 3 箇所: paths.rs:139（step 3 の
  boundary 判定）、paths.rs:167（step 4 の symlink target 再判定）、
  `crates/cheaper-tools/src/walker.rs:432`（walker の symlink target
  boundary 判定）。いずれも関数内修正で自動的に直り、個別変更は不要。

## Data and Control Flow

```text
resolve_workspace_path(root, input)
  -> nodepath::resolve(root, input)           # abs
  -> is_inside_workspace_root(root, abs)      # step 3 (修正対象)
       # r/c は case_insensitive 時に lowercased 済みの比較用文字列。
       # prefix は raw ではなく normalized 側（r）から構築する。
       prefix = r.ends_with(SEP) ? r : r + SEP
       c == r || c.starts_with(prefix)
  -> component walk (relative + join)         # volume root でも正常
```

## Non-Functional Constraints

- boundary は fail-closed を維持（誤許可に転じない）
- `case_insensitive` 比較 semantics 不変
- host `nodepath::SEP` のみを guard 対象とする（入力は canonicalize 済み
  root が前提。`resource_contains` と同一契約）

## Risks and Trade-offs

- `resolve_workspace_path` の実 FS test は platform symlink（macOS
  `/tmp` → `/private/tmp` 等）の影響を受け得るため、root 直下の
  nonexistent tail で構成する。これで component walk が早期 break し、
  symlink policy に左右されない。
- 非 canonical root（`C:/` 等の forward-slash root）が将来流入する
  場合は host SEP guard だけでは拾えない。現行呼び出し側は全て
  canonicalize 済みのため、契約として明記するに留める。

## Deferred Design Concerns

- workspace root の trailing separator の正規化責務は
  `canonicalize_workspace_root` に属するが、本変更では触らない。
  root に複数 trailing separator や mixed separator が来る運用が
  発生したら再検討。

## Verification Notes

- [ ] win32 build: `is_inside_workspace_root("C:\\", "C:\\resource-probe",
  true)` が `true`（`cfg!(windows)` gate。prefix は normalized 比較用
  文字列 `r` 側から構築すること — raw `root` から構築すると
  case-insensitive 時に不一致になる）
- [ ] posix build: `is_inside_workspace_root("/", "/a", false)` が
  `true`（`cfg!(unix)` gate）
- [ ] `resolve_workspace_path` が volume-root workspace で nonexistent
  tail の inside path を resolve する。root は platform の volume
  root（win32 は `env::temp_dir()` の drive root or `C:\`、posix は
  `/`）で、衝突しにくい一意名（例: `cheaper-volroot-probe-<pid>`）を
  使う。`resolve_workspace_path` は root の存在確認をしないため、
  nonexistent component で walk が早期 break して platform symlink
  に左右されず確定的。期待値も pin する（win32 例:
  `real = "C:\<name>\a"`、`display = "<name>/a"`）
- [ ] `is_inside_workspace_root("C:\\", "D:\\x", true)` が `false`
- [ ] 既存 boundary test が全て pass、`mise run check` が pass
