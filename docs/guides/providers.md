# Provider 設定ガイド

`cheaper` は model 解決を Pi runtime
（`@earendil-works/pi-coding-agent`）に委譲する。provider の
endpoint・認証は Pi の `models.json` と credential で、cheaper
側は `models.yaml` で model → tier の割当を行う。

## 設定ファイルの場所

| ファイル | 場所 | 役割 |
| --- | --- | --- |
| `models.json` | `<userConfigDir>/pi/models.json` | Pi の provider・model 定義・endpoint |
| `auth.json` | `<userConfigDir>/pi/auth.json` | OAuth credential・API key（login で永続化） |
| `models-store.json` | `<userConfigDir>/pi/models-store.json` | Pi の model catalog cache |
| `models.yaml` | `<userConfigDir>/models.yaml` → `<cwd>/.cheaper/models.yaml` | cheaper の tier 分類・preference |
| `policy.yaml` | 同上 | execution・tools・model・output・retry の policy override |

`<userConfigDir>` は `$CHEAPER_CONFIG_DIR` →
`$XDG_CONFIG_HOME/cheaper` → `~/.config/cheaper` の順で解決する
（詳細は [configuration ガイド](./configuration.md)）。後から
読んだ層が前の層を上書きする（project が user より優先）。
環境依存設定はリポジトリへコミットしない。

Pi の永続 file は `<userConfigDir>/pi/` に集約される。
cheaper は `PI_CODING_AGENT_DIR` を参照せず、Pi の既定 dir
（`~/.pi/agent`）には書き込まない。credential を含むため
project-local `.cheaper/` にも置かない — `.cheaper/` は
worker・`models.yaml`・`policy.yaml` の project override
専用。`cheaper auth`・worker 実行の双方が同じ credential
store（`<userConfigDir>/pi/auth.json`）を参照する。

## Ollama（ローカル provider）

ローカルの Ollama を provider として使う例。Ollama 自体の起動
方法は環境に依存する（本リポジトリの検証用コンテナ手順は
[CONTRIBUTING.md](../../CONTRIBUTING.md) を参照）。

### Pi `models.json`

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
  Pi が "no authentication method configured" で拒否する。Ollama
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

- built-in の tier 分類は model id のパターン（`haiku|mini|nano|
  flash|lite|small|codex-mini|\b(1|3|7|8)b\b`）で low を判定し、
  cost 0 / 未分類は **medium** になる。`4b` / `9b` のような
  suffix は `overrides` で low に pin する。
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

Pi には `openai-codex` provider が built-in であり、ChatGPT
サブスクリプションの OAuth 認証（Codex 系モデル）で利用できる。
API key は不要だが、初回のみ interactive な OAuth login が必要。

### セットアップ

`pi` の standalone install は不要。`cheaper auth` が同じ Pi
runtime の OAuth flow を内蔵しており、credential は同じ
`auth.json` に保存される。

```bash
cheaper auth login openai-codex
# ブラウザが開き ChatGPT アカウントへのサインインを求められる。
# browser が使えない環境では --method device で device code flow。

cheaper auth status openai-codex
# → result.providers[0].configured === true なら configured
```

OAuth 対応 provider は `anthropic` / `github-copilot` /
`openrouter` / `xai` 等も同じ手順。api_key 型は
`--type api_key` で対話入力。

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
`overrides` で low に pin する。対応状況は契約プランや API の
提供状態に依存するため、実際に `auth status` で確認して
model 一覧を確認する。

### 検証

```bash
echo '{"task":"summarize","paths":["<file>"]}' | cheaper bulk-read --cwd <ws>
# envelope の meta.model.provider が "openai-codex" になれば production provider 経由
```

- `openai-codex` は built-in provider のため `models.json` への
  provider 追加は不要（`auth.json` の OAuth credential のみで
  動く）。
- subscription 利用のため API 課金ではなく quota 消費。rate
  limit に注意。

## その他の provider

Pi runtime がサポートする provider は `cheaper auth status` で
一覧できる（login 可能な method も表示）。`models.json` に
OpenAI 互換 endpoint を持つ任意の provider を追加することも
可能（`api: "openai-completions"` + `baseUrl`）。

### bundled provider plugin

Pi built-in に無い provider は、cheaper が明示的に bundle
する provider plugin として `ModelRuntime` に登録される
（ADR-0028）。plugin は provider の metadata・auth flow・
model catalog・`streamSimple` 等の model transport を登録
でき、`cheaper auth status/login/logout`・model の tier
解決から通常の provider と同様に扱える。

Worker の実行環境を最小化するため、Pi の general-purpose
extension loader は有効化しない。external provider package
の discovery / load も行わない — 登録対象は cheaper が
明示的に bundle する plugin のみ。

### Devin Local（bundled）

provider id `devin` は Devin CLI / Devin Local の model 群を
使う bundled provider plugin（SPEC-0026 / ADR-0029）。Devin
Cloud Agent API ではなく、ローカルの Devin CLI 経由の model
access を対象とする。

- **前提**: `devin` CLI が PATH 上にあること。binary は
  `DEVIN_CLI` env で明示指定もできる。
- **login**: `cheaper auth login devin`（oauth method）—
  cheaper 専用の data dir を child process env
  （`XDG_DATA_HOME`、Windows では `LOCALAPPDATA` 併用）へ
  設定した状態で `devin auth login` をそのまま実行する。
  Devin CLI 自身の対話 flow が走り、生成された
  `credentials.toml` を cheaper が読んで Pi credential
  として永続化する。
- **credential の隔離**: Devin CLI の data dir は
  `<userConfigDir>/providers/devin/data/` 配下に閉じ込め
  られる（`<data>/devin/credentials.toml`）。global の
  `~/.local/share/devin` や Devin Desktop の credential は
  暗黙利用しない。`process.env` は変更されない。
- **model catalog**: `devin models list --format json` の
  live catalog が優先。認証前は最小の bootstrap list で
  provider を認識させ、login 直後・refresh 時に live
  catalog で置き換える。thinking level は variant suffix
  から `thinkingLevelMap` へ写像される。
- **logout**: `cheaper auth logout devin` で Pi 側の stored
  credential を削除。isolated data dir 内の
  `credentials.toml` は残る（global 汚染は無く、次回
  login で再利用される）。
- Devin 固有の slash command / Pi tool / event handler は
  持ち込まない（provider-only contract、ADR-0028）。

## 関連

- config dir・優先度・policy.yaml: [configuration ガイド](./configuration.md)
- CLI の auth command: [usage ガイド](./usage.md)
- リポジトリ開発時の検証用 Ollama コンテナ:
  [CONTRIBUTING.md](../../CONTRIBUTING.md)
