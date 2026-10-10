# Version と release 手順

製品 version の **canonical source of truth は root `Cargo.toml` の
`[workspace.package].version`**（ADR-034CpwNUNIDqrR92x7VaUR。旧
`packages/cli/package.json` canonical の ADR-034r05iHrZUE6WgAk8Cpqq は
superseded）。`cheaper --version`（`cheaper-cli` crate が
`version.workspace = true` で継承し、build.rs が binary へ埋め込む）
と配布 manifest `packages/apm/apm.yml` はこの値と一致しなければ
ならない。

配布は **Cargo source install**（ADR-034Dmb3kNAO4Ub6FxuQCtC）—
GitHub Release・asset・checksum manifest・APM archive の公開手順は
issue #85 で廃止された。release に必要なのは version bump + tag のみ。

## Version 宣言の一覧

| ファイル | 役割 | 扱い |
| --- | --- | --- |
| `Cargo.toml`（`[workspace.package].version`） | canonical source | **ここだけが真の宣言** |
| `packages/apm/apm.yml` | 配布 APM manifest | canonical と一致必須 |
| `apm.yml`（root） | `cheaper-dev` dev project | **対象外** — 製品 version ではなく dev project 識別子。独立してよい |

library crate（`crates/*/Cargo.toml` の各 `version`）は内部の path 依存で
あり publish・release されないため、製品 version の宣言箇所ではない。
`cli/version.json` の bundled 宣言は issue #85 で撤去済み — binary の
version は build.rs が `CARGO_PKG_VERSION`（= workspace version）から
compile 時に埋め込むため、canonical 宣言と常に一致する。

## 不一致の検出

`node scripts/check-version.mjs` が全宣言を canonical と照合する。
`mise run check`（SPEC-034r05iHK8bA7gpcCJQya9 の canonical verification
entrypoint）に組み込まれているため、CI を含む通常の検証経路で自動的に
検出される。不一致があれば file 名と値を列挙して exit 1。

## Version bump と公開

```bash
node scripts/set-version.mjs <X.Y.Z>   # Cargo.toml + packages/apm/apm.yml を一括更新
mise run check                          # 全検証（version / lockfile / payload check 含む）
```

1. `set-version.mjs` で bump（続けて `check-version.mjs` が自動実行される）。
2. `mise run check` が全 pass することを確認する。
3. 宣言変更を commit して branch を push し、merge する。
4. `git tag v<X.Y.Z>` で immutable tag を打ち、push する。

tag 公開後、consumer は下記で新 version を install できる:

```bash
cargo install --git https://github.com/scarecrow173/cheaper.git --locked --force cheaper-cli
# 特定 tag を pin する場合:
# cargo install --git https://github.com/scarecrow173/cheaper.git --tag v<X.Y.Z> --locked cheaper-cli
```

公開後の既存 tag を書き換えて再利用しない。失敗は新しい immutable
version/tag で fix-forward する。

## 注意

- 宣言を手で直接編集するのは避け、`set-version.mjs` を使う。ずれた
  状態で commit すると `mise run check`（CI）が fail する。
- `Cargo.lock` も consumer の `--locked` install 契約の一部。
  lockfile を再生成する変更は commit 済み `Cargo.lock` に反映してから
  tag を打つ（`scripts/check-lockfile.mjs` が drift を検出する）。
