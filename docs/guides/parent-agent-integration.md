# Parent-agent 連携ガイド

`cheaper` は親 Coding Agent（Claude Code / Codex / Copilot /
Cursor / Windsurf 等）から処理を委譲するための worker runtime。
本 guide は親 agent と cheaper の責務分担・配布 primitive・
target 別挙動・検証方法を説明する。

## 役割分担

- **親 Coding Agent**: 判断・統合・ユーザーとの対話。task の
  定義・worker の選定・結果の解釈・workspace への最終統合を行う。
- **cheaper**: 機械的・大 context の処理の実行。worker 定義・
  tool 制約・model 解決・schema 検証・usage 計測を行い、
  構造化された result と meta を返す。

親 agent は `cheaper` を 1-shot CLI として呼び出し、stdin に
task JSON を渡して stdout の envelope を読む（詳細は
[usage ガイド](./usage.md)）。

## 配布 primitive

cheaper は parent agent へ以下を配布する（APM package 経由）:

- **skills**（4 件）:
  - `cheaper-usage`: CLI contract（command・envelope・exit code・
    provider/model 設定）を agent へ教える
  - `cheaper-bulk-read` / `cheaper-code-write` /
    `cheaper-doc-write`: 各 built-in worker への委譲方法
- **hooks**（`pre_read_code`）: read 前に `count-tokens` を実行し、
  threshold（default 8192 token、
  `CHEAPER_READ_TOKEN_THRESHOLD` で上書き）を超える file の
  read を block して `bulk-read` へ誘導する。計測不能・
  判定不能なケースは fail-open（exit 0 で素通し）なので、
  設定が壊れていても通常の read が止まらない。
- **bundled CLI**: self-contained の `cheaper` CLI（skills/hooks
  が内部から直接呼べる）

## install

consumer（親 agent 環境）は `cheaper` の distributable APM package
を依存として install する:

```yaml
# consumer の apm.yml
dependencies:
  apm:
  - scarecrow173/cheaper/packages/apm@main   # virtual package 参照
  # または local path: - path: ../cheaper/packages/apm
```

```bash
apm install --target windsurf   # 利用する target を指定
```

## target 別挙動

hook の動作は target の hook event 体系に依存する。

- **Windsurf**: `pre_read_code` event が native で、read 前に
  hook が実行される。threshold 超過 file は block + bulk-read
  への誘導メッセージが返る。
- **その他 target**（Claude / Codex / Copilot / Cursor 等）:
  同じ hook 登録が deploy されるが、`pre_read_code` event を
  発火しないため no-op として動作する（登録は harmless、
  機能は無効）。target 固有の adapter 化は今後の拡張余地
  （ADR-0016・DISC-0009）。

skills（`cheaper-usage` 等）は target 非依存で機能する。

## 自動 vs 明示的な委譲

- **自動**: hook（`pre_read_code`）が threshold 超過 read を
  検出して parent agent へ bulk-read 委譲を促す。
- **明示**: parent agent（またはユーザーが skill を発動）が
  `cheaper bulk-read` 等を直接呼ぶ。

## 閾値の設定

`CHEAPER_READ_TOKEN_THRESHOLD` で large-read threshold（default
8192 token）を変更できる。hook 内の `count-tokens` 呼び出しは
workspace 内 file のみを対象とし、boundary 外・binary・
計測不能な file は fail-open（素通し）になる。

## 連携の確認

```bash
# hook bundle に deploy された CLI の動作確認
<deployed-hook>/cli/bin/cheaper.mjs --version
<deployed-hook>/cli/bin/cheaper.mjs count-tokens <file>

# skills/hooks が install されたかは target の config を確認
# （例: .windsurf/hooks/ 等、target 固有の deploy path）
```

実際の e2e 検証手順は [CONTRIBUTING.md](../../CONTRIBUTING.md) の
「実 provider 検証」を参照。

## 関連

- CLI contract: [usage ガイド](./usage.md)
- provider / auth 設定: [providers ガイド](./providers.md)
- 設計記録: `docs/designs/0013-apm-skills-and-pre-read-hook.md`・
  `docs/adr/0013-distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md`・
  `docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md`
