---
id: "PLAN-034IfA0UHILhGW5DsirUEp"
type: "plan"
status: "approved"
title: "Implement hook eval dedup verdict cache"
created: "2026-10-18"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/127"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/hook-duplicate-firing-single-evaluation-dedup.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/hook-duplicate-firing-single-evaluation-dedup.md"
    - "docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md"
    - "docs/adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-eval-dedup-verdict-cache-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement hook eval dedup verdict cache Implementation Plan

## Goal

`cheaper hook` の normalize → evaluate の間に canonical request fingerprint
ベースの verdict cache + leader election を挿入し、同一 tool call に対して
重複発火する cheaper hook 登録(issue #127)の file token 計測・bounded
materialization・bounded search scan を高々 1 回へ dedup する。全 firing は
同一内容の `Verdict` を受け取り、自 target の adapter で render するため
host 観測挙動は不変。

## Scope

単一 implementation stream。`routing_policy` の additive helper(T1) →
`eval_cache` 本体(T2) → `targets` 配線(T3) → integration test(T4) の直列
chain であり、critical path と release intent が同一のため分割しない。

In scope:

- `hook::eval_cache` 新設 — fingerprint 算出・verdict record read/write・
  leader lock・follower bounded wait・TTL sweep・`evaluate_dedup` wrapper。
- `routing_policy` additive 変更 — `Verdict` serde derive・
  `MEASURE_TIMEOUT`/`MEASURE_ENV_KEYS` の `pub(crate)` 化・
  `fingerprint_targets` + `FingerprintContext`/`ResolvedTarget`/`ResolvedKind`
  の追加。
- `embedded.rs` の `embedded_fingerprint()` を `pub(crate)` 化。
- `targets.rs` `handle_hook` 5 arm の `evaluate` 呼出しを
  `eval_cache::evaluate_dedup` へ差し替え。
- `<userConfigDir>/hook-eval/` 配下の store 規約(verdict record・lock・
  sweep)。workspace・`.cheaper/` には一切書かない。
- `tests/integration/hook_eval_dedup.rs` 新設 + `main.rs` mod 宣言。

Out of scope(spec / design と同じ境界):

- `hook-config sync` の legacy-conflict gate blocker → warning 降格
  (dedup 実証後の別 follow-up)。
- `evaluate` の verdict semantics・deny reason・threshold 既定値・
  fail-open/fail-closed 境界・`resolve_target` containment 規則の変更。
- Devin 側の import merge / firing dedup(upstream 責務)。
- `cheaper hook` process spawn 自体の削減(計測・実体化コストのみが削減対象)。
- 新規 crate 依存・新規 env var・設定 knob(時間定数は `DedupLimits` の
  module const とし、test は注入 seam で差し替える)。

## Upstream Documents

- `docs/specs/hook-duplicate-firing-single-evaluation-dedup.md`
  (SPEC-034Ie1ZEdUR2aBFVoqkBay, approved) — R1–R9 要件と acceptance criteria。
- `docs/adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md`
  (ADR-034Ie3HFd6tIJms6Z2NIow, accepted) — fingerprint verdict cache +
  leader election を normalize → evaluate の間に挿入する構造決定。

## Design Inputs

- `docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md`
  (DESIGN-034IefWW6OJ9rvESaNvKUK, approved) — fingerprint の field 列挙・
  verdict record wire schema・lock protocol・failure model・test 構成を
  確定済み。本 plan の実装青写真。

## File Map

| Path | Change | Responsibility | Notes |
| --- | --- | --- | --- |
| `crates/cheaper-cli/src/hook/eval_cache.rs` | add | fingerprint 算出・verdict record store(`<userConfigDir>/hook-eval/`)・leader lock・follower bounded poll・TTL sweep・`evaluate_dedup` / `evaluate_dedup_with` / `DedupLimits` | render・normalize・verdict 判定には関与しない。「`evaluate` を呼ぶか共有済み verdict を返すか」の分岐のみ |
| `crates/cheaper-cli/src/hook/mod.rs` | modify | `pub mod eval_cache;` 追加(`:16`–`17` の module 宣言群) | |
| `crates/cheaper-cli/src/hook/targets.rs` | modify | `handle_hook` 5 arm(`:638` windsurf / `:646` copilot / `:653` pretooluse / `:660` devin / `:671` cursor)の `evaluate(&request, &deps.eval_opts())` を `eval_cache::evaluate_dedup(...)` へ差し替え | normalize・render・`fallback_allow`・`execute_hook` catch-all・完了 `debug!` event は無変更 |
| `crates/cheaper-cli/src/hook/routing_policy.rs` | modify(additive) | `Verdict`(`:105`)へ serde derive + internally-tagged attribute、`MEASURE_TIMEOUT`(`:39`)と `MEASURE_ENV_KEYS`(`:176`)の `pub(crate)` 化、`fingerprint_targets` + `FingerprintContext`/`ResolvedTarget`/`ResolvedKind` 追加 | verdict semantics・`resolve_target` 規則は変更しない |
| `crates/cheaper-cli/src/embedded.rs` | modify | `embedded_fingerprint()`(`:123`)を `pub(crate)` 化 | fingerprint の builtin `None` marker に使用 |
| `crates/cheaper-cli/tests/integration/hook_eval_dedup.rs` | add | module 層 seam test + binary 層 dual-spawn test | `cheaper_cli::hook::eval_cache` の `#[doc(hidden)] pub` seam(`evaluate_dedup_with`/`DedupLimits`)を利用 |
| `crates/cheaper-cli/tests/integration/hook.rs` | modify | `Fixture`(`:37`)・`run_hook`(`:109`)・`windsurf_read`(`:96`)・`tool_use_payload`(`:698`)・`devin_payload`(`:906`)・`big_text`(`:22`)/`write_big`(`:82`)・`HookResult`(`:103`)を `pub(crate)` 化 — sibling module `hook_eval_dedup.rs` から `crate::hook::` 経由で再利用するため(アクセス対象の struct field も `pub(crate)` 化) | `Fixture` が `CHEAPER_CONFIG_DIR`・`HOME`・`USERPROFILE` を tempdir へ隔離する env map を供給する点が必須 — test が実 `~/.config/cheaper/hook-eval/` を一切触れないことを保証する |
| `crates/cheaper-cli/tests/integration/main.rs` | modify | `mod hook_eval_dedup;` 宣言追加(`mod hook_config;` の後、alphabetical 順) | AGENTS.md の CLI integration test 配置規約 |

## Implementation Sequence

task slice は T1 → T2 → T3 → T4 の直列(依存関係は Dependency Graph 参照)。

### T1 — routing_policy helpers(additive・挙動不変)

- [ ] `Verdict`(`routing_policy.rs:105`)に `#[derive(serde::Serialize,
      serde::Deserialize, PartialEq, Debug)]` + `#[serde(tag = "decision",
      rename_all = "lowercase")]` を追加 — wire 表現は `Allow →
      {"decision":"allow"}`、`Deny{reason} →
      {"decision":"deny","reason":"…"}`。`PartialEq`/`Debug` は T4 が
      2 つの verdict を `assert_eq!` で比較するために必要
- [ ] `MEASURE_TIMEOUT`(`:39`)を `pub(crate) const` 化 — follower wait
      上限の単一 source(DRY)
- [ ] `MEASURE_ENV_KEYS`(`:176`)を `pub(crate)` 化 — T2 の env snapshot
      が `measure_tokens` と同じ key 集合を cross-module で参照するため
      (consumer は `eval_cache` のみ。`resolve_request_cwd` は
      `fingerprint_targets` と同 module のため widening しない)
- [ ] `pub(crate) enum ResolvedKind { File, Dir, NonFile }`・
      `pub(crate) struct ResolvedTarget { pub resolved: String, pub kind:
      ResolvedKind }`・`pub(crate) struct FingerprintContext { pub
      workspace_root: Option<String>, pub targets: Vec<ResolvedTarget> }`・
      `pub(crate) fn fingerprint_targets(request: &RoutingRequest) ->
      FingerprintContext` を追加。variant 別規則は design の表どおり:
  - `workspace_root` = `cheaper_core::workspace::paths::
    canonicalize_workspace_root(resolve_request_cwd の絶対解決側 cwd_abs)`
    の結果、失敗時 `None`
  - `FileRead` — `resolve_target(path, cwd)` が `Resolution::File` →
    `[File]`。`Dir`/`WorkspaceRoot`/`OutOfScope` → `[]`(常時 allow の
    定数経路で dedup 対象外)
  - `Search` — `File` → `[File]`、`Dir`/`WorkspaceRoot` → `[Dir]`、
    `OutOfScope` → `[]`
  - `Shell` — `extract_read_candidates(command)`(`:1120`, 既 `pub`)の各
    候補を `resolve_target` し、`File` → `File`、`Dir`/`WorkspaceRoot` →
    `Dir`、それ以外(missing・非 file/dir・workspace 外)→ `NonFile`
    (raw candidate 文字列を `resolved` に保持)。candidates 空 → `[]`
  - `File`/`Dir` の `resolved` は `std::fs::canonicalize` 済み絶対 path
    (`resolve_target` の lexical `nodepath::resolve` 結果を canonicalize
    して `./x`・symlink 経由・語尾 `/` 差等の spelling 差を除去)。
    canonicalize 失敗(resolve 後の fs race)はその候補を `NonFile`
    (absent)へ畳む
- [ ] `embedded.rs` の `embedded_fingerprint()`(`:123`)を `pub(crate)` 化
- [ ] `mise exec -- cargo check -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` で compile を確認し、
      `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
      hook::` で既存 hook 契約が不変であることを確認

※ commit 単位の注意: T1 の新規 `pub(crate)` item(`fingerprint_targets`・
`FingerprintContext`/`ResolvedTarget`/`ResolvedKind`・`MEASURE_ENV_KEYS`)
は T2 が存在するまで consumer を持たない — `mise run check` の clippy
dead_code を避けるため T1+T2 は同一 commit で land する(または T2
登場まで module-private に留める)。

### T2 — eval_cache.rs 本体(depends T1)

- [ ] `hook/mod.rs` の module 宣言群(`:16`–`17`)へ `pub mod
      eval_cache;` を追加 — File Map への記載だけでは file が orphan
      のままなので checklist にも明示する
- [ ] `DedupLimits`(`#[doc(hidden)]`、`Clone + Copy`、全 field `pub`)+
      `DedupLimits::PRODUCTION` const を定義 — `verdict_ttl = 60_000ms`・
      `lock_stale = 120_000ms`・`follower_wait =
      routing_policy::MEASURE_TIMEOUT`(30_000ms)・`poll_interval = 50ms`
- [ ] store root を `resolve_user_config_dir(UserConfigDirOptions { env:
      Some(opts.env), homedir: None }).join("hook-eval")` で解決
      (`cheaper-core/config/dirs.rs:37` の既存規則を複製しない —
      `usage_log.rs` と同じ境界)
- [ ] env snapshot: `opts.env`(`EnvLookup` = `&dyn Fn`、非 Send)から
      `MEASURE_ENV_KEYS`(`:176`)∪ `CHEAPER_READ_TOKEN_THRESHOLD` を owned
      map へ写す(`measure_tokens`(`:201`)と同じ snapshot pattern。
      blocking phase へ env closure 自体は渡さない)
- [ ] fingerprint 算出: hash 入力専用の `#[derive(Hash)]` struct へ field
      単位で明示列挙し、`DefaultHasher` → `format!("{:016x}",
      hasher.finish())`(`embedded_fingerprint` と同じ形)。hash 入力は
      design Fingerprint 節の列挙どおり:
  - `"hook-eval-v1"` format version tag
  - request 由来: kind discriminant・`workspace_root`(`None` は
    `unresolved` marker)・`FileRead` は `range`(無し/`Some(offset,
    limit)`)・`Search` は `pattern`・`max_results`・`context`・`glob`・
    `output_mode` discriminant 文字列(`"content"`/`"files_with_matches"`/
    `"count"`/`"unknown"`)・`case_insensitive`・`Shell` は raw `command`
    (deny reason が `collapse_command(command)` を埋め込むため文字列差は
    verdict 差)
  - stat signature: `ctx.targets` 列挙順に `File` → `(resolved, "file",
    mtime_nanos, len)`、`Dir` → `(resolved, "dir", mtime_nanos)`、
    `NonFile`・metadata 失敗 → `(raw, "absent")` marker。mtime は
    `metadata.modified().duration_since(UNIX_EPOCH).as_nanos()`
  - policy/env 入力: `threshold(opts.env)`(`:130`)の `f64::to_bits()`・
    `count_max_bytes(opts.env)`(`count.rs:28`)の
    `unwrap_or(DEFAULT_COUNT_MAX_BYTES)` 後の `u64`・builtin は
    `Some(dir)` なら `<dir>/policy.yaml` の `(mtime_nanos, len)` or
    `absent`、`None` なら `embedded_fingerprint()` marker・user は
    `<userConfigDir>/policy.yaml` stat or `absent`・project は
    `<workspace_root>/.cheaper/policy.yaml` stat or `absent`
    (`workspace_root` が `None` なら `unresolved` marker)
  - **含めない**: raw `path`・`cwd`・`target`・`source_tool`・session・
    pid・`CHEAPER_CONFIG_DIR`/`XDG_CONFIG_HOME`/`USERPROFILE`/`HOME`
    (store root 分岐で構造的分離済み)・hook event 名。`RoutingRequest`
    への `derive(Hash)` は使わない(`source_tool` 混入で key が割れる)
- [ ] verdict record read: `<fp>.json` を読み `schemaVersion == 1`・
      `fingerprint` field が期待 fingerprint と一致・`verdict` deserialize
      成功・file mtime の checked-age(`SystemTime::now()
      .checked_duration_since(mtime)` が `Some(age) && age <=
      verdict_ttl`)を全て満たすもののみ serve。不存在・非 object・parse
      失敗・未知 version・fingerprint 不一致・TTL 超過・未来 mtime
      (`None` — clock skew・fs timestamp 異常)は全て absent 扱い
      (`usage.jsonl` と同じ forward-compat 規約)
- [ ] verdict record write: `{"schemaVersion":1,"createdAt":<epoch ms —
      観察用で freshness 判定には使わない>,"fingerprint":<16hex>,
      "verdict":<Verdict serde>}` を `cheaper_core::fs_util::atomic_write`
      (`fs_util.rs:34`)で書く。`windows_remove_first: false`・
      `#[cfg(unix)] unix_mode: Some(0o600)` は struct literal 指定
      (auth/store.rs 慣行 — Windows build でもそのまま compile)。
      parent dir の `create_dir_all` は caller(eval_cache)の責務
- [ ] leader lock: `<fp>.lock` を `OpenOptions::new().write(true)
      .create_new(true)` で取得し、`{"pid":…,"createdAt":…,"nonce":…}`
      の owner token JSON を書込む(nonce は `SystemTime` nanos + process
      内 `AtomicU64` counter で生成可 — 新規 dep 不要)。lock 削除は全て
      **delete-if-owner**(remove 直前に payload を読み直し token 照合)。
      payload が parse 不能な lock は ownership を証明できないため、
      stale-remove・leader-remove とも削除**しない**(保守的) —
      mtime sweep でのみ期限切れ除去され、それまで follower は
      `follower_wait` deadline 到達で self-eval へ進む。
      publish 順序は verdict `atomic_write` 完了 → lock release(「lock
      無し・verdict 未 commit」の窓を消す)。`create_new` の非
      `AlreadyExists` error は `eval_fn` 直接呼出しへ畳む
- [ ] follower bounded poll: `poll_interval` 周期で 1 tick 分の fs 操作を
      `spawn_blocking` へ逃がし、(1) record fresh+valid → serve、(2)
      lock 不在 → `create_new` retry(成功で昇格して評価・publish・
      release)、(3) lock mtime checked-age が `lock_stale` 超過
      **または `None`** → delete-if-owner remove + retry、(4) lock
      fresh → `tokio::time::sleep(poll_interval)` で次 tick。
      checked-age 規則は record path と対称に適用する —
      `checked_duration_since` が `None`(未来 mtime — clock skew・
      fs timestamp 異常)の lock は stale 扱いで remove 候補とする。
      poll 中の fs error は「verdict 未着」として継続。
      `follower_wait` deadline 到達で `eval_fn` 直接呼出し(cache
      write しない — publish 経路を leader 単独に保つ)。blocking
      `std::thread::sleep` は使わない
- [ ] sweep: dedup 経路先頭(blocking phase 内)で `hook-eval/` を
      best-effort sweep — `mtime > verdict_ttl` の `*.json`、`mtime >
      lock_stale` の `*.lock`、TTL 超過の `.<fp>.json.tmp-*` orphan を
      remove。checked-age 規則は上記と同一 — checked-age `None`(未来
      mtime)の entry も stale として remove 候補に含める。失敗は無視
- [ ] `#[doc(hidden)] pub async fn evaluate_dedup_with(request, opts,
      limits, eval_fn) -> Verdict` を実装 — `fingerprint_targets`(sync)
      → `ctx.targets` が完全に空なら `eval_fn` 直接呼出し(bypass — 定数
      allow 経路。全候補 `NonFile` の Shell は空ではなく cache 対象)→
      env snapshot → `spawn_blocking` phase A(sweep → policy/env 入力確定
      → stat signature 収集 → policy 層 signature → fingerprint 確定。
      fs error・`create_dir_all` 失敗 → `eval_fn` へ畳む)→
      `spawn_blocking` phase B(record read → serve or lock `create_new`
      + owner token 書込み)→ leader(`eval_fn` await → record write →
      delete-if-owner release)/follower(bounded poll)分岐。評価**後**の
      失敗 sub-case: leader の verdict `atomic_write` が `eval_fn`
      完遂後に失敗した場合は delete-if-owner で lock を外し、その
      verdict をそのまま返す(`eval_fn` を**再実行しない**)。
      「全失敗を `eval_fn` 直接呼出しへ畳む」規則は評価**前**の段階
      (fingerprint 確定・record read・lock 取得)にのみ適用する。
      `eval_fn` は
      `F: Fn() -> Fut, Fut: Future<Output = Verdict>`(借用を取る
      closure の HRTB 問題を構造で消す — caller が request/opts を
      capture する形)
- [ ] `pub async fn evaluate_dedup(request, opts) -> Verdict` =
      `evaluate_dedup_with(request, opts, &DedupLimits::PRODUCTION,
      || evaluate(request, opts))`
- [ ] dedup 分類(hit/leader/follower/fallback/bypass)を `tracing::debug!`
      で emit(metadata-only 規約 — `verdict` field と同じ分類値スタイル)。
      `HookResponse` shape・stdout/stderr 契約には触れない
- [ ] 任意(design が「実装判断」と明記): fast-path gate — `ctx.targets`
      が全て `File` かつ各 `len <= threshold`(byte fast path で allow
      確定)または全て `NonFile`(評価対象実 file 無し)の場合は協調を
      skip して `eval_fn` 直接呼出し。導入しても `NonFile` absent marker
      自体は fingerprint 構成に残す(mixed candidate 集合の同一性に必要)
- [ ] fail-open 確認: eval_cache 内に `unwrap`/`expect`/`panic!` を置かず、
      全失敗が `eval_fn` 直接呼出しへ畳まれることを code review で確認
      (design Failure model 表。copilot 経路で新たな非 0 exit を生じない)
- [ ] `mise exec -- cargo clippy -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が pass

### T3 — targets.rs 配線(depends T2)

- [ ] `targets.rs` へ `eval_cache` の import を追加(`use super::eval_cache;`
      等、同 module 内の既存 import 形式に合わせる)
- [ ] 5 arm の `evaluate(&request, &deps.eval_opts()).await` を
      `eval_cache::evaluate_dedup(&request, &deps.eval_opts()).await` へ
      差し替え(`:638` windsurf / `:646` copilot / `:653` pretooluse /
      `:660` devin / `:671` cursor)。戻り値は同じ `Verdict` で
      `verdict_name`・各 `*_render`・完了 `debug!` event は無変更
- [ ] `use super::routing_policy::{...}`(`targets.rs:16`)の import
      list から `evaluate` を除去 — 5 call site 差し替え後は未使用で
      clippy `-D warnings`(unused_imports)に抵触する
- [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
      hook::` で既存 5 target 契約に回帰が無いことを確認

### T4 — integration tests(depends T3)

新規 `crates/cheaper-cli/tests/integration/hook_eval_dedup.rs` + `main.rs`
へ `mod hook_eval_dedup;`。`#[tokio::test(flavor = "current_thread")]`・
`tests/common` の fixture/helper・`hook_config.rs` の `env!("CARGO_BIN_EXE_
cheaper")` PATH shim/spawn パターンを踏襲する。`hook.rs` の
`Fixture`/`run_hook`/payload helper(`windsurf_read`・`tool_use_payload`・
`devin_payload`・`big_text`/`write_big`・`HookResult`)は File Map の
`pub(crate)` 化により `crate::hook::` 経由で再利用する — `Fixture` が
`CHEAPER_CONFIG_DIR`・`HOME`・`USERPROFILE` を tempdir へ隔離する env
map を供給するため、test が実 `~/.config/cheaper/hook-eval/` を一切
触れないことが構造的に保証される。module 層 seam は
`cheaper_cli::hook::eval_cache::evaluate_dedup_with`(`#[doc(hidden)] pub` —
integration test crate から到達可)へ `Arc<AtomicUsize>` で計数する
`eval_fn`(200ms sleep で重なり確定)と小さい `DedupLimits` を注入する。
fingerprint は `<cfg>/hook-eval/` 内の `<16hex>.json` file 名から取得する
(最初の 1 firing で生成させてから lock/record を手配置する手順)。

- [ ] `concurrent_firings_single_evaluation` — module seam: 同一
      `RoutingRequest`・同一 `CHEAPER_CONFIG_DIR` で 2 つの
      `evaluate_dedup_with` を `tokio::join!` — 計数 = 1・両 verdict が
      同一内容。3 発目は計数不変の純粋 cache hit
- [ ] `binary_dual_spawn_same_verdict` — binary 層: `cheaper hook` を
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
- [ ] `per_target_render_identity` — 同一 payload 2 連続実行で 1 回目
      (leader 評価)と 2 回目(cache serve)の stdout/stderr/exit が byte
      同一 — windsurf(stderr + exit 2)・copilot(deny JSON + exit 0)・
      pretooluse(`hookSpecificOutput`)・devin(`decision:"block"`)・
      cursor(permission JSON)の 5 target(`crate::hook::` 経由で
      `pub(crate)` 化した fixture/payload helper を再利用)
- [ ] `stat_change_new_fingerprint` — file の size または mtime 変化で
      別 fingerprint → 再評価(計数増・別 `<fp>.json`)
- [ ] `threshold_change_new_fingerprint` — `CHEAPER_READ_TOKEN_THRESHOLD`
      変化で別 fingerprint → 再評価
- [ ] `policy_and_count_cap_new_fingerprint` — policy 層 + count cap の
      fingerprint invalidation: user `<userConfigDir>/policy.yaml` の stat
      変化(mtime/len)と `CHEAPER_COUNT_MAX_BYTES` 変化がそれぞれ別
      fingerprint を生成し、独立評価される(R2 — verdict を変え得る
      観測可能入力の同一性キー反映)
- [ ] `stale_lock_takeover` — `<fp>.lock` 手配置 +
      `std::fs::File::set_modified` で `lock_stale` 超過の古い mtime →
      follower が takeover して評価完遂
- [ ] `fresh_lock_no_verdict_self_eval` — fresh lock + verdict 無し
      (crash mid-eval 模擬)で follower が `follower_wait` deadline 後に
      self-eval(計数 = 2)
- [ ] `live_leader_delete_if_owner` — delete-if-owner primitive を直接
      unit test して決定的に検証: owner token T2 を含む lock file に
      T1 で remove を試みると file が残存し、T1 を含む lock file は
      T1 で除去される。3-party timing race の観察には依存しない —
      本 primitive 層 coverage が race observation を置き換える
      (unparseable payload の非削除規則もここで固定)
- [ ] `future_mtime_not_served` — 未来 mtime の record / lock が serve
      されず stale 扱いで sweep 対象
- [ ] `cwd_spelling_same_fingerprint` — tempdir 配下の**絶対** spelling
      差(`<tmp>/./sub`・`<tmp>/sub/`・symlink `<tmp>/link` →
      `<tmp>/sub`)が `canonicalize_workspace_root` + canonicalize 済み
      resolved path で同一 fingerprint に収束。相対 spelling
      (`./sub`・`sub`)は process cwd に anchor されるため
      `set_current_dir` が要り global state race を招く — 使わない
- [ ] `corrupt_record_fallback` — 非 JSON・未知 `schemaVersion`・
      `fingerprint` field 不一致・TTL 超過 backdated mtime の record が
      serve されず通常評価へ fallback
- [ ] `dir_nested_edit_ttl_bound` — dir target の nested file 編集は dir
      stat signature に現れず、TTL 内では同一 verdict が serve される
      (staleness bound = TTL + 評価時間であることを固定)
- [ ] `copilot_unwritable_cache_exit_zero` — `CHEAPER_CONFIG_DIR` を file
      path へ向け cache dir 作成不能にしても copilot target が常に
      exit 0 + schema-valid JSON
- [ ] `sweep_removes_stale` — TTL 超過 `.json`・stale `.lock`・orphan
      `.tmp-*` が次回起動で除去
- [ ] `no_workspace_writes` — workspace 内に `hook-eval/` が生成されない
      (R8 — `<ws>` 配下に新規 file/dir が無いことを assert)

## Dependency Graph

| Slice | Depends on | Blocks |
| --- | --- | --- |
| T1 routing_policy helpers | — | T2 |
| T2 eval_cache.rs 本体 | T1 | T3 |
| T3 targets.rs 配線 | T2 | T4 |
| T4 integration tests | T3 | — |

critical path は T1 → T2 → T3 → T4 で並行 stream は無い — T2 は T1 の
`pub(crate)` helper・serde derive を直接消費し、T3 は T2 の
`evaluate_dedup` が無ければ compile せず、T4 は T3 の配線済み binary と
seam を検証する。T4 内の個別 test case は互いに独立。

## Dependencies and Risks

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | TTL staleness — stat signature に現れない内容変化(mtime granularity 内の in-place write・dir target の nested file 編集)で古い verdict を serve(data-loss 相当) | med | low | checked-age TTL(60s)+ 評価時間で bound する受容済み残差(design Risks 節)。未来 mtime は `checked_duration_since` の `None` で stale 扱い。内容 hash・dir 再帰 stat は dedup の効率目標と競合するため採用しない |
| 2 | dead-leader window — leader crash で verdict 未 commit・lock 残存 | low | med | publish 順序(record `atomic_write` → lock release)で「lock 無し・verdict 無し」の窓を消し、follower は `lock_stale`(120s)で takeover または `follower_wait`(30s)deadline で self-eval。どちらでも verdict 必着・出力分岐なし |
| 3 | fs 失敗・record 破損 — dir 作成不可・read/write/lock error・corrupt JSON | med | low | 全経路 `eval_fn` 直接呼出しへ fail-open。`schemaVersion`・`fingerprint` field 照合・deserialize・checked-age の検証で破損 record を absent 扱い。copilot never-nonzero 契約は `copilot_unwritable_cache_exit_zero` で固定 |
| 4 | live leader の stale 誤判定 — shell multi-candidate 評価が `lock_stale` を超える非有界 eval | low | low | owner token 照合(delete-if-owner)で旧 leader が後継 lock を消せず、害は冗長評価のみに留まる。`lock_stale` 120s は単一 candidate の `MEASURE_TIMEOUT` を十分上回る |
| 5 | `DefaultHasher` 衝突で別入力の verdict を serve | low | low | 64bit・user-trusted 境界(`<userConfigDir>`)内 cache で、hook の guard は cost/routing 制御(security boundary ではない)のため許容(ADR More Information)。`fingerprint` field 照合が file 名と内容の不整合を排除 |
| 6 | per-target 応答の破壊的変更(breaking-change) | low | high | cache するのは canonical `Verdict` のみで render 済み `HookResponse` は共有しない。`per_target_render_identity` で 5 target の byte 同一を固定 |
| 7 | 外部依存 availability — 新規 crate dep・新規 env var 無し。外部状態は `<userConfigDir>` の fs のみ | low | low | fs 失敗は全て fail-open。時間定数は `DedupLimits` 注入で env var 化せず、test は並行実行でも決定的 |
| 8 | test flake — timing 依存の協調 test | med | low | `DedupLimits` を ms スケールへ注入 + 計数 `eval_fn` seam で決定的検証。binary 層は stdout/exit 一致と file 残存のみ assert(タイミング非依存) |
| 9 | transient-failure `Allow` の共有 — leader が計測の transient 失敗(timeout・IO error・import 失敗で `Measured::Failed` → `Allow`)で得た verdict が、TTL 内の同一 fingerprint firing 全てへ共有される | low | low | spec R2 で受容済みの fail-open trade-off(失敗由来の非決定的 `Allow` を fail-open 方向へ共有してよいと明示)。TTL 経過後の firing は再評価されるため永続化しない |

### Rollback

- **Trigger**: dedup 経路の障害(verdict 分岐・hook 応答異常・latency
  悪化・copilot 非 0 exit 報告)。
- **Procedure**: 完全 revert unit = `targets.rs` の 5 call site を
  `evaluate(&request, &deps.eval_opts()).await` へ差し戻し
  (`use super::routing_policy` import へ `evaluate` を復元し `use
  super::eval_cache;` を除去) + `tests/integration/hook_eval_dedup.rs`
  と `main.rs` の `mod hook_eval_dedup;` 宣言を除去 +
  `src/hook/eval_cache.rs` と `mod.rs` の `pub mod eval_cache;` 宣言を
  除去。`pub(crate)` 化した T1 helper・hook.rs fixture の visibility
  widener は残しても dead code として無害だが、clippy 警告回避と
  表層復元のため併せて戻してよい(optional)。dedup は additive で
  `evaluate()` 本体は無改変のため call site の差し戻しで経路全体が
  現状へ復帰する。`<userConfigDir>/hook-eval/` 配下の残存
  `<fp>.json`・`.lock` は他機能が読まない inert file で自然に
  無効化される(workspace・`.cheaper/` は無汚染)。
- **Verification**: revert 後に `mise exec -- cargo test -p cheaper-cli
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
  integration` が広く pass すること — `hook::` のみでなく全 module
  を対象に、hook_eval_dedup 除去後の他 module 回帰も確認する。

## Task Breakdown

task doc へ分割する場合の候補(T1–T4 がそのまま 1 task 単位):

- T1: `routing_policy` additive helpers(Verdict serde・`pub(crate)` 化・
  `fingerprint_targets`)— 挙動不変の基盤変更
- T2: `eval_cache.rs` 本体(fingerprint・record schema・lock protocol・
  bounded wait・sweep・fail-open)
- T3: `targets.rs` 5 call site 配線 + 既存 hook 契約の回帰確認
- T4: `hook_eval_dedup.rs` integration test 群(module seam + binary 層)

## Verification

| Spec AC / Requirement | Test / Check | Command |
| --- | --- | --- |
| AC1: 同一 request の N firing で計測高々 1 回・残り serve(R1/R3) | `concurrent_firings_single_evaluation`(計数 = 1)+ `binary_dual_spawn_same_verdict` | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_eval_dedup::` |
| AC2: 全 firing が同一 verdict・per-target contract で render(R4/R9) | `per_target_render_identity`(5 target byte 同一)+ `binary_dual_spawn_same_verdict` | 同上 |
| AC3: file stat・threshold・policy 層・count cap 変化で別 fingerprint・独立評価(R1/R2) | `stat_change_new_fingerprint`・`threshold_change_new_fingerprint`・`policy_and_count_cap_new_fingerprint`(policy 層 stat・`CHEAPER_COUNT_MAX_BYTES` の各 invalidation) | 同上 |
| AC4: leader crash / stale lock で follower が独立評価へ fallback(R5) | `stale_lock_takeover`・`fresh_lock_no_verdict_self_eval`・`live_leader_delete_if_owner` | 同上 |
| AC5: 破損 / fs 失敗で copilot 非 0 exit 無し・通常評価 fallback(R5/R6) | `corrupt_record_fallback`・`copilot_unwritable_cache_exit_zero` | 同上 |
| AC6: stale entry が bounded TTL 内に sweep・期限切れ serve 無し(R7) | `future_mtime_not_served`・`sweep_removes_stale` | 同上 |
| R1 補完: cwd spelling 差の canonical 収束・除外 field の非混入 | `cwd_spelling_same_fingerprint`(+ fingerprint 構成の code review) | 同上 |
| R8: workspace・`.cheaper/` 非書込・書込失敗が応答に影響しない | `no_workspace_writes`・`copilot_unwritable_cache_exit_zero` | 同上 |
| staleness bound 明文化(dir nested 編集は TTL のみで bound) | `dir_nested_edit_ttl_bound` | 同上 |
| 回帰: 既存 5 target 契約不変 | `hook::` 既存 test 全件 | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::` |
| 全体 | canonical pipeline(biome・fmt・clippy・version/doc-id・`cargo test --workspace`・`node --test`・lockfile/payload check) | `mise run check` |

## Open Questions / Gaps

- design が field 列挙・wire schema・lock protocol・failure model を
  確定済みのため、実装上の不明点は無い。残る判断余地は design が
  「実装判断」と明記した fast-path gate(全 `File` かつ `len <=
  threshold` / 全 `NonFile` で協調 skip)の採否のみ — 採否どちらでも
  correctness に影響せず、採用しても `NonFile` absent marker は
  fingerprint 構成に残す。
- nonce 生成は新規 dep を許さない範囲で実装者が選ぶ(`SystemTime` nanos
  + process 内 `AtomicU64` counter で一意性は十分)。

## Self-Review

- [x] Every spec requirement maps to a task.(R1–R9・AC1–AC6 →
      Verification matrix 参照)
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
      consistent.

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation — T1–T4
  は直列だが各 slice が review checkpoint として独立し、T4 の test
  case 群は列挙済みで機械的に分割可能。
- Independent work streams: なし(T1 → T2 → T3 → T4 の直列 critical
  path)。T4 内の個別 test case は互いに独立。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 前提: upstream 3 doc が approved/accepted で design が fingerprint
  field 列挙・wire schema・lock protocol・failure model を確定済み。
  plan は design を implementation order へ転記したもので独自判断を
  足していない。
- 重点確認: (a) `Verdict` の internally-tagged serde 表現が record
  schema の `verdict` field と一致するか、(b) `fingerprint_targets` が
  `resolve_target` の containment 規則を複製せず再利用しているか、
  (c) test が module seam(`evaluate_dedup_with`)と binary 層の 2 層で
  協調を網羅しているか、(d) fail-open — eval_cache 内に panic 経路が
  無いか。
- 除外確認: fingerprint に `target`・`source_tool`・session・pid・raw
  `path`/`cwd`・config-dir 系 env が混入していないこと(R1)。
