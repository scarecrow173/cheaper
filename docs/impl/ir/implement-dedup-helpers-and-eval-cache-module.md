---
id: "IMPL-034IpXmCLMAEep0qsWBOkZ"
type: "impl"
status: "completed"
title: "Implement dedup helpers and eval_cache module"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/src/hook/eval_cache.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/mod.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/embedded.rs"
      - type: "file"
        path: "docs/tasks/implement-dedup-helpers-and-eval-cache-module.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-dedup-helpers-and-eval-cache-module.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-eval-dedup-verdict-cache-contract.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement dedup helpers and eval_cache module

## Summary

issue #127(重複 hook firing)対応の plan T1+T2 を同一 commit で実装した。
`routing_policy` に additive な fingerprint helper を追加し、新規 module
`hook::eval_cache` に canonical request fingerprint verdict cache +
leader election(lock file + owner token・follower bounded poll・TTL
sweep)を実装した。`targets.rs` への配線(`evaluate` → `evaluate_dedup`
差し替え)は後続 task の範囲であり、本 task では module の追加と compile +
unit test 検証まで。挙動は additive で不変。

## Context

- task: `docs/tasks/implement-dedup-helpers-and-eval-cache-module.md`
  (TASK-034IpPs1L6kjNy6HyCalZs)
- plan: `docs/plans/implement-hook-eval-dedup-verdict-cache.md`
  (T1 routing_policy helpers + T2 eval_cache 本体の先頭 slice)
- design: `docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md`
  (DESIGN-034IefWW6OJ9rvESaNvKUK — fingerprint field 列挙・record
  schema・lock protocol・failure model)

## Implementation

### Changes

- `crates/cheaper-cli/src/hook/routing_policy.rs`(additive)
  - `Verdict` に `#[derive(Debug, PartialEq, serde::Serialize,
    serde::Deserialize)]` + `#[serde(tag = "decision", rename_all =
    "lowercase")]` — `Allow → {"decision":"allow"}` /
    `Deny{reason} → {"decision":"deny","reason":"…"}`。
  - `MEASURE_TIMEOUT`・`MEASURE_ENV_KEYS` を `pub(crate)` 化(follower
    wait 上限の単一 source・eval_cache の env snapshot 共有)。
  - `ResolvedKind`/`ResolvedTarget`/`FingerprintContext`/
    `fingerprint_targets` を追加 — `resolve_target` の containment
    規則を複製せず再利用し、File/Dir は `std::fs::canonicalize` 済み
    絶対 path(canonicalize 失敗は `NonFile` へ畳む)、
    `workspace_root` は `canonicalize_workspace_root(cwd_abs)` の
    結果(失敗時 `None`)。
  - test: `verdict_serde_uses_internally_tagged_decision` 追加。
- `crates/cheaper-cli/src/embedded.rs` — `embedded_fingerprint()` を
  `pub(crate)` 化(builtin `None` policy 層の fingerprint marker)。
- `crates/cheaper-cli/src/hook/mod.rs` — `pub mod eval_cache;`。
- `crates/cheaper-cli/src/hook/eval_cache.rs`(新規)
  - `DedupLimits`(`#[doc(hidden)]`・`Clone + Copy`)+
    `DedupLimits::PRODUCTION`(ttl 60s / lock_stale 120s /
    follower_wait = `MEASURE_TIMEOUT` 30s / poll 50ms)。
  - `evaluate_dedup`(pub wrapper)と `evaluate_dedup_with`
    (`#[doc(hidden)]` test seam、`F: Fn() -> Fut` で HRTB 問題を
    構造的に回避)。
  - fingerprint: `#[derive(Hash)] FingerprintInput`(format tag
    `"hook-eval-v1"` + variant field + stat signature + policy/env
    入力)を `DefaultHasher` → `{:016x}`。raw path/cwd・target・
    source_tool・config-dir 系 env は含めない。
  - record `<fp>.json`: `{schemaVersion:1, createdAt, fingerprint,
    verdict}` を `fs_util::atomic_write`(`windows_remove_first:
    false`・unix `0o600`)で write。read は schemaVersion==1・
    fingerprint 一致・verdict deserialize・file mtime checked-age
    (未来 mtime は stale)を全て満たすもののみ serve。
  - lock `<fp>.lock`: `OpenOptions::create_new` + owner token
    `{pid, createdAt, nonce}`(epoch nanos + `AtomicU64` counter)。
    全削除は delete-if-owner(payload 読み直し + token 照合)。
    unparseable payload は一切削除しない(保守的、sweep で期限切れ)。
  - publish 順序: verdict atomic_write 完了 → lock release。
    record write 失敗時も eval_fn は再実行せず verdict を返す。
  - follower bounded poll: 1 tick 分の fs 操作を `spawn_blocking` へ
    逃がし `tokio::time::sleep(poll_interval)` で次 tick。
    `follower_wait` deadline で自前評価(cache write しない)。
  - sweep: `mtime > ttl` の `*.json`・`mtime > lock_stale` の
    `*.lock`・orphan `.<fp>.json.tmp-*` を best-effort remove
    (checked-age None の未来 mtime も含む)。
  - fast-path gate(任意項目 — 採用): 全 target が
    `len <= threshold` の File、または全て NonFile の場合は協調を
    skip して直接評価。
  - 分類 hit/leader/follower/fallback/bypass を `tracing::debug!`
    (`dedup` field)で emit。
  - unit test 6 件(delete-if-owner 2・checked-age record/lock 2・
    sweep・fingerprint)。
- `docs/tasks/implement-dedup-helpers-and-eval-cache-module.md` —
  status `done`、checklist 全項目 checked。

### Decisions During Implementation

- `SystemTime::checked_duration_since` は存在しないため、checked-age
  規則は `SystemTime::duration_since(mtime).ok()`(`Err` = 未来
  mtime → `None` 相当)で実装した — design の「checked-age」規約と
  同じ semantics(future mtime = stale)。
- phase B(record read → lock `create_new`)と follower poll tick を
  単一の `poll_tick` blocking fn に統一 — 初回 tick が phase B、
  Wait 以降の tick が bounded poll に相当し、協調規則を 1 箇所で保持。
- acquire 時の owner token 書込み失敗は lock を残したまま取得失敗へ
  畳む(unparseable payload を自前 remove すると「unparseable は
  削除しない」不変条件を自ら破るため。sweep が `lock_stale` で回収)。
- env snapshot・store root 解決(`resolve_user_config_dir` が
  `EnvLookup` = 非 Send を要する)は async 側で行い、blocking phase
  には owned map・PathBuf のみ渡す(`measure_tokens` と同じ pattern)。
- eval_cache.rs 内 unit test は `unwrap`/`expect`/`panic!` を使わない
  形(`Result<(), Box<dyn Error>>` + `?` + `assert!`)で記述し、
  fail-open 確認 grep を file 全体で成立させた。

## Related Experiments

### Adopted

なし(既知の実装 task のため Experiment Log は作成していない)。

### Rejected

なし。

## Validation

- `mise exec -- cargo check -p cheaper-cli --features
  cheaper-cli/dev-seam` — exit 0。
- `mise exec -- cargo clippy -p cheaper-cli --features
  cheaper-cli/dev-seam --all-targets -- -D warnings` — exit 0
  (警告・dead_code 無し)。
- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
  hook::` — 24 passed / 0 failed(既存 hook 契約の回帰無し)。
- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib hook::` —
  25 passed / 0 failed(eval_cache 6・routing_policy serde 1 を含む)。
- `mise exec -- cargo fmt -p cheaper-cli -- --check` — clean。
- fail-open: `eval_cache.rs` 内に `unwrap()`/`expect(`/`panic!(`
  の呼出し経路無し(grep で残るのは spec 指定の `unwrap_or` 4 件のみ)。

## Risks

- `DefaultHasher` は adversarial 衝突耐性を持たないが、fingerprint は
  user-trusted `<userConfigDir>` 境界内の cache であり許容(design
  Risks と同じ評価)。
- mtime granularity / dir shallow signature による fingerprint 取り
  こぼしは TTL(60s)で staleness を bound する受容済み妥協。
- live leader が `lock_stale` を超える長時間 eval(shell
  multi-candidate)で stale 誤判定され得る — takeover しても害は
  冗長評価のみ(delete-if-owner が後継 lock の誤削除を防ぐ)。

## Follow-ups

- `docs/tasks/wire-evaluate-dedup-into-hook-target-call-sites.md`
  (T3)— `handle_hook` 5 arm の `evaluate` を `eval_cache::
  evaluate_dedup` へ差し替え。
- T4: `tests/integration/hook_eval_dedup.rs` + `main.rs` mod 宣言
  (module 層 seam は `evaluate_dedup_with`/`DedupLimits` として
  既に `#[doc(hidden)] pub` で到達可能)。
