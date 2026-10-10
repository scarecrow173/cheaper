# Provider 設定ガイド

`cheaper` は内蔵の model catalog で model を解決する。catalog は
bundled snapshot（binary 埋込の provider・model 定義）+ bundled
provider plugin + user overlay（`models.json`）で構成される。
credential は `credentials.json` で、cheaper 側は `models.yaml` で
model → tier の割当を行う。

## 設定ファイルの場所

| ファイル | 場所 | 役割 |
| --- | --- | --- |
| `models.json` | `<userConfigDir>/models.json` | provider・model 定義・endpoint の user overlay（bundled catalog を上書き・追加） |
| `credentials.json` | `<userConfigDir>/credentials.json` | OAuth credential・API key（login で永続化） |
| `models.yaml` | `<userConfigDir>/models.yaml` → `<cwd>/.cheaper/models.yaml` | cheaper の tier 分類・preference |
| `policy.yaml` | 同上 | execution・tools・model・output・retry の policy override |

`<userConfigDir>` は `$CHEAPER_CONFIG_DIR` →
`$XDG_CONFIG_HOME/cheaper` → `~/.config/cheaper` の順で解決する
（詳細は [configuration ガイド](./configuration.md)）。後から
読んだ層が前の層を上書きする（project が user より優先）。
環境依存設定はリポジトリへコミットしない。

credential は `<userConfigDir>/credentials.json` に集約される
（temp file + rename の atomic write）。credential を含むため
project-local `.cheaper/` には置かない — `.cheaper/` は
worker・`models.yaml`・`policy.yaml` の project override
専用。`cheaper auth`・worker 実行の双方が同じ credential
store を参照する。

旧 Pi runtime の永続 file（`<userConfigDir>/pi/` 以下の
`auth.json`・`models.json`・`models-store.json`）は移行されず、
一切読まれない。Rust runtime への切替後は全 provider で再 login
が必要。

## Ollama（ローカル provider）

ローカルの Ollama を provider として使う例。Ollama 自体の起動
方法は環境に依存する（本リポジトリの検証用コンテナ手順は
[CONTRIBUTING.md](../../CONTRIBUTING.md) を参照）。

### `models.json`（user overlay）

```json
{
  "providers": {
    "ollama": {
      "name": "Ollama (local)",
      "api": "openai-completions",
      "baseUrl": "http://localhost:11434/v1",
      "apiKey": "ollama",
      "models": [
        {
          "id": "qwen3.5:9b",
          "name": "Qwen 3.5 9B",
          "reasoning": true,
          "cost": { "input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0 },
          "contextWindow": 262144,
          "maxTokens": 32768,
          "samplingParams": {
            "reasoning_effort": "none",
            "temperature": 0
          }
        }
      ]
    }
  }
}
```

- `apiKey` は optional だが、apiKey も OAuth も無い provider は
  "no authentication method configured" で拒否される。Ollama
  は任意のリテラル値でよい（秘密情報ではない）。
- `samplingParams` はリクエストボディへそのまま merge される。
  thinking モデルは `reasoning_effort: "none"` で推論フェーズを
  無効化できる（無効化しないと output が reasoning に消費されて
  `content` が空になることがある）。

### cheaper `models.yaml`

```yaml
version: 1
overrides:
  "ollama/qwen3.5:9b": low
preferences:
  low:
    - "ollama/*"
```

- tier 分類は `overrides` → bundled curated exceptions →
  auto 分類 → unclassified（**medium**）の順で決まる。auto
  分類は worker の `outputPerInput`（既定 `1.0`）から
  `effectiveCost = input + output * outputPerInput` を計算し、
  利用可能な priced model 集合の percentile で下位 1/3 → low、
  中位 1/3 → medium、上位 1/3 → high に分ける。pricing は runtime
  catalog（bundled snapshot は models.dev 由来）が canonical で、local
  model（`ollama/*`）は curated exception で `low` に固定されて
  いる。model ごとの effective tier と由来（`tierSource`・
  `effectiveCost`・`effectiveCostPercentile`）は `cheaper
  models` で確認できる。`--worker <name>` で特定 worker の
  workload profile を基準にした ranking を表示できる。
  `4b` / `9b` のような suffix は `overrides` で low に pin する。
- built-in worker は `tier: low` 要求 + built-in policy は
  `tierFallback: none` なので、low tier に解決できる model が
  1 つ必要。

## `response_format`（構造化出力）の使い分け

- **main session では使わない**: `response_format`（`json_object`
  / `json_schema`）はコンテンツを JSON に強制して tool call を
  抑制するため、tool を使う worker が動かない。
- **repair session では有効**: repair は tool 無し session な
  ので、grammar 制約が安全に働き、小モデルの key 名誤りを抑制
  できる。runtime が repair session の `samplingParams` に
  worker の output schema を自動注入する。

## OpenAI（ChatGPT サブスクリプション / `openai-codex`）

`openai-codex` は bundled catalog の built-in provider であり、
ChatGPT サブスクリプションの OAuth 認証（Codex 系モデル）で
利用できる。API key は不要だが、初回のみ interactive な OAuth
login が必要。

### セットアップ

`cheaper auth` が OAuth flow を内蔵しており、credential は
`credentials.json` に保存される。

```bash
cheaper auth login openai-codex
# ブラウザが開き ChatGPT アカウントへのサインインを求められる。
# browser が使えない環境では --method device で device code flow。

cheaper auth status openai-codex
# → result.providers[0].configured === true なら configured
```

OAuth login は `auth status` が `methods` に `oauth` を
示す provider で同じ手順を使える（例: `anthropic` /
`github-copilot` / `openrouter` / `kimi-coding` /
`radius` / bundled `devin`）。browser auth_url flow と
device code flow のどちらが走るかは provider が提示する
method に従う。

API key 型の provider は `cheaper auth login <provider>
--type api_key` で対話入力するか、`XAI_API_KEY` のような
provider 別の環境変数で credential を解決する。各
provider が受け付ける login method は `cheaper auth
status` で確認するのが canonical。

### cheaper `models.yaml`

```yaml
version: 1
overrides:
  "openai-codex/gpt-5.5": low
preferences:
  low:
    - "openai-codex/gpt-5.5"
    - "ollama/*"   # ローカル fallback
```

利用する codex 系 model はアカウント側で利用可能なものを
`overrides` で low に pin する。`auth status` は
provider の auth capability（login method・
subscription flag）と configured state・credential
metadata を確認するコマンドであり、model catalog は
返さない。available model の一覧・pricing・effective tier は
`cheaper models` で確認し、実際に使われた model の確認は
worker 実行の result envelope `meta.model` で行う
（下記「検証」参照）。

### 検証

```bash
echo '{"task":"summarize","paths":["<file>"]}' | cheaper bulk-read --cwd <ws>
# envelope の meta.model.provider が "openai-codex" になれば production provider 経由
```

- `openai-codex` は built-in provider のため `models.json` への
  provider 追加は不要（`credentials.json` の OAuth credential
  のみで動く）。
- subscription 利用のため API 課金ではなく quota 消費。rate
  limit に注意。

## その他の provider

bundled catalog がサポートする provider と、各 provider が
受け付ける login method（`methods` の `oauth` /
`api_key`）は `cheaper auth status` で確認するのが
canonical。`models.json` に OpenAI 互換 endpoint を持つ
任意の provider を追加することも可能（`api:
"openai-completions"` + `baseUrl`）。

### bundled provider plugin

bundled catalog に無い provider は、cheaper が明示的に bundle
する provider plugin として runtime に登録される
（ADR-034r05iHrZUE6WoaNDNjvX）。plugin は provider の metadata・auth flow・
model catalog・model transport を登録
でき、`cheaper auth status/login/logout`・model の tier
解決から通常の provider と同様に扱える。

Worker の実行環境を最小化するため、external provider package の
discovery / load は行わない — 登録対象は cheaper が明示的に
bundle する plugin のみ。

### Devin Local（bundled）

provider id `devin` は Devin CLI / Devin Local の model 群を
使う bundled provider plugin（SPEC-034r05iHK8bA7gAHzYnfP8 / ADR-034r05iHrZUE6WqcIaH1CU）。Devin
Cloud Agent API ではなく、ローカルの Devin CLI 経由の model
access を対象とする。

- **前提**: `devin` CLI が PATH 上にあること。binary は
  `DEVIN_CLI` env で明示指定もできる。
- **login**: `cheaper auth login devin`（oauth method）—
  cheaper 専用の data dir を child process env
  （`XDG_DATA_HOME`、Windows では `LOCALAPPDATA` 併用）へ
  設定した状態で `devin auth login` をそのまま実行する。
  Devin CLI 自身の対話 flow が走り、生成された
  `credentials.toml` を cheaper が読んで credential store
  （`credentials.json`）へ永続化する。
- **credential の隔離**: Devin CLI の data dir は
  `<userConfigDir>/providers/devin/data/` 配下に閉じ込め
  られる（`<data>/devin/credentials.toml`）。global の
  `~/.local/share/devin` や Devin Desktop の credential は
  暗黙利用しない。`process.env` は変更されない。
- **Windows での credential snapshot**: Devin CLI 3000.x は
  Windows で env による credential path 変更を無視し、常に
  global の `%APPDATA%\devin\credentials.toml` を使う。この
  ため Windows の `cheaper auth login devin` は `devin auth
  login` 実行後に global の `credentials.toml` を cheaper
  data dir へ atomic copy（snapshot）する。cheaper が使う
  credential の供給源は変わらず cheaper-owned snapshot で
  あり、global toml は同一 account かの判定にのみ使う
  （key が一致すれば login を skip する）。`devin auth
  login` 自体は global session に作用する点に注意。
- **model catalog**: `devin models list --format json` の
  live catalog が優先。認証前は最小の bootstrap list で
  provider を認識させ、login 直後・refresh 時に live
  catalog で置き換える。thinking level は variant suffix
  から `thinkingLevelMap` へ写像される。
- **logout**: `cheaper auth logout devin` で stored credential
  を削除し、isolated env で `devin auth logout`
  を実行して cheaper-owned の `credentials.toml` も削除する
  （Devin CLI 未到達時も file 削除は best-effort で行う）。
  次回 `cheaper auth login devin` は `devin auth login` を
  再実行する。
  - **Windows では挙動が異なる**: `devin auth logout` は
    global session を消すため cheaper は spawn しない。
    cheaper-owned snapshot と stored credential のみ削除し、
    global Devin session は login 済みのまま残る。完全に
    sign out するには `devin auth logout` を手動実行する
    こと（logout 時にもその旨を通知する）。
- Devin 固有の slash command / tool / event handler は
  持ち込まない（provider-only contract、ADR-034r05iHrZUE6WoaNDNjvX）。

## 関連

- config dir・優先度・policy.yaml: [configuration ガイド](./configuration.md)
- CLI の auth command: [usage ガイド](./usage.md)
- リポジトリ開発時の検証用 Ollama コンテナ:
  [CONTRIBUTING.md](../../CONTRIBUTING.md)
