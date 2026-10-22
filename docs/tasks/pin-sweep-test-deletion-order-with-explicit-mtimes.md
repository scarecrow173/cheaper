---
id: "TASK-0340C4KaPSmy4CrDagevUC"
type: "task"
status: "done"
title: "Pin sweep test deletion order with explicit set_mtime timestamps"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/157"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/route/log_store.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
  implemented-by: []
  depends-on:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
  blocks:
    - "docs/tasks/verify-route-test-determinism-and-mise-check.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/deterministic-route-test-assertions.md"
  references:
    - "docs/designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
  defers: []
  deferred-by: []
---

# Pin sweep test deletion order with explicit set_mtime timestamps

## Work

`crates/cheaper-cli/src/route/log_store.rs` の
`sweep_enforces_ttl_count_size_and_skips_tmp`(:559-606)で、(b) count
sweep・(c) size sweep の対象 file に `File::set_modified` の一意な mtime
順序を与え、削除・残存 assertion を厳密集合へ絞る(design D2-D4)。

手順:

1. `#[cfg(test)] mod tests`(:384)内に mtime 付替え helper を追加する
   (先例: `tests/integration/hook_eval_dedup.rs:127-132`・
   `src/hook/eval_cache.rs:982` — doc comment も同系に揃える):

   ```rust
   /// log file の mtime を付け替える(`File::set_modified` — 新規 dep
   /// 不要)。
   fn set_mtime(path: &Path, time: SystemTime) -> io::Result<()> {
       fs::File::options()
           .write(true)
           .open(path)?
           .set_modified(time)
   }
   ```

   `use super::*` で `fs`/`Path`/`SystemTime`/`io` は導入済みのため追加
   import は不要。
2. test 先頭で `let now = SystemTime::now();` を 1 回だけ取得し、(a) TTL
   block の inline `fs::File::options()...set_modified(past)`(:575-581)
   を `set_mtime(&old, now - Duration::from_secs(3600)).unwrap()` の
   helper 呼出しへ揃える(同一 test 内の同一操作を 1 表現に集約 —
   表現ドリフト防止、design Components and Boundaries)。TTL の
   `removed == 1`・fresh 残存・old 削除の assert と in-flight `.tmp`
   writer は不変。
3. (b) count sweep: `a.log`/`b.log`/`c.log`/`fresh.log` へ
   `a = now-40s`、`fresh = now-30s`、`b = now-20s`、`c = now-10s` の
   `set_mtime` を sweep 前に適用し(`as_secs` で 1 秒超離れた一意 key)、
   `sweep_with(Duration::MAX, 2, u64::MAX)` の assertion を厳密集合へ:
   `assert_eq!(removed, 2)`、`!a.exists()`、`!fresh.exists()`、
   `b.exists()`、`c.exists()`。現行の `b.exists() || c.exists()`
   (:594)は廃止(design D3 — 削除集合 {a, fresh} は現行コメントの
   「fresh + a が消える」意図を維持)。
4. (c) size sweep: 直前の cleanup loop
   `for path in [&a, &b, &c, &fresh] { let _ = fs::remove_file(path); }`
   (:596-598)を**必ず維持**する(落とすと (b) の残留 file が (c) の
   mtime 順へ混ざり tie/列挙順依存が再導入される — design D4)。
   `big.log`(128B)に `now-20s`、`small.log`(8B)に `now-10s` の
   `set_mtime` を与え(big を古く)、`sweep_with(Duration::MAX,
   usize::MAX, 100)` の assertion を `assert_eq!(removed, 1)`・
   `!big.exists()`・`small.exists()` へ厳密化(`big.exists() !=
   small.exists()` :603 廃止)。順序を逆にすると small 先行削除でも残量
   128B > 100 で `removed == 2` となり意図が変わるため big-older は必須。

維持するもの(変更禁止):

- `.tmp` 除外 assert(:605)・TTL sweep の検証・`store()`/`write_log`
  helper・sweep 引数(TTL `Duration::from_secs(60)`、count `2`、size
  `100`)。
- 本番実装 `sweep_with`(:115-183) — test 外の code は一切変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/fix-route-test-flake-via-test-side-determinism.md (T2 slice)
- docs/designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md (D2-D5)

## Dependencies

- docs/plans/fix-route-test-flake-via-test-side-determinism.md

## Affected Files

- crates/cheaper-cli/src/route/log_store.rs (`#[cfg(test)] mod tests` のみ)

## Done When

- [x] `set_mtime` helper が `mod tests` 内にあり (a)(b)(c) 全 block で
      mtime 付替えが helper 経由に統一されている
- [x] (b) で a<fresh<b<c の明示 mtime と厳密集合 assert(removed==2・
      {a,fresh} 削除・{b,c} 残存)がある
- [x] (c) で big<small の明示 mtime と厳密集合 assert(removed==1・big
      削除・small 残存)があり、pre-(c) cleanup loop が維持されている
- [x] `.tmp` 除外・TTL assert・sweep 引数・本番 code が不変

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::log_store` exits 0
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib` exits 0(他 unit test の非 regress)
- [x] `mise exec -- cargo fmt --check` exits 0

## Done

`log_store.rs` の `mod tests` に `set_mtime` helper(`File::set_modified`)
を追加し、単一の `now` 起点で (a) `now-3600s`・(b) `a=now-40s /
fresh=now-30s / b=now-20s / c=now-10s`・(c) `big=now-20s / small=now-10s`
の一意な mtime 順序を付与した。assertion は厳密集合へ絞り、(b) は
`removed==2` + {a,fresh} 削除 + {b,c} 残存、(c) は `removed==1` +
big 削除 + small 残存を検証する。pre-(c) cleanup loop・`.tmp` 除外・
TTL assert・sweep 引数・本番 code は不変。
