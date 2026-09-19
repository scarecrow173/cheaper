# 設定ガイド

`cheaper` の設定は 2 系統に分かれる。

- **provider / auth**: Pi runtime が管理する `models.json`・
  credential。詳細は [providers ガイド](./providers.md)。
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
pricing は Pi の runtime catalog（models.dev 由来）を canonical
とし、欠損分は bundled snapshot で補完される。

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

execution:
  timeout: 120s          # run 全体の上限（model 解決・session 構築を含む）
  maxTurns: 8            # agent loop の最大 turn 数
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
  transientAttempts: 1   # transient failure の retry 回数
```

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
| `DEVIN_CLI` | `devin` provider が使う Devin CLI binary の path。未設定時は PATH 上の `devin` |

Pi runtime の永続 file（`auth.json`・`models.json`・
`models-store.json`）は user config dir 配下の `pi/` に
保存される（`<userConfigDir>/pi/`）。`PI_CODING_AGENT_DIR`
は cheaper では使用しない。`devin` provider の Devin CLI
credential は `<userConfigDir>/providers/devin/data/`
配下の child-process 専用 data dir に閉じ込められる。
詳細は [providers ガイド](./providers.md)。

## 関連

- provider・auth の設定: [providers ガイド](./providers.md)
- worker との関係（`tools.allow`・`model.maxTier`・`writeScope`）:
  [workers ガイド](./workers.md)
