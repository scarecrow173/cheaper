# Version と release 手順

製品 version の **canonical source of truth は `packages/cli/package.json`**
（ADR-034r05iHrZUE6WgAk8Cpqq）。`cheaper --version`、配布 manifest `packages/apm/apm.yml`、
bundle 内 `version.json`、各 workspace package の `version` field はすべて
この値と一致しなければならない。

## Version 宣言の一覧

| ファイル | 役割 | 扱い |
| --- | --- | --- |
| `packages/cli/package.json` | canonical source | **ここだけが真の宣言** |
| `packages/core/package.json` | workspace package | canonical と一致必須 |
| `packages/runtime-pi/package.json` | workspace package | canonical と一致必須 |
| `packages/apm/apm.yml` | 配布 APM manifest | canonical と一致必須 |
| `packages/apm/.apm/hooks/cli/version.json` | bundled CLI の version | 生成物（`bundle-cli.mjs` が cli から派生） |
| `apm.yml`（root） | `cheaper-dev` dev project | **対象外** — 製品 version ではなく dev project 識別子。独立してよい |

## 不一致の検出

`node scripts/check-version.mjs` が全宣言を canonical と照合する。
`mise run check`（SPEC-034r05iHK8bA7gpcCJQya9 の canonical verification entrypoint）に
組み込まれているため、CI を含む通常の検証経路で自動的に検出される。
不一致があれば file 名と値を列挙して exit 1。

## Version bump

```bash
node scripts/set-version.mjs <X.Y.Z>     # 全宣言を一括更新
node scripts/bundle-cli.mjs              # version.json を再生成
mise run check                           # 全検証（version check 含む）
```

`set-version.mjs` は上表の宣言 file をすべて書き換え、続けて
`check-version.mjs` を実行する。`version.json` は生成物のため bump
直後は不一致として検出され、bundle 再生成を促される（正常な流れ）。

## Release 手順

1. `node scripts/set-version.mjs <X.Y.Z>` で bump する。
2. `node scripts/bundle-cli.mjs` で `version.json` と bundled CLI を
   再生成する。
3. `mise run check` が全 pass することを確認する（lint → version →
   typecheck → test → bundle freshness）。
4. 宣言変更と生成物（`packages/apm/.apm/hooks/cli/`）を同一 commit に
   まとめる（`release:` または `chore:` prefix）。
5. `node packages/cli/bin/cheaper.js --version` と、bundle 内
   `version.json` が新しい値を返すことを目視確認する。
6. `git tag v<X.Y.Z>` で tag を打ち、push する。
7. GitHub Actions の `release` workflow を**手動 dispatch** し、
   `tag` input に `v<X.Y.Z>` を指定する。この workflow は
   `workflow_dispatch` のみを trigger に持ち、自動では実行されない。
   各 platform（Windows x64・Linux x64・Linux arm64）で SEA
   executable を build・smoke し、Release `v<X.Y.Z>` に添付する。
   macOS artifact は現時点では対象外。
8. tag の `packages/cli/package.json` version が `v<X.Y.Z>` と一致
   しない場合、workflow は version 検証で fail し artifact は
   upload されない。その場合は宣言を修正してから再度 dispatch する。

## 注意

- `version.json`・`bin/cheaper.mjs` は生成物。手編集しない。bundle
  本体に version 定数は埋め込まれない（`version.ts` が実行時に
  package.json → version.json fallback で読む）。
- 宣言を手で直接編集するのは避け、`set-version.mjs` を使う。ずれた
  状態で commit すると `mise run check`（CI）が fail する。
