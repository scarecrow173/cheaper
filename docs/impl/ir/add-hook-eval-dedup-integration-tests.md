---
id: "IMPL-034IqeWzYxJ36cp819qQN4"
type: "impl"
status: "completed"
title: "Add hook_eval_dedup integration tests"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_eval_dedup.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/main.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-hook-eval-dedup-integration-tests.md"
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
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Add hook_eval_dedup integration tests

## Summary

plan `implement-hook-eval-dedup-verdict-cache` の T4 — issue #127(同一
tool call への duplicate hook firing を 1 回の高価な評価へ収束させる
dedup verdict cache)の contract を、
`docs/test-specs/hook-eval-dedup-verdict-cache-contract.md` の
Examples に 1:1 対応する 16 件の integration test で固定した。

module 層 seam `evaluate_dedup_with`(`#[doc(hidden)] pub`)へ
`Arc<AtomicUsize>` で計数する `eval_fn`(200–500ms sleep で重なり
確定)と ms スケールの `DedupLimits` を注入し、協調・fingerprint
invalidation・fail-open fallback を timing race 非依存で決定的に
検証する。binary 層は `env!("CARGO_BIN_EXE_cheaper")` の dual spawn
で per-target render・exit code・store file 残存を検証する。

## Context

- task: `docs/tasks/add-hook-eval-dedup-integration-tests.md`
- plan: `docs/plans/implement-hook-eval-dedup-verdict-cache.md` (T4)
- test-spec: `docs/test-specs/hook-eval-dedup-verdict-cache-contract.md`
- design: `docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md`
- issue: <https://github.com/scarecrow173/cheaper/issues/127>
- 前提: T1/T2(`eval_cache.rs` + `evaluate_dedup_with`/`DedupLimits`
  seam、commit 856eec1)と T3(5 配線、commit 5511f3a)は済。

## Implementation

### Changes

- add `crates/cheaper-cli/tests/integration/hook_eval_dedup.rs`
  (~1100 行)。16 test は task の一覧と同名で、spec の AC 区分(AC1–AC6)
  へ comment section を対応付けた:
  - AC1/AC2: `concurrent_firings_single_evaluation`(計数 = 1・両
    verdict 同一・3 発目 cache hit)・`binary_dual_spawn_same_verdict`
    (devin/pretooluse dual spawn → 同一 devin payload → 各 target
    contract で同一 deny・record 1 件・lock 非残存)・
    `per_target_render_identity`(5 target × 2 連続実行の
    stdout/stderr/exit byte 同一 — deny が render されていることを
    先に assert し accidental-allow を排除)
  - AC3: `stat_change_new_fingerprint`・
    `threshold_change_new_fingerprint`・
    `policy_and_count_cap_new_fingerprint`・
    `cwd_spelling_same_fingerprint`(`<tmp>/./sub`・`<tmp>/sub/`・
    junction `<tmp>/link` → `<tmp>/sub` の絶対 spelling 差が同一
    fingerprint へ収束 — `set_current_dir` 不使用、相対 spelling は
    process cwd anchor のため除外)
  - AC4: `stale_lock_takeover`・`fresh_lock_no_verdict_self_eval`・
    `live_leader_delete_if_owner`(leader release・後続 lock 交替への
    token-mismatch 保護・unparseable fresh lock の非削除を経路観測)
  - AC5: `corrupt_record_fallback`(非 JSON・未知 schemaVersion・
    fingerprint 不一致・TTL 超過 backdated の 4 変種)・
    `copilot_unwritable_cache_exit_zero`(`CHEAPER_CONFIG_DIR` を
    regular file path へ向ける portable な失敗注入 — `observability.rs`
    の `unwritable_log_root` と同一手法)
  - AC6: `future_mtime_not_served`・`sweep_removes_stale`・
    `dir_nested_edit_ttl_bound`・`no_workspace_writes`(`<ws>` 配下
    snapshot 比較)
- modify `crates/cheaper-cli/tests/integration/hook.rs`: File Map
  どおり `Fixture`/`run_hook`/`windsurf_read`/`tool_use_payload`/
  `devin_payload`/`big_text`/`write_big`/`HookResult` と
  `Fixture::{new,write,write_big}`・アクセス対象 field(`ws`/`env`/
  `builtin_dir`)を `pub(crate)` 化 — `CHEAPER_CONFIG_DIR`・`HOME`・
  `USERPROFILE` の tempdir 隔離(env map)をそのまま再利用。
- modify `crates/cheaper-cli/tests/integration/main.rs`: `mod
  hook_eval_dedup;` を `mod hook_config;` の後へ alphabetical 順で宣言。
- modify `crates/cheaper-cli/src/hook/routing_policy.rs`
  `resolve_request_cwd`(`:250`): `cwd_abs` を `nodepath::resolve`
  で常に normalize(後述)。

### Decisions During Implementation

- **File Map 外の production 変更(`resolve_request_cwd`)**:
  `cwd_spelling_same_fingerprint` が `<tmp>/./sub` で record 非生成
  (計数 1 だが `hook-eval/` に `<fp>.json` が無い)を検出した。原因は
  `resolve_request_cwd` が absolute `cwd` を normalize せず
  `cwd_abs` に `/.` 残り、`nodepath::relative`(pre-normalized 前提)
  が `..\..\sub\big.txt` を返して `OutOfScope` → dedup 対象外の
  定数 allow 経路へ畳まれる点。これは dedup 以前の routing 契約で
  「`/.` spelling の cwd だけ verdict が allow に変わる」実 defect で
  あり、spec の「canonical workspace と resolved path が同一
  fingerprint に収束」を成立させるため所有層で修正した
  (`nodepath::resolve(process.cwd(), cwd)` = Node `path.resolve`
  semantics — absolute 入力も `.`/`..`/語尾 separator を normalize)。
  既存 `hook::` 24 件 + lib `hook::` unit test 25 件に回帰無しを
  確認済み。
- **unparseable lock の経路観測は fresh lock で行う**: `sweep_store`
  は `*.lock` を mtime stale 判定だけで回収するため、backdated stale
  lock は `poll_tick` の delete-if-owner へ到達する前に sweep が消す
  (初回実装はここで `NotFound` 失敗)。fresh unparseable lock は
  sweep 対象外で follower が削除せず `follower_wait` deadline 後
  self-eval — 経路観測できる。`remove_if_owner` primitive 自体の
  unparseable/token-mismatch 拒否は既存 unit test
  (`eval_cache::tests::remove_if_owner_*`、T2 追加)が直接固定 —
  spec の「primitive を直接 unit test」要件と 1:1 対応済みのため
  本 test は integration 観測側を担う。
- **PATH shim パターンを partial 再利用**: binary dual spawn は
  `hook_config.rs` 方式の PATH shim ではなく、`env!("CARGO_BIN_EXE_cheaper")`
  直接 spawn + `env_clear` + `Fixture.env` + OS 必須 key(PATH 等)の
  手動 env 構成 — hook は PATH shim を必要としない(親 agent 経由では
  なく payload で検証するため)。
- **`tests/common/mod.rs` の `Fixture` は使わない**: その `run` は
  stdout を JSON envelope と仮定して `serde_json::from_str` unwrap
  するため native hook output と不整合 — `hook.rs` 側の helper を
  `pub(crate)` 化して再利用(task 注意書きどおり)。
- **eval_fn の delay は 200–500ms**: `follower_wait`(300ms–10s)より
  十分短く、`tokio::time::sleep` で overlap を決定的にする。
  `live_leader_delete_if_owner` の swapper は lock file を poll して
  leader eval 中に token 交替(500ms eval window vs 2000×5ms poll)。

## Related Experiments

### Adopted

実験的探索は行わなかった(task は test 追加中心の機械的作業で、
採用判断が必要な代替案は発生しなかった — experiment log 無し)。

### Rejected

同上。

## Validation

- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_eval_dedup::`
  → **16/16 pass**(exit 0)
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::`
  → **24/24 pass**(fixture `pub(crate)` 化の回帰無し)
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration`
  → **202/202 pass**(integration target 全体)
- `mise exec -- cargo test -p cheaper-cli --features cheaper-cli/dev-seam --lib hook::`
  → **25/25 pass**(`resolve_request_cwd` 修正の unit test 回帰確認)
- `mise exec -- cargo clippy -p cheaper-cli --features cheaper-cli/dev-seam --all-targets -- -D warnings`
  → **clean**
- `mise exec -- cargo fmt --check` → **clean**(`cargo fmt` 適用後)
- `mise run check` → **環境要因で未完了**(下記 Risks 参照):
  `cargo clippy --workspace` 中に host の commit limit 枯渇
  (os error 1455/1450)で rustc が toolchain rlib を mmap 不能になり
  失敗。build 自体は `CARGO_BUILD_JOBS=2` で全工程通過済みのため
  code 起因ではない。

## Risks

- **検証環境の resource 枯渇**: 本 session 実行 host で親 `devin`
  process(PID 62068)が ~138 GB の private commit を占有し、
  `cargo build`(default parallelism)が `os error 1455`(ページング
  file 不足)で失敗する。`CARGO_BUILD_JOBS=2` で回避したが、
  `mise run check` 全体の通過は確認できなかった。`resolve_request_cwd`
  修正の範囲は `-p cheaper-cli` の clippy・fmt・lib/integration test
  で網羅済み(`cheaper-runtime`/`cheaper-workflow` 等は非依存)。別
  session やリソースに余裕のある環境で `mise run check` の再実行を
  推奨する。
- **`live_leader_delete_if_owner` の swapper は poll ベース**: leader
  の eval window(500ms)に対し 2000×5ms の十分な budget を持つが、
  極端に遅い環境では `TimedOut` で fail し得る(flaky 要因として
  記録。実測では ~10ms 以内に lock を観測)。
- **junction 作成が不能な環境**: `cwd_spelling_same_fingerprint` は
  `dir_link` 失敗時に symlink spelling を skip する(他 2 spelling で
  convergence は依然検証)。

## Follow-ups

- `resolve_request_cwd` の normalize 修正は `/.`・`/..`・trailing
  separator を含む cwd spelling で hook の routing verdict が変わる
  実 defect の fix — 必要なら changelog/issue への記載を検討(現行は
  本 task の commit に含まれる)。
- `mise run check` を resource に余裕のある環境で再実行し、workspace
  全体の回帰無しを確認する。

## Post-review hardening(2026-10-19、commit `fe85d76`)

実装 review で CONFIRMED となった test 側の指摘への修正:

- **`binary_dual_spawn_same_verdict` の真の overlap(C5)**:
  `collect_child` を `feed_stdin` + `wait_child` に分割し、dual
  spawn では両 child の stdin を **wait 前に全て書き切る**。従来の
  write→wait 逐次では overlap が起きず cross-process dedup を実証
  していなかった。record 1 件・lock 非残存・両 target の同一
  verdict render の assert は維持。
- **`stat_change_new_fingerprint` の恒真 assert 修正(C6)**:
  `fps.iter().all(|f| f != &fp1) || fps.contains(&fp1)`(常に真)
  → `fps.contains(&fp1)` — 「別 fingerprint の record が追加されて
  も旧 record は TTL 内で残存する」意図どおりの assert に修正。
- **新規 `fresh_lock_stale_crossing_takeover`(C9、17 件目)**:
  fresh mtime の valid-token lock を植え、`DedupLimits{lock_stale:
  50ms, follower_wait: 2s}` で follower の poll 中に lock が stale
  境界を跨ぐ経路を検証 — 初回 tick は Fresh → Wait、stale 越えの
  tick で Stale → rename-claim → takeover が決定的に走る。backdate
  手配置の `stale_lock_takeover` では到達しない mid-poll の
  Fresh→Stale 遷移と claim 経路を固定する。
- module 側の review 修正(C1/C2/C3/C4/C7/C8/C10/C11)は
  `implement-dedup-helpers-and-eval-cache-module.md` の IR を参照。

### Post-review Validation

- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
  hook_eval_dedup::` → **17/17 pass**(新規 test 含む)
- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration` →
  **203/203 pass**
- `mise exec -- cargo clippy -p cheaper-cli --features
  cheaper-cli/dev-seam --all-targets -- -D warnings` → clean
- `cargo fmt --check` → clean
- `mise run check` は引き続き host の commit limit 枯渇リスクのため
  未実行(前回 Risks と同じ環境要因)。
