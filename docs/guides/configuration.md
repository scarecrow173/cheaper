# 設定ガイド

`cheaper` の設定は 2 系統に分かれる。

- **provider / auth**: model catalog（bundled snapshot + plugin +
  user overlay の `models.json`）と credential
  （`credentials.json`）。詳細は [providers ガイド](./providers.md)。
- **cheaper 自身**: `models.yaml`（tier 割当）と `policy.yaml`
  （実行 policy）。本 guide で扱う。

## config dir の解決

| 層 | 場所 | 優先度 |
| --- | --- | --- |
| project | `<cwd>/.cheaper/`（`--cwd` で変更可） | 最優先 |
| user | 下記の解決順の最初に見つかった dir | project 未指定項目を補完 |
| built-in | CLI パッケージ同梱（`policy.yaml` の完全値） | 既定値 |

user config dir は以下の順で解決する:

1. `$CHEAPER_CONFIG_DIR`（設定されていれば最優先）
2. `$XDG_CONFIG_HOME/cheaper`
3. `~/.config/cheaper`（`$USERPROFILE` → `$HOME` → OS home の順）

`CHEAPER_CONFIG_DIR` は「普段の設定と検証設定を分けたい」場合の
canonical な隔離 seam。

## 優先度

読み込み順は built-in → user → project。後から読んだ層が前の層を
deep merge する（object は merge、array は replace）。project が
user より優先。

環境依存設定はリポジトリへコミットしない。

## `models.yaml`

provider `<provider>/<modelId>` → tier（`low`/`medium`/`high`）の
割当、tier ごとの model preference、resolution 対象からの除外を
定義する。

```yaml
version: 1
overrides:
  "ollama/qwen3.5:9b": low       # この model は low tier に pin
exclude:
  - "anthropic/claude-opus-4-5"  # 解決候補・--model 指定の双方から除外
  - "github-copilot/*"           # provider 単位の wildcard も可
preferences:
  low:
    - "ollama/*"                  # low tier でこの provider 系を優先
    - "openai-codex/gpt-5.5"
```

- `version` は必須（現在 `1`）。
- `overrides` は `"<provider>/<modelId>"` → tier の map。built-in
  の分類より常に優先される（分類 source は `explicit-override`）。
- `exclude` は `"<provider>/<modelId>"` または `"<provider>/*"` の
  配列。一致した model は tier 解決の候補から外れ、`--model` の
  直接指定でも `POLICY_VIOLATION` になる。user / project 層の
  `exclude` は union される（project 層が user の除外を消さない）。
- `preferences` は tier → model pattern の配列（`provider/*` の
  wildcard 可）。解決時にその tier の候補を優先する。

tier 分類の priority は `overrides` → bundled curated exceptions →
auto 分類 → unclassified（effective tier は `medium`）の順。
pricing は runtime catalog（bundled snapshot は models.dev
由来）を canonical とし、欠損分は bundled pricing snapshot で
補完される。

auto 分類は利用可能な（exclude 適用後の）priced model 集合の
**相対 ranking** で行う。worker の workload profile
`outputPerInput`（平均 output token / input token、既定 `1.0`）
から各 model の `effectiveCost = input + output * outputPerInput`
（USD/1M token）を計算し、effectiveCost の percentile が
下位 1/3 → `low`、中位 1/3 → `medium`、上位 1/3 → `high`
となる。同額の model は同一 percentile・同一 tier にまとめられ、
priced model が 1 つだけの場合は `medium` になる。pricing 欠落・
合計 0 の model は ranking 母集団に入らず unclassified →
`medium` とする。`ollama/*` のような local model は curated
exception で `low` に固定済み。各 model の effective tier と
分類・pricing の由来（`tierSource`・`effectiveCost`・
`effectiveCostPercentile`）は `cheaper models` で確認できる
（`--worker <name>` で特定 worker の workload を基準にできる）。
詳細は [providers ガイド](./providers.md)。

## `policy.yaml`

実行 policy。built-in の完全な既定値は CLI パッケージ同梱の
`policy.yaml`（下記）。user/project の `policy.yaml` は部分
override で、既定値に merge される。

```yaml
version: 1

process:
  maxConcurrent: 1       # 同時実行 task 数
  timeout: 2m            # task の実行上限（queue・admission 待ちは含まない）
  maxOutputBytes: 65536  # stdout/stderr 合計の保持上限（設定上限 16 MiB）
  killGrace: 2s          # SIGTERM → SIGKILL までの猶予
  heartbeat:
    interval: 30s        # stderr diagnostic の最大観測間隔
  tasks: {}              # 事前登録する named task

command:
  maxConcurrent: 1       # 同時実行 command 数（`process` pool と独立）
  timeout: 2m            # grant 実行の既定上限（queue・admission 待ちは含まない）
  maxOutputBytes: 65536  # stdout/stderr 合計の保持上限（設定上限 16 MiB）
  killGrace: 2s          # SIGTERM → SIGKILL までの猶予
  heartbeat:
    interval: 30s        # stderr diagnostic の最大観測間隔
  grants: {}             # 実行を許可する grant（trusted layer のみ authoring 可）

execution:
  timeout: 300s          # run 経路: 進捗の最大無言間隔（stall window。
                        # RunEvent の観測でリセット。workflow 経路は
                        # 従来どおり全体の absolute deadline）
  maxTurns: null         # null = turn 上限なし（既定）。正の整数は
                        # opt-in の bounded budget で、到達時は
                        # MAX_TURNS_EXCEEDED + checkpoint 保存
  maxToolCalls: 64       # tool call 総数の上限

tools:
  allow:                 # worker が使える tool 名（unknown tool は失敗）
    - read
    - grep
    - find
    - ls
    - write
    - edit
    - count_tokens
    - process
    - command

filesystem:
  symlinks: within-workspace   # deny | within-workspace
  denyPaths: []               # built-in deny pattern に追加する glob

model:
  maxTier: medium        # worker が解決してよい tier の上限
  tierFallback: none     # none | higher（該当 tier の model が無い場合の挙動）

output:
  maxTokens: 2000        # worker output の token 上限
  repairAttempts: 1      # schema 不適合 output の repair 試行回数

retry:
  transientAttempts: 1   # completion call 単位の provider transient
                        # 失敗（429/5xx/応答なし transport）への retry
                        # 回数。run-level abort は retry 対象外で、
                        # backoff 待ちは stall window の無言として計上

route:
  rewrite:
    approvalBypass: []   # permissionDecision:"allow" 併記の rewrite
                        # emit（親 agent の approval prompt を素通し
                        # する応答）を許可する hook target 名
                        # （trusted layer のみ authoring 可）

security:
  execDenyPatterns: []   # operator 宣言の executable 名 regex
                        # deny-list（既定: 空 = deny 無し。
                        # trusted layer のみ authoring 可）
```

`process` tool（policy 事前登録の named task を実行する
canonical tool）の task 定義・実行制御の詳細は
[managed process ガイド](./managed-process.md) を参照。

`command` section の field は `process` と同じ lifecycle 既定値
（`maxConcurrent`・`timeout`・`killGrace`・`heartbeat.interval`・
`maxOutputBytes`）に `grants`（実行を許可する executable の
宣言 map）を加えた構成。grant の shape は
`{executable, fixed?, argv?, inputs?, outputs?, cwd?, env?,
timeout?, killGrace?, maxOutputBytes?, exitCodes?, scope?}` で、
`executable` は bare name（ambient PATH
解決）または absolute path、`argv` は positional pattern
（literal は exact match、`prefix*` は prefix match、最終要素の
bare `*` は zero-or-more の残余 token を受ける rest marker）。
`fixed`/`inputs`/`outputs`/`scope` は issue #165 で追加された
field — `fixed:true` は `argv` を固定 argv literal として扱い、
`inputs`/`outputs` は declared I/O から resource manifest を
導出し、`scope` は `"trusted"`（既定）/`"project"`（自己弱体化）
の enum。`exitCodes` は受理 exit code の配列（未宣言は `[0]`）。
`command.grants` は builtin と user-scope の `policy.yaml` でのみ
authoring でき、project `.cheaper/policy.yaml` に `command` key を
含めると policy load が `InvalidRuntimePolicy` で失敗する
（narrowing も不可）。user policy の grants は
`cheaper policy grant {add,list,remove}` で管理できる（validation
付きの managed 経路 — `fixed:true`・明示 `scope` を含む spec は
`InvalidRequest` で拒否される。write path の caveat —
read-modify-write の last-wins・comment/整形の非保存 — は
[usage ガイド](./usage.md) を参照）。その他に invoker が
`--grant` flag で run 単位の非永続 grant を注入する経路がある。
grant・信頼境界・sandbox 限界の詳細は
[command ガイド](./command.md) を参照。

### unified `exec` grant namespace（resolver model）

`process`・`command` 両 section は、resolve 時に単一の `exec`
grant namespace へ畳まれる（issue #165）。authoring shape・
section 分離は不変だが、resolved policy では `process.tasks` が
`fixed:true` grant（executable = `command[0]`、argv =
`command[1..]` の literal、lifecycle 既定値は `process` section
由来）、`command.grants` が `!fixed` grant（`argv` は positional
pattern、lifecycle 既定値は `command` section 由来）として同じ
`ResolvedExecPolicy.grants` map に収まる。concurrency pool も
surface 別（`fixed` → `process.maxConcurrent`、`!fixed` →
`command.maxConcurrent`）に保たれる。

両 section の名空間は共有のため、`command.grants` の key と
`process.tasks` の key が衝突する policy は `InvalidRuntimePolicy`
+ `details.{grant,task}`（両方に衝突名）で fail closed に拒否
される。project policy 由来の task は `scope:"project"` の
grant として解決され、route の `grant:`（anchor）・
`grants:`（追加 delegated member）・`substitute.grant`
（substitute-exec mode の実行 grant）参照が解決できるのは
いずれも `scope:"trusted" && !fixed` の grant のみ
（`is_route_eligible` が単一 predicate — `fixed`・
`scope:"project"`・未知名は `Viability::NoGrant`）。
`grants`・`substitute`・`trusted` の宣言自体は builtin/user 層の
routes.yaml に限られ、project routes.yaml の entry に
`grants`・`substitute`・`trusted` key が存在すると route load が
`InvalidRuntimePolicy` で失敗する（project が宣言できるのは
単数 `grant:` anchor 参照のみ、かつ非衝突 `match.command` で —
`trusted` は builtin/user 層では受理されるが inert で、route の
trust は宣言値ではなく宣言層 provenance が決める）。
詳細は [managed process ガイド](./managed-process.md)・
[command ガイド](./command.md)・
[parent-agent 連携ガイド](./parent-agent-integration.md) を参照。

### route.rewrite.approvalBypass（hook rewrite の approval opt-in）

parent-agent hook が command を route run へ rewrite する応答で、
affirmative permission decision（`permissionDecision:"allow"`）を
併記する emit（bypassing emit）を許可する hook target 名の配列。

```yaml
route:
  rewrite:
    approvalBypass: [codex]   # bypassing emit を許可する target
```

- 既定値は `[]`（bypassing emit なし）。未指定・`null`・空配列は
  すべて `[]` と等価。値 domain は `codex`/`cursor`/`copilot` で、
  未知の値・重複した項目・非配列・非文字列 item、および非 object
  の `route`/`rewrite` は `InvalidRuntimePolicy` で policy load が
  失敗する（fail-loud）。
- `route` section は `command.grants` と同じ trust 境界を持ち、
  builtin と user-scope の `policy.yaml` でのみ authoring できる。
  project `.cheaper/policy.yaml` に `route` key を含めると policy
  load が `InvalidRuntimePolicy` で失敗する — untrusted な
  checkout が approval bypass を自己宣言できない構造保証。
- per-target の現状:

  | target | 状況 |
  | --- | --- |
  | `codex` | flag に列挙した場合のみ、rewrite 応答に `permissionDecision:"allow"` + `updatedInput` を emit（実機確定済み・codex-cli 0.162.1）。flag が無い場合は従来どおり fallback verdict を render し `updatedInput` は emit しない |
  | `cursor`/`copilot` | emit contract が実機未検証のため flag に列挙しても現在は **no-op**（fallback render を維持）。「flag は許可の宣言であり、有効化は emit contract 確定後」 |
  | `pretooluse`（claude）/`devin` | approval を温存する emit を既に持つため flag 不要・対象外 |

- `codex` の bypassing emit は親 agent の approval prompt を
  素通しするが、codex 自身の execpolicy を迂回するわけではない —
  rewritten command も codex の policy 評価（`blocked by policy`）
  を受ける。「policy opt-in で全検査を迂回する」機能ではない。
- なお codex では未 review の hook は実行されないため、hook の
  trust 確立（`--dangerously-bypass-hook-trust` 等）がこの emit
  の前提になる。

### security.execDenyPatterns（opt-in executable deny-list）

`command` tool の grant executable に対する、operator 宣言の
deterministic executable deny-list。各要素は executable 名の
regex で、match した executable への grant を拒否する。

```yaml
security:
  execDenyPatterns:
    - 'pythonw?|python\d+(?:\.\d+)*[a-z]?'   # 例: interpreter 類
```

- 既定値は `[]`（deny 無し）。`security` 節の未指定・
  `execDenyPatterns: null`・`execDenyPatterns: []` はすべて「何も
  拒否しない」と等価。#163 で mandatory deny 層は除去済みのため、
  宣言しない限り deny は一切効かない opt-in profile。
- `security` section は `command.grants`・`route` と同じ trust 境界
  を持ち、builtin と user-scope の `policy.yaml` でのみ authoring
  できる。project `.cheaper/policy.yaml` に `security` key を含める
  と policy load が `InvalidRuntimePolicy` で失敗し、invocation 層
  （`--grant` 合成 policy）に `security` key を含めると
  `InvalidRequest` + `details.layer="invocation"` で失敗する。
- merge は配列の**丸ごと置換** — union しない。builtin が `[a]`・
  user が `[b]` なら effective は `[b]`。builtin 値を継承するには
  user 側に `security` を書かない。
- match は executable basename stem（`/`・`\` 区切りの最終要素 →
  小文字化 → `.exe`/`.com` strip）への anchored case-insensitive
  完全一致で、declared spec の policy resolve・pin 時の resolved
  executable・spawn 前再検証の 3 適用点に同一 predicate で効く。
  適用対象は unified `exec` namespace の**全 grant** —
  `command.grants` の executable と `process.tasks` 由来の fixed
  grant executable の双方（issue #165 で均一化）。これは
  containment 機構ではなく known-bad executable への
  fail-fast — match 規約・旧 mandatory 一覧からの migration seed・
  残余の注意事項は [command ガイド](./command.md) を参照。

### execution.timeout と maxTurns（run 経路）

run 経路の `execution.timeout` は **stall window**（進捗の最大無言
間隔）— stream から `RunEvent` が観測されるたびに期限がリセット
され、最後の進捗から `timeout` だけ無言が続いた時点で `TIMEOUT`
abort になる。run 全体の絶対時間ではないため、進捗し続ける長大な
run は 300s を超えて完走できる。一方 `workflow run` の同名 field は
従来どおり workflow 全体の **absolute deadline** であり、意味を共有
しない。stall window は「進捗の最大無言間隔」なので、最長の単一
tool/command 実行（`process.timeout`・grant `timeout`）や非
streaming の completion call、transient retry の backoff 待ち
（`retry.transientAttempts` — provider transient 失敗
（429/5xx/応答なし transport）にのみ適用され、run-level abort
（`TIMEOUT`/`MAX_TURNS_EXCEEDED`/`ABORTED`/`MAX_TOOL_CALLS_EXCEEDED`）
は retry 対象外。retry の backoff 中は進捗 event が出ないため
stall window として計上される）より長く設定する必要がある。

`execution.maxTurns` は `null`（既定）または省略で **policy 上の
turn 上限なし** — 内部の runaway guard（Rig agent loop の既定上限）
のみ残る。正の整数を設定すると従来どおり `MAX_TURNS_EXCEEDED` の
policy abort 境界になる。`maxToolCalls` も同様に
`MAX_TOOL_CALLS_EXCEEDED` を発火する。

### budget 枯渇 checkpoint と `--resume`（run 経路）

`maxTurns`/`maxToolCalls` の policy 上限に達して abort した run は、
その時点までの tool 観測（commit 済み call の引数 + 結果抜粋）を
`<userConfigDir>/checkpoints/run-<id>.json` へ保存し、envelope 最外層
error の `error.details.checkpoint = {path, observationsCount}` で
参照を返す（`PARTIAL_MUTATION` wrap 時は wrapper の details に付く）。
保存は best-effort で、失敗しても error の分類・exit code は変わら
ない（`details.checkpoint` が省略されるだけ）。`TIMEOUT`・外部
signal abort・workflow job は対象外 — 再開可能な観測の定義上、
budget 枯渇のみを対象とする。

checkpoint からは `cheaper run <worker> --resume <path>` で再開
できる。worker 名は checkpoint に記録された worker と一致必須
（`--worker-file` 使用時は resolved worker 名と比較）で、不一致や
不正 file は `INVALID_INPUT`。再開 run は新しい session/policy/
budget で走り、前回の観測のみ user prompt の continuation section
として注入される。`--input`（値 `-` の stdin 指定を含む）を明示す
ると checkpoint の stored input を上書きし、省略時は stored input
を使って stdin を読まない。`--input` を変えて再開した場合、前回
観測は旧 input 由来の stale context になり得ることに注意。
`--grant` は checkpoint に永続化されない — 再開時に必要なら再度
指定する。resume run が再び上限に達した場合、新しい checkpoint は
前回+今回の観測を累積して保存する（連鎖 resume）。

部分 override では `version` と必要な項目だけ書く。

```yaml
# user config (~/.config/cheaper/policy.yaml) — 部分 override の例
version: 1
output:
  repairAttempts: 4     # 小モデル向けに repair 試行を増やす
```

## 環境変数

| 変数 | 効果 |
| --- | --- |
| `CHEAPER_CONFIG_DIR` | user config dir を任意の場所に変更する。test 隔離・検証用設定の分離に使う |
| `CHEAPER_COUNT_MAX_BYTES` | `count-tokens` の byte cap（default 16 MiB）。正の整数のみ有効 |
| `CHEAPER_READ_TOKEN_THRESHOLD` | parent-agent hook の large-read threshold（default 8192 token）。詳細は [integration ガイド](./parent-agent-integration.md) |
| `CHEAPER_LOG` | logging の有効化レベル（`off`/`error`/`warn`/`info`/`debug`/`trace`）。`.cheaper/logs` + stderr へ出力。CLI option `--log` と同じ |
| `CHEAPER_LOG_FORMAT` | ログ出力形式（`text`/`json`）。CLI option `--log-format` と同じ |
| `DEVIN_CLI` | `devin` provider が使う Devin CLI binary の path。未設定時は PATH 上の `devin` |

credential は `<userConfigDir>/credentials.json` に永続化される。
旧 Pi runtime の file（`<userConfigDir>/pi/` 以下）は一切
読まれない。`devin` provider の Devin CLI
credential は `<userConfigDir>/providers/devin/data/`
配下の child-process 専用 data dir に閉じ込められる。ただし
Windows では Devin CLI 3000.x が env による credential path
変更を無視して global `%APPDATA%\devin\credentials.toml` に
書くため、cheaper は login 後に global credential の
snapshot を同 dir へ保持する方式を取る（global session
自体は共有される。詳細は
[providers ガイド](./providers.md)）。

## 関連

- provider・auth の設定: [providers ガイド](./providers.md)
- worker との関係（`tools.allow`・`model.maxTier`・`writeScope`）:
  [workers ガイド](./workers.md)
- `process` tool の named task・実行制御:
  [managed process ガイド](./managed-process.md)
- `command` tool の grant・信頼境界・sandbox 限界:
  [command ガイド](./command.md)
