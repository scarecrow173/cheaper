---
id: "DESIGN-034Kp4aEcwrsOReUSPyxU9"
type: "design"
status: "draft"
title: "Deterministic route tests for ubuntu CI flake (issue #157)"
created: "2026-10-22"
updated: "2026-10-22"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/157"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/log_store.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "../adr/resolve-route-test-flake-via-test-side-determinism.md"
    - "../discovery/ubuntu-ci-route-test-flake.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Deterministic route tests for ubuntu CI flake (issue #157)

## Context

issue #157 で報告された 2 件の route 系 flaky test を、timing・
filesystem 列挙順への暗黙依存を排除した deterministic な assertion へ
修正する。discovery(ID `DISC-Gvis7cxcHxPQwsvoSGmuox`)で原因は確定済み:

- `exec_infra_failure_returns_error`
  (`crates/cheaper-cli/tests/integration/route_run.rs:599-637`):
  grant `timeout: '1ms'` で子 process(test binary 自身、`sleep`
  fixture)を spawn し、timeout kill 後に `marker.exists()` を assert
  している。POSIX 側の process-tree kill(`kill(-pid, SIGTERM)` →
  `SIGKILL`、`crates/cheaper-tools/src/tools/proc/tree.rs`)は
  ~1-3ms で発火し得るのに対し、子側の exec + Rust test binary 起動 +
  `fs::write(marker)` には通常でも数十 ms かかるため、ubuntu CI の
  負荷では kill が marker write に先勝つ。Windows では taskkill 経路が
  数百 ms 遅れるため marker が必ず先に作られ、platform 間の非対称が
  flake として現れていた。
- `sweep_enforces_ttl_count_size_and_skips_tmp`
  (`crates/cheaper-cli/src/route/log_store.rs:559-606`):
  `sweep_with` は mtime を `duration_since(UNIX_EPOCH).as_secs()` に
  **秒切り捨て**で収集し(log_store.rs:139-144)、
  `files.sort_by_key(mtime)`(line 166)で古い順に削除する。連続
  `fs::write` された file はほぼ必ず同一秒に乗り、stable sort の
  tie-break が `fs::read_dir` の列挙順 = filesystem 依存になる。

両者とも本番実装(timeout/kill 機構・sweep 削除順 semantics)は正しく、
違反している invariant は「test の assertion が制御下にない外部状態
(spawn→marker write の timing、readdir 列挙順)を観測している」こと。
責務境界は test 側にあり、ADR(Option A)で test-only の determinism
修正が選択済み。

## Scope

### In Scope

- `exec_infra_failure_returns_error`: `run_route` の戻り `Err` が
  `CheaperError` で `code() == ErrorCode::Timeout` であることを検証
  する assertion へ、`marker.exists()` を置き換える。
  `timeout: '1ms'`・30s sleep fixture・`is_err()` 相当の失敗検証・
  usage log に `"route"` record が無いことの検証は維持する。
- `sweep_enforces_ttl_count_size_and_skips_tmp`: (b) count sweep・
  (c) size sweep の対象 file に `File::set_modified` で一意な mtime
  順序を与え((a) TTL block が `old.log` に既に使っている pattern)、
  削除・残存 file の assertion を厳密な file 集合へ絞る。

### Out of Scope

- `proc::run`/`drive`/`cleanup`/`tree` の timeout・kill 実装変更。
- `RouteLogStore::sweep_with` の semantics 変更(mtime 秒精度・
  stable sort・tie 時任意順は production で問題ない)。
- 他の route 系 test・workflow test への波及。
- CI 構成(runs-on・retry・OS matrix)の変更。
- 新規 crate 依存の追加(`filetime` 等)。

## Components and Boundaries

### `exec_infra_failure_returns_error` 側

- 変更面は test body のみ。fixture 配線(`reject_fx` による
  `routes.yaml`/`routed-exec.workflow.json`/`policy.yaml` 生成、
  grant env の `CHEAPER_ROUTE_CHILD_MODE: sleep` と
  `CHEAPER_ROUTE_CHILD_MARKER`、`timeout: '1ms'` 注入、payload token
  生成)は一切変更しない。
- error 検証に必要な型は `cheaper_core::errors::{CheaperError,
  ErrorCode}`。`cheaper-core` は `cheaper-cli` の直接
  `[dependencies]`(crates/cheaper-cli/Cargo.toml)であり、同一
  package の integration test target から参照可能。既存先例:
  `tests/integration/envelope.rs:10` が同じ import を使っている。
- `run_route` は `Result<RouteRunOutput, cheaper_cli::cli::BoxError>`
  を返す(route_run.rs:183-197)。失敗値の取り出しは
  `.err().expect(...)` を使う — `unwrap_err()` は `RouteRunOutput:
  Debug` を要求するが、`Option::expect` は `T` の `Debug` 不要で
  常に使える。

### `sweep_enforces_ttl_count_size_and_skips_tmp` 側

- `#[cfg(test)] mod tests` 内に `set_mtime(path: &Path, time:
  SystemTime) -> io::Result<()>` の小 helper を置く。実体は
  `fs::File::options().write(true).open(path)?.set_modified(time)`
  のみ — 標準ライブラリ API で新規依存は要らない。先例:
  `crates/cheaper-cli/tests/integration/hook_eval_dedup.rs:127-132`
  の同名 helper、`crates/cheaper-cli/src/hook/eval_cache.rs:982`・
  同ファイル (a) block の inline 使用(log_store.rs:576-581)。
- helper を導入したら (a) block の inline 記述も helper 呼出しへ
  揃える — 同一 test 内の同一操作を 1 表現に集約する(表現ドリフト
  防止)。

## Data and Control Flow

### timeout error の決定的到達経路(`ErrorCode::Timeout` が post-spawn 証拠になる理由)

1. `command.spawn()` 成功後、`proc::monitor::drive`
   (`crates/cheaper-tools/src/tools/proc/monitor.rs`)が
   `sleep(timeout_ms)` で timeout を arm する。
2. 発火すると `timeout_controller.abort(Timeout)` →
   `AbortReason::from`(`crates/cheaper-tools/src/operation.rs`)→
   `wait_aborted` → cleanup → process-tree kill。
3. tool 層の error は `tool_executor_error` が `find_cheaper_error` で
   code を抽出し(`crates/cheaper-cli/src/workflow_run.rs:552-567`)、
   `WorkflowExecutorError(Timeout)` になる。
4. `outcome.error.code = Timeout` が `route::run::finish` に渡り、
   `route_exec.captured()` が `None` のとき
   `Err(CheaperError::new(error.code, "routed command did not
   complete"))` を返す(run.rs:450-463)。

`complete()` は tool 実行が `Ok(result)` を返したときのみ呼ばれる
(workflow_run.rs:532-536)。timeout kill は `Err` 側で返るため
`captured()` は `None` のままとなり、`finish` の error 分岐が
決定的に取られる。結果、最終 `Err` は必ず `CheaperError` 型で
`code() == ErrorCode::Timeout` — `downcast_ref` で取り出せる。

timeout error は `proc::drive` 内でしか生成されず、`drive` には
`command.spawn()` 成功後にしか入らないため、この code の検証は
「grant 経由で子が spawn され timeout kill で落ちた」ことの timing
非依存な証明になる — `marker.exists()` が担っていた識別力と同等
以上。

### `sweep_with` の順序決定機序

1. `fs::read_dir` で全 entry を走査し `*.tmp` を除外、
   `(mtime_secs, bytes, path)` を `Vec` に積む(log_store.rs:122-146)。
2. TTL 超過分を先に削除(log_store.rs:152-164)。
3. `files.sort_by_key(mtime)` で古い順化(stable — key 同値なら走査順
   保持)。
4. count 上限超過分を先頭(最古)から `remove(0)` で削除、次に合計
   size 上限を同様に削る(log_store.rs:168-181)。

test は file 生成直後に各 path の mtime を `set_modified` で
上書きし、step 3 の sort key が一意になるよう `as_secs` 単位で
1 秒以上離れた値を与える。これにより step 4 の削除順が
`read_dir` 列挙順に一切依存しなくなる。

## Decisions

### D1: spawn 証明は `Err` の `CheaperError.code() == ErrorCode::Timeout` で検証する

`run_route` の戻りから error を取り出し、次の形で assert する:

```rust
let err = run_route(&mut f, &token)
    .await
    .err()
    .expect("route must fail: grant timeout is 1ms");
assert_eq!(
    err.downcast_ref::<CheaperError>().map(CheaperError::code),
    Some(ErrorCode::Timeout),
    "timeout code は proc::drive 内・spawn 成功後にのみ生成される決定的 post-spawn 証拠",
);
```

- 選定理由: timeout code は `command.spawn()` 成功後の `proc::drive`
  内でしか生成されず、POSIX kill の早さ・子の起動遅延・marker write
  の有無に完全に独立。test が検証したい invariant「grant 経由で子が
  spawn され、infra failure(timeout)が error として返る」を直接
  表現する。
- 代替案:
  - grant timeout を数百 ms〜秒へ緩める(issue 本文の案):
    race の window を広げるだけで理論上の flake が残り、test も遅く
    なる → 不採用(ADR Option B)。
  - marker を poll/retry で待つ: 依然として timing 依存。timeout が
    先に発火する限り marker は永遠に来ない場合もあり、待機条件自体が
    不安定 → 不採用。
  - stderr text・usage record 等の副次的情報で spawn を推論する:
    code assert より識別力が低く brittle → 不採用。
- message 文字列("routed command did not complete")の assert は
  要求しない — code が invariant を担い、文言は実装詳細なので固定
  しない。

### D2: mtime 順序付けは std の `File::set_modified` で行い、新規依存を入れない

- 形式: test 内 helper `set_mtime(path, time)` =
  `fs::File::options().write(true).open(path)?.set_modified(time)`。
- 基準時刻は `let now = SystemTime::now();` を 1 回だけ取得し、
  `now - Duration::from_secs(k)` から導出する。`sweep_with` が
  `as_secs()` へ切り捨てるため、値は 1 秒以上(設計では 10 秒刻み)
  離す — `now` を共有しているため差分は必ず整数秒で一意になり、
  呼出し間の秒境界跨ぎで tie が再発しない。秒精度未満の FS や
  timestamp granularity が粗い FS でも安全。
- 代替案:
  - `filetime` crate 追加: std で十分(既存 code が同 API で実績
    あり)なので新規 dev-dep は YAGNI → 不採用。
  - write 間に sleep を挟んで自然 mtime を離す: 秒境界が必要なので
    1 秒超の待機 × file 数で遅く、しかも FS timestamp granularity
    依存が残る → 不採用。
  - file 生成順や名前順に頼る: これ自体が flake 原因(列挙順未定義)
    → 不採用。

### D3: (b) count sweep は a < fresh < b < c 順に mtime を与え、削除集合を {a, fresh} に固定する

- 現行コメントの期待「fresh + a が消える」を維持する順序:
  `a = now-40s`、`fresh = now-30s`、`b = now-20s`、`c = now-10s`
  (大小関係のみが意味を持つ。値自体は例示)。
- `sweep_with(Duration::MAX, 2, u64::MAX)` は 4 file から古い 2 件を
  削除 → 決定的に {a, fresh}。assertion は厳密集合へ:
  `assert_eq!(removed, 2)`、`!a.exists()`、`!fresh.exists()`、
  `b.exists()`、`c.exists()`。現行の `b.exists() || c.exists()`
  (条件付き disjunction)は廃止。
- 代替案: 削除対象を {a, b} に固定(a < b < fresh < c)する案も
  同等に決定的だが、現行コメントの意図(fresh + a が消える)を
  変えず最小の読み替えで済む {a, fresh} を採用。

### D4: (c) size sweep は big を small より古くして `removed == 1` の単一削除を維持する

- `big = now-20s`(古)、`small = now-10s`(新)とし、
  `sweep_with(Duration::MAX, usize::MAX, 100)` の削除対象を
  big(128B)に固定: 先頭から 1 件削ると合計は 8B ≤ 100 で停止。
- assertion: `assert_eq!(removed, 1)`、`!big.exists()`、
  `small.exists()`。現行の `big.exists() != small.exists()` を
  厳密形へ置き換える。
- 重要な順序制約: 逆順(small が古い)にすると small を先に消しても
  残り 128B > 100 で big も消え `removed == 2` になり、test 意図
  (合計上限に対する古い順削除 1 件)を変えてしまう。順序選択は
  assertion と連動させて決める。
- 代替案: `max_bytes` を変えて両方消える形にする等も可能だが、
  既存の `removed == 1` シナリオを維持する方が意図変更が最小。

### D5: error assertion・mtime helper ともに既存 repo convention を再利用する

- `err.downcast_ref::<CheaperError>().map(CheaperError::code)` の形は
  同ファイル群の既存 pattern と一致(log_store.rs:533/544/553、
  run.rs:1011/1069、`cheaper-tools`/`cheaper-workflow` の各 test)。
- `set_mtime` helper は `hook_eval_dedup.rs:127-132` の形を踏襲し
  doc comment も合わせる。
- `marker` 変数・`reject_fx` 呼出しは残す: signature が `&marker`
  を要求し、grant env の `CHEAPER_ROUTE_CHILD_MARKER` は
  `revalidation_failures_reject_without_executing` 側でも共有する
  fixture 構築物のため、assert 対象から外すだけで配線は変更しない。

## Acceptance Mapping

| SPEC requirement / Acceptance Criteria | 本 design の対応 |
| --- | --- |
| `run_route` が `Err` を返すことを維持 | D1: `.err().expect(...)` で `Err` 必須化(30s sleep vs 1ms timeout で決定的) |
| spawn 証明を `CheaperError.code() == ErrorCode::Timeout` で行う | D1: `downcast_ref::<CheaperError>().map(CheaperError::code)` assert。`marker.exists()` 系の timing 依存 assertion は残さない |
| usage log に `"route"` record が無い検証を維持 | D5: 現行 assertion を変更しない |
| (b)(c) で全対象 file に `File::set_modified` の一意 mtime 順序 | D2/D3/D4: helper 化 + 全 file へ明示 mtime |
| `.tmp` 除外・TTL sweep の検証を弱めない | (a) block と inflight `.tmp` の最終 assert は不変(helper 化のみ) |
| assertion が検証したい invariant を直接表す | D3/D4: 削除・残存を厳密 file 集合で assert |
| `cargo test ... route_run` / `cargo test -p cheaper-cli log_store` / `mise run check` が pass | Verification Notes に同一コマンドを記載 |

## Non-Functional Constraints

- 両 test は `fs::read_dir` 列挙順・mtime granularity・spawn→子側
  write の遅延・CPU 負荷に関わらず deterministic であること。
- `timeout: '1ms'` を維持するため wall-clock の遅延増加は無い。
- 本番 code(`proc`・`log_store`・`route::run`・`workflow_run`)への
  変更はゼロ — 変更面は 2 test file のみ。

## Risks and Trade-offs

- `marker.exists()` を外すと「子が実際に動作を開始した」行動証拠は
  失われる。ただし timeout code は spawn 成功後にしか到達しないため
  同等以上の post-spawn 証明であり、実害は無い(discovery 確認済み)。
- (a) block も helper 化する refactor は本質的でないが同一 test 内の
  表現統一のため。差分を最小化したい場合は (a) の inline 記述を残す
  選択も許容するが、統一を推奨。
- `set_modified` は platform 間で timestamp granularity が異なり得る
  が、1 秒以上離した値を使うため `as_secs` 比較に影響しない。

## Deferred Design Concerns

- `sweep_with` の tie-break を (mtime 秒, path) の secondary sort key
  で deterministic にする実装側 hardening — production の file 名
  (`<unix-ms>-<pid>-<hex>.log`)では mtime tie がほぼ発生せず主原因は
  test 構築側のため、本 scope では行わない(YAGNI。spec Deferred
  Design Concerns と同一)。将来 readdir 順への暗黙依存が別 test に
  現れた場合に再検討する。

## Verification Notes

- [ ] `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
  route_run::exec_infra_failure_returns_error -- --exact` が連続実行で
  安定 pass する
- [ ] `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam sweep_enforces` が
  pass する
- [ ] `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
  route_run` 全体・`cargo test -p cheaper-cli log_store` が pass する
- [ ] `mise run check` が pass する
- [ ] ubuntu CI で marker 未作成・readdir 順不定の条件でも fail
  しない(修正の性質上、assertion が timing/enumeration を参照しない
  ことのレビュー確認で代替可能)
