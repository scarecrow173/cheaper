---
id: "TASK-034DzYfbw07gFHdsAguwHp"
type: "task"
status: "done"
title: "T4: poc-rig 削除 (F5)"
created: "2026-10-10"
updated: "2026-10-10"
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
    - "docs/plans/execute-post-migration-structural-cleanup.md"
  implemented-by: []
  depends-on:
    - "docs/plans/execute-post-migration-structural-cleanup.md"
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
---

# T4: poc-rig 削除 (F5)

## Work

plan `docs/plans/execute-post-migration-structural-cleanup.md`
(PLAN-034Dz3LX6Jz18cbOR0KtFd)の T4 stream(F5)。workspace 構成の
残滓である `crates/poc-rig/`(standalone spike crate、既に workspace
`exclude` 済み)をディレクトリごと削除し、manifest・`.gitignore`・
`CONTRIBUTING.md` の poc-rig 専用記述を除去する。削除前に poc-rig の
知見が移行設計書/ADR に取り込み済みであることを確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-migration-structural-cleanup.md` — 本 task が
  implement する plan。File Map「T4: `poc-rig` 削除(F5)」が作業の正
- `docs/specs/post-migration-structural-cleanup-scope.md` — F5 stream の
  acceptance criteria
- `docs/designs/post-migration-structural-cleanup-reorganization.md` —
  target layout

## Dependencies

- task 依存なし(plan の Dependencies table で T4 は他 stream と独立・
  並列化可能)

## Affected Files

- `crates/poc-rig/` — ディレクトリ削除
- `Cargo.toml` — `exclude = ["crates/poc-rig"]`(line 8)と poc-rig 説明
  コメント block(line 2-4)を除去。line 1 の manifest header
  (`# Repo-root virtual manifest ...`)は残す
- `.gitignore` — `crates/*/target/`(line ~185)を除去
  (直上 comment の "standalone spikes" 用で poc-rig 専用)
- `CONTRIBUTING.md` — workspace 構成説明の `crates/poc-rig` 項
  (:89-90)を除去
- 削除前に `poc-rig` の知見が移行設計書/ADR に取り込み済みか確認
- 参照調査: `poc` を名前に含む doc/script 参照の確認。adapter の
  doc comment 内 `crates/poc-rig/*` 参照は歴史的記述として残存許容
  (Verification の residual-grep allowlist 参照)

## Done When

- [ ] `crates/poc-rig/` が削除され、root `Cargo.toml`・`.gitignore`・
      `CONTRIBUTING.md` から poc-rig 専用記述が除去されている
- [ ] `poc-rig` の知見が移行設計書/ADR に取り込み済みであることを確認
      した
- [ ] `cargo build --workspace` と `cargo test --workspace` が green

## Out of Scope

- adapter doc comment 内の `crates/poc-rig/*` 歴史的記述の書き換え —
  残存許容(allowlist 対象)として意図的に触らない

## Verification

- [ ] `mise exec -- cargo build --workspace` exits 0
- [ ] `mise exec -- cargo test --workspace` exits 0
- [ ] `test ! -d crates/poc-rig` exits 0(ディレクトリ不在)
- [ ] `git grep -n "poc-rig\|poc_rig" -- Cargo.toml .gitignore
      CONTRIBUTING.md` が 0 件を返す(exit 1)
- [ ] `git grep -n "poc-rig" -- crates/ scripts/ benchmarks/ .github/`
      の hit が intended-residue allowlist(adapter doc comment の
      歴史的記述)のみであることを確認する
