# Provider 設定ガイド

`cheaper` は model 解決を Pi (`@earendil-works/pi-coding-agent`) に委譲する。
provider 認証・endpoint は Pi の `models.json` で、cheaper 側は `models.yaml` で
tier 割り当てを行う。この構成は本リポジトリ外（ユーザー環境）に置く。

## 設定ファイルの場所

| ファイル | 場所 | 役割 |
| --- | --- | --- |
| `models.json` | `%USERPROFILE%\.pi\agent\models.json`（`PI_CODING_AGENT_DIR` で変更可） | Pi の provider・モデル定義・認証 |
| `models.yaml` | `$CHEAPER_CONFIG_DIR/models.yaml` → `<cwd>/.cheaper/models.yaml` | cheaper の tier 分類・preference |
| `policy.yaml` | 同上 | execution timeout / maxTurns / repairAttempts 等の policy 上書き |

後から読んだ層が前の層を上書きする（project が user より優先）。
環境依存設定はリポジトリへコミットしない。

## Ollama（ローカル・実 provider 検証済み）

### コンテナ

本リポジトリの `docker-compose.yml` で Ollama を起動する。NVIDIA GPU パススルー
（`deploy.resources.reservations.devices`）が設定済みのため、NVIDIA Container
Toolkit の無い環境では `docker compose up` が失敗する場合がある。その場合は
`deploy:` セクションを外すか、toolkit をインストールする。
詳細は AGENTS.md「検証用 Ollama コンテナ」。

```bash
docker compose up -d
docker compose exec ollama ollama pull qwen3.5:9b
curl http://localhost:11434/v1/models   # モデル一覧が返れば起動済み
```

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

注意点:

- `apiKey` は optional だが、apiKey も OAuth も無い provider は Pi の provider
  composer が "no authentication method configured" で拒否する。Ollama は任意の
  リテラル値でよい（秘密情報ではない）
- `samplingParams` はリクエストボディへそのまま merge される。thinking モデルは
  `reasoning_effort: "none"` で推論フェーズを無効化できる（`think: false` は
  Ollama OpenAI 互換では無視される実測あり）。無効化しないと output が reasoning
  に消費されて `content` が空になることがある

### cheaper `models.yaml`

```yaml
version: 1
overrides:
  "ollama/qwen3.5:9b": low
preferences:
  low:
    - "ollama/*"
```

- built-in の tier 分類は model id のパターン（`haiku|mini|nano|flash|lite|small|
  codex-mini|\b(1|3|7|8)b\b`）で low を判定し、cost 0 / 未分類は **medium** になる。
  `4b` / `9b` のような suffix は `overrides` で low にピン留めする
- built-in worker は `tier: low` 要求 + built-in policy は `tierFallback: none`
  なので、low tier に解決できるモデルが 1 つ必要

## `response_format`（構造化出力）の使い分け

- **main session では使わない**: `response_format`（`json_object` / `json_schema`）は
  コンテンツを JSON に強制するため tool call を抑制し、tool を使う worker が動かない
- **repair session では有効**: repair は tool 無し session（`noTools: "all"`）なので、
  grammar 制約が安全に働き、小モデルがキー名を間違える問題を抑制できる。
  runtime が repair session の `samplingParams` に `json_schema`（worker の output
  schema）を自動注入する（runtime-pi `repair.ts`）

## 小モデル運用の実測知識（EXP-0002）

- repair は例外的処理ではなく通常経路。`qwen3.5:4b` でも `json_schema` 制約付きの
  repair で schema 適合できたが、`repairAttempts` は余裕を持たせる（`policy.yaml` の
  `output.repairAttempts`、実測 4）
- CPU 推論 + thinking モデルは遅い。`reasoning_effort: "none"` で大幅に短縮される
- `execution.timeout` は agent execution timeout（model 解決・session 構築を含む
  run 区間）。`--timeout` で narrow できるが policy 上限は越えられない
- `CHEAPER_CONFIG_DIR` を smoke 用ディレクトリに向けると、普段のユーザー設定と
  検証設定を分離できる（`pnpm test` も user config を読まない hermetic 構成）
