---
id: "ADR-034r05iHrZUE6WgAk8Cpqq"
type: "adr"
status: "accepted"
title: "packages/cli/package.json を canonical version source とし check script で一致を強制する"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/unified-version-source-of-truth.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./separate-dev-apm-project-via-nested-distributable-package.md"
    - "./canonical-setup-check-entrypoints-as-mise-toml-tasks.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/cli-bundle-layout.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/unified-version-source-of-truth.md"
  derived-by:
    - "../designs/unified-version-source-of-truth.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 22. packages/cli/package.json を canonical version source とし check script で一致を強制する

## Context and Problem Statement

SPEC-034r05iHK8bA7grnYLdwgl は製品 version の canonical source を 1 箇所に定め、全宣言
（workspace package.json、配布 manifest、bundle `version.json`、
`--version`）の一致を要求する。現状は `packages/{cli,core,
runtime-pi}/package.json` が `0.0.0`、`packages/apm/apm.yml` が `1.0.0`
で、`cheaper --version` は `0.0.0` を返すが配布 manifest は `1.0.0` を
名乗る。

決めるべきこと: (a) canonical source をどこに置くか、(b) 一致をどう
保つか（生成で派生させるか、検証で発散を拒否するか）、(c) 初期値を
何に揃えるか。

## Decision Drivers

- `--version` は `packages/cli/package.json`（または sibling
  `version.json` fallback）を読む。CLI が「自分の version」を返す以上、
  cli の package.json は事実上の source である。
- `packages/apm/apm.yml` は APM が読む手書き manifest であり、bundle
  script からの自動生成対象にすると manifest 全体の生成管理が必要に
  なる（name・description・targets・primitives 宣言も含むため過剰）。
- `version.json` は既に `scripts/bundle-cli.mjs` が cli package.json
  から生成しており、派生経路は確立済み。
- 発散を防ぐだけなら、全 file を生成するより「宣言 + CI で一致検証」の
  方が単純で、人が編集する manifest の柔軟性を保てる。
- 初期値: 配布 manifest は既に `1.0.0` を名乗っている。`0.0.0` 側を
  `1.0.0` へ揃えるのが consumer から見た不変量を保つ最小変更。

## Considered Options

- **A: cli package.json canonical + check 検証**（採用）。全 version
  宣言を canonical と一致させるよう `scripts/check-version.mjs` で
  検証し、`mise run check` に組み込む。bump は `scripts/set-version.mjs
  <X.Y.Z>` が全宣言を更新する。
- **B: apm.yml canonical + package.json 派生**。配布 manifest を真に
  する案。`--version` の source と分離してしまい、bundle 時に
  package.json を書き換える必要が出る（build artifact の手編集に
  近く、pnpm workspace の package 管理と相性が悪い）。
- **C: 専用 VERSION file canonical + 全宣言を生成**。repo root の
  `VERSION` file を真にして全宣言を script 生成する案。宣言 file の
  手編集を禁止する必要があり、npm 系 tool（`pnpm version` 等）との
  整合が取りにくい。

## Decision Outcome

Chosen option: **A — `packages/cli/package.json` を canonical version
source とする**。理由:

- `--version` の既存の読み取り経路（package.json → version.json
  fallback）と source が一致し、表現ドリフトが起きない。
- `version.json` の生成経路は既に確立している（`bundle-cli.mjs`）。
- 手書き manifest（apm.yml）は宣言のまま残し、check で一致を強制する
  ため manifest の構造を生成管理に組み込む必要がない。

補足の決定:

- **一致対象**: `packages/cli/package.json`（canonical）、
  `packages/core/package.json`、`packages/runtime-pi/package.json`、
  `packages/apm/apm.yml`。生成物 `version.json` は bundle 由来のため
  check でも照合する（freshness check と二重だが安価）。
- **対象外**: root `apm.yml`（`cheaper-dev` dev project）は製品
  version ではなく project 識別子であり独立。
- **初期値**: `1.0.0` に統一（配布 manifest が既に名乗る値へ
  package.json 群を揃える）。
- **check の組み込み**: `mise.toml` の `check` task（SPEC-034r05iHK8bA7gpcCJQya9 の
  canonical entrypoint）に `node scripts/check-version.mjs` を追加。
  CI は同一 entrypoint を通るため CI check 追加は不要（既存構造で
  自動的に検出される）。
- **bump 手順**: `node scripts/set-version.mjs <X.Y.Z>` が対象 file を
  一括更新し、bundle 再生成を促す。手順は `docs/guides/release.md` に
  記す。

## Consequences

- `packages/{cli,core,runtime-pi}/package.json` は `1.0.0` になる。
  `cheaper --version` は `1.0.0` を返す。bundled CLI の `version.json`
  も `1.0.0` になる（bundle 再生成が必要）。
- version 宣言がどれか一つでも発散すると `mise run check` が fail し、
  CI が赤くなる。
- release 時の更新箇所は `set-version.mjs` 1 コマンドに集約される。
- root `apm.yml` の version は製品と無関係であることが文書で明記される。

## Confirmation

- `node scripts/check-version.mjs` が現状で pass し、いずれかの宣言を
  ずらすと fail する。
- `mise run check` が version check を含めて pass する。
- `cheaper --version` と bundled CLI が `1.0.0` を返す。
