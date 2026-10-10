---
id: "DISC-034D840CvSzto0OH0dO47y"
type: "discovery"
status: "draft"
title: "Volume-root workspace boundary check double separator"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/77"
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
    - "docs/tasks/phase-1-port-cheaper-core.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/volume-root-workspace-boundary-acceptance.md"
    - "../adr/guard-trailing-separator-in-workspace-boundary-predicate.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Volume-root workspace boundary check double separator

## Exploration Goal

issue #77（`isInsideWorkspaceRoot` が volume-root workspace で false を返す）
の bug-fix work chain の設計判断に必要な事実を確定することが目的。
TS 時代の defect が Rust port（`crates/cheaper-core/src/workspace/paths.rs`
の `is_inside_workspace_root`）にそのまま持ち越されている。

## Key Issues and Assumptions

- **confirmed**: `is_inside_workspace_root`（paths.rs:103-110）は
  `c == r || c.starts_with(&format!("{r}{}", nodepath::SEP))` で判定する。
  root が `C:\`（POSIX なら `/`）の場合 prefix が `C:\\` / `//` に二重化し、
  `C:\resource-probe` / `/a` が prefix match しないため `false` を返す。
- **confirmed**: `resolve_workspace_path` の algorithm step 3 が同 predicate を
  呼ぶため、filesystem root を workspace root にした run では root 自身
  （`c == r` で通る）を除く全ての配下 path が `outside-workspace` で reject
  される（boundary check は fail-closed なので誤許可ではなく誤拒否。
  security 上の過剰拒否であり escape ではない）。なお同 predicate は
  `crates/cheaper-tools/src/walker.rs:432` が symlink target の boundary
  判定にも呼んでおり、volume-root workspace では within-workspace policy
  の symlink target が全て silently skip される第 2 の影響経路がある
  （修正で同時に直る）。
- **confirmed**: 同形状の ancestor prefix 判定が
  `crates/cheaper-core/src/resources/predicates.rs` の `resource_contains`
  にも存在するが、こちらは `parent.ends_with(nodepath::SEP)` の guard 付きで
  既に正しい（#65-A の ResourceManifest 実装時に導入）。同一リポジトリ内に
  正規形の参照実装が既にある。
- **confirmed**: `canonicalize_workspace_root` は `dunce::canonicalize` の
  戻り値をそのまま返し、volume root（`C:\`）では trailing separator が
  残る。predicate 側で吸収するのが contract として正しい。
- **confirmed**: TS 版 `packages/core/src/workspace/paths.ts` の
  `isInsideWorkspaceRoot` にも同じ defect があった（git 履歴
  `38df919~1:packages/core/src/workspace/paths.ts:57` で
  `c.startsWith(r + path.sep)` を確認。issue #77 が報告元）。
  Rust port が byte parity を目指した際に持ち越した。predicate の contract
  （root 自身 or `root + SEP` prefix）は変えず、「root が既に SEP で終わる
  場合は二重化しない」に修正するのは semantics の補正であり parity 破壊
  ではない。
- **assumption**: guard は `ends_with(nodepath::SEP)`（host SEP）のみで
  win32 の `/` separator は見ない。`canonicalize_workspace_root` 経由の
  root は dunce canonicalize 済みで `\` 正規形のため到達不能だが、
  predicate の入力 contract は「canonicalize 済み root」を前提とする
  （`resource_contains` も同じ前提）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. predicate 側で `root.ends_with(SEP) ? root : root + SEP` | `resource_contains` と同一パターンで一貫、diff 最小、contract 不変 | 修正箇所が predicate に閉じる（他の trailing-sep 入力は呼び出し側依存） | yes |
| B. `canonicalize_workspace_root` で trailing SEP を除去 | root が正規形になるため下流全部に効く | `C:\` → `C:` は drive-relative path に意味が変わるため危険。root の display 値も変わる | no |
| C. component 分割比較（`relative` / components） | 構造的に正しい | `nodepath` の byte-parity 層を迂回する過剰な変更。`is_inside_workspace_root` は文字列 predicate のままでよい | no |
| D. `real`/`display` 側だけでなく predicate 呼び出し元すべてを列挙して個別修正 | 局所最適 | 同一 defect の複写を増やす。責務は predicate 側 | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: 修正箇所は `is_inside_workspace_root` 1 箇所。
  `resource_contains`（predicates.rs:36-40）と同じ guard を適用する。
- **confirmed**: test は `is_inside_workspace_root_cases` に volume-root
  case を追加する形で足りる（win32 shape `C:\` + `C:\x`、posix shape `/`
  + `/x` は `nodepath::SEP` 依存で `cfg!` 分岐が必要）。
- **hypothesis**: `resolve_workspace_path` 側に追加の volume-root 固有の
  分岐は不要。step 3 の predicate 修正で step 4 の component walk（
  `nodepath::relative` → `join`）も volume root で自然に動くはずだが、
  test で検証する。
- 影響面は boundary check のみ。`deny_paths`（display-relative 判定）や
  walker の canonical cycle key（`realpath` ベース）は別経路で影響しない。
  ただし `walker.rs:432` の symlink-target boundary 判定は同 predicate を
  共有するため修正の効果を受ける。

## Open Questions

- 現行の consumer で volume-root workspace を実際に使う経路があるか
  （`--cwd` で drive root を指定する運用）。影響範囲の実測は不要と
  思われるが、spec の受け入れ条件には edge case として明記する。
- posix `/` の workspace root も同じ修正で通るかは test で確認する
  （`is_inside_workspace_root` は `cfg!(windows)` なしの pure string
  predicate なので platform 差は `nodepath::SEP` のみ）。

## Promotion Candidates

- [x] spec-doc needed: volume-root workspace root の boundary predicate
  契約と受け入れ条件（誤拒否しない、`C:\`/`/` 両 shape の regression
  test、既存 case の非 regress）
- [x] adr-doc needed: 小さいが、predicate 修正 vs root 正規化の選択と
  「`resource_contains` と同一 guard pattern に揃える」一貫性判断を記録

## Sources and Relations

- issue #77: <https://github.com/scarecrow173/cheaper/issues/77>
- defect 箇所: `crates/cheaper-core/src/workspace/paths.rs`
  `is_inside_workspace_root`（TS `packages/core/src/workspace/paths.ts`
  の port）
- 正規形の参照実装: `crates/cheaper-core/src/resources/predicates.rs`
  `resource_contains`（#65-A）
- 関連 chain: `docs/tasks/phase-1-port-cheaper-core.md`（Rust port 元の
  work chain）
