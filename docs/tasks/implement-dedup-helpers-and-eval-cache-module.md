---
id: "TASK-034IpPs1L6kjNy6HyCalZs"
type: "task"
status: "done"
title: "Implement dedup helpers and eval_cache module"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/127"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-hook-eval-dedup-verdict-cache.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-hook-eval-dedup-verdict-cache.md"
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

# Implement dedup helpers and eval_cache module

## Work

plan の T1(`routing_policy` additive helpers)+ T2(`eval_cache.rs` 本体)を
**1 つの task・1 commit** で実装する。T1 の新規 `pub(crate)` item
(`fingerprint_targets`・`FingerprintContext`/`ResolvedTarget`/`ResolvedKind`・
`MEASURE_ENV_KEYS`)は T2 が存在するまで consumer を持たないため、分割
commit すると `mise run check` の clippy `dead_code` に抵触する — 必ず同一
commit で land する。挙動は additive で不変(`targets.rs` 配線は次 task)。

### T1 — routing_policy helpers(additive・挙動不変)

- [x] `Verdict`(`routing_policy.rs:105`)に `#[derive(serde::Serialize,
      serde::Deserialize, PartialEq, Debug)]` + `#[serde(tag = "decision",
      rename_all = "lowercase")]` を追加 — wire 表現は `Allow →
      {"decision":"allow"}`、`Deny{reason} →
      {"decision":"deny","reason":"…"}`。`PartialEq`/`Debug` は test が
      2 つの verdict を `assert_eq!` で比較するために必要
- [x] `MEASURE_TIMEOUT`(`:39`)を `pub(crate) const` 化 — follower wait
      上限の単一 source(DRY)
- [x] `MEASURE_ENV_KEYS`(`:176`)を `pub(crate)` 化 — eval_cache の env
      snapshot が `measure_tokens` と同じ key 集合を cross-module で参照
      するため(consumer は `eval_cache` のみ。`resolve_request_cwd` は
      `fingerprint_targets` と同 module のため widening しない)
- [x] `pub(crate) enum ResolvedKind { File, Dir, NonFile }`・
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
- [x] `embedded.rs` の `embedded_fingerprint()`(`:123`)を `pub(crate)` 化

### T2 — eval_cache.rs 本体

- [x] `hook/mod.rs` の module 宣言群(`:16`–`17`)へ `pub mod
      eval_cache;` を追加
- [x] `DedupLimits`(`#[doc(hidden)]`、`Clone + Copy`、全 field `pub`)+
      `DedupLimits::PRODUCTION` const を定義 — `verdict_ttl = 60_000ms`・
      `lock_stale = 120_000ms`・`follower_wait =
      routing_policy::MEASURE_TIMEOUT`(30_000ms)・`poll_interval = 50ms`
- [x] store root を `resolve_user_config_dir(UserConfigDirOptions { env:
      Some(opts.env), homedir: None }).join("hook-eval")` で解決
      (`cheaper-core/config/dirs.rs:37` の既存規則を複製しない —
      `usage_log.rs` と同じ境界)
- [x] env snapshot: `opts.env`(`EnvLookup` = `&dyn Fn`、非 Send)から
      `MEASURE_ENV_KEYS`(`:176`)∪ `CHEAPER_READ_TOKEN_THRESHOLD` を owned
      map へ写す(`measure_tokens`(`:201`)と同じ snapshot pattern。
      blocking phase へ env closure 自体は渡さない)
- [x] fingerprint 算出: hash 入力専用の `#[derive(Hash)]` struct へ field
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
- [x] verdict record read: `<fp>.json` を読み `schemaVersion == 1`・
      `fingerprint` field が期待 fingerprint と一致・`verdict` deserialize
      成功・file mtime の checked-age(`SystemTime::now()
      .checked_duration_since(mtime)` が `Some(age) && age <=
      verdict_ttl`)を全て満たすもののみ serve。不存在・非 object・parse
      失敗・未知 version・fingerprint 不一致・TTL 超過・未来 mtime
      (`None` — clock skew・fs timestamp 異常)は全て absent 扱い
      (`usage.jsonl` と同じ forward-compat 規約)
- [x] verdict record write: `{"schemaVersion":1,"createdAt":<epoch ms —
      観察用で freshness 判定には使わない>,"fingerprint":<16hex>,
      "verdict":<Verdict serde>}` を `cheaper_core::fs_util::atomic_write`
      (`fs_util.rs:34`)で書く。`windows_remove_first: false`・
      `#[cfg(unix)] unix_mode: Some(0o600)` は struct literal 指定
      (auth/store.rs 慣行 — Windows build でもそのまま compile)。
      parent dir の `create_dir_all` は caller(eval_cache)の責務
- [x] leader lock: `<fp>.lock` を `OpenOptions::new().write(true)
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
- [x] follower bounded poll: `poll_interval` 周期で 1 tick 分の fs 操作を
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
- [x] sweep: dedup 経路先頭(blocking phase 内)で `hook-eval/` を
      best-effort sweep — `mtime > verdict_ttl` の `*.json`、`mtime >
      lock_stale` の `*.lock`、TTL 超過の `.<fp>.json.tmp-*` orphan を
      remove。checked-age 規則は上記と同一 — checked-age `None`(未来
      mtime)の entry も stale として remove 候補に含める。失敗は無視
- [x] `#[doc(hidden)] pub async fn evaluate_dedup_with(request, opts,
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
- [x] `pub async fn evaluate_dedup(request, opts) -> Verdict` =
      `evaluate_dedup_with(request, opts, &DedupLimits::PRODUCTION,
      || evaluate(request, opts))`
- [x] dedup 分類(hit/leader/follower/fallback/bypass)を `tracing::debug!`
      で emit(metadata-only 規約 — `verdict` field と同じ分類値スタイル)。
      `HookResponse` shape・stdout/stderr 契約には触れない
- [x] 任意(design が「実装判断」と明記): fast-path gate — `ctx.targets`
      が全て `File` かつ各 `len <= threshold`(byte fast path で allow
      確定)または全て `NonFile`(評価対象実 file 無し)の場合は協調を
      skip して `eval_fn` 直接呼出し。導入しても `NonFile` absent marker
      自体は fingerprint 構成に残す(mixed candidate 集合の同一性に必要)
- [x] fail-open 確認: eval_cache 内に `unwrap`/`expect`/`panic!` を置かず、
      全失敗が `eval_fn` 直接呼出しへ畳まれることを code review で確認
      (design Failure model 表。copilot 経路で新たな非 0 exit を生じない)

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-hook-eval-dedup-verdict-cache.md (T1+T2 節)
- docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md
- docs/specs/hook-duplicate-firing-single-evaluation-dedup.md

## Dependencies

- なし(plan の直列 chain の先頭 slice)。後続 task
  `docs/tasks/wire-evaluate-dedup-into-hook-target-call-sites.md` は本
  task の `evaluate_dedup` に依存する。

## Affected Files

- add: `crates/cheaper-cli/src/hook/eval_cache.rs`
- modify: `crates/cheaper-cli/src/hook/mod.rs`(`pub mod eval_cache;`)
- modify(additive): `crates/cheaper-cli/src/hook/routing_policy.rs`
- modify: `crates/cheaper-cli/src/embedded.rs`(`embedded_fingerprint` の
  `pub(crate)` 化)
- store 規約: `<userConfigDir>/hook-eval/` 配下(verdict record・lock・
  sweep)。workspace・`.cheaper/` には一切書かない

## Done When

- [x] `Verdict` が internally-tagged serde 表現(`{"decision":"allow"}` /
      `{"decision":"deny","reason":"…"}`)で serialize/deserialize でき、
      `PartialEq`/`Debug` を持つ
- [x] `fingerprint_targets` が `resolve_target` の containment 規則を
      複製せず再利用し、variant 別 target 列挙 + canonical
      `workspace_root` を返す
- [x] `eval_cache` が fingerprint 算出・record read/write(checked-age
      TTL)・leader lock(owner token + delete-if-owner)・follower
      bounded poll・TTL sweep・`evaluate_dedup`/`evaluate_dedup_with`/
      `DedupLimits` を提供する
- [x] eval_cache 内に `unwrap`/`expect`/`panic!` が無く、全失敗が
      `eval_fn` 直接呼出しへ畳まれる(fail-open)
- [x] T1+T2 が同一 commit で land している(dead_code 回避)

## Verification

- [x] `mise exec -- cargo check -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が exit 0
- [x] `mise exec -- cargo clippy -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が exit 0(dead_code 警告無し)
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::` が exit 0(既存 hook 契約が不変)
- [x] `grep -n "unwrap\|expect\|panic!" crates/cheaper-cli/src/hook/eval_cache.rs` が 0 件(fail-open 確認)
