---
id: "TASK-034IpPzy0tH0R50dS4XIN0"
type: "task"
status: "done"
title: "Add hook_eval_dedup integration tests"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/127"
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
    - "docs/plans/implement-hook-eval-dedup-verdict-cache.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-hook-eval-dedup-verdict-cache.md"
    - "docs/tasks/wire-evaluate-dedup-into-hook-target-call-sites.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-eval-dedup-verdict-cache-contract.md"
  references:
    - "docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md"
    - "docs/specs/hook-duplicate-firing-single-evaluation-dedup.md"
  defers: []
  deferred-by: []
---

# Add hook_eval_dedup integration tests

## Work

plan の T4: 新規 `crates/cheaper-cli/tests/integration/hook_eval_dedup.rs`
+ `main.rs` へ `mod hook_eval_dedup;` 宣言(`mod hook_config;` の後、
alphabetical 順 — AGENTS.md の CLI integration test 配置規約)。
`#[tokio::test(flavor = "current_thread")]`・`tests/common` の
fixture/helper・`hook_config.rs` の `env!("CARGO_BIN_EXE_cheaper")`
PATH shim/spawn パターンを踏襲する。`hook.rs` の
`Fixture`/`run_hook`/payload helper(`windsurf_read`・
`tool_use_payload`・`devin_payload`・`big_text`/`write_big`・
`HookResult`)は File Map の `pub(crate)` 化により `crate::hook::`
経由で再利用する — `Fixture` が `CHEAPER_CONFIG_DIR`・`HOME`・
`USERPROFILE` を tempdir へ隔離する env map を供給するため、test が
実 `~/.config/cheaper/hook-eval/` を一切触れないことが構造的に保証
される。module 層 seam は
`cheaper_cli::hook::eval_cache::evaluate_dedup_with`(`#[doc(hidden)]
pub` — integration test crate から到達可)へ `Arc<AtomicUsize>` で計数
する `eval_fn`(200ms sleep で重なり確定)と小さい `DedupLimits` を
注入する。fingerprint は `<cfg>/hook-eval/` 内の `<16hex>.json` file
名から取得する(最初の 1 firing で生成させてから lock/record を手配置
する手順)。

### 前提変更: hook.rs の `pub(crate)` 化(File Map どおり)

- [x] `hook.rs` の `Fixture`(`:37`)・`run_hook`(`:109`)・
      `windsurf_read`(`:96`)・`tool_use_payload`(`:698`)・
      `devin_payload`(`:906`)・`big_text`(`:22`)/`write_big`(`:82`)・
      `HookResult`(`:103`)を `pub(crate)` 化 — sibling module
      `hook_eval_dedup.rs` から `crate::hook::` 経由で再利用するため
      (アクセス対象の struct field も `pub(crate)` 化)。`Fixture` が
      `CHEAPER_CONFIG_DIR`・`HOME`・`USERPROFILE` を tempdir へ隔離する
      env map を供給する点が必須 — test が実
      `~/.config/cheaper/hook-eval/` を一切触れないことを保証する

### Test case 一覧(test-spec の Examples と 1:1 対応)

- [x] `concurrent_firings_single_evaluation` — module seam: 同一
      `RoutingRequest`・同一 `CHEAPER_CONFIG_DIR` で 2 つの
      `evaluate_dedup_with` を `tokio::join!` — 計数 = 1・両 verdict が
      同一内容。3 発目は計数不変の純粋 cache hit
- [x] `binary_dual_spawn_same_verdict` — binary 層: `cheaper hook` を
      `--target devin` 1 process + `--target pretooluse` 1 process で
      同時 spawn(同一 `CHEAPER_CONFIG_DIR`・同一 cwd)。両 process へ
      **同一の devin-shaped payload**
      (`{"hook_event_name":"PreToolUse","tool_name":"read",
      "tool_input":{"file_path":...}}`)を流す — 余分な key は
      `tool_use_normalize` で無視され、両 target が同一 FileRead
      request へ normalize されるため **cross-target dedup** が証明
      できる。`file_path` は threshold 超過の実 file を pin する —
      deny verdict が token 計測経路を通り、optional fast-path gate
      (T2)を採用しても skip されない実計測 verdict となることを保証。
      assert: 両 process が各 target contract で同一内容の deny を
      render(devin `decision:"block"` / pretooluse
      `hookSpecificOutput`)・`hook-eval/<fp>.json` が 1 件・`.lock`
      非残存
- [x] `per_target_render_identity` — 同一 payload 2 連続実行で 1 回目
      (leader 評価)と 2 回目(cache serve)の stdout/stderr/exit が byte
      同一 — windsurf(stderr + exit 2)・copilot(deny JSON + exit 0)・
      pretooluse(`hookSpecificOutput`)・devin(`decision:"block"`)・
      cursor(permission JSON)の 5 target(`crate::hook::` 経由で
      `pub(crate)` 化した fixture/payload helper を再利用)
- [x] `stat_change_new_fingerprint` — file の size または mtime 変化で
      別 fingerprint → 再評価(計数増・別 `<fp>.json`)
- [x] `threshold_change_new_fingerprint` — `CHEAPER_READ_TOKEN_THRESHOLD`
      変化で別 fingerprint → 再評価
- [x] `policy_and_count_cap_new_fingerprint` — policy 層 + count cap の
      fingerprint invalidation: user `<userConfigDir>/policy.yaml` の stat
      変化(mtime/len)と `CHEAPER_COUNT_MAX_BYTES` 変化がそれぞれ別
      fingerprint を生成し、独立評価される(R2 — verdict を変え得る
      観測可能入力の同一性キー反映)
- [x] `stale_lock_takeover` — `<fp>.lock` 手配置 +
      `std::fs::File::set_modified` で `lock_stale` 超過の古い mtime →
      follower が takeover して評価完遂
- [x] `fresh_lock_no_verdict_self_eval` — fresh lock + verdict 無し
      (crash mid-eval 模擬)で follower が `follower_wait` deadline 後に
      self-eval(計数 = 2)
- [x] `live_leader_delete_if_owner` — delete-if-owner primitive を直接
      unit test して決定的に検証: owner token T2 を含む lock file に
      T1 で remove を試みると file が残存し、T1 を含む lock file は
      T1 で除去される。3-party timing race の観察には依存しない —
      本 primitive 層 coverage が race observation を置き換える
      (unparseable payload の非削除規則もここで固定)
- [x] `future_mtime_not_served` — 未来 mtime の record / lock が serve
      されず stale 扱いで sweep 対象
- [x] `cwd_spelling_same_fingerprint` — tempdir 配下の**絶対** spelling
      差(`<tmp>/./sub`・`<tmp>/sub/`・symlink `<tmp>/link` →
      `<tmp>/sub`)が `canonicalize_workspace_root` + canonicalize 済み
      resolved path で同一 fingerprint に収束。相対 spelling
      (`./sub`・`sub`)は process cwd に anchor されるため
      `set_current_dir` が要り global state race を招く — 使わない
- [x] `corrupt_record_fallback` — 非 JSON・未知 `schemaVersion`・
      `fingerprint` field 不一致・TTL 超過 backdated mtime の record が
      serve されず通常評価へ fallback
- [x] `dir_nested_edit_ttl_bound` — dir target の nested file 編集は dir
      stat signature に現れず、TTL 内では同一 verdict が serve される
      (staleness bound = TTL + 評価時間であることを固定)
- [x] `copilot_unwritable_cache_exit_zero` — `CHEAPER_CONFIG_DIR` を file
      path へ向け cache dir 作成不能にしても copilot target が常に
      exit 0 + schema-valid JSON
- [x] `sweep_removes_stale` — TTL 超過 `.json`・stale `.lock`・orphan
      `.tmp-*` が次回起動で除去
- [x] `no_workspace_writes` — workspace 内に `hook-eval/` が生成されない
      (R8 — `<ws>` 配下に新規 file/dir が無いことを assert)

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-hook-eval-dedup-verdict-cache.md (T4 節)
- docs/test-specs/hook-eval-dedup-verdict-cache-contract.md (test
  contract — AC1–AC6 ↔ test case 対応)
- docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md

## Dependencies

- `docs/tasks/wire-evaluate-dedup-into-hook-target-call-sites.md` —
  配線済み binary(`evaluate_dedup` が 5 arm で有効)と module seam
  (`evaluate_dedup_with`/`DedupLimits`)を検証対象とする(plan の直列
  chain: T3 → T4)。

## Affected Files

- add: `crates/cheaper-cli/tests/integration/hook_eval_dedup.rs`
- modify: `crates/cheaper-cli/tests/integration/hook.rs`(fixture/helper
  の `pub(crate)` 化のみ)
- modify: `crates/cheaper-cli/tests/integration/main.rs`(`mod
  hook_eval_dedup;` 宣言追加)

## Done When

- [x] 上記 16 test case が `hook_eval_dedup.rs` に実装され全件 pass
- [x] `hook.rs` fixture/helper が `pub(crate)` 化され sibling module から
      再利用されている(実 `~/.config/cheaper/hook-eval/` を触れない
      隔離が維持される)
- [x] `main.rs` に `mod hook_eval_dedup;` が alphabetical 順で宣言済み
- [x] 既存 `hook::` test 群を含む integration target 全体に回帰が無い

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_eval_dedup::` が exit 0
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::` が exit 0(fixture `pub(crate)` 化による既存契約回帰無し)
- [ ] `mise run check` が exit 0(canonical pipeline 全体) — 未完了: 本 session host の
      commit limit 枯渇(親 agent process が ~140GB private commit 占有、free virtual
      ~1.6GB)で `cargo test --workspace` の rustc が LLVM OOM(2 回実行、
      `-j 2`/`-j 1` 両方で失敗)。biome・fmt・workspace clippy・version/doc-id・
      node scripts/benchmarks test・lockfile・apm-payload の各 step は個別に全 pass。
      `cargo test -p cheaper-cli`(integration 202・lib 25)は別途全 pass — code 起因
      ではないため IR Risks に記録
