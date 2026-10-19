---
id: "DESIGN-034IefWW6OJ9rvESaNvKUK"
type: "design"
status: "approved"
title: "Hook eval dedup verdict cache and leader election"
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
    - "../adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "./devin-native-hooks-and-apm-bootstrap.md"
    - "./cross-agent-native-routing-adapter-architecture.md"
    - "./huge-file-byte-cap-guard-and-fail-closed-hook.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/hook-duplicate-firing-single-evaluation-dedup.md"
    - "../adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../adr/copilot-adapter-separate-script-never-nonzero-exit.md"
    - "../adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "../adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
    - "../discovery/hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md"
    - "../../crates/cheaper-cli/src/hook/mod.rs"
    - "../../crates/cheaper-cli/src/hook/targets.rs"
    - "../../crates/cheaper-cli/src/hook/routing_policy.rs"
    - "../../crates/cheaper-cli/src/count.rs"
    - "../../crates/cheaper-cli/src/usage_log.rs"
    - "../../crates/cheaper-cli/src/embedded.rs"
    - "../../crates/cheaper-core/src/fs_util.rs"
    - "../../crates/cheaper-core/src/config/dirs.rs"
  defers: []
  deferred-by: []
---

# Hook eval dedup verdict cache and leader election

## Context

Devin CLI は `read_config_from` で他 agent の hook 設定を merge import
し、1 回の tool call に対して複数の cheaper hook 登録が発火し得る。
各 firing は独立した `cheaper hook` process を spawn し、同一 file の
token 計測・bounded materialization を個別に実行するため、コストが
登録数に比例する(issue #127)。

`routing_policy.rs` の `evaluate`(`:1230`)が返す `Verdict` は、
canonical `RoutingRequest` の policy 関連 field・対象 file 状態・
`threshold`(`CHEAPER_READ_TOKEN_THRESHOLD`)・`count_max_bytes`
(`CHEAPER_COUNT_MAX_BYTES`)・`builtin_dir`・policy 層
(builtin / user / project `policy.yaml`)の**決定的関数**であり、
`target`・`source_tool`・session・pid を読まない — ただし計測の
transient 失敗(`Measured::Failed`・`RangedScan` / `SearchScan` の
失敗は `Allow` へ畳まれる)による非決定性を modulo とする。この
決定性により、同一入力集合を識別する fingerprint さえ正しく構成
できれば、verdict の共有は近似ではなく厳密に正しい。失敗由来の
`Allow` が TTL 内で共有され得る点は Risks and Trade-offs の
受容済み fail-open トレードオフどおり。

本 design は SPEC-034Ie1ZEdUR2aBFVoqkBay(requirements R1–R9)と
ADR-034Ie3HFd6tIJms6Z2NIow(「canonical request fingerprint verdict
cache + leader election」を `handle_hook` の normalize → evaluate の
間に挿入する決定)を実装可能な精度へ落とすものであり、以下を確定する:

- `hook::eval_cache` 新規 module の公開面と内部プロトコル
- fingerprint の**正確な field 列挙**(含めるもの / 除外するもの)
- verdict record の wire schema と TTL 判定
- leader election(lock file)・follower bounded wait・stale takeover
- 失敗時の fail-open 規約と test 構成

## Scope

### In Scope

- `crates/cheaper-cli/src/hook/eval_cache.rs` の新設 — fingerprint
  算出・verdict store read/write・leader lock・bounded poll・TTL sweep・
  `evaluate_dedup` wrapper。
- `crates/cheaper-cli/src/hook/mod.rs` — `pub mod eval_cache;` 追加。
- `crates/cheaper-cli/src/hook/targets.rs` — `handle_hook` 5 arm の
  `evaluate(&request, &deps.eval_opts())` 呼出し(現状 `:638`, `:646`,
  `:653`, `:660`, `:671`)を `eval_cache::evaluate_dedup(&request,
  &deps.eval_opts())` へ差し替え。normalize・render・`fallback_allow`・
  `execute_hook` の catch-all は変更しない。
- `crates/cheaper-cli/src/hook/routing_policy.rs` — additive 変更のみ:
  - `Verdict`(`:105`)へ `serde::Serialize`/`Deserialize` derive と
    wire 表現 attribute。
  - `MEASURE_TIMEOUT`(`:39`)を `pub(crate)` 化(follower wait 上限の
    単一 source)。
  - `resolve_target`(`:277`)と同一 containment 規則で resolved
    target 一覧と canonical workspace root を返す `pub(crate)`
    helper `fingerprint_targets` を追加(規則の複製を避ける。
    `extract_read_candidates`(`:1120`)は既に `pub`)。
- `crates/cheaper-cli/src/embedded.rs` — `embedded_fingerprint()`
  (`:123`)を `pub(crate)` 化(fingerprint の builtin `None` marker
  に使用)。
- `<userConfigDir>/hook-eval/` 配下の verdict entry・lock file・
  sweep 規約。
- `crates/cheaper-cli/tests/integration/hook_eval_dedup.rs` 新設 +
  `tests/integration/main.rs` への `mod hook_eval_dedup;` 宣言
  (AGENTS.md の CLI integration test 配置規約)。

### Out of Scope

- `hook-config sync` の legacy-conflict gate (`hook_config.rs` の
  `KNOWN_LEGACY_FILES` + `CONFLICT_PATTERN`)の blocker → warning 降格。
  dedup 実証後の別 follow-up。
- `evaluate` の verdict semantics・deny reason・threshold 既定値・
  fail-open/fail-closed 原則・`resolve_target` の containment 規則の
  変更。
- Devin 側の import merge / firing dedup(upstream 責務)。
- `cheaper hook` process spawn 自体の削減(計測・実体化コストのみが
  削減対象)。
- 新規 crate 依存・新規 env var・設定 knob(TTL・poll 間隔・wait 上限・
  stale 閾値は module 内 const とし、test は注入 seam で差し替える)。

## Components and Boundaries

### `hook::eval_cache`(新規)

責務: 「同一評価」の識別(fingerprint)・canonical verdict の共有
(store)・firing 間協調(leader election + bounded wait)・bounded
lifetime(TTL sweep)。**render・normalize・verdict 判定には一切関与
しない** — 関与するのは「`evaluate` を呼ぶか、共有済み verdict を
返すか」の分岐のみ。

公開面:

```rust
/// `handle_hook` が呼ぶ wrapper。dedup 適用可能な request では cache
/// hit / leader 評価 / follower 共有を行い、適用外・あらゆる失敗では
/// 通常 `evaluate` へ縮退する。
pub async fn evaluate_dedup(
    request: &RoutingRequest,
    opts: &EvaluateOptions<'_>,
) -> Verdict;

/// 協調の時間定数。production 値は `DedupLimits::PRODUCTION`。
/// test は小さい値へ差し替えて wait/stale/TTL を ms スケールで検証する。
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct DedupLimits {
    /// verdict entry の有効期間(discovery 暫定指針 ~60s)。
    pub verdict_ttl: Duration,     // 60_000 ms
    /// lock file の stale 判定閾値。`follower_wait`(30s)を十分に超える
    /// 値を取る。単一 candidate の計測 timeout(`MEASURE_TIMEOUT`)は
    /// 十分カバーするが、shell の multi-candidate 評価は candidate
    /// 数 × `MEASURE_TIMEOUT` で非有界なため、長時間 eval 中の live
    /// leader が stale と誤判定され得る — 誤判定時の takeover は
    /// owner token 照合(delete-if-owner)により冗長評価のみに抑える
    /// (Lock protocol 参照)。
    pub lock_stale: Duration,      // 120_000 ms
    /// follower が共有 verdict を待つ上限。`MEASURE_TIMEOUT` と同値 —
    /// これを超えて待っても leader の評価完了は保証されないため、
    /// deadline で自前評価へ移る(追加 latency の上界はこの値)。
    pub follower_wait: Duration,   // = routing_policy::MEASURE_TIMEOUT (30_000 ms)
    /// verdict file の poll 間隔。
    pub poll_interval: Duration,   // 50 ms
}

/// 計測呼出しを差し替え可能にした内部実装(integration test seam)。
/// `eval_fn` は通常 `|| evaluate(request, opts)` を capture する
/// closure。test は計数 closure を注入して「計測が高々 1 回」を
/// 直接観測する。
#[doc(hidden)]
pub async fn evaluate_dedup_with<F, Fut>(
    request: &RoutingRequest,
    opts: &EvaluateOptions<'_>,
    limits: &DedupLimits,
    eval_fn: F,
) -> Verdict
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Verdict>;

pub async fn evaluate_dedup(
    request: &RoutingRequest,
    opts: &EvaluateOptions<'_>,
) -> Verdict {
    evaluate_dedup_with(request, opts, &DedupLimits::PRODUCTION, || {
        evaluate(request, opts)
    })
    .await
}
```

`eval_fn` を引数非依存の `Fn() -> Fut` にするのは、借用を取る
closure の HRTB 表現を避け、caller 側が request/opts を capture する
形に揃えるため(async closure の lifetime 問題を構造で消す)。

`DedupLimits` を env var ではなく注入 param にするのは、spec の
「新規設定 knob を増やさない」制約と、test が process env を共有
しない並行実行でも決定的に振る舞わせるため。

### `hook::routing_policy`(additive)

```rust
// `Verdict` — cache record 化のため serde derive を追加。
// wire 表現は internally-tagged:
//   Allow         → {"decision":"allow"}
//   Deny{reason}  → {"decision":"deny","reason":"…"}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum Verdict { Allow, Deny { reason: String } }
```

- `pub(crate) const MEASURE_TIMEOUT: Duration` — follower wait の上限を
  計測 worst-case と同一 source に揃える(DRY)。
- fingerprint 用 helper:

```rust
/// `resolve_target` と同一の containment + metadata 規則で、request が
/// verdict を左右し得る filesystem target を列挙する。
/// `NonFile` は resolved 先が file/dir でない(missing・その他・
/// workspace 外)候補を指し、fingerprint 上は "absent" marker として
/// 符号化される — 「評価時点では存在しなかった」は verdict を変え得る
/// ため signature に含める必要がある(Shell の multi-candidate で、
/// absent → file の出現が別 fingerprint になることを保証)。
pub(crate) enum ResolvedKind { File, Dir, NonFile }
pub(crate) struct ResolvedTarget {
    // File/Dir: canonicalize 済みの絶対 path(cwd spelling 差を除去、
    // PathBuf — hash へ as_encoded_bytes を渡す)。
    // NonFile: candidate の raw 文字列。
    pub resolved: PathBuf,
    // resolve_target が計算した workspace-relative posix path —
    // deny reason への埋め込みで verdict を変え得る入力。
    pub rel_posix: String,
    pub kind: ResolvedKind,
}
/// fingerprint の workspace context。`workspace_root` は
/// `canonicalize_workspace_root(resolve_request_cwd の絶対解決側)`
/// の結果(失敗時 `None`)で、同一 workspace の異なる cwd spelling
/// (`./x`・symlink 経由・語尾 `/` 差等)を同一 fingerprint へ収束
/// させる。
pub(crate) struct FingerprintContext {
    pub workspace_root: Option<String>,
    pub targets: Vec<ResolvedTarget>,
}
pub(crate) fn fingerprint_targets(request: &RoutingRequest) -> FingerprintContext;
```

`fingerprint_targets` の規則:

| variant | 返す targets |
| --- | --- |
| `FileRead { path, cwd, .. }` | `resolve_target(path, cwd)` が `Resolution::File` → `[File]`。`Dir`/`WorkspaceRoot`/`OutOfScope` → `[]`(常時 allow の定数経路で dedup 対象外) |
| `Search { path, cwd, .. }` | `File` → `[File]`、`Dir`/`WorkspaceRoot` → `[Dir]`、`OutOfScope` → `[]`。空 `pattern` は `evaluate` の定数 allow 経路に揃えて target を列挙しない(`[]` → bypass) |
| `Shell { command, cwd }` | `extract_read_candidates(command)` の各候補を `resolve_target` し、`File` → `File`、`Dir`/`WorkspaceRoot` → `Dir`、それ以外 → `NonFile`(raw candidate 文字列を `resolved` に保持)。candidates が空 → `[]` |

### `hook::targets`

5 arm の `evaluate` 呼出しを `eval_cache::evaluate_dedup` へ差し替える
のみ。戻り値は同じ `Verdict` で、`verdict_name`・各 `*_render`・
`cheaper.hook` 完了 `debug!` event は無変更。dedup 結果
(hit/leader/follower/fallback/bypass)の分類は `eval_cache` 内部で
`tracing::debug!`(metadata-only 規約、`verdict` field と同じ分類値
スタイル)として emit し、`HookResponse` の shape には触れない。

### store 配置

```
<userConfigDir>/hook-eval/
  <fingerprint>.json                    # verdict record (atomic temp+rename)
  <fingerprint>.lock                    # leader election marker
  <fingerprint>.lock.claim-<pid>-<nonce> # stale takeover の rename-claim 残骸(orphan sweep 対象)
  .<fingerprint>.json.tmp-<pid>-<nanos> # atomic_write の temp (orphan sweep 対象)
```

- `<userConfigDir>` は `resolve_user_config_dir(UserConfigDirOptions {
  env: Some(opts.env), homedir: None })`(`cheaper-core/config/dirs.rs:37`)
  で解決し、`hook-eval` を join する。`usage_log.rs` と同じ境界 —
  workspace・`.cheaper/` には一切書かない。
- 解決した store dir が**絶対 path でない**場合(相対
  `CHEAPER_CONFIG_DIR` 等の env 由来)は cwd 依存の解釈になり file
  名衝突・意図しない書き込み先を招くため、協調を bypass して
  `eval_fn` 直接呼出しへ畳む(fail-open — amendment 参照)。
- `CHEAPER_CONFIG_DIR`/`XDG_CONFIG_HOME`/`USERPROFILE`/`HOME` はこの
  root を分岐するため、firing 間でこれらが異なれば構造的に別 store
  へ分離される — fingerprint への追加は不要(ADR の決定どおり)。

## Fingerprint

hash は `DefaultHasher`(`embedded.rs:123` `embedded_fingerprint()` と
同じ `format!("{:016x}", hasher.finish())` 形)を使い、結果の 16 進
16 文字を file 名とする。`RoutingRequest` への `derive(Hash)` は
**使わない** — `source_tool` が混入して firing 間で key が割れるため、
hash 入力は `eval_cache` 内の専用 struct に field 単位で明示列挙する。

### 含めるもの(canonical field 列挙)

1. format version tag — `"hook-eval-v1"`(fingerprint 構成を将来変えた
   場合の混同防止)。
2. request 由来(変更不能な policy 入力):

   | variant | hash 入力 field |
   | --- | --- |
   | 共通 | kind discriminant、`workspace_root`(`FingerprintContext.workspace_root` — canonicalize 済み実効 cwd。`None` は `unresolved` marker) |
   | `FileRead` | `range`(無し/`Some(offset, limit)`) |
   | `Search` | `pattern`、`max_results: Option<u64>`、`context: u64`、`glob: Option<String>`、`output_mode`(`"content"`/`"files_with_matches"`/`"count"`/`"unknown"` の discriminant 文字列)、`case_insensitive` |
   | `Shell` | `command`(**raw 文字列**) |

   raw `path`・`cwd` は hash **しない** — path の同一性は stat
   signature の canonicalize 済み resolved path で、cwd の同一性は
   `workspace_root` でカバーし、spelling 差(`./x`・symlink 経由・
   語尾 `/` 差)による fingerprint の分裂を防ぐ。`Shell` で raw
   `command` だけを残すのは任意性の排除ではなく correctness:
   deny reason が `"cheaper: blocked shell command '{collapse_command(command)}'."`
   を埋め込むため、candidate 集合が同じでも command 文字列が異なれば
   verdict(reason 文字列)が異なる。
3. stat signature — `FingerprintContext.targets` の各 target について、
   列挙順のまま `(resolved, rel_posix, kind, mtime, len)`:
   - `File` → `(resolved, rel_posix, file, mtime_nanos, len)`
   - `Dir` → `(resolved, rel_posix, dir, mtime_nanos)`(dir の `len`
     は platform 依存で意味を持たないため除外)
   - `NonFile` → `(raw candidate, absent)` marker
   `resolved` は File / Dir について canonicalize 済みの値を使い、
   `OsStr::as_encoded_bytes` の生 bytes で hash する(`PathBuf` のまま
   保持 — lossy 変換で非 UTF-8 path を潰さない)。
   `rel_posix` は `resolve_target` が計算した workspace-relative
   posix path(`ResolvedTarget.rel_posix`)— deny reason が
   `workspace-relative path: {rel_posix}` を埋め込むため verdict を
   変え得る入力であり同一性に含める(NonFile は `""`、
   `WorkspaceRoot` 由来の Dir は `evaluate_search` と同じ `"."`)。
   mtime は `metadata.modified()` の `duration_since(UNIX_EPOCH).
   as_nanos()`。metadata 取得に失敗した target は `absent` marker へ
   畳む(fs race は fingerprint 差異として自然に再評価へ倒れる)。
4. policy / env 入力(解決済み実効値):
   - `threshold(opts.env)`(`:130`)の `f64::to_bits()`
   - `count_max_bytes(opts.env)`(`count.rs:28`)の実効値 —
     `unwrap_or(DEFAULT_COUNT_MAX_BYTES)` 後の `u64`
   - `builtin_dir` 識別 — `Some(dir)` は `<dir>/policy.yaml` の
     `(mtime_nanos, len)` または `absent`(materialize 済み builtin
     policy も file として変わり得るため stat signature 化)。
     `None` は embedded policy を意味し、内容は binary 固定なので
     `embedded_fingerprint()`(`embedded.rs:123` — `pub(crate)` 化
     する)の値を marker として hash する。
   - user policy signature — `<userConfigDir>/policy.yaml` の
     `(mtime_nanos, len)` または `absent`
   - project policy signature — `<workspace_root>/.cheaper/
     policy.yaml` の `(mtime_nanos, len)` または `absent`
     (`FingerprintContext.workspace_root` が `None` の場合は
     `unresolved` marker)

   policy 層 signature(builtin `Some`・user・project)は spec R2 の
   「policy / env 入力」に対応する:
   `count_workspace_file_tokens`(`count.rs:53`)が `load_policy` 経由で
   builtin/user/project `policy.yaml`(`policy/load.rs:109`,`:229` —
   `filesystem.symlinks` 設定)を読むため、同一 fingerprint 内で
   policy 内容が変わると verdict が変わり得る。stat signature 化により
   「編集 = 別 fingerprint = 再評価」を保証する(ADR の field 列挙への
   design レベル精緻化 — builtin dir 識別に加え、残る policy 層の
   file 状態も key に含める)。

### 除外するもの(明示)

- `target`(`HookTarget`)— verdict を変えない。render のみに使われる。
- `source_tool` — provenance であり policy が読まない(前述)。
- session id・pid・transcript path・`agent_action_name`・
  `hook_event_name`・`tool_name` 等、`RoutingRequest` へ写像されない
  payload key — normalize で捨てられた情報は verdict の入力になり得ない。
- `CHEAPER_CONFIG_DIR`/`XDG_CONFIG_HOME`/`HOME`/`USERPROFILE` — store
  root の分岐として構造上分離済み。
- process cwd — `resolve_request_cwd`(`:243`)の `request.cwd ||
  process.cwd()` 規則で `workspace_root`・resolved path・stat
  signature に畳み込まれる。
- hook event 名 — normalize で消費済み。同一 canonical request に
  収束する event 同士は同一評価として正しく dedup される。

`ctx.targets` が**完全に空**の request — FileRead / Search が
`OutOfScope`(missing・workspace 外・非 file/dir)に解決した場合、
および Shell の `extract_read_candidates` が空(parse 不能・read
command 無し)の場合 — は **cache を介さず** `eval_fn` を直接呼ぶ
(verdict が定数 allow の経路で、共有する意味も signature も無い)。
`NonFile` は absent marker として signature に載るため、全候補が
`NonFile` の Shell request は空ではなく cache 対象である(後述の
optional fast-path gate を実装した場合はそちらで直接評価へ畳める)。

## Verdict record schema

`<fingerprint>.json` の内容:

```json
{
  "schemaVersion": 1,
  "createdAt": 1730000000000,
  "fingerprint": "0123456789abcdef",
  "verdict": { "decision": "deny", "reason": "cheaper: 'big.txt' has …" }
}
```

- `schemaVersion: 1` 固定。未知 version・非 object・parse 失敗・
  `verdict` の deserialize 失敗は**存在しないもの**として扱い、
  serve しない(R5)。`usage.jsonl` と同じ forward-compat 規約。
- `fingerprint` は record が属する fingerprint の 16 進文字列。
  read 時に期待 fingerprint と照合し、不一致は absent 扱い
  (file 名と内容の不整合・fingerprint 構成変更後の残骸を構造的に
  排除する)。
- `verdict` は `Verdict` の serde 表現そのもの(`{"decision":"allow"}` /
  `{"decision":"deny","reason":…}`)。
- `createdAt` は leader が記録する epoch ms で、観察用 field。
  **freshness の判定には使わない** — TTL は file mtime(atomic rename
  時に fs が付与し、leader が値を誤記録できない)で判定する。
  判定は checked-age 規則 — `now.checked_duration_since(mtime)` が
  `Some(age) <= verdict_ttl` の record のみ serve し、`None`
  (未来 mtime — clock skew・fs timestamp 異常)は TTL 超過と同じ
  stale 扱いで serve しない(**record の未来 mtime semantics は
  不変**)。lock の stale 判定は別規則(amendment — 未来 mtime を
  一律 stale としない): 過去 `age > lock_stale` または未来 mtime が
  `now + lock_stale` を超えるもののみ takeover/sweep 対象とし、
  skew 範囲内(`now + lock_stale` 以下)の未来 mtime・mtime 取得
  不能の lock は live 扱い(Fresh — 待機後 self-eval)。同一の
  horizon 規則(`past_stale_boundary`)を sweep にも適用する。
  なお verdict が反映するのは評価開始時点の入力であり
  record の mtime は評価完了時に刻まれるため、serve される
  verdict の staleness 上界は「評価時間 + TTL」となる。
- 書き込みは `cheaper_core::fs_util::atomic_write`(`fs_util.rs:34`、
  temp+rename、`windows_remove_first: false`)。`unix_mode` は struct
  literal で `#[cfg(unix)] unix_mode: Some(0o600)` と指定する
  (`auth/store.rs`・`policy.rs` と同じ sibling 慣行 — field 指定に
  `#[cfg(unix)]` が付くため Windows build でもそのまま compile
  する)。parent dir の `create_dir_all` は caller(eval_cache)の
  責務。

## Lock protocol / 協調フロー

lock は `<fingerprint>.lock` を `OpenOptions::new().write(true).
create_new(true)` で取得する(O_EXCL 相当の atomic create)。file
の中身は `{"pid":…,"createdAt":…,"nonce":…}` の診断 JSON で、
`(pid, createdAt, nonce)` の triple が lock owner を一意に識別する
**owner token** となる。権威は file の存在と mtime + token 照合 —
pid 生存確認のような新規依存・platform 分岐は持ち込まない。
lock の削除は 2 経路に分かれる(amendment):

- **leader の release — delete-if-owner**: remove 直前に payload を
  読み直し、自分の token と一致する場合に限り削除する。stale 判定
  → takeover で後継 leader が lock を再取得した後に旧 leader が
  後継の lock を削除する経路を防ぐ。`read`→`remove` の sub-µs gap
  に「自分の lock が claim で取り去られ後継 lock が再設置」される
  と後継 lock を消し得る残存窓は、害が冗長評価のみで観測困難な
  ため**受容済み**とする。
- **follower の stale takeover — rename-claim**: 観測した owner
  token を持つ stale lock を `<fp>.lock` → `<fp>.lock.claim-
  <pid>-<nonce>` へ `rename` する。rename は atomic で、成功した
  1 者のみが claim file の token を観測値と照合してから削除する
  — 削除対象が「観測した stale file そのもの」と証明でき、
  `remove_if_owner` を takeover に直接適用した場合の TOCTOU
  (read→remove 間に旧 release + 新 acquire が挟まる)を閉じる。
  token 不一致・非 file の claim(窓で新 lock / 非 file に
  差し替わった残存 race)は file を殺さず `<fp>.lock` へ戻す
  (best-effort — 再占有済みなら上書きせず orphan とし、sweep が
  `claim-*` 規則で回収)。rename 敗者・unparseable payload(token
  未観測)の lock は `create_new` retry へ畳み AlreadyExists で
  Wait(mtime sweep のみで期限切れ除去)。

- **leader**(lock 取得者): owner token を書き込んで lock を作成 →
  `eval_fn(request, opts).await` → verdict record を
  `atomic_write` → lock を読み直して token が自分のものなら
  `remove_file`(delete-if-owner)。**verdict の rename 完了が lock
  削除に先行する**ため、follower が観測するのは「lock 在り・
  verdict 無し」「lock 在り・verdict 在り」「lock 無し・verdict
  在り」のみで、「lock 無し・verdict 未 commit」の窓は無い。
  atomic_write の temp file は別名(`.<fp>.json.tmp-*`)のため、
  follower が部分書き込み record を読む経路も無い。
- **follower**(lock が既存): `poll_interval` 毎に、1 poll tick 分の
  fs 操作を `spawn_blocking` へ逃がして次を確認する:
  1. `<fp>.json` を読み fresh+valid → その verdict を返す(serve)。
  2. lock が存在しない → `create_new` を retry。成功なら自らが
     leader となり評価・publish・release する。失敗なら次 tick へ。
  3. lock が stale(過去 `age > lock_stale`、または未来 mtime が
     `now + lock_stale` 超過 — leader crash 等の死骸)→
     rename-claim(上記)で stale file を atomic に除去 →
     `create_new` retry(同上)。
  4. lock が生きている(fresh mtime、または skew 範囲内の未来
     mtime)→ `tokio::time::sleep(poll_interval)` して次 tick。
  5. lock が file でない(FIFO・dir 等)— `create_new` は永久に
     AlreadyExists で payload read は blocking し得るため協調不能
     → 即時自前評価(fallback)。
  `follower_wait` deadline 到達 → **自前評価**(`eval_fn` を直接呼ぶ。
  cache write は行わない — publish 経路を leader 単独に保ち、fallback
  側の協調状態を増やさない)。
- **stale takeover**: 死骸 lock の rename-claim 除去 + `create_new`
  retry により、leader crash 時も待機中の follower の 1 つが
  leader へ昇格し評価を完遂できる。rename に成功するのは常に 1 者
  のみで、競合しても `create_new` が 1 者のみ通すため、最悪でも
  「順次 2 者が同一 verdict を計算」= 冗長コストのみで、出力が
  分岐することはない。
- **live leader の誤判定**: `lock_stale`(120s)は単一 candidate の
  計測 timeout(`MEASURE_TIMEOUT` 30s)を十分上回るが、shell の
  multi-candidate 評価は candidate 数 × `MEASURE_TIMEOUT` で非有界
  なため、長時間 eval 中の live leader が stale と誤判定され得る。
  その場合 takeover が起きても害は冗長評価のみに留まる — 旧
  leader の release remove は token 照合(delete-if-owner)で後継
  lock を消せず、verdict record は同一 fingerprint・同一 verdict
  で上書きされるだけである。
- **sweep**: dedup 経路の先頭(blocking phase 内)で `hook-eval/` を
  best-effort sweep する — `past_stale_boundary(verdict_ttl)` を
  越えた `*.json`、`past_stale_boundary(lock_stale)` を越えた
  `*.lock`・`*.lock.claim-*` orphan、TTL 超過の `.tmp-*` orphan を
  remove(未来 mtime は horizon 超過のみ — skew 範囲内の live lock
  を誤削除しない)。無制限蓄積を防ぐ起点で、失敗は無視。

## Data and Control Flow

`evaluate_dedup` 1 回の呼出し:

1. env snapshot(async 側): `opts.env`(`EnvLookup` = `&dyn Fn` で
   非 Send、`dirs.rs:16`)から fingerprint が必要とする env key を読んで
   owned map に写す — `MEASURE_ENV_KEYS`(`:176`)∪
   `CHEAPER_READ_TOKEN_THRESHOLD`(`threshold` の入力)。
   `measure_tokens`(`:215`)と同じ snapshot pattern を再利用し、
   blocking phase へ env closure 自体は渡さない。store root 解決後、
   相対 store dir は bypass(fail-open)。
2. `spawn_blocking` phase A(`DEDUP_IO_BOUND` で囲む): request を
   clone して closure へ渡し、`fingerprint_targets`(sync —
   `resolve_target` 相当の metadata・canonicalize 呼出しを含む)→
   `ctx.targets` が空 → `eval_fn` 直接呼出し(bypass、定数 allow
   経路)。続けて sweep → env snapshot から policy / env 入力
   (`threshold`・`count_max_bytes`)を確定 → 各 target の stat
   signature 収集(metadata 失敗は `absent` marker へ畳む)→
   builtin / user / project `policy.yaml` signature → fingerprint
   確定。fs error・cache dir `create_dir_all` 失敗・timeout →
   `eval_fn` 直接呼出し。
3. `spawn_blocking` phase B(各 tick `DEDUP_IO_BOUND` で囲む):
   `<fp>.json` read(schemaVersion・`fingerprint` field 照合・
   deserialize・file mtime checked-age 検証)→ fresh+valid なら
   **serve**(eval_fn 不呼出し)。さもなくば `create_new` lock 試行
   (owner token 書込み)。
4. 分岐:
   - **serve**: verdict を返す。
   - **leader**: `eval_fn` await → blocking で record write +
     delete-if-owner で lock release(publish も `DEDUP_IO_BOUND`
     で囲み、timeout 時は計算済み verdict をそのまま返す)→
     verdict を返す。
   - **follower**: bounded poll(上記)→ serve / takeover / deadline
     → 最終的に verdict を返す。
5. 返った `Verdict` は `handle_hook` の既存経路(`verdict_name` +
   per-target render)へそのまま流す。

**phase A/B を `spawn_blocking` に載せる**のは `measure_tokens`
(`:201`)と同じ構造 — hook の tokio runtime を fs 待ちで占有しない。
`tokio::time::sleep` 以外の blocking `std::thread::sleep` は使わない。
全ての `spawn_blocking` await は `DEDUP_IO_BOUND`(=
`MEASURE_TIMEOUT`、単一 source)の `tokio::time::timeout` で囲む
(amendment)— hang した fs(FIFO・dead network FS)でも prepare・
poll tick は直接評価へ、publish は共有を諦めて計算済み verdict を
返す。timeout で切り離された blocking thread は cancel できず継続
するが観測しない(`measure_tokens` と同じ受容済み妥協)。

**optional fast-path gate**(実装判断): `ctx.targets` が全て `File`
かつ各 file の `len <= threshold`(byte fast path —
`evaluate_file_read`(`:329`)と同じく tokens <= bytes の不変条件で
計測を要さず allow が確定する場合)、および全て `NonFile`(評価
対象の実 file が存在しない場合)は、cache 協調を skip して
`eval_fn` を直接呼んでよい。これらは eval が自明に安く、lock・
poll の overhead が dedup 効果を上回るため。`NonFile` の absent
marker 自体は mixed candidate 集合の fingerprint 同一性
(absent → file 出現で別 fingerprint)に引き続き必要であり、
fast-path は共有を諦めるだけで verdict の correctness に影響
しない。

## Concurrency correctness の要点

- **verdict reuse の厳密性**: fingerprint が `evaluate` の全観測
  可能入力を被覆する(上記 field 列挙)限り、同一 fingerprint の
  2 firing は同一 verdict を持つ — cache reuse は近似でなく恒等。
  stat signature が内容 hash でなく mtime+len である残差リスクは
  評価時間 + TTL(60s)で bound する(record の mtime は評価完了時
  に刻まれるが、verdict が反映するのは評価開始時点の入力)。
- **per-target render 不変**: cache するのは canonical `Verdict` のみ。
  render 済み `HookResponse` を共有しないため、windsurf(stderr+exit 2)・
  copilot(deny JSON+exit 0)・pretooluse(`hookSpecificOutput`)・
  devin(`decision:block`)・cursor(permission JSON)の各 byte 互換
  contract は dedup 経路でも不変。
- **publish 順序**: verdict write(temp+rename)→ lock remove の順序が
  「lock 解放済みだが verdict 未到着」の窓を消す。
- **deny を弱めない**: serve するのは同一 fingerprint で leader が実際に
  計算した verdict のみ。stale/corrupt/別 key の record は常に
  「無い」扱いで再評価へ倒れ、allow 方向への近似は設計に存在しない。

## Failure model

| 失敗 | 挙動 |
| --- | --- |
| `resolve_user_config_dir` | PathBuf を返すのみで失敗しない(相対 store dir は bypass — amendment) |
| `create_dir_all` / sweep の fs error | 無視 → `eval_fn` 直接呼出しへ畳む |
| verdict read: 不存在・parse 失敗・未知 schemaVersion・`fingerprint` 不一致・TTL 超過(未来 mtime 含む) | 「absent」として扱い serve しない |
| lock `create_new` の非 AlreadyExists error | `eval_fn` 直接呼出し |
| lock が非 file(FIFO・dir 等) | 協調不能 → `eval_fn` 直接呼出し(payload read・削除をしない) |
| `spawn_blocking` await の `DEDUP_IO_BOUND` timeout / JoinError | prepare・poll tick → `eval_fn` 直接呼出し、publish → 計算済み verdict をそのまま返す |
| poll 中の fs error | 「verdict 未着」として継続、deadline で self-eval |
| leader の `atomic_write` 失敗 | lock を delete-if-owner で remove して verdict を返す(共有できないだけ) |
| leader crash(verdict 未 write / lock 残存) | follower は poll → `lock_stale` で rename-claim takeover、または `follower_wait` で self-eval |
| eval_cache 内 panic | `execute_hook` の `catch_run_panic` が `fallback_allow`(exit 0)へ畳む — ただし設計上 `unwrap`/`expect`/`panic!` を置かず、到達させない |
| copilot target の全 cache/lock 失敗 | どの経路も `Verdict` を返して `copilot_render` へ進むため、新たな非 0 exit を生じない(never-nonzero 契約維持) |

## Non-Functional Constraints

- 新規依存なし — `std::fs::OpenOptions`・`DefaultHasher`・既存の
  `serde`/`serde_json`/`tokio`/`cheaper_core::{fs_util, config::dirs}`
  のみ。
- 新規 env var なし。`CHEAPER_CONFIG_DIR` 等の既存解決規則を
  `resolve_user_config_dir` 経由でそのまま再利用(複製しない)。
- 書き込み先は `<userConfigDir>/hook-eval/` のみ — workspace・
  `.cheaper/`・stdout/stderr 契約に影響しない(R8)。書き込みは
  `record_usage` と同じ best-effort 契約。
- bounded: verdict TTL 60s・lock stale 120s・follower wait ≤ 30s・
  poll 50ms・sweep は自 dir のみ。
- `follower_wait` を個別計測呼出しの timeout(`MEASURE_TIMEOUT` 30s)
  に揃え、dedup 導入による最大追加 latency は `follower_wait`
  分(30s)。合計の最悪は `follower_wait` + 評価 worst-case(単一
  candidate で ~60s、shell の multi-candidate は candidate 数 ×
  `MEASURE_TIMEOUT` で非有界)。

## Risks and Trade-offs

- **mtime granularity / dir shallow signature**: 同一 fingerprint 内で
  file が内容のみ変わる(mtime granularity 内の fast successive write)
  ケース、および dir target の nested file 変更(dir の mtime は直下
  entry の変化でのみ進む)は取りこぼし得る。取りこぼしは古い verdict
  の serve に繋がるため、TTL で staleness を bound する設計上の妥協
  (ADR の consequence どおり)。取りこぼしを安全側へ倒す追加手段
  (内容 hash・dir 再帰 stat)はコストが dedup の目的を損なうため
  採用しない。
- **transient-failure allow の共有**: leader の計測が timeout/IO error で
  `Measured::Failed` → `Allow` になった場合、その allow が TTL 内の
  兄弟 firing にも共有される。「Allow のみ cache しない」案は、
  threshold 内の正常 allow も共有できなくなり dedup 効果を大きく
  削るため採用しない — leader 自身が返した verdict の共有であり、
  追加の弱体化ではない。
- **follower の追加 latency**: dedup 経由で追加される latency は
  最悪 `follower_wait`(30s)。deadline で自前評価へ移るため、合計の
  最悪は `follower_wait` + 評価 worst-case — 単一 candidate で
  ~60s(計測 timeout 30s + 後段 scan/tokenize)、shell の
  multi-candidate は candidate 数 × `MEASURE_TIMEOUT` で非有界。
  評価自体の worst-case は dedup 無しでも同じで、追加されるのは
  `follower_wait` 分のみ。
- **lock 協調の新規失敗 mode**: stale lock・corrupt entry・takeover
  race 等の test 対象が増える — `DedupLimits` 注入で決定的に検証する
  構成を Verification に組み込む。
- **hash 衝突**: `DefaultHasher` は adversarial 衝突耐性を持たないが、
  fingerprint は 64bit・user-trusted 境界(`<userConfigDir>`)内の
  cache であり、hook の guard は cost/routing 制御(security
  boundary ではない)のため許容(ADR More Information と同じ評価)。
  toolchain 差で hash が変わっても効果は hit rate のみ。

## Deferred Design Concerns

- legacy-conflict gate の blocker → warning 降格 | dedup で重複発火が
  無害化された後に意味を持つ | 本機構の実装・実証後 | 降格しない
  限り native + import 登録の共存は sync で拒否され続ける(現状維持は
  安全側)。
- TTL・`follower_wait`・`lock_stale` の実測ベース tuning | importer
  firing の時間分散実測が根拠 | live 計測が取れた時点 | 現行値は
  discovery 暫定指針と `MEASURE_TIMEOUT` 由来の保守的 bound。
- Devin 側 import dedup | 発火重複は host の責務 | Devin CLI 側に
  dedup が入った場合 | 本機構はその場合も無害に動作する。
- dir target の深い signature(nested mtime 集約等)| nested 変更の
  staleness を TTL より短くしたい場合 | 実害報告があった時点 |
  stat 走査コストが増え dedup の効率目標と競合する。
- `cheaper hook` spawn 自体の N 回 overhead | dedup の効果は
  計測・実体化に限定 | spawn が測定上の問題になった時点 | 登録数分の
  process 起動コストは残る。

## Amendments(implementation loopback — post-review)

実装 review で確定した修正を本文へ loopback 済み(design status は
approved のまま — 責務・境界・fail-open 規約・store 配置の変更は
なく、協調プロトコルの堅牢化と fingerprint 入力の精緻化のみ):

- **blocking I/O bound**: 全 `spawn_blocking` await(prepare・各
  poll tick・publish)を `DEDUP_IO_BOUND`(= `MEASURE_TIMEOUT`)の
  `tokio::time::timeout` で囲む。fingerprint target 列挙(sync
  canonicalize 含む)も request clone で同一 blocking phase へ移し、
  hang した fs でも bounded time で verdict resolve。
- **stale takeover の atomic claim**: follower の stale lock 除去を
  `remove_if_owner` 直接適用から rename-claim
  (`<fp>.lock` → `<fp>.lock.claim-<pid>-<nonce>` + token 照合)へ
  変更し、read→remove 間の TOCTOU を閉じる。leader release は
  delete-if-owner のまま(sub-µs 残存窓は受容済みとして明記)。
- **future-mtime semantics(record vs lock)**: record は不変(serve
  しない)。lock は skew 範囲内(`now + lock_stale` 以下)の未来
  mtime・mtime 取得不能を Fresh(削除不可)とし、`now + lock_stale`
  超過の未来 mtime または過去 `age > lock_stale` のみ takeover・
  sweep 対象(`past_stale_boundary` — sweep にも同一規則を適用)。
- **非 file lock**: dir・FIFO 等は Unrecoverable — payload read・
  削除を行わず即時 self-eval。
- **fingerprint 入力**: `resolved` を `OsStr::as_encoded_bytes` で
  hash(PathBuf 保持、lossy 回避)し、`rel_posix`(deny reason が
  埋め込む workspace-relative path)を stat signature に追加。
- **relative store dir guard**: 解決した store dir が絶対でなければ
  dedup を bypass(fail-open)。
- **その他**: `acquire_lock` の write_all 失敗時に自前 remove、
  `fingerprint_targets` の空 `pattern` Search を `evaluate` と同じ
  定数 allow 経路へ揃える(`[]` → bypass)、`.lock.claim-*` orphan
  を sweep 対象に追加。

## Verification Notes

test の配置は `crates/cheaper-cli/tests/integration/hook_eval_dedup.rs`
+ `main.rs` の `mod hook_eval_dedup;`。concurrency は**実 process
境界を跨がなくても全協調が file 経由**なため、2 層で検証する:

- **module 層(並行協調の決定的検証)**: `evaluate_dedup_with` に
  `AtomicUsize` で計数する fake `eval_fn`(200ms sleep で重なりを
  確定)と小さい `DedupLimits` を注入し、同一 request・同一
  `CHEAPER_CONFIG_DIR` で `tokio::join!` — 計数 = 1・両 verdict が
  同一内容であること。3 発目は純粋 cache hit(計数不変)。
- **binary 層(end-to-end)**: `env!("CARGO_BIN_EXE_cheaper")` の
  `std::process::Command` で 2 つの `cheaper hook` process を同一
  stdin payload・同一 `CHEAPER_CONFIG_DIR`・同一 cwd で同時 spawn
  (`hook_config.rs` test の PATH shim / spawn パターン踏襲)—
  stdout・exit code が一致し、`<fp>.json` が 1 件・`.lock` が残存
  しないこと。

ケース(spec acceptance criteria の写像):

- [ ] 同一 canonical request の並行 firing で eval 計数 = 1、全
  firing が同一 verdict 内容(module 層 + binary 層)。
- [ ] cache 経由 verdict の per-target render が既存出力と byte 同一
  (windsurf stderr+exit 2 / copilot deny JSON+exit 0 / pretooluse
  `hookSpecificOutput` / devin `decision:"block"` / cursor
  permission JSON — `hook.rs` の既存 fixture を再利用)。
- [ ] file stat 変化(size または mtime)・`CHEAPER_READ_TOKEN_THRESHOLD`
  変化で別 fingerprint → 再評価(計数増・別 `<fp>.json`)。
- [ ] leader crash 模擬: `<fp>.lock` を手配置して mtime を stale 閾値
  より古くする(`std::fs::File::set_modified` — 新規 dep 不要)と
  follower が takeover して評価完遂。fresh lock + verdict 無し
  (crash mid-eval)では follower が bounded wait 後に self-eval
  (計数 = 2)。**fresh** mtime で植えた lock が poll 中に `lock_stale`
  を跨ぐ経路(小さい `DedupLimits`)では mid-poll の Fresh→Stale
  遷移から rename-claim → takeover が走ること。
- [ ] live leader の takeover safety: `lock_stale` を超える長時間
  eval の live leader がいる状態で follower が stale 判定 →
  takeover しても、旧 leader が delete-if-owner の token 照合で
  後継 lock を削除できないこと(module 層で lock payload 操作を
  模擬)。
- [ ] 未来 mtime(`now.checked_duration_since(mtime)` → `None`)の
  record が serve されず sweep 対象になること。lock は skew 範囲内
  (`now + lock_stale` 以下)の未来 mtime が live 扱い(削除されず
  wait→self-eval)、`now + lock_stale` 超過のみ takeover・sweep
  対象になること(amendment — `past_stale_boundary`)。
- [ ] 異なる cwd spelling(`./sub`・`sub`・symlink 経由等)で書かれた
  request が canonical 化(`workspace_root` + resolved path)により
  同一 fingerprint になること。
- [ ] corrupt `<fp>.json`(非 JSON / 未知 schemaVersion /
  `fingerprint` field 不一致 / TTL 超過の backdated mtime)が
  serve されず通常評価へ fallback。
- [ ] dir target の nested file 編集が dir の stat signature に
  現れないため TTL 内では同一 verdict が serve される — staleness
  bound が TTL(と評価時間)のみであることを test または明文化で
  固定する。
- [ ] cache dir を書き込み不能(`CHEAPER_CONFIG_DIR` を file path へ)
  にした状態でも copilot target が常に exit 0 + schema-valid JSON。
- [ ] sweep: TTL 超過 `.json`・stale `.lock`・orphan `.tmp-*` が次回
  起動で除去される。
- [ ] workspace 内に `hook-eval/` が生成されない(R8 — `<ws>` 配下に
  新規 file/dir が無いことを assert)。
- [ ] `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
  hook_eval_dedup::` と `mise run check` が pass。
