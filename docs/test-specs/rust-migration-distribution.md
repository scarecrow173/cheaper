---
id: "TSPEC-034BgunW9gS0w1MEph4zCE"
type: "test-spec"
status: "proposed"
title: "rust-migration-distribution"
created: "2026-10-06"
updated: "2026-10-06"
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
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
  derived-by: []
  verifies:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# rust-migration-distribution

## Purpose

配布物の中身が Node SEA executable から Rust ネイティブ binary に換わって
も、consumer 側の取得・検証・実行経路(Release asset 名・SHA256SUMS・
launcher sh/ps1・apm bundle 配置)が無改修で成立することを保証する意図を
記録する。consumer が移行を意識しないことが contract であり、launcher
改修が許されないため、「変わるのは CI の build 手順と binary の中身
だけ」という境界を検証側から固定する。配布 contract 自体が再設計された
場合にのみ見直す。

## Feature

同一配布 contract の下での Rust release binary 切替

## Rules

### asset・検証・サイズ

- Release asset 名は `cheaper-linux-x64`・`cheaper-linux-arm64`・
  `cheaper-windows-x64.exe` を維持し、`releaseTargets`・manifest 規則・
  snapshot 構造を変更しない
- `SHA256SUMS` の生成形式(64hex + 空白 2 個 + basename)と検証フローは
  不変で、Rust binary が同一手順で hash 照合される
- `--version` probe は bare semver 1 行を stdout に返し、canonical
  version(`packages/cli/package.json`。宣言 drift は Cargo.toml
  workspace version を含めた照合で機械検出)と完全一致する。`models` は
  credential 非依存で exit 0 を返す
- strip 後の release binary は ≤20MB(目標 5-15MB)を CI の size assert
  gate で保証する。超過は緩和・閾値引上げではなく escalate 対象
- release profile は `lto`・`opt-level="s"`・`strip`・
  `codegen-units=1` とし、`panic="abort"` を設定しない(unwind 必須、
  `catch_unwind` → `INTERNAL_ERROR` 境界の前提)

### launcher・bundle・実行経路

- launcher(`cheaper` shell script / `cheaper.ps1`)は無改修で、stub
  Release からの download → SHA256SUMS 照合 → `--version` probe →
  exec が Rust binary で成立する
- `packages/apm/.apm/hooks/`(hooks.json・launcher)は無改変。hooks.json
  経由の `cheaper hook <event> --target <agent>` invocation が Rust
  binary で動作し fail-open を維持する
- `pnpm bundle` は従来通り `cli/bin/cheaper.mjs`・`version.json`・
  `workers/`・`policy.yaml` を生成し `check-apm-payload.mjs` が pass
  する。`build-apm-bundle.mjs --exe-dir` に rename 済み Rust asset を
  渡したとき同名 sibling 配置になる
- darwin binary は Release asset・manifest・bundle の `releaseTargets`
  に混入させず、build 検証 + sibling smoke のみとする(acquisition
  非対象)
- Windows の path canonicalize は verbatim prefix(`\\?\`)を除去し、
  `..` を component 単位で lexical resolve し、不存在 tail は「最深の
  存在する祖先を canonicalize + tail 連結」に従う。deny/writeScope/
  display/`root` field の文字列表現は TS 実装の byte 表現と一致させる
  (Windows golden fixture で固定)

## Examples

- Given: cargo release matrix の各 target(win-msvc・linux-musl x64・
  linux-musl arm64)。Then: 出力 binary が規定 asset 名に rename され、
  size assert で ≤20MiB と判定される。超過時は CI が fail する
- Given: 無改修 launcher に stub Release(sha256 付き Rust binary)を
  指定。Then: download → SHA256SUMS 照合 → `--version` probe → exec
  が通り、`--version` 出力が canonical version と一致する
- Given: SHA256 mismatch の asset。Then: 検証が取得を拒否し、破損
  binary が exec されない
- Given: `build-apm-bundle.mjs --exe-dir` に rename 済み Rust binary を
  渡す。Then: `.apm/hooks/cli/bin/<platform>/` に同名 sibling 配置され、
  hook 経路が sibling 優先でその binary を実行する
- Given: Windows で `..` 混在・不存在 tail・verbatim prefix を含む入力
  path。Then: canonicalize 結果の `root`・display 表現が TS 版 golden
  fixture と一致する
- Given: darwin artifact。Then: smoke は sibling 配置経路でのみ行われ、
  Release asset 列挙(3 binary + SHA256SUMS + tar.gz の 5 asset)に混入
  しない

## Guarantees

- consumer の hooks.json・launcher・version pin・SHA256SUMS 検証を触らず
  に Rust binary へ切替・rollback(旧 tag の SEA asset 再取得)ができる
- binary size が CI gate として fail 伝播し、20MB 超過が Release へ
  出ない
- `--version`/`models` の build smoke と version 宣言照合で binary 埋込
  version と canonical version の drift を二重に防ぐ
- Windows path 規則の byte 互換が golden fixture で固定され、sandbox
  境界(deny/writeScope)の見かけ差異で誤判定しない

## Non-goals

- darwin-* の Release asset 追加(Deferred、launcher の acquisition 分岐
  解放とセットで別判断)
- `cheaper.mjs` JS fallback の廃止判断(Phase 6)
- version canonical source の Cargo.toml 移行(Phase 5 では
  `packages/cli/package.json` 維持 + 照合追加)
- runtime/CLI 意味論の parity(differential golden・envelope・run
  semantics の各 test-spec が担う。本 spec は配布経路のみ)
- Linux gnu fallback の可否判断(musl 不達時は理由付き記録で許容、
  asset 名・contract は不変)

## Risk

- この保証が落ちると、asset 名・SHA256SUMS・launcher のどれかがずれた
  時点で consumer の取得経路が壊れ、version pin で pin された旧
  Release への rollback 経路まで同時に失う。`panic="abort"` 混入や
  size 超過の未検出は「envelope 無し終了」「配布不能」を Release 後に
  露呈させ、fix-forward(新 tag)しか残らない

